//! Flutter counterpart: material_ui's `floating_action_button_location.dart`.
//!
//! The standard locations and animators are per-thread `Rc`s handed out by functions on the
//! trait objects, so the scaffold compares them as Dart compares its `const` instances: by
//! identity (`Rc::ptr_eq`).

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    AlwaysStoppedAnimation, Animation, AnimationLazyListenerData, AnimationLazyListenerMixin,
    AnimationLocalListenersData, AnimationLocalListenersMixin, AnimationLocalStatusListenersData,
    AnimationLocalStatusListenersMixin, AnimationRef, AnimationStatus, AnimationStatusListener,
    AnyAnimation, CompoundAnimation, Curve, CurveTween, Curves, Drive, Interval, ReverseAnimation,
    Threshold, Tween, compound_did_start_listening, compound_did_stop_listening,
};
use waft_embedder::{Offset, TextDirection};
use waft_foundation::{App, Handle, Listener, Ref};

use crate::scaffold::ScaffoldPrelayoutGeometry;

/// The margin that a `FloatingActionButton` should leave between it and the
/// edge of the screen.
///
/// [`FloatingActionButtonLocation::end_float`](dyn FloatingActionButtonLocation::end_float)
/// uses this to set the appropriate margin between the `FloatingActionButton` and the end of
/// the screen.
pub const K_FLOATING_ACTION_BUTTON_MARGIN: f64 = 16.0;

/// The amount of time the `FloatingActionButton` takes to transition in or out.
///
/// The `Scaffold` uses this to set the duration of `FloatingActionButton`
/// motion, entrance, and exit animations.
pub const K_FLOATING_ACTION_BUTTON_SEGUE: Duration = Duration::from_millis(200);

/// The fraction of a circle the `FloatingActionButton` should turn when it enters.
///
/// Its value corresponds to 0.125 of a full circle, equivalent to 45 degrees or pi/4 radians.
pub const K_FLOATING_ACTION_BUTTON_TURN_INTERVAL: f64 = 0.125;

/// If a `FloatingActionButton` is used on a `Scaffold` in certain positions,
/// it is moved [`K_MINI_BUTTON_OFFSET_ADJUSTMENT`] pixels closer to the edge of the screen.
///
/// This is intended to be used with `FloatingActionButton.mini` set to true,
/// so that the floating action button appears to align with `CircleAvatar`s
/// in the `ListTile.leading` slot of a `ListTile` in a `ListView` in the
/// `Scaffold.body`.
///
/// More specifically:
/// * In the following positions, the `FloatingActionButton` is moved *horizontally*
///   closer to the edge of the screen:
///   * `FloatingActionButtonLocation.miniStartTop`
///   * `FloatingActionButtonLocation.miniStartFloat`
///   * `FloatingActionButtonLocation.miniStartDocked`
///   * `FloatingActionButtonLocation.miniEndTop`
///   * `FloatingActionButtonLocation.miniEndFloat`
///   * `FloatingActionButtonLocation.miniEndDocked`
/// * In the following positions, the `FloatingActionButton` is moved *vertically*
///   closer to the bottom of the screen:
///   * `FloatingActionButtonLocation.miniStartFloat`
///   * `FloatingActionButtonLocation.miniCenterFloat`
///   * `FloatingActionButtonLocation.miniEndFloat`
pub const K_MINI_BUTTON_OFFSET_ADJUSTMENT: f64 = 4.0;

/// An object that defines a position for the `FloatingActionButton`
/// based on the `Scaffold`'s [`ScaffoldPrelayoutGeometry`].
///
/// Flutter provides [`FloatingActionButtonLocation`]s for the common
/// `FloatingActionButton` placements in Material Design applications. These
/// locations are available as functions on `dyn FloatingActionButtonLocation`
/// (`<dyn FloatingActionButtonLocation>::end_float()`).
///
/// See also:
///
///  * `FloatingActionButton`, which is a circular button typically shown in the
///    bottom right corner of the app.
///  * [`FloatingActionButtonAnimator`], which is used to animate the
///    `Scaffold.floatingActionButton` from one [`FloatingActionButtonLocation`] to
///    another.
///  * [`ScaffoldPrelayoutGeometry`], the geometry that
///    [`FloatingActionButtonLocation`]s use to position the `FloatingActionButton`.
pub trait FloatingActionButtonLocation: Debug + 'static {
    /// Places the `FloatingActionButton` based on the `Scaffold`'s layout.
    ///
    /// This uses a [`ScaffoldPrelayoutGeometry`], which the `Scaffold` constructs
    /// during its layout phase after it has laid out every widget it can lay out
    /// except the `FloatingActionButton`. The `Scaffold` uses the [`Offset`]
    /// returned from this method to position the `FloatingActionButton` and
    /// complete its layout.
    fn get_offset(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry) -> Offset;
}

/// A shared [`FloatingActionButtonLocation`], compared by identity as Dart compares its
/// locations.
pub type FloatingActionButtonLocationRef = Rc<dyn FloatingActionButtonLocation>;

/// Defines a function on `dyn FloatingActionButtonLocation` handing out clones of one
/// per-thread `Rc` of a standard location: Dart's `static const`.
macro_rules! standard_locations {
    ($( $(#[$doc:meta])* $name:ident = $value:expr; )*) => {
        impl dyn FloatingActionButtonLocation {
            $(
                $(#[$doc])*
                pub fn $name() -> FloatingActionButtonLocationRef {
                    thread_local! {
                        static LOCATION: FloatingActionButtonLocationRef = Rc::new($value);
                    }
                    LOCATION.with(Rc::clone)
                }
            )*
        }
    };
}

standard_locations! {
    /// Start-aligned `FloatingActionButton`, floating over the transition between
    /// the `Scaffold.appBar` and the `Scaffold.body`.
    ///
    /// To align a floating action button with `CircleAvatar`s in the
    /// `ListTile.leading` slots of `ListTile`s in a `ListView` in the `Scaffold.body`,
    /// use [`mini_start_top`](Self::mini_start_top) and set `FloatingActionButton.mini` to
    /// true.
    ///
    /// This is unlikely to be a useful location for apps that lack a top `AppBar`
    /// or that use a `SliverAppBar` in the scaffold body itself.
    start_top = StartTopFabLocation;

    /// Start-aligned `FloatingActionButton`, floating over the transition between
    /// the `Scaffold.appBar` and the `Scaffold.body`, optimized for mini floating
    /// action buttons.
    ///
    /// This is intended to be used with `FloatingActionButton.mini` set to true,
    /// so that the floating action button appears to align with `CircleAvatar`s
    /// in the `ListTile.leading` slot of a `ListTile` in a `ListView` in the
    /// `Scaffold.body`.
    ///
    /// This is unlikely to be a useful location for apps that lack a top `AppBar`
    /// or that use a `SliverAppBar` in the scaffold body itself.
    mini_start_top = MiniStartTopFabLocation;

    /// Centered `FloatingActionButton`, floating over the transition between
    /// the `Scaffold.appBar` and the `Scaffold.body`.
    ///
    /// This is unlikely to be a useful location for apps that lack a top `AppBar`
    /// or that use a `SliverAppBar` in the scaffold body itself.
    center_top = CenterTopFabLocation;

    /// Centered `FloatingActionButton`, floating over the transition between
    /// the `Scaffold.appBar` and the `Scaffold.body`, intended to be used with
    /// `FloatingActionButton.mini` set to true.
    ///
    /// This is unlikely to be a useful location for apps that lack a top `AppBar`
    /// or that use a `SliverAppBar` in the scaffold body itself.
    mini_center_top = MiniCenterTopFabLocation;

    /// End-aligned `FloatingActionButton`, floating over the transition between
    /// the `Scaffold.appBar` and the `Scaffold.body`.
    ///
    /// To align a floating action button with `CircleAvatar`s in the
    /// `ListTile.trailing` slots of `ListTile`s in a `ListView` in the `Scaffold.body`,
    /// use [`mini_end_top`](Self::mini_end_top) and set `FloatingActionButton.mini` to true.
    ///
    /// This is unlikely to be a useful location for apps that lack a top `AppBar`
    /// or that use a `SliverAppBar` in the scaffold body itself.
    end_top = EndTopFabLocation;

    /// End-aligned `FloatingActionButton`, floating over the transition between
    /// the `Scaffold.appBar` and the `Scaffold.body`, optimized for mini floating
    /// action buttons.
    ///
    /// This is intended to be used with `FloatingActionButton.mini` set to true,
    /// so that the floating action button appears to align with `CircleAvatar`s
    /// in the `ListTile.trailing` slot of a `ListTile` in a `ListView` in the
    /// `Scaffold.body`.
    ///
    /// This is unlikely to be a useful location for apps that lack a top `AppBar`
    /// or that use a `SliverAppBar` in the scaffold body itself.
    mini_end_top = MiniEndTopFabLocation;

    /// Start-aligned `FloatingActionButton`, floating at the bottom of the screen.
    ///
    /// To align a floating action button with `CircleAvatar`s in the
    /// `ListTile.leading` slots of `ListTile`s in a `ListView` in the `Scaffold.body`,
    /// use [`mini_start_float`](Self::mini_start_float) and set `FloatingActionButton.mini`
    /// to true.
    start_float = StartFloatFabLocation;

    /// Start-aligned `FloatingActionButton`, floating at the bottom of the screen,
    /// optimized for mini floating action buttons.
    ///
    /// This is intended to be used with `FloatingActionButton.mini` set to true,
    /// so that the floating action button appears to align with `CircleAvatar`s
    /// in the `ListTile.leading` slot of a `ListTile` in a `ListView` in the
    /// `Scaffold.body`.
    ///
    /// Compared to [`start_float`](Self::start_float), floating action
    /// buttons using this location will move horizontally _and_ vertically
    /// closer to the edges, by [`K_MINI_BUTTON_OFFSET_ADJUSTMENT`] each.
    mini_start_float = MiniStartFloatFabLocation;

    /// Centered `FloatingActionButton`, floating at the bottom of the screen.
    ///
    /// To position a mini floating action button, use
    /// [`mini_center_float`](Self::mini_center_float) and set `FloatingActionButton.mini` to
    /// true.
    center_float = CenterFloatFabLocation;

    /// Centered `FloatingActionButton`, floating at the bottom of the screen,
    /// optimized for mini floating action buttons.
    ///
    /// This is intended to be used with `FloatingActionButton.mini` set to true,
    /// so that the floating action button appears to align horizontally with
    /// the locations [`mini_start_float`](Self::mini_start_float)
    /// and [`mini_end_float`](Self::mini_end_float).
    ///
    /// Compared to [`center_float`](Self::center_float), floating action
    /// buttons using this location will move vertically down
    /// by [`K_MINI_BUTTON_OFFSET_ADJUSTMENT`].
    mini_center_float = MiniCenterFloatFabLocation;

    /// End-aligned `FloatingActionButton`, floating at the bottom of the screen.
    ///
    /// This is the default alignment of `FloatingActionButton`s in Material applications.
    ///
    /// To align a floating action button with `CircleAvatar`s in the
    /// `ListTile.trailing` slots of `ListTile`s in a `ListView` in the `Scaffold.body`,
    /// use [`mini_end_float`](Self::mini_end_float) and set `FloatingActionButton.mini` to
    /// true.
    end_float = EndFloatFabLocation;

    /// End-aligned `FloatingActionButton`, floating at the bottom of the screen,
    /// optimized for mini floating action buttons.
    ///
    /// This is intended to be used with `FloatingActionButton.mini` set to true,
    /// so that the floating action button appears to align with `CircleAvatar`s
    /// in the `ListTile.trailing` slot of a `ListTile` in a `ListView` in the
    /// `Scaffold.body`.
    ///
    /// Compared to [`end_float`](Self::end_float), floating action
    /// buttons using this location will move horizontally _and_ vertically
    /// closer to the edges, by [`K_MINI_BUTTON_OFFSET_ADJUSTMENT`] each.
    mini_end_float = MiniEndFloatFabLocation;

    /// Start-aligned `FloatingActionButton`, floating over the
    /// `Scaffold.bottomNavigationBar` so that the center of the floating
    /// action button lines up with the top of the bottom navigation bar.
    ///
    /// To align a floating action button with `CircleAvatar`s in the
    /// `ListTile.leading` slots of `ListTile`s in a `ListView` in the `Scaffold.body`,
    /// use [`mini_start_docked`](Self::mini_start_docked) and set
    /// `FloatingActionButton.mini` to true.
    ///
    /// If the value of `Scaffold.bottomNavigationBar` is a `BottomAppBar`,
    /// the bottom app bar can include a "notch" in its shape that accommodates
    /// the overlapping floating action button.
    ///
    /// This is unlikely to be a useful location for apps that lack a bottom
    /// navigation bar.
    start_docked = StartDockedFabLocation;

    /// Start-aligned `FloatingActionButton`, floating over the
    /// `Scaffold.bottomNavigationBar` so that the center of the floating
    /// action button lines up with the top of the bottom navigation bar,
    /// optimized for mini floating action buttons.
    ///
    /// If the value of `Scaffold.bottomNavigationBar` is a `BottomAppBar`,
    /// the bottom app bar can include a "notch" in its shape that accommodates
    /// the overlapping floating action button.
    ///
    /// This is intended to be used with `FloatingActionButton.mini` set to true,
    /// so that the floating action button appears to align with `CircleAvatar`s
    /// in the `ListTile.leading` slot of a `ListTile` in a `ListView` in the
    /// `Scaffold.body`.
    ///
    /// This is unlikely to be a useful location for apps that lack a bottom
    /// navigation bar.
    mini_start_docked = MiniStartDockedFabLocation;

    /// Centered `FloatingActionButton`, floating over the
    /// `Scaffold.bottomNavigationBar` so that the center of the floating
    /// action button lines up with the top of the bottom navigation bar.
    ///
    /// If the value of `Scaffold.bottomNavigationBar` is a `BottomAppBar`,
    /// the bottom app bar can include a "notch" in its shape that accommodates
    /// the overlapping floating action button.
    ///
    /// This is unlikely to be a useful location for apps that lack a bottom
    /// navigation bar.
    center_docked = CenterDockedFabLocation;

    /// Centered `FloatingActionButton`, floating over the
    /// `Scaffold.bottomNavigationBar` so that the center of the floating
    /// action button lines up with the top of the bottom navigation bar;
    /// intended to be used with `FloatingActionButton.mini` set to true.
    ///
    /// If the value of `Scaffold.bottomNavigationBar` is a `BottomAppBar`,
    /// the bottom app bar can include a "notch" in its shape that accommodates
    /// the overlapping floating action button.
    ///
    /// This is unlikely to be a useful location for apps that lack a bottom
    /// navigation bar.
    mini_center_docked = MiniCenterDockedFabLocation;

    /// End-aligned `FloatingActionButton`, floating over the
    /// `Scaffold.bottomNavigationBar` so that the center of the floating
    /// action button lines up with the top of the bottom navigation bar.
    ///
    /// If the value of `Scaffold.bottomNavigationBar` is a `BottomAppBar`,
    /// the bottom app bar can include a "notch" in its shape that accommodates
    /// the overlapping floating action button.
    ///
    /// This is unlikely to be a useful location for apps that lack a bottom
    /// navigation bar.
    end_docked = EndDockedFabLocation;

    /// End-aligned `FloatingActionButton`, floating over the
    /// `Scaffold.bottomNavigationBar` so that the center of the floating
    /// action button lines up with the top of the bottom navigation bar,
    /// optimized for mini floating action buttons.
    ///
    /// To align a floating action button with `CircleAvatar`s in the
    /// `ListTile.trailing` slots of `ListTile`s in a `ListView` in the `Scaffold.body`,
    /// use [`mini_end_docked`](Self::mini_end_docked) and set `FloatingActionButton.mini` to
    /// true.
    ///
    /// If the value of `Scaffold.bottomNavigationBar` is a `BottomAppBar`,
    /// the bottom app bar can include a "notch" in its shape that accommodates
    /// the overlapping floating action button.
    ///
    /// This is intended to be used with `FloatingActionButton.mini` set to true,
    /// so that the floating action button appears to align with `CircleAvatar`s
    /// in the `ListTile.trailing` slot of a `ListTile` in a `ListView` in the
    /// `Scaffold.body`.
    ///
    /// This is unlikely to be a useful location for apps that lack a bottom
    /// navigation bar.
    mini_end_docked = MiniEndDockedFabLocation;

    /// End-aligned `FloatingActionButton`, floating over the
    /// `Scaffold.bottomNavigationBar` so that the floating
    /// action button lines up with the center of the bottom navigation bar.
    ///
    /// This is unlikely to be a useful location for apps which has a `BottomNavigationBar`
    /// or a non material 3 `BottomAppBar`.
    end_contained = EndContainedFabLocation;
}

/// A base trait that simplifies building [`FloatingActionButtonLocation`]s when
/// used with the mixins [`FabTopOffsetY`], [`FabFloatOffsetY`], [`FabDockedOffsetY`],
/// [`FabStartOffsetX`], [`FabCenterOffsetX`], [`FabEndOffsetX`], and
/// [`FabMiniOffsetAdjustment`].
///
/// A subclass of [`FloatingActionButtonLocation`] which implements its
/// [`get_offset`](FloatingActionButtonLocation::get_offset) method using three other methods:
/// [`get_offset_x`](Self::get_offset_x), [`get_offset_y`](Self::get_offset_y), and
/// [`is_mini`](Self::is_mini).
///
/// Different mixins on this class override different methods, so that combining
/// a set of mixins creates a floating action button location. A location implements the
/// mixins it uses and forwards [`get_offset_x`](Self::get_offset_x),
/// [`get_offset_y`](Self::get_offset_y) and [`is_mini`](Self::is_mini) to them, and its
/// [`FloatingActionButtonLocation::get_offset`] to [`StandardFabLocation::get_offset`].
///
/// For example: the location `<dyn FloatingActionButtonLocation>::mini_end_top()`
/// is based on a type that implements [`StandardFabLocation`]
/// with the mixins [`FabMiniOffsetAdjustment`], [`FabEndOffsetX`], and [`FabTopOffsetY`].
///
/// You can create your own implementation of [`StandardFabLocation`]
/// to implement a custom [`FloatingActionButtonLocation`].
pub trait StandardFabLocation: FloatingActionButtonLocation {
    /// Obtains the x-offset to place the `FloatingActionButton` based on the
    /// `Scaffold`'s layout.
    ///
    /// Used by [`get_offset`](Self::get_offset) to compute its x-coordinate.
    fn get_offset_x(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry, adjustment: f64) -> f64;

    /// Obtains the y-offset to place the `FloatingActionButton` based on the
    /// `Scaffold`'s layout.
    ///
    /// Used by [`get_offset`](Self::get_offset) to compute its y-coordinate.
    fn get_offset_y(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry, adjustment: f64) -> f64;

    /// A function returning whether this [`StandardFabLocation`] is optimized for
    /// mini `FloatingActionButton`s.
    fn is_mini(&self) -> bool {
        false
    }

    /// Dart's `StandardFabLocation.getOffset` override, which the implementing location's
    /// [`FloatingActionButtonLocation::get_offset`] calls.
    fn get_offset(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry) -> Offset {
        let adjustment = if StandardFabLocation::is_mini(self) {
            K_MINI_BUTTON_OFFSET_ADJUSTMENT
        } else {
            0.0
        };
        Offset::new(
            StandardFabLocation::get_offset_x(self, scaffold_geometry, adjustment),
            StandardFabLocation::get_offset_y(self, scaffold_geometry, adjustment),
        )
    }
}

/// Calculates x-offset for left-aligned [`FloatingActionButtonLocation`]s; Dart's
/// `StandardFabLocation._leftOffsetX`.
fn left_offset_x(scaffold_geometry: &ScaffoldPrelayoutGeometry, adjustment: f64) -> f64 {
    K_FLOATING_ACTION_BUTTON_MARGIN + scaffold_geometry.min_insets.left - adjustment
}

/// Calculates x-offset for right-aligned [`FloatingActionButtonLocation`]s; Dart's
/// `StandardFabLocation._rightOffsetX`.
fn right_offset_x(scaffold_geometry: &ScaffoldPrelayoutGeometry, adjustment: f64) -> f64 {
    scaffold_geometry.scaffold_size.width()
        - K_FLOATING_ACTION_BUTTON_MARGIN
        - scaffold_geometry.min_insets.right
        - scaffold_geometry.floating_action_button_size.width()
        + adjustment
}

/// Mixin for a "top" floating action button location, such as
/// `<dyn FloatingActionButtonLocation>::start_top()`.
///
/// The `adjustment`, typically [`K_MINI_BUTTON_OFFSET_ADJUSTMENT`], is ignored in the
/// Y axis of "top" positions. For "top" positions, the X offset is adjusted to
/// move closer to the edge of the screen. This is so that a minified floating
/// action button appears to align with `CircleAvatar`s in the
/// `ListTile.leading` slot of a `ListTile` in a `ListView` in the
/// `Scaffold.body`.
pub trait FabTopOffsetY: StandardFabLocation {
    /// Calculates y-offset for [`FloatingActionButtonLocation`]s floating over
    /// the transition between the `Scaffold.appBar` and the `Scaffold.body`.
    fn get_offset_y(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry, adjustment: f64) -> f64 {
        let _ = adjustment;
        if scaffold_geometry.content_top > scaffold_geometry.min_view_padding.top {
            let fab_half_height = scaffold_geometry.floating_action_button_size.height() / 2.0;
            return scaffold_geometry.content_top - fab_half_height;
        }
        // Otherwise, ensure we are placed within the bounds of a safe area.
        scaffold_geometry.min_view_padding.top
    }
}

/// Mixin for a "float" floating action button location, such as
/// `<dyn FloatingActionButtonLocation>::center_float()`.
pub trait FabFloatOffsetY: StandardFabLocation {
    /// Calculates y-offset for [`FloatingActionButtonLocation`]s floating at
    /// the bottom of the screen.
    fn get_offset_y(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry, adjustment: f64) -> f64 {
        let content_bottom = scaffold_geometry.content_bottom;
        let bottom_content_height = scaffold_geometry.scaffold_size.height() - content_bottom;
        let bottom_sheet_height = scaffold_geometry.bottom_sheet_size.height();
        let fab_height = scaffold_geometry.floating_action_button_size.height();
        let snack_bar_height = scaffold_geometry.snack_bar_size.height();
        let safe_margin = f64::max(
            K_FLOATING_ACTION_BUTTON_MARGIN,
            scaffold_geometry.min_view_padding.bottom - bottom_content_height
                + K_FLOATING_ACTION_BUTTON_MARGIN,
        );

        let mut fab_y = content_bottom - fab_height - safe_margin;
        if snack_bar_height > 0.0 {
            fab_y = f64::min(
                fab_y,
                content_bottom - snack_bar_height - fab_height - K_FLOATING_ACTION_BUTTON_MARGIN,
            );
        }
        if bottom_sheet_height > 0.0 {
            fab_y = f64::min(
                fab_y,
                content_bottom - bottom_sheet_height - fab_height / 2.0,
            );
        }
        fab_y + adjustment
    }
}

/// Mixin for a "docked" floating action button location, such as
/// `<dyn FloatingActionButtonLocation>::end_docked()`.
pub trait FabDockedOffsetY: StandardFabLocation {
    /// Calculates y-offset for [`FloatingActionButtonLocation`]s floating over the
    /// `Scaffold.bottomNavigationBar` so that the center of the floating
    /// action button lines up with the top of the bottom navigation bar.
    fn get_offset_y(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry, adjustment: f64) -> f64 {
        let _ = adjustment;
        let content_bottom = scaffold_geometry.content_bottom;
        let content_margin = scaffold_geometry.scaffold_size.height() - content_bottom;
        let bottom_view_padding = scaffold_geometry.min_view_padding.bottom;
        let bottom_sheet_height = scaffold_geometry.bottom_sheet_size.height();
        let fab_height = scaffold_geometry.floating_action_button_size.height();
        let snack_bar_height = scaffold_geometry.snack_bar_size.height();
        let bottom_min_inset = scaffold_geometry.min_insets.bottom;

        let safe_margin = if content_margin > bottom_min_inset + fab_height / 2.0 {
            // If contentMargin is higher than bottomMinInset enough to display the
            // FAB without clipping, don't provide a margin
            0.0
        } else if bottom_min_inset == 0.0 {
            // If bottomMinInset is zero(the software keyboard is not on the screen)
            // provide bottomViewPadding as margin
            bottom_view_padding
        } else {
            // Provide a margin that would shift the FAB enough so that it stays away
            // from the keyboard
            fab_height / 2.0 + K_FLOATING_ACTION_BUTTON_MARGIN
        };

        let mut fab_y = content_bottom - fab_height / 2.0 - safe_margin;
        // The FAB should sit with a margin between it and the snack bar.
        if snack_bar_height > 0.0 {
            fab_y = f64::min(
                fab_y,
                content_bottom - snack_bar_height - fab_height - K_FLOATING_ACTION_BUTTON_MARGIN,
            );
        }
        // The FAB should sit with its center in front of the top of the bottom sheet.
        if bottom_sheet_height > 0.0 {
            fab_y = f64::min(
                fab_y,
                content_bottom - bottom_sheet_height - fab_height / 2.0,
            );
        }
        let max_fab_y = scaffold_geometry.scaffold_size.height() - fab_height - safe_margin;
        f64::min(max_fab_y, fab_y)
    }
}

/// Mixin for a "contained" floating action button location, such as
/// `<dyn FloatingActionButtonLocation>::end_contained()`.
pub trait FabContainedOffsetY: StandardFabLocation {
    /// Calculates y-offset for [`FloatingActionButtonLocation`]s floating over the
    /// `Scaffold.bottomNavigationBar` so that the center of the floating
    /// action button lines up with the center of the bottom navigation bar.
    fn get_offset_y(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry, adjustment: f64) -> f64 {
        let _ = adjustment;
        let content_bottom = scaffold_geometry.content_bottom;
        let content_margin = scaffold_geometry.scaffold_size.height() - content_bottom;
        let bottom_view_padding = scaffold_geometry.min_view_padding.bottom;
        let fab_height = scaffold_geometry.floating_action_button_size.height();

        let safe_margin = if content_margin > bottom_view_padding + fab_height {
            // If contentMargin is higher than bottomViewPadding enough to display the
            // FAB without clipping, don't provide a margin
            0.0
        } else {
            bottom_view_padding
        };

        // This is to compute the distance between the content bottom to the top edge
        // of the floating action button. This can be negative if content margin is
        // too small.
        let content_bottom_to_fab_top = (content_margin - bottom_view_padding - fab_height) / 2.0;
        let fab_y = content_bottom + content_bottom_to_fab_top;
        let max_fab_y = scaffold_geometry.scaffold_size.height() - fab_height - safe_margin;

        f64::min(max_fab_y, fab_y)
    }
}

/// Mixin for a "start" floating action button location, such as
/// `<dyn FloatingActionButtonLocation>::start_top()`.
pub trait FabStartOffsetX: StandardFabLocation {
    /// Calculates x-offset for start-aligned [`FloatingActionButtonLocation`]s.
    fn get_offset_x(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry, adjustment: f64) -> f64 {
        match scaffold_geometry.text_direction {
            TextDirection::Rtl => right_offset_x(scaffold_geometry, adjustment),
            TextDirection::Ltr => left_offset_x(scaffold_geometry, adjustment),
        }
    }
}

/// Mixin for a "center" floating action button location, such as
/// `<dyn FloatingActionButtonLocation>::center_float()`.
pub trait FabCenterOffsetX: StandardFabLocation {
    /// Calculates x-offset for center-aligned [`FloatingActionButtonLocation`]s.
    fn get_offset_x(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry, adjustment: f64) -> f64 {
        let _ = adjustment;
        (scaffold_geometry.scaffold_size.width()
            - scaffold_geometry.floating_action_button_size.width())
            / 2.0
    }
}

/// Mixin for an "end" floating action button location, such as
/// `<dyn FloatingActionButtonLocation>::end_docked()`.
pub trait FabEndOffsetX: StandardFabLocation {
    /// Calculates x-offset for end-aligned [`FloatingActionButtonLocation`]s.
    fn get_offset_x(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry, adjustment: f64) -> f64 {
        match scaffold_geometry.text_direction {
            TextDirection::Rtl => left_offset_x(scaffold_geometry, adjustment),
            TextDirection::Ltr => right_offset_x(scaffold_geometry, adjustment),
        }
    }
}

/// Mixin for a "mini" floating action button location, such as
/// `<dyn FloatingActionButtonLocation>::mini_start_top()`.
pub trait FabMiniOffsetAdjustment: StandardFabLocation {
    /// Dart's `isMini` override.
    fn is_mini(&self) -> bool {
        true
    }
}

/// Declares one of Dart's private standard locations: a unit struct that extends
/// [`StandardFabLocation`] with an x mixin and a y mixin (and, when `mini`, with
/// [`FabMiniOffsetAdjustment`]), and whose `Debug` is Dart's `toString`.
macro_rules! standard_fab_location {
    ($name:ident, $label:literal, $x:ident, $y:ident $(, $mini:ident)?) => {
        #[doc = concat!("Dart's `_", stringify!($name), "`.")]
        struct $name;

        impl FloatingActionButtonLocation for $name {
            fn get_offset(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry) -> Offset {
                StandardFabLocation::get_offset(self, scaffold_geometry)
            }
        }

        impl StandardFabLocation for $name {
            fn get_offset_x(
                &self,
                scaffold_geometry: &ScaffoldPrelayoutGeometry,
                adjustment: f64,
            ) -> f64 {
                $x::get_offset_x(self, scaffold_geometry, adjustment)
            }

            fn get_offset_y(
                &self,
                scaffold_geometry: &ScaffoldPrelayoutGeometry,
                adjustment: f64,
            ) -> f64 {
                $y::get_offset_y(self, scaffold_geometry, adjustment)
            }

            $(
                fn is_mini(&self) -> bool {
                    $mini::is_mini(self)
                }
            )?
        }

        $(impl $mini for $name {})?
        impl $x for $name {}
        impl $y for $name {}

        impl Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!("FloatingActionButtonLocation.", $label))
            }
        }
    };
}

standard_fab_location!(
    StartTopFabLocation,
    "startTop",
    FabStartOffsetX,
    FabTopOffsetY
);
standard_fab_location!(
    MiniStartTopFabLocation,
    "miniStartTop",
    FabStartOffsetX,
    FabTopOffsetY,
    FabMiniOffsetAdjustment
);
standard_fab_location!(
    CenterTopFabLocation,
    "centerTop",
    FabCenterOffsetX,
    FabTopOffsetY
);
standard_fab_location!(
    MiniCenterTopFabLocation,
    "miniCenterTop",
    FabCenterOffsetX,
    FabTopOffsetY,
    FabMiniOffsetAdjustment
);
standard_fab_location!(EndTopFabLocation, "endTop", FabEndOffsetX, FabTopOffsetY);
standard_fab_location!(
    MiniEndTopFabLocation,
    "miniEndTop",
    FabEndOffsetX,
    FabTopOffsetY,
    FabMiniOffsetAdjustment
);
standard_fab_location!(
    StartFloatFabLocation,
    "startFloat",
    FabStartOffsetX,
    FabFloatOffsetY
);
standard_fab_location!(
    MiniStartFloatFabLocation,
    "miniStartFloat",
    FabStartOffsetX,
    FabFloatOffsetY,
    FabMiniOffsetAdjustment
);
standard_fab_location!(
    CenterFloatFabLocation,
    "centerFloat",
    FabCenterOffsetX,
    FabFloatOffsetY
);
standard_fab_location!(
    MiniCenterFloatFabLocation,
    "miniCenterFloat",
    FabCenterOffsetX,
    FabFloatOffsetY,
    FabMiniOffsetAdjustment
);
standard_fab_location!(
    EndFloatFabLocation,
    "endFloat",
    FabEndOffsetX,
    FabFloatOffsetY
);
standard_fab_location!(
    MiniEndFloatFabLocation,
    "miniEndFloat",
    FabEndOffsetX,
    FabFloatOffsetY,
    FabMiniOffsetAdjustment
);
standard_fab_location!(
    StartDockedFabLocation,
    "startDocked",
    FabStartOffsetX,
    FabDockedOffsetY
);
standard_fab_location!(
    MiniStartDockedFabLocation,
    "miniStartDocked",
    FabStartOffsetX,
    FabDockedOffsetY,
    FabMiniOffsetAdjustment
);
standard_fab_location!(
    CenterDockedFabLocation,
    "centerDocked",
    FabCenterOffsetX,
    FabDockedOffsetY
);
standard_fab_location!(
    MiniCenterDockedFabLocation,
    "miniCenterDocked",
    FabCenterOffsetX,
    FabDockedOffsetY,
    FabMiniOffsetAdjustment
);
standard_fab_location!(
    EndDockedFabLocation,
    "endDocked",
    FabEndOffsetX,
    FabDockedOffsetY
);
standard_fab_location!(
    MiniEndDockedFabLocation,
    "miniEndDocked",
    FabEndOffsetX,
    FabDockedOffsetY,
    FabMiniOffsetAdjustment
);
standard_fab_location!(
    EndContainedFabLocation,
    "endContained",
    FabEndOffsetX,
    FabContainedOffsetY
);

/// Provider of animations to move the `FloatingActionButton` between
/// [`FloatingActionButtonLocation`]s.
///
/// The `Scaffold` uses `Scaffold.floatingActionButtonAnimator` to define:
///
///  * The [`Offset`] of the `FloatingActionButton` between the old and new
///    [`FloatingActionButtonLocation`]s as part of the transition animation.
///  * An `Animation` to scale the `FloatingActionButton` during the transition.
///  * An `Animation` to rotate the `FloatingActionButton` during the transition.
///  * Where to start a new animation from if an animation is interrupted.
///
/// See also:
///
///  * `FloatingActionButton`, which is a circular button typically shown in the
///    bottom right corner of the app.
///  * [`FloatingActionButtonLocation`], which the `Scaffold` uses to place the
///    `Scaffold.floatingActionButton` within the `Scaffold`'s layout.
pub trait FloatingActionButtonAnimator: Debug + 'static {
    /// Gets the `FloatingActionButton`'s position relative to the origin of the
    /// `Scaffold` based on `progress`.
    ///
    /// `begin` is the [`Offset`] provided by the previous
    /// [`FloatingActionButtonLocation`].
    ///
    /// `end` is the [`Offset`] provided by the new
    /// [`FloatingActionButtonLocation`].
    ///
    /// `progress` is the current progress of the transition animation.
    /// When `progress` is 0.0, the returned [`Offset`] should be equal to `begin`.
    /// when `progress` is 1.0, the returned [`Offset`] should be equal to `end`.
    fn get_offset(&self, begin: Offset, end: Offset, progress: f64) -> Offset;

    /// Animates the scale of the `FloatingActionButton`.
    ///
    /// The animation should both start and end with a value of 1.0.
    ///
    /// For example, to create an animation that linearly scales out and then back in,
    /// you could join animations that pass each other with a `TrainHoppingAnimation`.
    fn get_scale_animation(&self, app: &mut App, parent: AnimationRef<f64>) -> AnimationRef<f64>;

    /// Animates the rotation of `Scaffold.floatingActionButton`.
    ///
    /// The animation should both start and end with a value of 0.0 or 1.0.
    ///
    /// The animation values are a fraction of a full circle, with 0.0 and 1.0
    /// corresponding to 0 and 360 degrees, while 0.5 corresponds to 180 degrees.
    ///
    /// For example, to create a rotation animation that rotates the
    /// `FloatingActionButton` through a full circle, drive a `Tween` from 0.0 to 1.0 with
    /// `parent`.
    fn get_rotation_animation(&self, app: &mut App, parent: AnimationRef<f64>)
    -> AnimationRef<f64>;

    /// Gets the progress value to restart a motion animation from when the animation is
    /// interrupted.
    ///
    /// `previous_value` is the value of the animation before it was interrupted.
    ///
    /// The restart of the animation will affect all three parts of the motion animation:
    /// offset animation, scale animation, and rotation animation.
    ///
    /// An interruption triggers if the `Scaffold` is given a new
    /// [`FloatingActionButtonLocation`] while it is still animating a transition between two
    /// previous [`FloatingActionButtonLocation`]s.
    ///
    /// A sensible default is usually 0.0, which is the same as restarting
    /// the animation from the beginning, regardless of the original state of the animation.
    fn get_animation_restart(&self, previous_value: f64) -> f64 {
        let _ = previous_value;
        0.0
    }
}

/// A shared [`FloatingActionButtonAnimator`], compared by identity as Dart compares its
/// animators.
pub type FloatingActionButtonAnimatorRef = Rc<dyn FloatingActionButtonAnimator>;

impl dyn FloatingActionButtonAnimator {
    /// Moves the `FloatingActionButton` by scaling out and then in at a new
    /// [`FloatingActionButtonLocation`].
    ///
    /// This animator shrinks the `FloatingActionButton` down until it disappears, then
    /// grows it back to full size at its new [`FloatingActionButtonLocation`].
    ///
    /// This is the default `FloatingActionButton` motion animation.
    pub fn scaling() -> FloatingActionButtonAnimatorRef {
        thread_local! {
            static ANIMATOR: FloatingActionButtonAnimatorRef = Rc::new(ScalingFabMotionAnimator);
        }
        ANIMATOR.with(Rc::clone)
    }

    /// Creates an instance of [`FloatingActionButtonAnimator`] where the
    /// `FloatingActionButton` does not animate on entrance and exit when
    /// [`FloatingActionButtonLocation`] is shown or hidden and when transitioning between
    /// [`FloatingActionButtonLocation`]s.
    pub fn no_animation() -> FloatingActionButtonAnimatorRef {
        thread_local! {
            static ANIMATOR: FloatingActionButtonAnimatorRef =
                Rc::new(NoAnimationFabMotionAnimator);
        }
        ANIMATOR.with(Rc::clone)
    }
}

/// Dart's `_ScalingFabMotionAnimator`.
#[derive(Debug)]
struct ScalingFabMotionAnimator;

/// Dart's `static final` tweens of `_ScalingFabMotionAnimator`, kept per [`App`] and made on
/// first use. They are shared by every animation the animator returns, so the `App` keeps them for
/// its life.
#[derive(Default)]
struct ScalingFabMotionAnimatorStatics {
    rotation_tween: Option<Ref<Tween<f64>>>,
    threshold_center_tween: Option<Ref<CurveTween>>,
}

impl ScalingFabMotionAnimator {
    // Because we only see the last half of the rotation tween,
    // it needs to go twice as far.
    fn rotation_tween(app: &mut App) -> Ref<Tween<f64>> {
        let statics = app.singleton::<ScalingFabMotionAnimatorStatics>();
        if let Some(rotation_tween) = app.get(statics).rotation_tween.clone() {
            return rotation_tween;
        }
        let rotation_tween = Tween::new(
            app,
            Some(1.0 - K_FLOATING_ACTION_BUTTON_TURN_INTERVAL * 2.0),
            Some(1.0),
        );
        app.get_mut(statics).rotation_tween = Some(rotation_tween.clone());
        rotation_tween
    }

    fn threshold_center_tween(app: &mut App) -> Ref<CurveTween> {
        let statics = app.singleton::<ScalingFabMotionAnimatorStatics>();
        if let Some(threshold_center_tween) = app.get(statics).threshold_center_tween.clone() {
            return threshold_center_tween;
        }
        let threshold_center_tween = CurveTween::new(app, Rc::new(Threshold::new(0.5)));
        app.get_mut(statics).threshold_center_tween = Some(threshold_center_tween.clone());
        threshold_center_tween
    }
}

impl FloatingActionButtonAnimator for ScalingFabMotionAnimator {
    fn get_offset(&self, begin: Offset, end: Offset, progress: f64) -> Offset {
        if progress < 0.5 { begin } else { end }
    }

    fn get_scale_animation(&self, app: &mut App, parent: AnimationRef<f64>) -> AnimationRef<f64> {
        // Animate the scale down from 1 to 0 in the first half of the animation
        // then from 0 back to 1 in the second half.
        let curve: Rc<dyn Curve> = Rc::new(Interval::new(0.5, 1.0, Curves::ease()));
        let flipped_tween = CurveTween::new(app, Rc::new(Rc::clone(&curve).flipped()));
        let flipped = parent.drive(app, flipped_tween);
        let first = ReverseAnimation::new(app, flipped).into_erased();
        let curve_tween = CurveTween::new(app, curve);
        let next = parent.drive(app, curve_tween);
        app.create(AnimationSwap::new(first, next, parent, 0.5))
            .into_erased()
    }

    fn get_rotation_animation(
        &self,
        app: &mut App,
        parent: AnimationRef<f64>,
    ) -> AnimationRef<f64> {
        // This rotation will turn on the way in, but not on the way out.
        let rotation_tween = ScalingFabMotionAnimator::rotation_tween(app);
        let first = parent.drive(app, rotation_tween);
        let threshold_center_tween = ScalingFabMotionAnimator::threshold_center_tween(app);
        let threshold = parent.drive(app, threshold_center_tween);
        let next = ReverseAnimation::new(app, threshold).into_erased();
        app.create(AnimationSwap::new(first, next, parent, 0.5))
            .into_erased()
    }

    // If the animation was just starting, we'll continue from where we left off.
    // If the animation was finishing, we'll treat it as if we were starting at that point in
    // reverse. This avoids a size jump during the animation.
    fn get_animation_restart(&self, previous_value: f64) -> f64 {
        f64::min(1.0 - previous_value, previous_value)
    }
}

/// Dart's `_NoAnimationFabMotionAnimator`.
#[derive(Debug)]
struct NoAnimationFabMotionAnimator;

/// Dart's `const AlwaysStoppedAnimation<double>(1.0)`, which Dart canonicalizes into one
/// object, kept per [`App`] and made on first use; the `App` keeps it for its life.
#[derive(Default)]
struct AlwaysStoppedAtOne {
    animation: Option<Ref<AlwaysStoppedAnimation<f64>>>,
}

impl AlwaysStoppedAtOne {
    fn instance(app: &mut App) -> AnimationRef<f64> {
        let statics = app.singleton::<AlwaysStoppedAtOne>();
        if let Some(animation) = app.get(statics).animation.clone() {
            return animation.into_erased();
        }
        let animation = app.create(AlwaysStoppedAnimation::new(1.0));
        app.get_mut(statics).animation = Some(animation.clone());
        animation.into_erased()
    }
}

impl FloatingActionButtonAnimator for NoAnimationFabMotionAnimator {
    fn get_offset(&self, begin: Offset, end: Offset, progress: f64) -> Offset {
        let _ = (begin, progress);
        end
    }

    fn get_rotation_animation(
        &self,
        app: &mut App,
        parent: AnimationRef<f64>,
    ) -> AnimationRef<f64> {
        let _ = parent;
        AlwaysStoppedAtOne::instance(app)
    }

    fn get_scale_animation(&self, app: &mut App, parent: AnimationRef<f64>) -> AnimationRef<f64> {
        let _ = parent;
        AlwaysStoppedAtOne::instance(app)
    }
}

/// An animation that swaps from one animation to the next when the
/// [`parent`](Self::parent) passes [`swap_threshold`](Self::swap_threshold).
///
/// The value of this animation is the value of `first` when `parent.value` <
/// [`swap_threshold`](Self::swap_threshold) and the value of `next` otherwise.
struct AnimationSwap<T: Clone + PartialEq + 'static> {
    first: AnimationRef<T>,
    next: AnimationRef<T>,
    parent: AnimationRef<f64>,
    swap_threshold: f64,
    last_status: Option<AnimationStatus>,
    last_value: Option<T>,
    lazy_listener: AnimationLazyListenerData,
    local_listeners: AnimationLocalListenersData,
    local_status_listeners: AnimationLocalStatusListenersData,
}

impl<T: Clone + PartialEq + 'static> AnimationSwap<T> {
    /// Creates an [`AnimationSwap`].
    ///
    /// Either argument can be an [`AnimationSwap`] itself to combine multiple
    /// animations.
    fn new(
        first: AnimationRef<T>,
        next: AnimationRef<T>,
        parent: AnimationRef<f64>,
        swap_threshold: f64,
    ) -> AnimationSwap<T> {
        AnimationSwap {
            first,
            next,
            parent,
            swap_threshold,
            last_status: None,
            last_value: None,
            lazy_listener: AnimationLazyListenerData::new(),
            local_listeners: AnimationLocalListenersData::new(),
            local_status_listeners: AnimationLocalStatusListenersData::new(),
        }
    }
}

impl<T: Clone + PartialEq + 'static> AnimationLazyListenerMixin for AnimationSwap<T> {
    fn lazy_listener_data(self: Handle<Self>, app: &App) -> &AnimationLazyListenerData {
        &app.get(self).lazy_listener
    }

    fn lazy_listener_data_mut(self: Handle<Self>, app: &mut App) -> &mut AnimationLazyListenerData {
        &mut app.get_mut(self).lazy_listener
    }

    fn did_start_listening(self: Handle<Self>, app: &mut App) {
        compound_did_start_listening::<AnimationSwap<T>, T>(self, app)
    }

    fn did_stop_listening(self: Handle<Self>, app: &mut App) {
        compound_did_stop_listening::<AnimationSwap<T>, T>(self, app)
    }
}

impl<T: Clone + PartialEq + 'static> AnimationLocalListenersMixin for AnimationSwap<T> {
    fn local_listeners_data(self: Handle<Self>, app: &App) -> &AnimationLocalListenersData {
        &app.get(self).local_listeners
    }

    fn local_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalListenersData {
        &mut app.get_mut(self).local_listeners
    }

    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl<T: Clone + PartialEq + 'static> AnimationLocalStatusListenersMixin for AnimationSwap<T> {
    fn local_status_listeners_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AnimationLocalStatusListenersData {
        &app.get(self).local_status_listeners
    }

    fn local_status_listeners_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AnimationLocalStatusListenersData {
        &mut app.get_mut(self).local_status_listeners
    }

    fn did_register_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_register_listener(self, app)
    }

    fn did_unregister_listener(self: Handle<Self>, app: &mut App) {
        AnimationLazyListenerMixin::did_unregister_listener(self, app)
    }
}

impl<T: Clone + PartialEq + 'static> CompoundAnimation<T> for AnimationSwap<T> {
    fn first(self: Handle<Self>, app: &App) -> AnyAnimation<T> {
        app.get(self).first.handle()
    }

    fn next(self: Handle<Self>, app: &App) -> AnyAnimation<T> {
        app.get(self).next.handle()
    }

    fn last_status(self: Handle<Self>, app: &App) -> Option<AnimationStatus> {
        app.get(self).last_status
    }

    fn last_status_mut(self: Handle<Self>, app: &mut App) -> &mut Option<AnimationStatus> {
        &mut app.get_mut(self).last_status
    }

    fn last_value(self: Handle<Self>, app: &App) -> &Option<T> {
        &app.get(self).last_value
    }

    fn last_value_mut(self: Handle<Self>, app: &mut App) -> &mut Option<T> {
        &mut app.get_mut(self).last_value
    }
}

// Dart: `class _AnimationSwap<T> extends CompoundAnimation<T>`.
impl<T: Clone + PartialEq + 'static> Animation<T> for AnimationSwap<T> {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        AnimationLocalListenersMixin::add_listener(self, app, listener)
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        AnimationLocalListenersMixin::remove_listener(self, app, listener)
    }

    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        AnimationLocalStatusListenersMixin::add_status_listener(self, app, listener)
    }

    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        AnimationLocalStatusListenersMixin::remove_status_listener(self, app, listener)
    }

    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        CompoundAnimation::status(self, app)
    }

    fn value(self: Handle<Self>, app: &App) -> T {
        let animation = app.get(self);
        if animation.parent.value(app) < animation.swap_threshold {
            animation.first.value(app)
        } else {
            animation.next.value(app)
        }
    }
}

#[cfg(test)]
mod tests {
    //! The cases of `test/floating_action_button_location_test.dart`.
    //!
    //! Dart's `_singleFabScaffold` and `buildTest` put a `BottomNavigationBar` in the scaffold,
    //! which is not ported. A case whose asserted values do not depend on it is ported without
    //! it, and says so; a case whose values depend on its height waits for it (listed at the
    //! end of this module).
    use waft_widgets::Text;

    use std::cell::RefCell;

    use waft_animation::AlwaysStoppedAnimation;
    use waft_embedder::{Rect, Size};
    use waft_foundation::Listenable;
    use waft_painting::EdgeInsets;
    use waft_rendering::CustomPainter;
    use waft_scheduler::SchedulerBinding;
    use waft_widgets::{
        AnyElement, BuildContext, Builder, Center, CustomPaint, Directionality, GlobalKey,
        IntoWidget, KeyRef, MediaQuery, MediaQueryData, PreferredSizeWidgetRef, RotationTransition,
        SizedBox, State, StateData, StatefulWidget, WidgetRef, WidgetsBinding, downcast_widget,
    };

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::app_bar::AppBar;
    use crate::floating_action_button::FloatingActionButton;
    use crate::material_localizations::tests::us_english;
    use crate::scaffold::{
        Scaffold, ScaffoldGeometry, ScaffoldGeometryNotifier, ScaffoldMessenger,
    };
    use crate::snack_bar::SnackBar;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    /// Dart's `const FloatingActionButton(onPressed: null, child: Text('1'))`.
    fn default_fab() -> FloatingActionButton {
        FloatingActionButton::new(None).child(Text::new("1"))
    }

    /// Dart's `_buildFrame`.
    fn build_frame(
        fab: Option<FloatingActionButton>,
        location: Option<FloatingActionButtonLocationRef>,
        listener: Option<WidgetRef>,
        text_direction: TextDirection,
        view_insets: EdgeInsets,
        bab: Option<WidgetRef>,
    ) -> WidgetRef {
        let mut scaffold = Scaffold::new().app_bar(PreferredSizeWidgetRef::new(
            AppBar::new().title(Text::new("FabLocation Test")),
        ));
        if let Some(location) = location {
            scaffold = scaffold.floating_action_button_location(location);
        }
        if let Some(fab) = fab {
            scaffold = scaffold.floating_action_button(fab);
        }
        if let Some(bab) = bab {
            scaffold = scaffold.bottom_navigation_bar(bab);
        }
        if let Some(listener) = listener {
            scaffold = scaffold.body(listener);
        }
        us_english(Directionality::new(
            text_direction,
            MediaQuery::new(MediaQueryData::new().view_insets(view_insets), scaffold).into_widget(),
        ))
    }

    /// `_buildFrame(location:)` with Dart's other defaults.
    fn frame(location: Option<FloatingActionButtonLocationRef>) -> WidgetRef {
        build_frame(
            Some(default_fab()),
            location,
            None,
            TextDirection::Ltr,
            EdgeInsets::only(0.0, 0.0, 0.0, 200.0),
            None,
        )
    }

    fn fab_box(tester: &WidgetTester) -> waft_rendering::AnyRenderBox {
        let found = tester.find_by_type::<FloatingActionButton>();
        assert_eq!(found.len(), 1, "one FloatingActionButton");
        tester.render_box_of(found[0])
    }

    /// `tester.getCenter(find.byType(FloatingActionButton))`.
    fn fab_center(tester: &WidgetTester) -> Offset {
        let render = fab_box(tester);
        let app = tester.cell.borrow();
        let size = render.size(&app);
        render.local_to_global(&app, size.center(Offset::ZERO), None)
    }

    /// `tester.getTopLeft(find.byType(FloatingActionButton))`.
    fn fab_top_left(tester: &WidgetTester) -> Offset {
        let render = fab_box(tester);
        let app = tester.cell.borrow();
        render.local_to_global(&app, Offset::ZERO, None)
    }

    /// `tester.getTopRight(find.byType(FloatingActionButton))`.
    fn fab_top_right(tester: &WidgetTester) -> Offset {
        let render = fab_box(tester);
        let app = tester.cell.borrow();
        let size = render.size(&app);
        render.local_to_global(&app, Offset::new(size.width(), 0.0), None)
    }

    /// `tester.getRect(find.byKey(key))`.
    fn rect_of(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        let render = tester.render_object(key).as_box().expect("a render box");
        let app = tester.cell.borrow();
        let size = render.size(&app);
        let top_left = render.local_to_global(&app, Offset::ZERO, None);
        let bottom_right =
            render.local_to_global(&app, Offset::new(size.width(), size.height()), None);
        Rect::from_points(top_left, bottom_right)
    }

    fn transient_callback_count(tester: &WidgetTester) -> usize {
        SchedulerBinding::transient_callback_count(&mut tester.cell.borrow_mut())
    }

    /// The tolerance of a position read through `local_to_global` across the FAB's
    /// `RotationTransition`: `Matrix4` is valo's f32 matrix (waft-gestures' PORTING.md), so a
    /// rotated position is exact only to f32 where Dart's f64 one is exact.
    const POSITION_TOLERANCE: f64 = 1e-4;

    /// Dart's `expect(x, y)` on a coordinate read through the FAB's transitions.
    fn assert_coordinate_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < POSITION_TOLERANCE,
            "{actual} is not {expected}"
        );
    }

    /// Dart's `expect(offset, Offset(..))` on a point read through the FAB's transitions.
    fn assert_offset_eq(actual: Offset, expected: Offset) {
        assert!(
            (actual.dx() - expected.dx()).abs() < POSITION_TOLERANCE
                && (actual.dy() - expected.dy()).abs() < POSITION_TOLERANCE,
            "{actual:?} is not {expected:?}"
        );
    }

    /// Dart's `rectMoreOrLessEquals`, within [`POSITION_TOLERANCE`] for the reason there.
    fn assert_rect_more_or_less_equals(actual: Rect, expected: Rect) {
        let close = |a: f64, b: f64| (a - b).abs() < POSITION_TOLERANCE;
        assert!(
            close(actual.left, expected.left)
                && close(actual.top, expected.top)
                && close(actual.right, expected.right)
                && close(actual.bottom, expected.bottom),
            "{actual:?} is not {expected:?}"
        );
    }

    /// Dart's `_StartTopFloatingActionButtonLocation`.
    #[derive(Debug)]
    struct StartTopFloatingActionButtonLocation;

    impl FloatingActionButtonLocation for StartTopFloatingActionButtonLocation {
        fn get_offset(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry) -> Offset {
            let fab_x = match scaffold_geometry.text_direction {
                TextDirection::Rtl => {
                    let start_padding =
                        K_FLOATING_ACTION_BUTTON_MARGIN + scaffold_geometry.min_insets.right;
                    scaffold_geometry.scaffold_size.width()
                        - scaffold_geometry.floating_action_button_size.width()
                        - start_padding
                }
                TextDirection::Ltr => {
                    K_FLOATING_ACTION_BUTTON_MARGIN + scaffold_geometry.min_insets.left
                }
            };
            let fab_y = scaffold_geometry.content_top
                - (scaffold_geometry.floating_action_button_size.height() / 2.0);
            Offset::new(fab_x, fab_y)
        }
    }

    /// Dart's `_NoScalingFabAnimator`.
    #[derive(Debug)]
    struct NoScalingFabAnimator;

    impl FloatingActionButtonAnimator for NoScalingFabAnimator {
        fn get_offset(&self, begin: Offset, end: Offset, progress: f64) -> Offset {
            if progress < 0.5 { begin } else { end }
        }

        fn get_scale_animation(
            &self,
            app: &mut App,
            _parent: AnimationRef<f64>,
        ) -> AnimationRef<f64> {
            app.create(AlwaysStoppedAnimation::new(1.0))
                .into_erased::<AnyAnimation<f64>>()
        }

        fn get_rotation_animation(
            &self,
            app: &mut App,
            _parent: AnimationRef<f64>,
        ) -> AnimationRef<f64> {
            app.create(AlwaysStoppedAnimation::new(1.0))
                .into_erased::<AnyAnimation<f64>>()
        }
    }

    // Basic floating action button locations

    // 'still animates motion when the floating action button is null'.
    #[test]
    fn a_null_floating_action_button_still_animates_its_motion() {
        test_widgets(|tester| {
            let null_frame = |location| {
                build_frame(
                    None,
                    location,
                    None,
                    TextDirection::Ltr,
                    EdgeInsets::only(0.0, 0.0, 0.0, 200.0),
                    None,
                )
            };
            tester.pump_widget(null_frame(None));

            assert!(tester.find_by_type::<FloatingActionButton>().is_empty());
            assert_eq!(transient_callback_count(tester), 0);

            tester.pump_widget(null_frame(Some(
                <dyn FloatingActionButtonLocation>::end_float(),
            )));

            assert!(tester.find_by_type::<FloatingActionButton>().is_empty());
            assert!(transient_callback_count(tester) > 0);

            tester.pump_widget(null_frame(Some(
                <dyn FloatingActionButtonLocation>::center_float(),
            )));

            assert!(tester.find_by_type::<FloatingActionButton>().is_empty());
            assert!(transient_callback_count(tester) > 0);
        });
    }

    // 'moves fab from center to end and back'.
    #[test]
    fn the_fab_moves_from_center_to_end_and_back() {
        test_widgets(|tester| {
            tester.pump_widget(frame(Some(<dyn FloatingActionButtonLocation>::end_float())));

            assert_offset_eq(fab_center(tester), Offset::new(756.0, 356.0));
            assert_eq!(transient_callback_count(tester), 0);

            tester.pump_widget(frame(Some(
                <dyn FloatingActionButtonLocation>::center_float(),
            )));

            assert!(transient_callback_count(tester) > 0);

            tester.pump_and_settle();

            assert_offset_eq(fab_center(tester), Offset::new(400.0, 356.0));
            assert_eq!(transient_callback_count(tester), 0);

            tester.pump_widget(frame(Some(<dyn FloatingActionButtonLocation>::end_float())));

            assert!(transient_callback_count(tester) > 0);

            tester.pump_and_settle();

            assert_offset_eq(fab_center(tester), Offset::new(756.0, 356.0));
            assert_eq!(transient_callback_count(tester), 0);
        });
    }

    // 'moves to and from custom-defined positions'.
    #[test]
    fn the_fab_moves_to_and_from_a_custom_location() {
        test_widgets(|tester| {
            tester.pump_widget(frame(Some(Rc::new(StartTopFloatingActionButtonLocation))));

            assert_offset_eq(fab_center(tester), Offset::new(44.0, 56.0));

            tester.pump_widget(frame(Some(
                <dyn FloatingActionButtonLocation>::center_float(),
            )));
            assert!(transient_callback_count(tester) > 0);

            tester.pump_and_settle();

            assert_offset_eq(fab_center(tester), Offset::new(400.0, 356.0));
            assert_eq!(transient_callback_count(tester), 0);

            tester.pump_widget(frame(Some(Rc::new(StartTopFloatingActionButtonLocation))));

            assert!(transient_callback_count(tester) > 0);

            tester.pump_and_settle();

            assert_offset_eq(fab_center(tester), Offset::new(44.0, 56.0));
            assert_eq!(transient_callback_count(tester), 0);
        });
    }

    // interrupts in-progress animations without jumps

    /// What the scaffold's geometry and the FAB's rotations were at one geometry notification:
    /// Dart's `check` reads them in the listener; here the listener records them and
    /// [`check_steps`] runs Dart's checks over the record.
    struct Step {
        rect: Option<Size>,
        rotations: Vec<f64>,
    }

    // The maximum amounts we expect the fab width and height to change
    // during one step of a transition.
    const MAX_DELTA_WIDTH: f64 = 12.5;
    const MAX_DELTA_HEIGHT: f64 = 12.5;

    // The maximum amounts we expect the fab icon to rotate during one step
    // of a transition.
    const MAX_DELTA_ROTATION: f64 = 0.09;

    /// Dart's `_GeometryCachePainter`: caches the scaffold's geometry when it paints, the
    /// only time it can be read.
    struct GeometryCachePainter {
        geometry_listenable: Ref<ScaffoldGeometryNotifier>,
        repaint: Rc<dyn Listenable>,
        value: Rc<RefCell<Option<ScaffoldGeometry>>>,
    }

    impl CustomPainter for GeometryCachePainter {
        fn repaint(&self) -> Option<&Rc<dyn Listenable>> {
            Some(&self.repaint)
        }

        fn paint(&self, app: &mut App, _canvas: &mut waft_embedder::Canvas, _size: Size) {
            *self.value.borrow_mut() = Some(self.geometry_listenable.value(app));
        }

        fn should_repaint(&self, _app: &App, _old_delegate: &dyn CustomPainter) -> bool {
            true
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    /// Dart's `_GeometryListener`, keyed so the case can reach its state.
    #[derive(Debug)]
    struct GeometryListener {
        key: KeyRef,
    }

    impl StatefulWidget for GeometryListener {
        type State = GeometryListenerState;

        fn key(&self) -> Option<&KeyRef> {
            Some(&self.key)
        }

        fn create_state(&self) -> GeometryListenerState {
            GeometryListenerState {
                state: StateData::new(),
                geometry_listenable: None,
                cache: Rc::new(RefCell::new(None)),
            }
        }
    }

    /// Dart's `_GeometryListenerState`.
    struct GeometryListenerState {
        state: StateData<GeometryListener>,
        geometry_listenable: Option<Ref<ScaffoldGeometryNotifier>>,
        cache: Rc<RefCell<Option<ScaffoldGeometry>>>,
    }

    impl State for GeometryListenerState {
        type Widget = GeometryListener;
        waft_widgets::state_accessors!();

        fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
            let context = self.context(app);
            let new_listenable = Scaffold::geometry_of(app, context);
            if app.get(self).geometry_listenable.as_ref() == Some(&new_listenable) {
                return;
            }
            app.get_mut(self).geometry_listenable = Some(new_listenable);
            app.get_mut(self).cache = Rc::new(RefCell::new(None));
        }

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let geometry_listenable = app
                .get(self)
                .geometry_listenable
                .clone()
                .expect("dependencies");
            CustomPaint::new()
                .painter(GeometryCachePainter {
                    geometry_listenable: geometry_listenable.clone(),
                    repaint: Rc::new(geometry_listenable),
                    value: Rc::clone(&app.get(self).cache),
                })
                .into_widget()
        }
    }

    /// Every mounted element, parents first.
    fn elements(app: &mut App) -> Vec<AnyElement> {
        let root = WidgetsBinding::instance(app).root_element(app);
        let mut all = Vec::new();
        let mut pending: Vec<AnyElement> = root.into_iter().collect();
        while let Some(element) = pending.pop() {
            all.push(element);
            let mut children = Vec::new();
            element.visit_children(app, &mut |child| children.push(child));
            pending.extend(children.into_iter().rev());
        }
        all
    }

    /// Dart's `setupListener`: from now on, every geometry notification records a [`Step`].
    fn setup_listener(tester: &WidgetTester, key: &GlobalKey) -> Rc<RefCell<Vec<Step>>> {
        let steps = Rc::new(RefCell::new(Vec::new()));
        let mut app = tester.cell.borrow_mut();
        let state = key
            .current_state::<GeometryListenerState>(&app)
            .expect("the listener's state");
        let listenable = app
            .get(state)
            .geometry_listenable
            .clone()
            .expect("dependencies");
        listenable.add_listener(
            &mut app,
            waft_foundation::Listener::new({
                let steps = Rc::clone(&steps);
                move |app| {
                    let geometry = *app.get(state).cache.borrow();
                    let rect = geometry
                        .and_then(|geometry| geometry.floating_action_button_area)
                        .map(|area| area.size());
                    let rotations = elements(app)
                        .into_iter()
                        .filter_map(|element| {
                            downcast_widget::<RotationTransition>(&**element.widget(app))
                                .map(|transition| transition.turns().value(app))
                        })
                        .collect();
                    steps.borrow_mut().push(Step { rect, rotations });
                }
            }),
        );
        steps
    }

    /// Dart's `check`, run over every recorded notification in order.
    fn check_steps(steps: &[Step]) {
        let mut previous_rect: Option<Size> = None;
        let mut previous_rotations: Option<&[f64]> = None;
        for step in steps {
            let current_rect = step.rect;
            // Measure the delta in width and height of the rect, and check that
            // it never grows by more than a safe amount.
            if let (Some(previous), Some(current)) = (previous_rect, current_rect) {
                let delta_width = current.width() - previous.width();
                let delta_height = current.height() - previous.height();
                assert!(
                    delta_width.abs() <= MAX_DELTA_WIDTH,
                    "The Floating Action Button's width should not change faster than \
                     {MAX_DELTA_WIDTH} per animation step.\nPrevious rect: {previous:?}, \
                     current rect: {current:?}"
                );
                assert!(
                    delta_height.abs() <= MAX_DELTA_HEIGHT,
                    "The Floating Action Button's width should not change faster than \
                     {MAX_DELTA_HEIGHT} per animation step.\nPrevious rect: {previous:?}, \
                     current rect: {current:?}"
                );
            }
            previous_rect = current_rect;

            // Measure the delta in rotation.
            // Check that it never grows by more than a safe amount.
            //
            // There may be multiple transitions all active at
            // the same time. We are concerned only with the closest one.
            let current_rotations = &step.rotations;
            if previous_rotations.is_some_and(|previous| !previous.is_empty())
                && !current_rotations.is_empty()
                && previous_rect.is_some()
                && current_rect.is_some()
            {
                let previous = previous_rotations.expect("checked");
                let deltas: Vec<f64> = current_rotations
                    .iter()
                    .map(|current_rotation| {
                        // Dart's loop keeps the last delta: `minDelta = delta` precedes the
                        // `min`.
                        let mut min_delta = 0.0;
                        for previous_rotation in previous {
                            let delta = (previous_rotation - current_rotation).abs();
                            min_delta = delta;
                            min_delta = f64::min(delta, min_delta);
                        }
                        min_delta
                    })
                    .collect();

                assert!(
                    deltas.iter().any(|delta| *delta < MAX_DELTA_ROTATION),
                    "The Floating Action Button's rotation should not change faster than \
                     {MAX_DELTA_ROTATION} per animation step.\nDetected deltas were: \
                     {deltas:?}\nPrevious values: {previous:?}, current values: \
                     {current_rotations:?}"
                );
            }
            previous_rotations = Some(current_rotations);
        }
    }

    fn listener_frame(
        key: &GlobalKey,
        fab: Option<FloatingActionButton>,
        location: FloatingActionButtonLocationRef,
    ) -> WidgetRef {
        build_frame(
            fab,
            Some(location),
            Some(
                GeometryListener {
                    key: key.clone().into(),
                }
                .into_widget(),
            ),
            TextDirection::Ltr,
            EdgeInsets::only(0.0, 0.0, 0.0, 200.0),
            None,
        )
    }

    // 'moving the fab to centerFloat'.
    #[test]
    fn moving_the_fab_to_center_float_does_not_jump() {
        test_widgets(|tester| {
            let listener = GlobalKey::new();
            // Create a scaffold with the fab at endFloat
            tester.pump_widget(listener_frame(
                &listener,
                Some(default_fab()),
                <dyn FloatingActionButtonLocation>::end_float(),
            ));
            let steps = setup_listener(tester, &listener);

            // Move the fab to centerFloat'
            tester.pump_widget(listener_frame(
                &listener,
                Some(default_fab()),
                <dyn FloatingActionButtonLocation>::center_float(),
            ));
            tester.pump_and_settle();
            check_steps(&steps.borrow());
            assert!(!steps.borrow().is_empty());
        });
    }

    // 'interrupting motion towards the StartTop location.'.
    #[test]
    fn interrupting_motion_towards_the_start_top_location_does_not_jump() {
        test_widgets(|tester| {
            let listener = GlobalKey::new();
            tester.pump_widget(listener_frame(
                &listener,
                Some(default_fab()),
                <dyn FloatingActionButtonLocation>::center_float(),
            ));
            let steps = setup_listener(tester, &listener);

            // Move the fab to the top start after creating the fab.
            tester.pump_widget(listener_frame(
                &listener,
                Some(default_fab()),
                Rc::new(StartTopFloatingActionButtonLocation),
            ));
            tester.pump_by(K_FLOATING_ACTION_BUTTON_SEGUE / 2);

            // Interrupt motion to move to the end float
            tester.pump_widget(listener_frame(
                &listener,
                Some(default_fab()),
                <dyn FloatingActionButtonLocation>::end_float(),
            ));
            tester.pump_and_settle();
            check_steps(&steps.borrow());
            assert!(!steps.borrow().is_empty());
        });
    }

    // 'interrupting entrance to remove the fab.'.
    #[test]
    fn interrupting_the_entrance_to_remove_the_fab_does_not_jump() {
        test_widgets(|tester| {
            let listener = GlobalKey::new();
            tester.pump_widget(listener_frame(
                &listener,
                None,
                <dyn FloatingActionButtonLocation>::center_float(),
            ));
            let steps = setup_listener(tester, &listener);

            // Animate the fab in.
            tester.pump_widget(listener_frame(
                &listener,
                Some(default_fab()),
                <dyn FloatingActionButtonLocation>::end_float(),
            ));
            tester.pump_by(K_FLOATING_ACTION_BUTTON_SEGUE / 2);

            // Remove the fab.
            tester.pump_widget(listener_frame(
                &listener,
                None,
                <dyn FloatingActionButtonLocation>::end_float(),
            ));
            tester.pump_and_settle();
            check_steps(&steps.borrow());
            assert!(!steps.borrow().is_empty());
        });
    }

    // 'interrupting entrance of a new fab.'.
    #[test]
    fn interrupting_the_entrance_of_a_new_fab_does_not_jump() {
        test_widgets(|tester| {
            let listener = GlobalKey::new();
            tester.pump_widget(listener_frame(
                &listener,
                None,
                <dyn FloatingActionButtonLocation>::end_float(),
            ));
            let steps = setup_listener(tester, &listener);

            // Bring in a new fab.
            tester.pump_widget(listener_frame(
                &listener,
                Some(default_fab()),
                <dyn FloatingActionButtonLocation>::center_float(),
            ));
            tester.pump_by(K_FLOATING_ACTION_BUTTON_SEGUE / 2);

            // Interrupt motion to move the fab.
            tester.pump_widget(listener_frame(
                &listener,
                Some(default_fab()),
                <dyn FloatingActionButtonLocation>::end_float(),
            ));
            tester.pump_and_settle();
            check_steps(&steps.borrow());
            assert!(!steps.borrow().is_empty());
        });
    }

    fn bab_frame(location: FloatingActionButtonLocationRef, bab: Option<f64>) -> WidgetRef {
        build_frame(
            Some(default_fab()),
            Some(location),
            None,
            TextDirection::Ltr,
            EdgeInsets::ZERO,
            bab.map(|height| SizedBox::new().height(height).into_widget()),
        )
    }

    // 'Docked floating action button locations'.
    #[test]
    fn docked_locations_center_the_fab_on_the_top_of_the_bottom_bar() {
        test_widgets(|tester| {
            tester.pump_widget(bab_frame(
                <dyn FloatingActionButtonLocation>::end_docked(),
                Some(100.0),
            ));

            // Scaffold 800x600, FAB is 56x56, BAB is 800x100, FAB's center is
            // at the top of the BAB.
            assert_offset_eq(fab_center(tester), Offset::new(756.0, 500.0));

            tester.pump_widget(bab_frame(
                <dyn FloatingActionButtonLocation>::center_docked(),
                Some(100.0),
            ));
            tester.pump_and_settle();
            assert_offset_eq(fab_center(tester), Offset::new(400.0, 500.0));

            tester.pump_widget(bab_frame(
                <dyn FloatingActionButtonLocation>::end_docked(),
                Some(100.0),
            ));
            tester.pump_and_settle();
            assert_offset_eq(fab_center(tester), Offset::new(756.0, 500.0));
        });
    }

    // 'Docked floating action button locations: no BAB, small BAB'.
    #[test]
    fn docked_locations_keep_the_fab_on_screen_without_or_with_a_small_bottom_bar() {
        test_widgets(|tester| {
            tester.pump_widget(bab_frame(
                <dyn FloatingActionButtonLocation>::end_docked(),
                None,
            ));
            assert_offset_eq(fab_center(tester), Offset::new(756.0, 572.0));

            tester.pump_widget(bab_frame(
                <dyn FloatingActionButtonLocation>::end_docked(),
                Some(16.0),
            ));
            assert_offset_eq(fab_center(tester), Offset::new(756.0, 572.0));
        });
    }

    // 'Contained floating action button locations'.
    #[test]
    fn contained_locations_center_the_fab_in_the_bottom_bar() {
        test_widgets(|tester| {
            tester.pump_widget(bab_frame(
                <dyn FloatingActionButtonLocation>::end_contained(),
                Some(100.0),
            ));

            // Scaffold 800x600, FAB is 56x56, BAB is 800x100, FAB's center is
            // at the top of the BAB.
            // Formula: scaffold height - BAB height + FAB height / 2 + BAB top & bottom margins.
            assert_offset_eq(fab_center(tester), Offset::new(756.0, 550.0));
        });
    }

    /// Dart's `_singleFabScaffold`, without its `BottomNavigationBar` (see the module docs).
    fn single_fab_scaffold(
        app: &App,
        location: FloatingActionButtonLocationRef,
        animator: Option<FloatingActionButtonAnimatorRef>,
    ) -> WidgetRef {
        let mut scaffold = Scaffold::new()
            .app_bar(PreferredSizeWidgetRef::new(
                AppBar::new().title(Text::new("FloatingActionButtonLocation Test.")),
            ))
            .floating_action_button(
                FloatingActionButton::new(Some(waft_foundation::Listener::new(|_app| {})))
                    .child(Text::new("beach_access")),
            )
            .floating_action_button_location(location);
        if let Some(animator) = animator {
            scaffold = scaffold.floating_action_button_animator(animator);
        }
        material_app(
            ThemeData::builder().use_material3(false).build(app),
            Directionality::new(TextDirection::Ltr, scaffold).into_widget(),
        )
    }

    // Moves involving new locations: 'Animator can be updated', without the
    // BottomNavigationBar, which only moves the FAB vertically: every value asserted is
    // horizontal.
    #[test]
    fn the_animator_can_be_updated() {
        test_widgets(|tester| {
            let mut fab_animator = <dyn FloatingActionButtonAnimator>::scaling();
            let mut fab_location = <dyn FloatingActionButtonLocation>::start_float();

            let animation_duration = K_FLOATING_ACTION_BUTTON_SEGUE * 2;
            let quarter = animation_duration / 4;

            let frame = |tester: &WidgetTester,
                         location: &FloatingActionButtonLocationRef,
                         animator: &FloatingActionButtonAnimatorRef| {
                single_fab_scaffold(
                    &tester.cell.borrow(),
                    Rc::clone(location),
                    Some(Rc::clone(animator)),
                )
            };
            let root = frame(tester, &fab_location, &fab_animator);
            tester.pump_widget(root);

            assert_eq!(tester.find_by_type::<FloatingActionButton>().len(), 1);
            assert_eq!(transient_callback_count(tester), 0);
            assert_coordinate_eq(fab_center(tester).dx(), 44.0);

            fab_location = <dyn FloatingActionButtonLocation>::end_float();
            let root = frame(tester, &fab_location, &fab_animator);
            tester.pump_widget(root);

            // Dart's `lessThan(16.0)` holds through the f64 rounding of the FAB's full-turn
            // rotation, which leaves the corner 7e-15 inside; valo's f32 rotation leaves it
            // 6e-6 outside, so the bound is Dart's within POSITION_TOLERANCE.
            assert!(fab_top_left(tester).dx() < 16.0 + POSITION_TOLERANCE);

            tester.pump_by(quarter);
            assert!(fab_top_left(tester).dx() > 16.0);

            tester.pump_by(quarter);
            assert_coordinate_eq(fab_center(tester).dx(), 756.0);
            assert!(fab_top_right(tester).dx() < 800.0 - 16.0);

            tester.pump_by(quarter);
            assert!(fab_top_right(tester).dx() < 800.0 - 16.0);

            tester.pump_by(quarter);
            assert_coordinate_eq(fab_top_right(tester).dx(), 800.0 - 16.0);

            fab_location = <dyn FloatingActionButtonLocation>::start_float();
            fab_animator = Rc::new(NoScalingFabAnimator);
            let root = frame(tester, &fab_location, &fab_animator);
            tester.pump_widget(root);

            tester.pump_by(quarter);
            assert_coordinate_eq(fab_center(tester).dx(), 756.0);
            assert_coordinate_eq(fab_top_right(tester).dx(), 800.0 - 16.0);

            tester.pump_by(quarter);
            assert_coordinate_eq(fab_center(tester).dx(), 44.0);
            // As above: Dart's bound within POSITION_TOLERANCE.
            assert!(fab_top_left(tester).dx() < 16.0 + POSITION_TOLERANCE);
        });
    }

    // Locations account for safe interactive areas

    /// Dart's `buildTest`, without `bottomNavigationBar:`, which waits.
    #[allow(clippy::too_many_arguments, reason = "Dart's buildTest arguments")]
    fn build_test(
        app: &App,
        location: FloatingActionButtonLocationRef,
        data: MediaQueryData,
        key: &GlobalKey,
        mini: bool,
        app_bar: bool,
        bottom_sheet: bool,
        resize_to_avoid_bottom_inset: bool,
    ) -> WidgetRef {
        let key = key.clone();
        let mut scaffold = Scaffold::new()
            .resize_to_avoid_bottom_inset(resize_to_avoid_bottom_inset)
            .floating_action_button_location(location)
            .floating_action_button(Builder::new(move |_app, context| {
                FloatingActionButton::new(Some(waft_foundation::Listener::new(move |app| {
                    ScaffoldMessenger::of(app, context).show_snack_bar(
                        app,
                        SnackBar::new(Text::new("Snacky!")),
                        None,
                    );
                })))
                .mini(mini)
                .key(key.clone().into())
                .child(Text::new("FAB"))
                .into_widget()
            }));
        if bottom_sheet {
            scaffold = scaffold.bottom_sheet(
                SizedBox::new()
                    .height(100.0)
                    .child(Center::new().child(Text::new("BottomSheet"))),
            );
        }
        if app_bar {
            scaffold = scaffold.app_bar(PreferredSizeWidgetRef::new(
                AppBar::new().title(Text::new("Demo")),
            ));
        }
        material_app(
            ThemeData::builder().use_material3(false).build(app),
            MediaQuery::new(data, scaffold).into_widget(),
        )
    }

    struct SafeAreaCase {
        location: FloatingActionButtonLocationRef,
        mini: bool,
        key: GlobalKey,
        next_pointer: i64,
    }

    impl SafeAreaCase {
        fn new(location: FloatingActionButtonLocationRef, mini: bool) -> SafeAreaCase {
            SafeAreaCase {
                location,
                mini,
                key: GlobalKey::new(),
                next_pointer: 1,
            }
        }

        /// `tester.pumpWidget(buildTest(location, data, key, mini:, bottomSheet:,
        /// resizeToAvoidBottomInset:))`.
        fn pump(
            &self,
            tester: &mut WidgetTester,
            data: MediaQueryData,
            bottom_sheet: bool,
            resize_to_avoid_bottom_inset: bool,
        ) {
            self.pump_with(
                tester,
                data,
                false,
                bottom_sheet,
                resize_to_avoid_bottom_inset,
            );
        }

        /// [`pump`](Self::pump) with `buildTest(appBar:)`.
        fn pump_with(
            &self,
            tester: &mut WidgetTester,
            data: MediaQueryData,
            app_bar: bool,
            bottom_sheet: bool,
            resize_to_avoid_bottom_inset: bool,
        ) {
            let root = build_test(
                &tester.cell.borrow(),
                Rc::clone(&self.location),
                data,
                &self.key,
                self.mini,
                app_bar,
                bottom_sheet,
                resize_to_avoid_bottom_inset,
            );
            tester.pump_widget(root);
        }

        fn rect(&self, tester: &WidgetTester) -> Rect {
            rect_of(tester, &self.key)
        }

        /// `tester.tap(find.byKey(floatingActionButton))` and `pumpAndSettle`: shows a snack
        /// bar.
        fn tap_and_settle(&mut self, tester: &mut WidgetTester) {
            let center = tester.center_of(&self.key);
            tester.tap(center, self.next_pointer);
            self.next_pointer += 1;
            tester.pump_and_settle();
        }
    }

    const KEYBOARD_HEIGHT: f64 = 200.0;
    const VIEW_PADDING: f64 = 50.0;

    fn view_padding_bottom() -> MediaQueryData {
        MediaQueryData::new().view_padding(EdgeInsets::only(0.0, 0.0, 0.0, VIEW_PADDING))
    }

    fn view_padding_bottom_with_keyboard() -> MediaQueryData {
        view_padding_bottom().view_insets(EdgeInsets::only(0.0, 0.0, 0.0, KEYBOARD_HEIGHT))
    }

    /// Dart's `runFloatTests`, without the `BottomNavigationBar` steps, which wait.
    fn run_float_tests(
        tester: &mut WidgetTester,
        location: FloatingActionButtonLocationRef,
        default_rect: Rect,
        bottom_sheet_rect: Rect,
        snack_bar_rect: Rect,
        mini: bool,
    ) {
        let mut case = SafeAreaCase::new(location, mini);
        // Default
        case.pump(tester, view_padding_bottom(), false, true);
        assert_rect_more_or_less_equals(case.rect(tester), default_rect);
        // Present keyboard and check position, should change
        case.pump(tester, view_padding_bottom_with_keyboard(), false, true);
        assert_rect_more_or_less_equals(
            case.rect(tester),
            default_rect.translate(0.0, VIEW_PADDING - KEYBOARD_HEIGHT),
        );

        // With resizeToAvoidBottomInset: false
        // With keyboard presented, should maintain default position
        case.pump(tester, view_padding_bottom_with_keyboard(), false, false);
        assert_rect_more_or_less_equals(case.rect(tester), default_rect);

        // BottomSheet default
        case.pump(tester, view_padding_bottom(), true, true);
        assert_rect_more_or_less_equals(case.rect(tester), bottom_sheet_rect);
        // Present keyboard and check position, bottomSheet and FAB both resize
        case.pump(tester, view_padding_bottom_with_keyboard(), true, true);
        assert_rect_more_or_less_equals(
            case.rect(tester),
            bottom_sheet_rect.translate(0.0, -KEYBOARD_HEIGHT),
        );

        // bottomSheet with resizeToAvoidBottomInset: false
        // With keyboard presented, should maintain default bottomSheet position
        case.pump(tester, view_padding_bottom_with_keyboard(), true, false);
        assert_rect_more_or_less_equals(case.rect(tester), bottom_sheet_rect);

        // SnackBar default
        case.pump(tester, view_padding_bottom(), false, true);
        case.tap_and_settle(tester); // Show SnackBar
        assert_rect_more_or_less_equals(case.rect(tester), snack_bar_rect);

        // SnackBar when resized for presented keyboard
        case.pump(tester, view_padding_bottom_with_keyboard(), false, true);
        case.tap_and_settle(tester); // Show SnackBar
        assert_rect_more_or_less_equals(
            case.rect(tester),
            snack_bar_rect.translate(
                0.0,
                -KEYBOARD_HEIGHT + K_FLOATING_ACTION_BUTTON_MARGIN / 2.0,
            ),
        );
    }

    // 'startFloat', without the BottomNavigationBar steps.
    #[test]
    fn start_float_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_float_tests(
                tester,
                <dyn FloatingActionButtonLocation>::start_float(),
                Rect::from_ltrb(16.0, 478.0, 72.0, 534.0),
                Rect::from_ltrb(16.0, 472.0, 72.0, 528.0),
                Rect::from_ltrb(16.0, 478.0, 72.0, 534.0),
                false,
            );
        });
    }

    // 'miniStartFloat', without the BottomNavigationBar steps.
    #[test]
    fn mini_start_float_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_float_tests(
                tester,
                <dyn FloatingActionButtonLocation>::mini_start_float(),
                Rect::from_ltrb(12.0, 490.0, 60.0, 538.0),
                Rect::from_ltrb(12.0, 480.0, 60.0, 528.0),
                Rect::from_ltrb(12.0, 490.0, 60.0, 538.0),
                true,
            );
        });
    }

    // 'centerFloat', without the BottomNavigationBar steps.
    #[test]
    fn center_float_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_float_tests(
                tester,
                <dyn FloatingActionButtonLocation>::center_float(),
                Rect::from_ltrb(372.0, 478.0, 428.0, 534.0),
                Rect::from_ltrb(372.0, 472.0, 428.0, 528.0),
                Rect::from_ltrb(372.0, 478.0, 428.0, 534.0),
                false,
            );
        });
    }

    // 'miniCenterFloat', without the BottomNavigationBar steps.
    #[test]
    fn mini_center_float_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_float_tests(
                tester,
                <dyn FloatingActionButtonLocation>::mini_center_float(),
                Rect::from_ltrb(376.0, 490.0, 424.0, 538.0),
                Rect::from_ltrb(376.0, 480.0, 424.0, 528.0),
                Rect::from_ltrb(376.0, 490.0, 424.0, 538.0),
                true,
            );
        });
    }

    // 'endFloat', without the BottomNavigationBar steps.
    #[test]
    fn end_float_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_float_tests(
                tester,
                <dyn FloatingActionButtonLocation>::end_float(),
                Rect::from_ltrb(728.0, 478.0, 784.0, 534.0),
                Rect::from_ltrb(728.0, 472.0, 784.0, 528.0),
                Rect::from_ltrb(728.0, 478.0, 784.0, 534.0),
                false,
            );
        });
    }

    // 'miniEndFloat', without the BottomNavigationBar steps.
    #[test]
    fn mini_end_float_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_float_tests(
                tester,
                <dyn FloatingActionButtonLocation>::mini_end_float(),
                Rect::from_ltrb(740.0, 490.0, 788.0, 538.0),
                Rect::from_ltrb(740.0, 480.0, 788.0, 528.0),
                Rect::from_ltrb(740.0, 490.0, 788.0, 538.0),
                true,
            );
        });
    }

    /// Dart's `runDockedTests`, without the `BottomNavigationBar` steps (every step that
    /// reads `bottomSheetRect` has one), which wait.
    fn run_docked_tests(
        tester: &mut WidgetTester,
        location: FloatingActionButtonLocationRef,
        default_rect: Rect,
        snack_bar_rect: Rect,
        mini: bool,
    ) {
        const SCAFFOLD_HEIGHT: f64 = 600.0;
        let mut case = SafeAreaCase::new(location, mini);
        // Default
        case.pump(tester, view_padding_bottom(), false, true);
        assert_rect_more_or_less_equals(case.rect(tester), default_rect);
        // Present keyboard and check position, should change
        case.pump(tester, view_padding_bottom_with_keyboard(), false, true);
        assert_rect_more_or_less_equals(
            case.rect(tester),
            default_rect.translate(
                0.0,
                VIEW_PADDING - KEYBOARD_HEIGHT - K_FLOATING_ACTION_BUTTON_MARGIN,
            ),
        );
        // The FAB should be away from the keyboard
        assert!(case.rect(tester).bottom < SCAFFOLD_HEIGHT - KEYBOARD_HEIGHT);

        // With resizeToAvoidBottomInset: false
        // With keyboard presented, should maintain default position
        case.pump(tester, view_padding_bottom_with_keyboard(), false, false);
        assert_rect_more_or_less_equals(case.rect(tester), default_rect);

        // SnackBar default
        case.pump(tester, view_padding_bottom(), false, true);
        case.tap_and_settle(tester); // Show SnackBar
        assert_rect_more_or_less_equals(case.rect(tester), snack_bar_rect);

        // SnackBar when resized for presented keyboard
        case.pump(tester, view_padding_bottom_with_keyboard(), false, true);
        case.tap_and_settle(tester); // Show SnackBar
        assert_rect_more_or_less_equals(
            case.rect(tester),
            snack_bar_rect.translate(0.0, -KEYBOARD_HEIGHT),
        );
        // The FAB should be away from the keyboard
        assert!(case.rect(tester).bottom < SCAFFOLD_HEIGHT - KEYBOARD_HEIGHT);
    }

    // 'startDocked', without the BottomNavigationBar steps.
    #[test]
    fn start_docked_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_docked_tests(
                tester,
                <dyn FloatingActionButtonLocation>::start_docked(),
                Rect::from_ltrb(16.0, 494.0, 72.0, 550.0),
                Rect::from_ltrb(16.0, 486.0, 72.0, 542.0),
                false,
            );
        });
    }

    // 'miniStartDocked', without the BottomNavigationBar steps.
    #[test]
    fn mini_start_docked_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_docked_tests(
                tester,
                <dyn FloatingActionButtonLocation>::mini_start_docked(),
                Rect::from_ltrb(12.0, 502.0, 60.0, 550.0),
                Rect::from_ltrb(12.0, 494.0, 60.0, 542.0),
                true,
            );
        });
    }

    // 'centerDocked', without the BottomNavigationBar steps.
    #[test]
    fn center_docked_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_docked_tests(
                tester,
                <dyn FloatingActionButtonLocation>::center_docked(),
                Rect::from_ltrb(372.0, 494.0, 428.0, 550.0),
                Rect::from_ltrb(372.0, 486.0, 428.0, 542.0),
                false,
            );
        });
    }

    // 'miniCenterDocked', without the BottomNavigationBar steps.
    #[test]
    fn mini_center_docked_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_docked_tests(
                tester,
                <dyn FloatingActionButtonLocation>::mini_center_docked(),
                Rect::from_ltrb(376.0, 502.0, 424.0, 550.0),
                Rect::from_ltrb(376.0, 494.0, 424.0, 542.0),
                true,
            );
        });
    }

    // 'endDocked', without the BottomNavigationBar steps.
    #[test]
    fn end_docked_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_docked_tests(
                tester,
                <dyn FloatingActionButtonLocation>::end_docked(),
                Rect::from_ltrb(728.0, 494.0, 784.0, 550.0),
                Rect::from_ltrb(728.0, 486.0, 784.0, 542.0),
                false,
            );
        });
    }

    // 'miniEndDocked', without the BottomNavigationBar steps.
    #[test]
    fn mini_end_docked_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_docked_tests(
                tester,
                <dyn FloatingActionButtonLocation>::mini_end_docked(),
                Rect::from_ltrb(740.0, 502.0, 788.0, 550.0),
                Rect::from_ltrb(740.0, 494.0, 788.0, 542.0),
                true,
            );
        });
    }

    /// Dart's `runTopTests`.
    fn run_top_tests(
        tester: &mut WidgetTester,
        location: FloatingActionButtonLocationRef,
        default_rect: Rect,
        app_bar_rect: Rect,
        mini: bool,
    ) {
        let case = SafeAreaCase::new(location, mini);
        // Default
        case.pump(
            tester,
            MediaQueryData::new().view_padding(EdgeInsets::only(0.0, VIEW_PADDING, 0.0, 0.0)),
            false,
            true,
        );
        assert_rect_more_or_less_equals(case.rect(tester), default_rect);

        // AppBar default
        case.pump_with(
            tester,
            MediaQueryData::new().view_padding(EdgeInsets::only(0.0, VIEW_PADDING, 0.0, 0.0)),
            true,
            false,
            true,
        );
        assert_rect_more_or_less_equals(case.rect(tester), app_bar_rect);
    }

    // 'startTop'.
    #[test]
    fn start_top_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_top_tests(
                tester,
                <dyn FloatingActionButtonLocation>::start_top(),
                Rect::from_ltrb(16.0, 50.0, 72.0, 106.0),
                // Positioned relative to AppBar
                Rect::from_ltrb(16.0, 28.0, 72.0, 84.0),
                false,
            );
        });
    }

    // 'miniStartTop'.
    #[test]
    fn mini_start_top_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_top_tests(
                tester,
                <dyn FloatingActionButtonLocation>::mini_start_top(),
                Rect::from_ltrb(12.0, 50.0, 60.0, 98.0),
                // Positioned relative to AppBar
                Rect::from_ltrb(12.0, 32.0, 60.0, 80.0),
                true,
            );
        });
    }

    // 'centerTop'.
    #[test]
    fn center_top_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_top_tests(
                tester,
                <dyn FloatingActionButtonLocation>::center_top(),
                Rect::from_ltrb(372.0, 50.0, 428.0, 106.0),
                // Positioned relative to AppBar
                Rect::from_ltrb(372.0, 28.0, 428.0, 84.0),
                false,
            );
        });
    }

    // 'miniCenterTop'.
    #[test]
    fn mini_center_top_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_top_tests(
                tester,
                <dyn FloatingActionButtonLocation>::mini_center_top(),
                Rect::from_ltrb(376.0, 50.0, 424.0, 98.0),
                // Positioned relative to AppBar
                Rect::from_ltrb(376.0, 32.0, 424.0, 80.0),
                true,
            );
        });
    }

    // 'endTop'.
    #[test]
    fn end_top_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_top_tests(
                tester,
                <dyn FloatingActionButtonLocation>::end_top(),
                Rect::from_ltrb(728.0, 50.0, 784.0, 106.0),
                // Positioned relative to AppBar
                Rect::from_ltrb(728.0, 28.0, 784.0, 84.0),
                false,
            );
        });
    }

    // 'miniEndTop'.
    #[test]
    fn mini_end_top_accounts_for_safe_interactive_areas() {
        test_widgets(|tester| {
            run_top_tests(
                tester,
                <dyn FloatingActionButtonLocation>::mini_end_top(),
                Rect::from_ltrb(740.0, 50.0, 788.0, 98.0),
                // Positioned relative to AppBar
                Rect::from_ltrb(740.0, 32.0, 788.0, 80.0),
                true,
            );
        });
    }

    /// `MaterialApp(home: Directionality(textDirection, Scaffold(appBar: AppBar(),
    /// floatingActionButton: const FloatingActionButton(onPressed: null),
    /// floatingActionButtonLocation: location)))`; the LTR cases have no `Directionality` of
    /// their own, and the app's is left to right.
    fn top_location_rect(
        tester: &mut WidgetTester,
        location: FloatingActionButtonLocationRef,
        text_direction: TextDirection,
    ) -> Rect {
        let theme = ThemeData::new(&tester.cell.borrow());
        let key = GlobalKey::new();
        tester.pump_widget(material_app(
            theme,
            Directionality::new(
                text_direction,
                Scaffold::new()
                    .app_bar(PreferredSizeWidgetRef::new(AppBar::new()))
                    .floating_action_button(FloatingActionButton::new(None).key(key.clone().into()))
                    .floating_action_button_location(location),
            )
            .into_widget(),
        ));
        rect_of(tester, &key)
    }

    // 'Start-top floating action button location LTR'.
    #[test]
    fn the_start_top_location_left_to_right() {
        test_widgets(|tester| {
            assert_rect_more_or_less_equals(
                top_location_rect(
                    tester,
                    <dyn FloatingActionButtonLocation>::start_top(),
                    TextDirection::Ltr,
                ),
                Rect::from_ltwh(16.0, 28.0, 56.0, 56.0),
            );
        });
    }

    // 'End-top floating action button location RTL'.
    #[test]
    fn the_end_top_location_right_to_left() {
        test_widgets(|tester| {
            assert_rect_more_or_less_equals(
                top_location_rect(
                    tester,
                    <dyn FloatingActionButtonLocation>::end_top(),
                    TextDirection::Rtl,
                ),
                Rect::from_ltwh(16.0, 28.0, 56.0, 56.0),
            );
        });
    }

    // 'Start-top floating action button location RTL'.
    #[test]
    fn the_start_top_location_right_to_left() {
        test_widgets(|tester| {
            assert_rect_more_or_less_equals(
                top_location_rect(
                    tester,
                    <dyn FloatingActionButtonLocation>::start_top(),
                    TextDirection::Rtl,
                ),
                Rect::from_ltwh(800.0 - 56.0 - 16.0, 28.0, 56.0, 56.0),
            );
        });
    }

    // 'End-top floating action button location LTR'.
    #[test]
    fn the_end_top_location_left_to_right() {
        test_widgets(|tester| {
            assert_rect_more_or_less_equals(
                top_location_rect(
                    tester,
                    <dyn FloatingActionButtonLocation>::end_top(),
                    TextDirection::Ltr,
                ),
                Rect::from_ltwh(800.0 - 56.0 - 16.0, 28.0, 56.0, 56.0),
            );
        });
    }

    // Waiting:
    //  * CircleAvatar: 'Mini-start-top floating action button location'.
    //  * BottomNavigationBar: every case of 'New Floating Action Button Locations' and 'Custom
    //    Floating Action Button Locations', 'Moves between new locations and new locations',
    //    'Moves between new locations and old locations', 'Moves between new locations and old
    //    locations with custom animator', and the BottomNavigationBar steps of the float and
    //    docked cases of 'Locations account for safe interactive areas'.
}
