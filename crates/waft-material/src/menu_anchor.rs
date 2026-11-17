//! Flutter counterpart: material_ui's `menu_anchor.dart`.
//!
//! A [`MenuAnchor`] shows a Material menu over a [`RawMenuAnchor`]; a [`MenuBar`] is a
//! [`RawMenuAnchorGroup`] of top-level [`SubmenuButton`]s. Every menu in the tree finds the
//! menu that holds it through the nearest `_MenuAnchorScope`, which an anchor builds above its
//! button and its overlay: a scope's state is the anchor of the widgets below it, so a widget
//! that reads it from its own context gets the menu it sits in, never one it owns.

#![allow(deprecated)]

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use unicode_segmentation::UnicodeSegmentation;
use waft_animation::{
    Animation, AnimationController, AnimationRef, AnimationStatus, AnimationStatusListener, Cubic,
    Curve, CurvedAnimation, FlippedCurve, Interval,
};
use waft_embedder::{
    Clip, Color, Offset, Rect, Size, TargetPlatform, TextDecoration, TextDirection, lerp_double,
};
use waft_foundation::{
    App, Handle, HandleId, Listener, OptionRetained, Ref, Timer, ValueChanged, VoidCallback,
};
use waft_gestures::{PointerDownEvent, PointerExitEvent, PointerHoverEvent};
use waft_painting::{
    Alignment, AlignmentGeometry, Axis, EdgeInsets, EdgeInsetsGeometry, InlineSpanRef,
    OutlinedBorderRef, ShapeBorderRef, TextSpan,
};
use waft_rendering::{
    BoxConstraints, CrossAxisAlignment, HitTestBehavior, LayerLink, MainAxisAlignment,
    MainAxisSize, SingleChildLayoutDelegate,
};
use waft_scheduler::{
    FrameCallback, SchedulerBinding, Ticker, TickerCallback, TickerProviderObject,
};
use waft_services::{
    HardwareKeyboard, KeyEvent, KeyEventCallback, LogicalKeyboardKey, MouseCursor, MouseCursorRef,
};
use waft_widgets::{
    Action, ActionData, ActionRef, Actions, ActivateIntent, AnimatedBuilder, AnyFocusNode,
    AnyMenuController, AnyScrollController, BuildContext, Builder, CharacterActivator,
    ClampingScrollPhysics, ClipRect, CompositedTransformFollower, CompositedTransformTarget,
    ConstrainedBox, CustomSingleChildLayout, DefaultTextStyle, DirectionalFocusIntent,
    Directionality, DismissIntent, DismissMenuAction, ExcludeFocus, ExcludeSemantics, Expanded,
    FadeTransition, Flex, FocusManager, FocusNode, FocusNodeLeaf, FocusNodeRef, FocusScope,
    FocusScopeNode, FocusTraversalGroup, FocusTraversalPolicy, GlobalKey, IgnorePointer,
    InheritedWidget, IntoWidget, IntrinsicHeight, IntrinsicWidth, KeyRef, MediaQuery,
    MediaQueryData, MenuController, MenuControllerRef, MenuSerializableShortcutRef, MergeSemantics,
    MouseRegion, NextFocusIntent, Padding, PreviousFocusIntent, PrimaryScrollController,
    RawMenuAnchor, RawMenuAnchorGroup, RawMenuOverlayInfo, ReadingOrderTraversalPolicy, RichText,
    Row, ScrollConfiguration, ScrollController, ScrollControllerLeaf, ScrollPhysicsRef, Semantics,
    ShortcutActivatorRef, ShortcutMap, ShortcutRegistry, ShortcutRegistryEntry, Shortcuts,
    SingleActivator, SingleChildScrollView, SingleTickerProviderStateMixin,
    SingleTickerProviderStateMixinData, State, StateData, StatefulWidget, StatelessWidget,
    TapRegion, Text, TraversalDirection, UnconstrainedBox, View, VoidCallbackIntent, WidgetRef,
    WidgetState, WidgetStateMouseCursor, WidgetStatePropertyRef, WidgetStates,
    WidgetStatesController, debug_check_has_overlay, downcast_widget,
};

use crate::button_style::{ButtonStyle, ButtonStyleGetters};
use crate::checkbox::Checkbox;
use crate::generated::menu_defaults_m3::{MenuBarDefaultsM3, MenuButtonDefaultsM3, MenuDefaultsM3};
use crate::icons::Icons;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::material::{Material, MaterialType};
use crate::material_localizations::MaterialLocalizations;
use crate::menu_bar_theme::MenuBarTheme;
use crate::menu_button_theme::MenuButtonTheme;
use crate::menu_style::{MenuStyle, MenuStyleGetters};
use crate::menu_theme::MenuTheme;
use crate::motion::Easing;
use crate::radio::Radio;
use crate::scrollbar::Scrollbar;
use crate::text_button::TextButton;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

// The default size of the arrow in _MenuItemLabel that indicates that a menu
// has a submenu.
const K_DEFAULT_SUBMENU_ICON_SIZE: f64 = 24.0;

// The default spacing between the leading icon, label, trailing icon, and
// shortcut label in a _MenuItemLabel.
pub(crate) const K_LABEL_ITEM_DEFAULT_SPACING: f64 = 12.0;

// The minimum spacing between the leading icon, label, trailing icon, and
// shortcut label in a _MenuItemLabel.
const K_LABEL_ITEM_MIN_SPACING: f64 = 4.0;

// Navigation shortcuts that we need to make sure are active when menus are
// open.
fn menu_traversal_shortcuts() -> ShortcutMap {
    fn shortcut(
        activator: SingleActivator,
        intent: impl waft_widgets::Intent,
    ) -> (ShortcutActivatorRef, waft_widgets::IntentRef) {
        (Rc::new(activator), Rc::new(intent))
    }
    vec![
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::GAME_BUTTON_A),
            ActivateIntent::new(),
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ESCAPE),
            DismissIntent::new(),
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::TAB),
            NextFocusIntent::new(),
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::TAB).shift(true),
            PreviousFocusIntent::new(),
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_DOWN),
            DirectionalFocusIntent::new(TraversalDirection::Down),
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_UP),
            DirectionalFocusIntent::new(TraversalDirection::Up),
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_LEFT),
            DirectionalFocusIntent::new(TraversalDirection::Left),
        ),
        shortcut(
            SingleActivator::new(LogicalKeyboardKey::ARROW_RIGHT),
            DirectionalFocusIntent::new(TraversalDirection::Right),
        ),
    ]
}

// The minimum vertical spacing on the outside of menus.
pub(crate) const K_MENU_VERTICAL_MIN_PADDING: f64 = 8.0;

// How close to the edge of the safe area the menu will be placed.
pub(crate) const K_MENU_VIEW_PADDING: f64 = 8.0;

// The minimum horizontal spacing on the outside of the top level menu.
pub(crate) const K_TOP_LEVEL_MENU_HORIZONTAL_MIN_PADDING: f64 = 4.0;

// The default opening animation duration for menus.
const K_MENU_OPENING_DURATION: Duration = Duration::from_millis(500);

// The default closing animation duration for menus.
const K_MENU_CLOSING_DURATION: Duration = Duration::from_millis(150);

/// The default curve used to animate the height of the menu panel while the
/// menu is opening.
///
/// **NOTE**: This animation curve follows the material web implementation of
/// [Easing.emphasized](https://github.com/material-components/material-web/blob/516cbc02bf770b7c3c5c6b546f1e5d81939b9f23/internal/motion/animation.ts#L18)
/// instead of the curve recommended by the Material 3 specification,
/// [Curves.easeInOutCubicEmphasized](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs).
/// This change was made because the web curve [was more consistent with the
/// timing of the menu item fade-in
/// animations](https://github.com/flutter/flutter/pull/176494#discussion_r2545178254).
fn menu_panel_height_forward_curve() -> Rc<dyn Curve> {
    Rc::new(Cubic::new(0.3, 0.0, 0.0, 1.0))
}

// The default curve used to animate the height of the menu panel while the menu
// is closing.
fn menu_panel_height_reverse_curve() -> Rc<dyn Curve> {
    Rc::new(TweenCurve::new(
        0.35,
        1.0,
        Rc::new(FlippedCurve::new(Easing::emphasized_accelerate())),
    ))
}

// The default curve used to animate the opacity of the menu panel when opening.
fn menu_panel_opacity_forward_curve() -> Rc<dyn Curve> {
    Rc::new(Interval::new(
        0.0,
        50.0 / 500.0,
        waft_animation::Curves::linear(),
    ))
}

// The default curve used to animate the opacity of the menu panel when closing.
#[allow(clippy::eq_op, reason = "Dart's `Interval(100 / 150, 150 / 150)`")]
fn menu_panel_opacity_reverse_curve() -> Rc<dyn Curve> {
    Rc::new(FlippedCurve::new(Rc::new(Interval::new(
        100.0 / 150.0,
        150.0 / 150.0,
        waft_animation::Curves::linear(),
    ))))
}

// The default fade-in duration of each menu item as a fraction of the opening
// duration.
const K_MENU_ITEM_RELATIVE_FADE_IN_DURATION: f64 = 1.0 / 2.0;

// The default fade-out duration of each menu item as a fraction of the closing
// duration.
const K_MENU_ITEM_RELATIVE_FADE_OUT_DURATION: f64 = 1.0 / 3.0;

// The default delay between the start of each menu item's fade-out as a
// fraction of the closing duration.
const K_MENU_ITEM_RELATIVE_FADE_OUT_DELAY: f64 = 1.0 / 3.0;

/// The type of builder function used by [`MenuAnchor::builder`] to build the
/// widget that the [`MenuAnchor`] surrounds.
///
/// The `context` is the context that the widget is being built in.
///
/// The `controller` is the [`MenuController`] that can be used to open and close
/// the menu with.
///
/// The `child` is an optional child supplied as the [`MenuAnchor::child`]
/// attribute. The child is intended to be incorporated in the result of the
/// function.
pub type MenuAnchorChildBuilder =
    Rc<dyn Fn(&mut App, BuildContext, AnyMenuController, Option<&WidgetRef>) -> WidgetRef>;

/// Dart's `_MenuAnchorScope`.
#[derive(Debug)]
struct MenuAnchorScope {
    state: AnyMenuAnchorState,
    animation_status: AnimationStatus,
    child: WidgetRef,
}

impl InheritedWidget for MenuAnchorScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &MenuAnchorScope) -> bool {
        debug_assert!(
            old_widget.state == self.state,
            "The state of a MenuAnchor should not change."
        );
        old_widget.animation_status != self.animation_status
    }
}

// A curve that linearly interpolates between two values over a given curve.
//
// For example, `_TweenCurve(0.2, 0.8, curve: Curves.easeIn)` will produce a
// curve that starts at 0.2, ends at 0.8, and follows the easeIn curve between
// those two values. The curve is applied first, and then the result is linearly
// interpolated between begin and end.
//
// This differs from an `Interval` in that an Interval changes the duration over
// which the curve is applied, whereas `_TweenCurve` changes the output range of
// the curve.
struct TweenCurve {
    begin: f64,
    end: f64,
    curve: Rc<dyn Curve>,
}

impl TweenCurve {
    fn new(begin: f64, end: f64, curve: Rc<dyn Curve>) -> TweenCurve {
        debug_assert!(begin >= 0.0);
        debug_assert!(begin <= 1.0);
        debug_assert!(end >= 0.0);
        debug_assert!(end <= 1.0);
        debug_assert!(end >= begin);
        TweenCurve { begin, end, curve }
    }
}

impl Curve for TweenCurve {
    fn transform_internal(&self, t: f64) -> f64 {
        let t = self.curve.transform(t);
        lerp_double(Some(self.begin), Some(self.end), t).expect("both ends")
    }
}

impl Debug for TweenCurve {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "_TweenCurve({}, {}, {:?})",
            self.begin, self.end, self.curve
        )
    }
}

/// Dart's `'$Type(${widget.child})'`, the debug label of a button's own focus node.
fn describe_child(type_name: &str, child: &Option<WidgetRef>) -> String {
    match child {
        Some(child) => format!("{type_name}({child:?})"),
        None => format!("{type_name}(null)"),
    }
}

/// Dart's `widget.menuChildren != oldWidget.menuChildren`, an identity test on the list: here
/// the same widgets in the same order.
fn same_widgets(a: &[WidgetRef], b: &[WidgetRef]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| Rc::ptr_eq(a, b))
}

// ---------------------------------------------------------------------------------------------
// MenuAnchor

/// A widget used to mark the "anchor" for a set of submenus, defining the
/// rectangle used to position the menu, which can be done either with an
/// explicit location, or with an alignment.
///
/// When creating a menu with [`MenuBar`] or a [`SubmenuButton`], a [`MenuAnchor`] is
/// not needed, since they provide their own internally.
///
/// The [`MenuAnchor`] is meant to be a slightly lower level interface than
/// [`MenuBar`], used in situations where a [`MenuBar`] isn't appropriate, or to
/// construct widgets or screen regions that have submenus.
///
/// To programmatically control a [`MenuAnchor`], like opening or closing it, or checking its
/// state, you can get its associated [`MenuController`]. Use `MenuController::maybe_of(app,
/// context)` to retrieve the controller for the closest [`MenuAnchor`] ancestor of a given
/// [`BuildContext`]. More detailed usage of [`MenuController`] is available in its type
/// documentation.
///
/// The [`MenuStyle::visual_density`] setting only affects horizontal padding,
/// and it will never make it negative. Vertical padding is not affected at all.
pub struct MenuAnchor {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,

    /// An optional controller that allows opening and closing of the menu from
    /// other widgets.
    pub controller: Option<MenuControllerRef>,

    /// The [`child_focus_node`](Self::child_focus_node) attribute is the optional `FocusNode`
    /// also associated to the [`child`](Self::child) or [`builder`](Self::builder) widget that
    /// opens the menu.
    ///
    /// The focus node should be attached to the widget that should receive focus
    /// if keyboard focus traversal moves the focus off of the submenu with the
    /// arrow keys.
    ///
    /// If not supplied, then keyboard traversal from the menu back to the
    /// controlling button when the menu is open is disabled.
    pub child_focus_node: Option<FocusNodeRef>,

    /// The [`MenuStyle`] that defines the visual attributes of the menu bar.
    ///
    /// Colors and sizing of the menus is controllable via the [`MenuStyle`].
    ///
    /// Defaults to the ambient `MenuThemeData::style`.
    pub style: Option<MenuStyle>,

    /// The offset of the menu relative to the alignment origin determined by
    /// [`MenuStyle::alignment`] on the [`style`](Self::style) attribute and the ambient
    /// `Directionality`.
    ///
    /// Use this for adjustments of the menu placement.
    ///
    /// Increasing `Offset::dy` values of [`alignment_offset`](Self::alignment_offset) move
    /// the menu position down.
    ///
    /// If the [`MenuStyle::alignment`] from [`style`](Self::style) is not an
    /// `AlignmentDirectional` (e.g. `Alignment`), then increasing `Offset::dx` values of
    /// [`alignment_offset`](Self::alignment_offset) move the menu position to the right.
    ///
    /// If the [`MenuStyle::alignment`] from [`style`](Self::style) is an
    /// `AlignmentDirectional`, then in a `TextDirection::Ltr` `Directionality`, increasing
    /// `Offset::dx` values of [`alignment_offset`](Self::alignment_offset) move the menu
    /// position to the right. In a `TextDirection::Rtl` directionality, increasing
    /// `Offset::dx` values of [`alignment_offset`](Self::alignment_offset) move the menu
    /// position to the left.
    ///
    /// Defaults to `Offset::ZERO`.
    pub alignment_offset: Option<Offset>,

    /// An optional [`LayerLink`] to attach the menu to the widget that this
    /// [`MenuAnchor`] surrounds.
    ///
    /// When provided, the menu will follow the widget that this [`MenuAnchor`]
    /// surrounds if it moves because of view insets changes.
    pub layer_link: Option<Ref<LayerLink>>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to `Clip::HardEdge`.
    pub clip_behavior: Clip,

    /// Whether the menus will be closed if the anchor area is tapped.
    ///
    /// For menus opened by buttons that toggle the menu, if the button is tapped
    /// when the menu is open, the button should close the menu. But if
    /// [`anchor_tap_closes_menu`](Self::anchor_tap_closes_menu) is true, then the menu will
    /// close, and (surprisingly) immediately re-open. This is because tapping on the button
    /// closes the menu before the `on_pressed` or `on_tap` handler is called
    /// because of it being considered to be "outside" the menu system, and then
    /// the button (seeing that the menu is closed) immediately reopens the menu.
    /// The result is that the user thinks that tapping on the button does
    /// nothing. So, for button-initiated menus, this value is typically false so
    /// that the menu anchor area is considered "inside" of the menu system and
    /// doesn't cause it to close unless [`MenuControllerLeaf::close`] is called.
    ///
    /// For menus that are positioned using [`MenuControllerLeaf::open`]'s `position`
    /// parameter, it is often desirable that clicking on the anchor always closes
    /// the menu since the anchor area isn't usually considered part of the menu
    /// system by the user. In this case
    /// [`anchor_tap_closes_menu`](Self::anchor_tap_closes_menu) should be true.
    ///
    /// Defaults to false.
    #[deprecated(
        note = "Use consumeOutsideTap instead. This feature was deprecated after \
                v3.16.0-8.0.pre."
    )]
    pub anchor_tap_closes_menu: bool,

    /// Whether or not a tap event that closes the menu will be permitted to
    /// continue on to the gesture arena.
    ///
    /// If false, then tapping outside of a menu when the menu is open will both
    /// close the menu, and allow the tap to participate in the gesture arena. If
    /// true, then it will only close the menu, and the tap event will be
    /// consumed.
    ///
    /// Defaults to false.
    pub consume_outside_tap: bool,

    /// A callback that is invoked when the menu begins opening.
    ///
    /// Defaults to null.
    pub on_open: Option<VoidCallback>,

    /// A callback that is invoked when the menu finishes closing.
    ///
    /// Defaults to null.
    pub on_close: Option<VoidCallback>,

    /// Determine if the menu panel can be wrapped by a `UnconstrainedBox` which allows
    /// the panel to render at its "natural" size.
    ///
    /// Defaults to true as it allows developers to render the menu panel at the
    /// size it should be. When it is set to false, it can be useful when the menu should
    /// be constrained in both main axis and cross axis, such as a `DropdownMenu`.
    pub cross_axis_unconstrained: bool,

    /// Whether the menu panel should be rendered in the root `Overlay`.
    ///
    /// See [`RawMenuAnchor::use_root_overlay`].
    ///
    /// Defaults to false.
    pub use_root_overlay: bool,

    /// Whether this widget should open or close a submenu with an animation.
    ///
    /// Defaults to false.
    pub animated: bool,

    /// An optional callback that is invoked when the [`AnimationStatus`] of the
    /// menu changes during open and close animations.
    ///
    /// If [`animated`](Self::animated) is false, this callback will only be invoked with
    /// [`AnimationStatus::Completed`] when the menu is opened, and
    /// [`AnimationStatus::Dismissed`] when the menu is closed.
    ///
    /// This callback provides a way to determine when the menu is opening or
    /// closing. This is necessary because the [`MenuControllerLeaf::is_open`] property
    /// remains true throughout the opening, opened, and closing phases, and
    /// therefore cannot be used on its own to determine the current animation
    /// direction.
    ///
    /// Defaults to null.
    pub on_animation_status_changed: Option<ValueChanged<AnimationStatus>>,

    /// A list of children containing the menu items that are the contents of the
    /// menu surrounded by this [`MenuAnchor`].
    ///
    /// Menus using [`MenuItemButton`] can have a `SingleActivator` or `CharacterActivator`
    /// assigned to them as their [`MenuItemButton::shortcut`], which will display an
    /// appropriate shortcut hint. Even though the shortcut labels are displayed in the menu,
    /// shortcuts are not automatically handled. They must be available in whatever context
    /// they are appropriate, and handled via another mechanism.
    pub menu_children: Vec<WidgetRef>,

    /// The widget that this [`MenuAnchor`] surrounds.
    ///
    /// Typically this is a button used to open the menu by calling
    /// [`MenuControllerLeaf::open`] on the `controller` passed to the builder.
    ///
    /// If not supplied, then the [`MenuAnchor`] will be the size that its parent
    /// allocates for it.
    ///
    /// If provided, the builder will be called each time the menu is opened or
    /// closed.
    pub builder: Option<MenuAnchorChildBuilder>,

    /// The semantic label of the menu.
    ///
    /// Defaults to null.
    pub semantic_label: Option<String>,

    /// The optional child to be passed to the [`builder`](Self::builder).
    ///
    /// Supply this child if there is a portion of the widget tree built in
    /// [`builder`](Self::builder) that doesn't depend on the `controller` or `context`
    /// supplied to the [`builder`](Self::builder). It will be more efficient, since Flutter
    /// doesn't then need to rebuild this child when those change.
    pub child: Option<WidgetRef>,

    /// The padding between the edge of the safe area and the menu panel.
    ///
    /// Defaults to `EdgeInsets::all(8)`.
    pub reserved_padding: Option<EdgeInsetsGeometry>,
}

impl MenuAnchor {
    /// Creates a const [`MenuAnchor`].
    ///
    /// The [`menu_children`](Self::menu_children) argument is required.
    pub fn new(menu_children: Vec<WidgetRef>) -> MenuAnchor {
        MenuAnchor {
            key: None,
            controller: None,
            child_focus_node: None,
            style: None,
            alignment_offset: Some(Offset::ZERO),
            reserved_padding: None,
            layer_link: None,
            clip_behavior: Clip::HardEdge,
            anchor_tap_closes_menu: false,
            consume_outside_tap: false,
            on_open: None,
            on_close: None,
            cross_axis_unconstrained: true,
            use_root_overlay: false,
            animated: false,
            on_animation_status_changed: None,
            menu_children,
            builder: None,
            semantic_label: None,
            child: None,
        }
    }

    /// Dart `MenuAnchor(key:)`.
    pub fn key(mut self, key: KeyRef) -> MenuAnchor {
        self.key = Some(key);
        self
    }

    /// Dart `MenuAnchor(controller:)`.
    pub fn controller(mut self, controller: MenuControllerRef) -> MenuAnchor {
        self.controller = Some(controller);
        self
    }

    /// Dart `MenuAnchor(childFocusNode:)`.
    pub fn child_focus_node(mut self, child_focus_node: FocusNodeRef) -> MenuAnchor {
        self.child_focus_node = Some(child_focus_node);
        self
    }

    /// Dart `MenuAnchor(style:)`.
    pub fn style(mut self, style: MenuStyle) -> MenuAnchor {
        self.style = Some(style);
        self
    }

    /// Dart `MenuAnchor(alignmentOffset:)`.
    pub fn alignment_offset(mut self, alignment_offset: Offset) -> MenuAnchor {
        self.alignment_offset = Some(alignment_offset);
        self
    }

    /// Dart `MenuAnchor(reservedPadding:)`.
    pub fn reserved_padding(mut self, reserved_padding: EdgeInsetsGeometry) -> MenuAnchor {
        self.reserved_padding = Some(reserved_padding);
        self
    }

    /// Dart `MenuAnchor(layerLink:)`.
    pub fn layer_link(mut self, layer_link: Ref<LayerLink>) -> MenuAnchor {
        self.layer_link = Some(layer_link);
        self
    }

    /// Dart `MenuAnchor(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> MenuAnchor {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `MenuAnchor(anchorTapClosesMenu:)`.
    #[deprecated(
        note = "Use consumeOutsideTap instead. This feature was deprecated after \
                v3.16.0-8.0.pre."
    )]
    pub fn anchor_tap_closes_menu(mut self, anchor_tap_closes_menu: bool) -> MenuAnchor {
        self.anchor_tap_closes_menu = anchor_tap_closes_menu;
        self
    }

    /// Dart `MenuAnchor(consumeOutsideTap:)`.
    pub fn consume_outside_tap(mut self, consume_outside_tap: bool) -> MenuAnchor {
        self.consume_outside_tap = consume_outside_tap;
        self
    }

    /// Dart `MenuAnchor(onOpen:)`.
    pub fn on_open(mut self, on_open: VoidCallback) -> MenuAnchor {
        self.on_open = Some(on_open);
        self
    }

    /// Dart `MenuAnchor(onClose:)`.
    pub fn on_close(mut self, on_close: VoidCallback) -> MenuAnchor {
        self.on_close = Some(on_close);
        self
    }

    /// Dart `MenuAnchor(crossAxisUnconstrained:)`.
    pub fn cross_axis_unconstrained(mut self, cross_axis_unconstrained: bool) -> MenuAnchor {
        self.cross_axis_unconstrained = cross_axis_unconstrained;
        self
    }

    /// Dart `MenuAnchor(useRootOverlay:)`.
    pub fn use_root_overlay(mut self, use_root_overlay: bool) -> MenuAnchor {
        self.use_root_overlay = use_root_overlay;
        self
    }

    /// Dart `MenuAnchor(animated:)`.
    pub fn animated(mut self, animated: bool) -> MenuAnchor {
        self.animated = animated;
        self
    }

    /// Dart `MenuAnchor(onAnimationStatusChanged:)`.
    pub fn on_animation_status_changed(
        mut self,
        on_animation_status_changed: ValueChanged<AnimationStatus>,
    ) -> MenuAnchor {
        self.on_animation_status_changed = Some(on_animation_status_changed);
        self
    }

    /// Dart `MenuAnchor(builder:)`.
    pub fn builder(
        mut self,
        builder: impl Fn(&mut App, BuildContext, AnyMenuController, Option<&WidgetRef>) -> WidgetRef
        + 'static,
    ) -> MenuAnchor {
        self.builder = Some(Rc::new(builder));
        self
    }

    /// Dart `MenuAnchor(semanticLabel:)`.
    pub fn semantic_label(mut self, semantic_label: impl Into<String>) -> MenuAnchor {
        self.semantic_label = Some(semantic_label.into());
        self
    }

    /// Dart `MenuAnchor(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> MenuAnchor {
        self.child = Some(child.into_widget());
        self
    }
}

impl Debug for MenuAnchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("MenuAnchor");
        if self.anchor_tap_closes_menu {
            properties.field("anchorTapClosesMenu", &"AUTO-CLOSE");
        }
        properties
            .field("focusNode", &self.child_focus_node)
            .field("style", &self.style)
            .field("clipBehavior", &self.clip_behavior)
            .field("alignmentOffset", &self.alignment_offset)
            .field("menuChildren", &self.menu_children)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for MenuAnchor {
    type State = MenuAnchorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> MenuAnchorState {
        MenuAnchorState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            menu_anchor_state: MenuAnchorStateData::new(),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// _MenuAnchorState

/// The fields of Dart's `_MenuAnchorState`, which its subclass `_MenuBarAnchorState` shares;
/// each state carries them under the field `menu_anchor_state`.
struct MenuAnchorStateData {
    internal_menu_controller: Option<Ref<MenuController>>,
    // Dart's field initializers; an arena object needs the `App`, so `init_state` makes them.
    menu_scope_node: Option<Ref<FocusScopeNode>>,
    animation_controller: Option<Ref<AnimationController>>,
    height_animation: Option<Ref<CurvedAnimation>>,
    opacity_animation: Option<Ref<CurvedAnimation>>,
    menu_children: Vec<WidgetRef>,
    cached_animations: Vec<Ref<CurvedAnimation>>,
}

impl MenuAnchorStateData {
    fn new() -> MenuAnchorStateData {
        MenuAnchorStateData {
            internal_menu_controller: None,
            menu_scope_node: None,
            animation_controller: None,
            height_animation: None,
            opacity_animation: None,
            menu_children: Vec::new(),
            cached_animations: Vec::new(),
        }
    }
}

/// Dart's `_MenuAnchorState`: the state of a [`MenuAnchor`].
pub struct MenuAnchorState {
    state: StateData<MenuAnchor>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    menu_anchor_state: MenuAnchorStateData,
}

/// The base class Dart's `_MenuAnchorState` is to `_MenuBarAnchorState`: its fields, its
/// overridable members and its bodies. A leaf's `State` hooks call this trait's
/// `init_state`, `did_update_widget`, `dispose` and `build` by name, where Dart runs the base
/// class's body through `super`.
trait MenuAnchorStateLeaf: SingleTickerProviderStateMixin + TickerProviderObject {
    fn menu_anchor_state_data(self: Handle<Self>, app: &App) -> &MenuAnchorStateData;

    fn menu_anchor_state_data_mut(self: Handle<Self>, app: &mut App) -> &mut MenuAnchorStateData;

    /// The state's widget as the [`MenuAnchor`] Dart's `widget` is.
    fn menu_anchor(self: Handle<Self>, app: &App) -> &MenuAnchor;

    /// Dart's `_orientation`.
    fn orientation() -> Axis {
        Axis::Vertical
    }

    /// This state as the erased [`AnyMenuAnchorState`] a `_MenuAnchorScope` holds.
    fn as_menu_anchor_state(self: Handle<Self>) -> AnyMenuAnchorState {
        AnyMenuAnchorState {
            id: self.id(),
            vtable: const { &MenuAnchorStateVTable::of::<Self>() },
        }
    }

    fn animation_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        self.menu_anchor_state_data(app)
            .animation_controller
            .handle()
            .expect("created in init_state")
    }

    /// Dart's `initState` body.
    fn init_state(self: Handle<Self>, app: &mut App) {
        let menu_scope_node = FocusScopeNode::new(app);
        let animation_controller = AnimationController::new(
            app,
            None,
            None,
            None,
            0.0,
            1.0,
            waft_animation::AnimationBehavior::Normal,
            self,
        );
        let height_animation = CurvedAnimation::new(
            app,
            animation_controller.erase(),
            menu_panel_height_forward_curve(),
            Some(menu_panel_height_reverse_curve()),
        );
        let opacity_animation = CurvedAnimation::new(
            app,
            animation_controller.erase(),
            menu_panel_opacity_forward_curve(),
            Some(menu_panel_opacity_reverse_curve()),
        );
        let data = self.menu_anchor_state_data_mut(app);
        data.menu_scope_node = Some(menu_scope_node);
        data.animation_controller = Some(animation_controller.clone());
        data.height_animation = Some(height_animation);
        data.opacity_animation = Some(opacity_animation);

        self.resolve_animation_controller(app);
        self.resolve_menu_items(app);
        animation_controller.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, Self::handle_animation_status_changed),
        );

        if self.menu_anchor(app).controller.is_none() {
            let controller = MenuController::new(app);
            self.menu_anchor_state_data_mut(app)
                .internal_menu_controller = Some(controller);
        }
    }

    /// Dart's `didUpdateWidget` body.
    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MenuAnchor) {
        let widget = self.menu_anchor(app);
        let controller = widget.controller.clone();
        if old_widget.controller != controller {
            let internal_menu_controller = if controller.is_none() {
                Some(MenuController::new(app))
            } else {
                None
            };
            self.menu_anchor_state_data_mut(app)
                .internal_menu_controller = internal_menu_controller;
        }

        let widget = self.menu_anchor(app);
        if old_widget.animated != widget.animated
            || !same_widgets(&widget.menu_children, &old_widget.menu_children)
        {
            self.resolve_menu_items(app);
        }

        if old_widget.animated != self.menu_anchor(app).animated {
            self.resolve_animation_controller(app);
        }
    }

    /// Dart's `dispose` body, up to `super.dispose()`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        let data = self.menu_anchor_state_data_mut(app);
        data.menu_children.clear();
        let cached_animations = std::mem::take(&mut data.cached_animations);
        let menu_scope_node = data
            .menu_scope_node
            .handle()
            .expect("created in init_state");
        let height_animation = data
            .height_animation
            .handle()
            .expect("created in init_state");
        let opacity_animation = data
            .opacity_animation
            .handle()
            .expect("created in init_state");
        let animation_controller = data
            .animation_controller
            .handle()
            .expect("created in init_state");
        for animation in cached_animations {
            animation.dispose(app);
        }
        menu_scope_node.as_node().dispose(app);
        height_animation.dispose(app);
        opacity_animation.dispose(app);
        animation_controller.stop(app, true);
        animation_controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    /// Dart's `_resolveAnimationController`.
    fn resolve_animation_controller(self: Handle<Self>, app: &mut App) {
        let animated = self.menu_anchor(app).animated;
        let controller = app.get_mut(self.animation_controller(app));
        if animated {
            controller.duration = Some(K_MENU_OPENING_DURATION);
            controller.reverse_duration = Some(K_MENU_CLOSING_DURATION);
        } else {
            controller.duration = Some(Duration::ZERO);
            controller.reverse_duration = Some(Duration::ZERO);
        }
    }

    /// Dart's `_resolveMenuItems`.
    fn resolve_menu_items(self: Handle<Self>, app: &mut App) {
        let data = self.menu_anchor_state_data_mut(app);
        data.menu_children = Vec::new();
        let cached_animations = std::mem::take(&mut data.cached_animations);
        for animation in cached_animations {
            animation.dispose(app);
        }

        let widget = self.menu_anchor(app);
        let item_count = widget.menu_children.len();
        if item_count == 0 {
            return;
        }

        if !widget.animated {
            let menu_children = widget.menu_children.clone();
            self.menu_anchor_state_data_mut(app).menu_children = menu_children;
            return;
        }

        const FORWARD_FINAL_ITEM_OFFSET: f64 = 1.0 - K_MENU_ITEM_RELATIVE_FADE_IN_DURATION;
        const REVERSE_FINAL_ITEM_OFFSET: f64 =
            1.0 - K_MENU_ITEM_RELATIVE_FADE_OUT_DURATION - K_MENU_ITEM_RELATIVE_FADE_OUT_DELAY;

        let mut forward_progress = 0.0;
        let mut reverse_progress = 0.0;
        let mut item_fade_in_gap = 0.0;
        let mut item_fade_out_gap = 0.0;
        if item_count > 1 {
            // Spread every item evenly across the remaining time after accounting for
            // the fade in/out durations.
            item_fade_in_gap = FORWARD_FINAL_ITEM_OFFSET / (item_count - 1) as f64;
            item_fade_out_gap = REVERSE_FINAL_ITEM_OFFSET / (item_count - 1) as f64;
        }

        let parent: AnimationRef<f64> = self
            .menu_anchor_state_data(app)
            .animation_controller
            .as_ref()
            .expect("created in init_state")
            .erase();
        for child in widget.menu_children.clone() {
            let forward_curve = Interval::new(
                forward_progress,
                forward_progress + K_MENU_ITEM_RELATIVE_FADE_IN_DURATION,
                waft_animation::Curves::linear(),
            );

            let reverse_curve = Interval::new(
                reverse_progress,
                reverse_progress + K_MENU_ITEM_RELATIVE_FADE_OUT_DURATION,
                waft_animation::Curves::linear(),
            );

            let animation = CurvedAnimation::new(
                app,
                parent.clone(),
                Rc::new(forward_curve),
                Some(Rc::new(reverse_curve)),
            );

            let data = self.menu_anchor_state_data_mut(app);
            data.cached_animations.push(animation.clone());
            data.menu_children.push(
                FadeTransition::new(animation.erase())
                    .child(child)
                    .into_widget(),
            );
            forward_progress += item_fade_in_gap;
            reverse_progress += item_fade_out_gap;
        }
    }

    /// Dart's `_handleAnimationStatusChanged`.
    fn handle_animation_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        self.set_state(app, |_| {
            // Rebuild to update isClosedOrClosing and notify dependents of AnimationStatus changes.
        });
        if let Some(on_animation_status_changed) =
            self.menu_anchor(app).on_animation_status_changed.clone()
        {
            on_animation_status_changed(app, status);
        }
    }

    /// Dart's `_handleMenuOpenRequest`.
    fn handle_menu_open_request(
        self: Handle<Self>,
        app: &mut App,
        _position: Option<Offset>,
        show_overlay: VoidCallback,
    ) {
        let this = self.as_menu_anchor_state();
        // If this menu's parent is closing, submenus should not open. This prevents
        // a submenu calling MenuController.open() after a parent menu has started
        // closing.
        if this
            .parent(app)
            .is_some_and(|parent| parent.is_closing(app))
        {
            return;
        }

        show_overlay(app);

        let animation_controller = self.animation_controller(app);
        if animation_controller.status(app).is_forward_or_completed() {
            return;
        }

        animation_controller.forward(app, None);

        if this.is_submenu(app) || self.menu_anchor(app).child_focus_node.is_some() {
            return;
        }

        let context = self.context(app);
        let scope = FocusScope::of(app, context, true);
        let focused_context = scope
            .focused_child(app)
            .and_then(|focused_child| focused_child.context(app));
        let is_anchor_focused = focused_context.is_some_and(|focused_context| {
            AnyMenuAnchorState::maybe_of(app, focused_context)
                .is_some_and(|anchor| anchor.root(app) == this)
        });
        if is_anchor_focused {
            return;
        }

        let initial_focus = FocusManager::instance(app).primary_focus(app);
        let view = View::maybe_id_of(app, context);
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _timestamp| {
                if self.mounted(app)
                    && !this.is_closing_or_closed(app)
                    && FocusManager::instance(app).primary_focus(app) == initial_focus
                {
                    let menu_scope_node = this.menu_scope_node(app);
                    scope.set_first_focus(app, menu_scope_node);
                }
            }),
            view,
        );
    }

    /// Dart's `_handleMenuCloseRequest`.
    fn handle_menu_close_request(self: Handle<Self>, app: &mut App, hide_overlay: VoidCallback) {
        let animation_controller = self.animation_controller(app);
        if !animation_controller.status(app).is_forward_or_completed() {
            return;
        }

        animation_controller
            .reverse(app, None)
            .when_complete(app, Listener::new(move |app| hide_overlay(app)));
    }

    /// Dart's `build`, which `_MenuBarAnchorState` overrides.
    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let this = self.as_menu_anchor_state();
        let widget = self.menu_anchor(app);
        let mut anchor =
            RawMenuAnchor::new(this.menu_controller(app), move |app, context, info| {
                self.build_overlay(app, context, info)
            })
            .on_open_requested(Rc::new(move |app, position, show_overlay| {
                self.handle_menu_open_request(app, position, show_overlay);
            }))
            .on_close_requested(Rc::new(move |app, hide_overlay| {
                self.handle_menu_close_request(app, hide_overlay);
            }))
            .use_root_overlay(widget.use_root_overlay)
            .consume_outside_taps(widget.consume_outside_tap);
        anchor.on_open = widget.on_open.clone();
        anchor.on_close = widget.on_close.clone();
        anchor.child_focus_node = widget.child_focus_node.clone();
        anchor.builder = widget.builder.clone();
        anchor.child = widget.child.clone();
        let layer_link = widget.layer_link.clone();
        let child = MenuAnchorScope {
            state: this,
            animation_status: self.animation_controller(app).status(app),
            child: anchor.into_widget(),
        }
        .into_widget();

        match layer_link {
            None => child,
            Some(layer_link) => CompositedTransformTarget::new(layer_link)
                .child(child)
                .into_widget(),
        }
    }

    /// Dart's `_buildOverlay`.
    fn build_overlay(
        self: Handle<Self>,
        app: &mut App,
        _context: BuildContext,
        position: RawMenuOverlayInfo,
    ) -> WidgetRef {
        let this = self.as_menu_anchor_state();
        let data = self.menu_anchor_state_data(app);
        let fade_animation: AnimationRef<f64> = data
            .opacity_animation
            .as_ref()
            .expect("created in init_state")
            .erase();
        let height_animation: AnimationRef<f64> = data
            .height_animation
            .as_ref()
            .expect("created in init_state")
            .erase();
        let menu_scope_node = data.menu_scope_node.clone().expect("created in init_state");
        let menu_children = data.menu_children.clone();
        let widget = self.menu_anchor(app);
        let mut submenu = Submenu {
            fade_animation,
            height_animation,
            layer_link: widget.layer_link.clone(),
            consume_outside_taps: widget.consume_outside_tap,
            menu_scope_node,
            menu_style: widget.style.clone(),
            clip_behavior: widget.clip_behavior,
            menu_children,
            cross_axis_unconstrained: widget.cross_axis_unconstrained,
            menu_position: position,
            anchor: this,
            alignment_offset: widget.alignment_offset.unwrap_or(Offset::ZERO),
            reserved_padding: widget
                .reserved_padding
                .unwrap_or(EdgeInsetsGeometry::all(K_MENU_VIEW_PADDING)),
        }
        .into_widget();

        // Only inject the semantics node if a label is provided
        if let Some(semantic_label) = widget.semantic_label.clone() {
            submenu = Semantics::new()
                .container(true)
                .explicit_child_nodes(true)
                .label(semantic_label)
                .child(submenu)
                .into_widget();
        }

        // ExcludeSemantics, ExcludeFocus, and IgnorePointer are used to effectively
        // disable all interactions with the menu while it is closing.
        //
        // An animated menu should behave the same as a menu without animations.
        // Focus should be able to move to a menu as soon as it starts to open, and
        // a menu should not be interactive as soon as it starts to close.
        let is_closing_or_closed = this.is_closing_or_closed(app);
        ExcludeSemantics::new()
            .excluding(is_closing_or_closed)
            .child(
                IgnorePointer::new()
                    .ignoring(is_closing_or_closed)
                    .child(ExcludeFocus::new(submenu).excluding(is_closing_or_closed)),
            )
            .into_widget()
    }
}

impl MenuAnchorStateLeaf for MenuAnchorState {
    fn menu_anchor_state_data(self: Handle<Self>, app: &App) -> &MenuAnchorStateData {
        &app.get(self).menu_anchor_state
    }

    fn menu_anchor_state_data_mut(self: Handle<Self>, app: &mut App) -> &mut MenuAnchorStateData {
        &mut app.get_mut(self).menu_anchor_state
    }

    fn menu_anchor(self: Handle<Self>, app: &App) -> &MenuAnchor {
        self.widget(app)
    }
}

impl SingleTickerProviderStateMixin for MenuAnchorState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for MenuAnchorState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for MenuAnchorState {
    type Widget = MenuAnchor;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        MenuAnchorStateLeaf::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MenuAnchor) {
        MenuAnchorStateLeaf::did_update_widget(self, app, old_widget);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        MenuAnchorStateLeaf::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        MenuAnchorStateLeaf::build(self, app, context)
    }
}

impl Debug for MenuAnchorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_MenuAnchorState")
    }
}

/// The vtable of an erased [`AnyMenuAnchorState`]: one `&'static` table per leaf type.
struct MenuAnchorStateVTable {
    type_name: fn() -> &'static str,
    data: fn(&App, HandleId) -> &MenuAnchorStateData,
    widget: fn(&App, HandleId) -> &MenuAnchor,
    orientation: fn() -> Axis,
    context: fn(&App, HandleId) -> BuildContext,
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn typed_handle<L: 'static>(id: HandleId) -> Handle<L> {
    Handle::from_id(id)
}

impl MenuAnchorStateVTable {
    const fn of<L: MenuAnchorStateLeaf>() -> MenuAnchorStateVTable {
        MenuAnchorStateVTable {
            type_name: std::any::type_name::<L>,
            data: |app, id| L::menu_anchor_state_data(typed_handle(id), app),
            widget: |app, id| L::menu_anchor(typed_handle(id), app),
            orientation: L::orientation,
            context: |app, id| L::context(typed_handle(id), app),
        }
    }
}

/// Erased `_MenuAnchorState`: what a field or parameter Dart types as `_MenuAnchorState`
/// becomes, a [`MenuAnchorState`] or a menu bar's state.
///
/// Its methods are Dart's members that no subclass overrides.
#[derive(Clone, Copy)]
struct AnyMenuAnchorState {
    id: HandleId,
    vtable: &'static MenuAnchorStateVTable,
}

impl PartialEq for AnyMenuAnchorState {
    fn eq(&self, other: &AnyMenuAnchorState) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyMenuAnchorState {}

impl Debug for AnyMenuAnchorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyMenuAnchorState {
    fn data(self, app: &App) -> &MenuAnchorStateData {
        (self.vtable.data)(app, self.id)
    }

    /// Dart's `widget`.
    fn widget(self, app: &App) -> &MenuAnchor {
        (self.vtable.widget)(app, self.id)
    }

    /// Dart's `context`.
    fn context(self, app: &App) -> BuildContext {
        (self.vtable.context)(app, self.id)
    }

    /// Dart's `_orientation`.
    fn orientation(self) -> Axis {
        (self.vtable.orientation)()
    }

    /// Dart's `_menuController`.
    fn menu_controller(self, app: &App) -> MenuControllerRef {
        self.widget(app).controller.clone().unwrap_or_else(|| {
            self.data(app)
                .internal_menu_controller
                .as_ref()
                .expect("made when the widget has no controller")
                .erase()
        })
    }

    /// Dart's `_menuScopeNode`.
    fn menu_scope_node(self, app: &App) -> Handle<FocusScopeNode> {
        self.data(app)
            .menu_scope_node
            .handle()
            .expect("created in init_state")
    }

    fn animation_controller(self, app: &App) -> Handle<AnimationController> {
        self.data(app)
            .animation_controller
            .handle()
            .expect("created in init_state")
    }

    /// Dart's `_parent`.
    fn parent(self, app: &App) -> Option<AnyMenuAnchorState> {
        AnyMenuAnchorState::maybe_of(app, self.context(app))
    }

    /// Dart's `isSubmenu`.
    fn is_submenu(self, app: &App) -> bool {
        MenuController::maybe_of(app, self.context(app)).is_some()
    }

    /// Dart's `isClosingOrClosed`.
    fn is_closing_or_closed(self, app: &App) -> bool {
        match self.animation_controller(app).status(app) {
            AnimationStatus::Dismissed | AnimationStatus::Reverse => true,
            AnimationStatus::Forward | AnimationStatus::Completed => false,
        }
    }

    /// Dart's `isClosing`.
    fn is_closing(self, app: &App) -> bool {
        match self.animation_controller(app).status(app) {
            AnimationStatus::Reverse => true,
            AnimationStatus::Dismissed | AnimationStatus::Forward | AnimationStatus::Completed => {
                false
            }
        }
    }

    /// Dart's `_root`.
    fn root(self, app: &App) -> AnyMenuAnchorState {
        let mut anchor = self;
        while let Some(parent) = anchor.parent(app) {
            anchor = parent;
        }
        anchor
    }

    /// Dart's `_focusButton`.
    fn focus_button(self, app: &mut App) {
        let Some(child_focus_node) = self.widget(app).child_focus_node.clone() else {
            return;
        };
        child_focus_node.request_focus(app, None);
    }

    /// Dart's `_focusFirstMenuItem`.
    fn focus_first_menu_item(self, app: &mut App) {
        let menu_scope_node = self.menu_scope_node(app);
        let Some(context) = menu_scope_node
            .as_node()
            .context(app)
            .filter(|context| context.mounted(app))
        else {
            return;
        };
        let (policy, _made) = traversal_policy_of(app, context);
        let first_focus = policy.find_first_focus(app, menu_scope_node.as_node(), true);
        if let Some(first_focus) = first_focus {
            first_focus.request_focus(app, None);
        }
    }

    /// Dart's `_focusLastMenuItem`.
    fn focus_last_menu_item(self, app: &mut App) {
        let menu_scope_node = self.menu_scope_node(app);
        let Some(context) = menu_scope_node
            .as_node()
            .context(app)
            .filter(|context| context.mounted(app))
        else {
            return;
        };
        let (policy, _made) = traversal_policy_of(app, context);
        let last_focus = policy.find_last_focus(app, menu_scope_node.as_node(), true);
        last_focus.request_focus(app, None);
    }

    /// Dart's `_maybeOf`.
    fn maybe_of(app: &App, context: BuildContext) -> Option<AnyMenuAnchorState> {
        context
            .get_inherited_widget_of_exact_type::<MenuAnchorScope>(app)
            .map(|scope| scope.state)
    }

    /// Dart's `_maybeAnimationStatusOf`.
    fn maybe_animation_status_of(app: &mut App, context: BuildContext) -> Option<AnimationStatus> {
        context
            .depend_on_inherited_widget_of_exact_type::<MenuAnchorScope>(app)
            .map(|scope| scope.animation_status)
    }
}

/// Dart's `FocusTraversalGroup.maybeOf(context) ?? ReadingOrderTraversalPolicy()`, with the
/// policy it made, if any, for the caller to keep while it uses the first.
fn traversal_policy_of(
    app: &mut App,
    context: BuildContext,
) -> (
    waft_widgets::AnyFocusTraversalPolicy,
    Option<Ref<ReadingOrderTraversalPolicy>>,
) {
    if let Some(policy) = FocusTraversalGroup::maybe_of(app, context) {
        return (policy, None);
    }
    let policy = ReadingOrderTraversalPolicy::new(app);
    (policy.as_policy(), Some(policy))
}

// ---------------------------------------------------------------------------------------------
// MenuBar

/// A menu bar that manages cascading child menus.
///
/// This is a Material Design menu bar that typically resides above the main
/// body of an application (but can go anywhere) that defines a menu system for
/// invoking callbacks in response to user selection of a menu item.
///
/// The menus can be opened with a click or tap. Once a menu is opened, it can
/// be navigated by using the arrow and tab keys or via mouse hover. Selecting a
/// menu item can be done by pressing enter, or by clicking or tapping on the
/// menu item. Clicking or tapping on any part of the user interface that isn't
/// part of the menu system controlled by the same controller will cause all of
/// the menus controlled by that controller to close, as will pressing the
/// escape key.
///
/// When a menu item with a submenu is clicked on, it toggles the visibility of
/// the submenu. When the menu item is hovered over, the submenu will open, and
/// hovering over other items will close the previous menu and open the newly
/// hovered one. When those open/close transitions occur,
/// [`SubmenuButton::on_open`], and [`SubmenuButton::on_close`] are called on the
/// corresponding [`SubmenuButton`] child of the menu bar.
///
/// Menus using [`MenuItemButton`] can have a `SingleActivator` or
/// `CharacterActivator` assigned to them as their [`MenuItemButton::shortcut`],
/// which will display an appropriate shortcut hint. Even though the shortcut
/// labels are displayed in the menu, shortcuts are not automatically handled.
/// They must be available in whatever context they are appropriate, and handled
/// via another mechanism.
///
/// If shortcuts should be generally enabled, but are not easily defined in a
/// context surrounding the menu bar, consider registering them with a
/// `ShortcutRegistry` (one is already included in the `WidgetsApp`, and thus
/// also `MaterialApp` and `CupertinoApp`). To be sure that selecting a menu item
/// and triggering the shortcut do the same thing, it is recommended that they
/// call the same callback.
///
/// See also:
///
/// * [`MenuAnchor`], a widget that creates a region with a submenu and shows it
///   when requested.
/// * [`SubmenuButton`], a menu item which manages a submenu.
/// * [`MenuItemButton`], a leaf menu item which displays the label, an optional
///   shortcut label, and optional leading and trailing icons.
/// * `PlatformMenuBar`, which creates a menu bar that is rendered by the host
///   platform instead of by Flutter (on macOS, for example).
/// * `ShortcutRegistry`, a registry of shortcuts that apply for the entire
///   application.
/// * `VoidCallbackIntent`, to define intents that will call a `VoidCallback` and
///   work with the `Actions` and `Shortcuts` system.
/// * `CallbackShortcuts`, to define shortcuts that call a callback without
///   involving `Actions`.
pub struct MenuBar {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,

    /// The [`MenuStyle`] that defines the visual attributes of the menu bar.
    ///
    /// Colors and sizing of the menus is controllable via the [`MenuStyle`].
    ///
    /// Defaults to the ambient `MenuThemeData::style`.
    pub style: Option<MenuStyle>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to `Clip::None`.
    pub clip_behavior: Clip,

    /// The [`MenuController`] to use for this menu bar.
    pub controller: Option<MenuControllerRef>,

    /// The list of menu items that are the top level children of the [`MenuBar`].
    ///
    /// A Widget in Flutter is immutable, so directly modifying the [`children`](Self::children)
    /// with `Vec` APIs will result in incorrect behaviors. Whenever the menus list is modified,
    /// a new list object must be provided.
    pub children: Vec<WidgetRef>,
}

impl MenuBar {
    /// Creates a const [`MenuBar`].
    ///
    /// The [`children`](Self::children) argument is required.
    pub fn new(children: Vec<WidgetRef>) -> MenuBar {
        MenuBar {
            key: None,
            style: None,
            clip_behavior: Clip::None,
            controller: None,
            children,
        }
    }

    /// Dart `MenuBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> MenuBar {
        self.key = Some(key);
        self
    }

    /// Dart `MenuBar(style:)`.
    pub fn style(mut self, style: MenuStyle) -> MenuBar {
        self.style = Some(style);
        self
    }

    /// Dart `MenuBar(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> MenuBar {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `MenuBar(controller:)`.
    pub fn controller(mut self, controller: MenuControllerRef) -> MenuBar {
        self.controller = Some(controller);
        self
    }
}

impl Debug for MenuBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuBar")
            .field("style", &self.style)
            .field("clipBehavior", &self.clip_behavior)
            .field("children", &self.children)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for MenuBar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_overlay(app, context));
        let mut menu_anchor =
            MenuAnchor::new(self.children.clone()).clip_behavior(self.clip_behavior);
        menu_anchor.controller = self.controller.clone();
        menu_anchor.style = self.style.clone();
        MenuBarAnchor { menu_anchor }.into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// MenuItemButton

/// A button for use in a [`MenuBar`], in a menu created with [`MenuAnchor`], or on
/// its own, that can be activated by click or keyboard navigation.
///
/// This widget represents a leaf entry in a menu hierarchy that is typically
/// part of a [`MenuBar`], but may be used independently, or as part of a menu
/// created with a [`MenuAnchor`].
///
/// Menus using [`MenuItemButton`] can have a `SingleActivator` or `CharacterActivator`
/// assigned to them as their [`shortcut`](Self::shortcut), which will display an
/// appropriate shortcut hint. Even though the shortcut labels are displayed in the menu,
/// shortcuts are not automatically handled. They must be available in whatever context they
/// are appropriate, and handled via another mechanism.
///
/// See also:
///
/// * [`MenuBar`], a class that creates a top level menu bar in a Material Design
///   style.
/// * [`MenuAnchor`], a widget that creates a region with a submenu and shows it
///   when requested.
/// * [`SubmenuButton`], a menu item similar to this one which manages a submenu.
/// * `PlatformMenuBar`, which creates a menu bar that is rendered by the host
///   platform instead of by Flutter (on macOS, for example).
/// * `ShortcutRegistry`, a registry of shortcuts that apply for the entire
///   application.
/// * `VoidCallbackIntent`, to define intents that will call a `VoidCallback` and
///   work with the `Actions` and `Shortcuts` system.
/// * `CallbackShortcuts` to define shortcuts that call a callback without
///   involving `Actions`.
pub struct MenuItemButton {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,

    /// Called when the button is tapped or otherwise activated.
    ///
    /// If this callback is null, then the button will be disabled.
    ///
    /// See also:
    ///
    ///  * [`enabled`](Self::enabled), which is true if the button is enabled.
    pub on_pressed: Option<Listener>,

    /// Called when a pointer enters or exits the button response area.
    ///
    /// The value passed to the callback is true if a pointer has entered button
    /// area and false if a pointer has exited.
    pub on_hover: Option<ValueChanged<bool>>,

    /// Determine if hovering can request focus.
    ///
    /// Defaults to true.
    pub request_focus_on_hover: bool,

    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's node gains focus, and false if it loses
    /// focus.
    pub on_focus_change: Option<ValueChanged<bool>>,

    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,

    /// True if this widget will be selected as the initial focus when no other
    /// node in its scope is currently focused.
    pub autofocus: bool,

    /// The optional shortcut that selects this [`MenuItemButton`].
    ///
    /// Even though the shortcut label is displayed in the menu, the shortcut is not
    /// automatically handled; see [`MenuBar`].
    pub shortcut: Option<MenuSerializableShortcutRef>,

    /// An optional Semantics label, applied to the entire [`MenuItemButton`].
    ///
    /// A screen reader will default to reading the derived text on the
    /// [`MenuItemButton`] itself, which is not guaranteed to be readable.
    /// (For some shortcuts, such as comma, semicolon, and other
    /// punctuation, screen readers read silence).
    ///
    /// Setting this label overwrites the semantics properties of the entire
    /// Widget, including its children. Consider wrapping this widget in
    /// `Semantics` if you want to customize other properties besides just
    /// the label.
    ///
    /// Null by default.
    pub semantics_label: Option<String>,

    /// Customizes this button's appearance.
    ///
    /// Non-null properties of this style override the corresponding properties in
    /// [`theme_style_of`](Self::theme_style_of) and [`default_style_of`](Self::default_style_of).
    /// `WidgetStateProperty`s that resolve to non-null values will similarly override the
    /// corresponding `WidgetStateProperty`s in [`theme_style_of`](Self::theme_style_of) and
    /// [`default_style_of`](Self::default_style_of).
    ///
    /// Null by default.
    pub style: Option<ButtonStyle>,

    /// Represents the interactive "state" of this widget in terms of a set of
    /// `WidgetState`s, including `WidgetState::Pressed`, `WidgetState::Hovered`,
    /// `WidgetState::Focused`.
    pub states_controller: Option<Ref<WidgetStatesController>>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to `Clip::None`.
    pub clip_behavior: Clip,

    /// An optional icon to display before the [`child`](Self::child) label.
    pub leading_icon: Option<WidgetRef>,

    /// An optional icon to display after the [`child`](Self::child) label.
    pub trailing_icon: Option<WidgetRef>,

    /// Determines if the menu will be closed when a [`MenuItemButton`]
    /// is pressed.
    ///
    /// Defaults to true.
    pub close_on_activate: bool,

    /// The direction in which the menu item expands.
    ///
    /// If the menu item button is a descendant of [`MenuAnchor`] or [`MenuBar`], then
    /// this property is ignored.
    ///
    /// If [`overflow_axis`](Self::overflow_axis) is `Axis::Vertical`, the menu will be
    /// expanded vertically. If [`overflow_axis`](Self::overflow_axis) is `Axis::Horizontal`,
    /// then the menu will be expanded horizontally.
    ///
    /// Defaults to `Axis::Horizontal`.
    pub overflow_axis: Axis,

    /// The widget displayed in the center of this button.
    ///
    /// Typically this is the button's label, using a `Text` widget.
    pub child: Option<WidgetRef>,
}

impl Default for MenuItemButton {
    fn default() -> MenuItemButton {
        MenuItemButton {
            key: None,
            on_pressed: None,
            on_hover: None,
            request_focus_on_hover: true,
            on_focus_change: None,
            focus_node: None,
            autofocus: false,
            shortcut: None,
            semantics_label: None,
            style: None,
            states_controller: None,
            clip_behavior: Clip::None,
            leading_icon: None,
            trailing_icon: None,
            close_on_activate: true,
            overflow_axis: Axis::Horizontal,
            child: None,
        }
    }
}

impl MenuItemButton {
    /// Creates a const [`MenuItemButton`].
    pub fn new() -> MenuItemButton {
        MenuItemButton::default()
    }

    /// Dart `MenuItemButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> MenuItemButton {
        self.key = Some(key);
        self
    }

    /// Dart `MenuItemButton(onPressed:)`.
    pub fn on_pressed(mut self, on_pressed: Listener) -> MenuItemButton {
        self.on_pressed = Some(on_pressed);
        self
    }

    /// Dart `MenuItemButton(onHover:)`.
    pub fn on_hover(mut self, on_hover: ValueChanged<bool>) -> MenuItemButton {
        self.on_hover = Some(on_hover);
        self
    }

    /// Dart `MenuItemButton(requestFocusOnHover:)`.
    pub fn request_focus_on_hover(mut self, request_focus_on_hover: bool) -> MenuItemButton {
        self.request_focus_on_hover = request_focus_on_hover;
        self
    }

    /// Dart `MenuItemButton(onFocusChange:)`.
    pub fn on_focus_change(mut self, on_focus_change: ValueChanged<bool>) -> MenuItemButton {
        self.on_focus_change = Some(on_focus_change);
        self
    }

    /// Dart `MenuItemButton(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> MenuItemButton {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `MenuItemButton(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> MenuItemButton {
        self.autofocus = autofocus;
        self
    }

    /// Dart `MenuItemButton(shortcut:)`.
    pub fn shortcut(mut self, shortcut: MenuSerializableShortcutRef) -> MenuItemButton {
        self.shortcut = Some(shortcut);
        self
    }

    /// Dart `MenuItemButton(semanticsLabel:)`.
    pub fn semantics_label(mut self, semantics_label: impl Into<String>) -> MenuItemButton {
        self.semantics_label = Some(semantics_label.into());
        self
    }

    /// Dart `MenuItemButton(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> MenuItemButton {
        self.style = Some(style);
        self
    }

    /// Dart `MenuItemButton(statesController:)`.
    pub fn states_controller(
        mut self,
        states_controller: Ref<WidgetStatesController>,
    ) -> MenuItemButton {
        self.states_controller = Some(states_controller);
        self
    }

    /// Dart `MenuItemButton(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> MenuItemButton {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `MenuItemButton(leadingIcon:)`.
    pub fn leading_icon<K>(mut self, leading_icon: impl IntoWidget<K>) -> MenuItemButton {
        self.leading_icon = Some(leading_icon.into_widget());
        self
    }

    /// Dart `MenuItemButton(trailingIcon:)`.
    pub fn trailing_icon<K>(mut self, trailing_icon: impl IntoWidget<K>) -> MenuItemButton {
        self.trailing_icon = Some(trailing_icon.into_widget());
        self
    }

    /// Dart `MenuItemButton(closeOnActivate:)`.
    pub fn close_on_activate(mut self, close_on_activate: bool) -> MenuItemButton {
        self.close_on_activate = close_on_activate;
        self
    }

    /// Dart `MenuItemButton(overflowAxis:)`.
    pub fn overflow_axis(mut self, overflow_axis: Axis) -> MenuItemButton {
        self.overflow_axis = overflow_axis;
        self
    }

    /// Dart `MenuItemButton(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> MenuItemButton {
        self.child = Some(child.into_widget());
        self
    }

    /// Whether the button is enabled or disabled.
    ///
    /// To enable a button, set its [`on_pressed`](Self::on_pressed) property to a non-null
    /// value.
    pub fn enabled(&self) -> bool {
        self.on_pressed.is_some()
    }

    /// Defines the button's default appearance.
    ///
    /// The button `child`'s `Text` and `Icon` widgets are rendered with the [`ButtonStyle`]'s
    /// foreground color. The button's `InkWell` adds the style's overlay color when the button
    /// is focused, hovered or pressed. The button's background color becomes its `Material`
    /// color and is transparent by default.
    pub fn default_style_of(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Box<dyn ButtonStyleGetters> {
        let _ = app;
        Box::new(MenuButtonDefaultsM3::new(context))
    }

    /// Returns the [`MenuButtonThemeData::style`](crate::MenuButtonThemeData::style) of the
    /// closest [`MenuButtonTheme`] ancestor.
    pub fn theme_style_of(&self, app: &mut App, context: BuildContext) -> Option<ButtonStyle> {
        MenuButtonTheme::of(app, context).style
    }

    /// A static convenience method that constructs a [`MenuItemButton`]'s
    /// [`ButtonStyle`] given simple values.
    ///
    /// The `foreground_color` color is used to create a `WidgetStateProperty`
    /// [`ButtonStyle::foreground_color`] value. Specify a value for `foreground_color`
    /// to specify the color of the button's icons. Use `background_color` for the
    /// button's background fill color. Use `disabled_foreground_color` and
    /// `disabled_background_color` to specify the button's disabled icon and fill
    /// color.
    ///
    /// Similarly, the `enabled_mouse_cursor` and `disabled_mouse_cursor`
    /// parameters are used to construct [`ButtonStyle::mouse_cursor`].
    ///
    /// The `icon_color`, `disabled_icon_color` are used to construct
    /// [`ButtonStyle::icon_color`] and `icon_size` is used to construct
    /// [`ButtonStyle::icon_size`].
    ///
    /// All of the other parameters are either used directly or used to create a
    /// `WidgetStateProperty` with a single value for all states.
    ///
    /// All parameters default to null, by default this method returns a
    /// [`ButtonStyle`] that doesn't override anything.
    ///
    /// For example, to override the default foreground color for a
    /// [`MenuItemButton`], as well as its overlay color, with all of the standard
    /// opacity adjustments for the pressed, focused, and hovered states, one
    /// could write:
    ///
    /// ```text
    /// MenuItemButton::new()
    ///     .leading_icon(Icon::new(Some(Icons::pets())))
    ///     .style(MenuItemButton::style_from().foreground_color(Colors::GREEN.primary()).build())
    ///     .on_pressed(Listener::new(|_app| {
    ///         // ...
    ///     }))
    ///     .child(Text::new("Button Label"))
    /// ```
    pub fn style_from() -> MenuButtonStyleFromBuilder {
        MenuButtonStyleFromBuilder::default()
    }
}

impl Debug for MenuItemButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("MenuItemButton");
        if self.on_pressed.is_none() {
            properties.field("enabled", &"DISABLED");
        }
        properties
            .field("style", &self.style)
            .field("shortcut", &self.shortcut)
            .field("focusNode", &self.focus_node)
            .field("clipBehavior", &self.clip_behavior)
            .field("statesController", &self.states_controller)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for MenuItemButton {
    type State = MenuItemButtonState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> MenuItemButtonState {
        MenuItemButtonState {
            state: StateData::new(),
            internal_focus_node: None,
            is_hovered: false,
        }
    }
}

/// Dart's `MenuItemButton.styleFrom(..)` and `SubmenuButton.styleFrom(..)` arguments, which
/// both hand to `TextButton.styleFrom`; [`build`](Self::build) makes the [`ButtonStyle`].
#[derive(Clone, Default)]
pub struct MenuButtonStyleFromBuilder {
    foreground_color: Option<Color>,
    background_color: Option<Color>,
    disabled_foreground_color: Option<Color>,
    disabled_background_color: Option<Color>,
    shadow_color: Option<Color>,
    surface_tint_color: Option<Color>,
    icon_color: Option<Color>,
    icon_size: Option<f64>,
    disabled_icon_color: Option<Color>,
    text_style: Option<waft_painting::TextStyle>,
    overlay_color: Option<Color>,
    elevation: Option<f64>,
    padding: Option<EdgeInsetsGeometry>,
    minimum_size: Option<Size>,
    fixed_size: Option<Size>,
    maximum_size: Option<Size>,
    enabled_mouse_cursor: Option<MouseCursorRef>,
    disabled_mouse_cursor: Option<MouseCursorRef>,
    side: Option<waft_painting::BorderSide>,
    shape: Option<OutlinedBorderRef>,
    visual_density: Option<VisualDensity>,
    tap_target_size: Option<MaterialTapTargetSize>,
    animation_duration: Option<Duration>,
    enable_feedback: Option<bool>,
    alignment: Option<AlignmentGeometry>,
    splash_factory: Option<InteractiveInkFeatureFactoryRef>,
}

/// The fluent setters of Dart's named `styleFrom` arguments.
macro_rules! menu_button_style_from_setters {
    ($($field:ident: $ty:ty => $dart:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `MenuItemButton.styleFrom(", $dart, ":)`.")]
            pub fn $field(mut self, $field: $ty) -> MenuButtonStyleFromBuilder {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl MenuButtonStyleFromBuilder {
    menu_button_style_from_setters!(
        foreground_color: Color => "foregroundColor",
        background_color: Color => "backgroundColor",
        disabled_foreground_color: Color => "disabledForegroundColor",
        disabled_background_color: Color => "disabledBackgroundColor",
        shadow_color: Color => "shadowColor",
        surface_tint_color: Color => "surfaceTintColor",
        icon_color: Color => "iconColor",
        icon_size: f64 => "iconSize",
        disabled_icon_color: Color => "disabledIconColor",
        text_style: waft_painting::TextStyle => "textStyle",
        overlay_color: Color => "overlayColor",
        elevation: f64 => "elevation",
        padding: EdgeInsetsGeometry => "padding",
        minimum_size: Size => "minimumSize",
        fixed_size: Size => "fixedSize",
        maximum_size: Size => "maximumSize",
        enabled_mouse_cursor: MouseCursorRef => "enabledMouseCursor",
        disabled_mouse_cursor: MouseCursorRef => "disabledMouseCursor",
        side: waft_painting::BorderSide => "side",
        visual_density: VisualDensity => "visualDensity",
        tap_target_size: MaterialTapTargetSize => "tapTargetSize",
        animation_duration: Duration => "animationDuration",
        enable_feedback: bool => "enableFeedback",
        alignment: AlignmentGeometry => "alignment",
        splash_factory: InteractiveInkFeatureFactoryRef => "splashFactory",
    );

    /// Dart `MenuItemButton.styleFrom(shape:)`.
    pub fn shape(mut self, shape: impl Into<OutlinedBorderRef>) -> MenuButtonStyleFromBuilder {
        self.shape = Some(shape.into());
        self
    }

    /// The [`ButtonStyle`] `TextButton.styleFrom` makes from these values.
    pub fn build(self) -> ButtonStyle {
        let mut style_from = TextButton::style_from();
        macro_rules! forward {
            ($($field:ident),* $(,)?) => {
                $(
                    if let Some(value) = self.$field {
                        style_from = style_from.$field(value);
                    }
                )*
            };
        }
        forward!(
            foreground_color,
            background_color,
            disabled_background_color,
            disabled_foreground_color,
            shadow_color,
            surface_tint_color,
            icon_color,
            icon_size,
            disabled_icon_color,
            text_style,
            overlay_color,
            elevation,
            padding,
            minimum_size,
            fixed_size,
            maximum_size,
            enabled_mouse_cursor,
            disabled_mouse_cursor,
            side,
            shape,
            visual_density,
            tap_target_size,
            animation_duration,
            enable_feedback,
            alignment,
            splash_factory,
        );
        style_from.build()
    }
}

/// Dart's `_MenuItemButtonState`.
pub struct MenuItemButtonState {
    state: StateData<MenuItemButton>,
    // If a focus node isn't given to the widget, then we have to manage our own.
    internal_focus_node: Option<Ref<FocusNode>>,
    is_hovered: bool,
}

impl MenuItemButtonState {
    /// Dart's `_focusNode`.
    fn focus_node(self: Handle<Self>, app: &App) -> FocusNodeRef {
        self.widget(app)
            .focus_node
            .clone()
            .or_else(|| app.get(self).internal_focus_node.as_ref().map(Ref::erase))
            .expect("made when the widget has no focus node")
    }

    /// Dart's `_anchor`.
    fn anchor(self: Handle<Self>, app: &App) -> Option<AnyMenuAnchorState> {
        AnyMenuAnchorState::maybe_of(app, self.context(app))
    }

    fn handle_focus_change_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::handle_focus_change)
    }

    /// Dart's `_handleFocusChange`.
    fn handle_focus_change(self: Handle<Self>, app: &mut App) {
        if !self.focus_node(app).has_primary_focus(app) {
            // Close any child menus of this button's menu.
            if let Some(controller) = MenuController::maybe_of(app, self.context(app)) {
                controller.close_children(app);
            }
        }
    }

    /// Dart's `_handlePointerExit`.
    fn handle_pointer_exit(self: Handle<Self>, app: &mut App, _event: PointerExitEvent) {
        if app.get(self).is_hovered {
            if let Some(on_hover) = self.widget(app).on_hover.clone() {
                on_hover(app, false);
            }
            app.get_mut(self).is_hovered = false;
        }
    }

    // TextButton.onHover and MouseRegion.onHover can't be used without triggering
    // focus on scroll.
    fn handle_pointer_hover(self: Handle<Self>, app: &mut App, _event: PointerHoverEvent) {
        if !app.get(self).is_hovered {
            app.get_mut(self).is_hovered = true;
            if let Some(on_hover) = self.widget(app).on_hover.clone() {
                on_hover(app, true);
            }
            if self.widget(app).request_focus_on_hover {
                self.focus_node(app).request_focus(app, None);

                // Without invalidating the focus policy, switching to directional focus
                // may not originate at this node.
                let context = self.context(app);
                let policy = FocusTraversalGroup::of(app, context);
                let scope = FocusScope::of(app, context, true);
                policy.invalidate_scope_data(app, scope);
            }
        }
    }

    /// Dart's `_handleSelect`.
    fn handle_select(self: Handle<Self>, app: &mut App) {
        if self.widget(app).close_on_activate
            && let Some(anchor) = self.anchor(app)
        {
            anchor.root(app).menu_controller(app).close(app);
        }
        // Delay the call to onPressed until post-frame so that the focus is
        // restored to what it was before the menu was opened before the action is
        // executed.
        let last_on_pressed = self.widget(app).on_pressed.clone();
        let view = View::maybe_id_of(app, self.context(app));
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _timestamp| {
                FocusManager::instance(app).apply_focus_changes_if_needed(app);
                // Dart's `widget` outlives the state's `dispose`, as this state's own does not.
                let on_pressed = if app.contains(self) {
                    self.widget(app).on_pressed.clone()
                } else {
                    last_on_pressed.clone()
                };
                if let Some(on_pressed) = on_pressed {
                    on_pressed.call(app);
                }
            }),
            view,
        );
    }

    /// Dart's `_createInternalFocusNodeIfNeeded`.
    fn create_internal_focus_node_if_needed(self: Handle<Self>, app: &mut App) {
        if self.widget(app).focus_node.is_none() {
            let internal_focus_node = FocusNode::new(app);
            app.get_mut(self).internal_focus_node = Some(internal_focus_node.clone());
            if cfg!(debug_assertions) {
                let label = describe_child("MenuItemButton", &self.widget(app).child);
                internal_focus_node.set_debug_label(app, Some(label));
            }
        }
    }
}

impl State for MenuItemButtonState {
    type Widget = MenuItemButton;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.create_internal_focus_node_if_needed(app);
        self.focus_node(app)
            .add_listener(app, self.handle_focus_change_listener());
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.focus_node(app)
            .remove_listener(app, &self.handle_focus_change_listener());
        if let Some(internal_focus_node) = app.get_mut(self).internal_focus_node.take() {
            internal_focus_node.dispose(app);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MenuItemButton) {
        if self.widget(app).focus_node != old_widget.focus_node {
            if let Some(old_focus_node) = old_widget
                .focus_node
                .clone()
                .or_else(|| app.get(self).internal_focus_node.as_ref().map(Ref::erase))
            {
                old_focus_node.remove_listener(app, &self.handle_focus_change_listener());
            }
            if self.widget(app).focus_node.is_some()
                && let Some(internal_focus_node) = app.get_mut(self).internal_focus_node.take()
            {
                internal_focus_node.dispose(app);
            }
            self.create_internal_focus_node_if_needed(app);
            self.focus_node(app)
                .add_listener(app, self.handle_focus_change_listener());
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // Since we don't want to use the theme style or default style from the
        // TextButton, we merge the styles, merging them in the right order when
        // each type of style exists. Each "*StyleOf" function is only called once.
        let widget_ref = context.widget(app).clone();
        let widget = downcast_widget::<MenuItemButton>(&*widget_ref)
            .expect("a MenuItemButtonState builds its MenuItemButton");
        let mut merged_style = match widget.theme_style_of(app, context) {
            Some(theme_style) => {
                theme_style.merge(Some(&widget.default_style_of(app, context).copy_with(app)))
            }
            // Dart passes the defaults object on as the text button's style, whose getters the
            // button reads all of when it builds.
            None => widget.default_style_of(app, context).copy_with(app),
        };
        if let Some(style) = &widget.style {
            merged_style = style.merge(Some(&merged_style));
        }

        let enabled = widget.enabled();
        let overflow_axis = self.anchor(app).map_or(
            self.widget(app).overflow_axis,
            AnyMenuAnchorState::orientation,
        );
        let widget = self.widget(app);
        let label = MenuItemLabel {
            has_submenu: false,
            show_decoration: true,
            leading_icon: widget.leading_icon.clone(),
            trailing_icon: widget.trailing_icon.clone(),
            shortcut: widget.shortcut.clone(),
            semantics_label: widget.semantics_label.clone(),
            overflow_axis,
            submenu_icon: None,
            child: widget.child.clone(),
        };
        let mut button = TextButton::new(
            enabled.then(|| Listener::handle_method(self, Self::handle_select)),
            Some(label),
        )
        .focus_node(self.focus_node(app))
        .style(merged_style)
        .autofocus(enabled && widget.autofocus)
        .clip_behavior(widget.clip_behavior);
        if enabled && let Some(on_focus_change) = widget.on_focus_change.clone() {
            button = button.on_focus_change(on_focus_change);
        }
        if let Some(states_controller) = widget.states_controller.clone() {
            button = button.states_controller(states_controller);
        }
        let mut child = button.into_widget();

        if platform_supports_accelerators(app) && enabled {
            child = MenuAcceleratorCallbackBinding::new(child)
                .on_invoke(Listener::handle_method(self, Self::handle_select))
                .into_widget();
        }

        let widget = self.widget(app);
        if widget.on_hover.is_some() || widget.request_focus_on_hover {
            child = MouseRegion::new()
                .on_hover(Rc::new(move |app: &mut App, event| {
                    self.handle_pointer_hover(app, event)
                }))
                .on_exit(Rc::new(move |app: &mut App, event| {
                    self.handle_pointer_exit(app, event)
                }))
                .child(child)
                .into_widget();
        }

        MergeSemantics::new().child(child).into_widget()
    }
}

impl Debug for MenuItemButtonState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_MenuItemButtonState")
            .field("is_hovered", &self.is_hovered)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------------------------
// CheckboxMenuButton

/// A menu item that combines a [`Checkbox`] widget with a [`MenuItemButton`].
///
/// To style the checkbox separately from the button, add a `CheckboxTheme`
/// ancestor.
///
/// See also:
///
/// - [`MenuBar`], a widget that creates a menu bar of cascading menu items.
/// - [`MenuAnchor`], a widget that defines a region which can host a cascading
///   menu.
pub struct CheckboxMenuButton {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,

    /// Whether this checkbox is checked.
    ///
    /// When [`tristate`](Self::tristate) is true, a value of null corresponds to the mixed
    /// state. When [`tristate`](Self::tristate) is false, this value must not be null.
    pub value: Option<bool>,

    /// If true, then the checkbox's [`value`](Self::value) can be true, false, or null.
    ///
    /// [`CheckboxMenuButton`] displays a dash when its value is null.
    ///
    /// When a tri-state checkbox ([`tristate`](Self::tristate) is true) is tapped, its
    /// [`on_changed`](Self::on_changed) callback will be applied to true if the current value
    /// is false, to null if value is true, and to false if value is null (i.e. it cycles
    /// through false => true => null => false when tapped).
    ///
    /// If tristate is false (the default), [`value`](Self::value) must not be null.
    pub tristate: bool,

    /// True if this checkbox wants to show an error state.
    ///
    /// The checkbox will have different default container color and check color when
    /// this is true. This is only used when `ThemeData::use_material3` is set to true.
    ///
    /// Defaults to false.
    pub is_error: bool,

    /// Called when the value of the checkbox should change.
    ///
    /// The checkbox passes the new value to the callback but does not actually
    /// change state until the parent widget rebuilds the checkbox with the new
    /// value.
    ///
    /// If this callback is null, the menu item will be displayed as disabled
    /// and will not respond to input gestures.
    ///
    /// When the checkbox is tapped, if [`tristate`](Self::tristate) is false (the default)
    /// then the [`on_changed`](Self::on_changed) callback will be applied to `!value`. If
    /// [`tristate`](Self::tristate) is true this callback cycle from false to true to null and
    /// then back to false again.
    pub on_changed: Option<ValueChanged<Option<bool>>>,

    /// Called when a pointer enters or exits the button response area.
    ///
    /// The value passed to the callback is true if a pointer has entered button
    /// area and false if a pointer has exited.
    pub on_hover: Option<ValueChanged<bool>>,

    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's node gains focus, and false if it loses
    /// focus.
    pub on_focus_change: Option<ValueChanged<bool>>,

    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,

    /// The optional shortcut that selects this [`MenuItemButton`].
    pub shortcut: Option<MenuSerializableShortcutRef>,

    /// Customizes this button's appearance.
    ///
    /// Non-null properties of this style override the corresponding properties in
    /// [`MenuItemButton::theme_style_of`] and [`MenuItemButton::default_style_of`].
    /// `WidgetStateProperty`s that resolve to non-null values will similarly
    /// override the corresponding `WidgetStateProperty`s in
    /// [`MenuItemButton::theme_style_of`] and [`MenuItemButton::default_style_of`].
    ///
    /// Null by default.
    pub style: Option<ButtonStyle>,

    /// Represents the interactive "state" of this widget in terms of a set of
    /// `WidgetState`s.
    pub states_controller: Option<Ref<WidgetStatesController>>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to `Clip::None`.
    pub clip_behavior: Clip,

    /// An optional icon to display after the [`child`](Self::child) label.
    pub trailing_icon: Option<WidgetRef>,

    /// Determines if the menu will be closed when a [`MenuItemButton`]
    /// is pressed.
    ///
    /// Defaults to true.
    pub close_on_activate: bool,

    /// The widget displayed in the center of this button.
    ///
    /// Typically this is the button's label, using a `Text` widget.
    pub child: Option<WidgetRef>,
}

impl CheckboxMenuButton {
    /// Creates a const [`CheckboxMenuButton`].
    ///
    /// The [`child`](Self::child), [`value`](Self::value), and
    /// [`on_changed`](Self::on_changed) attributes are required.
    pub fn new<K>(
        value: Option<bool>,
        on_changed: Option<ValueChanged<Option<bool>>>,
        child: Option<impl IntoWidget<K>>,
    ) -> CheckboxMenuButton {
        CheckboxMenuButton {
            key: None,
            value,
            tristate: false,
            is_error: false,
            on_changed,
            on_hover: None,
            on_focus_change: None,
            focus_node: None,
            shortcut: None,
            style: None,
            states_controller: None,
            clip_behavior: Clip::None,
            trailing_icon: None,
            close_on_activate: true,
            child: child.map(IntoWidget::into_widget),
        }
    }

    /// Dart `CheckboxMenuButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> CheckboxMenuButton {
        self.key = Some(key);
        self
    }

    /// Dart `CheckboxMenuButton(tristate:)`.
    pub fn tristate(mut self, tristate: bool) -> CheckboxMenuButton {
        self.tristate = tristate;
        self
    }

    /// Dart `CheckboxMenuButton(isError:)`.
    pub fn is_error(mut self, is_error: bool) -> CheckboxMenuButton {
        self.is_error = is_error;
        self
    }

    /// Dart `CheckboxMenuButton(onHover:)`.
    pub fn on_hover(mut self, on_hover: ValueChanged<bool>) -> CheckboxMenuButton {
        self.on_hover = Some(on_hover);
        self
    }

    /// Dart `CheckboxMenuButton(onFocusChange:)`.
    pub fn on_focus_change(mut self, on_focus_change: ValueChanged<bool>) -> CheckboxMenuButton {
        self.on_focus_change = Some(on_focus_change);
        self
    }

    /// Dart `CheckboxMenuButton(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> CheckboxMenuButton {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `CheckboxMenuButton(shortcut:)`.
    pub fn shortcut(mut self, shortcut: MenuSerializableShortcutRef) -> CheckboxMenuButton {
        self.shortcut = Some(shortcut);
        self
    }

    /// Dart `CheckboxMenuButton(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> CheckboxMenuButton {
        self.style = Some(style);
        self
    }

    /// Dart `CheckboxMenuButton(statesController:)`.
    pub fn states_controller(
        mut self,
        states_controller: Ref<WidgetStatesController>,
    ) -> CheckboxMenuButton {
        self.states_controller = Some(states_controller);
        self
    }

    /// Dart `CheckboxMenuButton(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> CheckboxMenuButton {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `CheckboxMenuButton(trailingIcon:)`.
    pub fn trailing_icon<K>(mut self, trailing_icon: impl IntoWidget<K>) -> CheckboxMenuButton {
        self.trailing_icon = Some(trailing_icon.into_widget());
        self
    }

    /// Dart `CheckboxMenuButton(closeOnActivate:)`.
    pub fn close_on_activate(mut self, close_on_activate: bool) -> CheckboxMenuButton {
        self.close_on_activate = close_on_activate;
        self
    }

    /// Whether the button is enabled or disabled.
    ///
    /// To enable a button, set its [`on_changed`](Self::on_changed) property to a non-null
    /// value.
    pub fn enabled(&self) -> bool {
        self.on_changed.is_some()
    }
}

impl Debug for CheckboxMenuButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheckboxMenuButton")
            .field("value", &self.value)
            .field("tristate", &self.tristate)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for CheckboxMenuButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let on_pressed = self.on_changed.clone().map(|on_changed| {
            let (value, tristate) = (self.value, self.tristate);
            Listener::new(move |app| match value {
                Some(false) => on_changed(app, Some(true)),
                Some(true) => on_changed(app, if tristate { None } else { Some(false) }),
                None => on_changed(app, Some(false)),
            })
        });
        let checkbox = Checkbox::new(self.value, self.on_changed.clone())
            .tristate(self.tristate)
            .is_error(self.is_error);
        let leading_icon = ExcludeFocus::new(
            IgnorePointer::new().child(
                ConstrainedBox::new(
                    BoxConstraints::new()
                        .max_height(Checkbox::WIDTH)
                        .max_width(Checkbox::WIDTH),
                )
                .child(checkbox),
            ),
        );
        let mut button = MenuItemButton::new()
            .style_option(self.style.clone())
            .leading_icon(leading_icon)
            .clip_behavior(self.clip_behavior)
            .close_on_activate(self.close_on_activate);
        button.key = self.key.clone();
        button.on_pressed = on_pressed;
        button.on_hover = self.on_hover.clone();
        button.on_focus_change = self.on_focus_change.clone();
        button.focus_node = self.focus_node.clone();
        button.shortcut = self.shortcut.clone();
        button.states_controller = self.states_controller.clone();
        button.trailing_icon = self.trailing_icon.clone();
        button.child = self.child.clone();
        button.into_widget()
    }
}

impl MenuItemButton {
    /// The fluent form of a Dart `style:` argument that may be null.
    fn style_option(mut self, style: Option<ButtonStyle>) -> MenuItemButton {
        self.style = style;
        self
    }
}

// ---------------------------------------------------------------------------------------------
// RadioMenuButton

/// A menu item that combines a [`Radio`] widget with a [`MenuItemButton`].
///
/// To style the radio button separately from the overall button, add a
/// `RadioTheme` ancestor.
///
/// See also:
///
/// - [`MenuBar`], a widget that creates a menu bar of cascading menu items.
/// - [`MenuAnchor`], a widget that defines a region which can host a cascading
///   menu.
pub struct RadioMenuButton<T: Clone + PartialEq + 'static> {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,

    /// The value represented by this radio button.
    ///
    /// This radio button is considered selected if its [`value`](Self::value) matches the
    /// [`group_value`](Self::group_value).
    pub value: T,

    /// The currently selected value for a group of radio buttons.
    ///
    /// This radio button is considered selected if its [`value`](Self::value) matches the
    /// [`group_value`](Self::group_value).
    pub group_value: Option<T>,

    /// Set to true if this radio button is allowed to be returned to an
    /// indeterminate state by selecting it again when selected.
    ///
    /// To indicate returning to an indeterminate state, [`on_changed`](Self::on_changed) will
    /// be called with null.
    ///
    /// If true, [`on_changed`](Self::on_changed) is called with [`value`](Self::value) when
    /// selected while [`group_value`](Self::group_value) != [`value`](Self::value), and with
    /// null when selected again while [`group_value`](Self::group_value) ==
    /// [`value`](Self::value).
    ///
    /// If false, [`on_changed`](Self::on_changed) will be called with [`value`](Self::value)
    /// when it is selected while [`group_value`](Self::group_value) != [`value`](Self::value),
    /// and only by selecting another radio button in the group (i.e. changing the value of
    /// [`group_value`](Self::group_value)) can this radio button be unselected.
    ///
    /// The default is false.
    pub toggleable: bool,

    /// Called when the user selects this radio button.
    ///
    /// The radio button passes [`value`](Self::value) as a parameter to this callback. The
    /// radio button does not actually change state until the parent widget rebuilds the
    /// radio button with the new [`group_value`](Self::group_value).
    ///
    /// If null, the radio button will be displayed as disabled.
    ///
    /// The provided callback will not be invoked if this radio button is already
    /// selected.
    pub on_changed: Option<ValueChanged<Option<T>>>,

    /// Called when a pointer enters or exits the button response area.
    ///
    /// The value passed to the callback is true if a pointer has entered button
    /// area and false if a pointer has exited.
    pub on_hover: Option<ValueChanged<bool>>,

    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's node gains focus, and false if it loses
    /// focus.
    pub on_focus_change: Option<ValueChanged<bool>>,

    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,

    /// The optional shortcut that selects this [`MenuItemButton`].
    pub shortcut: Option<MenuSerializableShortcutRef>,

    /// Customizes this button's appearance.
    ///
    /// Non-null properties of this style override the corresponding properties in
    /// [`MenuItemButton::theme_style_of`] and [`MenuItemButton::default_style_of`].
    /// `WidgetStateProperty`s that resolve to non-null values will similarly
    /// override the corresponding `WidgetStateProperty`s in
    /// [`MenuItemButton::theme_style_of`] and [`MenuItemButton::default_style_of`].
    ///
    /// Null by default.
    pub style: Option<ButtonStyle>,

    /// Represents the interactive "state" of this widget in terms of a set of
    /// `WidgetState`s.
    pub states_controller: Option<Ref<WidgetStatesController>>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to `Clip::None`.
    pub clip_behavior: Clip,

    /// An optional icon to display after the [`child`](Self::child) label.
    pub trailing_icon: Option<WidgetRef>,

    /// Determines if the menu will be closed when a [`MenuItemButton`]
    /// is pressed.
    ///
    /// Defaults to true.
    pub close_on_activate: bool,

    /// The widget displayed in the center of this button.
    ///
    /// Typically this is the button's label, using a `Text` widget.
    pub child: Option<WidgetRef>,
}

impl<T: Clone + PartialEq + Debug + 'static> RadioMenuButton<T> {
    /// Creates a const [`RadioMenuButton`].
    ///
    /// The [`child`](Self::child) attribute is required.
    pub fn new<K>(
        value: T,
        group_value: Option<T>,
        on_changed: Option<ValueChanged<Option<T>>>,
        child: Option<impl IntoWidget<K>>,
    ) -> RadioMenuButton<T> {
        RadioMenuButton {
            key: None,
            value,
            group_value,
            toggleable: false,
            on_changed,
            on_hover: None,
            on_focus_change: None,
            focus_node: None,
            shortcut: None,
            style: None,
            states_controller: None,
            clip_behavior: Clip::None,
            trailing_icon: None,
            close_on_activate: true,
            child: child.map(IntoWidget::into_widget),
        }
    }

    /// Dart `RadioMenuButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> RadioMenuButton<T> {
        self.key = Some(key);
        self
    }

    /// Dart `RadioMenuButton(toggleable:)`.
    pub fn toggleable(mut self, toggleable: bool) -> RadioMenuButton<T> {
        self.toggleable = toggleable;
        self
    }

    /// Dart `RadioMenuButton(onHover:)`.
    pub fn on_hover(mut self, on_hover: ValueChanged<bool>) -> RadioMenuButton<T> {
        self.on_hover = Some(on_hover);
        self
    }

    /// Dart `RadioMenuButton(onFocusChange:)`.
    pub fn on_focus_change(mut self, on_focus_change: ValueChanged<bool>) -> RadioMenuButton<T> {
        self.on_focus_change = Some(on_focus_change);
        self
    }

    /// Dart `RadioMenuButton(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> RadioMenuButton<T> {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `RadioMenuButton(shortcut:)`.
    pub fn shortcut(mut self, shortcut: MenuSerializableShortcutRef) -> RadioMenuButton<T> {
        self.shortcut = Some(shortcut);
        self
    }

    /// Dart `RadioMenuButton(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> RadioMenuButton<T> {
        self.style = Some(style);
        self
    }

    /// Dart `RadioMenuButton(statesController:)`.
    pub fn states_controller(
        mut self,
        states_controller: Ref<WidgetStatesController>,
    ) -> RadioMenuButton<T> {
        self.states_controller = Some(states_controller);
        self
    }

    /// Dart `RadioMenuButton(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> RadioMenuButton<T> {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `RadioMenuButton(trailingIcon:)`.
    pub fn trailing_icon<K>(mut self, trailing_icon: impl IntoWidget<K>) -> RadioMenuButton<T> {
        self.trailing_icon = Some(trailing_icon.into_widget());
        self
    }

    /// Dart `RadioMenuButton(closeOnActivate:)`.
    pub fn close_on_activate(mut self, close_on_activate: bool) -> RadioMenuButton<T> {
        self.close_on_activate = close_on_activate;
        self
    }

    /// Whether the button is enabled or disabled.
    ///
    /// To enable a button, set its [`on_changed`](Self::on_changed) property to a non-null
    /// value.
    pub fn enabled(&self) -> bool {
        self.on_changed.is_some()
    }
}

impl<T: Clone + PartialEq + 'static> Debug for RadioMenuButton<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RadioMenuButton").finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> StatelessWidget for RadioMenuButton<T> {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let on_pressed = self.on_changed.clone().map(|on_changed| {
            let (value, group_value, toggleable) = (
                self.value.clone(),
                self.group_value.clone(),
                self.toggleable,
            );
            Listener::new(move |app| {
                if toggleable && group_value.as_ref() == Some(&value) {
                    return on_changed(app, None);
                }
                on_changed(app, Some(value.clone()));
            })
        });
        let mut radio = Radio::new(self.value.clone()).toggleable(self.toggleable);
        if let Some(group_value) = self.group_value.clone() {
            radio = radio.group_value(group_value);
        }
        if let Some(on_changed) = self.on_changed.clone() {
            radio = radio.on_changed(on_changed);
        }
        let leading_icon = ExcludeFocus::new(
            IgnorePointer::new().child(
                ConstrainedBox::new(
                    BoxConstraints::new()
                        .max_height(Checkbox::WIDTH)
                        .max_width(Checkbox::WIDTH),
                )
                .child(radio),
            ),
        );
        let mut button = MenuItemButton::new()
            .style_option(self.style.clone())
            .leading_icon(leading_icon)
            .clip_behavior(self.clip_behavior)
            .close_on_activate(self.close_on_activate);
        button.key = self.key.clone();
        button.on_pressed = on_pressed;
        button.on_hover = self.on_hover.clone();
        button.on_focus_change = self.on_focus_change.clone();
        button.focus_node = self.focus_node.clone();
        button.shortcut = self.shortcut.clone();
        button.states_controller = self.states_controller.clone();
        button.trailing_icon = self.trailing_icon.clone();
        button.child = self.child.clone();
        button.into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// SubmenuButton

/// A menu button that displays a cascading menu.
///
/// It can be used as part of a [`MenuBar`], or as a standalone widget.
///
/// This widget represents a menu item that has a submenu. Like the leaf
/// [`MenuItemButton`], it shows a label with an optional leading or trailing
/// icon, but additionally shows an arrow icon showing that it has a submenu.
///
/// By default the submenu will appear to the side of the controlling button.
/// The alignment and offset of the submenu can be controlled by setting
/// [`MenuStyle::alignment`] on the [`style`](Self::style) and the
/// [`alignment_offset`](Self::alignment_offset) argument, respectively.
///
/// When activated (by being clicked, through keyboard navigation, or via
/// hovering with a mouse), it will open a submenu containing the
/// [`menu_children`](Self::menu_children).
///
/// If [`menu_children`](Self::menu_children) is empty, then this menu item will appear
/// disabled.
///
/// See also:
///
/// * [`MenuItemButton`], a widget that represents a leaf menu item that does not
///   host a submenu.
/// * [`MenuBar`], a widget that renders menu items in a row in a Material Design
///   style.
/// * [`MenuAnchor`], a widget that creates a region with a submenu and shows it
///   when requested.
/// * `PlatformMenuBar`, a widget that renders similar menu bar items from a
///   `PlatformMenuItem` using platform-native APIs instead of Flutter.
pub struct SubmenuButton {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,

    /// Called when a pointer enters or exits the button response area.
    ///
    /// The value passed to the callback is true if a pointer has entered this
    /// part of the button and false if a pointer has exited.
    pub on_hover: Option<ValueChanged<bool>>,

    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's [`focus_node`](Self::focus_node) gains focus, and
    /// false if it loses focus.
    pub on_focus_change: Option<ValueChanged<bool>>,

    /// A callback that is invoked when the menu is opened.
    pub on_open: Option<VoidCallback>,

    /// A callback that is invoked when the menu is closed.
    pub on_close: Option<VoidCallback>,

    /// An optional [`MenuController`] for this submenu.
    pub controller: Option<MenuControllerRef>,

    /// Customizes this button's appearance.
    ///
    /// Non-null properties of this style override the corresponding properties in
    /// [`theme_style_of`](Self::theme_style_of) and [`default_style_of`](Self::default_style_of).
    /// `WidgetStateProperty`s that resolve to non-null values will similarly override the
    /// corresponding `WidgetStateProperty`s in [`theme_style_of`](Self::theme_style_of) and
    /// [`default_style_of`](Self::default_style_of).
    ///
    /// Null by default.
    pub style: Option<ButtonStyle>,

    /// The [`MenuStyle`] of the menu specified by [`menu_children`](Self::menu_children).
    ///
    /// Defaults to the value of `MenuThemeData::style` of the ambient [`MenuTheme`].
    pub menu_style: Option<MenuStyle>,

    /// The offset of the menu relative to the alignment origin determined by
    /// [`MenuStyle::alignment`] on the [`style`](Self::style) attribute.
    ///
    /// Use this for fine adjustments of the menu placement.
    ///
    /// Defaults to an offset that takes into account the padding of the menu so
    /// that the top starting corner of the first menu item is aligned with the
    /// top of the [`MenuAnchor`] region.
    pub alignment_offset: Option<Offset>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to `Clip::HardEdge`.
    pub clip_behavior: Clip,

    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,

    /// Represents the interactive "state" of this widget in terms of a set of
    /// `WidgetState`s.
    pub states_controller: Option<Ref<WidgetStatesController>>,

    /// An optional icon to display before the [`child`](Self::child).
    pub leading_icon: Option<WidgetRef>,

    /// If provided, the widget replaces the default [`SubmenuButton`] arrow icon.
    ///
    /// Resolves in the following states:
    ///  * `WidgetState::Disabled`.
    ///  * `WidgetState::Hovered`.
    ///  * `WidgetState::Focused`.
    ///
    /// If this is null, then the value of `MenuThemeData::submenu_icon` is used.
    /// If that is also null, then defaults to a right arrow icon with the size
    /// of 24 pixels.
    pub submenu_icon: Option<WidgetStatePropertyRef<Option<WidgetRef>>>,

    /// An optional icon to display after the [`child`](Self::child).
    pub trailing_icon: Option<WidgetRef>,

    /// Whether the menu panel should be rendered in the root `Overlay`.
    ///
    /// See [`RawMenuAnchor::use_root_overlay`].
    ///
    /// Defaults to false.
    pub use_root_overlay: bool,

    /// An optional callback that is invoked when the [`AnimationStatus`] of the
    /// menu changes during open and close animations.
    ///
    /// If [`animated`](Self::animated) is false, this callback will only be invoked with
    /// [`AnimationStatus::Completed`] when the menu is opened, and
    /// [`AnimationStatus::Dismissed`] when the menu is closed.
    ///
    /// Because the [`MenuControllerLeaf::is_open`] property is true while the menu is
    /// opening, opened, and closing, it cannot be used to determine whether the
    /// menu is in the process of closing or opening. As such, this callback
    /// provides a way to determine when the menu is opening or closing.
    ///
    /// Defaults to null.
    pub on_animation_status_changed: Option<ValueChanged<AnimationStatus>>,

    /// The list of widgets that appear in the menu when it is opened.
    ///
    /// These can be any widget, but are typically either [`MenuItemButton`] or
    /// [`SubmenuButton`] widgets.
    ///
    /// If [`menu_children`](Self::menu_children) is empty, then the button for this menu item
    /// will be disabled.
    pub menu_children: Vec<WidgetRef>,

    /// The duration to wait before opening the menu when the button is hovered.
    ///
    /// Because [`MenuBar`] children can only be opened by hover if a sibling
    /// [`SubmenuButton`] is already open, providing a
    /// [`hover_open_delay`](Self::hover_open_delay) to direct children of a [`MenuBar`] will
    /// panic in a debug build. This is to avoid the case where the
    /// [`hover_open_delay`](Self::hover_open_delay) for a [`SubmenuButton`] is longer than
    /// the duration of a sibling's closing animation, which leads to that menu never opening.
    ///
    /// Defaults to `Duration::ZERO`.
    pub hover_open_delay: Duration,

    /// Whether the menu should open and close with an animation.
    ///
    /// Defaults to false.
    pub animated: bool,

    /// The widget displayed in the middle portion of this button.
    ///
    /// Typically this is the button's label, using a `Text` widget.
    pub child: Option<WidgetRef>,
}

impl SubmenuButton {
    /// Creates a const [`SubmenuButton`].
    ///
    /// The [`child`](Self::child) and [`menu_children`](Self::menu_children) attributes are
    /// required.
    pub fn new<K>(
        menu_children: Vec<WidgetRef>,
        child: Option<impl IntoWidget<K>>,
    ) -> SubmenuButton {
        SubmenuButton {
            key: None,
            on_hover: None,
            on_focus_change: None,
            on_open: None,
            on_close: None,
            controller: None,
            style: None,
            menu_style: None,
            alignment_offset: None,
            clip_behavior: Clip::HardEdge,
            focus_node: None,
            states_controller: None,
            leading_icon: None,
            trailing_icon: None,
            submenu_icon: None,
            use_root_overlay: false,
            hover_open_delay: Duration::ZERO,
            animated: false,
            on_animation_status_changed: None,
            menu_children,
            child: child.map(IntoWidget::into_widget),
        }
    }

    /// Dart `SubmenuButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> SubmenuButton {
        self.key = Some(key);
        self
    }

    /// Dart `SubmenuButton(onHover:)`.
    pub fn on_hover(mut self, on_hover: ValueChanged<bool>) -> SubmenuButton {
        self.on_hover = Some(on_hover);
        self
    }

    /// Dart `SubmenuButton(onFocusChange:)`.
    pub fn on_focus_change(mut self, on_focus_change: ValueChanged<bool>) -> SubmenuButton {
        self.on_focus_change = Some(on_focus_change);
        self
    }

    /// Dart `SubmenuButton(onOpen:)`.
    pub fn on_open(mut self, on_open: VoidCallback) -> SubmenuButton {
        self.on_open = Some(on_open);
        self
    }

    /// Dart `SubmenuButton(onClose:)`.
    pub fn on_close(mut self, on_close: VoidCallback) -> SubmenuButton {
        self.on_close = Some(on_close);
        self
    }

    /// Dart `SubmenuButton(controller:)`.
    pub fn controller(mut self, controller: MenuControllerRef) -> SubmenuButton {
        self.controller = Some(controller);
        self
    }

    /// Dart `SubmenuButton(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> SubmenuButton {
        self.style = Some(style);
        self
    }

    /// Dart `SubmenuButton(menuStyle:)`.
    pub fn menu_style(mut self, menu_style: MenuStyle) -> SubmenuButton {
        self.menu_style = Some(menu_style);
        self
    }

    /// Dart `SubmenuButton(alignmentOffset:)`.
    pub fn alignment_offset(mut self, alignment_offset: Offset) -> SubmenuButton {
        self.alignment_offset = Some(alignment_offset);
        self
    }

    /// Dart `SubmenuButton(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> SubmenuButton {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `SubmenuButton(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> SubmenuButton {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `SubmenuButton(statesController:)`.
    pub fn states_controller(
        mut self,
        states_controller: Ref<WidgetStatesController>,
    ) -> SubmenuButton {
        self.states_controller = Some(states_controller);
        self
    }

    /// Dart `SubmenuButton(leadingIcon:)`.
    pub fn leading_icon<K>(mut self, leading_icon: impl IntoWidget<K>) -> SubmenuButton {
        self.leading_icon = Some(leading_icon.into_widget());
        self
    }

    /// Dart `SubmenuButton(trailingIcon:)`.
    pub fn trailing_icon<K>(mut self, trailing_icon: impl IntoWidget<K>) -> SubmenuButton {
        self.trailing_icon = Some(trailing_icon.into_widget());
        self
    }

    /// Dart `SubmenuButton(submenuIcon:)`.
    pub fn submenu_icon(
        mut self,
        submenu_icon: WidgetStatePropertyRef<Option<WidgetRef>>,
    ) -> SubmenuButton {
        self.submenu_icon = Some(submenu_icon);
        self
    }

    /// Dart `SubmenuButton(useRootOverlay:)`.
    pub fn use_root_overlay(mut self, use_root_overlay: bool) -> SubmenuButton {
        self.use_root_overlay = use_root_overlay;
        self
    }

    /// Dart `SubmenuButton(hoverOpenDelay:)`.
    pub fn hover_open_delay(mut self, hover_open_delay: Duration) -> SubmenuButton {
        self.hover_open_delay = hover_open_delay;
        self
    }

    /// Dart `SubmenuButton(animated:)`.
    pub fn animated(mut self, animated: bool) -> SubmenuButton {
        self.animated = animated;
        self
    }

    /// Dart `SubmenuButton(onAnimationStatusChanged:)`.
    pub fn on_animation_status_changed(
        mut self,
        on_animation_status_changed: ValueChanged<AnimationStatus>,
    ) -> SubmenuButton {
        self.on_animation_status_changed = Some(on_animation_status_changed);
        self
    }

    /// Defines the button's default appearance.
    ///
    /// The button `child`'s `Text` and `Icon` widgets are rendered with the [`ButtonStyle`]'s
    /// foreground color. The button's `InkWell` adds the style's overlay color when the button
    /// is focused, hovered or pressed. The button's background color becomes its `Material`
    /// color and is transparent by default.
    pub fn default_style_of(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Box<dyn ButtonStyleGetters> {
        let _ = app;
        Box::new(MenuButtonDefaultsM3::new(context))
    }

    /// Returns the [`MenuButtonThemeData::style`](crate::MenuButtonThemeData::style) of the
    /// closest [`MenuButtonTheme`] ancestor.
    pub fn theme_style_of(&self, app: &mut App, context: BuildContext) -> Option<ButtonStyle> {
        MenuButtonTheme::of(app, context).style
    }

    /// A static convenience method that constructs a [`SubmenuButton`]'s
    /// [`ButtonStyle`] given simple values.
    ///
    /// The `foreground_color` color is used to create a `WidgetStateProperty`
    /// [`ButtonStyle::foreground_color`] value. Specify a value for `foreground_color`
    /// to specify the color of the button's icons. Use `background_color` for the
    /// button's background fill color. Use `disabled_foreground_color` and
    /// `disabled_background_color` to specify the button's disabled icon and fill
    /// color.
    ///
    /// Similarly, the `enabled_mouse_cursor` and `disabled_mouse_cursor`
    /// parameters are used to construct [`ButtonStyle::mouse_cursor`].
    ///
    /// The `icon_color`, `disabled_icon_color` are used to construct
    /// [`ButtonStyle::icon_color`] and `icon_size` is used to construct
    /// [`ButtonStyle::icon_size`].
    ///
    /// All of the other parameters are either used directly or used to create a
    /// `WidgetStateProperty` with a single value for all states.
    ///
    /// All parameters default to null, by default this method returns a
    /// [`ButtonStyle`] that doesn't override anything.
    ///
    /// For example, to override the default foreground color for a
    /// [`SubmenuButton`], as well as its overlay color, with all of the standard
    /// opacity adjustments for the pressed, focused, and hovered states, one
    /// could write:
    ///
    /// ```text
    /// SubmenuButton::new(menu_children, Some(Text::new("Button Label")))
    ///     .leading_icon(Icon::new(Some(Icons::pets())))
    ///     .style(SubmenuButton::style_from().foreground_color(Colors::GREEN.primary()).build())
    /// ```
    pub fn style_from() -> MenuButtonStyleFromBuilder {
        MenuButtonStyleFromBuilder::default()
    }
}

impl Debug for SubmenuButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubmenuButton")
            .field("focusNode", &self.focus_node)
            .field("menuStyle", &self.menu_style)
            .field("alignmentOffset", &self.alignment_offset)
            .field("clipBehavior", &self.clip_behavior)
            .field("menuChildren", &self.menu_children)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SubmenuButton {
    type State = SubmenuButtonState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SubmenuButtonState {
        SubmenuButtonState {
            state: StateData::new(),
            directional_focus_action: None,
            waiting_to_focus_menu: false,
            is_open_on_focus_enabled: true,
            internal_menu_controller: None,
            internal_focus_node: None,
            anchor_key: GlobalKey::new(),
            is_hovered: false,
            animation_status: AnimationStatus::Dismissed,
            hover_open_timer: None,
        }
    }
}

/// Dart's `_SubmenuButtonState`.
pub struct SubmenuButtonState {
    state: StateData<SubmenuButton>,
    // Dart's `late final actions`, made on the first build that reads it.
    directional_focus_action: Option<Ref<SubmenuDirectionalFocusAction>>,
    waiting_to_focus_menu: bool,
    is_open_on_focus_enabled: bool,
    internal_menu_controller: Option<Ref<MenuController>>,
    internal_focus_node: Option<Ref<FocusNode>>,
    anchor_key: GlobalKey,
    is_hovered: bool,
    animation_status: AnimationStatus,
    hover_open_timer: Option<Timer>,
}

impl SubmenuButtonState {
    /// Dart's `_menuController`.
    fn menu_controller(self: Handle<Self>, app: &App) -> MenuControllerRef {
        self.widget(app).controller.clone().unwrap_or_else(|| {
            app.get(self)
                .internal_menu_controller
                .as_ref()
                .expect("made when the widget has no controller")
                .erase()
        })
    }

    /// Dart's `_parent`.
    fn parent(self: Handle<Self>, app: &App) -> Option<AnyMenuAnchorState> {
        AnyMenuAnchorState::maybe_of(app, self.context(app))
    }

    /// Dart's `_anchorState`.
    fn anchor_state(self: Handle<Self>, app: &App) -> Option<AnyMenuAnchorState> {
        app.get(self)
            .anchor_key
            .current_state::<MenuAnchorState>(app)
            .map(MenuAnchorStateLeaf::as_menu_anchor_state)
    }

    /// Dart's `_buttonFocusNode`.
    fn button_focus_node(self: Handle<Self>, app: &App) -> FocusNodeRef {
        self.widget(app)
            .focus_node
            .clone()
            .or_else(|| app.get(self).internal_focus_node.as_ref().map(Ref::erase))
            .expect("made when the widget has no focus node")
    }

    /// Dart's `_enabled`.
    fn enabled(self: Handle<Self>, app: &App) -> bool {
        !self.widget(app).menu_children.is_empty()
    }

    fn handle_focus_change_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::handle_focus_change)
    }

    fn internal_focus_node_debug_label(self: Handle<Self>, app: &mut App) {
        if cfg!(debug_assertions)
            && let Some(internal_focus_node) = app.get(self).internal_focus_node.clone()
        {
            let label = describe_child("SubmenuButton", &self.widget(app).child);
            internal_focus_node.set_debug_label(app, Some(label));
        }
    }

    /// The `builder` Dart's `build` hands its `MenuAnchor`.
    fn build_button(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        controller: AnyMenuController,
        child: Option<&WidgetRef>,
        submenu_icon: WidgetRef,
    ) -> WidgetRef {
        // Since we don't want to use the theme style or default style from the
        // TextButton, we merge the styles, merging them in the right order when
        // each type of style exists. Each "*StyleOf" function is only called
        // once.
        let widget_ref = self.context(app).widget(app).clone();
        let widget = downcast_widget::<SubmenuButton>(&*widget_ref)
            .expect("a SubmenuButtonState builds its SubmenuButton");
        let mut merged_style = match widget.theme_style_of(app, context) {
            Some(theme_style) => {
                theme_style.merge(Some(&widget.default_style_of(app, context).copy_with(app)))
            }
            // Dart passes the defaults object on as the text button's style, whose getters the
            // button reads all of when it builds.
            None => widget.default_style_of(app, context).copy_with(app),
        };
        if let Some(style) = &widget.style {
            merged_style = style.merge(Some(&merged_style));
        }

        let toggle_show_menu = Listener::new(move |app| {
            if !self.mounted(app) {
                return;
            }
            if app.get(self).animation_status.is_forward_or_completed() {
                controller.close(app);
            } else {
                controller.open(app, None);
            }
        });

        let handle_pointer_exit = move |app: &mut App, _event: PointerExitEvent| {
            if app.get(self).is_hovered {
                if let Some(on_hover) = self.widget(app).on_hover.clone() {
                    on_hover(app, false);
                }
                app.get_mut(self).is_hovered = false;
                self.clear_hover_open_timer(app);
            }
        };

        // MouseRegion.onEnter and TextButton.onHover are called
        // if a button is hovered after scrolling. This interferes with
        // focus traversal and scroll position. MouseRegion.onHover avoids
        // this issue.
        let handle_pointer_hover = move |app: &mut App, _event: PointerHoverEvent| {
            if !app.get(self).is_hovered {
                app.get_mut(self).is_hovered = true;
                if let Some(on_hover) = self.widget(app).on_hover.clone() {
                    on_hover(app, true);
                }
                let root = AnyMenuAnchorState::maybe_of(app, context)
                    .expect("a SubmenuButton's button is built below its MenuAnchor")
                    .root(app);
                // Don't open the root menu bar menus on hover unless a sibling menu
                // is already open. This means that the user has to first click to
                // open a menu on the menu bar before hovering allows them to traverse
                // it.
                if self
                    .parent(app)
                    .is_some_and(|parent| parent.orientation() == Axis::Horizontal)
                    && !root.menu_controller(app).is_open(app)
                {
                    return;
                }

                if self.button_focus_node(app).has_primary_focus(app) {
                    self.clear_hover_open_timer(app);
                    self.maybe_open_menu_on_hover_or_focus(app);
                } else {
                    self.button_focus_node(app).request_focus(app, None);
                }
            }
        };

        let enabled = self.enabled(app);
        let widget = self.widget(app);
        let show_decoration = self
            .parent(app)
            .map_or(Axis::Horizontal, AnyMenuAnchorState::orientation)
            == Axis::Vertical;
        let label = MenuItemLabel {
            has_submenu: true,
            show_decoration,
            leading_icon: widget.leading_icon.clone(),
            trailing_icon: widget.trailing_icon.clone(),
            shortcut: None,
            semantics_label: None,
            overflow_axis: Axis::Vertical,
            submenu_icon: Some(submenu_icon),
            child: child.cloned(),
        };
        let mut button = TextButton::new(enabled.then(|| toggle_show_menu.clone()), Some(label))
            .style(merged_style)
            .focus_node(self.button_focus_node(app));
        if enabled && let Some(on_focus_change) = self.widget(app).on_focus_change.clone() {
            button = button.on_focus_change(on_focus_change);
        }
        let expanded = enabled && app.get(self).animation_status.is_forward_or_completed();
        let mut child = MergeSemantics::new()
            .child(Semantics::new().expanded(expanded).child(button))
            .into_widget();

        if !enabled {
            return child;
        }

        child = MouseRegion::new()
            .on_hover(Rc::new(handle_pointer_hover))
            .on_exit(Rc::new(handle_pointer_exit))
            .child(child)
            .into_widget();

        if platform_supports_accelerators(app) {
            return MenuAcceleratorCallbackBinding::new(child)
                .on_invoke(toggle_show_menu)
                .has_submenu(true)
                .into_widget();
        }

        child
    }

    /// Dart's `_handleAnimationStatusChanged`.
    fn handle_animation_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        self.set_state(app, |state| {
            state.animation_status = status;
        });
        if let Some(on_animation_status_changed) =
            self.widget(app).on_animation_status_changed.clone()
        {
            on_animation_status_changed(app, status);
        }
    }

    /// Dart's `_handleClose`.
    fn handle_close(self: Handle<Self>, app: &mut App) {
        // After closing the children of this submenu, this submenu button will
        // regain focus. Because submenu buttons open on focus, this submenu will
        // immediately reopen. To prevent this from happening, we prevent focus on
        // SubmenuButtons that do not already have focus using the _openOnFocus
        // flag. This flag is reset after one frame.
        if !self.button_focus_node(app).has_focus(app) {
            app.get_mut(self).is_open_on_focus_enabled = false;
            let view = View::maybe_id_of(app, self.context(app));
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    FocusManager::instance(app).apply_focus_changes_if_needed(app);
                    if app.contains(self) {
                        app.get_mut(self).is_open_on_focus_enabled = true;
                    }
                }),
                view,
            );
        }
        if let Some(on_close) = self.widget(app).on_close.clone() {
            on_close(app);
        }
    }

    /// Dart's `_handleOpen`.
    fn handle_open(self: Handle<Self>, app: &mut App) {
        if !app.get(self).waiting_to_focus_menu {
            let view = View::maybe_id_of(app, self.context(app));
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    if self.mounted(app) {
                        self.button_focus_node(app).request_focus(app, None);
                        app.get_mut(self).waiting_to_focus_menu = false;
                    }
                }),
                view,
            );
            app.get_mut(self).waiting_to_focus_menu = true;
        }
        self.set_state(app, |_| {
            // Rebuild with updated controller.isOpen value
        });
        if let Some(on_open) = self.widget(app).on_open.clone() {
            on_open(app);
        }
    }

    /// Dart's `_computeMenuPadding`.
    fn compute_menu_padding(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> EdgeInsets {
        let insets = match self
            .widget(app)
            .menu_style
            .as_ref()
            .and_then(|menu_style| menu_style.padding.clone())
        {
            Some(insets) => insets,
            None => match MenuTheme::of(app, context)
                .style
                .and_then(|style| style.padding)
            {
                Some(insets) => insets,
                None => MenuDefaultsM3::new(context)
                    .padding(app)
                    .expect("the menu defaults have a padding"),
            },
        };
        let states = self
            .widget(app)
            .states_controller
            .as_ref()
            .map(|states_controller| app.get(states_controller).value().clone())
            .unwrap_or_default();
        let direction = Directionality::of(app, context);
        insets
            .resolve(&states)
            .expect("the menu padding resolves")
            .resolve(Some(direction))
    }

    /// Dart's `_handleFocusChange`.
    fn handle_focus_change(self: Handle<Self>, app: &mut App) {
        self.clear_hover_open_timer(app);
        if !self.button_focus_node(app).has_primary_focus(app) {
            let anchor_state = self
                .anchor_state(app)
                .expect("the SubmenuButton's MenuAnchor is mounted");
            if !anchor_state.menu_scope_node(app).as_node().has_focus(app)
                && app.get(self).animation_status.is_forward_or_completed()
            {
                self.menu_controller(app).close(app);
            }
            return;
        }

        self.maybe_open_menu_on_hover_or_focus(app);
    }

    /// Dart's `_maybeOpenMenuOnHoverOrFocus`.
    fn maybe_open_menu_on_hover_or_focus(self: Handle<Self>, app: &mut App) {
        if !app.get(self).is_open_on_focus_enabled {
            return;
        }

        if self.menu_controller(app).is_open(app) {
            if app.get(self).animation_status != AnimationStatus::Reverse {
                // If the menu isn't closing, there's no reason to reopen it.
                return;
            }

            if app.get(self).is_hovered {
                // If the button is hovered, avoid reopening the menu that the user deliberately
                // closed.
                return;
            }

            if self
                .parent(app)
                .is_some_and(|parent| parent.orientation() == Axis::Horizontal)
            {
                // Top-level (horizontal) buttons in a menubar will try to reopen when
                // closed, since focus returns to their button.
                return;
            }
        }

        let hover_open_delay = self.widget(app).hover_open_delay;
        if hover_open_delay == Duration::ZERO {
            self.menu_controller(app).open(app, None);
            return;
        }

        let timer = Timer::new(
            app,
            hover_open_delay,
            Listener::new(move |app| {
                // The menu controller could change, so we can't use a tearoff.
                self.menu_controller(app).open(app, None);
            }),
        );
        app.get_mut(self).hover_open_timer = Some(timer);
    }

    /// Dart's `_clearHoverOpenTimer`.
    fn clear_hover_open_timer(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).hover_open_timer.take() {
            timer.cancel(app);
        }
    }

    /// Dart's `_debugValidateHoverOpenDelay`.
    fn debug_validate_hover_open_delay(self: Handle<Self>, app: &App) -> bool {
        if self
            .parent(app)
            .is_some_and(|parent| parent.orientation() == Axis::Horizontal)
            && self.widget(app).hover_open_delay > Duration::ZERO
        {
            panic!(
                "A non-zero hoverOpenDelay was used in a top-level SubmenuButton situated in a \
                 MenuBar.\nMenuBar children can only be opened by hover if a sibling \
                 SubmenuButton is already open. When the hoverOpenDelay for a SubmenuButton is \
                 longer than the closing animation of a sibling SubmenuButton, that sibling will \
                 close before this SubmenuButton begins opening, leading to this SubmenuButton \
                 never opening.\nThe affected SubmenuButton is: {:?}",
                self.widget(app)
            );
        }
        true
    }
}

impl State for SubmenuButtonState {
    type Widget = SubmenuButton;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        if self.widget(app).focus_node.is_none() {
            let internal_focus_node = FocusNode::new(app);
            app.get_mut(self).internal_focus_node = Some(internal_focus_node);
            self.internal_focus_node_debug_label(app);
        }
        if self.widget(app).controller.is_none() {
            let controller = MenuController::new(app);
            app.get_mut(self).internal_menu_controller = Some(controller);
        }
        self.button_focus_node(app)
            .add_listener(app, self.handle_focus_change_listener());
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.debug_validate_hover_open_delay(app));
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.clear_hover_open_timer(app);
        self.button_focus_node(app)
            .remove_listener(app, &self.handle_focus_change_listener());
        if let Some(internal_focus_node) = app.get_mut(self).internal_focus_node.take() {
            internal_focus_node.dispose(app);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SubmenuButton) {
        debug_assert!(self.debug_validate_hover_open_delay(app));
        if self.widget(app).focus_node != old_widget.focus_node {
            match old_widget.focus_node.clone() {
                None => {
                    if let Some(internal_focus_node) = app.get_mut(self).internal_focus_node.take()
                    {
                        internal_focus_node
                            .erase::<AnyFocusNode>()
                            .remove_listener(app, &self.handle_focus_change_listener());
                        internal_focus_node.dispose(app);
                    }
                }
                Some(old_focus_node) => {
                    old_focus_node.remove_listener(app, &self.handle_focus_change_listener());
                }
            }
            if self.widget(app).focus_node.is_none() {
                if app.get(self).internal_focus_node.is_none() {
                    let internal_focus_node = FocusNode::new(app);
                    app.get_mut(self).internal_focus_node = Some(internal_focus_node);
                }
                self.internal_focus_node_debug_label(app);
            }
            self.button_focus_node(app)
                .add_listener(app, self.handle_focus_change_listener());
        }
        if self.widget(app).controller != old_widget.controller {
            let internal_menu_controller = if old_widget.controller.is_none() {
                None
            } else {
                Some(MenuController::new(app))
            };
            app.get_mut(self).internal_menu_controller = internal_menu_controller;
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut menu_padding_offset = self.widget(app).alignment_offset.unwrap_or(Offset::ZERO);
        let menu_padding = self.compute_menu_padding(app, context);
        let orientation = self
            .parent(app)
            .map_or(Axis::Vertical, AnyMenuAnchorState::orientation);

        // Move the submenu over by the size of the menu padding, so that
        // the first menu item aligns with the submenu button that opens it.
        menu_padding_offset = menu_padding_offset
            + match (orientation, Directionality::of(app, context)) {
                (Axis::Horizontal, TextDirection::Rtl) => Offset::new(menu_padding.right, 0.0),
                (Axis::Horizontal, TextDirection::Ltr) => Offset::new(-menu_padding.left, 0.0),
                (Axis::Vertical, TextDirection::Rtl) => Offset::new(0.0, -menu_padding.top),
                (Axis::Vertical, TextDirection::Ltr) => Offset::new(0.0, -menu_padding.top),
            };
        let mut states = WidgetStates::new();
        if !self.enabled(app) {
            states.insert(WidgetState::Disabled);
        }
        if app.get(self).is_hovered {
            states.insert(WidgetState::Hovered);
        }
        if self.button_focus_node(app).has_focus(app) {
            states.insert(WidgetState::Focused);
        }
        let submenu_icon = self
            .widget(app)
            .submenu_icon
            .as_ref()
            .and_then(|submenu_icon| submenu_icon.resolve(&states));
        let submenu_icon = match submenu_icon {
            Some(submenu_icon) => submenu_icon,
            None => match MenuTheme::of(app, context)
                .submenu_icon
                .and_then(|submenu_icon| submenu_icon.resolve(&states))
            {
                Some(submenu_icon) => submenu_icon,
                None => waft_widgets::Icon::new(Some(
                    Icons::arrow_right(), // Automatically switches with text direction.
                ))
                .size(K_DEFAULT_SUBMENU_ICON_SIZE)
                .into_widget(),
            },
        };

        let directional_focus_action = match app.get(self).directional_focus_action.clone() {
            Some(action) => action,
            None => {
                let action = app.create(SubmenuDirectionalFocusAction {
                    action: ActionData::new(),
                    submenu: self,
                });
                app.get_mut(self).directional_focus_action = Some(action.clone());
                action
            }
        };
        let mut actions: HashMap<TypeId, ActionRef> = HashMap::new();
        actions.insert(
            <SubmenuDirectionalFocusAction as Action>::intent_type(),
            directional_focus_action.into_erased(),
        );

        let widget = self.widget(app);
        let mut menu_anchor = MenuAnchor::new(widget.menu_children.clone())
            .key(app.get(self).anchor_key.clone().into())
            .on_animation_status_changed(Rc::new(move |app: &mut App, status| {
                self.handle_animation_status_changed(app, status);
            }))
            .controller(self.menu_controller(app))
            .child_focus_node(self.button_focus_node(app))
            .alignment_offset(menu_padding_offset)
            .clip_behavior(widget.clip_behavior)
            .on_close(Rc::new(move |app: &mut App| self.handle_close(app)))
            .on_open(Rc::new(move |app: &mut App| self.handle_open(app)))
            .use_root_overlay(widget.use_root_overlay)
            .animated(widget.animated)
            .builder(move |app, context, controller, child| {
                self.build_button(app, context, controller, child, submenu_icon.clone())
            });
        menu_anchor.style = widget.menu_style.clone();
        menu_anchor.child = widget.child.clone();

        Actions::new(actions, menu_anchor).into_widget()
    }
}

impl Debug for SubmenuButtonState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_SubmenuButtonState")
            .field("animation_status", &self.animation_status)
            .finish_non_exhaustive()
    }
}

/// Dart's `_SubmenuDirectionalFocusAction`, which extends `DirectionalFocusAction` and
/// overrides its `invoke`.
struct SubmenuDirectionalFocusAction {
    action: ActionData,
    submenu: Handle<SubmenuButtonState>,
}

impl SubmenuDirectionalFocusAction {
    fn parent(self: Handle<Self>, app: &App) -> Option<AnyMenuAnchorState> {
        app.get(self).submenu.parent(app)
    }

    fn anchor_state(self: Handle<Self>, app: &App) -> Option<AnyMenuAnchorState> {
        app.get(self).submenu.anchor_state(app)
    }

    fn controller(self: Handle<Self>, app: &App) -> AnyMenuController {
        app.get(self).submenu.menu_controller(app).handle()
    }

    /// The orientation of the menu that contains this submenu button.
    fn orientation(self: Handle<Self>, app: &App) -> Option<Axis> {
        self.parent(app).map(AnyMenuAnchorState::orientation)
    }

    /// Whether the anchor that intercepted this DirectionalFocusAction is a submenu.
    fn is_submenu(self: Handle<Self>, app: &App) -> bool {
        self.button(app).has_primary_focus(app)
    }

    fn button(self: Handle<Self>, app: &App) -> AnyFocusNode {
        app.get(self).submenu.button_focus_node(app).handle()
    }
}

impl Action for SubmenuDirectionalFocusAction {
    type Intent = DirectionalFocusIntent;
    waft_widgets::action_accessors!();

    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        intent: &DirectionalFocusIntent,
    ) -> Option<Rc<dyn Any>> {
        let submenu = app.get(self).submenu;
        let directionality = Directionality::of(app, submenu.context(app));
        let is_submenu = self.is_submenu(app);
        match (self.orientation(app), directionality, intent.direction) {
            (Some(Axis::Horizontal), TextDirection::Ltr, TraversalDirection::Left)
            | (Some(Axis::Horizontal), TextDirection::Rtl, TraversalDirection::Right) => {
                // Focus this MenuBar SubmenuButton, then move focus to the previous focusable
                // MenuBar item.
                let button = self.button(app);
                button.request_focus(app, None);
                button.previous_focus(app);
                return None;
            }
            (Some(Axis::Horizontal), TextDirection::Ltr, TraversalDirection::Right)
            | (Some(Axis::Horizontal), TextDirection::Rtl, TraversalDirection::Left) => {
                // Focus this MenuBar SubmenuButton, then move focus to the next focusable
                // MenuBar item.
                let button = self.button(app);
                button.request_focus(app, None);
                button.next_focus(app);
                return None;
            }
            (Some(Axis::Horizontal), _, TraversalDirection::Down) => {
                if is_submenu {
                    // If this is a top-level (horizontal) button in a menubar, focus the
                    // first item in this button's submenu.
                    if let Some(anchor_state) = self.anchor_state(app) {
                        anchor_state.focus_first_menu_item(app);
                    }
                    return None;
                }
            }
            (Some(Axis::Horizontal), _, TraversalDirection::Up) => {
                if is_submenu {
                    // If this is a top-level (horizontal) button in a menubar, focus the
                    // last item in this button's submenu. This makes navigating into
                    // upward-oriented submenus more intuitive.
                    if let Some(anchor_state) = self.anchor_state(app) {
                        anchor_state.focus_last_menu_item(app);
                    }
                    return None;
                }
            }
            (Some(Axis::Vertical), TextDirection::Ltr, TraversalDirection::Left)
            | (Some(Axis::Vertical), TextDirection::Rtl, TraversalDirection::Right) => {
                let parent = self.parent(app);
                let grandparent = parent.and_then(|parent| parent.parent(app));
                if grandparent.map(AnyMenuAnchorState::orientation) == Some(Axis::Horizontal) {
                    if is_submenu {
                        let parent = parent.expect("a grandparent has a parent");
                        if let Some(child_focus_node) = parent.widget(app).child_focus_node.clone()
                        {
                            child_focus_node.request_focus(app, None);
                            child_focus_node.previous_focus(app);
                        }
                    } else {
                        // MenuBar SubmenuButton => SubmenuButton => child
                        // Focus the parent SubmenuButton anchor attached to this child.
                        if let Some(anchor_state) = self.anchor_state(app) {
                            anchor_state.focus_button(app);
                        }
                    }
                } else if is_submenu {
                    // Moving in the closing direction while focused on a
                    // SubmenuButton within a root MenuAnchor menu should not close
                    // the menu.
                    grandparent?;
                    if let Some(parent) = parent {
                        parent.focus_button(app);
                        parent.menu_controller(app).close(app);
                    }
                } else {
                    // If focus is not on a submenu button, closing the anchor this item
                    // presides in will close the menu and focus the anchor button.
                    self.controller(app).close(app);
                }
                return None;
            }
            (Some(Axis::Vertical), TextDirection::Ltr, TraversalDirection::Right)
            | (Some(Axis::Vertical), TextDirection::Rtl, TraversalDirection::Left)
                if is_submenu =>
            {
                let controller = self.controller(app);
                if controller.is_open(app) {
                    if let Some(anchor_state) = self.anchor_state(app) {
                        anchor_state.focus_first_menu_item(app);
                    }
                } else {
                    controller.open(app, None);
                    let view = View::maybe_id_of(app, submenu.context(app));
                    SchedulerBinding::add_post_frame_callback(
                        app,
                        FrameCallback::new(move |app, _timestamp| {
                            if controller.is_open(app)
                                && let Some(anchor_state) = self.anchor_state(app)
                            {
                                anchor_state.focus_first_menu_item(app);
                            }
                        }),
                        view,
                    );
                }
                return None;
            }
            _ => {}
        }

        Actions::maybe_invoke(app, submenu.context(app), intent);
        None
    }
}

// ---------------------------------------------------------------------------------------------
// _LocalizedShortcutLabeler

/// A helper class used to generate shortcut labels for a
/// `MenuSerializableShortcut` (a subset of the implementors of
/// `ShortcutActivator`).
///
/// This helper class is typically used by the [`MenuItemButton`] and
/// [`SubmenuButton`] classes to display a label for their assigned shortcuts.
///
/// Call [`get_shortcut_label`](Self::get_shortcut_label) with the
/// `MenuSerializableShortcut` to get a label for it.
///
/// For instance, calling [`get_shortcut_label`](Self::get_shortcut_label) with
/// `SingleActivator::new(LogicalKeyboardKey::KEY_A).control(true)` would return "⌃ A" on
/// macOS, "Ctrl A" in an US English locale, and "Strg A" in a German locale.
struct LocalizedShortcutLabeler;

impl LocalizedShortcutLabeler {
    /// Dart's `_shortcutGraphicEquivalents`.
    fn shortcut_graphic_equivalent(key: LogicalKeyboardKey) -> Option<&'static str> {
        match key {
            LogicalKeyboardKey::ARROW_LEFT => Some("←"),
            LogicalKeyboardKey::ARROW_RIGHT => Some("→"),
            LogicalKeyboardKey::ARROW_UP => Some("↑"),
            LogicalKeyboardKey::ARROW_DOWN => Some("↓"),
            LogicalKeyboardKey::ENTER => Some("↵"),
            _ => None,
        }
    }

    /// Dart's `_modifiers`.
    const MODIFIERS: [LogicalKeyboardKey; 12] = [
        LogicalKeyboardKey::ALT,
        LogicalKeyboardKey::CONTROL,
        LogicalKeyboardKey::META,
        LogicalKeyboardKey::SHIFT,
        LogicalKeyboardKey::ALT_LEFT,
        LogicalKeyboardKey::CONTROL_LEFT,
        LogicalKeyboardKey::META_LEFT,
        LogicalKeyboardKey::SHIFT_LEFT,
        LogicalKeyboardKey::ALT_RIGHT,
        LogicalKeyboardKey::CONTROL_RIGHT,
        LogicalKeyboardKey::META_RIGHT,
        LogicalKeyboardKey::SHIFT_RIGHT,
    ];

    /// Return the instance for this singleton.
    fn instance() -> LocalizedShortcutLabeler {
        LocalizedShortcutLabeler
    }

    /// Returns the label to be shown to the user in the UI when a
    /// `MenuSerializableShortcut` is used as a keyboard shortcut.
    ///
    /// When the target platform is `TargetPlatform::MacOS` or `TargetPlatform::IOS`, this
    /// will return graphical key representations when it can. For instance, the default
    /// `LogicalKeyboardKey::SHIFT` will return '⇧', and the arrow keys will return arrows.
    /// The key `LogicalKeyboardKey::META` will show as '⌘', `LogicalKeyboardKey::CONTROL`
    /// will show as '˄', and `LogicalKeyboardKey::ALT` will show as '⌥'.
    ///
    /// The keys are joined by spaces on macOS and iOS, and by "+" on other
    /// platforms.
    fn get_shortcut_label(
        &self,
        app: &App,
        shortcut: &MenuSerializableShortcutRef,
        localizations: &dyn MaterialLocalizations,
    ) -> String {
        let serialized = shortcut.serialize_for_menu();
        let uses_symbolic_modifiers = uses_symbolic_modifiers(app);
        let key_separator = if uses_symbolic_modifiers {
            // Use "⌃ ⇧ A" style on macOS and iOS.
            " "
        } else {
            // Use "Ctrl+Shift+A" style.
            "+"
        };
        let modifier = |key| self.get_modifier_label(app, key, localizations);
        if let Some(trigger) = serialized.trigger() {
            let mut modifiers = Vec::new();
            let (control, alt, shift, meta) = (
                serialized.control().expect("a trigger has modifiers"),
                serialized.alt().expect("a trigger has modifiers"),
                serialized.shift().expect("a trigger has modifiers"),
                serialized.meta().expect("a trigger has modifiers"),
            );
            if uses_symbolic_modifiers {
                // macOS/iOS platform convention uses this ordering, with ⌘ always last.
                if control {
                    modifiers.push(modifier(LogicalKeyboardKey::CONTROL));
                }
                if alt {
                    modifiers.push(modifier(LogicalKeyboardKey::ALT));
                }
                if shift {
                    modifiers.push(modifier(LogicalKeyboardKey::SHIFT));
                }
                if meta {
                    modifiers.push(modifier(LogicalKeyboardKey::META));
                }
            } else {
                // This order matches the LogicalKeySet version.
                if alt {
                    modifiers.push(modifier(LogicalKeyboardKey::ALT));
                }
                if control {
                    modifiers.push(modifier(LogicalKeyboardKey::CONTROL));
                }
                if meta {
                    modifiers.push(modifier(LogicalKeyboardKey::META));
                }
                if shift {
                    modifiers.push(modifier(LogicalKeyboardKey::SHIFT));
                }
            }
            let shortcut_trigger = match Self::shortcut_graphic_equivalent(trigger) {
                Some(graphic) => graphic.to_string(),
                // Otherwise, look it up, and if we don't have a translation for it,
                // then fall back to the key label.
                None => self
                    .get_localized_name(trigger, localizations)
                    .unwrap_or_else(|| trigger.key_label()),
            };
            if !shortcut_trigger.is_empty() {
                modifiers.push(shortcut_trigger);
            }
            return modifiers.join(key_separator);
        } else if let Some(character) = serialized.character_value() {
            let mut modifiers = Vec::new();
            let (control, alt, meta) = (
                serialized.control().expect("a character has modifiers"),
                serialized.alt().expect("a character has modifiers"),
                serialized.meta().expect("a character has modifiers"),
            );
            // Character based shortcuts cannot check shifted keys.
            if uses_symbolic_modifiers {
                // macOS/iOS platform convention uses this ordering, with ⌘ always last.
                if control {
                    modifiers.push(modifier(LogicalKeyboardKey::CONTROL));
                }
                if alt {
                    modifiers.push(modifier(LogicalKeyboardKey::ALT));
                }
                if meta {
                    modifiers.push(modifier(LogicalKeyboardKey::META));
                }
            } else {
                // This order matches the LogicalKeySet version.
                if alt {
                    modifiers.push(modifier(LogicalKeyboardKey::ALT));
                }
                if control {
                    modifiers.push(modifier(LogicalKeyboardKey::CONTROL));
                }
                if meta {
                    modifiers.push(modifier(LogicalKeyboardKey::META));
                }
            }
            modifiers.push(character.to_string());
            return modifiers.join(key_separator);
        }
        panic!(
            "Shortcut labels for ShortcutActivators that do not implement \
             MenuSerializableShortcut (e.g. ShortcutActivators other than SingleActivator or \
             CharacterActivator) are not supported."
        );
    }

    // Tries to look up the key in an internal table, and if it can't find it,
    // then fall back to the key's keyLabel.
    fn get_localized_name(
        &self,
        key: LogicalKeyboardKey,
        localizations: &dyn MaterialLocalizations,
    ) -> Option<String> {
        let name = match key {
            LogicalKeyboardKey::ALT_GRAPH => localizations.keyboard_key_alt_graph(),
            LogicalKeyboardKey::BACKSPACE => localizations.keyboard_key_backspace(),
            LogicalKeyboardKey::CAPS_LOCK => localizations.keyboard_key_caps_lock(),
            LogicalKeyboardKey::CHANNEL_DOWN => localizations.keyboard_key_channel_down(),
            LogicalKeyboardKey::CHANNEL_UP => localizations.keyboard_key_channel_up(),
            LogicalKeyboardKey::DELETE => localizations.keyboard_key_delete(),
            LogicalKeyboardKey::EJECT => localizations.keyboard_key_eject(),
            LogicalKeyboardKey::END => localizations.keyboard_key_end(),
            LogicalKeyboardKey::ESCAPE => localizations.keyboard_key_escape(),
            LogicalKeyboardKey::FN => localizations.keyboard_key_fn(),
            LogicalKeyboardKey::HOME => localizations.keyboard_key_home(),
            LogicalKeyboardKey::INSERT => localizations.keyboard_key_insert(),
            LogicalKeyboardKey::NUM_LOCK => localizations.keyboard_key_num_lock(),
            LogicalKeyboardKey::NUMPAD1 => localizations.keyboard_key_numpad1(),
            LogicalKeyboardKey::NUMPAD2 => localizations.keyboard_key_numpad2(),
            LogicalKeyboardKey::NUMPAD3 => localizations.keyboard_key_numpad3(),
            LogicalKeyboardKey::NUMPAD4 => localizations.keyboard_key_numpad4(),
            LogicalKeyboardKey::NUMPAD5 => localizations.keyboard_key_numpad5(),
            LogicalKeyboardKey::NUMPAD6 => localizations.keyboard_key_numpad6(),
            LogicalKeyboardKey::NUMPAD7 => localizations.keyboard_key_numpad7(),
            LogicalKeyboardKey::NUMPAD8 => localizations.keyboard_key_numpad8(),
            LogicalKeyboardKey::NUMPAD9 => localizations.keyboard_key_numpad9(),
            LogicalKeyboardKey::NUMPAD0 => localizations.keyboard_key_numpad0(),
            LogicalKeyboardKey::NUMPAD_ADD => localizations.keyboard_key_numpad_add(),
            LogicalKeyboardKey::NUMPAD_COMMA => localizations.keyboard_key_numpad_comma(),
            LogicalKeyboardKey::NUMPAD_DECIMAL => localizations.keyboard_key_numpad_decimal(),
            LogicalKeyboardKey::NUMPAD_DIVIDE => localizations.keyboard_key_numpad_divide(),
            LogicalKeyboardKey::NUMPAD_ENTER => localizations.keyboard_key_numpad_enter(),
            LogicalKeyboardKey::NUMPAD_EQUAL => localizations.keyboard_key_numpad_equal(),
            LogicalKeyboardKey::NUMPAD_MULTIPLY => localizations.keyboard_key_numpad_multiply(),
            LogicalKeyboardKey::NUMPAD_PAREN_LEFT => localizations.keyboard_key_numpad_paren_left(),
            LogicalKeyboardKey::NUMPAD_PAREN_RIGHT => {
                localizations.keyboard_key_numpad_paren_right()
            }
            LogicalKeyboardKey::NUMPAD_SUBTRACT => localizations.keyboard_key_numpad_subtract(),
            LogicalKeyboardKey::PAGE_DOWN => localizations.keyboard_key_page_down(),
            LogicalKeyboardKey::PAGE_UP => localizations.keyboard_key_page_up(),
            LogicalKeyboardKey::POWER => localizations.keyboard_key_power(),
            LogicalKeyboardKey::POWER_OFF => localizations.keyboard_key_power_off(),
            LogicalKeyboardKey::PRINT_SCREEN => localizations.keyboard_key_print_screen(),
            LogicalKeyboardKey::SCROLL_LOCK => localizations.keyboard_key_scroll_lock(),
            LogicalKeyboardKey::SELECT => localizations.keyboard_key_select(),
            LogicalKeyboardKey::SPACE => localizations.keyboard_key_space(),
            _ => return None,
        };
        Some(name.to_string())
    }

    fn get_modifier_label(
        &self,
        app: &App,
        modifier: LogicalKeyboardKey,
        localizations: &dyn MaterialLocalizations,
    ) -> String {
        debug_assert!(
            Self::MODIFIERS.contains(&modifier),
            "{} is not a modifier key",
            modifier.key_label()
        );
        let platform = app.platform().target_platform();
        if modifier == LogicalKeyboardKey::META
            || modifier == LogicalKeyboardKey::META_LEFT
            || modifier == LogicalKeyboardKey::META_RIGHT
        {
            return match platform {
                TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::Linux => {
                    localizations.keyboard_key_meta().to_string()
                }
                TargetPlatform::Windows => localizations.keyboard_key_meta_windows().to_string(),
                TargetPlatform::IOS | TargetPlatform::MacOS => "⌘".to_string(),
            };
        }
        if modifier == LogicalKeyboardKey::ALT
            || modifier == LogicalKeyboardKey::ALT_LEFT
            || modifier == LogicalKeyboardKey::ALT_RIGHT
        {
            return match platform {
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::Linux
                | TargetPlatform::Windows => localizations.keyboard_key_alt().to_string(),
                TargetPlatform::IOS | TargetPlatform::MacOS => "⌥".to_string(),
            };
        }
        if modifier == LogicalKeyboardKey::CONTROL
            || modifier == LogicalKeyboardKey::CONTROL_LEFT
            || modifier == LogicalKeyboardKey::CONTROL_RIGHT
        {
            // '⎈' (a boat helm wheel, not an asterisk) is apparently the standard
            // icon for "control", but only seems to appear on the French Canadian
            // keyboard. A '✲' (an open center asterisk) appears on some Microsoft
            // keyboards. For all but macOS (which has standardized on "⌃", it seems),
            // we just return the local translation of "Ctrl".
            return match platform {
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::Linux
                | TargetPlatform::Windows => localizations.keyboard_key_control().to_string(),
                TargetPlatform::IOS | TargetPlatform::MacOS => "⌃".to_string(),
            };
        }
        if modifier == LogicalKeyboardKey::SHIFT
            || modifier == LogicalKeyboardKey::SHIFT_LEFT
            || modifier == LogicalKeyboardKey::SHIFT_RIGHT
        {
            return match platform {
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::Linux
                | TargetPlatform::Windows => localizations.keyboard_key_shift().to_string(),
                TargetPlatform::IOS | TargetPlatform::MacOS => "⇧".to_string(),
            };
        }
        panic!("Keyboard key {} is not a modifier.", modifier.key_label());
    }
}

// ---------------------------------------------------------------------------------------------
// _MenuBarAnchor

/// MenuBar-specific private specialization of [`MenuAnchor`] so that it can act
/// differently in regards to orientation, how open works, and what gets built.
///
/// Dart's `_MenuBarAnchor extends MenuAnchor`: the [`MenuAnchor`] it is.
struct MenuBarAnchor {
    menu_anchor: MenuAnchor,
}

impl Debug for MenuBarAnchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.menu_anchor, f)
    }
}

impl StatefulWidget for MenuBarAnchor {
    type State = MenuBarAnchorState;

    fn key(&self) -> Option<&KeyRef> {
        self.menu_anchor.key.as_ref()
    }

    fn create_state(&self) -> MenuBarAnchorState {
        MenuBarAnchorState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            menu_anchor_state: MenuAnchorStateData::new(),
            dismiss_menu_action: None,
        }
    }
}

/// Dart's `_MenuBarAnchorState extends _MenuAnchorState`.
struct MenuBarAnchorState {
    state: StateData<MenuBarAnchor>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    menu_anchor_state: MenuAnchorStateData,
    // Dart's `late final actions`, made on the first build with the controller of the moment.
    dismiss_menu_action: Option<Ref<DismissMenuAction>>,
}

impl MenuAnchorStateLeaf for MenuBarAnchorState {
    fn menu_anchor_state_data(self: Handle<Self>, app: &App) -> &MenuAnchorStateData {
        &app.get(self).menu_anchor_state
    }

    fn menu_anchor_state_data_mut(self: Handle<Self>, app: &mut App) -> &mut MenuAnchorStateData {
        &mut app.get_mut(self).menu_anchor_state
    }

    fn menu_anchor(self: Handle<Self>, app: &App) -> &MenuAnchor {
        &self.widget(app).menu_anchor
    }

    fn orientation() -> Axis {
        Axis::Horizontal
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let this = self.as_menu_anchor_state();
        let dismiss_menu_action = match app.get(self).dismiss_menu_action.clone() {
            Some(action) => action,
            None => {
                let controller = this.menu_controller(app);
                let action = DismissMenuAction::new(app, controller);
                app.get_mut(self).dismiss_menu_action = Some(action.clone());
                action
            }
        };
        let mut actions: HashMap<TypeId, ActionRef> = HashMap::new();
        actions.insert(
            <DismissMenuAction as Action>::intent_type(),
            dismiss_menu_action.into_erased(),
        );
        let widget = self.menu_anchor(app);
        let child = Actions::new(
            actions,
            Shortcuts::new(
                menu_traversal_shortcuts(),
                MenuPanel {
                    menu_style: widget.style.clone(),
                    clip_behavior: widget.clip_behavior,
                    orientation: Self::orientation(),
                    cross_axis_unconstrained: true,
                    height_animation: None,
                    children: widget.menu_children.clone(),
                },
            ),
        )
        .into_widget();
        let menu_scope_node = self
            .menu_anchor_state_data(app)
            .menu_scope_node
            .clone()
            .expect("created in init_state");
        MenuAnchorScope {
            state: this,
            animation_status: self.animation_controller(app).status(app),
            child: RawMenuAnchorGroup::new(
                Builder::new(move |app, context| {
                    let is_open = MenuController::maybe_is_open_of(app, context).unwrap_or(false);
                    FocusScope::new(ExcludeFocus::new(child.clone()).excluding(!is_open))
                        .node(menu_scope_node.clone())
                        .skip_traversal(!is_open)
                        .can_request_focus(is_open)
                        .descendants_are_focusable(true)
                        .into_widget()
                }),
                this.menu_controller(app),
            )
            .into_widget(),
        }
        .into_widget()
    }
}

impl SingleTickerProviderStateMixin for MenuBarAnchorState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for MenuBarAnchorState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for MenuBarAnchorState {
    type Widget = MenuBarAnchor;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        MenuAnchorStateLeaf::init_state(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MenuBarAnchor) {
        MenuAnchorStateLeaf::did_update_widget(self, app, &old_widget.menu_anchor);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        MenuAnchorStateLeaf::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        MenuAnchorStateLeaf::build(self, app, context)
    }
}

impl Debug for MenuBarAnchorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_MenuBarAnchorState")
    }
}

// ---------------------------------------------------------------------------------------------
// MenuAcceleratorCallbackBinding

/// An `InheritedWidget` that provides a descendant [`MenuAcceleratorLabel`] with
/// the function to invoke when the accelerator is pressed.
///
/// This is used when creating your own custom menu item for use with
/// [`MenuAnchor`] or [`MenuBar`]. Provided menu items such as [`MenuItemButton`] and
/// [`SubmenuButton`] already supply this wrapper internally.
#[derive(Clone, Debug)]
pub struct MenuAcceleratorCallbackBinding {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,

    /// The function that pressing the accelerator defined in a descendant
    /// [`MenuAcceleratorLabel`] will invoke.
    ///
    /// If set to null, then the accelerator won't be enabled.
    pub on_invoke: Option<Listener>,

    /// Whether or not the associated label will host its own submenu or not.
    ///
    /// This setting determines when accelerators are active, since accelerators
    /// for menu items that open submenus shouldn't be active when the submenu is
    /// open.
    pub has_submenu: bool,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl MenuAcceleratorCallbackBinding {
    /// Create a const [`MenuAcceleratorCallbackBinding`].
    ///
    /// The [`child`](Self::child) parameter is required.
    pub fn new<K>(child: impl IntoWidget<K>) -> MenuAcceleratorCallbackBinding {
        MenuAcceleratorCallbackBinding {
            key: None,
            on_invoke: None,
            has_submenu: false,
            child: child.into_widget(),
        }
    }

    /// Dart `MenuAcceleratorCallbackBinding(key:)`.
    pub fn key(mut self, key: KeyRef) -> MenuAcceleratorCallbackBinding {
        self.key = Some(key);
        self
    }

    /// Dart `MenuAcceleratorCallbackBinding(onInvoke:)`.
    pub fn on_invoke(mut self, on_invoke: Listener) -> MenuAcceleratorCallbackBinding {
        self.on_invoke = Some(on_invoke);
        self
    }

    /// Dart `MenuAcceleratorCallbackBinding(hasSubmenu:)`.
    pub fn has_submenu(mut self, has_submenu: bool) -> MenuAcceleratorCallbackBinding {
        self.has_submenu = has_submenu;
        self
    }

    /// Returns the active [`MenuAcceleratorCallbackBinding`] in the given context, if any,
    /// and creates a dependency relationship that will rebuild the context when
    /// [`on_invoke`](Self::on_invoke) changes.
    ///
    /// If no [`MenuAcceleratorCallbackBinding`] is found, returns null.
    ///
    /// See also:
    ///
    /// * [`of`](Self::of), which is similar, but panics if no
    ///   [`MenuAcceleratorCallbackBinding`] is found.
    pub fn maybe_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<MenuAcceleratorCallbackBinding> {
        context
            .depend_on_inherited_widget_of_exact_type::<MenuAcceleratorCallbackBinding>(app)
            .cloned()
    }

    /// Returns the active [`MenuAcceleratorCallbackBinding`] in the given context, and
    /// creates a dependency relationship that will rebuild the context when
    /// [`on_invoke`](Self::on_invoke) changes.
    ///
    /// If no [`MenuAcceleratorCallbackBinding`] is found, this panics.
    ///
    /// See also:
    ///
    /// * [`maybe_of`](Self::maybe_of), which is similar, but returns null if no
    ///   [`MenuAcceleratorCallbackBinding`] is found.
    pub fn of(app: &mut App, context: BuildContext) -> MenuAcceleratorCallbackBinding {
        let result = MenuAcceleratorCallbackBinding::maybe_of(app, context);
        match result {
            Some(result) => result,
            None => panic!(
                "MenuAcceleratorWrapper.of() was called with a context that does not contain a \
                 MenuAcceleratorWrapper in the given context.\nNo MenuAcceleratorWrapper \
                 ancestor could be found in the context that was passed to \
                 MenuAcceleratorWrapper.of(). This can happen because you are using a widget \
                 that looks for a MenuAcceleratorWrapper ancestor, and do not have a \
                 MenuAcceleratorWrapper widget ancestor.\nThe context used was:\n  {context:?}"
            ),
        }
    }
}

impl InheritedWidget for MenuAcceleratorCallbackBinding {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &MenuAcceleratorCallbackBinding) -> bool {
        self.on_invoke != old_widget.on_invoke || self.has_submenu != old_widget.has_submenu
    }
}

// ---------------------------------------------------------------------------------------------
// MenuAcceleratorLabel

/// The type of builder function used for building a [`MenuAcceleratorLabel`]'s
/// [`MenuAcceleratorLabel::builder`] function.
///
/// The arguments to the function are as follows:
///
/// * The `context` supplies the [`BuildContext`] to use.
/// * The `label` is the [`MenuAcceleratorLabel::label`] attribute for the relevant
///   [`MenuAcceleratorLabel`] with the accelerator markers stripped out of it.
/// * The `index` is the index of the accelerator character within the
///   grapheme clusters of `label` that applies to this accelerator. If it is -1, then the
///   accelerator should not be highlighted. Otherwise, the given character
///   should be highlighted somehow in the rendered label (typically with an
///   underscore). Importantly, `index` is not an index into the [`String`]
///   `label`, it is an index into its user-visible characters (a.k.a. grapheme clusters),
///   not Unicode code points.
///
/// See also:
///
/// * [`MenuAcceleratorLabel::default_label_builder`], which is the implementation
///   used as the default value for [`MenuAcceleratorLabel::builder`].
pub type MenuAcceleratorChildBuilder = Rc<dyn Fn(&mut App, BuildContext, &str, isize) -> WidgetRef>;

/// A widget that draws the label text for a menu item (typically a
/// [`MenuItemButton`] or [`SubmenuButton`]) and renders its child with information
/// about the currently active keyboard accelerator.
///
/// On platforms other than macOS and iOS, this widget listens for the Alt key
/// to be pressed, and when it is down, will update the label by calling the
/// builder again with the position of the accelerator in the label string.
/// While the Alt key is pressed, it registers a shortcut with the
/// `ShortcutRegistry` mapped to a `VoidCallbackIntent` containing the callback
/// defined by the nearest [`MenuAcceleratorCallbackBinding`].
///
/// Because the accelerators are registered with the `ShortcutRegistry`, any
/// other shortcuts in the widget tree between the primary focus and the
/// `ShortcutRegistry` that define Alt-based shortcuts using the same keys will
/// take precedence over the accelerators.
///
/// Because accelerators aren't used on macOS and iOS, the label ignores the Alt
/// key on those platforms, and the [`builder`](Self::builder) is always given -1 as an
/// accelerator index. Accelerator labels are still stripped of their
/// accelerator markers.
///
/// The built-in menu items [`MenuItemButton`] and [`SubmenuButton`] already provide
/// the appropriate [`MenuAcceleratorCallbackBinding`], so unless you are creating
/// your own custom menu item type that takes a [`MenuAcceleratorLabel`], it is
/// not necessary to provide one.
pub struct MenuAcceleratorLabel {
    /// See `Widget::key`.
    pub key: Option<KeyRef>,

    /// The label string that should be displayed.
    ///
    /// The label string provides the label text, as well as the possible
    /// characters which could be used as accelerators in the menu system.
    ///
    /// To indicate which letters in the label are to be used as accelerators, add
    /// an "&" character before the character in the string. If more than one
    /// character has an "&" in front of it, then the characters appearing earlier
    /// in the string are preferred. To represent a literal "&", insert "&&" into
    /// the string. All other ampersands will be removed from the string before
    /// calling [`MenuAcceleratorLabel::builder`]. Bare ampersands at the end of the
    /// string or before whitespace are stripped and ignored.
    ///
    /// See also:
    ///
    /// * [`display_label`](Self::display_label), which returns the [`label`](Self::label)
    ///   with all of the ampersands stripped out of it, and double ampersands converted to
    ///   ampersands.
    /// * [`strip_accelerator_markers`](Self::strip_accelerator_markers), which returns the
    ///   supplied string with all of the ampersands stripped out of it, and double ampersands
    ///   converted to ampersands, and optionally calls a callback with the index of the
    ///   accelerator character found.
    pub label: String,

    /// The optional [`MenuAcceleratorChildBuilder`] which is used to build the
    /// widget that displays the label itself.
    ///
    /// The [`default_label_builder`](Self::default_label_builder) function serves as the
    /// default value for [`builder`](Self::builder), rendering the label as a `RichText`
    /// widget with appropriate `TextSpan`s for rendering the label with an underscore under
    /// the selected accelerator for the label when accelerators have been activated.
    ///
    /// When writing the builder function, it's not necessary to take the current
    /// platform into account. On platforms which don't support accelerators (e.g.
    /// macOS and iOS), the passed accelerator index will always be -1, and the
    /// accelerator markers will already be stripped.
    pub builder: MenuAcceleratorChildBuilder,
}

impl MenuAcceleratorLabel {
    /// Creates a const [`MenuAcceleratorLabel`].
    ///
    /// The [`label`](Self::label) parameter is required.
    pub fn new(label: impl Into<String>) -> MenuAcceleratorLabel {
        MenuAcceleratorLabel {
            key: None,
            label: label.into(),
            builder: Rc::new(MenuAcceleratorLabel::default_label_builder),
        }
    }

    /// Dart `MenuAcceleratorLabel(key:)`.
    pub fn key(mut self, key: KeyRef) -> MenuAcceleratorLabel {
        self.key = Some(key);
        self
    }

    /// Dart `MenuAcceleratorLabel(builder:)`.
    pub fn builder(
        mut self,
        builder: impl Fn(&mut App, BuildContext, &str, isize) -> WidgetRef + 'static,
    ) -> MenuAcceleratorLabel {
        self.builder = Rc::new(builder);
        self
    }

    /// Returns the [`label`](Self::label) with any accelerator markers removed.
    ///
    /// This getter just calls [`strip_accelerator_markers`](Self::strip_accelerator_markers)
    /// with the [`label`](Self::label).
    pub fn display_label(&self) -> String {
        MenuAcceleratorLabel::strip_accelerator_markers(&self.label, None)
    }

    /// Whether [`label`](Self::label) contains an accelerator definition.
    ///
    /// Dart's `RegExp(r'&(?!([&\s]|$))')`: an ampersand followed by a character that is
    /// neither an ampersand nor whitespace.
    pub fn has_accelerator(&self) -> bool {
        let mut chars = self.label.chars().peekable();
        while let Some(character) = chars.next() {
            if character == '&'
                && chars
                    .peek()
                    .is_some_and(|next| *next != '&' && !next.is_whitespace())
            {
                return true;
            }
        }
        false
    }

    /// Serves as the default value for [`builder`](Self::builder), rendering the label as a
    /// `RichText` widget with appropriate `TextSpan`s for rendering the label with an
    /// underscore under the selected accelerator for the label when the `index` is
    /// non-negative, and a `Text` widget when the `index` is negative.
    pub fn default_label_builder(
        app: &mut App,
        context: BuildContext,
        label: &str,
        index: isize,
    ) -> WidgetRef {
        if index < 0 {
            return Text::new(label).into_widget();
        }
        let index = index as usize;
        let default_style = DefaultTextStyle::of(app, context).style;
        let characters: Vec<&str> = label.graphemes(true).collect();
        let mut children: Vec<InlineSpanRef> = Vec::new();
        if index > 0 {
            children.push(Rc::new(
                TextSpan::new()
                    .text(characters[..index].concat())
                    .style(default_style.clone()),
            ));
        }
        children.push(Rc::new(
            TextSpan::new()
                .text(characters[index..index + 1].concat())
                .style(
                    default_style
                        .copy_with()
                        .decoration(TextDecoration::UNDERLINE),
                ),
        ));
        if index < characters.len() - 1 {
            children.push(Rc::new(
                TextSpan::new()
                    .text(characters[index + 1..].concat())
                    .style(default_style),
            ));
        }
        RichText::new(Rc::new(TextSpan::new().children(children))).into_widget()
    }

    /// Strips out any accelerator markers from the given `label`, and unescapes
    /// any escaped ampersands.
    ///
    /// If `set_index` is supplied, it will be called before this function returns
    /// with the index in the returned string of the accelerator character.
    ///
    /// See [`label`](Self::label) for the markers.
    pub fn strip_accelerator_markers(
        label: &str,
        set_index: Option<&mut dyn FnMut(isize)>,
    ) -> String {
        let mut quoted_ampersands: isize = 0;
        let mut display_label = String::new();
        let mut accelerator_index: isize = -1;
        // Use characters so that we don't split up surrogate pairs and interpret
        // them incorrectly.
        let label_chars: Vec<&str> = label.graphemes(true).collect();
        let ampersand = "&";
        let mut last_was_ampersand = false;
        for (i, character) in label_chars.iter().enumerate() {
            // Stop looking one before the end, since a single ampersand at the end is
            // just treated as a quoted ampersand.
            if last_was_ampersand {
                last_was_ampersand = false;
                display_label.push_str(character);
                continue;
            }
            if *character != ampersand {
                display_label.push_str(character);
                continue;
            }
            if i == label_chars.len() - 1 {
                // Strip bare ampersands at the end of a string.
                break;
            }
            last_was_ampersand = true;
            let accelerator_character = label_chars[i + 1];
            if accelerator_index == -1
                && accelerator_character != ampersand
                && !accelerator_character.trim().is_empty()
            {
                // Don't set the accelerator index if the character is an ampersand,
                // or whitespace.
                accelerator_index = i as isize - quoted_ampersands;
            }
            // As we encounter '&<character>' pairs, the following indices must be
            // adjusted so that they correspond with indices in the stripped string.
            quoted_ampersands += 1;
        }
        if let Some(set_index) = set_index {
            set_index(accelerator_index);
        }
        display_label
    }
}

impl Debug for MenuAcceleratorLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MenuAcceleratorLabel(\"{}\")", self.label)
    }
}

impl StatefulWidget for MenuAcceleratorLabel {
    type State = MenuAcceleratorLabelState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> MenuAcceleratorLabelState {
        MenuAcceleratorLabelState {
            state: StateData::new(),
            display_label: String::new(),
            accelerator_index: -1,
            binding: None,
            menu_controller: None,
            shortcut_registry: None,
            shortcut_registry_entry: None,
            show_accelerators: false,
            key_event_handler: None,
        }
    }
}

/// Dart's `_MenuAcceleratorLabelState`.
pub struct MenuAcceleratorLabelState {
    state: StateData<MenuAcceleratorLabel>,
    display_label: String,
    accelerator_index: isize,
    binding: Option<MenuAcceleratorCallbackBinding>,
    menu_controller: Option<MenuControllerRef>,
    shortcut_registry: Option<Ref<ShortcutRegistry>>,
    shortcut_registry_entry: Option<Ref<ShortcutRegistryEntry>>,
    show_accelerators: bool,
    // Dart's `_listenToKeyEvent` tear-off, kept to remove the same handler again.
    key_event_handler: Option<KeyEventCallback>,
}

impl MenuAcceleratorLabelState {
    /// Dart's `_altIsPressed`.
    fn alt_is_pressed(app: &mut App) -> bool {
        let keyboard = HardwareKeyboard::instance(app);
        let pressed = keyboard.logical_keys_pressed(app);
        [
            LogicalKeyboardKey::ALT_LEFT,
            LogicalKeyboardKey::ALT_RIGHT,
            LogicalKeyboardKey::ALT,
        ]
        .iter()
        .any(|key| pressed.contains(key))
    }

    /// Dart's `_listenToKeyEvent`.
    fn listen_to_key_event(self: Handle<Self>, app: &mut App, _event: &KeyEvent) -> bool {
        debug_assert!(platform_supports_accelerators(app));
        let show_accelerators = Self::alt_is_pressed(app);
        self.set_state(app, |state| {
            state.show_accelerators = show_accelerators;
        });
        self.update_accelerator_shortcut(app);
        // Just listening, so it doesn't ever handle a key.
        false
    }

    /// Dart's `_updateAcceleratorShortcut`.
    fn update_accelerator_shortcut(self: Handle<Self>, app: &mut App) {
        debug_assert!(platform_supports_accelerators(app));
        if let Some(entry) = app.get_mut(self).shortcut_registry_entry.take() {
            entry.dispose(app);
        }
        // Before registering an accelerator as a shortcut it should meet these
        // conditions:
        //
        // 1) Is showing accelerators (i.e. Alt key is down).
        // 2) Has an accelerator marker in the label.
        // 3) Has an associated action callback for the label (from the
        //    MenuAcceleratorCallbackBinding).
        // 4) Is part of an anchor that either doesn't have a submenu, or doesn't
        //    have any submenus currently open (only the "deepest" open menu should
        //    have accelerator shortcuts registered).
        let this = app.get(self);
        let Some(on_invoke) = this
            .binding
            .as_ref()
            .and_then(|binding| binding.on_invoke.clone())
        else {
            return;
        };
        let has_submenu = this
            .binding
            .as_ref()
            .is_some_and(|binding| binding.has_submenu);
        if this.show_accelerators
            && this.accelerator_index != -1
            && (!has_submenu
                || !this
                    .menu_controller
                    .as_ref()
                    .is_some_and(|menu_controller| menu_controller.is_open(app)))
        {
            // Dart indexes the label's UTF-16 code units here.
            let code_unit = this
                .display_label
                .encode_utf16()
                .nth(this.accelerator_index as usize)
                .expect("the accelerator index is inside the label");
            let accelerator_character = String::from_utf16_lossy(&[code_unit]).to_lowercase();
            if let Some(shortcut_registry) = this.shortcut_registry.clone() {
                let activator: ShortcutActivatorRef =
                    Rc::new(CharacterActivator::new(accelerator_character).alt(true));
                let entry = shortcut_registry.add_all(
                    app,
                    vec![(activator, Rc::new(VoidCallbackIntent::new(on_invoke)))],
                );
                app.get_mut(self).shortcut_registry_entry = Some(entry);
            }
        }
    }

    /// Dart's `_updateDisplayLabel`.
    fn update_display_label(self: Handle<Self>, app: &mut App) {
        let mut accelerator_index = -1;
        let display_label = MenuAcceleratorLabel::strip_accelerator_markers(
            &self.widget(app).label,
            Some(&mut |index| {
                accelerator_index = index;
            }),
        );
        let this = app.get_mut(self);
        this.display_label = display_label;
        this.accelerator_index = accelerator_index;
    }
}

impl State for MenuAcceleratorLabelState {
    type Widget = MenuAcceleratorLabel;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        if platform_supports_accelerators(app) {
            let show_accelerators = Self::alt_is_pressed(app);
            app.get_mut(self).show_accelerators = show_accelerators;
            let handler: KeyEventCallback = Rc::new(move |app: &mut App, event: &KeyEvent| {
                self.listen_to_key_event(app, event)
            });
            app.get_mut(self).key_event_handler = Some(Rc::clone(&handler));
            HardwareKeyboard::instance(app).add_handler(app, handler);
        }
        self.update_display_label(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            platform_supports_accelerators(app) || app.get(self).shortcut_registry_entry.is_none()
        );
        app.get_mut(self).display_label = String::new();
        if platform_supports_accelerators(app) {
            if let Some(entry) = app.get_mut(self).shortcut_registry_entry.take() {
                entry.dispose(app);
            }
            let this = app.get_mut(self);
            this.shortcut_registry = None;
            this.menu_controller = None;
            if let Some(handler) = this.key_event_handler.take() {
                HardwareKeyboard::instance(app).remove_handler(app, &handler);
            }
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        if !platform_supports_accelerators(app) {
            return;
        }
        let context = self.context(app);
        let binding = MenuAcceleratorCallbackBinding::maybe_of(app, context);
        let menu_controller =
            MenuController::maybe_of(app, context).and_then(|controller| controller.upgrade(app));
        let shortcut_registry =
            ShortcutRegistry::maybe_of(app, context).and_then(|registry| registry.upgrade(app));
        let this = app.get_mut(self);
        this.binding = binding;
        this.menu_controller = menu_controller;
        this.shortcut_registry = shortcut_registry;
        self.update_accelerator_shortcut(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MenuAcceleratorLabel) {
        if self.widget(app).label != old_widget.label {
            self.update_display_label(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let this = app.get(self);
        let index = if this.show_accelerators {
            this.accelerator_index
        } else {
            -1
        };
        let display_label = this.display_label.clone();
        let builder = Rc::clone(&self.widget(app).builder);
        builder(app, context, &display_label, index)
    }
}

impl Debug for MenuAcceleratorLabelState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_MenuAcceleratorLabelState")
            .field("display_label", &self.display_label)
            .field("accelerator_index", &self.accelerator_index)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------------------------
// _MenuItemLabel

/// A label widget that is used as the label for a [`MenuItemButton`] or
/// [`SubmenuButton`].
///
/// It not only shows the [`SubmenuButton::child`] or [`MenuItemButton::child`], but if
/// there is a shortcut associated with the [`MenuItemButton`], it will display a
/// mnemonic for the shortcut. For [`SubmenuButton`]s, it will display a visual
/// indicator that there is a submenu.
struct MenuItemLabel {
    /// Whether or not this menu has a submenu.
    ///
    /// Determines whether the submenu arrow is shown or not.
    has_submenu: bool,

    /// Whether or not this item should show decorations like shortcut labels or
    /// submenu arrows. Items in a [`MenuBar`] don't show these decorations when
    /// they are laid out horizontally.
    show_decoration: bool,

    /// The optional icon that comes before the [`child`](Self::child).
    leading_icon: Option<WidgetRef>,

    /// The optional icon that comes after the [`child`](Self::child).
    trailing_icon: Option<WidgetRef>,

    /// The shortcut for this label, so that it can generate a string describing
    /// the shortcut.
    shortcut: Option<MenuSerializableShortcutRef>,

    /// An optional Semantics label, which replaces the generated string when
    /// read by a screen reader.
    semantics_label: Option<String>,

    /// The direction in which the menu item expands.
    overflow_axis: Axis,

    /// The submenu icon that is displayed when [`show_decoration`](Self::show_decoration) and
    /// [`has_submenu`](Self::has_submenu) are true.
    submenu_icon: Option<WidgetRef>,

    /// An optional child widget that is displayed in the label.
    child: Option<WidgetRef>,
}

impl Debug for MenuItemLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_MenuItemLabel")
            .field("shortcut", &self.shortcut)
            .field("hasSubmenu", &self.has_submenu)
            .field("showDecoration", &self.show_decoration)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for MenuItemLabel {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let density = Theme::of(app, context).visual_density;
        let horizontal_padding =
            K_LABEL_ITEM_MIN_SPACING.max(K_LABEL_ITEM_DEFAULT_SPACING + density.horizontal * 2.0);
        let start_padding = if self.leading_icon.is_some() {
            EdgeInsetsGeometry::directional(horizontal_padding, 0.0, 0.0, 0.0)
        } else {
            EdgeInsetsGeometry::ZERO
        };
        let leadings = if self.overflow_axis == Axis::Vertical {
            let mut children: Vec<WidgetRef> = self.leading_icon.iter().cloned().collect();
            if let Some(child) = &self.child {
                children.push(
                    Expanded::new(
                        ClipRect::new().child(Padding::new(start_padding).child(child.clone())),
                    )
                    .into_widget(),
                );
            }
            Expanded::new(
                Row::new()
                    .main_axis_size(MainAxisSize::Min)
                    .children(children),
            )
            .into_widget()
        } else {
            let mut children: Vec<WidgetRef> = self.leading_icon.iter().cloned().collect();
            if let Some(child) = &self.child {
                children.push(
                    Padding::new(start_padding)
                        .child(child.clone())
                        .into_widget(),
                );
            }
            Row::new()
                .main_axis_size(MainAxisSize::Min)
                .children(children)
                .into_widget()
        };

        let item_padding = EdgeInsetsGeometry::directional(horizontal_padding, 0.0, 0.0, 0.0);
        let mut children = vec![leadings];
        if let Some(trailing_icon) = &self.trailing_icon {
            children.push(
                Padding::new(item_padding)
                    .child(trailing_icon.clone())
                    .into_widget(),
            );
        }
        if self.show_decoration
            && let Some(shortcut) = &self.shortcut
        {
            let localizations = <dyn MaterialLocalizations>::of(app, context);
            let label = LocalizedShortcutLabeler::instance().get_shortcut_label(
                app,
                shortcut,
                &*localizations,
            );
            children.push(
                Padding::new(item_padding)
                    .child(Text::new(label))
                    .into_widget(),
            );
        }
        if self.show_decoration && self.has_submenu {
            let mut padding = Padding::new(item_padding);
            if let Some(submenu_icon) = &self.submenu_icon {
                padding = padding.child(submenu_icon.clone());
            }
            children.push(padding.into_widget());
        }
        let mut menu_item_label = Row::new()
            .main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .children(children)
            .into_widget();
        if let Some(semantics_label) = self.semantics_label.clone() {
            menu_item_label = Semantics::new()
                .label(semantics_label)
                .exclude_semantics(true)
                .child(menu_item_label)
                .into_widget();
        }
        menu_item_label
    }
}

// ---------------------------------------------------------------------------------------------
// _MenuLayout

// Positions the menu in the view while trying to keep as much as possible
// visible in the view.
#[derive(Debug)]
struct MenuLayout {
    // Rectangle of underlying button, relative to the overlay's dimensions.
    anchor_rect: Rect,

    // Whether to prefer going to the left or to the right.
    text_direction: TextDirection,

    // The alignment to use when finding the ideal location for the menu.
    alignment: AlignmentGeometry,

    // The offset from the alignment position to find the ideal location for the
    // menu.
    alignment_offset: Offset,

    // The position passed to the open method, if any.
    menu_position: Option<Offset>,

    // The padding on the inside of the menu, so it can be accounted for when
    // positioning.
    menu_padding: EdgeInsetsGeometry,

    // The orientation of this menu.
    orientation: Axis,

    // The orientation of this menu's parent.
    parent_orientation: Axis,

    // How close to the edge of the safe area the menu will be placed.
    reserved_padding: EdgeInsetsGeometry,

    // The factor by which the height of the menu is scaled.
    height_factor: f64,

    // Used to ensure the menu is positioned within the safe area and respects
    // view insets such as the software keyboard.
    media_query_data: MediaQueryData,
}

impl MenuLayout {
    fn position_child(&self, child_size: Size, overlay_rect: Rect) -> Offset {
        let anchor_rect = self.anchor_rect;
        let mut x;
        let mut y;
        match self.menu_position {
            None => {
                let mut desired_position = self
                    .alignment
                    .resolve(Some(self.text_direction))
                    .within_rect(anchor_rect);
                let directional_offset =
                    if matches!(self.alignment, AlignmentGeometry::Directional(_)) {
                        match self.text_direction {
                            TextDirection::Rtl => {
                                Offset::new(-self.alignment_offset.dx(), self.alignment_offset.dy())
                            }
                            TextDirection::Ltr => self.alignment_offset,
                        }
                    } else {
                        self.alignment_offset
                    };
                desired_position = desired_position + directional_offset;
                x = desired_position.dx();
                y = desired_position.dy();
                match self.text_direction {
                    TextDirection::Rtl => x -= child_size.width(),
                    TextDirection::Ltr => {}
                }
            }
            Some(menu_position) => {
                let adjusted_position = menu_position + anchor_rect.top_left();
                x = adjusted_position.dx();
                y = adjusted_position.dy();
            }
        }

        // The whole overlay: the display-feature sub-screens are deferred.
        let allowed_rect = overlay_rect;
        let off_left_side = |x: f64| x < allowed_rect.left;
        let off_right_side = |x: f64| x + child_size.width() > allowed_rect.right;
        let off_top = |y: f64| y < allowed_rect.top;
        let off_bottom = |y: f64| y + child_size.height() > allowed_rect.bottom;
        // Avoid going outside an area defined as the rectangle offset from the
        // edge of the screen by the button padding. If the menu is off of the screen,
        // move the menu to the other side of the button first, and then if it
        // doesn't fit there, then just move it over as much as needed to make it
        // fit.
        if child_size.width() >= allowed_rect.width() {
            // It just doesn't fit, so put as much on the screen as possible.
            x = allowed_rect.left;
        } else if off_left_side(x) {
            // If the parent is a different orientation than the current one, then
            // just push it over instead of trying the other side.
            if self.parent_orientation != self.orientation {
                x = allowed_rect.left;
            } else {
                let new_x = anchor_rect.right + self.alignment_offset.dx();
                if !off_right_side(new_x) {
                    x = new_x;
                } else {
                    x = allowed_rect.left;
                }
            }
        } else if off_right_side(x) {
            if self.parent_orientation != self.orientation {
                x = allowed_rect.right - child_size.width();
            } else {
                let new_x = anchor_rect.left - child_size.width() - self.alignment_offset.dx();
                if !off_left_side(new_x) {
                    x = new_x;
                } else {
                    x = allowed_rect.right - child_size.width();
                }
            }
        }
        if child_size.height() >= allowed_rect.height() {
            // Too tall to fit, fit as much on as possible.
            y = allowed_rect.top;
        } else if off_top(y) {
            let new_y = anchor_rect.bottom;
            if !off_bottom(new_y) {
                y = new_y;
            } else {
                y = allowed_rect.top;
            }
        } else if off_bottom(y) {
            let new_y = anchor_rect.top - child_size.height();
            if !off_top(new_y) {
                // Only move the menu up if its parent is horizontal (MenuAnchor/MenuBar).
                if self.parent_orientation == Axis::Horizontal {
                    y = new_y - self.alignment_offset.dy();
                } else {
                    y = new_y;
                }
            } else {
                y = allowed_rect.bottom - child_size.height();
            }
        }
        Offset::new(x, y)
    }
}

impl SingleChildLayoutDelegate for MenuLayout {
    fn get_constraints_for_child(&self, constraints: BoxConstraints) -> BoxConstraints {
        // The menu can be at most the size of the overlay minus the view padding
        // in each direction.
        BoxConstraints::loose(constraints.biggest()).deflate(self.reserved_padding)
    }

    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        // size: The size of the overlay.
        // childSize: The size of the menu, when fully open, as determined by
        // getConstraintsForChild.
        let overlay_rect = self.media_query_data.padding.deflate_rect(
            self.media_query_data
                .view_insets
                .deflate_rect(Offset::ZERO & size),
        );
        let unconstrained_height = if self.height_factor > 0.01 {
            child_size.height() / self.height_factor
        } else {
            0.0
        };
        let child_height_estimate = unconstrained_height.min(size.height());
        let child_size_estimate = Size::new(child_size.width(), child_height_estimate);
        let final_position = self.position_child(child_size_estimate, overlay_rect);

        if self.menu_position.is_some() {
            return final_position;
        }

        // If the menu sits above the anchor when fully open, grow upward.
        // Keep the bottom (attachment) fixed by shifting the top-left during animation.
        let grows_up =
            final_position.dy() + child_size_estimate.height() <= self.anchor_rect.center().dy();
        if grows_up {
            let dy = child_height_estimate - child_size.height();
            return Offset::new(final_position.dx(), final_position.dy() + dy);
        }

        let initial_position = Offset::new(final_position.dx(), self.anchor_rect.bottom);
        Offset::lerp(
            Some(initial_position),
            Some(final_position),
            self.height_factor,
        )
        .expect("both ends")
    }

    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
        let Some(old_delegate) = old_delegate.as_any().downcast_ref::<MenuLayout>() else {
            return true;
        };
        self.anchor_rect != old_delegate.anchor_rect
            || self.text_direction != old_delegate.text_direction
            || self.alignment != old_delegate.alignment
            || self.alignment_offset != old_delegate.alignment_offset
            || self.menu_position != old_delegate.menu_position
            || self.menu_padding != old_delegate.menu_padding
            || self.orientation != old_delegate.orientation
            || self.parent_orientation != old_delegate.parent_orientation
            || self.reserved_padding != old_delegate.reserved_padding
            || self.height_factor != old_delegate.height_factor
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------------------------
// _MenuPanel

/// The first value a menu style gives, in the order Dart's `effectiveValue` asks the
/// widget's, the theme's and the default style.
fn effective_value<T>(
    app: &mut App,
    styles: [Option<&dyn MenuStyleGetters>; 3],
    get_property: impl Fn(&dyn MenuStyleGetters, &mut App) -> Option<T>,
) -> Option<T> {
    styles
        .into_iter()
        .flatten()
        .find_map(|style| get_property(style, app))
}

/// Dart's `resolve`: the first style property that resolves to a value with no state.
fn resolve<T: 'static>(
    app: &mut App,
    styles: [Option<&dyn MenuStyleGetters>; 3],
    get_property: impl Fn(&dyn MenuStyleGetters, &mut App) -> Option<WidgetStatePropertyRef<Option<T>>>,
) -> Option<T> {
    let states = WidgetStates::new();
    effective_value(app, styles, |style, app| {
        get_property(style, app).and_then(|property| property.resolve(&states))
    })
}

/// A widget that manages a list of menu buttons in a menu.
///
/// It sizes itself to the widest/tallest item it contains, and then sizes all
/// the other entries to match.
pub(crate) struct MenuPanel {
    /// The menu style that has all the attributes for this menu panel.
    menu_style: Option<MenuStyle>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to `Clip::None`.
    clip_behavior: Clip,

    /// Determine if a `UnconstrainedBox` can be applied to the menu panel to allow it to
    /// render at its "natural" size.
    ///
    /// Defaults to true. When it is set to false, it can be useful when the menu should
    /// be constrained in both main-axis and cross-axis, such as a `DropdownMenu`.
    cross_axis_unconstrained: bool,

    /// The layout orientation of this panel.
    pub(crate) orientation: Axis,

    /// The animation that controls the height of the menu panel.
    height_animation: Option<AnimationRef<f64>>,

    /// The list of widgets to use as children of this menu panel.
    ///
    /// These are the top level [`SubmenuButton`]s.
    children: Vec<WidgetRef>,
}

impl Debug for MenuPanel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_MenuPanel")
            .field("orientation", &self.orientation)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for MenuPanel {
    type State = MenuPanelState;

    fn create_state(&self) -> MenuPanelState {
        MenuPanelState {
            state: StateData::new(),
            scroll_controller: None,
        }
    }
}

/// Dart's `_MenuPanelState`.
pub(crate) struct MenuPanelState {
    state: StateData<MenuPanel>,
    // Dart's field initializer; an arena object needs the `App`, so `init_state` makes it.
    scroll_controller: Option<Ref<ScrollController>>,
}

impl MenuPanelState {
    fn intrinsic_cross_size(self: Handle<Self>, app: &App, child: WidgetRef) -> WidgetRef {
        match self.widget(app).orientation {
            Axis::Horizontal => IntrinsicHeight::new().child(child).into_widget(),
            Axis::Vertical => IntrinsicWidth::new().child(child).into_widget(),
        }
    }
}

impl State for MenuPanelState {
    type Widget = MenuPanel;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let scroll_controller = ScrollController::default(app);
        app.get_mut(self).scroll_controller = Some(scroll_controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(scroll_controller) = app.get_mut(self).scroll_controller.take() {
            scroll_controller.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let (theme_style, default_style) = match self.widget(app).orientation {
            Axis::Horizontal => (
                MenuBarTheme::of(app, context).style.clone(),
                Box::new(MenuBarDefaultsM3::new(context)) as Box<dyn MenuStyleGetters>,
            ),
            Axis::Vertical => (
                MenuTheme::of(app, context).style,
                Box::new(MenuDefaultsM3::new(context)) as Box<dyn MenuStyleGetters>,
            ),
        };
        let widget_style = self.widget(app).menu_style.clone();
        let styles: [Option<&dyn MenuStyleGetters>; 3] = [
            widget_style
                .as_ref()
                .map(|style| style as &dyn MenuStyleGetters),
            theme_style
                .as_ref()
                .map(|style| style as &dyn MenuStyleGetters),
            Some(&*default_style),
        ];

        let background_color = resolve(app, styles, |style, app| style.background_color(app));
        let shadow_color = resolve(app, styles, |style, app| style.shadow_color(app));
        let surface_tint_color = resolve(app, styles, |style, app| style.surface_tint_color(app));
        let elevation = resolve(app, styles, |style, app| style.elevation(app)).unwrap_or(0.0);
        let minimum_size = resolve(app, styles, |style, app| style.minimum_size(app));
        let fixed_size = resolve(app, styles, |style, app| style.fixed_size(app));
        let maximum_size = resolve(app, styles, |style, app| style.maximum_size(app));
        let side = resolve(app, styles, |style, app| style.side(app));
        let shape: ShapeBorderRef = resolve(app, styles, |style, app| style.shape(app))
            .expect("the default menu style has a shape")
            .copy_with(side)
            .into();
        let visual_density = effective_value(app, styles, |style, app| style.visual_density(app))
            .unwrap_or(VisualDensity::STANDARD);
        let padding = resolve(app, styles, |style, app| style.padding(app))
            .unwrap_or(EdgeInsetsGeometry::ZERO);
        let density_adjustment = visual_density.base_size_adjustment();
        // Scrollbar uses the `MediaQuery.padding` as its painter padding by
        // default. Menu overlays are already positioned within the safe area, so
        // applying that padding again shortens the scrollbar track and stops the
        // thumb from reaching the end of the menu. Remove the padding only for the
        // Scrollbar and restore the ambient MediaQuery for menu children.
        let ambient_media_query = MediaQuery::of(app, context);
        let scrollbar_media_query = ambient_media_query.remove_padding(true, true, true, true);
        // Per the Material Design team: don't allow the VisualDensity
        // adjustment to reduce the width of the left/right padding. If we
        // did, VisualDensity.compact, the default for desktop/web, would
        // reduce the horizontal padding to zero. Vertical padding
        // is not affected at all.
        let dx = density_adjustment.dx().max(0.0);
        let resolved_padding = padding
            .add(EdgeInsetsGeometry::symmetric(0.0, dx))
            .clamp(&EdgeInsetsGeometry::ZERO, &EdgeInsetsGeometry::INFINITY);

        let mut effective_constraints = visual_density.effective_constraints(
            BoxConstraints::new()
                .min_width(minimum_size.map_or(0.0, |size| size.width()))
                .min_height(minimum_size.map_or(0.0, |size| size.height()))
                .max_width(maximum_size.map_or(f64::INFINITY, |size| size.width()))
                .max_height(maximum_size.map_or(f64::INFINITY, |size| size.height())),
        );
        if let Some(fixed_size) = fixed_size {
            let size = effective_constraints.constrain(fixed_size);
            if size.width().is_finite() {
                effective_constraints = effective_constraints
                    .min_width(size.width())
                    .max_width(size.width());
            }
            if size.height().is_finite() {
                effective_constraints = effective_constraints
                    .min_height(size.height())
                    .max_height(size.height());
            }
        }

        // If the menu panel is horizontal, then the children should be wrapped in
        // an IntrinsicWidth widget to ensure that the children are as wide as the
        // widest child.
        let widget = self.widget(app);
        let mut children = widget.children.clone();
        if widget.orientation == Axis::Horizontal {
            children = children
                .into_iter()
                .map(|child| IntrinsicWidth::new().child(child).into_widget())
                .collect();
        }

        let display_scrollbar = match AnyMenuAnchorState::maybe_animation_status_of(app, context) {
            Some(AnimationStatus::Completed) => true,
            Some(AnimationStatus::Forward)
            | Some(AnimationStatus::Reverse)
            | Some(AnimationStatus::Dismissed)
            | None => false,
        };

        let widget = self.widget(app);
        let orientation = widget.orientation;
        let scroll_controller = app
            .get(self)
            .scroll_controller
            .as_ref()
            .expect("created in init_state")
            .erase::<AnyScrollController>();
        let behavior = ScrollConfiguration::of(app, context)
            .copy_with()
            .scrollbars(false)
            .overscroll(false)
            .physics(Rc::new(ClampingScrollPhysics::new()) as ScrollPhysicsRef);
        let text_direction = Directionality::of(app, context);
        let mut menu_panel = Padding::new(resolved_padding)
            .child(ScrollConfiguration::new(
                waft_widgets::ScrollBehaviorRef::new(behavior),
                PrimaryScrollController::new(
                    scroll_controller.clone(),
                    MediaQuery::new(
                        scrollbar_media_query,
                        Scrollbar::new(
                            MediaQuery::new(
                                ambient_media_query,
                                SingleChildScrollView::new()
                                    .controller(scroll_controller)
                                    .scroll_direction(orientation)
                                    .child(
                                        Flex::new(orientation)
                                            .cross_axis_alignment(CrossAxisAlignment::Start)
                                            .text_direction(text_direction)
                                            .main_axis_size(MainAxisSize::Min)
                                            .children(children),
                                    ),
                            )
                            .into_widget(),
                        )
                        .thumb_visibility(display_scrollbar),
                    )
                    .into_widget(),
                ),
            ))
            .into_widget();

        if let Some(height_animation) = self.widget(app).height_animation.clone() {
            // An AnimatedBuilder is used instead of SizeTransition because Material
            // already introduces its own ClipRRect for the shape.
            menu_panel = AnimatedBuilder::new(
                Rc::new(height_animation.clone()),
                move |app, _context, child| {
                    waft_widgets::Align::new()
                        .alignment(AlignmentGeometry::TOP_START)
                        .height_factor(height_animation.value(app))
                        .width_factor(1.0)
                        .child(child.cloned().expect("given a child"))
                        .into_widget()
                },
            )
            .child(menu_panel)
            .into_widget();
        }

        let mut material = Material::new()
            .elevation(elevation)
            .shape(shape)
            .r#type(if background_color.is_none() {
                MaterialType::Transparency
            } else {
                MaterialType::Canvas
            })
            .clip_behavior(self.widget(app).clip_behavior)
            .child(menu_panel);
        if let Some(background_color) = background_color {
            material = material.color(background_color);
        }
        if let Some(shadow_color) = shadow_color {
            material = material.shadow_color(shadow_color);
        }
        if let Some(surface_tint_color) = surface_tint_color {
            material = material.surface_tint_color(surface_tint_color);
        }
        menu_panel = self.intrinsic_cross_size(app, material.into_widget());

        let widget = self.widget(app);
        if widget.cross_axis_unconstrained {
            menu_panel = UnconstrainedBox::new()
                .constrained_axis(widget.orientation)
                .clip_behavior(Clip::HardEdge)
                .alignment(AlignmentGeometry::CENTER_START)
                .child(menu_panel)
                .into_widget();
        }

        ConstrainedBox::new(effective_constraints)
            .child(menu_panel)
            .into_widget()
    }
}

impl Debug for MenuPanelState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_MenuPanelState")
    }
}

// ---------------------------------------------------------------------------------------------
// _Submenu

// A widget that defines the menu drawn in the overlay.
struct Submenu {
    menu_scope_node: Ref<FocusScopeNode>,
    menu_position: RawMenuOverlayInfo,
    anchor: AnyMenuAnchorState,
    layer_link: Option<Ref<LayerLink>>,
    menu_style: Option<MenuStyle>,
    // Dart's field, which `build` does not read: it reads the anchor's `consume_outside_tap`.
    #[allow(dead_code)]
    consume_outside_taps: bool,
    alignment_offset: Offset,
    clip_behavior: Clip,
    cross_axis_unconstrained: bool,
    menu_children: Vec<WidgetRef>,
    fade_animation: AnimationRef<f64>,
    height_animation: AnimationRef<f64>,
    reserved_padding: EdgeInsetsGeometry,
}

impl Debug for Submenu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_Submenu")
            .field("anchor", &self.anchor)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for Submenu {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let anchor = self.anchor;
        // Use the text direction of the context where the button is.
        let text_direction = Directionality::of(app, context);
        let (theme_style, default_style) =
            match anchor.parent(app).map(AnyMenuAnchorState::orientation) {
                Some(Axis::Horizontal) | None => (
                    MenuBarTheme::of(app, context).style.clone(),
                    Box::new(MenuBarDefaultsM3::new(context)) as Box<dyn MenuStyleGetters>,
                ),
                Some(Axis::Vertical) => (
                    MenuTheme::of(app, context).style,
                    Box::new(MenuDefaultsM3::new(context)) as Box<dyn MenuStyleGetters>,
                ),
            };
        let styles: [Option<&dyn MenuStyleGetters>; 3] = [
            self.menu_style
                .as_ref()
                .map(|style| style as &dyn MenuStyleGetters),
            theme_style
                .as_ref()
                .map(|style| style as &dyn MenuStyleGetters),
            Some(&*default_style),
        ];

        // The mouse cursor resolves without the `App`, so it reads each style's property now.
        let cursor_properties = [
            self.menu_style
                .as_ref()
                .and_then(|style| style.mouse_cursor.clone()),
            theme_style
                .as_ref()
                .and_then(|style| style.mouse_cursor.clone()),
            default_style.mouse_cursor(app),
        ];
        let mouse_cursor = WidgetStateMouseCursor::resolve_with(
            move |states| {
                cursor_properties
                    .iter()
                    .flatten()
                    .find_map(|mouse_cursor| mouse_cursor.resolve(states))
                    .unwrap_or_else(<dyn MouseCursor>::uncontrolled)
            },
            Some("Menu_MouseCursor"),
        );

        let visual_density = effective_value(app, styles, |style, app| style.visual_density(app))
            .unwrap_or_else(|| Theme::of(app, context).visual_density);
        let alignment = effective_value(app, styles, |style, app| style.alignment(app))
            .expect("the default menu style has an alignment");
        let padding = resolve(app, styles, |style, app| style.padding(app))
            .unwrap_or(EdgeInsetsGeometry::ZERO);
        let density_adjustment = visual_density.base_size_adjustment();
        // Per the Material Design team: don't allow the VisualDensity
        // adjustment to reduce the width of the left/right padding. If we
        // did, VisualDensity.compact, the default for desktop/web, would
        // reduce the horizontal padding to zero.
        let dx = density_adjustment.dx().max(0.0);
        let resolved_padding = padding
            .add(EdgeInsetsGeometry::from_ltrb(dx, 0.0, dx, 0.0))
            .clamp(&EdgeInsetsGeometry::ZERO, &EdgeInsetsGeometry::INFINITY);

        let anchor_rect = if self.layer_link.is_none() {
            Rect::from_ltrb(
                self.menu_position.anchor_rect.left + dx,
                self.menu_position.anchor_rect.top,
                self.menu_position.anchor_rect.right,
                self.menu_position.anchor_rect.bottom,
            )
        } else {
            Rect::ZERO
        };

        let dismiss_menu_action = DismissMenuAction::new(app, anchor.menu_controller(app));
        let mut actions: HashMap<TypeId, ActionRef> = HashMap::new();
        actions.insert(
            <DismissMenuAction as Action>::intent_type(),
            dismiss_menu_action.into_erased(),
        );

        let consume_outside_taps = anchor.root(app).menu_controller(app).is_open(app)
            && anchor.widget(app).consume_outside_tap;
        let menu_panel = TapRegion::new(
            MouseRegion::new()
                .cursor(mouse_cursor.into())
                .hit_test_behavior(HitTestBehavior::DeferToChild)
                .child(Actions::new(
                    actions,
                    Shortcuts::new(
                        menu_traversal_shortcuts(),
                        FocusScope::new(FadeTransition::new(self.fade_animation.clone()).child(
                            MenuPanel {
                                menu_style: self.menu_style.clone(),
                                clip_behavior: self.clip_behavior,
                                orientation: anchor.orientation(),
                                cross_axis_unconstrained: self.cross_axis_unconstrained,
                                height_animation: Some(self.height_animation.clone()),
                                children: self.menu_children.clone(),
                            },
                        ))
                        .node(self.menu_scope_node.clone())
                        .skip_traversal(true),
                    ),
                )),
        )
        .group_id(self.menu_position.tap_region_group_id)
        .consume_outside_taps(consume_outside_taps)
        .on_tap_outside(Rc::new(move |app: &mut App, _event: PointerDownEvent| {
            anchor.menu_controller(app).close(app);
        }))
        .into_widget();

        let menu_padding = resolved_padding;
        let alignment_offset = self.alignment_offset;
        let menu_position = self.menu_position.position;
        let orientation = anchor.orientation();
        let parent_orientation = anchor
            .parent(app)
            .map_or(Axis::Horizontal, AnyMenuAnchorState::orientation);
        let reserved_padding = self.reserved_padding;
        let height_animation = self.height_animation.clone();
        let theme = Theme::of(app, context)
            .copy_with()
            .visual_density(visual_density)
            .build();
        let layout = Theme::new(
            theme,
            ConstrainedBox::new(BoxConstraints::loose(self.menu_position.overlay_size)).child(
                AnimatedBuilder::new(
                    Rc::new(height_animation.clone()),
                    move |app, context, _child| {
                        let media_query = MediaQuery::of(app, context);
                        CustomSingleChildLayout::new(Rc::new(MenuLayout {
                            anchor_rect,
                            text_direction,
                            menu_padding,
                            alignment,
                            alignment_offset,
                            menu_position,
                            orientation,
                            parent_orientation,
                            reserved_padding,
                            height_factor: height_animation.value(app),
                            media_query_data: media_query,
                        }))
                        .child(menu_panel.clone())
                        .into_widget()
                    },
                ),
            ),
        )
        .into_widget();

        match self.layer_link.clone() {
            None => layout,
            Some(layer_link) => CompositedTransformFollower::new(layer_link)
                .target_anchor(Alignment::BOTTOM_LEFT)
                .child(layout)
                .into_widget(),
        }
    }
}

/// Whether the target platform is an Apple platform (Mac or iOS).
fn is_cupertino(app: &App) -> bool {
    match app.platform().target_platform() {
        TargetPlatform::IOS | TargetPlatform::MacOS => true,
        TargetPlatform::Android
        | TargetPlatform::Fuchsia
        | TargetPlatform::Linux
        | TargetPlatform::Windows => false,
    }
}

/// Whether the target platform is one that uses symbolic shortcuts.
///
/// Mac and iOS use special symbols for modifier keys instead of their names,
/// render them in a particular order defined by Apple's human interface
/// guidelines, and format them so that the modifier keys always align.
fn uses_symbolic_modifiers(app: &App) -> bool {
    is_cupertino(app)
}

fn platform_supports_accelerators(app: &App) -> bool {
    // On iOS and macOS, pressing the Option key (a.k.a. the Alt key) causes a
    // different set of characters to be generated, and the native menus don't
    // support accelerators anyhow, so we just disable accelerators on these
    // platforms.
    !is_cupertino(app)
}

#[cfg(test)]
mod tests {
    //! Ported from `test/menu_anchor_test.dart`; the goldens and the semantics-tree cases are
    //! not ported.

    use std::cell::RefCell;

    use waft_services::PhysicalKeyboardKey;
    use waft_widgets::{AnyElement, Column, Container, Icon, primary_focus};

    use waft_rendering::testing::{PaintCall, assert_paints, paints};
    use waft_widgets::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, assert_more_or_less_equals_within,
        expect, find, finds_nothing, finds_one_widget, test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::test_support::{PaintInspection, material_app};

    /// What the first ink features render object and its subtree paint: Dart's `paints` on
    /// `getOverlayColor(tester)`.
    fn overlay_ops(tester: &WidgetTester) -> Vec<waft_embedder::valo::Op> {
        let render = waft_rendering::RenderBox::as_object(
            tester.first::<crate::material::RenderInkFeatures>(),
        );
        tester.painted_ops_deep(render)
    }
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::m3;

    // ---- Dart's `TestMenu` and `createTestMenus` ----

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TestMenu {
        MainMenu0,
        MainMenu1,
        MainMenu2,
        MainMenu3,
        MainMenu4,
        MainMenu5,
        SubMenu00,
        SubMenu01,
        SubMenu02,
        SubMenu10,
        SubMenu11,
        SubMenu12,
        SubMenu20,
        SubMenu30,
        SubMenu40,
        SubMenu41,
        SubMenu42,
        SubSubMenu110,
        SubSubMenu111,
        SubSubMenu112,
        SubSubMenu113,
        AnchorButton,
        OutsideButton,
    }

    impl TestMenu {
        fn accelerator_label(self) -> &'static str {
            match self {
                TestMenu::MainMenu0 => "&Menu 0",
                TestMenu::MainMenu1 => "M&enu &1",
                TestMenu::MainMenu2 => "Me&nu 2",
                TestMenu::MainMenu3 => "Men&u 3",
                TestMenu::MainMenu4 => "Menu &4",
                TestMenu::MainMenu5 => "Menu &5 && &6 &",
                TestMenu::SubMenu00 => "Sub &Menu 0&0",
                TestMenu::SubMenu01 => "Sub Menu 0&1",
                TestMenu::SubMenu02 => "Sub Menu 0&2",
                TestMenu::SubMenu10 => "Sub Menu 1&0",
                TestMenu::SubMenu11 => "Sub Menu 1&1",
                TestMenu::SubMenu12 => "Sub Menu 1&2",
                TestMenu::SubMenu20 => "Sub Menu 2&0",
                TestMenu::SubMenu30 => "Sub Menu 3&0",
                TestMenu::SubMenu40 => "Sub Menu 4&0",
                TestMenu::SubMenu41 => "Sub Menu 4&1",
                TestMenu::SubMenu42 => "Sub Menu 4&2",
                TestMenu::SubSubMenu110 => "Sub Sub Menu 11&0",
                TestMenu::SubSubMenu111 => "Sub Sub Menu 11&1",
                TestMenu::SubSubMenu112 => "Sub Sub Menu 11&2",
                TestMenu::SubSubMenu113 => "Sub Sub Menu 11&3",
                TestMenu::AnchorButton => "Press Me",
                TestMenu::OutsideButton => "Outside",
            }
        }

        /// Dart's `label`: the accelerator markers stripped.
        fn label(self) -> String {
            MenuAcceleratorLabel::strip_accelerator_markers(self.accelerator_label(), None)
        }
    }

    /// The lists Dart's `onPressed`, `onOpen` and `onClose` test callbacks add to.
    #[derive(Clone, Default)]
    struct Log {
        selected: Rc<RefCell<Vec<TestMenu>>>,
        opened: Rc<RefCell<Vec<TestMenu>>>,
        closed: Rc<RefCell<Vec<TestMenu>>>,
    }

    impl Log {
        fn selected(&self) -> Vec<TestMenu> {
            self.selected.borrow().clone()
        }

        fn opened(&self) -> Vec<TestMenu> {
            self.opened.borrow().clone()
        }

        fn closed(&self) -> Vec<TestMenu> {
            self.closed.borrow().clone()
        }

        fn clear(&self) {
            self.selected.borrow_mut().clear();
            self.opened.borrow_mut().clear();
            self.closed.borrow_mut().clear();
        }
    }

    /// Dart's `createTestMenus` arguments.
    #[derive(Clone, Default)]
    struct TestMenus {
        on_pressed: Option<Rc<RefCell<Vec<TestMenu>>>>,
        on_open: Option<Rc<RefCell<Vec<TestMenu>>>>,
        on_close: Option<Rc<RefCell<Vec<TestMenu>>>>,
        shortcuts: Vec<(TestMenu, MenuSerializableShortcutRef)>,
        include_extra_groups: bool,
        accelerators: bool,
    }

    impl TestMenus {
        /// `createTestMenus(onPressed: onPressed)`.
        fn pressed(log: &Log) -> TestMenus {
            TestMenus {
                on_pressed: Some(Rc::clone(&log.selected)),
                ..TestMenus::default()
            }
        }

        /// `createTestMenus(onPressed: onPressed, onOpen: onOpen, onClose: onClose)`.
        fn all(log: &Log) -> TestMenus {
            TestMenus {
                on_pressed: Some(Rc::clone(&log.selected)),
                on_open: Some(Rc::clone(&log.opened)),
                on_close: Some(Rc::clone(&log.closed)),
                ..TestMenus::default()
            }
        }

        fn label(&self, menu: TestMenu) -> WidgetRef {
            if self.accelerators {
                MenuAcceleratorLabel::new(menu.accelerator_label()).into_widget()
            } else {
                Text::new(menu.label()).into_widget()
            }
        }

        fn submenu_button(&self, menu: TestMenu, menu_children: Vec<WidgetRef>) -> WidgetRef {
            let mut button = SubmenuButton::new(menu_children, Some(self.label(menu)));
            if let Some(opened) = self.on_open.clone() {
                button = button.on_open(Rc::new(move |_app: &mut App| {
                    opened.borrow_mut().push(menu)
                }));
            }
            if let Some(closed) = self.on_close.clone() {
                button = button.on_close(Rc::new(move |_app: &mut App| {
                    closed.borrow_mut().push(menu)
                }));
            }
            button.into_widget()
        }

        fn menu_item_button(
            &self,
            menu: TestMenu,
            enabled: bool,
            leading_icon: Option<WidgetRef>,
            key: Option<KeyRef>,
        ) -> WidgetRef {
            let mut button = MenuItemButton::new().child(self.label(menu));
            button.key = key;
            if enabled && let Some(selected) = self.on_pressed.clone() {
                button =
                    button.on_pressed(Listener::new(move |_app| selected.borrow_mut().push(menu)));
            }
            button.shortcut = self
                .shortcuts
                .iter()
                .find(|(shortcut_menu, _)| *shortcut_menu == menu)
                .map(|(_, shortcut)| Rc::clone(shortcut));
            button.leading_icon = leading_icon;
            button.into_widget()
        }

        /// Dart's `createTestMenus`.
        fn build(&self) -> Vec<WidgetRef> {
            let item = |menu| self.menu_item_button(menu, true, None, None);
            let disabled = |menu| self.menu_item_button(menu, false, None, None);
            let mut result = vec![
                self.submenu_button(
                    TestMenu::MainMenu0,
                    vec![
                        self.menu_item_button(
                            TestMenu::SubMenu00,
                            true,
                            Some(Icon::new(Some(Icons::add())).into_widget()),
                            None,
                        ),
                        item(TestMenu::SubMenu01),
                        item(TestMenu::SubMenu02),
                    ],
                ),
                self.submenu_button(
                    TestMenu::MainMenu1,
                    vec![
                        item(TestMenu::SubMenu10),
                        self.submenu_button(
                            TestMenu::SubMenu11,
                            vec![
                                self.menu_item_button(
                                    TestMenu::SubSubMenu110,
                                    true,
                                    None,
                                    Some(waft_foundation::UniqueKey::new().into()),
                                ),
                                item(TestMenu::SubSubMenu111),
                                item(TestMenu::SubSubMenu112),
                                item(TestMenu::SubSubMenu113),
                            ],
                        ),
                        item(TestMenu::SubMenu12),
                    ],
                ),
                self.submenu_button(
                    TestMenu::MainMenu2,
                    vec![self.menu_item_button(
                        TestMenu::SubMenu20,
                        false,
                        Some(Icon::new(Some(Icons::ac_unit())).into_widget()),
                        None,
                    )],
                ),
            ];
            if self.include_extra_groups {
                result.push(
                    self.submenu_button(TestMenu::MainMenu3, vec![disabled(TestMenu::SubMenu30)]),
                );
                result.push(self.submenu_button(
                    TestMenu::MainMenu4,
                    vec![
                        disabled(TestMenu::SubMenu40),
                        disabled(TestMenu::SubMenu41),
                        disabled(TestMenu::SubMenu42),
                    ],
                ));
            }
            result.push(self.submenu_button(TestMenu::MainMenu5, Vec::new()));
            result
        }
    }

    // ---- finders and gestures ----

    fn only(found: Vec<AnyElement>) -> AnyElement {
        assert_eq!(found.len(), 1, "one widget found");
        found[0]
    }

    /// Dart's `find.text(text)` with `findsOneWidget` (true) or `findsNothing` (false).
    ///
    /// The kit's onstage finders do not see an `OverlayPortal`'s child, where a menu is, so
    /// these finders count every mounted `Text`; no case here builds offstage text.
    fn shows(tester: &WidgetTester, text: &str) -> bool {
        match find::text(text).evaluate(tester).len() {
            0 => false,
            1 => true,
            count => panic!("{count} widgets show {text:?}"),
        }
    }

    /// Dart's `tester.getRect(find.byType(W))` of the only one.
    fn rect_of<W: 'static>(tester: &WidgetTester) -> Rect {
        tester.get_rect(tester.render_box_of(tester.element(find::by_type::<W>())))
    }

    fn rect_of_element(tester: &WidgetTester, element: AnyElement) -> Rect {
        tester.get_rect(tester.render_box_of(element))
    }

    /// Dart's `find.ancestor(of: find.text(text), matching: find.byType(W))`, nearest first.
    fn ancestors_of_type<W: 'static>(tester: &WidgetTester, text: &str) -> Vec<AnyElement> {
        let ancestors = tester.ancestors(tester.element(find::text(text)));
        let app = tester.cell.borrow();
        ancestors
            .into_iter()
            .filter(|ancestor| {
                waft_widgets::downcast_widget::<W>(&**ancestor.widget(&app)).is_some()
            })
            .collect()
    }

    /// Dart's `findMenuPanels()`.
    fn menu_panels(tester: &WidgetTester) -> Vec<AnyElement> {
        find::by_type::<MenuPanel>().evaluate(tester)
    }

    /// Dart's `hoverOver`: a new mouse, as Dart's `createGesture` makes one, moved over `at`,
    /// then a frame.
    fn hover_over(tester: &mut WidgetTester, at: Offset) -> TestPointer {
        use std::sync::atomic::{AtomicI64, Ordering};
        static NEXT_DEVICE: AtomicI64 = AtomicI64::new(1);
        let mut mouse = TestPointer::mouse(NEXT_DEVICE.fetch_add(1, Ordering::Relaxed), 0);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        tester.move_to(&mut mouse, at, false);
        tester.pump();
        mouse
    }

    /// Dart's `tester.sendKeyEvent(key)`.
    fn send_key(tester: &mut WidgetTester, logical: LogicalKeyboardKey) {
        tester.send_key_event(physical_of(logical), logical);
    }

    /// The physical key flutter_test's key simulator sends with `logical`.
    fn physical_of(logical: LogicalKeyboardKey) -> PhysicalKeyboardKey {
        match logical {
            LogicalKeyboardKey::ARROW_DOWN => PhysicalKeyboardKey::ARROW_DOWN,
            LogicalKeyboardKey::ARROW_UP => PhysicalKeyboardKey::ARROW_UP,
            LogicalKeyboardKey::ARROW_LEFT => PhysicalKeyboardKey::ARROW_LEFT,
            LogicalKeyboardKey::ARROW_RIGHT => PhysicalKeyboardKey::ARROW_RIGHT,
            LogicalKeyboardKey::TAB => PhysicalKeyboardKey::TAB,
            LogicalKeyboardKey::ESCAPE => PhysicalKeyboardKey::ESCAPE,
            LogicalKeyboardKey::ENTER => PhysicalKeyboardKey::ENTER,
            LogicalKeyboardKey::SPACE => PhysicalKeyboardKey::SPACE,
            LogicalKeyboardKey::SHIFT_LEFT => PhysicalKeyboardKey::SHIFT_LEFT,
            LogicalKeyboardKey::ALT_LEFT => PhysicalKeyboardKey::ALT_LEFT,
            LogicalKeyboardKey::KEY_M => PhysicalKeyboardKey::KEY_M,
            LogicalKeyboardKey::KEY_E => PhysicalKeyboardKey::KEY_E,
            LogicalKeyboardKey::KEY_N => PhysicalKeyboardKey::KEY_N,
            LogicalKeyboardKey::KEY_U => PhysicalKeyboardKey::KEY_U,
            LogicalKeyboardKey::DIGIT1 => PhysicalKeyboardKey::DIGIT1,
            LogicalKeyboardKey::DIGIT0 => PhysicalKeyboardKey::DIGIT0,
            _ => panic!("no physical key for {logical:?} in this kit"),
        }
    }

    /// Dart's `focusedMenu`: the debug label of the primary focus.
    fn focused_menu(tester: &WidgetTester) -> Option<String> {
        let mut app = tester.cell.borrow_mut();
        primary_focus(&mut app).and_then(|node| node.debug_label(&app))
    }

    /// Dart's `tester.pumpWidget(MaterialApp(theme: theme(app), home: Material(child: child)))`.
    fn pump_material(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        child: impl FnOnce(&mut App) -> WidgetRef,
    ) {
        let root = {
            let mut app = tester.cell.borrow_mut();
            let theme = theme(&app);
            let child = child(&mut app);
            material_app(theme, Material::new().child(child).into_widget())
        };
        tester.pump_widget(root);
    }

    fn pump(tester: &mut WidgetTester) {
        tester.pump();
    }

    /// The debug label a button gives its own focus node: Dart's `'$Type(${widget.child})'`
    /// of a `Text(text)` child, as `focusedMenu` reads it.
    fn button_label(type_name: &str, text: &str) -> String {
        describe_child(type_name, &Some(Text::new(text).into_widget()))
    }

    /// Dart's `const Placeholder()` where a case only needs it to fill its slot.
    fn placeholder() -> WidgetRef {
        ConstrainedBox::new(BoxConstraints::expand(None, None)).into_widget()
    }

    // ---- cases ----

    #[test]
    fn a_menu_bar_opens_its_submenus_level_by_level() {
        test_widgets(|tester| {
            let log = Log::default();
            let controller = MenuController::new(&mut tester.cell.borrow_mut())
                .into_erased::<AnyMenuController>();
            let menus = TestMenus::all(&log);
            pump_material(tester, m3, |_app| {
                MenuBar::new(menus.build())
                    .controller(controller.clone())
                    .into_widget()
            });

            assert!(shows(tester, &TestMenu::MainMenu0.label()));
            assert!(shows(tester, &TestMenu::MainMenu1.label()));
            assert!(shows(tester, &TestMenu::MainMenu2.label()));
            assert!(!shows(tester, &TestMenu::SubMenu10.label()));
            assert!(!shows(tester, &TestMenu::SubSubMenu110.label()));
            assert!(log.opened().is_empty());

            tester.tap(find::text(&TestMenu::MainMenu1.label()));
            pump(tester);

            assert!(shows(tester, &TestMenu::MainMenu0.label()));
            assert!(shows(tester, &TestMenu::MainMenu1.label()));
            assert!(shows(tester, &TestMenu::MainMenu2.label()));
            assert!(shows(tester, &TestMenu::SubMenu10.label()));
            assert!(shows(tester, &TestMenu::SubMenu11.label()));
            assert!(shows(tester, &TestMenu::SubMenu12.label()));
            assert!(!shows(tester, &TestMenu::SubSubMenu110.label()));
            assert!(!shows(tester, &TestMenu::SubSubMenu111.label()));
            assert!(!shows(tester, &TestMenu::SubSubMenu112.label()));
            assert_eq!(log.opened().last(), Some(&TestMenu::MainMenu1));
            log.clear();

            tester.tap(find::text(&TestMenu::SubMenu11.label()));
            pump(tester);

            assert!(shows(tester, &TestMenu::MainMenu0.label()));
            assert!(shows(tester, &TestMenu::MainMenu1.label()));
            assert!(shows(tester, &TestMenu::MainMenu2.label()));
            assert!(shows(tester, &TestMenu::SubMenu10.label()));
            assert!(shows(tester, &TestMenu::SubMenu11.label()));
            assert!(shows(tester, &TestMenu::SubMenu12.label()));
            assert!(shows(tester, &TestMenu::SubSubMenu110.label()));
            assert!(shows(tester, &TestMenu::SubSubMenu111.label()));
            assert!(shows(tester, &TestMenu::SubSubMenu112.label()));
            assert_eq!(log.opened().last(), Some(&TestMenu::SubMenu11));
        });
    }

    mod functions {
        use std::cell::Cell;
        use waft_painting::ShapeBorder;

        use waft_widgets::{Center, Column};

        use super::*;
        use crate::colors::Colors;
        use crate::elevated_button::ElevatedButton;
        use crate::filled_button::FilledButton;
        use crate::tooltip::tests::m2;

        /// Dart's `find.descendant(of: of, matching: find.byType(Material)).first`, read through
        /// `read`.
        fn first_material_under<R>(
            tester: &WidgetTester,
            of: AnyElement,
            read: impl FnOnce(&Material) -> R,
        ) -> R {
            let material = tester.descendants_of_type::<Material>(of)[0];
            tester.widget::<Material, R>(material, read)
        }

        /// The `W` that has a `Text(text)` below it: Dart's `find.widgetWithText(W, text)`.
        fn widget_with_text<W: 'static>(tester: &WidgetTester, text: &str) -> AnyElement {
            ancestors_of_type::<W>(tester, text)[0]
        }

        /// Dart's `getMenuBarMaterial(tester)`: the first `Material` under the first menu panel.
        fn menu_bar_material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
            let panel = menu_panels(tester)[0];
            let material = tester.descendants_of_type::<Material>(panel)[0];
            tester.widget::<Material, R>(material, read)
        }

        fn shape_is(material: &Material, expected: &dyn ShapeBorder) -> bool {
            material
                .shape
                .as_ref()
                .is_some_and(|shape| shape.eq_shape(expected))
        }

        fn default_menu_border() -> waft_painting::RoundedRectangleBorder {
            waft_painting::RoundedRectangleBorder::new(
                waft_painting::BorderSide::NONE,
                waft_painting::BorderRadiusGeometry::all(waft_embedder::Radius::circular(4.0)),
            )
        }

        /// The defaults a menu panel's `Material` takes from `theme`.
        fn assert_menu_material_defaults(material: &Material, theme: &ThemeData) {
            assert_eq!(material.color, Some(theme.color_scheme.surface_container()));
            assert_eq!(material.shadow_color, Some(theme.color_scheme.shadow()));
            assert_eq!(material.surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(material.elevation, 3.0);
            assert!(shape_is(material, &default_menu_border()));
        }

        /// Dart's `buildTestApp` arguments.
        #[derive(Clone)]
        struct TestApp {
            alignment: Option<AlignmentGeometry>,
            alignment_offset: Offset,
            text_direction: TextDirection,
            consumes_outside_tap: bool,
            log: Option<Log>,
        }

        impl Default for TestApp {
            fn default() -> TestApp {
                TestApp {
                    alignment: None,
                    alignment_offset: Offset::ZERO,
                    text_direction: TextDirection::Ltr,
                    consumes_outside_tap: false,
                    log: None,
                }
            }
        }

        /// Dart's `buildTestApp(..)`, with its `controller` and `menuItemKey`.
        fn build_test_app(
            tester: &mut WidgetTester,
            options: TestApp,
            controller: MenuControllerRef,
            menu_item_key: &GlobalKey,
        ) {
            let menu_item_key = menu_item_key.clone();
            pump_material(tester, m2, move |app| {
                let focus_node = FocusNode::new(app).into_erased::<AnyFocusNode>();
                let push = |list: Option<Rc<RefCell<Vec<TestMenu>>>>, menu: TestMenu| {
                    Listener::new(move |_app| {
                        if let Some(list) = &list {
                            list.borrow_mut().push(menu);
                        }
                    })
                };
                let log = options.log.clone();
                let selected = log.as_ref().map(|log| Rc::clone(&log.selected));
                let opened = log.as_ref().map(|log| Rc::clone(&log.opened));
                let closed = log.as_ref().map(|log| Rc::clone(&log.closed));
                let mut style = MenuStyle::new();
                if let Some(alignment) = options.alignment {
                    style = style.alignment(alignment);
                }
                let on_outside = push(selected.clone(), TestMenu::OutsideButton);
                let on_anchor = push(selected.clone(), TestMenu::AnchorButton);
                let on_open = push(opened, TestMenu::AnchorButton);
                let on_close = push(closed, TestMenu::AnchorButton);
                let anchor = MenuAnchor::new(vec![
                    MenuItemButton::new()
                        .key(menu_item_key.clone().into())
                        .shortcut(Rc::new(
                            SingleActivator::new(LogicalKeyboardKey::KEY_B).control(true),
                        ))
                        .on_pressed(push(selected.clone(), TestMenu::SubMenu00))
                        .child(Text::new(TestMenu::SubMenu00.label()))
                        .into_widget(),
                    MenuItemButton::new()
                        .leading_icon(Icon::new(Some(Icons::send())))
                        .trailing_icon(Icon::new(Some(Icons::mail())))
                        .on_pressed(push(selected, TestMenu::SubMenu01))
                        .child(Text::new(TestMenu::SubMenu01.label()))
                        .into_widget(),
                ])
                .child_focus_node(focus_node.clone())
                .controller(controller.clone())
                .alignment_offset(options.alignment_offset)
                .consume_outside_tap(options.consumes_outside_tap)
                .style(style)
                .on_open(Rc::new(move |app: &mut App| on_open.call(app)))
                .on_close(Rc::new(move |app: &mut App| on_close.call(app)))
                .builder(move |_app, _context, controller, child| {
                    let on_anchor = on_anchor.clone();
                    ElevatedButton::new(
                        Some(Listener::new(move |app| {
                            if controller.is_open(app) {
                                controller.close(app);
                            } else {
                                controller.open(app, None);
                            }
                            on_anchor.call(app);
                        })),
                        child.cloned(),
                    )
                    .focus_node(focus_node.clone())
                    .into_widget()
                })
                .child(Text::new(TestMenu::AnchorButton.label()));
                Directionality::new(
                    options.text_direction,
                    Column::new()
                        .children(vec![
                            waft_widgets::GestureDetector::new()
                                .on_tap(on_outside)
                                .child(Text::new(TestMenu::OutsideButton.label()))
                                .into_widget(),
                            anchor.into_widget(),
                        ])
                        .into_widget(),
                )
                .into_widget()
            });
        }

        fn controller(tester: &WidgetTester) -> MenuControllerRef {
            MenuController::new(&mut tester.cell.borrow_mut()).into_erased()
        }

        /// Dart's `find.ancestor(of: find.byKey(key), matching: find.byType(FocusScope)).first`.
        fn menu_scope_rect_of_key(tester: &WidgetTester, key: &GlobalKey) -> Rect {
            let element = {
                let app = tester.cell.borrow();
                key.current_context(&app)
                    .expect("the keyed widget is mounted")
            };
            let scope = tester
                .ancestors(element)
                .into_iter()
                .find(|ancestor| {
                    let app = tester.cell.borrow();
                    waft_widgets::downcast_widget::<FocusScope>(&**ancestor.widget(&app)).is_some()
                })
                .expect("a FocusScope above the item");
            rect_of_element(tester, scope)
        }

        /// Dart's `find.ancestor(of: find.text(text), matching: find.byType(FocusScope)).first`.
        fn menu_scope_rect(tester: &WidgetTester, text: &str) -> Rect {
            rect_of_element(tester, ancestors_of_type::<FocusScope>(tester, text)[0])
        }

        /// A menu bar of the test menus in an `Expanded` of a `Row`, above a placeholder.
        fn expanded_bar(menus: &TestMenus) -> WidgetRef {
            Column::new()
                .children(vec![
                    Row::new()
                        .children(vec![
                            Expanded::new(MenuBar::new(menus.build())).into_widget(),
                        ])
                        .into_widget(),
                    Expanded::new(placeholder()).into_widget(),
                ])
                .into_widget()
        }

        // menu_anchor_test.dart 'Menu defaults'.
        #[test]
        fn menus_take_their_default_colors_shapes_and_text_styles_from_the_theme() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let menus = TestMenus::all(&log);
                let theme = ThemeData::new(&tester.cell.borrow());
                let theme_for_app = theme.clone();
                pump_material(
                    tester,
                    move |_app| theme_for_app,
                    |_app| {
                        MenuBar::new(menus.build())
                            .controller(controller)
                            .into_widget()
                    },
                );

                // Menu bar (horizontal menu).
                let text_button = find::by_type::<TextButton>().evaluate(tester)[0];
                let menu_material = tester.ancestor_of_type::<Material>(text_button);
                assert!(log.opened().is_empty());
                tester.widget::<Material, _>(menu_material, |material| {
                    assert_menu_material_defaults(material, &theme);
                });

                first_material_under(tester, text_button, |material| {
                    assert_eq!(material.color, Some(Colors::TRANSPARENT));
                    assert_eq!(material.elevation, 0.0);
                    assert!(shape_is(
                        material,
                        &waft_painting::RoundedRectangleBorder::default()
                    ));
                    let text_style = material.text_style.as_ref().expect("a text style");
                    assert_eq!(text_style.color, Some(theme.color_scheme.on_surface.into()));
                    assert_eq!(text_style.font_size, Some(14.0));
                    assert_eq!(text_style.height, Some(1.43));
                });

                // Vertical menu.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                let item = widget_with_text::<TextButton>(tester, &TestMenu::SubMenu10.label());
                let menu_material = tester.ancestor_of_type::<Material>(item);
                assert_eq!(log.opened().last(), Some(&TestMenu::MainMenu1));
                tester.widget::<Material, _>(menu_material, |material| {
                    assert_menu_material_defaults(material, &theme);
                });

                first_material_under(tester, item, |material| {
                    assert_eq!(material.color, Some(Colors::TRANSPARENT));
                    assert_eq!(material.elevation, 0.0);
                    assert!(shape_is(
                        material,
                        &waft_painting::RoundedRectangleBorder::default()
                    ));
                    let text_style = material.text_style.as_ref().expect("a text style");
                    assert_eq!(text_style.color, Some(theme.color_scheme.on_surface.into()));
                    assert_eq!(text_style.font_size, Some(14.0));
                    assert_eq!(text_style.height, Some(1.43));
                });

                tester.tap(find::text(&TestMenu::MainMenu0.label()));
                pump(tester);
                let icon = tester.element(find::by_icon(&Icons::add()).first());
                let glyph = tester.glyph_style(icon);
                assert_eq!(
                    glyph.color,
                    Some(theme.color_scheme.on_surface_variant().into())
                );
            });
        }

        // menu_anchor_test.dart 'Menu defaults - disabled'.
        #[test]
        fn disabled_menu_items_take_the_disabled_default_colors() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let menus = TestMenus::all(&log);
                let theme = ThemeData::new(&tester.cell.borrow());
                let theme_for_app = theme.clone();
                pump_material(
                    tester,
                    move |_app| theme_for_app,
                    |_app| {
                        MenuBar::new(menus.build())
                            .controller(controller)
                            .into_widget()
                    },
                );
                let disabled = theme.color_scheme.on_surface.with_opacity(0.38);

                // Menu bar (horizontal menu).
                let button = widget_with_text::<TextButton>(tester, &TestMenu::MainMenu5.label());
                let menu_material = tester.ancestor_of_type::<Material>(button);
                assert!(log.opened().is_empty());
                tester.widget::<Material, _>(menu_material, |material| {
                    assert_menu_material_defaults(material, &theme);
                });
                first_material_under(tester, button, |material| {
                    assert_eq!(material.color, Some(Colors::TRANSPARENT));
                    assert_eq!(material.elevation, 0.0);
                    assert!(shape_is(
                        material,
                        &waft_painting::RoundedRectangleBorder::default()
                    ));
                    let text_style = material.text_style.as_ref().expect("a text style");
                    assert_eq!(text_style.color, Some(disabled.into()));
                });

                // Vertical menu.
                tester.tap(find::text(&TestMenu::MainMenu2.label()));
                pump(tester);

                let item = widget_with_text::<TextButton>(tester, &TestMenu::SubMenu20.label());
                let menu_material = tester.ancestor_of_type::<Material>(item);
                tester.widget::<Material, _>(menu_material, |material| {
                    assert_menu_material_defaults(material, &theme);
                });
                first_material_under(tester, item, |material| {
                    assert_eq!(material.color, Some(Colors::TRANSPARENT));
                    assert_eq!(material.elevation, 0.0);
                    assert!(shape_is(
                        material,
                        &waft_painting::RoundedRectangleBorder::default()
                    ));
                    let text_style = material.text_style.as_ref().expect("a text style");
                    assert_eq!(text_style.color, Some(disabled.into()));
                });

                let icon = tester.element(find::by_icon(&Icons::ac_unit()).first());
                let glyph = tester.glyph_style(icon);
                assert_eq!(glyph.color, Some(disabled.into()));
            });
        }

        // menu_anchor_test.dart 'Focus is returned to previous focus before invoking onPressed'.
        #[test]
        fn focus_returns_to_where_it_was_before_an_item_is_invoked() {
            test_widgets(|tester| {
                let controller = controller(tester);
                let button_focus = {
                    let mut app = tester.cell.borrow_mut();
                    let node = FocusNode::new(&mut app).into_erased::<AnyFocusNode>();
                    node.set_debug_label(&mut app, Some("Button Focus".to_string()));
                    node
                };
                let focus_in_on_pressed: Rc<Cell<Option<AnyFocusNode>>> = Rc::default();
                let sink = Rc::clone(&focus_in_on_pressed);
                let on_menu_selected: MenuCallback =
                    Rc::new(move |app: &mut App, _item| sink.set(primary_focus(app)));
                pump_material(tester, m3, {
                    let button_focus = button_focus.clone();
                    move |_app| {
                        Column::new()
                            .children(vec![
                                MenuBar::new(create_test_menus_calling(on_menu_selected))
                                    .controller(controller)
                                    .into_widget(),
                                ElevatedButton::new(
                                    Some(Listener::new(|_app| {})),
                                    Some(Text::new("Press Me")),
                                )
                                .autofocus(true)
                                .focus_node(button_focus.clone())
                                .into_widget(),
                            ])
                            .into_widget()
                    }
                });

                pump(tester);
                assert_eq!(
                    primary_focus(&mut tester.cell.borrow_mut()),
                    Some(button_focus.handle())
                );

                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                tester.tap(find::text(&TestMenu::SubMenu11.label()));
                pump(tester);

                tester.tap(find::text(&TestMenu::SubSubMenu110.label()));
                pump(tester);

                assert_eq!(focus_in_on_pressed.get(), Some(button_focus.handle()));
                assert_eq!(
                    primary_focus(&mut tester.cell.borrow_mut()),
                    Some(button_focus.handle())
                );
            });
        }

        /// Dart's `void Function(TestMenu)`.
        type MenuCallback = Rc<dyn Fn(&mut App, TestMenu)>;

        /// Dart's `createTestMenus(onPressed: on_pressed)` for a callback that does more than
        /// log the item: the same menus, each enabled item calling `on_pressed`.
        fn create_test_menus_calling(on_pressed: MenuCallback) -> Vec<WidgetRef> {
            let item = |menu: TestMenu, enabled: bool, leading_icon: Option<WidgetRef>| {
                let mut button = MenuItemButton::new().child(Text::new(menu.label()));
                if enabled {
                    let on_pressed = Rc::clone(&on_pressed);
                    button = button.on_pressed(Listener::new(move |app| on_pressed(app, menu)));
                }
                button.leading_icon = leading_icon;
                button.into_widget()
            };
            let submenu = |menu: TestMenu, children: Vec<WidgetRef>| {
                SubmenuButton::new(children, Some(Text::new(menu.label()))).into_widget()
            };
            vec![
                submenu(
                    TestMenu::MainMenu0,
                    vec![
                        item(
                            TestMenu::SubMenu00,
                            true,
                            Some(Icon::new(Some(Icons::add())).into_widget()),
                        ),
                        item(TestMenu::SubMenu01, true, None),
                        item(TestMenu::SubMenu02, true, None),
                    ],
                ),
                submenu(
                    TestMenu::MainMenu1,
                    vec![
                        item(TestMenu::SubMenu10, true, None),
                        submenu(
                            TestMenu::SubMenu11,
                            vec![
                                item(TestMenu::SubSubMenu110, true, None),
                                item(TestMenu::SubSubMenu111, true, None),
                                item(TestMenu::SubSubMenu112, true, None),
                                item(TestMenu::SubSubMenu113, true, None),
                            ],
                        ),
                        item(TestMenu::SubMenu12, true, None),
                    ],
                ),
                submenu(
                    TestMenu::MainMenu2,
                    vec![item(
                        TestMenu::SubMenu20,
                        false,
                        Some(Icon::new(Some(Icons::ac_unit())).into_widget()),
                    )],
                ),
                submenu(TestMenu::MainMenu5, Vec::new()),
            ]
        }

        // menu_anchor_test.dart 'geometry'.
        #[test]
        fn a_menu_bar_and_its_menus_have_flutters_geometry() {
            test_widgets(|tester| {
                let log = Log::default();
                let menus = TestMenus::pressed(&log);
                pump_material(tester, m2, |_app| expanded_bar(&menus));
                pump(tester);

                assert_eq!(
                    rect_of::<MenuBar>(tester),
                    Rect::from_ltrb(0.0, 0.0, 800.0, 48.0)
                );

                // Open and make sure things are the right size.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                assert_eq!(
                    rect_of::<MenuBar>(tester),
                    Rect::from_ltrb(0.0, 0.0, 800.0, 48.0)
                );
                assert_eq!(
                    tester.get_rect(find::text(&TestMenu::SubMenu10.label())),
                    Rect::from_ltrb(124.0, 73.0, 314.0, 87.0)
                );
                let materials = ancestors_of_type::<Material>(tester, &TestMenu::SubMenu10.label());
                assert_eq!(
                    rect_of_element(tester, materials[1]),
                    Rect::from_ltrb(112.0, 48.0, 326.0, 208.0)
                );
            });
        }

        // menu_anchor_test.dart 'geometry with RTL direction'.
        #[test]
        fn a_right_to_left_menu_bar_and_its_menus_have_flutters_geometry() {
            test_widgets(|tester| {
                let log = Log::default();
                let menus = TestMenus::pressed(&log);
                pump_material(tester, m2, |_app| {
                    Directionality::new(TextDirection::Rtl, expanded_bar(&menus)).into_widget()
                });
                pump(tester);

                assert_eq!(
                    rect_of::<MenuBar>(tester),
                    Rect::from_ltrb(0.0, 0.0, 800.0, 48.0)
                );

                // Open and make sure things are the right size.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                assert_eq!(
                    rect_of::<MenuBar>(tester),
                    Rect::from_ltrb(0.0, 0.0, 800.0, 48.0)
                );
                assert_eq!(
                    tester.get_rect(find::text(&TestMenu::SubMenu10.label())),
                    Rect::from_ltrb(486.0, 73.0, 676.0, 87.0)
                );
                let materials = ancestors_of_type::<Material>(tester, &TestMenu::SubMenu10.label());
                assert_eq!(
                    rect_of_element(tester, materials[1]),
                    Rect::from_ltrb(474.0, 48.0, 688.0, 208.0)
                );

                // Close and make sure it goes back where it was.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                assert_eq!(
                    rect_of::<MenuBar>(tester),
                    Rect::from_ltrb(0.0, 0.0, 800.0, 48.0)
                );
            });
        }

        // menu_anchor_test.dart 'menu alignment and offset in LTR'.
        #[test]
        fn a_menu_follows_its_alignment_and_offset_left_to_right() {
            test_widgets(|tester| {
                let controller = controller(tester);
                let key = GlobalKey::new();
                build_test_app(tester, TestApp::default(), controller.clone(), &key);

                assert_eq!(
                    rect_of::<ElevatedButton>(tester),
                    Rect::from_ltrb(328.0, 14.0, 472.0, 62.0)
                );

                // Open the menu and make sure things are the right size, in the right place.
                tester.tap(find::text("Press Me"));
                pump(tester);
                assert_eq!(
                    menu_scope_rect_of_key(tester, &key),
                    Rect::from_ltrb(328.0, 62.0, 602.0, 174.0)
                );

                let with_alignment = |tester: &mut WidgetTester, alignment| {
                    let options = TestApp {
                        alignment: Some(alignment),
                        ..TestApp::default()
                    };
                    build_test_app(tester, options, controller.clone(), &key);
                    pump(tester);
                    menu_scope_rect_of_key(tester, &key)
                };
                assert_eq!(
                    with_alignment(tester, AlignmentGeometry::TOP_START),
                    Rect::from_ltrb(328.0, 14.0, 602.0, 126.0)
                );
                assert_eq!(
                    with_alignment(
                        tester,
                        AlignmentGeometry::Directional(waft_painting::AlignmentDirectional::CENTER)
                    ),
                    Rect::from_ltrb(400.0, 38.0, 674.0, 150.0)
                );
                assert_eq!(
                    with_alignment(tester, AlignmentGeometry::BOTTOM_END),
                    Rect::from_ltrb(472.0, 62.0, 746.0, 174.0)
                );

                let menu_rect = with_alignment(tester, AlignmentGeometry::TOP_START);
                let options = TestApp {
                    alignment: Some(AlignmentGeometry::TOP_START),
                    alignment_offset: Offset::new(10.0, 20.0),
                    ..TestApp::default()
                };
                build_test_app(tester, options, controller.clone(), &key);
                pump(tester);
                let offset_menu_rect = menu_scope_rect_of_key(tester, &key);
                assert_eq!(
                    offset_menu_rect.top_left() - menu_rect.top_left(),
                    Offset::new(10.0, 20.0)
                );
            });
        }

        // menu_anchor_test.dart 'menu alignment and offset in RTL'.
        #[test]
        fn a_menu_follows_its_alignment_and_offset_right_to_left() {
            test_widgets(|tester| {
                let controller = controller(tester);
                let key = GlobalKey::new();
                let rtl = TestApp {
                    text_direction: TextDirection::Rtl,
                    ..TestApp::default()
                };
                build_test_app(tester, rtl.clone(), controller.clone(), &key);

                assert_eq!(
                    rect_of::<ElevatedButton>(tester),
                    Rect::from_ltrb(328.0, 14.0, 472.0, 62.0)
                );

                let label = TestMenu::SubMenu00.label();
                // Open the menu and make sure things are the right size, in the right place.
                tester.tap(find::text("Press Me"));
                pump(tester);
                assert_eq!(
                    menu_scope_rect(tester, &label),
                    Rect::from_ltrb(198.0, 62.0, 472.0, 174.0)
                );

                let with_alignment = |tester: &mut WidgetTester, alignment| {
                    let options = TestApp {
                        alignment: Some(alignment),
                        ..rtl.clone()
                    };
                    build_test_app(tester, options, controller.clone(), &key);
                    pump(tester);
                    menu_scope_rect(tester, &label)
                };
                assert_eq!(
                    with_alignment(tester, AlignmentGeometry::TOP_START),
                    Rect::from_ltrb(198.0, 14.0, 472.0, 126.0)
                );
                assert_eq!(
                    with_alignment(
                        tester,
                        AlignmentGeometry::Directional(waft_painting::AlignmentDirectional::CENTER)
                    ),
                    Rect::from_ltrb(126.0, 38.0, 400.0, 150.0)
                );
                assert_eq!(
                    with_alignment(tester, AlignmentGeometry::BOTTOM_END),
                    Rect::from_ltrb(54.0, 62.0, 328.0, 174.0)
                );

                let menu_rect = with_alignment(tester, AlignmentGeometry::TOP_START);
                let options = TestApp {
                    alignment: Some(AlignmentGeometry::TOP_START),
                    alignment_offset: Offset::new(10.0, 20.0),
                    ..rtl.clone()
                };
                build_test_app(tester, options, controller.clone(), &key);
                pump(tester);
                assert_eq!(
                    menu_scope_rect(tester, &label).top_left() - menu_rect.top_left(),
                    Offset::new(-10.0, 20.0)
                );
            });
        }

        // menu_anchor_test.dart 'menu position in LTR'.
        #[test]
        fn a_menu_opens_at_the_position_given_left_to_right() {
            test_widgets(|tester| {
                let controller = controller(tester);
                let key = GlobalKey::new();
                let options = TestApp {
                    alignment_offset: Offset::new(100.0, 50.0),
                    ..TestApp::default()
                };
                build_test_app(tester, options, controller.clone(), &key);

                assert_eq!(
                    rect_of::<ElevatedButton>(tester),
                    Rect::from_ltrb(328.0, 14.0, 472.0, 62.0)
                );

                let label = TestMenu::SubMenu00.label();
                // Open the menu and make sure things are the right size, in the right place.
                tester.tap(find::text("Press Me"));
                pump(tester);
                assert_eq!(
                    menu_scope_rect(tester, &label),
                    Rect::from_ltrb(428.0, 112.0, 702.0, 224.0)
                );

                // Now move the menu by calling open() again with a local position on the
                // anchor.
                controller.open(
                    &mut tester.cell.borrow_mut(),
                    Some(Offset::new(200.0, 200.0)),
                );
                pump(tester);
                assert_eq!(
                    menu_scope_rect(tester, &label),
                    Rect::from_ltrb(526.0, 214.0, 800.0, 326.0)
                );
            });
        }

        // menu_anchor_test.dart 'menu position in RTL'.
        #[test]
        fn a_menu_opens_at_the_position_given_right_to_left() {
            test_widgets(|tester| {
                let controller = controller(tester);
                let key = GlobalKey::new();
                let options = TestApp {
                    alignment_offset: Offset::new(100.0, 50.0),
                    text_direction: TextDirection::Rtl,
                    ..TestApp::default()
                };
                build_test_app(tester, options, controller.clone(), &key);

                assert_eq!(
                    rect_of::<ElevatedButton>(tester),
                    Rect::from_ltrb(328.0, 14.0, 472.0, 62.0)
                );

                let label = TestMenu::SubMenu00.label();
                // Open the menu and make sure things are the right size, in the right place.
                tester.tap(find::text("Press Me"));
                pump(tester);
                assert_eq!(
                    menu_scope_rect(tester, &label),
                    Rect::from_ltrb(98.0, 112.0, 372.0, 224.0)
                );

                // Now move the menu by calling open() again with a local position on the
                // anchor.
                controller.open(
                    &mut tester.cell.borrow_mut(),
                    Some(Offset::new(400.0, 200.0)),
                );
                pump(tester);
                assert_eq!(
                    menu_scope_rect(tester, &label),
                    Rect::from_ltrb(526.0, 214.0, 800.0, 326.0)
                );
            });
        }

        /// Dart's `Padding(padding: EdgeInsets.all(10), child: MaterialApp(..))` around a padded
        /// bar of the test menus.
        fn pump_padded_bar(tester: &mut WidgetTester, menus: &TestMenus, direction: TextDirection) {
            let root = {
                let app = tester.cell.borrow();
                let bar = Column::new()
                    .children(vec![
                        Padding::new(EdgeInsetsGeometry::all(12.0))
                            .child(Row::new().children(vec![
                                Expanded::new(MenuBar::new(menus.build())).into_widget(),
                            ]))
                            .into_widget(),
                        Expanded::new(placeholder()).into_widget(),
                    ])
                    .into_widget();
                Padding::new(EdgeInsetsGeometry::all(10.0))
                    .child(material_app(
                        m2(&app),
                        Material::new()
                            .child(Directionality::new(direction, bar))
                            .into_widget(),
                    ))
                    .into_widget()
            };
            tester.pump_widget(root);
        }

        // menu_anchor_test.dart 'works with Padding around menu and overlay'.
        #[test]
        fn menus_place_themselves_with_padding_around_the_bar_and_the_overlay() {
            test_widgets(|tester| {
                let log = Log::default();
                let menus = TestMenus::pressed(&log);
                pump_padded_bar(tester, &menus, TextDirection::Ltr);
                pump(tester);

                assert_eq!(
                    rect_of::<MenuBar>(tester),
                    Rect::from_ltrb(22.0, 22.0, 778.0, 70.0)
                );

                // Open and make sure things are the right size.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                assert_eq!(
                    rect_of::<MenuBar>(tester),
                    Rect::from_ltrb(22.0, 22.0, 778.0, 70.0)
                );
                assert_eq!(
                    tester.get_rect(find::text(&TestMenu::SubMenu10.label())),
                    Rect::from_ltrb(146.0, 95.0, 336.0, 109.0)
                );
                let materials = ancestors_of_type::<Material>(tester, &TestMenu::SubMenu10.label());
                assert_eq!(
                    rect_of_element(tester, materials[1]),
                    Rect::from_ltrb(134.0, 70.0, 348.0, 230.0)
                );

                // Close and make sure it goes back where it was.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                assert_eq!(
                    rect_of::<MenuBar>(tester),
                    Rect::from_ltrb(22.0, 22.0, 778.0, 70.0)
                );
            });
        }

        // menu_anchor_test.dart 'works with Padding around menu and overlay with RTL direction'.
        #[test]
        fn right_to_left_menus_place_themselves_with_padding_around_the_bar_and_the_overlay() {
            test_widgets(|tester| {
                let log = Log::default();
                let menus = TestMenus::pressed(&log);
                pump_padded_bar(tester, &menus, TextDirection::Rtl);
                pump(tester);

                assert_eq!(
                    rect_of::<MenuBar>(tester),
                    Rect::from_ltrb(22.0, 22.0, 778.0, 70.0)
                );

                // Open and make sure things are the right size.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                assert_eq!(
                    rect_of::<MenuBar>(tester),
                    Rect::from_ltrb(22.0, 22.0, 778.0, 70.0)
                );
                assert_eq!(
                    tester.get_rect(find::text(&TestMenu::SubMenu10.label())),
                    Rect::from_ltrb(464.0, 95.0, 654.0, 109.0)
                );
                let materials = ancestors_of_type::<Material>(tester, &TestMenu::SubMenu10.label());
                assert_eq!(
                    rect_of_element(tester, materials[1]),
                    Rect::from_ltrb(452.0, 70.0, 666.0, 230.0)
                );

                // Close and make sure it goes back where it was.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                assert_eq!(
                    rect_of::<MenuBar>(tester),
                    Rect::from_ltrb(22.0, 22.0, 778.0, 70.0)
                );
            });
        }

        // menu_anchor_test.dart 'visual attributes can be set'.
        #[test]
        fn a_menu_bar_takes_the_elevation_and_background_color_of_its_style() {
            test_widgets(|tester| {
                let log = Log::default();
                let menus = TestMenus::pressed(&log);
                pump_material(tester, m3, |_app| {
                    Column::new()
                        .children(vec![
                            Row::new()
                                .children(vec![
                                    Expanded::new(
                                        MenuBar::new(menus.build()).style(
                                            MenuStyle::new()
                                                .elevation(Rc::new(
                                                    waft_widgets::WidgetStatePropertyAll::new(
                                                        Some(10.0),
                                                    ),
                                                ))
                                                .background_color(Rc::new(
                                                    waft_widgets::WidgetStatePropertyAll::new(
                                                        Some(Colors::RED.primary()),
                                                    ),
                                                )),
                                        ),
                                    )
                                    .into_widget(),
                                ])
                                .into_widget(),
                            Expanded::new(placeholder()).into_widget(),
                        ])
                        .into_widget()
                });
                assert_eq!(
                    rect_of_element(tester, only(menu_panels(tester))),
                    Rect::from_ltrb(0.0, 0.0, 800.0, 48.0)
                );
                menu_bar_material(tester, |material| {
                    assert_eq!(material.elevation, 10.0);
                    assert_eq!(material.color, Some(Colors::RED.primary()));
                });
            });
        }

        /// Dart's `MenuAnchor(menuChildren: [MenuItemButton(child: Text('Button 1'))], builder:
        /// FilledButton 'Tap me')` in a centered `Material`.
        fn clip_behavior_anchor(clip_behavior: Option<Clip>) -> WidgetRef {
            let mut anchor = MenuAnchor::new(vec![
                MenuItemButton::new()
                    .child(Text::new("Button 1"))
                    .into_widget(),
            ])
            .builder(|_app, _context, controller, _child| {
                FilledButton::new(
                    Some(Listener::new(move |app| controller.open(app, None))),
                    Some(Text::new("Tap me")),
                )
                .into_widget()
            });
            if let Some(clip_behavior) = clip_behavior {
                anchor = anchor.clip_behavior(clip_behavior);
            }
            Center::new().child(anchor).into_widget()
        }

        // menu_anchor_test.dart 'MenuAnchor clip behavior'.
        #[test]
        fn a_menu_anchor_clips_its_menu_hard_edged_unless_told_otherwise() {
            test_widgets(|tester| {
                pump_material(tester, m3, |_app| clip_behavior_anchor(None));
                tester.tap(find::text("Tap me"));
                pump(tester);

                // Test default clip behavior.
                assert_eq!(
                    menu_bar_material(tester, |material| material.clip_behavior),
                    Clip::HardEdge
                );

                // Close the menu.
                tester.tap_at(Offset::new(10.0, 10.0), 1);
                tester.pump_and_settle();
                pump_material(tester, m3, |_app| {
                    clip_behavior_anchor(Some(Clip::AntiAlias))
                });
                tester.tap(find::text("Tap me"));
                pump(tester);

                // Test custom clip behavior.
                assert_eq!(
                    menu_bar_material(tester, |material| material.clip_behavior),
                    Clip::AntiAlias
                );
            });
        }

        // menu_anchor_test.dart 'open and close works'.
        #[test]
        fn submenus_open_and_close_with_taps_on_their_buttons() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let menus = TestMenus::all(&log);
                pump_material(tester, m3, |_app| {
                    MenuBar::new(menus.build())
                        .controller(controller)
                        .into_widget()
                });

                assert!(log.opened().is_empty());
                assert!(log.closed().is_empty());

                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);
                assert_eq!(log.opened(), [TestMenu::MainMenu1]);
                assert!(log.closed().is_empty());
                log.clear();

                tester.tap(find::text(&TestMenu::SubMenu11.label()));
                pump(tester);

                assert_eq!(log.opened(), [TestMenu::SubMenu11]);
                assert!(log.closed().is_empty());
                log.clear();

                tester.tap(find::text(&TestMenu::SubMenu11.label()));
                pump(tester);

                assert!(log.opened().is_empty());
                assert_eq!(log.closed(), [TestMenu::SubMenu11]);
                log.clear();

                tester.tap(find::text(&TestMenu::MainMenu0.label()));
                pump(tester);

                assert_eq!(log.opened(), [TestMenu::MainMenu0]);
                assert_eq!(log.closed(), [TestMenu::MainMenu1]);
            });
        }

        /// `focusedMenu` against `button_label(type_name, text)`.
        #[track_caller]
        fn assert_focused(tester: &WidgetTester, type_name: &str, text: &str) {
            assert_eq!(focused_menu(tester), Some(button_label(type_name, text)));
        }

        fn submenu(tester: &WidgetTester, text: &str) {
            assert_focused(tester, "SubmenuButton", text);
        }

        fn item(tester: &WidgetTester, text: &str) {
            assert_focused(tester, "MenuItemButton", text);
        }

        // menu_anchor_test.dart 'Menus close and consume tap when open and tapped outside'.
        #[test]
        fn an_open_menu_closes_and_consumes_a_tap_outside_when_told_to() {
            test_widgets(|tester| {
                let controller = controller(tester);
                let key = GlobalKey::new();
                let log = Log::default();
                let options = TestApp {
                    consumes_outside_tap: true,
                    log: Some(log.clone()),
                    ..TestApp::default()
                };
                build_test_app(tester, options, controller.clone(), &key);

                assert!(log.opened().is_empty());
                assert!(log.closed().is_empty());

                // Doesn't consume tap when the menu is closed.
                tester.tap(find::text(&TestMenu::OutsideButton.label()));
                pump(tester);
                assert_eq!(log.selected(), [TestMenu::OutsideButton]);
                log.clear();

                tester.tap(find::text(&TestMenu::AnchorButton.label()));
                pump(tester);
                assert_eq!(log.opened(), [TestMenu::AnchorButton]);
                assert!(log.closed().is_empty());
                assert_eq!(log.selected(), [TestMenu::AnchorButton]);
                log.clear();

                tester.tap(find::text(&TestMenu::OutsideButton.label()));
                pump(tester);

                assert!(log.opened().is_empty());
                assert_eq!(log.closed(), [TestMenu::AnchorButton]);
                // When the menu is open, don't expect the outside button to be selected:
                // it's supposed to consume the key down.
                assert!(log.selected().is_empty());
            });
        }

        // menu_anchor_test.dart "Menus close and don't consume tap when open and tapped outside".
        #[test]
        fn an_open_menu_closes_and_passes_on_a_tap_outside_by_default() {
            test_widgets(|tester| {
                let controller = controller(tester);
                let key = GlobalKey::new();
                let log = Log::default();
                let options = TestApp {
                    log: Some(log.clone()),
                    ..TestApp::default()
                };
                build_test_app(tester, options, controller.clone(), &key);

                assert!(log.opened().is_empty());
                assert!(log.closed().is_empty());

                // Doesn't consume tap when the menu is closed.
                tester.tap(find::text(&TestMenu::OutsideButton.label()));
                pump(tester);
                assert_eq!(log.selected(), [TestMenu::OutsideButton]);
                log.clear();

                tester.tap(find::text(&TestMenu::AnchorButton.label()));
                pump(tester);
                assert_eq!(log.opened(), [TestMenu::AnchorButton]);
                assert!(log.closed().is_empty());
                assert_eq!(log.selected(), [TestMenu::AnchorButton]);
                log.clear();

                tester.tap(find::text(&TestMenu::OutsideButton.label()));
                pump(tester);

                assert!(log.opened().is_empty());
                assert_eq!(log.closed(), [TestMenu::AnchorButton]);
                // Because consumesOutsideTap is false, this is expected to receive its
                // tap.
                assert_eq!(log.selected(), [TestMenu::OutsideButton]);
            });
        }

        // menu_anchor_test.dart 'select works'.
        #[test]
        fn selecting_an_item_closes_every_menu() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let menus = TestMenus::all(&log);
                pump_material(tester, m3, |_app| {
                    MenuBar::new(menus.build())
                        .controller(controller)
                        .into_widget()
                });

                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                tester.tap(find::text(&TestMenu::SubMenu11.label()));
                pump(tester);

                assert_eq!(log.opened(), [TestMenu::MainMenu1, TestMenu::SubMenu11]);
                log.opened.borrow_mut().clear();
                tester.tap(find::text(&TestMenu::SubSubMenu110.label()));
                pump(tester);

                assert_eq!(log.selected(), [TestMenu::SubSubMenu110]);

                // Selecting a non-submenu item should close all the menus.
                assert!(log.opened().is_empty());
                assert!(!shows(tester, &TestMenu::SubSubMenu110.label()));
                assert!(!shows(tester, &TestMenu::SubMenu11.label()));
            });
        }

        // menu_anchor_test.dart 'menus can be traversed multiple times'.
        #[test]
        fn menus_can_be_traversed_again_after_escape_closes_them() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let menus = TestMenus::all(&log);
                pump_material(tester, m3, |_app| {
                    Column::new()
                        .children(vec![
                            MenuItemButton::new()
                                .autofocus(true)
                                .on_pressed(Listener::new(|_app| {}))
                                .child(Text::new("External Focus"))
                                .into_widget(),
                            MenuBar::new(menus.build())
                                .controller(controller)
                                .into_widget(),
                        ])
                        .into_widget()
                });

                // Have to open a menu initially to start things going.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);
                submenu(tester, "Menu 1");

                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                item(tester, "Sub Menu 10");

                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                submenu(tester, "Sub Menu 11");

                send_key(tester, LogicalKeyboardKey::ESCAPE);
                pump(tester);
                item(tester, "External Focus");

                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);
                submenu(tester, "Menu 1");

                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                item(tester, "Sub Menu 10");

                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                submenu(tester, "Sub Menu 11");
            });
        }

        // menu_anchor_test.dart 'keyboard tab traversal works'.
        #[test]
        fn tab_and_shift_tab_move_between_the_menu_bar_buttons() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let menus = TestMenus::all(&log);
                pump_material(tester, m3, |_app| {
                    Column::new()
                        .children(vec![
                            MenuBar::new(menus.build())
                                .controller(controller)
                                .into_widget(),
                            Expanded::new(placeholder()).into_widget(),
                        ])
                        .into_widget()
                });

                // Have to open a menu initially to start things going.
                tester.tap(find::text(&TestMenu::MainMenu0.label()));
                tester.pump_and_settle();

                submenu(tester, "Menu 0");
                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 1");
                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 2");
                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 0");

                tester.send_key_down_event(
                    physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                    LogicalKeyboardKey::SHIFT_LEFT,
                );
                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 2");
                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 1");
                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 0");
                tester.send_key_up_event(
                    physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                    LogicalKeyboardKey::SHIFT_LEFT,
                );
                log.clear();

                // Test closing a menu with enter.
                send_key(tester, LogicalKeyboardKey::ENTER);
                pump(tester);
                assert!(log.opened().is_empty());
                assert_eq!(log.closed(), [TestMenu::MainMenu0]);
            });
        }

        // menu_anchor_test.dart 'keyboard directional traversal works'.
        #[test]
        fn arrow_keys_move_through_a_menu_bar_and_its_menus() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let menus = TestMenus::all(&log);
                pump_material(tester, m3, |_app| {
                    MenuBar::new(menus.build())
                        .controller(controller)
                        .into_widget()
                });
                let key = |tester: &mut WidgetTester, key| send_key(tester, key);
                use LogicalKeyboardKey as K;

                // Have to open a menu initially to start things going.
                tester.tap(find::text(&TestMenu::MainMenu0.label()));
                tester.pump_and_settle();

                key(tester, K::ARROW_RIGHT);
                pump(tester);
                submenu(tester, "Menu 1");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Menu 10");

                key(tester, K::ARROW_DOWN);
                submenu(tester, "Sub Menu 11");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Menu 12");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Menu 12");

                key(tester, K::ARROW_UP);
                pump(tester);
                submenu(tester, "Sub Menu 11");

                // Open the next submenu.
                key(tester, K::ARROW_RIGHT);
                pump(tester);
                item(tester, "Sub Sub Menu 110");

                // Go back, close the submenu.
                key(tester, K::ARROW_LEFT);
                pump(tester);
                submenu(tester, "Sub Menu 11");

                // Move up, should close the submenu.
                key(tester, K::ARROW_UP);
                pump(tester);
                item(tester, "Sub Menu 10");

                // Move down, should reopen the submenu.
                key(tester, K::ARROW_DOWN);
                pump(tester);
                submenu(tester, "Sub Menu 11");

                // Open the next submenu again.
                key(tester, K::ARROW_RIGHT);
                pump(tester);
                item(tester, "Sub Sub Menu 110");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Sub Menu 111");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Sub Menu 112");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Sub Menu 113");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Sub Menu 113");

                // Since this is a leaf off of a vertical menu, moving left should
                // return to this menu's parent button.
                key(tester, K::ARROW_LEFT);
                submenu(tester, "Sub Menu 11");

                // Moving left while in a first-level submenu should focus the
                // previous top-level menubar anchor.
                key(tester, K::ARROW_LEFT);
                pump(tester);
                submenu(tester, "Menu 0");

                key(tester, K::ARROW_RIGHT);
                pump(tester);
                submenu(tester, "Menu 1");

                // Pressing arrowup from a top-level menubar anchor should focus the last
                // item in that anchor's submenu.
                key(tester, K::ARROW_UP);
                item(tester, "Sub Menu 12");

                key(tester, K::ARROW_UP);
                submenu(tester, "Sub Menu 11");
                pump(tester);

                // Enter the submenu.
                key(tester, K::ARROW_RIGHT);
                item(tester, "Sub Sub Menu 110");

                // Move to next top-level menu button.
                key(tester, K::ARROW_RIGHT);
                pump(tester);
                submenu(tester, "Menu 2");
            });
        }

        // menu_anchor_test.dart 'keyboard directional traversal works in RTL mode'.
        #[test]
        fn arrow_keys_move_through_a_right_to_left_menu_bar_and_its_menus() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let menus = TestMenus::all(&log);
                let root = {
                    let app = tester.cell.borrow();
                    material_app(
                        m3(&app),
                        Directionality::new(
                            TextDirection::Rtl,
                            Material::new()
                                .child(MenuBar::new(menus.build()).controller(controller)),
                        )
                        .into_widget(),
                    )
                };
                tester.pump_widget(root);
                let key = |tester: &mut WidgetTester, key| send_key(tester, key);
                use LogicalKeyboardKey as K;

                // Have to open a menu initially to start things going.
                tester.tap(find::text(&TestMenu::MainMenu0.label()));
                pump(tester);

                key(tester, K::ARROW_LEFT);
                submenu(tester, "Menu 1");

                key(tester, K::ARROW_DOWN);
                pump(tester);
                submenu(tester, "Menu 1");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Menu 10");

                key(tester, K::ARROW_DOWN);
                submenu(tester, "Sub Menu 11");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Menu 12");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Menu 12");

                key(tester, K::ARROW_UP);
                pump(tester);
                submenu(tester, "Sub Menu 11");

                // Open the next submenu.
                key(tester, K::ARROW_LEFT);
                pump(tester);
                item(tester, "Sub Sub Menu 110");

                // Go back, close the submenu.
                key(tester, K::ARROW_RIGHT);
                pump(tester);
                submenu(tester, "Sub Menu 11");

                // Move up, should close the submenu.
                key(tester, K::ARROW_UP);
                pump(tester);
                item(tester, "Sub Menu 10");

                // Move down, should reopen the submenu.
                key(tester, K::ARROW_DOWN);
                pump(tester);
                submenu(tester, "Sub Menu 11");

                // Open the next submenu again.
                key(tester, K::ARROW_LEFT);
                pump(tester);
                item(tester, "Sub Sub Menu 110");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Sub Menu 111");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Sub Menu 112");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Sub Menu 113");

                key(tester, K::ARROW_DOWN);
                item(tester, "Sub Sub Menu 113");

                // Since this is a leaf off of a vertical menu, moving right should
                // return to this menu's parent button.
                key(tester, K::ARROW_RIGHT);
                submenu(tester, "Sub Menu 11");

                // Moving left while in a first-level submenu should focus the
                // previous top-level menubar anchor.
                key(tester, K::ARROW_RIGHT);
                pump(tester);
                submenu(tester, "Menu 0");

                key(tester, K::ARROW_LEFT);
                pump(tester);
                submenu(tester, "Menu 1");

                // Pressing arrowup from a top-level menubar anchor should focus the last
                // item in that anchor's submenu.
                key(tester, K::ARROW_UP);
                item(tester, "Sub Menu 12");

                key(tester, K::ARROW_UP);
                pump(tester);
                submenu(tester, "Sub Menu 11");

                // Enter the submenu.
                key(tester, K::ARROW_LEFT);
                item(tester, "Sub Sub Menu 110");

                // Move to next top-level menu button.
                key(tester, K::ARROW_LEFT);
                pump(tester);
                submenu(tester, "Menu 2");
            });
        }

        /// Dart's `MenuAnchor(childFocusNode: buttonFocusNode, menuChildren: [MenuItemButton('start'),
        /// ...createTestMenus(..)], builder: TextButton(label))` in a `Column`, under
        /// `direction`; answers the button's focus node, labeled 'Press Me'.
        fn pump_anchor_with_test_menus(
            tester: &mut WidgetTester,
            log: &Log,
            label: &'static str,
            direction: TextDirection,
        ) -> AnyFocusNode {
            let button_focus_node = {
                let mut app = tester.cell.borrow_mut();
                let node = FocusNode::new(&mut app).into_erased::<AnyFocusNode>();
                node.set_debug_label(&mut app, Some(TestMenu::AnchorButton.label()));
                node
            };
            let menus = TestMenus::all(log);
            let mut menu_children = vec![
                MenuItemButton::new()
                    .on_pressed(Listener::new(|_app| {}))
                    .child(Text::new("start"))
                    .into_widget(),
            ];
            menu_children.extend(menus.build());
            let anchor = MenuAnchor::new(menu_children)
                .child_focus_node(button_focus_node.clone())
                .builder({
                    let button_focus_node = button_focus_node.clone();
                    move |_app, _context, controller, _child| {
                        TextButton::new(
                            Some(Listener::new(move |app| {
                                if controller.is_open(app) {
                                    controller.close(app);
                                } else {
                                    controller.open(app, None);
                                }
                            })),
                            Some(Text::new(label)),
                        )
                        .focus_node(button_focus_node.clone())
                        .into_widget()
                    }
                });
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    m3(&app),
                    Directionality::new(
                        direction,
                        Material::new().child(Column::new().children(vec![anchor.into_widget()])),
                    )
                    .into_widget(),
                )
            };
            tester.pump_widget(root);
            button_focus_node.handle()
        }

        fn focused_is_anchor_button(tester: &WidgetTester) -> bool {
            focused_menu(tester) == Some(TestMenu::AnchorButton.label())
        }

        // menu_anchor_test.dart 'MenuAnchor tab traversal works'.
        #[test]
        fn tab_and_shift_tab_move_between_the_items_of_a_menu_anchor() {
            test_widgets(|tester| {
                let log = Log::default();
                pump_anchor_with_test_menus(tester, &log, "Press Me", TextDirection::Ltr);

                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                assert!(focused_is_anchor_button(tester));

                send_key(tester, LogicalKeyboardKey::ENTER);
                pump(tester);
                assert!(focused_is_anchor_button(tester));

                // Directional traversal doesn't work until a menu item is focused.
                // To start focusing, hover over the first menu item.
                let start = tester.get_rect(find::text("start")).center();
                hover_over(tester, start);
                pump(tester);
                item(tester, "start");

                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 0");

                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 1");

                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 2");

                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                item(tester, "start");

                tester.send_key_down_event(
                    physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                    LogicalKeyboardKey::SHIFT_LEFT,
                );
                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 2");

                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 1");

                send_key(tester, LogicalKeyboardKey::TAB);
                pump(tester);
                submenu(tester, "Menu 0");

                tester.send_key_up_event(
                    physical_of(LogicalKeyboardKey::SHIFT_LEFT),
                    LogicalKeyboardKey::SHIFT_LEFT,
                );
                log.clear();

                // Test closing a menu with enter.
                send_key(tester, LogicalKeyboardKey::ENTER);
                pump(tester);
                assert!(log.opened().is_empty());
                assert_eq!(log.closed(), [TestMenu::MainMenu0]);
            });
        }

        /// The steps the two 'MenuAnchor * directional traversal works' cases share, with `into`
        /// the arrow that enters a submenu and `out` the one that leaves it.
        fn anchor_directional_traversal(tester: &mut WidgetTester, direction: TextDirection) {
            use LogicalKeyboardKey as K;
            let (into, out) = match direction {
                TextDirection::Ltr => (K::ARROW_RIGHT, K::ARROW_LEFT),
                TextDirection::Rtl => (K::ARROW_LEFT, K::ARROW_RIGHT),
            };
            let log = Log::default();
            pump_anchor_with_test_menus(tester, &log, "Open", direction);

            send_key(tester, K::TAB);
            assert!(focused_is_anchor_button(tester));

            send_key(tester, K::ENTER);
            pump(tester);
            assert!(focused_is_anchor_button(tester));
            assert!(shows(tester, "start"));

            // Directional traversal doesn't work until a menu item is focused.
            // To start focusing, hover over the first menu item.
            let start = tester.get_rect(find::text("start")).center();
            hover_over(tester, start);
            pump(tester);
            item(tester, "start");

            send_key(tester, K::ARROW_DOWN);
            pump(tester);
            submenu(tester, "Menu 0");
            assert!(shows(tester, "Sub Menu 00"));

            send_key(tester, out);
            pump(tester);
            submenu(tester, "Menu 0");

            send_key(tester, into);
            pump(tester);
            item(tester, "Sub Menu 00");

            send_key(tester, K::ARROW_DOWN);
            item(tester, "Sub Menu 01");

            send_key(tester, K::ARROW_DOWN);
            item(tester, "Sub Menu 02");

            // We're at the deepest menu, so the arrow into a submenu should not change focus.
            send_key(tester, into);
            item(tester, "Sub Menu 02");

            // The arrow out of a submenu should move focus to the parent anchor.
            send_key(tester, out);
            pump(tester);
            submenu(tester, "Menu 0");
            assert!(!shows(tester, "Sub Menu 00"));

            // We're at the root menu, so the arrow out should not change focus and
            // should not open the submenu.
            send_key(tester, out);
            submenu(tester, "Menu 0");
            assert!(!shows(tester, "Sub Menu 00"));

            // Open the submenu again.
            send_key(tester, K::SPACE);
            pump(tester);
            submenu(tester, "Menu 0");
            assert!(shows(tester, "Sub Menu 00"));

            // Close all menus.
            send_key(tester, K::ESCAPE);
            pump(tester);
            assert!(focused_is_anchor_button(tester));
            expect(tester, &find::by_type::<MenuItemButton>(), finds_nothing());
        }

        // menu_anchor_test.dart 'MenuAnchor LTR directional traversal works'.
        #[test]
        fn arrow_keys_move_through_a_menu_anchor_left_to_right() {
            test_widgets(|tester| anchor_directional_traversal(tester, TextDirection::Ltr));
        }

        // menu_anchor_test.dart 'MenuAnchor RTL directional traversal works'.
        #[test]
        fn arrow_keys_move_through_a_menu_anchor_right_to_left() {
            test_widgets(|tester| anchor_directional_traversal(tester, TextDirection::Rtl));
        }

        /// Dart's `listenForFocusChanges()`: the `focusedMenu` a `FocusManager` listener keeps,
        /// `None` until focus first changes.
        fn listen_for_focus_changes(tester: &WidgetTester) -> Rc<RefCell<Option<String>>> {
            let focused_menu: Rc<RefCell<Option<String>>> = Rc::default();
            let sink = Rc::clone(&focused_menu);
            let mut app = tester.cell.borrow_mut();
            let manager = FocusManager::instance(&mut app);
            waft_foundation::ListenableObject::add_listener(
                manager,
                &mut app,
                Listener::new(move |app| {
                    let label = primary_focus(app)
                        .map(|node| node.debug_label(app).unwrap_or_else(|| format!("{node:?}")));
                    *sink.borrow_mut() = label;
                }),
            );
            focused_menu
        }

        /// One mouse that Dart's successive `hoverOver` calls move around.
        struct Mouse(TestPointer);

        impl Mouse {
            fn new(tester: &mut WidgetTester) -> Mouse {
                let mut mouse = TestPointer::mouse(1, 0);
                tester.add_pointer(&mut mouse, Offset::ZERO);
                Mouse(mouse)
            }

            /// Dart's `hoverOver(tester, find.text(text))`, then the case's `tester.pump()`.
            fn hover_text(&mut self, tester: &mut WidgetTester, text: &str) {
                let at = tester.get_rect(find::text(text)).center();
                tester.move_to(&mut self.0, at, false);
                pump(tester);
            }

            /// Dart's `pointer.scroll(delta)`: a scroll signal where the mouse is.
            fn scroll(&self, tester: &mut WidgetTester, delta: Offset) {
                use waft_embedder::{
                    PointerChange, PointerData, PointerDataPacket, PointerSignalKind,
                };
                let mut app = tester.cell.borrow_mut();
                waft_gestures::GestureBinding::instance(&mut app).handle_pointer_data_packet(
                    &mut app,
                    PointerDataPacket::new(vec![PointerData {
                        change: PointerChange::Hover,
                        kind: self.0.kind,
                        device: self.0.id,
                        pointer_identifier: self.0.id,
                        signal_kind: Some(PointerSignalKind::Scroll),
                        physical_x: self.0.position.dx() * 2.0,
                        physical_y: self.0.position.dy() * 2.0,
                        scroll_delta_x: delta.dx() * 2.0,
                        scroll_delta_y: delta.dy() * 2.0,
                        ..PointerData::default()
                    }]),
                );
                app.drain_microtasks();
                drop(app);
                tester.cell.checkpoint();
            }
        }

        // menu_anchor_test.dart 'hover traversal works'.
        #[test]
        fn hovering_opens_and_focuses_menus_once_one_is_open() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let menus = TestMenus::all(&log);
                pump_material(tester, m3, |_app| {
                    MenuBar::new(menus.build())
                        .controller(controller)
                        .into_widget()
                });

                let focused_menu = listen_for_focus_changes(tester);
                let focused = |text: &str, type_name: &str| {
                    assert_eq!(*focused_menu.borrow(), Some(button_label(type_name, text)));
                };
                let mut mouse = Mouse::new(tester);

                // Hovering when the menu is not yet open does nothing.
                mouse.hover_text(tester, &TestMenu::MainMenu0.label());
                pump(tester);
                assert_eq!(*focused_menu.borrow(), None);

                // Have to open a menu initially to start things going.
                tester.tap(find::text(&TestMenu::MainMenu0.label()));
                pump(tester);
                focused("Menu 0", "SubmenuButton");

                // Hovering when the menu is already  open does nothing.
                mouse.hover_text(tester, &TestMenu::MainMenu0.label());
                pump(tester);
                focused("Menu 0", "SubmenuButton");

                // Hovering over the other main menu items opens them now.
                mouse.hover_text(tester, &TestMenu::MainMenu2.label());
                pump(tester);
                focused("Menu 2", "SubmenuButton");

                mouse.hover_text(tester, &TestMenu::MainMenu1.label());
                pump(tester);
                focused("Menu 1", "SubmenuButton");

                // Hovering over the menu items focuses them.
                mouse.hover_text(tester, &TestMenu::SubMenu10.label());
                pump(tester);
                focused("Sub Menu 10", "MenuItemButton");

                mouse.hover_text(tester, &TestMenu::SubMenu11.label());
                pump(tester);
                focused("Sub Menu 11", "SubmenuButton");

                mouse.hover_text(tester, &TestMenu::SubSubMenu110.label());
                pump(tester);
                focused("Sub Sub Menu 110", "MenuItemButton");
            });
        }

        // menu_anchor_test.dart 'hover traversal invalidates directional focus scope data'.
        #[test]
        fn hovering_resets_where_directional_traversal_starts() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let menus = TestMenus::all(&log);
                pump_material(tester, m3, |_app| {
                    MenuBar::new(menus.build())
                        .controller(controller)
                        .into_widget()
                });
                let mut mouse = Mouse::new(tester);

                // Have to open a menu initially to start things going.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);
                submenu(tester, "Menu 1");

                mouse.hover_text(tester, &TestMenu::SubMenu12.label());
                pump(tester);
                item(tester, "Sub Menu 12");

                // Move pointer to disabled menu.
                mouse.hover_text(tester, &TestMenu::MainMenu5.label());
                pump(tester);
                item(tester, "Sub Menu 12");

                send_key(tester, LogicalKeyboardKey::ARROW_UP);
                pump(tester);
                submenu(tester, "Sub Menu 11");

                send_key(tester, LogicalKeyboardKey::ARROW_UP);
                item(tester, "Sub Menu 10");

                mouse.hover_text(tester, &TestMenu::SubMenu12.label());
                pump(tester);
                item(tester, "Sub Menu 12");

                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                item(tester, "Sub Menu 12");
            });
        }

        /// Dart's `SubmenuButton(hoverOpenDelay: 500ms, menuChildren: [Item 0], child: 'Submenu')`,
        /// under a `Text('Outside')` when `outside` is set.
        fn pump_delayed_submenu(tester: &mut WidgetTester, outside: bool) {
            pump_material(tester, m3, move |_app| {
                let submenu = SubmenuButton::new(
                    vec![
                        MenuItemButton::new()
                            .on_pressed(Listener::new(|_app| {}))
                            .child(Text::new("Item 0"))
                            .into_widget(),
                    ],
                    Some(Text::new("Submenu")),
                )
                .hover_open_delay(Duration::from_millis(500))
                .into_widget();
                if outside {
                    Column::new()
                        .children(vec![Text::new("Outside").into_widget(), submenu])
                        .into_widget()
                } else {
                    submenu
                }
            });
        }

        // menu_anchor_test.dart 'hoverOpenDelay delays when a SubmenuButton opens'.
        #[test]
        fn a_hover_open_delay_holds_back_the_submenu() {
            test_widgets(|tester| {
                pump_delayed_submenu(tester, false);
                let mut mouse = Mouse::new(tester);

                // Hover over the submenu button
                mouse.hover_text(tester, "Submenu");
                tester.pump_by(Duration::from_millis(499));

                assert!(!shows(tester, "Item 0"));

                tester.pump_by(Duration::from_millis(1));

                // Menu should now be open
                assert!(shows(tester, "Item 0"));
            });
        }

        // menu_anchor_test.dart 'hoverOpenDelay can be cancelled'.
        #[test]
        fn leaving_the_button_cancels_a_hover_open_delay() {
            test_widgets(|tester| {
                pump_delayed_submenu(tester, true);
                let mut mouse = Mouse::new(tester);

                // Hover over the submenu button
                mouse.hover_text(tester, "Submenu");
                tester.pump_by(Duration::from_millis(499));

                assert!(!shows(tester, "Item 0"));

                // Move hover away before the delay completes
                mouse.hover_text(tester, "Outside");
                tester.pump_by(Duration::from_millis(1));

                assert!(!shows(tester, "Item 0"));
            });
        }

        // menu_anchor_test.dart 'hoverOpenDelay restarts after cancellation'.
        #[test]
        fn a_hover_open_delay_starts_over_after_it_is_cancelled() {
            test_widgets(|tester| {
                pump_delayed_submenu(tester, true);
                let mut mouse = Mouse::new(tester);

                mouse.hover_text(tester, "Submenu");
                tester.pump_by(Duration::from_millis(499));

                assert!(!shows(tester, "Item 0"));

                mouse.hover_text(tester, "Outside");
                tester.pump_by(Duration::from_millis(1));

                assert!(!shows(tester, "Item 0"));

                mouse.hover_text(tester, "Submenu");
                tester.pump_by(Duration::from_millis(500));

                assert!(shows(tester, "Item 0"));
            });
        }

        // menu_anchor_test.dart 'Throws if non-zero hoverOpenDelay is applied to MenuBar items'.
        #[test]
        #[should_panic(
            expected = "A non-zero hoverOpenDelay was used in a top-level SubmenuButton situated in a MenuBar"
        )]
        fn a_hover_open_delay_on_a_menu_bar_button_panics_in_a_debug_build() {
            test_widgets(|tester| {
                pump_material(tester, m3, |_app| {
                    MenuBar::new(vec![
                        SubmenuButton::new(
                            vec![
                                MenuItemButton::new()
                                    .on_pressed(Listener::new(|_app| {}))
                                    .child(Text::new("Item 0"))
                                    .into_widget(),
                            ],
                            Some(Text::new("Submenu")),
                        )
                        .hover_open_delay(Duration::from_millis(1))
                        .into_widget(),
                    ])
                    .into_widget()
                });
            });
        }

        // menu_anchor_test.dart 'scrolling does not trigger hover traversal'.
        #[test]
        fn scrolling_a_menu_under_the_pointer_does_not_move_focus() {
            test_widgets(|tester| {
                let controller = controller(tester);
                let scrolled_menu_item_key = GlobalKey::new();
                let key = scrolled_menu_item_key.clone();
                pump_material(tester, m3, {
                    let controller = controller.clone();
                    move |_app| {
                        let menu_children = (0..20)
                            .map(|i| {
                                let mut button = MenuItemButton::new()
                                    .on_pressed(Listener::new(|_app| {}))
                                    .child(Text::new(format!("Item {i}")));
                                if i == 15 {
                                    button = button.key(key.clone().into());
                                }
                                button.into_widget()
                            })
                            .collect();
                        MenuAnchor::new(menu_children)
                            .style(MenuStyle::new().fixed_size(Rc::new(
                                waft_widgets::WidgetStatePropertyAll::new(Some(Size::from_height(
                                    200.0,
                                ))),
                            )))
                            .controller(controller.clone())
                            .into_widget()
                    }
                });

                let focused_menu = listen_for_focus_changes(tester);
                let focused = |text: &str| {
                    assert_eq!(
                        *focused_menu.borrow(),
                        Some(button_label("MenuItemButton", text))
                    );
                };

                controller.open(&mut tester.cell.borrow_mut(), None);
                tester.pump_and_settle();

                let mut mouse = Mouse::new(tester);
                mouse.hover_text(tester, "Item 1");
                pump(tester);
                focused("Item 1");

                // Scroll the menu while the pointer is over a menu item. The focus should
                // not change.
                let item_15 = tester.element(find::text("Item 15"));
                {
                    let mut app = tester.cell.borrow_mut();
                    let render = item_15.find_render_object(&app).expect("laid out");
                    render.show_on_screen(
                        &mut app,
                        None,
                        None,
                        Duration::ZERO,
                        waft_animation::Curves::ease(),
                    );
                }
                tester.pump_and_settle();
                focused("Item 1");

                // Traverse with the keyboard to test that the menu scrolls without hover
                // focus affecting the focused menu.
                for i in 2..20 {
                    send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                    pump(tester);
                    focused(&format!("Item {i}"));
                }
            });
        }

        /// Dart's `SingleChildScrollView(controller: scroll_controller, child: Container(height:
        /// 1000, alignment: alignment, child: child))`.
        fn tall_scroll_view(
            scroll_controller: waft_widgets::ScrollControllerRef,
            alignment: AlignmentGeometry,
            child: WidgetRef,
        ) -> WidgetRef {
            SingleChildScrollView::new()
                .controller(scroll_controller)
                .child(
                    Container::new()
                        .height(1000.0)
                        .alignment(alignment)
                        .child(child),
                )
                .into_widget()
        }

        // menu_anchor_test.dart 'menus close on ancestor scroll'.
        #[test]
        fn menus_close_when_an_ancestor_scrolls() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let scroll_controller = ScrollController::default(&mut tester.cell.borrow_mut())
                    .into_erased::<waft_widgets::AnyScrollController>();
                let menus = TestMenus::all(&log);
                pump_material(tester, m3, |_app| {
                    tall_scroll_view(
                        scroll_controller.clone(),
                        AlignmentGeometry::CENTER,
                        MenuBar::new(menus.build())
                            .controller(controller)
                            .into_widget(),
                    )
                });

                tester.tap(find::text(&TestMenu::MainMenu0.label()));
                pump(tester);

                assert!(!log.opened().is_empty());
                assert!(log.closed().is_empty());
                log.opened.borrow_mut().clear();

                scroll_controller.jump_to(&mut tester.cell.borrow_mut(), 1000.0);
                pump(tester);

                assert!(log.opened().is_empty());
                assert!(!log.closed().is_empty());
            });
        }

        // menu_anchor_test.dart 'menus do not close on root menu internal scroll'.
        #[test]
        fn menus_stay_open_when_the_root_menu_scrolls_itself() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let scroll_controller = ScrollController::default(&mut tester.cell.borrow_mut())
                    .into_erased::<waft_widgets::AnyScrollController>();
                let root_opened = Rc::new(Cell::new(false));
                let (opened, closed) = (Rc::clone(&root_opened), Rc::clone(&root_opened));
                let menus = TestMenus {
                    include_extra_groups: true,
                    ..TestMenus::all(&log)
                };
                let root = {
                    let app = tester.cell.borrow();
                    let theme = ThemeData::builder()
                        .menu_button_theme(
                            crate::menu_button_theme::MenuButtonThemeData::new().style(
                                // Increase menu items height to make root menu scrollable.
                                TextButton::style_from()
                                    .minimum_size(Size::from_height(200.0))
                                    .build(),
                            ),
                        )
                        .build(&app);
                    let anchor = MenuAnchor::new(menus.build())
                        .controller(controller)
                        .alignment_offset(Offset::new(0.0, 10.0))
                        .builder(|_app, _context, controller, _child| {
                            FilledButton::tonal(
                                Some(Listener::new(move |app| {
                                    if controller.is_open(app) {
                                        controller.close(app);
                                    } else {
                                        controller.open(app, None);
                                    }
                                })),
                                Some(Text::new("Show menu")),
                            )
                            .into_widget()
                        })
                        .on_open(Rc::new(move |_app: &mut App| opened.set(true)))
                        .on_close(Rc::new(move |_app: &mut App| closed.set(false)));
                    material_app(
                        theme,
                        Material::new()
                            .child(tall_scroll_view(
                                scroll_controller.clone(),
                                AlignmentGeometry::TOP_LEFT,
                                anchor.into_widget(),
                            ))
                            .into_widget(),
                    )
                };
                tester.pump_widget(root);

                tester.tap(find::text("Show menu"));
                pump(tester);
                assert!(root_opened.get());

                // Hover the first item.
                let mut mouse = Mouse::new(tester);
                mouse.hover_text(tester, &TestMenu::MainMenu0.label());
                assert!(!log.opened().is_empty());

                // Menus do not close on internal scroll.
                mouse.scroll(tester, Offset::new(0.0, 30.0));
                pump(tester);
                assert!(root_opened.get());
                assert!(log.closed().is_empty());

                // Menus close on external scroll.
                scroll_controller.jump_to(&mut tester.cell.borrow_mut(), 1000.0);
                pump(tester);
                assert!(!root_opened.get());
                assert!(!log.closed().is_empty());
            });
        }

        // menu_anchor_test.dart 'menus close on view size change'.
        #[test]
        fn menus_close_when_the_view_size_changes() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = controller(tester);
                let scroll_controller = ScrollController::default(&mut tester.cell.borrow_mut())
                    .into_erased::<waft_widgets::AnyScrollController>();
                let build = |tester: &mut WidgetTester, size: Option<Size>| {
                    let menus = TestMenus::all(&log);
                    let (scroll_controller, controller) =
                        (scroll_controller.clone(), controller.clone());
                    pump_material(tester, m3, move |_app| {
                        waft_widgets::Builder::new(move |app, context| {
                            let view = waft_widgets::View::of(app, context);
                            let mut data = MediaQueryData::from_view(app, &*view, None);
                            if let Some(size) = size {
                                data = data.size(size);
                            }
                            MediaQuery::new(
                                data,
                                tall_scroll_view(
                                    scroll_controller.clone(),
                                    AlignmentGeometry::CENTER,
                                    MenuBar::new(menus.build())
                                        .controller(controller.clone())
                                        .into_widget(),
                                ),
                            )
                            .into_widget()
                        })
                        .into_widget()
                    });
                };

                build(tester, None);

                tester.tap(find::text(&TestMenu::MainMenu0.label()));
                pump(tester);

                assert!(!log.opened().is_empty());
                assert!(log.closed().is_empty());
                log.opened.borrow_mut().clear();

                let small_size = Size::new(200.0, 200.0);
                build(tester, Some(small_size));
                pump(tester);

                assert!(log.opened().is_empty());
                assert!(!log.closed().is_empty());
            });
        }

        // menu_anchor_test.dart 'Shortcuts of MenuAnchor do not rely on WidgetsApp.shortcuts'.
        #[test]
        fn a_menu_anchor_traverses_with_arrow_keys_without_the_apps_shortcuts() {
            test_widgets(|tester| {
                let child_node = {
                    let mut app = tester.cell.borrow_mut();
                    let node = FocusNode::new(&mut app).into_erased::<AnyFocusNode>();
                    node.set_debug_label(&mut app, Some("Dropdown Inkwell".to_string()));
                    node
                };
                let root = {
                    let app = tester.cell.borrow();
                    let menu_children = (0..3)
                        .map(|i| {
                            MenuItemButton::new()
                                .child(Text::new(format!("Submenu item {i}")))
                                .on_pressed(Listener::new(|_app| {}))
                                .into_widget()
                        })
                        .collect();
                    let anchor = MenuAnchor::new(menu_children)
                        .child_focus_node(child_node.clone())
                        .builder(move |_app, _context, controller, _child| {
                            crate::ink_well::InkWell::new()
                                .focus_node(child_node.clone())
                                .on_tap(Listener::new(move |app| controller.open(app, None)))
                                .child(Text::new("Main button"))
                                .into_widget()
                        });
                    crate::app::MaterialApp::new()
                        // Clear WidgetsApp.shortcuts to make sure MenuAnchor doesn't rely on
                        // it.
                        .shortcuts(Vec::new())
                        .theme(m3(&app))
                        .home(crate::scaffold::Scaffold::new().body(anchor))
                        .into_widget()
                };
                tester.pump_widget(root);

                // Open the drop down menu and focus on the MenuAnchor.
                tester.tap(find::text("Main button"));
                tester.pump_and_settle();
                assert!(shows(tester, "Submenu item 0"));

                // Press arrowDown, and the first submenu button should be focused.
                // This is the critical part. It used to not work on Web.
                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                pump(tester);
                item(tester, "Submenu item 0");

                // Press arrowDown, and the second submenu button should be focused.
                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                pump(tester);
                item(tester, "Submenu item 1");
            });
        }

        /// Dart's `MaterialApp(theme: ThemeData(scrollbarTheme:), home: Material(child: MenuBar(
        /// [SubmenuButton('Main Menu', [MenuItemButton('Category') 1000 tall])])))`, the bar under
        /// a `ScrollbarTheme` of `local_theme` when one is given.
        fn pump_tall_menu(
            tester: &mut WidgetTester,
            scrollbar_theme: crate::scrollbar_theme::ScrollbarThemeData,
            local_theme: Option<crate::scrollbar_theme::ScrollbarThemeData>,
        ) {
            let root = {
                let app = tester.cell.borrow();
                let bar = MenuBar::new(vec![
                    SubmenuButton::new(
                        vec![
                            MenuItemButton::new()
                                .style(ButtonStyle::new().minimum_size(Rc::new(
                                    waft_widgets::WidgetStatePropertyAll::new(Some(
                                        Size::from_height(1000.0),
                                    )),
                                )))
                                .on_pressed(Listener::new(|_app| {}))
                                .child(Text::new("Category"))
                                .into_widget(),
                        ],
                        Some(Text::new("Main Menu")),
                    )
                    .into_widget(),
                ])
                .into_widget();
                let home = match local_theme {
                    Some(local_theme) => {
                        crate::scrollbar_theme::ScrollbarTheme::new(local_theme, bar).into_widget()
                    }
                    None => bar,
                };
                material_app(
                    ThemeData::builder()
                        .scrollbar_theme(scrollbar_theme)
                        .build(&app),
                    Material::new().child(home).into_widget(),
                )
            };
            tester.pump_widget(root);
        }

        /// Dart's `expect(find.byType(Scrollbar).last, paints..rrect(color: color))`: what the
        /// foreground painter under the last `Scrollbar` paints, painted on its own.
        fn last_scrollbar_paints_a_path_in(tester: &WidgetTester, color: u32) -> bool {
            let scrollbar = tester.element(find::by_type::<crate::scrollbar::Scrollbar>().last());
            let mut app = tester.cell.borrow_mut();
            let render = scrollbar.find_render_object(&app).expect("laid out");
            let mut pending = vec![render];
            let mut painter = None;
            while let Some(object) = pending.pop() {
                if let Some(custom_paint) =
                    object.downcast::<waft_rendering::RenderCustomPaint>(&app)
                    && let Some(foreground) = custom_paint.foreground_painter(&app)
                {
                    painter = Some((
                        foreground,
                        waft_rendering::RenderBox::size(custom_paint, &app),
                    ));
                    break;
                }
                object.visit_children(&app, &mut |child| pending.push(child));
            }
            let (painter, size) = painter.expect("a foreground painter under the Scrollbar");
            let mut canvas = waft_embedder::Canvas::new();
            painter.paint(&mut app, &mut canvas, size);
            paints(
                canvas.build().ops(),
                &[PaintCall::rrect(Some(Color::new(color)))],
            )
            .is_ok()
        }

        // menu_anchor_test.dart 'Menu scrollbar inherits ScrollbarTheme'
        // (TargetPlatformVariant.desktop()).
        #[test]
        fn a_menus_scrollbar_takes_the_ambient_scrollbar_theme() {
            for platform in [
                TargetPlatform::Linux,
                TargetPlatform::MacOS,
                TargetPlatform::Windows,
            ] {
                test_widgets(|_| {
                    let mut tester = WidgetTester::on(platform);
                    let tester = &mut tester;
                    let scrollbar_theme = crate::scrollbar_theme::ScrollbarThemeData::new()
                        .thumb_color(Rc::new(waft_widgets::WidgetStatePropertyAll::new(Some(
                            Color::new(0xffff0000),
                        ))))
                        .thumb_visibility(Rc::new(waft_widgets::WidgetStatePropertyAll::new(
                            Some(true),
                        )));
                    pump_tall_menu(tester, scrollbar_theme.clone(), None);

                    tester.tap(find::text("Main Menu"));
                    tester.pump_and_settle();

                    // Test Scrollbar thumb color.
                    assert!(last_scrollbar_paints_a_path_in(tester, 0xffff0000));

                    // Close the menu.
                    tester.tap_at(Offset::new(10.0, 10.0), 1);
                    tester.pump_and_settle();

                    let local_theme = scrollbar_theme.copy_with().thumb_color(Rc::new(
                        waft_widgets::WidgetStatePropertyAll::new(Some(Color::new(0xff00ff00))),
                    ));
                    pump_tall_menu(tester, scrollbar_theme, Some(local_theme));

                    tester.tap(find::text("Main Menu"));
                    tester.pump_and_settle();

                    // Scrollbar thumb color should be updated.
                    assert!(last_scrollbar_paints_a_path_in(tester, 0xff00ff00));
                });
            }
        }

        // menu_anchor_test.dart 'Menu scrollbar does not inherit MediaQuery padding' (material_ui).
        #[test]
        fn a_menus_scrollbar_does_not_inherit_the_media_query_padding() {
            test_widgets(|tester| {
                use waft_rendering::testing::RecordedCall;
                use waft_widgets::{Scrollable, ScrollableState, ScrollbarPainterRef};

                use crate::scrollbar::Scrollbar;
                use crate::text_button::TextButton;

                tester.view().set_device_pixel_ratio(1.0);
                tester.view().set_padding(waft_embedder::ViewPadding {
                    left: 8.0,
                    top: 16.0,
                    right: 32.0,
                    bottom: 24.0,
                });
                let menu_item_padding: Rc<Cell<Option<EdgeInsets>>> = Rc::default();
                let controller: MenuControllerRef =
                    MenuController::new(&mut tester.cell.borrow_mut()).into_erased();

                let mut menu_children = vec![
                    waft_widgets::Builder::new({
                        let menu_item_padding = Rc::clone(&menu_item_padding);
                        move |app, context| {
                            menu_item_padding.set(Some(MediaQuery::padding_of(app, context)));
                            MenuItemButton::new()
                                .on_pressed(Listener::new(|_app| {}))
                                .child(Text::new("Item 0"))
                                .into_widget()
                        }
                    })
                    .into_widget(),
                ];
                menu_children.extend((1..4).map(|i| {
                    MenuItemButton::new()
                        .on_pressed(Listener::new(|_app| {}))
                        .child(Text::new(format!("Item {i}")))
                        .into_widget()
                }));
                let anchor = MenuAnchor::new(menu_children)
                    .controller(controller)
                    .style(MenuStyle::new().maximum_size(Rc::new(
                        waft_widgets::WidgetStatePropertyAll::new(Some(Size::from_height(100.0))),
                    )))
                    .builder(|_app, _context, controller, _child| {
                        TextButton::new(
                            Some(Listener::new(move |app| controller.open(app, None))),
                            Some(Text::new("Open menu")),
                        )
                        .into_widget()
                    });
                tester.pump_widget(
                    crate::app::MaterialApp::new()
                        .home(Material::new().child(anchor))
                        .into_widget(),
                );

                tester.tap(find::text("Open menu"));
                tester.pump_and_settle();

                let scrollbar_painter = tester.widget(
                    find::descendant(
                        find::by_type::<Scrollbar>(),
                        find::by_widget_predicate(
                            "a CustomPaint with a ScrollbarPainter",
                            |_app, widget| {
                                waft_widgets::downcast_widget::<waft_widgets::CustomPaint>(widget)
                                    .and_then(|custom_paint| {
                                        custom_paint.foreground_painter.as_ref()
                                    })
                                    .is_some_and(|painter| {
                                        painter
                                            .as_any()
                                            .downcast_ref::<ScrollbarPainterRef>()
                                            .is_some()
                                    })
                            },
                        ),
                    ),
                    |custom_paint: &waft_widgets::CustomPaint| {
                        custom_paint
                            .foreground_painter
                            .as_ref()
                            .and_then(|painter| {
                                painter
                                    .as_any()
                                    .downcast_ref::<ScrollbarPainterRef>()
                                    .cloned()
                            })
                            .expect("a ScrollbarPainter")
                    },
                );
                // The scrollbar ignores the view padding, but the menu children still
                // observe it.
                assert_eq!(
                    scrollbar_painter.painter().padding(&tester.cell.borrow()),
                    EdgeInsets::ZERO.into()
                );
                assert_eq!(
                    menu_item_padding.get(),
                    Some(EdgeInsets::from_ltrb(8.0, 16.0, 32.0, 24.0))
                );

                // The menu scrolls all the way to the bottom and the scrollbar thumb
                // reaches the end of its track instead of stopping short of it.
                let scrollable = tester.element(find::descendant(
                    find::by_type::<Scrollbar>(),
                    find::by_type::<Scrollable>(),
                ));
                let position = scrollable
                    .state_handle::<ScrollableState>(&tester.cell.borrow())
                    .expect("a Scrollable element holds a ScrollableState")
                    .position(&tester.cell.borrow());
                assert!(position.max_scroll_extent(&tester.cell.borrow()) > 0.0);
                tester.drag(find::by_type::<Scrollbar>(), Offset::new(0.0, -200.0));
                tester.pump_and_settle();
                {
                    let app = tester.cell.borrow();
                    assert_eq!(position.pixels(&app), position.max_scroll_extent(&app));
                }

                // The thumb is painted at the very end of the track rather than stopping
                // short of it by the height of the removed bottom padding.
                let scrollbar_extent = tester.get_size(find::by_type::<Scrollbar>()).height();
                let scrollbar = tester.render_object(find::by_type::<Scrollbar>());
                waft_rendering::testing::assert_paints(
                    &tester.painted_ops_deep(scrollbar),
                    &[PaintCall::something(move |call| {
                        // The thumb is a rect on Android and a rounded rect elsewhere. The
                        // track shares the same shape but always starts at the top, so only
                        // the thumb is considered here.
                        let shape = match call {
                            RecordedCall::Rect { rect, .. } => Some(*rect),
                            RecordedCall::RRect { bounds, .. } => Some(*bounds),
                            _ => None,
                        };
                        Ok(shape.is_some_and(|shape| {
                            shape.top > 0.0
                                && (shape.bottom - scrollbar_extent).abs()
                                    <= f64::from(f32::EPSILON) * scrollbar_extent.max(1.0)
                        }))
                    })],
                );
            });
        }
    }

    /// The 'Accelerators', 'MenuController' and 'MenuItemButton' groups.
    mod items {
        use std::cell::Cell;
        use std::panic::{AssertUnwindSafe, catch_unwind};

        use waft_widgets::{Center, SizedBox};

        use super::*;
        use crate::badge::Badge;
        use crate::filled_button::FilledButton;
        use crate::scaffold::Scaffold;

        const ALL_PLATFORMS: [TargetPlatform; 6] = [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS,
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ];

        const NON_APPLE_PLATFORMS: [TargetPlatform; 4] = [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::Linux,
            TargetPlatform::Windows,
        ];

        const APPLE_PLATFORMS: [TargetPlatform; 2] = [TargetPlatform::IOS, TargetPlatform::MacOS];

        /// Dart's `tester.sendKeyEvent(key, character: character)`.
        fn send_key_with_character(
            tester: &mut WidgetTester,
            logical: LogicalKeyboardKey,
            character: &str,
        ) {
            tester.send_key_down_event_with_character(physical_of(logical), logical, character);
            tester.send_key_up_event(physical_of(logical), logical);
        }

        /// Dart's `MenuBar(key: UniqueKey(), controller: controller, children: menus)`.
        fn pump_menu_bar(
            tester: &mut WidgetTester,
            controller: MenuControllerRef,
            menus: TestMenus,
        ) {
            pump_material(tester, m3, |_app| {
                MenuBar::new(menus.build())
                    .key(waft_foundation::UniqueKey::new().into())
                    .controller(controller.clone())
                    .into_widget()
            });
        }

        fn new_controller(tester: &WidgetTester) -> MenuControllerRef {
            MenuController::new(&mut tester.cell.borrow_mut()).into_erased()
        }

        /// Dart's `findMnemonic(label)`: the last `Text` in the `_MenuItemLabel` that shows
        /// `label`.
        fn mnemonic(tester: &WidgetTester, label: &str) -> Option<String> {
            let item_label = ancestors_of_type::<MenuItemLabel>(tester, label)[0];
            let texts = tester.descendants_of_type::<Text>(item_label);
            tester.widget::<Text, _>(*texts.last().expect("a Text"), |text| text.data.clone())
        }

        fn shortcut(activator: SingleActivator) -> MenuSerializableShortcutRef {
            Rc::new(activator)
        }

        fn character_shortcut(activator: CharacterActivator) -> MenuSerializableShortcutRef {
            Rc::new(activator)
        }

        // ---- Accelerators ----

        // menu_anchor_test.dart 'Accelerator markers are stripped properly'.
        #[test]
        fn accelerator_markers_are_stripped_properly() {
            let expected = [
                ("Plain String", "Plain String"),
                ("&Simple Accelerator", "Simple Accelerator"),
                ("&Multiple &Accelerators", "Multiple Accelerators"),
                ("Whitespace & Accelerators", "Whitespace  Accelerators"),
                ("&Quoted && Ampersand", "Quoted & Ampersand"),
                ("Ampersand at End &", "Ampersand at End "),
                (
                    "&&Multiple Ampersands &&& &&&A &&&&B &&&&",
                    "&Multiple Ampersands & &A &&B &&",
                ),
                (
                    "Bohrium 𨨏 Code point U+28A0F",
                    "Bohrium 𨨏 Code point U+28A0F",
                ),
            ];
            let expected_indices = [-1, 0, 0, -1, 0, -1, 24, -1];
            let expected_has_accelerator = [false, true, true, false, true, false, true, false];
            for (count, (key, label)) in expected.into_iter().enumerate() {
                let mut accelerator_index = -1;
                assert_eq!(
                    MenuAcceleratorLabel::strip_accelerator_markers(
                        key,
                        Some(&mut |index| accelerator_index = index),
                    ),
                    label,
                    "'{key}' label doesn't match {label}"
                );
                assert_eq!(
                    accelerator_index, expected_indices[count],
                    "'{key}' index doesn't match {}",
                    expected_indices[count]
                );
                assert_eq!(
                    MenuAcceleratorLabel::new(key).has_accelerator(),
                    expected_has_accelerator[count],
                    "'{key}' hasAccelerator isn't {}",
                    expected_has_accelerator[count]
                );
            }
        }

        // menu_anchor_test.dart 'can invoke menu items'.
        #[test]
        fn accelerators_invoke_menu_items_off_apple_platforms() {
            test_widgets_variant(TargetPlatformVariant::new(NON_APPLE_PLATFORMS), |tester| {
                let platform = tester.target_platform();

                let log = Log::default();
                let controller = new_controller(tester);
                pump_menu_bar(
                    tester,
                    controller.clone(),
                    TestMenus {
                        accelerators: true,
                        ..TestMenus::all(&log)
                    },
                );

                tester.send_key_down_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "m");
                pump(tester);
                // Makes sure that identical accelerators in parent menu items don't
                // shadow the ones in the children.
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "m");
                tester.send_key_up_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);

                assert_eq!(log.opened(), [TestMenu::MainMenu0], "{platform:?}");
                assert_eq!(log.closed(), [TestMenu::MainMenu0], "{platform:?}");
                assert_eq!(log.selected(), [TestMenu::SubMenu00], "{platform:?}");
                // Selecting a non-submenu item should close all the menus.
                assert!(!shows(tester, &TestMenu::SubMenu00.label()));
                log.clear();

                // Invoking several levels deep.
                tester.send_key_down_event(
                    PhysicalKeyboardKey::ALT_RIGHT,
                    LogicalKeyboardKey::ALT_RIGHT,
                );
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "e");
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "1");
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "1");
                tester.send_key_up_event(
                    PhysicalKeyboardKey::ALT_RIGHT,
                    LogicalKeyboardKey::ALT_RIGHT,
                );
                pump(tester);

                assert_eq!(
                    log.opened(),
                    [TestMenu::MainMenu1, TestMenu::SubMenu11],
                    "{platform:?}"
                );
                assert_eq!(
                    log.closed(),
                    [TestMenu::SubMenu11, TestMenu::MainMenu1],
                    "{platform:?}"
                );
                assert_eq!(log.selected(), [TestMenu::SubSubMenu111], "{platform:?}");
            });
        }

        // menu_anchor_test.dart 'can combine with regular keyboard navigation'.
        #[test]
        fn accelerators_combine_with_regular_keyboard_navigation() {
            test_widgets_variant(TargetPlatformVariant::new(NON_APPLE_PLATFORMS), |tester| {
                let platform = tester.target_platform();

                let log = Log::default();
                let controller = new_controller(tester);
                pump_menu_bar(
                    tester,
                    controller.clone(),
                    TestMenus {
                        accelerators: true,
                        ..TestMenus::all(&log)
                    },
                );

                // Combining accelerators and regular keyboard navigation works.
                tester.send_key_down_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "e");
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "1");
                pump(tester);
                tester.send_key_up_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);
                send_key(tester, LogicalKeyboardKey::ARROW_RIGHT);
                pump(tester);
                send_key(tester, LogicalKeyboardKey::ENTER);
                pump(tester);

                assert_eq!(
                    log.opened(),
                    [TestMenu::MainMenu1, TestMenu::SubMenu11],
                    "{platform:?}"
                );
                assert_eq!(
                    log.closed(),
                    [TestMenu::SubMenu11, TestMenu::MainMenu1],
                    "{platform:?}"
                );
                assert_eq!(log.selected(), [TestMenu::SubSubMenu110], "{platform:?}");
            });
        }

        // menu_anchor_test.dart 'can combine with mouse'.
        #[test]
        fn accelerators_combine_with_the_mouse() {
            test_widgets_variant(TargetPlatformVariant::new(NON_APPLE_PLATFORMS), |tester| {
                let platform = tester.target_platform();

                let log = Log::default();
                let controller = new_controller(tester);
                pump_menu_bar(
                    tester,
                    controller.clone(),
                    TestMenus {
                        accelerators: true,
                        ..TestMenus::all(&log)
                    },
                );

                // Combining accelerators and regular keyboard navigation works.
                tester.send_key_down_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "e");
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "1");
                pump(tester);
                tester.send_key_up_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);
                tester.tap(find::text(&TestMenu::SubSubMenu112.label()));
                pump(tester);

                assert_eq!(
                    log.opened(),
                    [TestMenu::MainMenu1, TestMenu::SubMenu11],
                    "{platform:?}"
                );
                assert_eq!(
                    log.closed(),
                    [TestMenu::SubMenu11, TestMenu::MainMenu1],
                    "{platform:?}"
                );
                assert_eq!(log.selected(), [TestMenu::SubSubMenu112], "{platform:?}");
            });
        }

        // menu_anchor_test.dart "disabled items don't respond to accelerators".
        #[test]
        fn disabled_items_do_not_respond_to_accelerators() {
            test_widgets_variant(TargetPlatformVariant::new(NON_APPLE_PLATFORMS), |tester| {
                let platform = tester.target_platform();

                let log = Log::default();
                let controller = new_controller(tester);
                pump_menu_bar(
                    tester,
                    controller.clone(),
                    TestMenus {
                        accelerators: true,
                        ..TestMenus::all(&log)
                    },
                );

                tester.send_key_down_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "5");
                pump(tester);
                tester.send_key_up_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);

                assert!(log.opened().is_empty(), "{platform:?}");
                assert!(log.closed().is_empty(), "{platform:?}");
                assert!(log.selected().is_empty(), "{platform:?}");
                // Selecting a non-submenu item should close all the menus.
                assert!(!shows(tester, &TestMenu::SubMenu00.label()));
            });
        }

        // menu_anchor_test.dart "Apple platforms don't react to accelerators".
        #[test]
        fn apple_platforms_do_not_react_to_accelerators() {
            test_widgets_variant(TargetPlatformVariant::new(APPLE_PLATFORMS), |tester| {
                let platform = tester.target_platform();

                let log = Log::default();
                let controller = new_controller(tester);
                pump_menu_bar(
                    tester,
                    controller.clone(),
                    TestMenus {
                        accelerators: true,
                        ..TestMenus::all(&log)
                    },
                );

                tester.send_key_down_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "m");
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "m");
                tester.send_key_up_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);

                assert!(log.opened().is_empty(), "{platform:?}");
                assert!(log.closed().is_empty(), "{platform:?}");
                assert!(log.selected().is_empty(), "{platform:?}");

                // Or with the option key equivalents.
                tester.send_key_down_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "µ");
                pump(tester);
                send_key_with_character(tester, LogicalKeyboardKey::KEY_M, "µ");
                tester.send_key_up_event(
                    physical_of(LogicalKeyboardKey::ALT_LEFT),
                    LogicalKeyboardKey::ALT_LEFT,
                );
                pump(tester);

                assert!(log.opened().is_empty(), "{platform:?}");
                assert!(log.closed().is_empty(), "{platform:?}");
                assert!(log.selected().is_empty(), "{platform:?}");
            });
        }

        // ---- MenuController ----

        // menu_anchor_test.dart 'Moving a controller to a new instance works'.
        #[test]
        fn a_controller_moves_to_a_new_menu_bar() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_menu_bar(tester, controller.clone(), TestMenus::default());

                // Open a menu initially.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                tester.tap(find::text(&TestMenu::SubMenu11.label()));
                pump(tester);

                // Now pump a new menu with a different UniqueKey to dispose of the opened
                // menu's node, but keep the existing controller.
                pump_menu_bar(
                    tester,
                    controller.clone(),
                    TestMenus {
                        include_extra_groups: true,
                        ..TestMenus::default()
                    },
                );
                tester.pump_and_settle();
            });
        }

        // menu_anchor_test.dart 'closing via controller works'.
        #[test]
        fn closing_through_the_controller_closes_every_open_menu() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = new_controller(tester);
                let menus = TestMenus {
                    shortcuts: vec![(
                        TestMenu::SubSubMenu110,
                        shortcut(SingleActivator::new(LogicalKeyboardKey::KEY_A).control(true)),
                    )],
                    ..TestMenus::all(&log)
                };
                pump_material(tester, m3, |_app| {
                    MenuBar::new(menus.build())
                        .controller(controller.clone())
                        .into_widget()
                });

                // Open a menu initially.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                tester.tap(find::text(&TestMenu::SubMenu11.label()));
                pump(tester);
                let mut opened = log.opened();
                opened.sort_by_key(|menu| *menu as u32);
                assert_eq!(opened, [TestMenu::MainMenu1, TestMenu::SubMenu11]);
                log.clear();

                // Close menus using the controller.
                controller.close(&mut tester.cell.borrow_mut());
                pump(tester);

                // The menu should go away,
                let mut closed = log.closed();
                closed.sort_by_key(|menu| *menu as u32);
                assert_eq!(closed, [TestMenu::MainMenu1, TestMenu::SubMenu11]);
                assert!(log.opened().is_empty());
            });
        }

        // menu_anchor_test.dart 'internal controller is created when the controller is null'.
        #[test]
        fn an_anchor_without_a_controller_makes_its_own() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let test_controller: Rc<Cell<Option<AnyMenuController>>> = Rc::default();

                let anchor = |controller: Option<MenuControllerRef>| {
                    let sink = Rc::clone(&test_controller);
                    let mut anchor = MenuAnchor::new(Vec::new()).builder(
                        move |_app, _context, controller, _child| {
                            sink.set(Some(controller));
                            Text::new("Anchor").into_widget()
                        },
                    );
                    anchor.controller = controller;
                    anchor.into_widget()
                };
                let root = {
                    let app = tester.cell.borrow();
                    material_app(m3(&app), anchor(Some(controller.clone())))
                };
                tester.pump_widget(root);

                assert_eq!(test_controller.get(), Some(controller.handle()));

                let root = {
                    let app = tester.cell.borrow();
                    material_app(m3(&app), anchor(None))
                };
                tester.pump_widget(root);

                assert!(test_controller.get().is_some());
                assert_ne!(test_controller.get(), Some(controller.handle()));
            });
        }

        // ---- MenuItemButton ----

        // menu_anchor_test.dart 'Shortcut mnemonics are displayed'.
        #[test]
        fn shortcut_mnemonics_are_displayed_for_each_platform() {
            test_widgets_variant(TargetPlatformVariant::new(ALL_PLATFORMS), |tester| {
                let platform = tester.target_platform();

                let controller = new_controller(tester);
                let menus = TestMenus {
                    shortcuts: vec![
                        (
                            TestMenu::SubSubMenu110,
                            shortcut(SingleActivator::new(LogicalKeyboardKey::KEY_A).control(true)),
                        ),
                        (
                            TestMenu::SubSubMenu111,
                            shortcut(SingleActivator::new(LogicalKeyboardKey::KEY_B).shift(true)),
                        ),
                        (
                            TestMenu::SubSubMenu112,
                            shortcut(SingleActivator::new(LogicalKeyboardKey::KEY_C).alt(true)),
                        ),
                        (
                            TestMenu::SubSubMenu113,
                            shortcut(SingleActivator::new(LogicalKeyboardKey::KEY_D).meta(true)),
                        ),
                    ],
                    ..TestMenus::default()
                };
                pump_material(tester, m3, |_app| {
                    MenuBar::new(menus.build())
                        .controller(controller.clone())
                        .into_widget()
                });

                // Open a menu initially.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                tester.tap(find::text(&TestMenu::SubMenu11.label()));
                pump(tester);

                let labels: Vec<Option<String>> = [
                    TestMenu::SubSubMenu110,
                    TestMenu::SubSubMenu111,
                    TestMenu::SubSubMenu112,
                    TestMenu::SubSubMenu113,
                ]
                .into_iter()
                .map(|menu| mnemonic(tester, &menu.label()))
                .collect();
                let expected = match platform {
                    TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::Linux => {
                        ["Ctrl+A", "Shift+B", "Alt+C", "Meta+D"]
                    }
                    TargetPlatform::Windows => ["Ctrl+A", "Shift+B", "Alt+C", "Win+D"],
                    TargetPlatform::IOS | TargetPlatform::MacOS => ["⌃ A", "⇧ B", "⌥ C", "⌘ D"],
                };
                assert_eq!(
                    labels,
                    expected.map(|label| Some(label.to_string())),
                    "{platform:?}"
                );

                let menus = TestMenus {
                    include_extra_groups: true,
                    shortcuts: vec![
                        (
                            TestMenu::SubSubMenu110,
                            shortcut(SingleActivator::new(LogicalKeyboardKey::ARROW_RIGHT)),
                        ),
                        (
                            TestMenu::SubSubMenu111,
                            shortcut(SingleActivator::new(LogicalKeyboardKey::ARROW_LEFT)),
                        ),
                        (
                            TestMenu::SubSubMenu112,
                            shortcut(SingleActivator::new(LogicalKeyboardKey::ARROW_UP)),
                        ),
                        (
                            TestMenu::SubSubMenu113,
                            shortcut(SingleActivator::new(LogicalKeyboardKey::ARROW_DOWN)),
                        ),
                    ],
                    ..TestMenus::default()
                };
                pump_material(tester, m3, |_app| {
                    MenuBar::new(menus.build())
                        .controller(controller.clone())
                        .into_widget()
                });
                tester.pump_and_settle();

                assert_eq!(
                    mnemonic(tester, &TestMenu::SubSubMenu110.label()).as_deref(),
                    Some("→")
                );
                assert_eq!(
                    mnemonic(tester, &TestMenu::SubSubMenu111.label()).as_deref(),
                    Some("←")
                );
                assert_eq!(
                    mnemonic(tester, &TestMenu::SubSubMenu112.label()).as_deref(),
                    Some("↑")
                );
                assert_eq!(
                    mnemonic(tester, &TestMenu::SubSubMenu113.label()).as_deref(),
                    Some("↓")
                );

                // Try some weirder ones.
                let menus = TestMenus {
                    shortcuts: vec![
                        (
                            TestMenu::SubSubMenu110,
                            shortcut(SingleActivator::new(LogicalKeyboardKey::ESCAPE)),
                        ),
                        (
                            TestMenu::SubSubMenu111,
                            shortcut(SingleActivator::new(LogicalKeyboardKey::FN)),
                        ),
                        (
                            TestMenu::SubSubMenu112,
                            shortcut(SingleActivator::new(LogicalKeyboardKey::ENTER)),
                        ),
                    ],
                    ..TestMenus::default()
                };
                pump_material(tester, m3, |_app| {
                    MenuBar::new(menus.build())
                        .controller(controller)
                        .into_widget()
                });
                tester.pump_and_settle();

                assert_eq!(
                    mnemonic(tester, &TestMenu::SubSubMenu110.label()).as_deref(),
                    Some("Esc")
                );
                assert_eq!(
                    mnemonic(tester, &TestMenu::SubSubMenu111.label()).as_deref(),
                    Some("Fn")
                );
                assert_eq!(
                    mnemonic(tester, &TestMenu::SubSubMenu112.label()).as_deref(),
                    Some("↵")
                );
            });
        }

        // menu_anchor_test.dart 'CharacterActivator shortcut mnemonics include modifiers'.
        #[test]
        fn character_activator_mnemonics_include_their_modifiers() {
            test_widgets_variant(TargetPlatformVariant::new(ALL_PLATFORMS), |tester| {
                let platform = tester.target_platform();

                let controller = new_controller(tester);
                let menus = TestMenus {
                    shortcuts: vec![
                        (
                            TestMenu::SubSubMenu110,
                            character_shortcut(CharacterActivator::new("A").control(true)),
                        ),
                        (
                            TestMenu::SubSubMenu111,
                            character_shortcut(CharacterActivator::new("B").alt(true)),
                        ),
                        (
                            TestMenu::SubSubMenu112,
                            character_shortcut(CharacterActivator::new("C").meta(true)),
                        ),
                    ],
                    ..TestMenus::default()
                };
                pump_material(tester, m3, |_app| {
                    MenuBar::new(menus.build())
                        .controller(controller)
                        .into_widget()
                });

                // Open a menu initially.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                tester.tap(find::text(&TestMenu::SubMenu11.label()));
                pump(tester);

                let labels: Vec<Option<String>> = [
                    TestMenu::SubSubMenu110,
                    TestMenu::SubSubMenu111,
                    TestMenu::SubSubMenu112,
                ]
                .into_iter()
                .map(|menu| mnemonic(tester, &menu.label()))
                .collect();
                let expected = match platform {
                    TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::Linux => {
                        ["Ctrl+A", "Alt+B", "Meta+C"]
                    }
                    TargetPlatform::Windows => ["Ctrl+A", "Alt+B", "Win+C"],
                    TargetPlatform::IOS | TargetPlatform::MacOS => ["⌃ A", "⌥ B", "⌘ C"],
                };
                assert_eq!(
                    labels,
                    expected.map(|label| Some(label.to_string())),
                    "{platform:?}"
                );
            });
        }

        /// Dart's `MenuBar(controller:, children: [SubmenuButton(menuChildren: [item], child:
        /// Text(mainMenu0))])`.
        fn pump_one_submenu(
            tester: &mut WidgetTester,
            controller: MenuControllerRef,
            item: MenuItemButton,
        ) {
            pump_material(tester, m3, |_app| {
                MenuBar::new(vec![
                    SubmenuButton::new(
                        vec![item.into_widget()],
                        Some(Text::new(TestMenu::MainMenu0.label())),
                    )
                    .into_widget(),
                ])
                .controller(controller.clone())
                .into_widget()
            });
        }

        // menu_anchor_test.dart 'leadingIcon is used when set'.
        #[test]
        fn a_leading_icon_is_shown_when_set() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_one_submenu(
                    tester,
                    controller.clone(),
                    MenuItemButton::new()
                        .leading_icon(Text::new("leadingIcon"))
                        .child(Text::new(TestMenu::SubMenu00.label())),
                );

                tester.tap(find::text(&TestMenu::MainMenu0.label()));
                pump(tester);

                assert!(shows(tester, "leadingIcon"));
            });
        }

        // menu_anchor_test.dart 'autofocus is used when set and widget is enabled'.
        #[test]
        fn an_enabled_autofocus_item_takes_focus_when_the_menu_opens() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_material(tester, m3, |_app| {
                    Column::new()
                        .children(vec![
                            MenuAnchor::new(vec![
                                MenuItemButton::new()
                                    .autofocus(true)
                                    // Required for clickability.
                                    .on_pressed(Listener::new(|_app| {}))
                                    .child(Text::new(TestMenu::MainMenu0.label()))
                                    .into_widget(),
                                MenuItemButton::new()
                                    .on_pressed(Listener::new(|_app| {}))
                                    .child(Text::new(TestMenu::MainMenu1.label()))
                                    .into_widget(),
                            ])
                            .controller(controller.clone())
                            .into_widget(),
                            Expanded::new(placeholder()).into_widget(),
                        ])
                        .into_widget()
                });

                controller.open(&mut tester.cell.borrow_mut(), None);
                pump(tester);

                assert!(controller.is_open(&tester.cell.borrow()));
                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("MenuItemButton", &TestMenu::MainMenu0.label()))
                );
            });
        }

        // menu_anchor_test.dart 'trailingIcon is used when set'.
        #[test]
        fn a_trailing_icon_is_shown_when_set() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_one_submenu(
                    tester,
                    controller.clone(),
                    MenuItemButton::new()
                        .trailing_icon(Text::new("trailingIcon"))
                        .child(Text::new(TestMenu::SubMenu00.label())),
                );

                tester.tap(find::text(&TestMenu::MainMenu0.label()));
                pump(tester);

                assert!(shows(tester, "trailingIcon"));
            });
        }

        // menu_anchor_test.dart 'SubmenuButton uses supplied controller'.
        #[test]
        fn a_submenu_button_uses_the_controller_it_is_given() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let submenu_controller = new_controller(tester);
                let menu_bar = |submenu_controller: Option<MenuControllerRef>| {
                    let mut submenu = SubmenuButton::new(
                        vec![
                            MenuItemButton::new()
                                .child(Text::new(TestMenu::SubMenu00.label()))
                                .into_widget(),
                        ],
                        Some(Text::new(TestMenu::MainMenu0.label())),
                    );
                    submenu.controller = submenu_controller;
                    MenuBar::new(vec![submenu.into_widget()])
                        .controller(controller.clone())
                        .into_widget()
                };
                pump_material(tester, m3, |_app| {
                    menu_bar(Some(submenu_controller.clone()))
                });

                submenu_controller.open(&mut tester.cell.borrow_mut(), None);
                pump(tester);
                assert!(shows(tester, &TestMenu::SubMenu00.label()));

                submenu_controller.close(&mut tester.cell.borrow_mut());
                pump(tester);
                assert!(!shows(tester, &TestMenu::SubMenu00.label()));

                // Now remove the controller and try to control it.
                pump_material(tester, m3, |_app| menu_bar(None));

                let opened = catch_unwind(AssertUnwindSafe(|| {
                    submenu_controller.open(&mut tester.cell.borrow_mut(), None);
                }));
                assert!(
                    opened.is_err(),
                    "opening a detached controller fails an assert"
                );
                pump(tester);
                assert!(!shows(tester, &TestMenu::SubMenu00.label()));
            });
        }

        /// Dart's `MenuAnchor(controller:, menuChildren: [MenuItemButton(closeOnActivate:,
        /// onPressed: () {}, child: Text('Button 1'))], builder: FilledButton('Tap me'))` in a
        /// `Center`.
        fn pump_tap_me_anchor(
            tester: &mut WidgetTester,
            controller: MenuControllerRef,
            close_on_activate: bool,
        ) {
            pump_material(tester, m3, |_app| {
                Center::new()
                    .child(
                        MenuAnchor::new(vec![
                            MenuItemButton::new()
                                .close_on_activate(close_on_activate)
                                .on_pressed(Listener::new(|_app| {}))
                                .child(Text::new("Button 1"))
                                .into_widget(),
                        ])
                        .controller(controller.clone())
                        .builder(|_app, _context, controller, _child| {
                            FilledButton::new(
                                Some(Listener::new(move |app| controller.open(app, None))),
                                Some(Text::new("Tap me")),
                            )
                            .into_widget()
                        }),
                    )
                    .into_widget()
            });
        }

        // menu_anchor_test.dart 'MenuItemButton respects closeOnActivate property'.
        #[test]
        fn close_on_activate_decides_whether_pressing_an_item_closes_the_menu() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_tap_me_anchor(tester, controller.clone(), true);

                tester.tap(find::text("Tap me"));
                pump(tester);
                expect(
                    tester,
                    &find::by_type::<MenuItemButton>(),
                    finds_one_widget(),
                );

                // Taps the MenuItemButton which should close the menu.
                tester.tap(find::text("Button 1"));
                pump(tester);
                expect(tester, &find::by_type::<MenuItemButton>(), finds_nothing());

                tester.pump_and_settle();

                pump_tap_me_anchor(tester, controller.clone(), false);

                tester.tap(find::text("Tap me"));
                pump(tester);
                expect(
                    tester,
                    &find::by_type::<MenuItemButton>(),
                    finds_one_widget(),
                );

                // Taps the MenuItemButton which shouldn't close the menu.
                tester.tap(find::text("Button 1"));
                pump(tester);
                expect(
                    tester,
                    &find::by_type::<MenuItemButton>(),
                    finds_one_widget(),
                );
            });
        }

        /// Dart's `MaterialApp(home: Scaffold(body: SizedBox(width: width, child: button)))`.
        fn pump_in_scaffold(tester: &mut WidgetTester, width: Option<f64>, button: MenuItemButton) {
            let root = {
                let app = tester.cell.borrow();
                let mut sized_box = SizedBox::new().child(button);
                if let Some(width) = width {
                    sized_box = sized_box.width(width);
                }
                material_app(m3(&app), Scaffold::new().body(sized_box).into_widget())
            };
            tester.pump_widget(root);
        }

        // menu_anchor_test.dart 'MenuItemButton does not overflow when child is long'.
        #[test]
        fn a_vertical_overflow_item_with_a_long_child_builds() {
            test_widgets(|tester| {
                pump_in_scaffold(
                    tester,
                    Some(200.0),
                    MenuItemButton::new()
                        .overflow_axis(Axis::Vertical)
                        .on_pressed(Listener::new(|_app| {}))
                        .child(Text::new(
                            "MenuItem Button does not overflow when child is long",
                        )),
                );
            });
        }

        // menu_anchor_test.dart 'MenuItemButton layout is updated by overflowAxis'. The last
        // step's overflow assertion is not taken: Waft's flex reports no overflow (deferred
        // with the overflow indicator in waft-rendering).
        #[test]
        fn an_item_lays_out_by_its_overflow_axis() {
            test_widgets(|tester| {
                let build_menu_button =
                    |tester: &mut WidgetTester, overflow_axis: Axis, constrained_layout: bool| {
                        pump_in_scaffold(
                        tester,
                        constrained_layout.then_some(200.0),
                        MenuItemButton::new()
                            .overflow_axis(overflow_axis)
                            .on_pressed(Listener::new(|_app| {}))
                            .child(Text::new(
                                "This is a very long text that will wrap to the multiple lines.",
                            )),
                    );
                    };

                // Test a long MenuItemButton in an unconstrained layout with vertical overflow axis.
                build_menu_button(tester, Axis::Vertical, false);
                assert_eq!(
                    rect_of::<MenuItemButton>(tester).size(),
                    Size::new(800.0, 48.0)
                );

                // Test a long MenuItemButton in an unconstrained layout with horizontal overflow axis.
                build_menu_button(tester, Axis::Horizontal, false);
                assert_eq!(
                    rect_of::<MenuItemButton>(tester).size(),
                    Size::new(800.0, 48.0)
                );

                // Test a long MenuItemButton in a constrained layout with vertical overflow axis.
                build_menu_button(tester, Axis::Vertical, true);
                assert_eq!(
                    rect_of::<MenuItemButton>(tester).size(),
                    Size::new(200.0, 120.0)
                );

                // Test a long MenuItemButton in a constrained layout with horizontal overflow axis.
                build_menu_button(tester, Axis::Horizontal, true);
                assert_eq!(
                    rect_of::<MenuItemButton>(tester).size(),
                    Size::new(200.0, 48.0)
                );
            });
        }

        // menu_anchor_test.dart 'MenuItemButton does not clip leadingIcon'.
        #[test]
        fn an_item_clips_its_label_but_not_its_leading_icon() {
            test_widgets(|tester| {
                let label = "This is a very long menu item label that must be clipped";
                let leading = GlobalKey::new();
                let leading_key = leading.clone();
                let root = {
                    let app = tester.cell.borrow();
                    material_app(
                        m3(&app),
                        Scaffold::new()
                            .body(
                                MenuAnchor::new(vec![
                                    SizedBox::new()
                                        .width(200.0)
                                        .child(
                                            MenuItemButton::new()
                                                .leading_icon(
                                                    Badge::new().label(Text::new("1")).child(
                                                        Icon::new(Some(Icons::add()))
                                                            .key(leading_key.into()),
                                                    ),
                                                )
                                                .on_pressed(Listener::new(|_app| {}))
                                                .child(Text::new(label).max_lines(1)),
                                        )
                                        .into_widget(),
                                ])
                                .builder(
                                    |_app, _context, controller, _child| {
                                        TextButton::new(
                                            Some(Listener::new(move |app| {
                                                controller.open(app, None)
                                            })),
                                            Some(Text::new("Open")),
                                        )
                                        .into_widget()
                                    },
                                ),
                            )
                            .into_widget(),
                    )
                };
                tester.pump_widget(root);

                tester.tap(find::text("Open"));
                tester.pump_and_settle();

                // Decorations that paint outside the leading icon's bounds stay visible.
                let leading_element = leading
                    .current_context(&tester.cell.borrow())
                    .expect("the leading icon is mounted");
                let clips_above = |tester: &WidgetTester, element: AnyElement| {
                    let ancestors = tester.ancestors(element);
                    let app = tester.cell.borrow();
                    ancestors
                        .into_iter()
                        .filter(|ancestor| {
                            waft_widgets::downcast_widget::<ClipRect>(&**ancestor.widget(&app))
                                .is_some()
                        })
                        .count()
                };
                assert_eq!(clips_above(tester, leading_element), 0);

                // The label itself is still clipped so that long labels truncate.
                let label_element = find::text(label).evaluate(tester)[0];
                assert_eq!(clips_above(tester, label_element), 1);
                let app = tester.cell.borrow();
                let paragraph = label_element
                    .find_render_object(&app)
                    .and_then(|render| render.downcast::<waft_rendering::RenderParagraph>(&app))
                    .expect("a Text lays out a paragraph");
                assert!(paragraph.did_exceed_max_lines(&app));
            });
        }

        // menu_anchor_test.dart 'MenuItemButton.styleFrom overlayColor overrides default overlay
        // color'.
        #[test]
        fn style_from_overlay_color_overrides_the_default_overlay_color() {
            test_widgets(|tester| {
                let overlay_color = Color::new(0xffff0000);
                pump_in_scaffold(
                    tester,
                    None,
                    MenuItemButton::new()
                        .style(
                            MenuItemButton::style_from()
                                .overlay_color(overlay_color)
                                .build(),
                        )
                        .on_pressed(Listener::new(|_app| {}))
                        .child(Text::new("MenuItem")),
                );

                // Hovered.
                let center = rect_of::<MenuItemButton>(tester).center();
                let mut gesture = TestPointer::mouse(1, 0);
                tester.add_pointer(&mut gesture, Offset::ZERO);
                tester.move_to(&mut gesture, center, false);
                tester.pump_and_settle();
                assert_paints(
                    &overlay_ops(tester),
                    &[PaintCall::rect(Some(overlay_color.with_opacity(0.08)))],
                );

                // Highlighted (pressed).
                gesture.buttons = waft_gestures::K_PRIMARY_BUTTON;
                tester.down(&mut gesture, center);
                tester.pump_and_settle();
                assert_paints(
                    &overlay_ops(tester),
                    &[
                        PaintCall::rect(Some(overlay_color.with_opacity(0.08))),
                        PaintCall::rect(Some(overlay_color.with_opacity(0.08))),
                        PaintCall::rect(Some(overlay_color.with_opacity(0.1))),
                    ],
                );
            });
        }

        // menu_anchor_test.dart 'MenuItemButton can build when its child is null'.
        #[test]
        fn an_item_with_no_child_builds() {
            test_widgets(|tester| {
                pump_in_scaffold(tester, Some(200.0), MenuItemButton::new());
            });
        }
    }

    mod layout {
        //! menu_anchor_test.dart's 'Layout' group with the cases that follow it up to 'Mouse
        //! cursors', and the 'Mouse cursors' group.

        use std::cell::Cell;

        use waft_embedder::FontStyle;
        use waft_painting::{BoxDecoration, TextStyle};
        use waft_rendering::AnyRenderObject;
        use waft_services::{SystemMouseCursor, SystemMouseCursors};

        use waft_widgets::{
            Align, Center, DecoratedBox, Overlay, OverlayEntry, Positioned, RenderTheater,
            SizedBox, Stack, StatefulBuilder, UnfocusDisposition, WidgetStatePropertyAll,
        };

        use super::*;
        use crate::app::MaterialApp;
        use crate::colors::Colors;
        use crate::filled_button::FilledButton;
        use crate::material_state::MaterialStatePropertyAll;
        use crate::scaffold::Scaffold;
        use crate::text_theme::TextTheme;
        use crate::tooltip::tests::m2;

        /// `tester.pumpWidget(MaterialApp(theme: theme(app), home: home(app)))`.
        fn pump_app(
            tester: &mut WidgetTester,
            theme: impl FnOnce(&App) -> ThemeData,
            home: impl FnOnce(&mut App) -> WidgetRef,
        ) {
            let root = {
                let mut app = tester.cell.borrow_mut();
                let theme = theme(&app);
                let home = home(&mut app);
                material_app(theme, home)
            };
            tester.pump_widget(root);
        }

        /// Dart's `collectSubmenuRects()`: the rects of the menu panels, in finder order.
        fn submenu_rects(tester: &WidgetTester) -> Vec<Rect> {
            menu_panels(tester)
                .into_iter()
                .map(|panel| rect_of_element(tester, panel))
                .collect()
        }

        /// Dart's `tester.getCenter(find.byType(W).first)`.
        fn center_of_first<W: 'static>(tester: &WidgetTester) -> Offset {
            let element = *find::by_type::<W>()
                .evaluate(tester)
                .first()
                .expect("a widget of the type");
            rect_of_element(tester, element).center()
        }

        /// Dart's `tester.getCenter(find.byType(W).last)`.
        fn center_of_last<W: 'static>(tester: &WidgetTester) -> Offset {
            let element = *find::by_type::<W>()
                .evaluate(tester)
                .last()
                .expect("a widget of the type");
            rect_of_element(tester, element).center()
        }

        /// A `FilledButton` labelled `label` that toggles `controller`'s menu, as the builders of
        /// Dart's cases write it.
        fn toggle_button(controller: AnyMenuController, label: &str) -> WidgetRef {
            FilledButton::new(
                Some(Listener::new(move |app| {
                    if controller.is_open(app) {
                        controller.close(app);
                    } else {
                        controller.open(app, None);
                    }
                })),
                Some(Text::new(label)),
            )
            .into_widget()
        }

        fn new_controller(tester: &WidgetTester) -> MenuControllerRef {
            MenuController::new(&mut tester.cell.borrow_mut()).into_erased()
        }

        /// Dart's `findsNWidgets(count)` of `find.byType(W)`.
        fn count_of<W: 'static>(tester: &WidgetTester) -> usize {
            find::by_type::<W>().evaluate(tester).len()
        }

        // ---- Layout ----

        // menu_anchor_test.dart 'tapping MenuItemButton with null focus node'.
        #[test]
        fn tapping_a_menu_item_button_that_drops_its_focus_node_does_not_panic() {
            test_widgets(|tester| {
                let button_focus_node: Rc<RefCell<Option<FocusNodeRef>>> =
                    Rc::new(RefCell::new(Some(
                        FocusNode::new(&mut tester.cell.borrow_mut()).into_erased::<AnyFocusNode>(),
                    )));
                let focus_node = Rc::clone(&button_focus_node);
                pump_app(tester, m3, move |_app| {
                    StatefulBuilder::new(move |_app, _context, set_state| {
                        let mut item = MenuItemButton::new()
                            .close_on_activate(false)
                            .child(Text::new("Set focus to null"));
                        item.focus_node = focus_node.borrow().clone();
                        let focus_node = Rc::clone(&focus_node);
                        let item = item.on_pressed(Listener::new(move |app| {
                            let focus_node = Rc::clone(&focus_node);
                            set_state(app, &mut |app| {
                                if let Some(node) = focus_node.borrow_mut().take() {
                                    node.dispose(app);
                                }
                            });
                        }));
                        MenuAnchor::new(vec![item.into_widget()])
                            .builder(|_app, _context, controller, _child| {
                                TextButton::new(
                                    Some(Listener::new(move |app| {
                                        if controller.is_open(app) {
                                            controller.close(app);
                                        } else {
                                            controller.open(app, None);
                                        }
                                    })),
                                    Some(Text::new("OPEN MENU")),
                                )
                                .into_widget()
                            })
                            .into_widget()
                    })
                    .into_widget()
                });

                tester.tap(find::text("OPEN MENU"));
                pump(tester);

                assert!(shows(tester, "Set focus to null"));

                tester.tap(find::text("Set focus to null"));
                tester.pump_and_settle();
                assert!(button_focus_node.take().is_none());
            });
        }

        /// The anchor of Dart's 'constrained menus show up in the right place with offset' cases:
        /// four nested submenus, the first and third offset by 10 along the menu.
        fn nested_offset_menus(tester: &mut WidgetTester, text_direction: TextDirection) {
            pump_app(tester, m2, move |_app| {
                let submenu_4 =
                    SubmenuButton::new(Vec::new(), Some(Text::new("SubMenuButton4"))).into_widget();
                let submenu_3 =
                    SubmenuButton::new(vec![submenu_4], Some(Text::new("SubMenuButton3")))
                        .alignment_offset(Offset::new(10.0, 0.0))
                        .into_widget();
                let submenu_2 =
                    SubmenuButton::new(vec![submenu_3], Some(Text::new("SubMenuButton2")))
                        .into_widget();
                let submenu_1 =
                    SubmenuButton::new(vec![submenu_2], Some(Text::new("SubMenuButton1")))
                        .alignment_offset(Offset::new(10.0, 0.0))
                        .into_widget();
                let alignment = match text_direction {
                    TextDirection::Ltr => AlignmentGeometry::TOP_LEFT,
                    TextDirection::Rtl => AlignmentGeometry::TOP_RIGHT,
                };
                Directionality::new(
                    text_direction,
                    Align::new().alignment(alignment).child(
                        MenuAnchor::new(vec![submenu_1]).builder(
                            |_app, _context, controller, _child| {
                                toggle_button(controller, "Tap me")
                            },
                        ),
                    ),
                )
                .into_widget()
            });
            pump(tester);

            for text in [
                "Tap me",
                "SubMenuButton1",
                "SubMenuButton2",
                "SubMenuButton3",
            ] {
                tester.tap(find::text(text));
                pump(tester);
            }
        }

        // menu_anchor_test.dart 'constrained menus show up in the right place with offset in LTR'.
        #[test]
        fn constrained_menus_show_up_in_the_right_place_with_offset_in_ltr() {
            test_widgets(|tester| {
                nested_offset_menus(tester, TextDirection::Ltr);

                assert_eq!(count_of::<SubmenuButton>(tester), 4);
                assert_eq!(
                    submenu_rects(tester),
                    [
                        Rect::from_ltrb(0.0, 48.0, 256.0, 112.0),
                        Rect::from_ltrb(266.0, 48.0, 522.0, 112.0),
                        Rect::from_ltrb(522.0, 48.0, 778.0, 112.0),
                        Rect::from_ltrb(256.0, 48.0, 512.0, 112.0),
                    ]
                );
            });
        }

        // menu_anchor_test.dart 'constrained menus show up in the right place with offset in RTL'.
        #[test]
        fn constrained_menus_show_up_in_the_right_place_with_offset_in_rtl() {
            test_widgets(|tester| {
                nested_offset_menus(tester, TextDirection::Rtl);

                assert_eq!(count_of::<SubmenuButton>(tester), 4);
                assert_eq!(
                    submenu_rects(tester),
                    [
                        Rect::from_ltrb(544.0, 48.0, 800.0, 112.0),
                        Rect::from_ltrb(278.0, 48.0, 534.0, 112.0),
                        Rect::from_ltrb(22.0, 48.0, 278.0, 112.0),
                        Rect::from_ltrb(288.0, 48.0, 544.0, 112.0),
                    ]
                );
            });
        }

        /// Dart's 'vertically constrained menus' boilerplate: a menu anchored at the bottom left of
        /// the view, opened by its button.
        fn bottom_anchored_menu(tester: &mut WidgetTester, alignment_offset: Option<Offset>) {
            pump_app(tester, m2, move |_app| {
                let mut anchor = MenuAnchor::new(vec![
                    MenuItemButton::new()
                        .child(Text::new("Button1"))
                        .into_widget(),
                ])
                .builder(|_app, _context, controller, _child| toggle_button(controller, "Tap me"));
                if let Some(alignment_offset) = alignment_offset {
                    anchor = anchor.alignment_offset(alignment_offset);
                }
                Directionality::new(
                    TextDirection::Ltr,
                    Align::new()
                        .alignment(AlignmentGeometry::BOTTOM_LEFT)
                        .child(anchor),
                )
                .into_widget()
            });

            pump(tester);
            tester.tap(find::text("Tap me"));
            pump(tester);
        }

        // menu_anchor_test.dart 'vertically constrained menus are positioned above the anchor by
        // default'.
        #[test]
        fn vertically_constrained_menus_are_positioned_above_the_anchor_by_default() {
            test_widgets(|tester| {
                bottom_anchored_menu(tester, None);

                assert_eq!(count_of::<MenuItemButton>(tester), 1);
                // Test the default offset (0, 0) vertical position.
                assert_eq!(
                    submenu_rects(tester),
                    [Rect::from_ltrb(0.0, 488.0, 122.0, 552.0)]
                );
            });
        }

        // menu_anchor_test.dart 'vertically constrained menus are positioned above the anchor with
        // the provided offset'.
        #[test]
        fn vertically_constrained_menus_are_positioned_above_the_anchor_with_the_provided_offset() {
            test_widgets(|tester| {
                bottom_anchored_menu(tester, Some(Offset::new(0.0, 50.0)));

                assert_eq!(count_of::<MenuItemButton>(tester), 1);
                // Test the offset (0, 50) vertical position.
                assert_eq!(
                    submenu_rects(tester),
                    [Rect::from_ltrb(0.0, 438.0, 122.0, 502.0)]
                );
            });
        }

        /// Dart's `tester.getRect(find.byKey(key))`.
        fn rect_of_key(tester: &WidgetTester, key: &GlobalKey) -> Rect {
            tester.get_rect(tester.render_object(key).as_box().expect("a render box"))
        }

        /// Dart's `tester.getRect(findMenuPanels())`.
        fn menu_panel_rect(tester: &WidgetTester) -> Rect {
            rect_of_element(tester, only(menu_panels(tester)))
        }

        // menu_anchor_test.dart 'Menu follows content position when a LayerLink is provided'.
        #[test]
        fn a_menu_follows_its_content_when_given_a_layer_link() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let content_key = GlobalKey::new();

                let boilerplate = |tester: &mut WidgetTester, bottom_insets: f64| {
                    let content_key = content_key.clone();
                    pump_app(tester, m3, {
                        let controller = controller.clone();
                        move |app| {
                            let layer_link = LayerLink::new(app);
                            MediaQuery::new(
                                MediaQueryData::new().view_insets(EdgeInsets::only(
                                    0.0,
                                    0.0,
                                    0.0,
                                    bottom_insets,
                                )),
                                Scaffold::new().body(
                                    Center::new().child(
                                        MenuAnchor::new(vec![
                                            MenuItemButton::new()
                                                .on_pressed(Listener::new(|_app| {}))
                                                .child(Text::new("Button 1"))
                                                .into_widget(),
                                        ])
                                        .controller(controller.clone())
                                        .layer_link(layer_link)
                                        .builder(
                                            move |_app, _context, _controller, _child| {
                                                SizedBox::new()
                                                    .key(content_key.clone().into())
                                                    .width(100.0)
                                                    .height(100.0)
                                                    .into_widget()
                                            },
                                        ),
                                    ),
                                ),
                            )
                            .into_widget()
                        }
                    });
                };

                // Build once without bottom insets and open the menu.
                boilerplate(tester, 0.0);
                controller.open(&mut tester.cell.borrow_mut(), None);
                pump(tester);

                // Menu vertical position is just under the content.
                assert_eq!(
                    menu_panel_rect(tester).top,
                    rect_of_key(tester, &content_key).bottom
                );

                // Simulate the keyboard opening resizing the view.
                boilerplate(tester, 100.0);
                pump(tester);

                // Menu vertical position is just under the content.
                assert_eq!(
                    menu_panel_rect(tester).top,
                    rect_of_key(tester, &content_key).bottom
                );
            });
        }

        // menu_anchor_test.dart 'menu is positioned to avoid the software keyboard'.
        #[test]
        fn a_menu_is_positioned_to_avoid_the_software_keyboard() {
            test_widgets(|tester| {
                // Regression test for https://github.com/flutter/flutter/issues/142921
                // The menu should not extend into the area occupied by the software keyboard.
                let screen_size = Size::new(600.0, 800.0);
                // Dart's `changeSurfaceSize(tester, screenSize)`.
                tester.set_surface_size(Some(screen_size));
                const KEYBOARD_HEIGHT: f64 = 200.0;
                let controller = new_controller(tester);

                let root = {
                    let app = tester.cell.borrow();
                    MaterialApp::new()
                        .theme(m3(&app))
                        .builder(|app, context, child| {
                            MediaQuery::new(
                                MediaQuery::of(app, context)
                                    .copy_with()
                                    .view_insets(EdgeInsets::only(0.0, 0.0, 0.0, KEYBOARD_HEIGHT)),
                                child.cloned().expect("the navigator"),
                            )
                            .into_widget()
                        })
                        .home(Material::new().child(Stack::new().children(vec![
                            // Position anchor just above the keyboard, so menu would extend
                            // into keyboard area if opened below.
                            Positioned::new(
                                MenuAnchor::new(
                                    (0..5)
                                        .map(|index| {
                                            MenuItemButton::new()
                                                .on_pressed(Listener::new(|_app| {}))
                                                .child(Text::new(format!("Item {index}")))
                                                .into_widget()
                                        })
                                        .collect(),
                                )
                                .controller(controller.clone())
                                .builder(
                                    |_app, _context, controller, _child| {
                                        FilledButton::new(
                                            Some(Listener::new(move |app| {
                                                controller.open(app, None);
                                            })),
                                            Some(Text::new("Open Menu")),
                                        )
                                        .into_widget()
                                    },
                                ),
                            )
                            .left(100.0)
                            // 100px above keyboard
                            .bottom(KEYBOARD_HEIGHT + 100.0)
                            .into_widget(),
                        ])))
                        .into_widget()
                };
                tester.pump_widget(root);

                controller.open(&mut tester.cell.borrow_mut(), None);
                tester.pump_and_settle();

                let menu_rect = menu_panel_rect(tester);
                // Menu should not extend into the keyboard area (bottom 200px of screen).
                assert!(
                    menu_rect.bottom <= screen_size.height() - KEYBOARD_HEIGHT,
                    "Menu bottom ({}) should not extend into keyboard area (below {})",
                    menu_rect.bottom,
                    screen_size.height() - KEYBOARD_HEIGHT
                );
            });
        }

        // menu_anchor_test.dart 'Menu is correctly offset when a LayerLink is provided and
        // alignmentOffset is set'.
        #[test]
        fn a_menu_with_a_layer_link_is_offset_by_its_alignment_offset() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let content_key = GlobalKey::new();
                const HORIZONTAL_OFFSET: f64 = 16.0;
                const VERTICAL_OFFSET: f64 = 20.0;

                let key = content_key.clone();
                pump_app(tester, m3, {
                    let controller = controller.clone();
                    move |app| {
                        let layer_link = LayerLink::new(app);
                        Scaffold::new()
                            .body(
                                Center::new().child(
                                    MenuAnchor::new(vec![
                                        MenuItemButton::new()
                                            .on_pressed(Listener::new(|_app| {}))
                                            .child(Text::new("Button 1"))
                                            .into_widget(),
                                    ])
                                    .controller(controller.clone())
                                    .layer_link(layer_link)
                                    .alignment_offset(Offset::new(
                                        HORIZONTAL_OFFSET,
                                        VERTICAL_OFFSET,
                                    ))
                                    .builder(
                                        move |_app, _context, _controller, _child| {
                                            SizedBox::new()
                                                .key(key.clone().into())
                                                .width(100.0)
                                                .height(100.0)
                                                .into_widget()
                                        },
                                    ),
                                ),
                            )
                            .into_widget()
                    }
                });

                controller.open(&mut tester.cell.borrow_mut(), None);
                pump(tester);

                assert_eq!(
                    menu_panel_rect(tester).top,
                    rect_of_key(tester, &content_key).bottom + VERTICAL_OFFSET
                );
                assert_eq!(
                    menu_panel_rect(tester).left,
                    rect_of_key(tester, &content_key).left + HORIZONTAL_OFFSET
                );
            });
        }

        // menu_anchor_test.dart 'Menu vertical padding should not be reduced with compact visual
        // density'.
        #[test]
        fn compact_visual_density_does_not_reduce_a_menus_vertical_padding() {
            test_widgets(|tester| {
                // Helper function to get menu padding by measuring first/last items.
                let menu_padding = |tester: &WidgetTester| -> (f64, f64) {
                    // Find any menu items that are available.
                    let menu_items = find::by_type::<SubmenuButton>().evaluate(tester);
                    if menu_items.len() < 2 {
                        return (0.0, 0.0);
                    }

                    let first_item = rect_of_element(tester, menu_items[0]);
                    let last_item = rect_of_element(tester, *menu_items.last().expect("two items"));
                    let materials = find::by_type::<Material>().evaluate(tester);
                    let menu_panel =
                        rect_of_element(tester, *materials.last().expect("a material"));

                    let top_padding = first_item.top - menu_panel.top;
                    let bottom_padding = menu_panel.bottom - last_item.bottom;
                    (top_padding, bottom_padding)
                };

                let build_simple_menu_anchor =
                    |tester: &mut WidgetTester, visual_density: VisualDensity| {
                        pump_app(
                            tester,
                            |app| {
                                ThemeData::builder()
                                    .visual_density(visual_density)
                                    .build(app)
                            },
                            |_app| {
                                let text = |text: &str| {
                                    DecoratedBox::new(
                                        BoxDecoration::new().color(Colors::BLUE.primary()),
                                    )
                                    .child(Text::new(text))
                                    .into_widget()
                                };
                                Directionality::new(
                                    TextDirection::Ltr,
                                    Scaffold::new().body(
                                        MenuAnchor::new(vec![text("Text 1"), text("Text 2")])
                                            .style(MenuStyle::new().padding(Rc::new(
                                                WidgetStatePropertyAll::new(Some(
                                                    EdgeInsetsGeometry::symmetric(12.0, 4.0),
                                                )),
                                            )))
                                            .builder(|_app, _context, controller, _child| {
                                                TextButton::new(
                                                    Some(Listener::new(move |app| {
                                                        if controller.is_open(app) {
                                                            controller.close(app);
                                                        } else {
                                                            controller.open(app, None);
                                                        }
                                                    })),
                                                    Some(Text::new("OPEN MENU")),
                                                )
                                                .into_widget()
                                            }),
                                    ),
                                )
                                .into_widget()
                            },
                        );

                        tester.tap(find::text("OPEN MENU"));
                        pump(tester);
                    };

                // Pump widget with standard visual density.
                build_simple_menu_anchor(tester, VisualDensity::STANDARD);

                let (top_standard, bottom_standard) = menu_padding(tester);

                // Pump widget with compact visual density.
                build_simple_menu_anchor(tester, VisualDensity::COMPACT);

                let (top_compact, bottom_compact) = menu_padding(tester);

                // Compare standard vs compact padding.
                assert_eq!(
                    top_compact, top_standard,
                    "Compact visual density should not change top padding. Standard: \
                     {top_standard}, Compact: {top_compact}"
                );
                assert_eq!(
                    bottom_compact, bottom_standard,
                    "Compact visual density should not change bottom padding. Standard: \
                     {bottom_standard}, Compact: {bottom_compact}"
                );
            });
        }

        // ---- LocalizedShortcutLabeler ----

        const ALL_PLATFORMS: [TargetPlatform; 6] = [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS,
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ];

        /// A menu bar of one submenu, "Menu 0", whose items show `shortcuts`, opened.
        fn open_shortcut_menu(
            tester: &mut WidgetTester,
            shortcuts: [(MenuSerializableShortcutRef, TestMenu); 2],
        ) {
            let controller = new_controller(tester);
            pump_material(tester, m3, move |_app| {
                let items = shortcuts
                    .iter()
                    .map(|(shortcut, menu)| {
                        MenuItemButton::new()
                            .shortcut(Rc::clone(shortcut))
                            .child(Text::new(menu.label()))
                            .into_widget()
                    })
                    .collect();
                MenuBar::new(vec![
                    SubmenuButton::new(items, Some(Text::new(TestMenu::MainMenu0.label())))
                        .into_widget(),
                ])
                .controller(controller)
                .into_widget()
            });
            tester.tap(find::text(&TestMenu::MainMenu0.label()));
            pump(tester);
        }

        // menu_anchor_test.dart 'getShortcutLabel returns the right labels'.
        #[test]
        fn shortcut_labels_name_the_modifiers_of_each_platform() {
            for platform in ALL_PLATFORMS {
                test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                    let (
                        expected_meta,
                        expected_ctrl,
                        expected_alt,
                        expected_separator,
                        expected_shift,
                    ) = match platform {
                        TargetPlatform::Android
                        | TargetPlatform::Fuchsia
                        | TargetPlatform::Linux
                        | TargetPlatform::Windows => (
                            if platform == TargetPlatform::Windows {
                                "Win"
                            } else {
                                "Meta"
                            },
                            "Ctrl",
                            "Alt",
                            "+",
                            "Shift",
                        ),
                        TargetPlatform::IOS | TargetPlatform::MacOS => ("⌘", "⌃", "⌥", " ", "⇧"),
                    };

                    let all_modifiers: MenuSerializableShortcutRef = Rc::new(
                        SingleActivator::new(LogicalKeyboardKey::KEY_A)
                            .control(true)
                            .meta(true)
                            .shift(true)
                            .alt(true),
                    );
                    let all_expected = match platform {
                        TargetPlatform::Android
                        | TargetPlatform::Fuchsia
                        | TargetPlatform::Linux
                        | TargetPlatform::Windows => [
                            expected_alt,
                            expected_ctrl,
                            expected_meta,
                            expected_shift,
                            "A",
                        ]
                        .join(expected_separator),
                        TargetPlatform::IOS | TargetPlatform::MacOS => [
                            expected_ctrl,
                            expected_alt,
                            expected_shift,
                            expected_meta,
                            "A",
                        ]
                        .join(expected_separator),
                    };
                    let char_shortcuts: MenuSerializableShortcutRef =
                        Rc::new(CharacterActivator::new("ñ"));
                    let char_expected = "ñ";
                    open_shortcut_menu(
                        tester,
                        [
                            (all_modifiers, TestMenu::SubMenu10),
                            (char_shortcuts, TestMenu::SubMenu11),
                        ],
                    );

                    assert!(shows(tester, &all_expected), "{platform:?}");
                    assert!(shows(tester, char_expected), "{platform:?}");
                });
            }
        }

        // menu_anchor_test.dart 'getShortcutLabel formats unlocalized non-Unicode keys'.
        #[test]
        fn shortcut_labels_fall_back_to_the_key_label_of_unlocalized_keys() {
            for platform in ALL_PLATFORMS {
                test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                    let (expected_ctrl, expected_separator) = match platform {
                        TargetPlatform::Android
                        | TargetPlatform::Fuchsia
                        | TargetPlatform::Linux
                        | TargetPlatform::Windows => ("Ctrl", "+"),
                        TargetPlatform::IOS | TargetPlatform::MacOS => ("⌃", " "),
                    };

                    open_shortcut_menu(
                        tester,
                        [
                            (
                                Rc::new(
                                    SingleActivator::new(LogicalKeyboardKey::TAB).control(true),
                                ),
                                TestMenu::SubMenu10,
                            ),
                            (
                                Rc::new(SingleActivator::new(LogicalKeyboardKey::F1).control(true)),
                                TestMenu::SubMenu11,
                            ),
                        ],
                    );

                    assert!(
                        shows(tester, &format!("{expected_ctrl}{expected_separator}Tab")),
                        "{platform:?}"
                    );
                    assert!(
                        shows(tester, &format!("{expected_ctrl}{expected_separator}F1")),
                        "{platform:?}"
                    );
                });
            }
        }

        // ---- CheckboxMenuButton ----

        // menu_anchor_test.dart 'tapping toggles checkbox'.
        #[test]
        fn tapping_a_checkbox_menu_button_cycles_its_tristate_value() {
            test_widgets(|tester| {
                let check_box_value: Rc<Cell<Option<bool>>> = Rc::new(Cell::new(None));
                let value = Rc::clone(&check_box_value);
                pump_app(tester, m3, move |_app| {
                    StatefulBuilder::new(move |_app, _context, set_state| {
                        let changed = Rc::clone(&value);
                        MenuBar::new(vec![
                            SubmenuButton::new(
                                vec![
                                    CheckboxMenuButton::new(
                                        value.get(),
                                        Some(Rc::new(
                                            move |app: &mut App, new_value: Option<bool>| {
                                                let changed = Rc::clone(&changed);
                                                set_state(app, &mut |_app| changed.set(new_value));
                                            },
                                        )),
                                        Some(Text::new("checkbox")),
                                    )
                                    .tristate(true)
                                    .into_widget(),
                                ],
                                Some(Text::new("submenu")),
                            )
                            .into_widget(),
                        ])
                        .into_widget()
                    })
                    .into_widget()
                });

                let submenu = center_of_first::<SubmenuButton>(tester);
                tester.tap_at(submenu, 1);
                pump(tester);

                let checkbox = tester.element(find::by_type::<CheckboxMenuButton>());
                assert_eq!(
                    tester.widget(checkbox, |button: &CheckboxMenuButton| button.value),
                    None
                );

                for expected in [Some(false), Some(true), None] {
                    if expected != Some(false) {
                        let submenu = center_of_first::<SubmenuButton>(tester);
                        tester.tap_at(submenu, 1);
                        pump(tester);
                    }
                    let checkbox = center_of_first::<CheckboxMenuButton>(tester);
                    tester.tap_at(checkbox, 1);
                    tester.pump_and_settle();
                    assert_eq!(check_box_value.get(), expected);
                }
            });
        }

        // ---- RadioMenuButton ----

        // menu_anchor_test.dart 'tapping toggles radio button'.
        #[test]
        fn tapping_a_toggleable_radio_menu_button_selects_and_clears_it() {
            test_widgets(|tester| {
                let radio_value: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));
                let value = Rc::clone(&radio_value);
                pump_app(tester, m3, move |_app| {
                    StatefulBuilder::new(move |_app, _context, set_state| {
                        let radio = |radio: i32| {
                            let changed = Rc::clone(&value);
                            let set_state = Rc::clone(&set_state);
                            RadioMenuButton::new(
                                radio,
                                value.get(),
                                Some(Rc::new(move |app: &mut App, new_value: Option<i32>| {
                                    let changed = Rc::clone(&changed);
                                    set_state(app, &mut |_app| changed.set(new_value));
                                })),
                                Some(Text::new(format!("radio {radio}"))),
                            )
                            .toggleable(true)
                            .into_widget()
                        };
                        MenuBar::new(vec![
                            SubmenuButton::new(
                                vec![radio(0), radio(1)],
                                Some(Text::new("submenu")),
                            )
                            .into_widget(),
                        ])
                        .into_widget()
                    })
                    .into_widget()
                });

                let submenu = center_of_first::<SubmenuButton>(tester);
                tester.tap_at(submenu, 1);
                pump(tester);

                let first = find::by_type::<RadioMenuButton<i32>>().evaluate(tester)[0];
                assert_eq!(
                    tester.widget(first, |button: &RadioMenuButton<i32>| button.group_value),
                    None
                );

                let first = center_of_first::<RadioMenuButton<i32>>(tester);
                tester.tap_at(first, 1);
                tester.pump_and_settle();
                assert_eq!(radio_value.get(), Some(0));

                let submenu = center_of_first::<SubmenuButton>(tester);
                tester.tap_at(submenu, 1);
                pump(tester);
                let first = center_of_first::<RadioMenuButton<i32>>(tester);
                tester.tap_at(first, 1);
                tester.pump_and_settle();
                assert_eq!(radio_value.get(), None);

                let submenu = center_of_first::<SubmenuButton>(tester);
                tester.tap_at(submenu, 1);
                pump(tester);
                let last = center_of_last::<RadioMenuButton<i32>>(tester);
                tester.tap_at(last, 1);
                tester.pump_and_settle();
                assert_eq!(radio_value.get(), Some(1));
            });
        }

        // ---- the cases after the Semantics group ----

        /// Dart's `find.descendant(of: of, matching: find.byType(Material)).first`.
        fn first_material_below(tester: &WidgetTester, of: AnyElement) -> AnyElement {
            tester.descendants_of_type::<Material>(of)[0]
        }

        /// Dart's `find.widgetWithText(TextButton, text)`.
        fn text_button_with_text(tester: &WidgetTester, text: &str) -> AnyElement {
            let buttons = ancestors_of_type::<TextButton>(tester, text);
            *buttons.first().expect("a TextButton above the text")
        }

        // menu_anchor_test.dart 'Material3 - Menu uses correct text styles'.
        #[test]
        fn material3_menu_buttons_use_the_label_large_text_style() {
            test_widgets(|tester| {
                let log = Log::default();
                let controller = new_controller(tester);
                let menu_text_style = TextStyle::new()
                    .font_size(18.5)
                    .font_style(FontStyle::Italic)
                    .word_spacing(1.2)
                    .decoration(TextDecoration::LINE_THROUGH);
                let style = menu_text_style.clone();
                let menus = TestMenus::all(&log);
                pump_material(
                    tester,
                    move |app| {
                        ThemeData::builder()
                            .text_theme(TextTheme::new().label_large(style))
                            .build(app)
                    },
                    |_app| {
                        MenuBar::new(menus.build())
                            .controller(controller)
                            .into_widget()
                    },
                );

                let check = |tester: &WidgetTester, button: AnyElement| {
                    let material = first_material_below(tester, button);
                    tester.widget(material, |material: &Material| {
                        let text_style = material.text_style.as_ref().expect("a text style");
                        assert_eq!(text_style.font_size, menu_text_style.font_size);
                        assert_eq!(text_style.font_style, menu_text_style.font_style);
                        assert_eq!(text_style.word_spacing, menu_text_style.word_spacing);
                        assert_eq!(text_style.decoration, menu_text_style.decoration);
                    });
                };

                // Test menu button text style uses the TextTheme.labelLarge.
                check(tester, find::by_type::<TextButton>().evaluate(tester)[0]);

                // Open the menu.
                tester.tap(find::text(&TestMenu::MainMenu1.label()));
                pump(tester);

                // Test menu item text style uses the TextTheme.labelLarge.
                check(
                    tester,
                    text_button_with_text(tester, &TestMenu::SubMenu10.label()),
                );
            });
        }

        // menu_anchor_test.dart 'SubmenuButton.onFocusChange is respected'.
        #[test]
        fn a_submenu_button_reports_its_focus_changes() {
            test_widgets(|tester| {
                let focus_node =
                    FocusNode::new(&mut tester.cell.borrow_mut()).into_erased::<AnyFocusNode>();
                let on_focus_change_called = Rc::new(Cell::new(0));
                let called = Rc::clone(&on_focus_change_called);
                pump_material(tester, m3, {
                    let focus_node = focus_node.clone();
                    move |_app| {
                        StatefulBuilder::new(move |_app, _context, set_state| {
                            let called = Rc::clone(&called);
                            SubmenuButton::new(
                                vec![
                                    MenuItemButton::new()
                                        .child(Text::new("item 0"))
                                        .into_widget(),
                                ],
                                Some(Text::new("Submenu 0")),
                            )
                            .focus_node(focus_node.clone())
                            .on_focus_change(Rc::new(move |app: &mut App, _value: bool| {
                                let called = Rc::clone(&called);
                                set_state(app, &mut |_app| called.set(called.get() + 1));
                            }))
                            .into_widget()
                        })
                        .into_widget()
                    }
                });

                focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
                pump(tester);
                assert!(focus_node.has_focus(&mut tester.cell.borrow_mut()));
                assert_eq!(on_focus_change_called.get(), 1);

                focus_node.unfocus(&mut tester.cell.borrow_mut(), UnfocusDisposition::Scope);
                pump(tester);
                assert!(!focus_node.has_focus(&mut tester.cell.borrow_mut()));
                assert_eq!(on_focus_change_called.get(), 2);
            });
        }

        // menu_anchor_test.dart 'Horizontal _MenuPanel wraps children with IntrinsicWidth'.
        #[test]
        fn a_horizontal_menu_panel_wraps_its_children_in_intrinsic_widths() {
            test_widgets(|tester| {
                pump_material(tester, m3, |_app| {
                    MenuBar::new(vec![
                        MenuItemButton::new()
                            .on_pressed(Listener::new(|_app| {}))
                            .child(Text::new("Menu Item"))
                            .into_widget(),
                    ])
                    .into_widget()
                });

                // Horizontal _MenuPanel wraps children with IntrinsicWidth to ensure MenuItemButton
                // with vertical overflow axis is as wide as the widest child.
                let item = tester.element(find::by_type::<MenuItemButton>());
                let ancestors = tester.ancestors(item);
                let app = tester.cell.borrow();
                let intrinsic_widths = ancestors
                    .into_iter()
                    .filter(|ancestor| {
                        waft_widgets::downcast_widget::<IntrinsicWidth>(&**ancestor.widget(&app))
                            .is_some()
                    })
                    .count();
                assert_eq!(intrinsic_widths, 1);
            });
        }

        // menu_anchor_test.dart 'SubmenuButton.styleFrom overlayColor overrides default overlay
        // color'.
        #[test]
        fn submenu_button_style_from_overlay_color_overrides_the_default_overlay_color() {
            test_widgets(|tester| {
                let overlay_color = Color::new(0xffff00ff);
                pump_app(tester, m3, move |_app| {
                    Scaffold::new()
                        .body(
                            SubmenuButton::new(
                                vec![
                                    MenuItemButton::new()
                                        .on_pressed(Listener::new(|_app| {}))
                                        .child(Text::new("MenuItemButton"))
                                        .into_widget(),
                                ],
                                Some(Text::new("Submenu")),
                            )
                            .style(
                                SubmenuButton::style_from()
                                    .overlay_color(overlay_color)
                                    .build(),
                            ),
                        )
                        .into_widget()
                });

                // Hovered.
                let center = center_of_first::<SubmenuButton>(tester);
                let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
                tester.add_pointer(&mut gesture, Offset::ZERO);
                tester.move_to(&mut gesture, center, false);
                tester.pump_and_settle();
                assert_paints(
                    &overlay_ops(tester),
                    &[PaintCall::rect(Some(overlay_color.with_opacity(0.08)))],
                );

                // Highlighted (pressed).
                tester.down(&mut gesture, center);
                tester.pump_and_settle();
                assert_paints(
                    &overlay_ops(tester),
                    &[
                        PaintCall::rect(Some(overlay_color.with_opacity(0.08))),
                        PaintCall::rect(Some(overlay_color.with_opacity(0.08))),
                        PaintCall::rect(Some(overlay_color.with_opacity(0.1))),
                    ],
                );
            });
        }

        /// Dart's `iconStyle(tester, icon)`.
        fn icon_color(tester: &WidgetTester, icon: &waft_widgets::IconData) -> Option<Color> {
            let icon = tester.element(find::by_icon(icon).first());
            tester.glyph_style(icon).color.as_deref().copied()
        }

        // menu_anchor_test.dart 'MenuItemButton.styleFrom can customize the button icon'.
        #[test]
        fn menu_item_button_style_from_customizes_the_button_icon() {
            test_widgets(|tester| {
                let icon_color_value = Color::new(0xFFF000FF);
                const ICON_SIZE: f64 = 32.0;
                let disabled_icon_color = Color::new(0xFFFFF000);
                let build_button = |tester: &mut WidgetTester, enabled: bool| {
                    pump_material(tester, m3, move |_app| {
                        let mut button = MenuItemButton::new()
                            .style(
                                MenuItemButton::style_from()
                                    .icon_color(icon_color_value)
                                    .icon_size(ICON_SIZE)
                                    .disabled_icon_color(disabled_icon_color)
                                    .build(),
                            )
                            .trailing_icon(Icon::new(Some(Icons::add())))
                            .child(Text::new("Button"));
                        if enabled {
                            button = button.on_pressed(Listener::new(|_app| {}));
                        }
                        Center::new().child(button).into_widget()
                    });
                };

                // Test enabled button.
                build_button(tester, true);
                let icon = tester.element(find::by_icon(&Icons::add()).first());
                assert_eq!(
                    rect_of_element(tester, icon).size(),
                    Size::new(ICON_SIZE, ICON_SIZE)
                );
                assert_eq!(icon_color(tester, &Icons::add()), Some(icon_color_value));

                // Test disabled button.
                build_button(tester, false);
                assert_eq!(icon_color(tester, &Icons::add()), Some(disabled_icon_color));
            });
        }

        // menu_anchor_test.dart 'SubmenuButton.styleFrom can customize the button icon'.
        #[test]
        fn submenu_button_style_from_customizes_the_button_icon() {
            test_widgets(|tester| {
                let icon_color_value = Color::new(0xFFF000FF);
                const ICON_SIZE: f64 = 32.0;
                let disabled_icon_color = Color::new(0xFFFFF000);
                let build_button = |tester: &mut WidgetTester, enabled: bool| {
                    pump_material(tester, m3, move |_app| {
                        let menu_children = if enabled {
                            vec![Text::new("Item").into_widget()]
                        } else {
                            Vec::new()
                        };
                        Center::new()
                            .child(
                                SubmenuButton::new(menu_children, Some(Text::new("SubmenuButton")))
                                    .style(
                                        SubmenuButton::style_from()
                                            .icon_color(icon_color_value)
                                            .icon_size(ICON_SIZE)
                                            .disabled_icon_color(disabled_icon_color)
                                            .build(),
                                    )
                                    .trailing_icon(Icon::new(Some(Icons::add()))),
                            )
                            .into_widget()
                    });
                };

                // Test enabled button.
                build_button(tester, true);
                let icon = tester.element(find::by_icon(&Icons::add()).first());
                assert_eq!(
                    rect_of_element(tester, icon).size(),
                    Size::new(ICON_SIZE, ICON_SIZE)
                );
                assert_eq!(icon_color(tester, &Icons::add()), Some(icon_color_value));

                // Test disabled button.
                build_button(tester, false);
                assert_eq!(icon_color(tester, &Icons::add()), Some(disabled_icon_color));
            });
        }

        /// Dart's `ancestorRenderTheaters(child)`: the theaters above `child`, nearest first.
        fn ancestor_render_theaters(
            tester: &WidgetTester,
            child: AnyRenderObject,
        ) -> Vec<AnyRenderObject> {
            let app = tester.cell.borrow();
            std::iter::successors(Some(child), |node| node.parent(&app))
                .filter(|node| node.downcast::<RenderTheater>(&app).is_some())
                .collect()
        }

        /// Dart's boilerplate of the useRootOverlay cases: a menu in an entry of a nested `Overlay`
        /// keyed `overlay_key`, its one item keyed `menu_item_key`.
        fn nested_overlay_menu(
            tester: &mut WidgetTester,
            controller: MenuControllerRef,
            use_root_overlay: bool,
            overlay_key: &GlobalKey,
            menu_item_key: &GlobalKey,
        ) {
            let (overlay_key, menu_item_key) = (overlay_key.clone(), menu_item_key.clone());
            pump_app(tester, m3, move |app| {
                let entry = OverlayEntry::new(
                    app,
                    Rc::new(move |_app, _context| {
                        Scaffold::new()
                            .body(
                                Center::new().child(
                                    MenuAnchor::new(vec![
                                        MenuItemButton::new()
                                            .key(menu_item_key.clone().into())
                                            .on_pressed(Listener::new(|_app| {}))
                                            .child(Text::new("Item 1"))
                                            .into_widget(),
                                    ])
                                    .use_root_overlay(use_root_overlay)
                                    .controller(controller.clone()),
                                ),
                            )
                            .into_widget()
                    }),
                    false,
                    false,
                    false,
                );
                Overlay::new()
                    .key(overlay_key.into())
                    .initial_entries([entry])
                    .into_widget()
            });
        }

        /// The render object of every `Overlay`: Dart's `tester.renderObject(find.byWidget(o))`.
        fn overlay_render_objects(tester: &WidgetTester) -> Vec<AnyRenderObject> {
            let overlays = find::by_type::<Overlay>().evaluate(tester);
            let app = tester.cell.borrow();
            overlays
                .into_iter()
                .map(|overlay| {
                    overlay
                        .find_render_object(&app)
                        .expect("an overlay's theater")
                })
                .collect()
        }

        // menu_anchor_test.dart 'Content is shown in the root overlay when useRootOverlay is
        // true'.
        #[test]
        fn a_menu_using_the_root_overlay_shows_in_the_root_overlay() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let overlay_key = GlobalKey::new();
                let menu_item_key = GlobalKey::new();
                nested_overlay_menu(
                    tester,
                    controller.clone(),
                    true,
                    &overlay_key,
                    &menu_item_key,
                );
                assert!(
                    menu_item_key
                        .current_context(&tester.cell.borrow())
                        .is_none()
                );

                // Open the menu.
                controller.open(&mut tester.cell.borrow_mut(), None);
                pump(tester);
                assert!(
                    menu_item_key
                        .current_context(&tester.cell.borrow())
                        .is_some()
                );

                // Expect two overlays: the root overlay created by MaterialApp and the
                // overlay created by the boilerplate code.
                let overlays = overlay_render_objects(tester);
                assert_eq!(overlays.len(), 2);
                let non_root_overlay = tester.render_object(&overlay_key);
                let root_overlay = *overlays
                    .iter()
                    .find(|overlay| **overlay != non_root_overlay)
                    .expect("the root overlay");

                // Check that the ancestor _RenderTheater for the menu item is the one
                // from the root overlay.
                let theaters =
                    ancestor_render_theaters(tester, tester.render_object(&menu_item_key));
                assert_eq!(theaters, [root_overlay]);
            });
        }

        // menu_anchor_test.dart 'Content is shown in the nearest ancestor overlay when
        // useRootOverlay is false'.
        #[test]
        fn a_menu_not_using_the_root_overlay_shows_in_the_nearest_overlay() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let overlay_key = GlobalKey::new();
                let menu_item_key = GlobalKey::new();
                nested_overlay_menu(
                    tester,
                    controller.clone(),
                    false,
                    &overlay_key,
                    &menu_item_key,
                );
                assert!(
                    menu_item_key
                        .current_context(&tester.cell.borrow())
                        .is_none()
                );

                // Open the menu.
                controller.open(&mut tester.cell.borrow_mut(), None);
                pump(tester);
                assert!(
                    menu_item_key
                        .current_context(&tester.cell.borrow())
                        .is_some()
                );

                // Expect two overlays: the root overlay created by MaterialApp and the
                // overlay created by the boilerplate code.
                assert_eq!(overlay_render_objects(tester).len(), 2);
                let non_root_overlay = tester.render_object(&overlay_key);

                // Check that the ancestor _RenderTheater for the menu item is the one
                // from the root overlay.
                let theaters =
                    ancestor_render_theaters(tester, tester.render_object(&menu_item_key));
                assert_eq!(theaters.first(), Some(&non_root_overlay));
            });
        }

        // menu_anchor_test.dart 'Unattached MenuController does not throw when calling close'.
        #[test]
        fn closing_an_unattached_menu_controller_does_not_panic() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                controller.close(&mut tester.cell.borrow_mut());
                pump(tester);
            });
        }

        // menu_anchor_test.dart 'Unattached MenuController returns false when calling isOpen'.
        #[test]
        fn an_unattached_menu_controller_is_not_open() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                assert!(!controller.is_open(&tester.cell.borrow()));
            });
        }

        // menu_anchor_test.dart 'MenuAnchor updates isOpen state correctly'.
        #[test]
        fn a_menu_anchor_rebuilds_its_button_with_the_controllers_is_open() {
            test_widgets(|tester| {
                let is_open = Rc::new(Cell::new(false));
                let open_count = Rc::new(Cell::new(0));
                let close_count = Rc::new(Cell::new(0));
                let (built_open, opens, closes) = (
                    Rc::clone(&is_open),
                    Rc::clone(&open_count),
                    Rc::clone(&close_count),
                );
                pump_app(tester, m3, move |_app| {
                    Scaffold::new()
                        .body(
                            Center::new().child(
                                MenuAnchor::new(vec![
                                    MenuItemButton::new()
                                        .child(Text::new("menu item"))
                                        .into_widget(),
                                ])
                                .builder(move |app, _context, controller, _child| {
                                    built_open.set(controller.is_open(app));
                                    let label = if built_open.get() { "close" } else { "open" };
                                    toggle_button(controller, label)
                                })
                                .on_open(Rc::new(move |_app: &mut App| opens.set(opens.get() + 1)))
                                .on_close(Rc::new(
                                    move |_app: &mut App| closes.set(closes.get() + 1),
                                )),
                            ),
                        )
                        .into_widget()
                });

                assert!(shows(tester, "open"));
                assert!(!is_open.get());
                assert_eq!(open_count.get(), 0);
                assert_eq!(close_count.get(), 0);

                let button = center_of_first::<FilledButton>(tester);
                tester.tap_at(button, 1);
                pump(tester);

                assert!(shows(tester, "close"));
                assert!(is_open.get());
                assert_eq!(open_count.get(), 1);
                assert_eq!(close_count.get(), 0);

                let button = center_of_first::<FilledButton>(tester);
                tester.tap_at(button, 1);
                pump(tester);

                assert!(shows(tester, "open"));
                assert!(!is_open.get());
                assert_eq!(open_count.get(), 1);
                assert_eq!(close_count.get(), 1);
            });
        }

        /// How many `Icon`s show `icon`: Dart's `find.byIcon(icon)`.
        fn count_icon(tester: &WidgetTester, icon: &waft_widgets::IconData) -> usize {
            find::by_type::<Icon>()
                .evaluate(tester)
                .into_iter()
                .filter(|element| {
                    tester.widget(*element, |widget: &Icon| widget.icon.as_ref() == Some(icon))
                })
                .count()
        }

        // menu_anchor_test.dart 'SubmenuButton.submenuIcon updates default arrow icon'.
        #[test]
        fn a_submenu_icon_resolves_against_the_buttons_states() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let disabled_icon = Icons::close();
                let hovered_icon = Icons::bolt();
                let focused_icon = Icons::favorite();
                let default_icon = Icons::add();
                let submenu_icon: WidgetStatePropertyRef<Option<WidgetRef>> = {
                    let (disabled_icon, hovered_icon, focused_icon, default_icon) = (
                        disabled_icon.clone(),
                        hovered_icon.clone(),
                        focused_icon.clone(),
                        default_icon.clone(),
                    );
                    <dyn waft_widgets::WidgetStateProperty<Option<WidgetRef>>>::resolve_with(
                        move |states| {
                            let icon = if states.contains(&WidgetState::Disabled) {
                                &disabled_icon
                            } else if states.contains(&WidgetState::Hovered) {
                                &hovered_icon
                            } else if states.contains(&WidgetState::Focused) {
                                &focused_icon
                            } else {
                                &default_icon
                            };
                            Some(Icon::new(Some(icon.clone())).into_widget())
                        },
                    )
                };

                let build_menu = |tester: &mut WidgetTester,
                                  icon: Option<WidgetStatePropertyRef<Option<WidgetRef>>>,
                                  enabled: bool| {
                    pump_material(tester, m3, {
                        let controller = controller.clone();
                        move |_app| {
                            let menu_children = if enabled {
                                vec![
                                    MenuItemButton::new()
                                        .child(Text::new(TestMenu::MainMenu0.label()))
                                        .into_widget(),
                                ]
                            } else {
                                Vec::new()
                            };
                            let mut inner = SubmenuButton::new(
                                menu_children,
                                Some(Text::new(TestMenu::SubSubMenu110.label())),
                            );
                            inner.submenu_icon = icon;
                            MenuBar::new(vec![
                                SubmenuButton::new(
                                    vec![inner.into_widget()],
                                    Some(Text::new(TestMenu::SubMenu00.label())),
                                )
                                .into_widget(),
                            ])
                            .controller(controller.clone())
                            .into_widget()
                        }
                    });
                };

                build_menu(tester, None, true);
                tester.tap(find::text(&TestMenu::SubMenu00.label()));
                pump(tester);

                assert_eq!(count_icon(tester, &Icons::arrow_right()), 1);

                controller.close(&mut tester.cell.borrow_mut());
                pump(tester);

                build_menu(tester, Some(Rc::clone(&submenu_icon)), true);
                tester.tap(find::text(&TestMenu::SubMenu00.label()));
                pump(tester);
                assert_eq!(count_icon(tester, &default_icon), 1);

                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                pump(tester);
                assert_eq!(count_icon(tester, &focused_icon), 1);

                controller.close(&mut tester.cell.borrow_mut());
                pump(tester);

                tester.tap(find::text(&TestMenu::SubMenu00.label()));
                pump(tester);
                let at = tester
                    .get_rect(find::text(&TestMenu::SubSubMenu110.label()))
                    .center();
                hover_over(tester, at);
                pump(tester);
                assert_eq!(count_icon(tester, &hovered_icon), 1);

                controller.close(&mut tester.cell.borrow_mut());
                pump(tester);

                build_menu(tester, Some(Rc::clone(&submenu_icon)), false);
                tester.tap(find::text(&TestMenu::SubMenu00.label()));
                pump(tester);
                assert_eq!(count_icon(tester, &disabled_icon), 1);
            });
        }

        // ---- Mouse cursors ----

        fn shows_cursor(tester: &WidgetTester, cursor: SystemMouseCursor) -> bool {
            let cursor: MouseCursorRef = cursor.into();
            *tester.active_cursor(1) == *cursor
        }

        /// Dart's mouse gesture added at (1000, 1000), checked to show the basic cursor there,
        /// then moved to `at`.
        fn hover_from_outside(tester: &mut WidgetTester, at: impl FnOnce(&WidgetTester) -> Offset) {
            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::new(1000.0, 1000.0));

            assert!(shows_cursor(tester, SystemMouseCursors::BASIC));

            let at = at(tester);
            tester.move_to(&mut gesture, at, false);
            pump(tester);
        }

        // menu_anchor_test.dart 'SubmenuButton has expected default mouse cursor on hover'.
        #[test]
        fn a_submenu_button_shows_the_basic_cursor_by_default() {
            test_widgets(|tester| {
                pump_material(tester, m3, |_app| {
                    SubmenuButton::new(
                        vec![
                            MenuItemButton::new()
                                .on_pressed(Listener::new(|_app| {}))
                                .child(Text::new("Test menu button item"))
                                .into_widget(),
                        ],
                        Some(Text::new("Main Menu")),
                    )
                    .into_widget()
                });

                hover_from_outside(tester, center_of_first::<SubmenuButton>);

                assert!(shows_cursor(tester, SystemMouseCursors::BASIC));
            });
        }

        // menu_anchor_test.dart 'MenuItemButton has expected default mouse cursor on hover'.
        #[test]
        fn a_menu_item_button_shows_the_basic_cursor_by_default() {
            test_widgets(|tester| {
                pump_material(tester, m3, |_app| {
                    MenuBar::new(vec![
                        SubmenuButton::new(
                            vec![
                                MenuItemButton::new()
                                    .on_pressed(Listener::new(|_app| {}))
                                    .child(Text::new("Test menu item"))
                                    .into_widget(),
                            ],
                            Some(Text::new("File")),
                        )
                        .into_widget(),
                    ])
                    .into_widget()
                });

                // Open SubmenuButton.
                tester.tap(find::text("File"));
                tester.pump_and_settle();

                // Move to MenuItemButton.
                hover_from_outside(tester, center_of_first::<MenuItemButton>);

                assert!(shows_cursor(tester, SystemMouseCursors::BASIC));
            });
        }

        // menu_anchor_test.dart 'CheckboxMenuButton has expected default mouse cursor on hover'.
        #[test]
        fn a_checkbox_menu_button_shows_the_basic_cursor_by_default() {
            test_widgets(|tester| {
                pump_material(tester, m3, |_app| {
                    CheckboxMenuButton::new(
                        Some(false),
                        Some(Rc::new(|_app: &mut App, _value: Option<bool>| {})),
                        Some(Text::new("Checkbox")),
                    )
                    .into_widget()
                });

                hover_from_outside(tester, center_of_first::<CheckboxMenuButton>);

                assert!(shows_cursor(tester, SystemMouseCursors::BASIC));
            });
        }

        // menu_anchor_test.dart 'RadioMenuButton has expected default mouse cursor on hover'.
        #[test]
        fn a_radio_menu_button_shows_the_basic_cursor_by_default() {
            test_widgets(|tester| {
                pump_material(tester, m3, |_app| {
                    RadioMenuButton::new(
                        1,
                        Some(0),
                        Some(Rc::new(|_app: &mut App, _value: Option<i32>| {})),
                        Some(Text::new("Radio")),
                    )
                    .into_widget()
                });

                hover_from_outside(tester, center_of_first::<RadioMenuButton<i32>>);

                assert!(shows_cursor(tester, SystemMouseCursors::BASIC));
            });
        }

        /// Dart's `ButtonStyle(mouseCursor: WidgetStateProperty.all(SystemMouseCursors.cell))`.
        fn cell_cursor_style() -> ButtonStyle {
            ButtonStyle::new().mouse_cursor(Rc::new(MaterialStatePropertyAll::new(Some(
                MouseCursorRef::from(SystemMouseCursors::CELL),
            ))))
        }

        /// Dart's mouse gesture added at `at`.
        fn add_mouse_at(tester: &mut WidgetTester, at: Offset) {
            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, at);
        }

        // menu_anchor_test.dart 'MenuItemButton has expected mouse cursor when explicitly
        // configured'.
        #[test]
        fn a_menu_item_button_shows_the_cursor_of_its_style() {
            test_widgets(|tester| {
                pump_material(tester, m3, |_app| {
                    MenuItemButton::new()
                        .on_pressed(Listener::new(|_app| {}))
                        .style(cell_cursor_style())
                        .child(Text::new("Menu Item"))
                        .into_widget()
                });

                let at = center_of_first::<MenuItemButton>(tester);
                add_mouse_at(tester, at);

                assert!(shows_cursor(tester, SystemMouseCursors::CELL));
            });
        }

        // menu_anchor_test.dart 'CheckboxMenuButton has expected mouse cursor when explicitly
        // configured'.
        #[test]
        fn a_checkbox_menu_button_shows_the_cursor_of_its_style() {
            test_widgets(|tester| {
                pump_material(tester, m3, |_app| {
                    CheckboxMenuButton::new(
                        Some(true),
                        Some(Rc::new(|_app: &mut App, _value: Option<bool>| {})),
                        Some(Text::new("Menu Item")),
                    )
                    .style(cell_cursor_style())
                    .into_widget()
                });

                let at = center_of_first::<CheckboxMenuButton>(tester);
                add_mouse_at(tester, at);

                assert!(shows_cursor(tester, SystemMouseCursors::CELL));
            });
        }

        // menu_anchor_test.dart 'RadioMenuButton has expected mouse cursor when explicitly
        // configured'.
        #[test]
        fn a_radio_menu_button_shows_the_cursor_of_its_style() {
            test_widgets(|tester| {
                pump_material(tester, m3, |_app| {
                    RadioMenuButton::new(
                        false,
                        None,
                        Some(Rc::new(|_app: &mut App, _value: Option<bool>| {})),
                        Some(Text::new("Menu Item")),
                    )
                    .style(cell_cursor_style())
                    .into_widget()
                });

                let at = center_of_first::<RadioMenuButton<bool>>(tester);
                add_mouse_at(tester, at);

                assert!(shows_cursor(tester, SystemMouseCursors::CELL));
            });
        }

        // menu_anchor_test.dart 'SubmenuButton has expected mouse cursor when explicitly
        // configured'.
        #[test]
        fn a_submenu_button_shows_the_cursor_of_its_style() {
            test_widgets(|tester| {
                pump_material(tester, m3, |_app| {
                    MenuBar::new(vec![
                        SubmenuButton::new(
                            vec![
                                MenuItemButton::new()
                                    .on_pressed(Listener::new(|_app| {}))
                                    .child(Text::new("Test menu item"))
                                    .into_widget(),
                            ],
                            Some(Text::new("File")),
                        )
                        .style(cell_cursor_style())
                        .into_widget(),
                    ])
                    .into_widget()
                });

                let at = center_of_first::<SubmenuButton>(tester);
                add_mouse_at(tester, at);

                assert!(shows_cursor(tester, SystemMouseCursors::CELL));
            });
        }
    }

    /// menu_anchor_test.dart's 'Animations' group and the top-level cases after it.
    mod animations {
        use std::cell::Cell;

        use waft_widgets::{Center, Column, SizedBox};

        use super::*;
        use crate::dialog::{AlertDialog, ShowDialog};
        use crate::scaffold::Scaffold;

        fn new_controller(tester: &WidgetTester) -> MenuControllerRef {
            MenuController::new(&mut tester.cell.borrow_mut()).into_erased()
        }

        fn open(
            tester: &mut WidgetTester,
            controller: AnyMenuController,
            position: Option<Offset>,
        ) {
            controller.open(&mut tester.cell.borrow_mut(), position);
            tester.cell.checkpoint();
        }

        fn close(tester: &mut WidgetTester, controller: AnyMenuController) {
            controller.close(&mut tester.cell.borrow_mut());
            tester.cell.checkpoint();
        }

        fn pump_ms(tester: &mut WidgetTester, ms: u64) {
            tester.pump_by(Duration::from_millis(ms));
        }

        fn new_focus_node(tester: &WidgetTester, label: Option<&str>) -> FocusNodeRef {
            let mut app = tester.cell.borrow_mut();
            let node = FocusNode::new(&mut app).into_erased::<AnyFocusNode>();
            if let Some(label) = label {
                node.set_debug_label(&mut app, Some(label.to_owned()));
            }
            node
        }

        fn request_focus(tester: &mut WidgetTester, node: AnyFocusNode) {
            node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.cell.checkpoint();
        }

        fn has_focus(tester: &WidgetTester, node: AnyFocusNode) -> bool {
            node.has_focus(&mut tester.cell.borrow_mut())
        }

        fn has_primary_focus(tester: &WidgetTester, node: AnyFocusNode) -> bool {
            node.has_primary_focus(&tester.cell.borrow())
        }

        /// Dart's `MenuItemButton(onPressed: () {}, child: Text(text))`.
        fn item(text: &str) -> WidgetRef {
            MenuItemButton::new()
                .on_pressed(Listener::new(|_app| {}))
                .child(Text::new(text))
                .into_widget()
        }

        fn items(texts: &[&str]) -> Vec<WidgetRef> {
            texts.iter().map(|text| item(text)).collect()
        }

        /// Dart's `hoverOver`, whose `tester.createGesture` gives each gesture a pointer and a
        /// device of its own.
        fn hover_with_new_mouse(tester: &mut WidgetTester, device: i64, at: Offset) {
            let mut mouse = TestPointer::mouse(device, 0);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            tester.move_to(&mut mouse, at, false);
            tester.pump();
        }

        /// The value of a `FadeTransition`'s opacity.
        fn opacity(tester: &WidgetTester, fade: AnyElement) -> f64 {
            let animation = tester.widget::<FadeTransition, _>(fade, |fade| fade.opacity.clone());
            animation.value(&tester.cell.borrow())
        }

        /// The nearest of `of`'s ancestors whose widget is a `FadeTransition`.
        fn nearest_fade(tester: &WidgetTester, of: AnyElement) -> AnyElement {
            tester.ancestor_of_type::<FadeTransition>(of)
        }

        /// Dart's `firstWidget<FadeTransition>(find.ancestor(of: find.widgetWithText(MenuItemButton,
        /// text), matching: find.byType(FadeTransition))).opacity.value`.
        fn item_opacity(tester: &WidgetTester, text: &str) -> f64 {
            let button = ancestors_of_type::<MenuItemButton>(tester, text)[0];
            opacity(tester, nearest_fade(tester, button))
        }

        /// Dart's `firstWidget<FadeTransition>(find.widgetWithText(FadeTransition, text))
        /// .opacity.value`.
        fn text_fade_opacity(tester: &WidgetTester, text: &str) -> f64 {
            let fade = ancestors_of_type::<FadeTransition>(tester, text)[0];
            opacity(tester, fade)
        }

        /// The opacity of the `FadeTransition` nearest above the menu panel.
        fn panel_opacity(tester: &WidgetTester) -> f64 {
            let panel = only(menu_panels(tester));
            opacity(tester, nearest_fade(tester, panel))
        }

        /// Dart's `tester.getRect(find.descendant(of: find.byType(MenuAnchor), matching:
        /// find.byType(FocusScope)))`: the menu's `FocusScope`.
        fn panel_rect(tester: &WidgetTester) -> Rect {
            let anchor = tester.element(find::by_type::<MenuAnchor>());
            let scope = only(tester.descendants_of_type::<FocusScope>(anchor));
            rect_of_element(tester, scope)
        }

        /// Dart's `MenuAnchor(controller: controller, animated: true, menuChildren: children)` as a
        /// `MaterialApp`'s `Material` home.
        fn pump_animated_anchor(
            tester: &mut WidgetTester,
            controller: MenuControllerRef,
            children: Vec<WidgetRef>,
        ) {
            pump_material(tester, m3, |_app| {
                MenuAnchor::new(children)
                    .controller(controller.clone())
                    .animated(true)
                    .into_widget()
            });
        }

        /// Dart's `MenuStyle(padding: WidgetStatePropertyAll(EdgeInsets.zero))`.
        fn zero_padding() -> MenuStyle {
            MenuStyle::new().padding(Rc::new(waft_widgets::WidgetStatePropertyAll::new(Some(
                EdgeInsetsGeometry::ZERO,
            ))))
        }

        /// A slot an `onAnimationStatusChanged` writes to.
        fn status_sink() -> (
            Rc<Cell<Option<AnimationStatus>>>,
            ValueChanged<AnimationStatus>,
        ) {
            let status = Rc::new(Cell::new(None));
            let sink = Rc::clone(&status);
            (
                status,
                Rc::new(move |_app: &mut App, value| sink.set(Some(value))),
            )
        }

        /// Dart's `TextButton(onPressed: () => controller.isOpen ? close : open, child: Text(text))`
        /// builder of a `MenuAnchor`.
        fn toggle_button(
            text: &'static str,
        ) -> impl Fn(&mut App, BuildContext, AnyMenuController, Option<&WidgetRef>) -> WidgetRef + 'static
        {
            move |_app, _context, controller, _child| {
                TextButton::new(
                    Some(Listener::new(move |app| {
                        if controller.is_open(app) {
                            controller.close(app);
                        } else {
                            controller.open(app, None);
                        }
                    })),
                    Some(Text::new(text)),
                )
                .into_widget()
            }
        }

        // menu_anchor_test.dart 'Animations can be disabled'.
        #[test]
        fn animations_can_be_disabled() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_animated_anchor(tester, controller.clone(), items(&["Item 0"]));

                // Open the menu and verify animations are enabled.
                open(tester, controller.handle(), None);
                pump(tester);
                pump_ms(tester, 50);

                assert_more_or_less_equals_within(item_opacity(tester, "Item 0"), 0.2, 0.05);

                pump_material(tester, m3, |_app| {
                    MenuAnchor::new(items(&["Item 0"]))
                        .controller(controller)
                        .into_widget()
                });

                assert_eq!(item_opacity(tester, "Item 0"), 1.0);
            });
        }

        // menu_anchor_test.dart 'MenuAnchor children can be changed'.
        #[test]
        fn the_children_of_an_animated_menu_anchor_can_be_changed() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_animated_anchor(tester, controller.clone(), items(&["Item 0"]));

                open(tester, controller.handle(), None);
                pump(tester);
                pump_ms(tester, 50);

                assert_more_or_less_equals_within(item_opacity(tester, "Item 0"), 0.2, 0.05);

                pump_animated_anchor(tester, controller.clone(), items(&["Item 1"]));

                assert_more_or_less_equals_within(item_opacity(tester, "Item 1"), 0.2, 0.05);
            });
        }

        // menu_anchor_test.dart 'Menu panel fades in'.
        #[test]
        fn the_menu_panel_fades_in_over_50_ms() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_animated_anchor(
                    tester,
                    controller.clone(),
                    items(&["Item 0", "Item 1", "Item 2"]),
                );

                open(tester, controller.handle(), None);
                pump(tester);

                // Opacity values at different millisecond offsets during the 50 ms fade-in
                // animation.
                for (_ms, expected) in [
                    (0, 0.0),
                    (10, 0.2),
                    (20, 0.4),
                    (30, 0.6),
                    (40, 0.8),
                    (50, 1.0),
                ] {
                    assert_more_or_less_equals_within(panel_opacity(tester), expected, 0.05);
                    pump_ms(tester, 10);
                }
            });
        }

        // menu_anchor_test.dart 'Menu panel fades out'.
        #[test]
        fn the_menu_panel_fades_out_over_the_last_50_ms_of_closing() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_animated_anchor(
                    tester,
                    controller.clone(),
                    items(&["Item 0", "Item 1", "Item 2"]),
                );

                open(tester, controller.handle(), None);
                pump(tester);
                tester.pump_and_settle();
                close(tester, controller.handle());
                pump(tester);

                assert_more_or_less_equals_within(panel_opacity(tester), 1.0, 0.05);

                pump_ms(tester, 100);

                // Opacity values at different millisecond offsets during the 50 ms fade-out
                // animation.
                for (_ms, expected) in [(110, 1.0), (120, 0.8), (130, 0.6), (140, 0.4), (150, 0.2)]
                {
                    assert_more_or_less_equals_within(panel_opacity(tester), expected, 0.05);
                    pump_ms(tester, 10);
                }
            });
        }

        /// Dart's padded-to-zero animated `MenuAnchor` of one 160 pixel tall child.
        fn pump_tall_anchor(tester: &mut WidgetTester, controller: MenuControllerRef) {
            pump_material(tester, m3, |_app| {
                MenuAnchor::new(vec![SizedBox::new().height(160.0).into_widget()])
                    .style(zero_padding())
                    .controller(controller.clone())
                    .animated(true)
                    .into_widget()
            });
        }

        // menu_anchor_test.dart 'Menu panel height eases out while opening'.
        #[test]
        fn the_menu_panel_height_eases_out_while_opening() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_tall_anchor(tester, controller.clone());

                open(tester, controller.handle(), None);
                pump(tester);

                for (_ms, expected) in [
                    (0, 0.0),
                    (100, 60.5),
                    (200, 125.5),
                    (300, 148.0),
                    (400, 157.5),
                    (500, 160.0),
                ] {
                    let height = panel_rect(tester).height();
                    assert_more_or_less_equals_within(height, expected, 1.0);
                    pump_ms(tester, 100);
                }
            });
        }

        // menu_anchor_test.dart 'Menu panel height eases in while closing'.
        #[test]
        fn the_menu_panel_height_eases_in_while_closing() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_tall_anchor(tester, controller.clone());

                open(tester, controller.handle(), None);
                pump(tester);
                tester.pump_and_settle();
                close(tester, controller.handle());
                pump(tester);

                assert_eq!(panel_rect(tester).height(), 160.0, "at t=0");

                pump_ms(tester, 50);

                assert_more_or_less_equals_within(panel_rect(tester).height(), 153.0, 2.0);

                pump_ms(tester, 50);

                for (_ms, expected) in [
                    (100, 129.0),
                    (110, 120.0),
                    (120, 110.0),
                    (130, 97.0),
                    (140, 80.0),
                    (150, 0.0),
                ] {
                    assert_more_or_less_equals_within(panel_rect(tester).height(), expected, 2.0);
                    pump_ms(tester, 10);
                }
            });
        }

        // menu_anchor_test.dart 'Item fade-in animation staggers over 500 ms'.
        #[test]
        fn the_item_fade_in_staggers_over_500_ms() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_animated_anchor(
                    tester,
                    controller.clone(),
                    items(&["Item 0", "Item 1", "Item 2", "Item 3"]),
                );

                open(tester, controller.handle(), None);
                pump(tester);

                // Opacity values at different millisecond offsets during the 500ms opening
                // animation.
                for (_time, expected) in [
                    (100, [0.400, 0.0667, 0.00, 0.00]),
                    (200, [0.800, 0.467, 0.133, 0.00]),
                    (300, [1.00, 0.867, 0.533, 0.200]),
                    (400, [1.00, 1.00, 0.933, 0.600]),
                    (500, [1.00, 1.00, 1.00, 1.00]),
                ] {
                    pump_ms(tester, 100);
                    for (i, expected) in expected.into_iter().enumerate() {
                        let opacity = item_opacity(tester, &format!("Item {i}"));
                        assert_more_or_less_equals_within(opacity, expected, 0.05);
                    }
                }
            });
        }

        // menu_anchor_test.dart 'Item fade-out animation staggers over 150 ms with 50 ms delay'.
        #[test]
        fn the_item_fade_out_staggers_over_150_ms_after_a_50_ms_delay() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_material(tester, m3, |_app| {
                    MenuAnchor::new(items(&["Item 0", "Item 1", "Item 2", "Item 3"]))
                        .builder(toggle_button("Open Menu"))
                        .controller(controller.clone())
                        .animated(true)
                        .into_widget()
                });

                tester.tap(find::text("Open Menu"));
                pump(tester);
                pump_ms(tester, 501);
                close(tester, controller.handle());
                pump(tester);

                // Opacity values at different millisecond offsets during the 150ms close animation.
                for (_time, expected) in [
                    (0, [1.00, 1.00, 1.00, 1.00]),
                    (25, [1.00, 1.00, 1.00, 1.00]),
                    (50, [1.00, 1.00, 1.00, 1.00]),
                    (75, [1.00, 1.00, 0.833, 0.500]),
                    (100, [1.00, 0.667, 0.333, 0.00]),
                    (125, [0.500, 0.167, 0.00, 0.00]),
                    (150, [0.00, 0.00, 0.00, 0.00]),
                ] {
                    for (i, expected) in expected.into_iter().enumerate() {
                        let opacity = text_fade_opacity(tester, &format!("Item {i}"));
                        assert_more_or_less_equals_within(opacity, expected, 0.05);
                    }
                    pump_ms(tester, 25);
                }
            });
        }

        // menu_anchor_test.dart 'MenuAnchor calls onAnimationStatusChanged with correct values'.
        #[test]
        fn an_animated_menu_anchor_reports_each_animation_status() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let (status, on_status) = status_sink();
                pump_material(tester, m3, |_app| {
                    MenuAnchor::new(items(&["Item 0"]))
                        .controller(controller.clone())
                        .animated(true)
                        .on_animation_status_changed(on_status)
                        .into_widget()
                });

                // Open the menu
                open(tester, controller.handle(), None);
                pump(tester);

                assert_eq!(status.get(), Some(AnimationStatus::Forward));

                pump_ms(tester, 200);

                assert_eq!(status.get(), Some(AnimationStatus::Forward));

                close(tester, controller.handle());
                pump(tester);

                assert_eq!(status.get(), Some(AnimationStatus::Reverse));

                pump_ms(tester, 50);

                assert_eq!(status.get(), Some(AnimationStatus::Reverse));

                open(tester, controller.handle(), None);
                pump(tester);

                assert_eq!(status.get(), Some(AnimationStatus::Forward));

                tester.pump_and_settle();

                assert_eq!(status.get(), Some(AnimationStatus::Completed));

                close(tester, controller.handle());

                tester.pump_and_settle();

                assert_eq!(status.get(), Some(AnimationStatus::Dismissed));
            });
        }

        // menu_anchor_test.dart 'MenuAnchor without animations calls onAnimationStatusChanged with
        // correct values'.
        #[test]
        fn a_menu_anchor_without_animations_reports_completed_and_dismissed() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let (status, on_status) = status_sink();
                pump_material(tester, m3, |_app| {
                    MenuAnchor::new(items(&["Item 0"]))
                        .controller(controller.clone())
                        .on_animation_status_changed(on_status)
                        .into_widget()
                });

                // Open the menu
                open(tester, controller.handle(), None);
                pump(tester);

                assert_eq!(status.get(), Some(AnimationStatus::Completed));

                close(tester, controller.handle());
                pump(tester);

                assert_eq!(status.get(), Some(AnimationStatus::Dismissed));

                open(tester, controller.handle(), None);
                pump(tester);

                assert_eq!(status.get(), Some(AnimationStatus::Completed));
            });
        }

        // menu_anchor_test.dart 'SubmenuButton onAnimationStatusChanged is invoked with correct
        // values'.
        #[test]
        fn an_animated_submenu_button_reports_each_animation_status() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let submenu_controller = new_controller(tester);
                let (status, on_status) = status_sink();
                pump_material(tester, m3, |_app| {
                    MenuAnchor::new(vec![
                        SubmenuButton::new(items(&["Item 0"]), Some(Text::new("Submenu")))
                            .controller(submenu_controller.clone())
                            .animated(true)
                            .on_animation_status_changed(on_status)
                            .into_widget(),
                    ])
                    .controller(controller.clone())
                    .animated(true)
                    .into_widget()
                });

                // Open the menu
                open(tester, controller.handle(), None);
                pump(tester);
                open(tester, submenu_controller.handle(), None);
                pump(tester);

                assert_eq!(status.get(), Some(AnimationStatus::Forward));

                pump_ms(tester, 200);

                assert_eq!(status.get(), Some(AnimationStatus::Forward));

                close(tester, submenu_controller.handle());
                pump(tester);

                assert_eq!(status.get(), Some(AnimationStatus::Reverse));

                pump_ms(tester, 50);

                assert_eq!(status.get(), Some(AnimationStatus::Reverse));

                open(tester, submenu_controller.handle(), None);
                pump(tester);

                assert_eq!(status.get(), Some(AnimationStatus::Forward));

                tester.pump_and_settle();

                assert_eq!(status.get(), Some(AnimationStatus::Completed));

                close(tester, submenu_controller.handle());

                tester.pump_and_settle();

                assert_eq!(status.get(), Some(AnimationStatus::Dismissed));

                open(tester, submenu_controller.handle(), None);
                tester.pump_and_settle();

                assert_eq!(status.get(), Some(AnimationStatus::Completed));

                close(tester, controller.handle());
                pump(tester);

                assert_eq!(status.get(), Some(AnimationStatus::Reverse));

                pump_ms(tester, 50);

                open(tester, submenu_controller.handle(), None);
                pump(tester);
                pump_ms(tester, 25);

                // Reopen request should be ignored since the parent menu is closing.
                assert_eq!(status.get(), Some(AnimationStatus::Reverse));

                pump(tester);
                tester.pump_and_settle();

                assert_eq!(status.get(), Some(AnimationStatus::Dismissed));
            });
        }

        // menu_anchor_test.dart 'Menu items can be pressed during opening animation'.
        #[test]
        fn menu_items_can_be_pressed_while_the_menu_opens() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let press_count = Rc::new(Cell::new(0));
                let pressable = |text: &str| {
                    let press_count = Rc::clone(&press_count);
                    MenuItemButton::new()
                        .on_pressed(Listener::new(move |_app| {
                            press_count.set(press_count.get() + 1)
                        }))
                        .child(Text::new(text))
                        .into_widget()
                };
                let children = vec![pressable("Item 0"), pressable("Item 1")];
                pump_animated_anchor(tester, controller.clone(), children);

                open(tester, controller.handle(), None);
                pump(tester);
                pump_ms(tester, 100);

                tester.tap(find::text("Item 0"));
                pump(tester);

                assert_eq!(press_count.get(), 1);

                open(tester, controller.handle(), None);
                pump(tester);
                pump_ms(tester, 250);
                tester.tap(find::text("Item 1"));
                pump(tester);

                assert_eq!(press_count.get(), 2);
            });
        }

        // menu_anchor_test.dart 'Pointer is blocked while menu is closing'.
        #[test]
        fn a_closing_menu_does_not_take_taps() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let item_pressed = Rc::new(Cell::new(false));
                let pressed = Rc::clone(&item_pressed);
                pump_animated_anchor(
                    tester,
                    controller.clone(),
                    vec![
                        MenuItemButton::new()
                            .on_pressed(Listener::new(move |_app| pressed.set(true)))
                            .child(Text::new("Item 0"))
                            .into_widget(),
                    ],
                );

                // Open and wait for animation to complete
                open(tester, controller.handle(), None);
                tester.pump_and_settle();

                assert!(shows(tester, "Item 0"));

                // Start closing
                close(tester, controller.handle());
                pump(tester);

                // Try to tap the item while closing
                tester.tap(find::text("Item 0"));
                pump(tester);

                // Item should not have been pressed
                assert!(!item_pressed.get());
            });
        }

        // menu_anchor_test.dart 'Menu items can be focused during opening animation'.
        #[test]
        fn menu_items_can_be_focused_while_the_menu_opens() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let item0_focus_node = new_focus_node(tester, None);
                let item1_focus_node = new_focus_node(tester, None);
                pump_animated_anchor(
                    tester,
                    controller.clone(),
                    vec![
                        MenuItemButton::new()
                            .focus_node(item0_focus_node.clone())
                            .on_pressed(Listener::new(|_app| {}))
                            .child(Text::new("Item 0"))
                            .into_widget(),
                        MenuItemButton::new()
                            .focus_node(item1_focus_node.clone())
                            .on_pressed(Listener::new(|_app| {}))
                            .child(Text::new("Item 1"))
                            .into_widget(),
                    ],
                );

                open(tester, controller.handle(), None);
                pump(tester);

                request_focus(tester, item0_focus_node.handle());
                pump(tester);

                assert!(has_focus(tester, item0_focus_node.handle()));

                request_focus(tester, item1_focus_node.handle());
                pump(tester);

                assert!(has_focus(tester, item1_focus_node.handle()));
            });
        }

        // menu_anchor_test.dart 'Focus returns to parent scope during closing animation'.
        #[test]
        fn focus_returns_to_the_outside_button_as_soon_as_the_menu_starts_closing() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let submenu_focus_node = new_focus_node(tester, None);
                let outside_focus_node = new_focus_node(tester, None);
                pump_material(tester, m3, |_app| {
                    ConstrainedBox::new(BoxConstraints::new().min_width(400.0).min_height(400.0))
                        .child(Column::new().children(vec![
                                TextButton::new(
                                    Some(Listener::new(|_app| {})),
                                    Some(Text::new("Outside")),
                                )
                                .focus_node(outside_focus_node.clone())
                                .into_widget(),
                                MenuAnchor::new(vec![
                                    MenuItemButton::new()
                                        .autofocus(true)
                                        .on_pressed(Listener::new(|_app| {}))
                                        .child(Text::new("Item 0"))
                                        .into_widget(),
                                    SubmenuButton::new(
                                        vec![
                                            MenuItemButton::new()
                                                .focus_node(submenu_focus_node.clone())
                                                .on_pressed(Listener::new(|_app| {}))
                                                .child(Text::new("Nested"))
                                                .into_widget(),
                                        ],
                                        Some(Text::new("Submenu")),
                                    )
                                    .animated(true)
                                    .into_widget(),
                                ])
                                .controller(controller.clone())
                                .animated(true)
                                .builder(|_app, _context, controller, _child| {
                                    TextButton::new(
                                        Some(Listener::new(move |app| {
                                            if controller.is_open(app) {
                                                controller.close(app);
                                            } else {
                                                controller.open(app, None);
                                            }
                                        })),
                                        Some(Text::new("Open Menu")),
                                    )
                                    .autofocus(true)
                                    .into_widget()
                                })
                                .into_widget(),
                            ]))
                        .into_widget()
                });

                request_focus(tester, outside_focus_node.handle());
                pump(tester);

                assert!(has_primary_focus(tester, outside_focus_node.handle()));

                tester.tap(find::text("Open Menu"));
                pump(tester);
                pump_ms(tester, 500);

                let submenu = tester.get_rect(find::text("Submenu")).center();
                hover_over(tester, submenu);
                pump(tester);
                pump_ms(tester, 500);

                assert!(shows(tester, "Nested"));

                request_focus(tester, submenu_focus_node.handle());
                pump(tester);

                assert!(has_primary_focus(tester, submenu_focus_node.handle()));

                close(tester, controller.handle());
                pump(tester);

                assert!(has_primary_focus(tester, outside_focus_node.handle()));

                pump_ms(tester, 50);

                assert!(has_primary_focus(tester, outside_focus_node.handle()));

                tester.pump_and_settle();

                assert!(has_primary_focus(tester, outside_focus_node.handle()));
            });
        }

        // menu_anchor_test.dart 'Submenu items can be traversed during opening animation'.
        #[test]
        fn submenu_items_can_be_traversed_while_the_menus_open() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_material(tester, m3, |_app| {
                    MenuBar::new(vec![
                        SubmenuButton::new(
                            vec![
                                item("Item 0"),
                                SubmenuButton::new(
                                    items(&["Sub Item 0", "Sub Item 1"]),
                                    Some(Text::new("Submenu")),
                                )
                                .animated(true)
                                .into_widget(),
                            ],
                            Some(Text::new("Menu")),
                        )
                        .animated(true)
                        .into_widget(),
                    ])
                    .controller(controller)
                    .into_widget()
                });

                tester.tap(find::text("Menu"));
                pump(tester);
                pump_ms(tester, 100);

                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("SubmenuButton", "Menu"))
                );

                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);

                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("MenuItemButton", "Item 0"))
                );

                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                pump(tester);

                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("SubmenuButton", "Submenu"))
                );

                send_key(tester, LogicalKeyboardKey::ARROW_RIGHT);
                pump(tester);

                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("MenuItemButton", "Sub Item 0"))
                );

                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                pump(tester);

                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("MenuItemButton", "Sub Item 1"))
                );

                send_key(tester, LogicalKeyboardKey::ARROW_LEFT);
                pump(tester);

                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("SubmenuButton", "Submenu"))
                );

                send_key(tester, LogicalKeyboardKey::ARROW_RIGHT);
                pump(tester);

                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("MenuItemButton", "Sub Item 0"))
                );

                send_key(tester, LogicalKeyboardKey::ARROW_LEFT);
                pump(tester);

                send_key(tester, LogicalKeyboardKey::ARROW_UP);
                pump(tester);

                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("MenuItemButton", "Item 0"))
                );
            });
        }

        // menu_anchor_test.dart 'Hover traversal works during opening animation'.
        #[test]
        fn hovering_moves_focus_while_the_menu_opens() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_animated_anchor(
                    tester,
                    controller.clone(),
                    items(&["Item 0", "Item 1", "Item 2"]),
                );

                open(tester, controller.handle(), None);
                pump(tester);
                pump_ms(tester, 100);

                for (device, (wait, text)) in [
                    (0, "Item 0"),
                    (100, "Item 1"),
                    (100, "Item 2"),
                    (200, "Item 0"),
                ]
                .into_iter()
                .enumerate()
                {
                    if wait > 0 {
                        pump_ms(tester, wait);
                    }
                    let center = tester.get_rect(find::text(text)).center();
                    hover_with_new_mouse(tester, device as i64 + 1, center);
                    pump(tester);

                    assert_eq!(
                        focused_menu(tester),
                        Some(button_label("MenuItemButton", text))
                    );
                }
            });
        }

        // menu_anchor_test.dart 'MenuBar items can be traversed during submenu opening animation'.
        #[test]
        fn menu_bar_items_can_be_traversed_while_a_submenu_opens() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_material(tester, m3, |_app| {
                    let submenu = |label: &str| {
                        SubmenuButton::new(
                            items(&[&format!("{label} Item")]),
                            Some(Text::new(label)),
                        )
                        .animated(true)
                        .into_widget()
                    };
                    MenuBar::new(vec![
                        submenu("Menu 0"),
                        submenu("Menu 1"),
                        submenu("Menu 2"),
                    ])
                    .controller(controller)
                    .into_widget()
                });

                tester.tap(find::text("Menu 0"));
                pump(tester);
                pump_ms(tester, 20);
                send_key(tester, LogicalKeyboardKey::ARROW_RIGHT);
                pump(tester);

                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("SubmenuButton", "Menu 1"))
                );

                send_key(tester, LogicalKeyboardKey::ARROW_RIGHT);
                pump(tester);

                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("SubmenuButton", "Menu 2"))
                );

                pump_ms(tester, 20);
                send_key(tester, LogicalKeyboardKey::ARROW_LEFT);
                pump(tester);

                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("SubmenuButton", "Menu 1"))
                );
            });
        }

        // menu_anchor_test.dart 'Focus state remains consistent throughout opening animation'.
        #[test]
        fn a_focused_item_keeps_focus_while_the_menu_opens() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let item_focus_node = new_focus_node(tester, None);
                pump_material(tester, m3, |_app| {
                    Column::new()
                        .children(vec![
                            TextButton::new(
                                Some(Listener::new(|_app| {})),
                                Some(Text::new("Outside")),
                            )
                            .autofocus(true)
                            .into_widget(),
                            MenuAnchor::new(vec![
                                MenuItemButton::new()
                                    .focus_node(item_focus_node.clone())
                                    .on_pressed(Listener::new(|_app| {}))
                                    .child(Text::new("Item 0"))
                                    .into_widget(),
                                MenuItemButton::new()
                                    .autofocus(true)
                                    .on_pressed(Listener::new(|_app| {}))
                                    .child(Text::new("Item 1"))
                                    .into_widget(),
                            ])
                            .controller(controller.clone())
                            .animated(true)
                            .into_widget(),
                        ])
                        .into_widget()
                });

                open(tester, controller.handle(), None);
                pump(tester);

                request_focus(tester, item_focus_node.handle());
                pump(tester);

                assert!(has_primary_focus(tester, item_focus_node.handle()));

                pump_ms(tester, 200);

                assert!(has_primary_focus(tester, item_focus_node.handle()));

                tester.pump_and_settle();

                assert!(has_primary_focus(tester, item_focus_node.handle()));
            });
        }

        // menu_anchor_test.dart 'Scrollbar thumb is not shown while animated menu is opening'.
        #[test]
        fn the_scrollbar_thumb_shows_only_once_the_menu_has_opened() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let texts: Vec<String> = (0..20).map(|i| format!("Item {i}")).collect();
                let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
                pump_animated_anchor(tester, controller.clone(), items(&texts));

                let thumb_visibility = |tester: &WidgetTester| {
                    let scrollbar = tester.element(find::by_type::<Scrollbar>());
                    tester.widget::<Scrollbar, _>(scrollbar, |scrollbar| scrollbar.thumb_visibility)
                };

                open(tester, controller.handle(), None);
                pump(tester);

                assert_eq!(thumb_visibility(tester), Some(false));

                // Progress the animation to the middle.
                pump_ms(tester, 250);

                assert_eq!(thumb_visibility(tester), Some(false));

                tester.pump_and_settle();

                assert_eq!(thumb_visibility(tester), Some(true));
            });
        }

        // menu_anchor_test.dart 'Positioned menus always begin animating at the target position'.
        #[test]
        fn a_positioned_menu_begins_animating_at_its_position() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let target_position = Offset::new(400.0, 400.0);
                pump_tall_anchor(tester, controller.clone());

                open(tester, controller.handle(), Some(target_position));

                pump(tester);
                pump_ms(tester, 2);

                assert_eq!(panel_rect(tester).top_left(), target_position);

                tester.pump_and_settle();

                assert_eq!(panel_rect(tester).top_left(), target_position);
            });
        }

        /// Dart's centred `MenuAnchor` of an 800 by 24 item, whose builder is as wide.
        fn pump_wide_anchor(
            tester: &mut WidgetTester,
            controller: MenuControllerRef,
            reserved_padding: Option<EdgeInsetsGeometry>,
        ) {
            pump_material(tester, m3, |_app| {
                let mut anchor = MenuAnchor::new(vec![
                    SizedBox::new().width(800.0).height(24.0).into_widget(),
                ])
                .controller(controller.clone())
                .builder(|_app, _context, _controller, _child| {
                    SizedBox::new().width(800.0).height(24.0).into_widget()
                });
                if let Some(reserved_padding) = reserved_padding {
                    anchor = anchor.reserved_padding(reserved_padding);
                }
                Center::new().child(anchor).into_widget()
            });
        }

        fn menu_panel_rect(tester: &WidgetTester) -> Rect {
            rect_of_element(tester, only(menu_panels(tester)))
        }

        // menu_anchor_test.dart 'Menu panel default reserved padding'.
        #[test]
        fn the_menu_panel_keeps_8_pixels_from_the_view_by_default() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                pump_wide_anchor(tester, controller.clone(), None);

                open(tester, controller.handle(), None);
                pump(tester);

                let default_reserved_padding = 8.0; // See K_MENU_VIEW_PADDING.
                assert_eq!(
                    menu_panel_rect(tester).width(),
                    800.0 - default_reserved_padding * 2.0
                );
            });
        }

        // menu_anchor_test.dart 'Menu panel accepts custom reserved padding'.
        #[test]
        fn the_menu_panel_keeps_its_reserved_padding_from_the_view() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let reserved_padding = EdgeInsetsGeometry::symmetric(0.0, 13.0);
                pump_wide_anchor(tester, controller.clone(), Some(reserved_padding));

                open(tester, controller.handle(), None);
                pump(tester);

                assert_eq!(
                    menu_panel_rect(tester).width(),
                    800.0 - reserved_padding.horizontal()
                );
            });
        }

        /// `tester.pumpWidget(MaterialApp(home: home))`.
        fn pump_home(tester: &mut WidgetTester, home: impl FnOnce(&mut App) -> WidgetRef) {
            let root = {
                let mut app = tester.cell.borrow_mut();
                let theme = m3(&app);
                let home = home(&mut app);
                material_app(theme, home)
            };
            tester.pump_widget(root);
        }

        /// Dart's `Center(child: SizedBox.shrink(child: child))` as a `MaterialApp`'s home.
        fn pump_shrunk(tester: &mut WidgetTester, child: WidgetRef) {
            pump_home(tester, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(child))
                    .into_widget()
            });
        }

        fn size_of<W: 'static>(tester: &WidgetTester) -> Size {
            rect_of::<W>(tester).size()
        }

        // menu_anchor_test.dart 'MenuAcceleratorLabel does not crash at zero area'.
        #[test]
        fn a_menu_accelerator_label_of_zero_area_does_not_panic() {
            test_widgets(|tester| {
                pump_shrunk(tester, MenuAcceleratorLabel::new("X").into_widget());
                assert_eq!(size_of::<MenuAcceleratorLabel>(tester), Size::ZERO);
            });
        }

        // menu_anchor_test.dart 'Layout updates when reserved padding changes'.
        #[test]
        fn the_menu_lays_out_again_when_its_reserved_padding_changes() {
            test_widgets(|tester| {
                let controller = new_controller(tester);
                let reserved_padding = EdgeInsetsGeometry::symmetric(0.0, 13.0);
                let wide = || vec![SizedBox::new().width(800.0).height(24.0).into_widget()];

                pump_home(tester, |_app| {
                    MenuAnchor::new(wide())
                        .controller(controller.clone())
                        .into_widget()
                });

                open(tester, controller.handle(), Some(Offset::ZERO));
                pump(tester);

                pump_home(tester, |_app| {
                    MenuAnchor::new(wide())
                        .controller(controller)
                        .reserved_padding(reserved_padding)
                        .into_widget()
                });

                assert_eq!(
                    menu_panel_rect(tester).width(),
                    800.0 - reserved_padding.horizontal()
                );
            });
        }

        // menu_anchor_test.dart 'SubmenuButton does not crash at zero area'.
        #[test]
        fn a_submenu_button_of_zero_area_does_not_panic() {
            test_widgets(|tester| {
                pump_shrunk(
                    tester,
                    SubmenuButton::new(Vec::new(), None::<WidgetRef>).into_widget(),
                );
                assert_eq!(size_of::<SubmenuButton>(tester), Size::ZERO);
            });
        }

        // menu_anchor_test.dart 'MenuBar does not crash at zero area'.
        #[test]
        fn a_menu_bar_of_zero_area_does_not_panic() {
            test_widgets(|tester| {
                pump_shrunk(
                    tester,
                    MenuBar::new(vec![Text::new("X").into_widget()]).into_widget(),
                );
                assert_eq!(size_of::<MenuBar>(tester), Size::ZERO);
            });
        }

        // menu_anchor_test.dart 'MenuItemButton does not crash at zero area'.
        #[test]
        fn a_menu_item_button_of_zero_area_does_not_panic() {
            test_widgets(|tester| {
                pump_shrunk(tester, MenuItemButton::new().into_widget());
                assert_eq!(size_of::<MenuItemButton>(tester), Size::ZERO);
            });
        }

        // menu_anchor_test.dart 'RadioMenuButton does not crash at zero area'.
        #[test]
        fn a_radio_menu_button_of_zero_area_does_not_panic() {
            test_widgets(|tester| {
                pump_shrunk(
                    tester,
                    RadioMenuButton::new(
                        true,
                        Some(true),
                        Some(Rc::new(|_app: &mut App, _value: Option<bool>| {})),
                        None::<WidgetRef>,
                    )
                    .into_widget(),
                );
                assert_eq!(size_of::<RadioMenuButton<bool>>(tester), Size::ZERO);
            });
        }

        // menu_anchor_test.dart 'CheckboxMenuButton does not crash at zero area'.
        #[test]
        fn a_checkbox_menu_button_of_zero_area_does_not_panic() {
            test_widgets(|tester| {
                pump_shrunk(
                    tester,
                    CheckboxMenuButton::new(
                        Some(true),
                        Some(Rc::new(|_app: &mut App, _value: Option<bool>| {})),
                        Some(Text::new("X")),
                    )
                    .into_widget(),
                );
                assert_eq!(size_of::<CheckboxMenuButton>(tester), Size::ZERO);
            });
        }

        // menu_anchor_test.dart 'MenuAnchor does not crash at zero area'.
        #[test]
        fn a_menu_anchor_in_a_view_of_zero_area_does_not_panic() {
            test_widgets(|tester| {
                tester.view().set_physical_size(Size::ZERO);
                let menu_controller = new_controller(tester);
                pump_home(tester, |_app| {
                    Scaffold::new()
                        .body(
                            Center::new().child(
                                MenuAnchor::new(vec![Text::new("X").into_widget()])
                                    .controller(menu_controller.clone()),
                            ),
                        )
                        .into_widget()
                });
                assert_eq!(size_of::<MenuAnchor>(tester), Size::ZERO);
                open(tester, menu_controller.handle(), None);
                pump(tester);
                assert!(shows(tester, "X"));
            });
        }

        // menu_anchor_test.dart 'Escape closes MenuAnchor opened by tap without childFocusNode'.
        #[test]
        fn escape_closes_a_menu_anchor_opened_by_a_tap_without_a_child_focus_node() {
            test_widgets(|tester| {
                let outside_focus_node = new_focus_node(tester, Some("Outside Focus"));
                pump_home(tester, |_app| {
                    Scaffold::new()
                        .body(Column::new().children(vec![
                                TextButton::new(
                                    Some(Listener::new(|_app| {})),
                                    Some(Text::new("Outside")),
                                )
                                .focus_node(outside_focus_node.clone())
                                .autofocus(true)
                                .into_widget(),
                                MenuAnchor::new(items(&["Item 1", "Item 2"]))
                                    .builder(toggle_button("Open menu"))
                                    .into_widget(),
                            ]))
                        .into_widget()
                });

                pump(tester);
                assert!(has_primary_focus(tester, outside_focus_node.handle()));

                tester.tap(find::text("Open menu"));
                tester.pump_and_settle();
                assert!(shows(tester, "Item 1"));

                send_key(tester, LogicalKeyboardKey::ESCAPE);
                tester.pump_and_settle();
                assert!(!shows(tester, "Item 1"));
                assert!(has_primary_focus(tester, outside_focus_node.handle()));

                // Reopen by tap, traverse with arrow key, and close with Escape.
                tester.tap(find::text("Open menu"));
                tester.pump_and_settle();
                assert!(shows(tester, "Item 1"));

                send_key(tester, LogicalKeyboardKey::ARROW_DOWN);
                pump(tester);
                assert_eq!(
                    focused_menu(tester),
                    Some(button_label("MenuItemButton", "Item 1"))
                );

                send_key(tester, LogicalKeyboardKey::ESCAPE);
                tester.pump_and_settle();
                assert!(!shows(tester, "Item 1"));
                assert!(has_primary_focus(tester, outside_focus_node.handle()));
            });
        }

        // menu_anchor_test.dart 'Escape closes MenuAnchor opened by tap inside a Dialog before
        // dismissing the Dialog'.
        #[test]
        fn escape_closes_a_menu_anchor_in_a_dialog_before_it_dismisses_the_dialog() {
            test_widgets(|tester| {
                pump_home(tester, |_app| {
                    Scaffold::new()
                        .body(Builder::new(|_app, context| {
                            TextButton::new(
                                Some(Listener::new(move |app| {
                                    let builder: waft_widgets::WidgetBuilder =
                                        Rc::new(|_app, _context| {
                                            AlertDialog::new()
                                                .title(Text::new("Dialog Title"))
                                                .content(
                                                    MenuAnchor::new(items(&["Menu Item 1"]))
                                                        .builder(
                                                            |_app, _context, controller, _child| {
                                                                TextButton::new(
                                                                    Some(Listener::new(
                                                                        move |app| {
                                                                            controller
                                                                                .open(app, None)
                                                                        },
                                                                    )),
                                                                    Some(Text::new("Open menu")),
                                                                )
                                                                .into_widget()
                                                            },
                                                        ),
                                                )
                                                .into_widget()
                                        });
                                    drop(ShowDialog::new(builder).show(app, context));
                                })),
                                Some(Text::new("Show Dialog")),
                            )
                            .into_widget()
                        }))
                        .into_widget()
                });

                tester.tap(find::text("Show Dialog"));
                tester.pump_and_settle();
                assert!(shows(tester, "Dialog Title"));

                tester.tap(find::text("Open menu"));
                tester.pump_and_settle();
                assert!(shows(tester, "Menu Item 1"));

                // First Escape closes only the MenuAnchor menu, keeping the Dialog open.
                send_key(tester, LogicalKeyboardKey::ESCAPE);
                tester.pump_and_settle();
                assert!(!shows(tester, "Menu Item 1"));
                assert!(shows(tester, "Dialog Title"));

                // Second Escape dismisses the Dialog.
                send_key(tester, LogicalKeyboardKey::ESCAPE);
                tester.pump_and_settle();
                assert!(!shows(tester, "Dialog Title"));
            });
        }
    }
}
