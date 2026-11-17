//! Flutter counterpart: `widgets/raw_menu_anchor.dart`.
//!
//! A [`RawMenuAnchor`] shows a floating menu in an [`OverlayPortal`]; a [`RawMenuAnchorGroup`]
//! is an always-open anchor, such as a menu bar. Anchors below one another form a tree: each
//! finds its parent through the nearest [`MenuController`]'s anchor, opening one closes its
//! siblings, and closing one closes its children first. A [`MenuController`] is attached to at
//! most one anchor at a time, and a detached controller answers `is_open` with false.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Offset, Rect, Size};
use waft_foundation::{
    App, Handle, HandleId, ListenableObject, Listener, OptionRetained, Ref, Retained, VoidCallback,
};
use waft_gestures::PointerDownEvent;
use waft_painting::transform_rect;
use waft_scheduler::{FrameCallback, SchedulerBinding, SchedulerPhase};

use crate::framework::{
    BuildContext, GlobalKey, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget,
    WidgetRef,
};
use crate::view::View;
use crate::widgets::actions::{
    Action, ActionData, ActionRef, Actions, ActivateIntent, DismissAction, DismissIntent,
};
use crate::widgets::basic::{Builder, SizedBox};
use crate::widgets::default_text_editing_shortcuts::shortcut_map;
use crate::widgets::focus_manager::FocusNodeRef;
use crate::widgets::focus_traversal::{DirectionalFocusIntent, TraversalDirection};
use crate::widgets::media_query::MediaQuery;
use crate::widgets::overlay::{
    OverlayChildLayoutInfo, OverlayChildLocation, OverlayPortal, OverlayPortalController,
};
use crate::widgets::scroll_position::ScrollPositionRef;
use crate::widgets::scrollable::Scrollable;
use crate::widgets::shortcuts::{ShortcutMap, Shortcuts};
use crate::widgets::tap_region::{TapRegion, TapRegionCallback, TapRegionGroupId};

/// Dart's `_kMenuTraversalShortcuts`.
fn menu_traversal_shortcuts() -> ShortcutMap {
    shortcut_map![
        GAME_BUTTON_A => ActivateIntent::new(),
        ESCAPE => DismissIntent::new(),
        ARROW_DOWN => DirectionalFocusIntent::new(TraversalDirection::Down),
        ARROW_UP => DirectionalFocusIntent::new(TraversalDirection::Up),
        ARROW_LEFT => DirectionalFocusIntent::new(TraversalDirection::Left),
        ARROW_RIGHT => DirectionalFocusIntent::new(TraversalDirection::Right),
    ]
}

/// Anchor and menu information passed to [`RawMenuAnchor`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawMenuOverlayInfo {
    /// The position of the anchor widget that the menu is attached to, relative to the nearest
    /// ancestor [`Overlay`](crate::Overlay) when [`RawMenuAnchor::use_root_overlay`] is false,
    /// or the root [`Overlay`](crate::Overlay) when [`RawMenuAnchor::use_root_overlay`] is true.
    pub anchor_rect: Rect,

    /// The [`Size`] of the overlay that the menu is being shown in.
    pub overlay_size: Size,

    /// The `position` argument passed to [`MenuController::open`].
    ///
    /// The position should be used to offset the menu relative to the top-left corner of the
    /// anchor.
    pub position: Option<Offset>,

    /// The [`TapRegion::group_id`] of the [`TapRegion`] that wraps widgets in this menu system.
    pub tap_region_group_id: TapRegionGroupId,
}

impl RawMenuOverlayInfo {
    /// Creates a [`RawMenuOverlayInfo`]; Dart's optional `position` is the setter.
    pub fn new(
        anchor_rect: Rect,
        overlay_size: Size,
        tap_region_group_id: TapRegionGroupId,
    ) -> RawMenuOverlayInfo {
        RawMenuOverlayInfo {
            anchor_rect,
            overlay_size,
            position: None,
            tap_region_group_id,
        }
    }

    /// Dart `RawMenuOverlayInfo(position:)`.
    pub fn position(mut self, position: Offset) -> RawMenuOverlayInfo {
        self.position = Some(position);
        self
    }
}

/// Signature for the builder function used by [`RawMenuAnchor::overlay_builder`] to build a
/// menu's overlay.
///
/// The `context` is the context that the overlay is being built in.
///
/// The `info` describes the anchor's [`Rect`], the [`Size`] of the overlay, the
/// [`TapRegion::group_id`] used by members of the menu system, and the `position` argument
/// passed to [`MenuController::open`].
pub type RawMenuAnchorOverlayBuilder =
    Rc<dyn Fn(&mut App, BuildContext, RawMenuOverlayInfo) -> WidgetRef>;

/// Signature for the builder function used by [`RawMenuAnchor::builder`] to build the widget
/// that the [`RawMenuAnchor`] surrounds.
///
/// The `context` is the context in which the anchor is being built.
///
/// The `controller` is the [`MenuController`] that can be used to open and close the menu.
///
/// The `child` is an optional child supplied as the [`RawMenuAnchor::child`] attribute. The
/// child is intended to be incorporated in the result of the function.
pub type RawMenuAnchorChildBuilder =
    Rc<dyn Fn(&mut App, BuildContext, AnyMenuController, Option<&WidgetRef>) -> WidgetRef>;

/// Signature for the callback used by [`RawMenuAnchor::on_open_requested`] to intercept
/// requests to open a menu.
///
/// See [`RawMenuAnchor::on_open_requested`] for more information.
pub type RawMenuAnchorOpenRequestedCallback = Rc<dyn Fn(&mut App, Option<Offset>, VoidCallback)>;

/// Signature for the callback used by [`RawMenuAnchor::on_close_requested`] to intercept
/// requests to close a menu.
///
/// See [`RawMenuAnchor::on_close_requested`] for more information.
pub type RawMenuAnchorCloseRequestedCallback = Rc<dyn Fn(&mut App, VoidCallback)>;

// An InheritedWidget used to notify anchor descendants when a menu opens and closes, and to
// pass the anchor's controller to descendants.
#[derive(Debug)]
struct MenuControllerScope {
    is_open: bool,
    controller: MenuControllerRef,
    child: WidgetRef,
}

impl InheritedWidget for MenuControllerScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &MenuControllerScope) -> bool {
        self.is_open != old_widget.is_open
    }
}

/// A widget that wraps a child and anchors a floating menu.
///
/// The child can be any widget, but is typically a button, a text field, or, in the case of
/// context menus, the entire screen.
///
/// The menu overlay of a [`RawMenuAnchor`] is shown by calling [`MenuController::open`] on an
/// attached [`MenuController`].
///
/// When a [`RawMenuAnchor`] is opened, [`overlay_builder`](Self::overlay_builder) is called to
/// construct the menu contents within an [`Overlay`](crate::Overlay). The
/// [`Overlay`](crate::Overlay) allows the menu to "float" on top of other widgets. The `info`
/// argument passed to [`overlay_builder`](Self::overlay_builder) provides the anchor's
/// [`Rect`], the [`Size`] of the overlay, the [`TapRegion::group_id`] used by members of the
/// menu system, and the `position` argument passed to [`MenuController::open`].
///
/// If [`MenuController::open`] is called with a `position` argument, it will be passed to the
/// `info` argument of the `overlay_builder` function.
///
/// The [`RawMenuAnchor`] does not manage semantics and focus of the menu.
///
/// ### Adding animations to menus
///
/// A [`RawMenuAnchor`] has no knowledge of animations, as evident from its APIs, which don't
/// involve `AnimationController` at all. It only knows whether the overlay is shown or hidden.
///
/// If another widget intends to implement a menu with opening and closing transitions,
/// [`RawMenuAnchor`]'s overlay should remain visible throughout both the opening and closing
/// animation durations.
///
/// This means that the `show_overlay` callback passed to
/// [`on_open_requested`](Self::on_open_requested) should be called before the first frame of
/// the opening animation. Conversely, `hide_overlay` within
/// [`on_close_requested`](Self::on_close_requested) should only be called after the closing
/// animation has completed.
///
/// This also means that, if [`MenuController::open`] is called while the overlay is already
/// visible, [`RawMenuAnchor`] has no way of knowing whether the menu is currently opening,
/// closing, or stably displayed. The parent widget will need to manage additional information
/// (such as the state of an `AnimationController`) to determine how to respond in such
/// scenarios.
///
/// To programmatically control a [`RawMenuAnchor`], like opening or closing it, or checking its
/// state, you can get its associated [`MenuController`]. Use [`MenuController::maybe_of`] to
/// retrieve the controller for the closest [`RawMenuAnchor`] ancestor of a given
/// [`BuildContext`]. More detailed usage of [`MenuController`] is available in its type
/// documentation.
pub struct RawMenuAnchor {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// Called when the menu overlay is shown.
    ///
    /// When [`MenuController::open`] is called, [`on_open_requested`](Self::on_open_requested)
    /// is invoked with a `show_overlay` callback that, when called, shows the menu overlay and
    /// triggers [`on_open`](Self::on_open).
    ///
    /// The default implementation of [`on_open_requested`](Self::on_open_requested) calls
    /// `show_overlay` synchronously, thereby calling [`on_open`](Self::on_open) synchronously.
    /// In this case, [`on_open`](Self::on_open) is called regardless of whether the menu
    /// overlay is already showing.
    ///
    /// Custom implementations of [`on_open_requested`](Self::on_open_requested) can delay the
    /// call to `show_overlay`, or not call it at all, in which case [`on_open`](Self::on_open)
    /// will not be called. Calling `show_overlay` after disposal is a no-op, and will not
    /// trigger [`on_open`](Self::on_open).
    ///
    /// A typical usage is to respond when the menu first becomes interactive, such as by
    /// setting focus to a menu item.
    pub on_open: Option<VoidCallback>,

    /// Called when the menu overlay is hidden.
    ///
    /// When [`MenuController::close`] is called,
    /// [`on_close_requested`](Self::on_close_requested) is invoked with a `hide_overlay`
    /// callback that, when called, hides the menu overlay and triggers
    /// [`on_close`](Self::on_close).
    ///
    /// The default implementation of [`on_close_requested`](Self::on_close_requested) calls
    /// `hide_overlay` synchronously, thereby calling [`on_close`](Self::on_close)
    /// synchronously. In this case, [`on_close`](Self::on_close) is called regardless of
    /// whether the menu overlay is already hidden.
    ///
    /// Custom implementations of [`on_close_requested`](Self::on_close_requested) can delay the
    /// call to `hide_overlay` or not call it at all, in which case
    /// [`on_close`](Self::on_close) will not be called. Calling `hide_overlay` after disposal
    /// is a no-op, and will not trigger [`on_close`](Self::on_close).
    pub on_close: Option<VoidCallback>,

    /// Called when a request is made to open the menu.
    ///
    /// This callback is triggered every time [`MenuController::open`] is called, even when the
    /// menu overlay is already showing. As a result, this callback is a good place to begin
    /// menu opening animations, or observe when a menu is repositioned.
    ///
    /// After an open request is intercepted, the `show_overlay` callback should be called when
    /// the menu overlay (the widget built by [`overlay_builder`](Self::overlay_builder)) is
    /// ready to be shown. This can occur immediately (the default behavior), or after a delay.
    /// Calling `show_overlay` sets [`MenuController::is_open`] to true, builds (or rebuilds)
    /// the overlay widget, and shows the menu overlay at the front of the overlay stack.
    ///
    /// If `show_overlay` is not called, the menu will stay hidden. Calling `show_overlay` after
    /// disposal is a no-op, meaning it will not trigger [`on_open`](Self::on_open) or show the
    /// menu overlay.
    ///
    /// If a [`RawMenuAnchor`] is used in a themed menu that plays an opening animation, the
    /// themed menu should show the overlay before starting the opening animation, since the
    /// animation plays on the overlay itself.
    ///
    /// The `position` argument is the `position` that [`MenuController::open`] was called
    /// with.
    ///
    /// A typical [`on_open_requested`](Self::on_open_requested) consists of the following
    /// steps:
    ///
    ///  1. Optional delay.
    ///  2. Call `show_overlay` (whose call chain eventually invokes
    ///     [`on_open`](Self::on_open)).
    ///  3. Optionally start the opening animation.
    ///
    /// Defaults to a callback that immediately shows the menu.
    pub on_open_requested: RawMenuAnchorOpenRequestedCallback,

    /// Called when a request is made to close the menu.
    ///
    /// This callback can be used to add a delay or a closing animation before the menu is
    /// hidden.
    ///
    /// This callback is triggered every time [`MenuController::close`] is called, even when the
    /// menu overlay is already hidden.
    ///
    /// This callback is also triggered when a sibling [`RawMenuAnchor`] is opened. In this
    /// case, the callback can be used to add a delay or a closing animation while the sibling
    /// menu opens. When implementing this behavior, consider disabling interactions so that the
    /// closing menu does not interfere with the opening sibling menu. Also consider disabling
    /// semantics, focus, and hit testing for the closing menu for the duration of the closing
    /// animation.
    ///
    /// Pending timers or animations started in a previous call to
    /// [`on_close_requested`](Self::on_close_requested) should be canceled when this callback
    /// is triggered to prevent them from closing the menu at an unintended time.
    ///
    /// If the menu is not closed, this callback will also be called when the root menu anchor
    /// is scrolled and when the screen is resized.
    ///
    /// After a close request is intercepted and closing behaviors have completed, the
    /// `hide_overlay` callback should be called. This callback sets [`MenuController::is_open`]
    /// to false and hides the menu overlay widget. If the [`RawMenuAnchor`] is used in a themed
    /// menu that plays a closing animation, `hide_overlay` should be called after the closing
    /// animation has ended, since the animation plays on the overlay itself. This means that
    /// [`MenuController::is_open`] will stay true while closing animations are running.
    ///
    /// Calling `hide_overlay` after disposal is a no-op, meaning it will not trigger
    /// [`on_close`](Self::on_close) or hide the menu overlay.
    ///
    /// Typically, [`on_close_requested`](Self::on_close_requested) consists of the following
    /// steps:
    ///
    ///  1. Optionally start the closing animation and wait for it to complete.
    ///  2. Call `hide_overlay` (whose call chain eventually invokes
    ///     [`on_close`](Self::on_close)).
    ///
    /// Throughout the closing sequence, menus should typically not be focusable or interactive.
    ///
    /// Defaults to a callback that immediately hides the menu.
    pub on_close_requested: RawMenuAnchorCloseRequestedCallback,

    /// A builder that builds the widget that this [`RawMenuAnchor`] surrounds.
    ///
    /// Typically, this is a button used to open the menu by calling [`MenuController::open`] on
    /// the `controller` passed to the builder.
    ///
    /// If not supplied, then the [`RawMenuAnchor`] will be the size that its parent allocates
    /// for it.
    pub builder: Option<RawMenuAnchorChildBuilder>,

    /// The optional child to be passed to the [`builder`](Self::builder).
    ///
    /// Supply this child if there is a portion of the widget tree built in
    /// [`builder`](Self::builder) that doesn't depend on the `controller` or `context` supplied
    /// to the [`builder`](Self::builder). It will be more efficient, since the framework
    /// doesn't then need to rebuild this child when those change.
    pub child: Option<WidgetRef>,

    /// Called to build and position the menu overlay.
    ///
    /// The [`overlay_builder`](Self::overlay_builder) function is passed a
    /// [`RawMenuOverlayInfo`] object that defines the anchor's [`Rect`], the [`Size`] of the
    /// overlay, the [`TapRegion::group_id`] for the menu system, and the position [`Offset`]
    /// passed to [`MenuController::open`].
    ///
    /// To ensure taps are properly consumed, the [`RawMenuOverlayInfo::tap_region_group_id`]
    /// should be passed to a [`TapRegion`] widget that wraps the menu panel.
    pub overlay_builder: RawMenuAnchorOverlayBuilder,

    /// Whether the menu panel should be rendered in the root [`Overlay`](crate::Overlay).
    ///
    /// When true, the menu is mounted in the root overlay. Rendering the menu in the root
    /// overlay prevents the menu from being obscured by other widgets.
    ///
    /// When false, the menu is rendered in the nearest ancestor [`Overlay`](crate::Overlay).
    ///
    /// Submenus will always use the same overlay as their top-level ancestor, so setting a
    /// [`use_root_overlay`](Self::use_root_overlay) value on a submenu will have no effect.
    ///
    /// Defaults to false on overlay menus.
    pub use_root_overlay: bool,

    /// The focus node attached to the widget that takes focus when the menu is opened or
    /// closed.
    ///
    /// If not supplied, the anchor will not retain focus when the menu is opened.
    pub child_focus_node: Option<FocusNodeRef>,

    /// Whether a tap event that closes the menu will be permitted to continue on to the gesture
    /// arena.
    ///
    /// If false, then tapping outside of a menu when the menu is open will both close the menu,
    /// and allow the tap to participate in the gesture arena.
    ///
    /// If true, then it will only close the menu, and the tap event will be consumed.
    ///
    /// Defaults to false.
    pub consume_outside_taps: bool,

    /// A [`MenuController`] that allows opening and closing of the menu from other widgets.
    pub controller: MenuControllerRef,
}

impl RawMenuAnchor {
    /// A [`RawMenuAnchor`] that delegates overlay construction to an
    /// [`overlay_builder`](Self::overlay_builder); Dart's optional named arguments are the
    /// setters.
    pub fn new(
        controller: MenuControllerRef,
        overlay_builder: impl Fn(&mut App, BuildContext, RawMenuOverlayInfo) -> WidgetRef + 'static,
    ) -> RawMenuAnchor {
        RawMenuAnchor {
            key: None,
            on_open: None,
            on_close: None,
            on_open_requested: Rc::new(RawMenuAnchor::default_on_open_requested),
            on_close_requested: Rc::new(RawMenuAnchor::default_on_close_requested),
            builder: None,
            child: None,
            overlay_builder: Rc::new(overlay_builder),
            use_root_overlay: false,
            child_focus_node: None,
            consume_outside_taps: false,
            controller,
        }
    }

    /// Dart `RawMenuAnchor(key:)`.
    pub fn key(mut self, key: KeyRef) -> RawMenuAnchor {
        self.key = Some(key);
        self
    }

    /// Dart `RawMenuAnchor(childFocusNode:)`.
    pub fn child_focus_node(mut self, child_focus_node: FocusNodeRef) -> RawMenuAnchor {
        self.child_focus_node = Some(child_focus_node);
        self
    }

    /// Dart `RawMenuAnchor(consumeOutsideTaps:)`.
    pub fn consume_outside_taps(mut self, consume_outside_taps: bool) -> RawMenuAnchor {
        self.consume_outside_taps = consume_outside_taps;
        self
    }

    /// Dart `RawMenuAnchor(onOpen:)`.
    pub fn on_open(mut self, on_open: VoidCallback) -> RawMenuAnchor {
        self.on_open = Some(on_open);
        self
    }

    /// Dart `RawMenuAnchor(onClose:)`.
    pub fn on_close(mut self, on_close: VoidCallback) -> RawMenuAnchor {
        self.on_close = Some(on_close);
        self
    }

    /// Dart `RawMenuAnchor(onOpenRequested:)`.
    pub fn on_open_requested(
        mut self,
        on_open_requested: RawMenuAnchorOpenRequestedCallback,
    ) -> RawMenuAnchor {
        self.on_open_requested = on_open_requested;
        self
    }

    /// Dart `RawMenuAnchor(onCloseRequested:)`.
    pub fn on_close_requested(
        mut self,
        on_close_requested: RawMenuAnchorCloseRequestedCallback,
    ) -> RawMenuAnchor {
        self.on_close_requested = on_close_requested;
        self
    }

    /// Dart `RawMenuAnchor(useRootOverlay:)`.
    pub fn use_root_overlay(mut self, use_root_overlay: bool) -> RawMenuAnchor {
        self.use_root_overlay = use_root_overlay;
        self
    }

    /// Dart `RawMenuAnchor(builder:)`.
    pub fn builder(
        mut self,
        builder: impl Fn(&mut App, BuildContext, AnyMenuController, Option<&WidgetRef>) -> WidgetRef
        + 'static,
    ) -> RawMenuAnchor {
        self.builder = Some(Rc::new(builder));
        self
    }

    /// Dart `RawMenuAnchor(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> RawMenuAnchor {
        self.child = Some(child.into_widget());
        self
    }

    fn default_on_open_requested(
        app: &mut App,
        _position: Option<Offset>,
        show_overlay: VoidCallback,
    ) {
        show_overlay(app);
    }

    fn default_on_close_requested(app: &mut App, hide_overlay: VoidCallback) {
        hide_overlay(app);
    }
}

impl Debug for RawMenuAnchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("RawMenuAnchor");
        if self.child_focus_node.is_some() {
            debug.field("focusNode", &"has focusNode");
        }
        debug
            .field(
                "useRootOverlay",
                &if self.use_root_overlay {
                    "use root overlay"
                } else {
                    "use nearest overlay"
                },
            )
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for RawMenuAnchor {
    type State = RawMenuAnchorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RawMenuAnchorState {
        RawMenuAnchorState {
            state: StateData::new(),
            raw_menu_anchor_base: RawMenuAnchorBaseMixinData::new(),
            anchor_key: if cfg!(debug_assertions) {
                GlobalKey::labeled("MenuAnchor")
            } else {
                GlobalKey::new()
            },
            overlay_controller: None,
            menu_position: None,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// _RawMenuAnchorBaseMixin

/// The vtable of an erased [`AnyRawMenuAnchorBase`]: one `&'static` table per leaf type.
struct RawMenuAnchorBaseVTable {
    type_name: fn() -> &'static str,
    parent: fn(&App, HandleId) -> Option<AnyRawMenuAnchorBase>,
    as_raw_menu_anchor_state: fn(HandleId) -> Option<Handle<RawMenuAnchorState>>,
    menu_controller: fn(&App, HandleId) -> AnyMenuController,
    is_open: fn(&App, HandleId) -> bool,
    add_child: fn(&mut App, HandleId, AnyRawMenuAnchorBase),
    remove_child: fn(&mut App, HandleId, AnyRawMenuAnchorBase),
    child_changed_open_state: fn(&mut App, HandleId),
    close: fn(&mut App, HandleId, bool),
    handle_open_request: fn(&mut App, HandleId, Option<Offset>),
    handle_close_request: fn(&mut App, HandleId),
    request_children_close: fn(&mut App, HandleId),
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<L: 'static>(id: HandleId) -> Handle<L> {
    Handle::from_id(id)
}

impl RawMenuAnchorBaseVTable {
    const fn of<L: RawMenuAnchorBaseMixin>() -> RawMenuAnchorBaseVTable {
        RawMenuAnchorBaseVTable {
            type_name: std::any::type_name::<L>,
            parent: |app, id| L::raw_menu_anchor_base_data(resolve(id), app).parent,
            as_raw_menu_anchor_state: |id| L::as_raw_menu_anchor_state(resolve(id)),
            menu_controller: |app, id| L::menu_controller(resolve(id), app),
            is_open: |app, id| L::is_open(resolve(id), app),
            add_child: |app, id, child| L::add_child(resolve(id), app, child),
            remove_child: |app, id, child| L::remove_child(resolve(id), app, child),
            child_changed_open_state: |app, id| L::child_changed_open_state(resolve(id), app),
            close: |app, id, in_dispose| L::close(resolve(id), app, in_dispose),
            handle_open_request: |app, id, position| {
                L::handle_open_request(resolve(id), app, position)
            },
            handle_close_request: |app, id| L::handle_close_request(resolve(id), app),
            request_children_close: |app, id| L::request_children_close(resolve(id), app),
        }
    }
}

/// Erased `_RawMenuAnchorBaseMixin`: one identity and a static vtable.
///
/// This is what a field Dart types as `_RawMenuAnchorBaseMixin` becomes.
#[derive(Clone, Copy)]
struct AnyRawMenuAnchorBase {
    id: HandleId,
    vtable: &'static RawMenuAnchorBaseVTable,
}

impl PartialEq for AnyRawMenuAnchorBase {
    fn eq(&self, other: &AnyRawMenuAnchorBase) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyRawMenuAnchorBase {}

impl Debug for AnyRawMenuAnchorBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyRawMenuAnchorBase {
    fn parent(self, app: &App) -> Option<AnyRawMenuAnchorBase> {
        (self.vtable.parent)(app, self.id)
    }

    /// Dart's `anchor is _RawMenuAnchorState`.
    fn as_raw_menu_anchor_state(self) -> Option<Handle<RawMenuAnchorState>> {
        (self.vtable.as_raw_menu_anchor_state)(self.id)
    }

    fn menu_controller(self, app: &App) -> AnyMenuController {
        (self.vtable.menu_controller)(app, self.id)
    }

    fn is_open(self, app: &App) -> bool {
        (self.vtable.is_open)(app, self.id)
    }

    /// See [`RawMenuAnchorBaseMixin::root`].
    fn root(self, app: &App) -> AnyRawMenuAnchorBase {
        let mut anchor = self;
        while let Some(parent) = anchor.parent(app) {
            anchor = parent;
        }
        anchor
    }

    fn add_child(self, app: &mut App, child: AnyRawMenuAnchorBase) {
        (self.vtable.add_child)(app, self.id, child);
    }

    fn remove_child(self, app: &mut App, child: AnyRawMenuAnchorBase) {
        (self.vtable.remove_child)(app, self.id, child);
    }

    fn child_changed_open_state(self, app: &mut App) {
        (self.vtable.child_changed_open_state)(app, self.id);
    }

    fn close(self, app: &mut App, in_dispose: bool) {
        (self.vtable.close)(app, self.id, in_dispose);
    }

    fn handle_open_request(self, app: &mut App, position: Option<Offset>) {
        (self.vtable.handle_open_request)(app, self.id, position);
    }

    fn handle_close_request(self, app: &mut App) {
        (self.vtable.handle_close_request)(app, self.id);
    }

    fn request_children_close(self, app: &mut App) {
        (self.vtable.request_children_close)(app, self.id);
    }
}

/// The fields Dart's `_RawMenuAnchorBaseMixin` declares; an anchor state carries this bag
/// under the field `raw_menu_anchor_base`.
struct RawMenuAnchorBaseMixinData {
    anchor_children: Vec<AnyRawMenuAnchorBase>,
    parent: Option<AnyRawMenuAnchorBase>,
    scroll_position: Option<ScrollPositionRef>,
    view_size: Option<Size>,
}

impl RawMenuAnchorBaseMixinData {
    fn new() -> RawMenuAnchorBaseMixinData {
        RawMenuAnchorBaseMixinData {
            anchor_children: Vec::new(),
            parent: None,
            scroll_position: None,
            view_size: None,
        }
    }
}

/// Dart's `_RawMenuAnchorBaseMixin`: the common interface and state for both kinds of anchor,
/// [`RawMenuAnchor`] and [`RawMenuAnchorGroup`].
///
/// A leaf's `State` hooks call this trait's `init_state`, `did_change_dependencies`, `dispose`
/// and `build` by name, where Dart runs the mixin's body through `super`.
trait RawMenuAnchorBaseMixin: State {
    /// Dart's `_RawMenuAnchorBaseMixin` fields, held under the field `raw_menu_anchor_base`.
    fn raw_menu_anchor_base_data(self: Handle<Self>, app: &App) -> &RawMenuAnchorBaseMixinData;

    /// See [`raw_menu_anchor_base_data`](Self::raw_menu_anchor_base_data).
    fn raw_menu_anchor_base_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RawMenuAnchorBaseMixinData;

    /// This anchor as the erased [`AnyRawMenuAnchorBase`].
    fn as_raw_menu_anchor_base(self: Handle<Self>) -> AnyRawMenuAnchorBase {
        AnyRawMenuAnchorBase {
            id: self.id(),
            vtable: const { &RawMenuAnchorBaseVTable::of::<Self>() },
        }
    }

    /// Dart's `this is _RawMenuAnchorState`: the [`RawMenuAnchorState`] fills it.
    fn as_raw_menu_anchor_state(self: Handle<Self>) -> Option<Handle<RawMenuAnchorState>> {
        None
    }

    /// Whether this anchor is the top node of the menu tree.
    fn is_root(self: Handle<Self>, app: &App) -> bool {
        self.raw_menu_anchor_base_data(app).parent.is_none()
    }

    /// The [`MenuController`] that is used by this anchor.
    fn menu_controller(self: Handle<Self>, app: &App) -> AnyMenuController;

    /// Whether this submenu's overlay is visible.
    fn is_open(self: Handle<Self>, app: &App) -> bool;

    /// The root of the menu tree that this anchor is in.
    fn root(self: Handle<Self>, app: &App) -> AnyRawMenuAnchorBase {
        self.as_raw_menu_anchor_base().root(app)
    }

    /// Dart's `initState` body.
    fn init_state(self: Handle<Self>, app: &mut App) {
        let anchor = self.as_raw_menu_anchor_base();
        let controller = self.menu_controller(app);
        controller.attach(app, anchor);
    }

    /// Dart's `didChangeDependencies` body.
    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let new_parent =
            MenuController::maybe_of(app, context).and_then(|controller| controller.anchor(app));
        let parent = self.raw_menu_anchor_base_data(app).parent;
        if new_parent != parent {
            debug_assert!(
                new_parent != Some(self.as_raw_menu_anchor_base()),
                "A MenuController should only be attached to one anchor at a time."
            );
            if let Some(parent) = parent {
                parent.remove_child(app, self.as_raw_menu_anchor_base());
            }
            self.raw_menu_anchor_base_data_mut(app).parent = new_parent;
            if let Some(new_parent) = new_parent {
                new_parent.add_child(app, self.as_raw_menu_anchor_base());
            }
        }

        if self.is_root(app) {
            if let Some(position) = self.raw_menu_anchor_base_data(app).scroll_position.handle() {
                position
                    .is_scrolling_notifier(app)
                    .remove_listener(app, &Self::handle_scroll_tear_off(self));
            }
            let position =
                Scrollable::maybe_of(app, context, None).map(|state| state.position(app));
            let kept_position = position.map(|position| position.keep(app));
            self.raw_menu_anchor_base_data_mut(app).scroll_position = kept_position;
            if let Some(position) = position {
                position
                    .is_scrolling_notifier(app)
                    .add_listener(app, Self::handle_scroll_tear_off(self));
            }

            let new_size = MediaQuery::size_of(app, context);
            let view_size = self.raw_menu_anchor_base_data(app).view_size;
            if view_size.is_some_and(|view_size| new_size != view_size) && self.is_open(app) {
                // Close the menus if the view changes size.
                self.handle_close_request(app);
            }
            self.raw_menu_anchor_base_data_mut(app).view_size = Some(new_size);
        }
    }

    /// Dart's `dispose` body.
    fn dispose(self: Handle<Self>, app: &mut App) {
        if self.is_open(app) {
            self.close(app, true);
        }

        if self.is_root(app) {
            if let Some(position) = self.raw_menu_anchor_base_data(app).scroll_position.handle() {
                position
                    .is_scrolling_notifier(app)
                    .remove_listener(app, &Self::handle_scroll_tear_off(self));
            }
            self.raw_menu_anchor_base_data_mut(app).scroll_position = None;
        }

        if let Some(parent) = self.raw_menu_anchor_base_data(app).parent {
            parent.remove_child(app, self.as_raw_menu_anchor_base());
        }
        let data = self.raw_menu_anchor_base_data_mut(app);
        data.parent = None;
        data.anchor_children.clear();
        let anchor = self.as_raw_menu_anchor_base();
        self.menu_controller(app).detach(app, anchor);
    }

    /// Dart's `_addChild`.
    fn add_child(self: Handle<Self>, app: &mut App, child: AnyRawMenuAnchorBase) {
        let children = &mut self.raw_menu_anchor_base_data_mut(app).anchor_children;
        debug_assert!(!children.contains(&child));
        children.push(child);
    }

    /// Dart's `_removeChild`.
    fn remove_child(self: Handle<Self>, app: &mut App, child: AnyRawMenuAnchorBase) {
        let children = &mut self.raw_menu_anchor_base_data_mut(app).anchor_children;
        debug_assert!(children.contains(&child));
        if let Some(index) = children.iter().position(|entry| *entry == child) {
            children.remove(index);
        }
    }

    /// Dart's `_handleScroll`.
    fn handle_scroll(self: Handle<Self>, app: &mut App) {
        // If an ancestor scrolls, and we're a root anchor, then close the menus. Don't just
        // close it on *any* scroll, since we want to be able to scroll menus themselves if
        // they're too big for the view.
        if self.is_open(app) {
            self.handle_close_request(app);
        }
    }

    /// The tear-off Dart passes to `isScrollingNotifier.addListener` and `removeListener`.
    fn handle_scroll_tear_off(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::handle_scroll)
    }

    /// Dart's `_childChangedOpenState`.
    fn child_changed_open_state(self: Handle<Self>, app: &mut App) {
        if let Some(parent) = self.raw_menu_anchor_base_data(app).parent {
            parent.child_changed_open_state(app);
        }
        if SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks {
            // Mark dirty now, but only if not in a build.
            self.set_state(app, |_| {});
        } else {
            let view = View::maybe_id_of(app, self.context(app));
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    self.set_state(app, |_| {});
                }),
                view,
            );
        }
    }

    /// Open the menu, optionally at a position relative to the anchor.
    ///
    /// Call this when the menu overlay should be shown and added to the widget tree.
    ///
    /// The optional `position` argument should specify the location of the menu in the local
    /// coordinates of the anchor.
    fn open(self: Handle<Self>, app: &mut App, position: Option<Offset>);

    /// Close the menu and all of its children.
    ///
    /// Call this when the menu overlay should be hidden and removed from the widget tree.
    ///
    /// If `in_dispose` is true, this method call was triggered by the widget being unmounted.
    fn close(self: Handle<Self>, app: &mut App, in_dispose: bool);

    /// Implemented by leaves to define what to do when [`MenuController::open`] is called.
    ///
    /// This method should not be directly called by leaves. Its call chain should eventually
    /// invoke [`open`](Self::open).
    fn handle_open_request(self: Handle<Self>, app: &mut App, position: Option<Offset>);

    /// Implemented by leaves to define what to do when [`MenuController::close`] is called.
    ///
    /// This method should not be directly called by leaves. Its call chain should eventually
    /// invoke [`close`](Self::close).
    fn handle_close_request(self: Handle<Self>, app: &mut App);

    /// Close the open submenus of this menu.
    ///
    /// This method will call [`close`](Self::close) on each child of this menu, which will
    /// immediately close the child.
    ///
    /// If `in_dispose` is true, this method was triggered by the widget being unmounted.
    ///
    /// See also:
    /// * [`request_children_close`](Self::request_children_close), which triggers the closing
    ///   sequence of each child by calling [`handle_close_request`](Self::handle_close_request)
    ///   on each child.
    fn close_children(self: Handle<Self>, app: &mut App, in_dispose: bool) {
        let children = self.raw_menu_anchor_base_data(app).anchor_children.clone();
        for child in children {
            child.close(app, in_dispose);
        }
    }

    /// Request that the open submenus of this menu be closed.
    ///
    /// This method will call [`handle_close_request`](Self::handle_close_request) on each child
    /// of this menu, which will trigger the closing sequence of each child.
    ///
    /// See also:
    /// * [`close_children`](Self::close_children), which immediately closes each child without
    ///   triggering the closing sequence.
    fn request_children_close(self: Handle<Self>, app: &mut App) {
        let children = self.raw_menu_anchor_base_data(app).anchor_children.clone();
        for child in children {
            child.handle_close_request(app);
        }
    }

    /// Handles taps outside of the menu surface.
    ///
    /// By default, this closes this submenu's children.
    fn handle_outside_tap(
        self: Handle<Self>,
        app: &mut App,
        _pointer_down_event: PointerDownEvent,
    ) {
        if self.is_open(app) {
            self.request_children_close(app);
        }
    }

    /// Dart's `handleOutsideTap` tear-off, as the [`TapRegion`] callback.
    fn handle_outside_tap_callback(self: Handle<Self>) -> TapRegionCallback {
        Rc::new(move |app: &mut App, event: PointerDownEvent| self.handle_outside_tap(app, event))
    }

    /// Used to build the anchor widget in leaves.
    fn build_anchor(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef;

    /// Dart's `build`, which no leaf overrides.
    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let is_open = self.is_open(app);
        let controller = self.menu_controller(app).keep(app);
        let mut actions: HashMap<TypeId, ActionRef> = HashMap::new();
        // Check if open to allow DismissIntent to bubble when the menu is closed.
        if is_open {
            actions.insert(
                <DismissMenuAction as Action>::intent_type(),
                DismissMenuAction::new(app, controller.clone()).into_erased(),
            );
        }
        MenuControllerScope {
            is_open,
            controller,
            child: Actions::new(
                actions,
                Builder::new(move |app, context| self.build_anchor(app, context)),
            )
            .into_widget(),
        }
        .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// _RawMenuAnchorState

/// Dart's `_RawMenuAnchorState`.
pub struct RawMenuAnchorState {
    state: StateData<RawMenuAnchor>,
    raw_menu_anchor_base: RawMenuAnchorBaseMixinData,
    // The global key used to determine the bounding rect for the anchor.
    anchor_key: GlobalKey,
    // Dart's field initializer; an arena object needs the `App`, so `init_state` makes it.
    overlay_controller: Option<Ref<OverlayPortalController>>,
    menu_position: Option<Offset>,
}

impl RawMenuAnchorState {
    fn overlay_controller(self: Handle<Self>, app: &App) -> Handle<OverlayPortalController> {
        app.get(self)
            .overlay_controller
            .handle()
            .expect("created in init_state")
    }

    /// Dart's `_isRootOverlayAnchor`.
    fn is_root_overlay_anchor(self: Handle<Self>, app: &App) -> bool {
        self.raw_menu_anchor_base_data(app)
            .parent
            .and_then(AnyRawMenuAnchorBase::as_raw_menu_anchor_state)
            .is_none()
    }

    // If we are a nested menu, we still want to use the same overlay as the root menu.
    fn use_root_overlay(self: Handle<Self>, app: &App) -> bool {
        if let Some(parent) = self
            .raw_menu_anchor_base_data(app)
            .parent
            .and_then(AnyRawMenuAnchorBase::as_raw_menu_anchor_state)
        {
            return parent.use_root_overlay(app);
        }

        debug_assert!(self.is_root_overlay_anchor(app));
        self.widget(app).use_root_overlay
    }

    /// Dart's `close` tear-off, as the `hide_overlay` callback.
    fn close_callback(self: Handle<Self>) -> VoidCallback {
        Rc::new(move |app: &mut App| {
            // Disposed: Dart's `close` then finds the overlay hidden and does nothing, but a
            // Rust handle to it is gone.
            if self.mounted(app) {
                self.close(app, false);
            }
        })
    }

    fn build_overlay(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        layout_info: OverlayChildLayoutInfo,
    ) -> WidgetRef {
        let transform = layout_info.child_paint_transform();
        let anchor_size = layout_info.child_size();

        // Transform the anchor rectangle using the full transform matrix.
        let anchor_rect = transform_rect(&transform, Offset::ZERO & anchor_size);

        let info = RawMenuOverlayInfo {
            anchor_rect,
            overlay_size: layout_info.overlay_size(),
            position: app.get(self).menu_position,
            tap_region_group_id: self.root(app).menu_controller(app).id().into(),
        };

        let overlay_builder = Rc::clone(&self.widget(app).overlay_builder);
        overlay_builder(app, context, info)
    }
}

impl RawMenuAnchorBaseMixin for RawMenuAnchorState {
    fn raw_menu_anchor_base_data(self: Handle<Self>, app: &App) -> &RawMenuAnchorBaseMixinData {
        &app.get(self).raw_menu_anchor_base
    }

    fn raw_menu_anchor_base_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RawMenuAnchorBaseMixinData {
        &mut app.get_mut(self).raw_menu_anchor_base
    }

    fn as_raw_menu_anchor_state(self: Handle<Self>) -> Option<Handle<RawMenuAnchorState>> {
        Some(self)
    }

    fn is_open(self: Handle<Self>, app: &App) -> bool {
        self.overlay_controller(app).is_showing(app)
    }

    fn menu_controller(self: Handle<Self>, app: &App) -> AnyMenuController {
        self.widget(app).controller.handle()
    }

    fn open(self: Handle<Self>, app: &mut App, position: Option<Offset>) {
        if !self.mounted(app) {
            return;
        }

        if self.is_open(app) {
            // The menu is already open, but we need to move to another location, so close it
            // first.
            self.close(app, false);
        }

        // Close all siblings.
        let parent = self.raw_menu_anchor_base_data(app).parent;
        if let Some(parent) = parent {
            parent.request_children_close(app);
        }
        debug_assert!(!self.overlay_controller(app).is_showing(app));
        app.get_mut(self).menu_position = position;
        if let Some(parent) = parent {
            parent.child_changed_open_state(app);
        }
        self.overlay_controller(app).show(app);

        if self.is_root_overlay_anchor(app)
            && let Some(focus_node) = self.widget(app).child_focus_node.handle()
        {
            focus_node.request_focus(app, None);
        }

        if let Some(on_open) = self.widget(app).on_open.clone() {
            on_open(app);
        }
        // Mark dirty to notify MenuController dependents.
        self.set_state(app, |_| {});
    }

    fn close(self: Handle<Self>, app: &mut App, in_dispose: bool) {
        if !self.is_open(app) {
            return;
        }

        // Parents will close after children do.
        self.close_children(app, in_dispose);

        // Don't hide if we're in the middle of a build.
        if SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks {
            self.overlay_controller(app).hide(app);
        } else if !in_dispose {
            let view = View::maybe_id_of(app, self.context(app));
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    // Disposed within the frame: its overlay controller went with it.
                    if self.mounted(app) {
                        self.overlay_controller(app).hide(app);
                    }
                }),
                view,
            );
        }

        if !in_dispose {
            // Notify that _childIsOpen changed state, but only if not currently disposing.
            if let Some(parent) = self.raw_menu_anchor_base_data(app).parent {
                parent.child_changed_open_state(app);
            }
            if let Some(on_close) = self.widget(app).on_close.clone() {
                on_close(app);
            }
            if self.mounted(app)
                && SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks
            {
                // Mark dirty, but only if mounted and not in a build.
                self.set_state(app, |_| {});
            }
        }
    }

    fn handle_open_request(self: Handle<Self>, app: &mut App, position: Option<Offset>) {
        let on_open_requested = Rc::clone(&self.widget(app).on_open_requested);
        on_open_requested(
            app,
            position,
            Rc::new(move |app: &mut App| self.open(app, position)),
        );
    }

    fn handle_close_request(self: Handle<Self>, app: &mut App) {
        // Changes in MediaQuery::size_of(context) cause RawMenuAnchor to close during
        // did_change_dependencies. When this happens, calling set_state during the closing
        // sequence (handle_close_request -> on_close_requested -> hide_overlay) would schedule
        // a build during a build. We avoid this by checking if we're in a build, and if so, we
        // schedule the close for the next frame.
        if SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks {
            let on_close_requested = Rc::clone(&self.widget(app).on_close_requested);
            on_close_requested(app, self.close_callback());
        } else {
            let view = View::maybe_id_of(app, self.context(app));
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _timestamp| {
                    if self.mounted(app) {
                        let on_close_requested = Rc::clone(&self.widget(app).on_close_requested);
                        on_close_requested(app, self.close_callback());
                    }
                }),
                view,
            );
        }

        // Parents will request close before children do.
        self.request_children_close(app);
    }

    fn build_anchor(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let root = self.root(app);
        let consume_outside_taps = root.is_open(app) && self.widget(app).consume_outside_taps;
        let anchor_key = app.get(self).anchor_key.clone();
        let child = Shortcuts::new(
            menu_traversal_shortcuts(),
            TapRegion::new(
                Builder::new(move |app, context| {
                    let widget = self.widget(app);
                    let child = widget.child.clone();
                    match widget.builder.clone() {
                        Some(builder) => {
                            let controller = self.menu_controller(app);
                            builder(app, context, controller, child.as_ref())
                        }
                        None => child.unwrap_or_else(|| SizedBox::new().into_widget()),
                    }
                })
                .key(anchor_key.into()),
            )
            .group_id(root.menu_controller(app).id())
            .consume_outside_taps(consume_outside_taps)
            .on_tap_outside(self.handle_outside_tap_callback()),
        )
        .include_semantics(false);

        let overlay_location = if self.use_root_overlay(app) {
            OverlayChildLocation::RootOverlay
        } else {
            OverlayChildLocation::NearestOverlay
        };
        let overlay_controller = app
            .get(self)
            .overlay_controller
            .clone()
            .expect("created in init_state");
        OverlayPortal::overlay_child_layout_builder(
            overlay_controller,
            move |app, context, layout_info| self.build_overlay(app, context, layout_info),
        )
        .overlay_location(overlay_location)
        .child(child)
        .into_widget()
    }
}

impl State for RawMenuAnchorState {
    type Widget = RawMenuAnchor;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let debug_label = cfg!(debug_assertions).then(|| "MenuAnchor controller".to_string());
        let overlay_controller = OverlayPortalController::new(app, debug_label);
        app.get_mut(self).overlay_controller = Some(overlay_controller);
        RawMenuAnchorBaseMixin::init_state(self, app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        RawMenuAnchorBaseMixin::did_change_dependencies(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &RawMenuAnchor) {
        let controller = self.widget(app).controller.handle();
        if old_widget.controller != controller {
            let anchor = self.as_raw_menu_anchor_base();
            old_widget.controller.detach(app, anchor);
            controller.attach(app, anchor);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        RawMenuAnchorBaseMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        RawMenuAnchorBaseMixin::build(self, app, context)
    }
}

// ---------------------------------------------------------------------------------------------
// RawMenuAnchorGroup

/// Creates a menu anchor that is always visible and is not displayed in an [`OverlayPortal`].
///
/// A [`RawMenuAnchorGroup`] can be used to create a menu bar that handles external taps and
/// keyboard shortcuts, but defines no default focus or keyboard traversal to enable more
/// flexibility.
///
/// When a [`MenuController`] is given to a [`RawMenuAnchorGroup`],
///  - [`MenuController::open`] has no effect.
///  - [`MenuController::close`] closes all child [`RawMenuAnchor`]s that are open.
///  - [`MenuController::is_open`] reflects whether any child [`RawMenuAnchor`] is open.
///
/// A [`child`](Self::child) must be provided.
///
/// See also:
/// * [`RawMenuAnchor`], a widget that defines a region attached to a floating submenu.
pub struct RawMenuAnchorGroup {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The child displayed by the [`RawMenuAnchorGroup`].
    ///
    /// To access the [`MenuController`] from the [`child`](Self::child), place the child in a
    /// builder and call [`MenuController::maybe_of`].
    pub child: WidgetRef,

    /// A [`MenuController`] that allows the closing of the menu from other widgets.
    pub controller: MenuControllerRef,
}

impl RawMenuAnchorGroup {
    /// Creates a [`RawMenuAnchorGroup`].
    pub fn new<K>(child: impl IntoWidget<K>, controller: MenuControllerRef) -> RawMenuAnchorGroup {
        RawMenuAnchorGroup {
            key: None,
            child: child.into_widget(),
            controller,
        }
    }

    /// Dart `RawMenuAnchorGroup(key:)`.
    pub fn key(mut self, key: KeyRef) -> RawMenuAnchorGroup {
        self.key = Some(key);
        self
    }
}

impl Debug for RawMenuAnchorGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawMenuAnchorGroup")
            .field("controller", &"has controller")
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for RawMenuAnchorGroup {
    type State = RawMenuAnchorGroupState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RawMenuAnchorGroupState {
        RawMenuAnchorGroupState {
            state: StateData::new(),
            raw_menu_anchor_base: RawMenuAnchorBaseMixinData::new(),
        }
    }
}

/// Dart's `_RawMenuAnchorGroupState`.
pub struct RawMenuAnchorGroupState {
    state: StateData<RawMenuAnchorGroup>,
    raw_menu_anchor_base: RawMenuAnchorBaseMixinData,
}

impl RawMenuAnchorBaseMixin for RawMenuAnchorGroupState {
    fn raw_menu_anchor_base_data(self: Handle<Self>, app: &App) -> &RawMenuAnchorBaseMixinData {
        &app.get(self).raw_menu_anchor_base
    }

    fn raw_menu_anchor_base_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RawMenuAnchorBaseMixinData {
        &mut app.get_mut(self).raw_menu_anchor_base
    }

    fn is_open(self: Handle<Self>, app: &App) -> bool {
        self.raw_menu_anchor_base_data(app)
            .anchor_children
            .iter()
            .any(|child| child.is_open(app))
    }

    fn menu_controller(self: Handle<Self>, app: &App) -> AnyMenuController {
        self.widget(app).controller.handle()
    }

    fn close(self: Handle<Self>, app: &mut App, in_dispose: bool) {
        if !self.is_open(app) {
            return;
        }

        self.close_children(app, in_dispose);
        if !in_dispose {
            if SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks {
                // Mark dirty, but only if mounted and not in a build.
                self.set_state(app, |_| {});
            } else {
                let view = View::maybe_id_of(app, self.context(app));
                SchedulerBinding::add_post_frame_callback(
                    app,
                    FrameCallback::new(move |app, _timestamp| {
                        if self.mounted(app) {
                            // Mark dirty.
                            self.set_state(app, |_| {});
                        }
                    }),
                    view,
                );
            }
        }
    }

    fn open(self: Handle<Self>, app: &mut App, _position: Option<Offset>) {
        debug_assert!(
            self.menu_controller(app).anchor(app) == Some(self.as_raw_menu_anchor_base())
        );
        // Menu nodes are always open, so this is a no-op.
    }

    fn handle_close_request(self: Handle<Self>, app: &mut App) {
        self.request_children_close(app);
    }

    fn handle_open_request(self: Handle<Self>, app: &mut App, position: Option<Offset>) {
        self.open(app, position);
    }

    fn build_anchor(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let child = self.widget(app).child.clone();
        TapRegion::new(child)
            .group_id(self.root(app).menu_controller(app).id())
            .on_tap_outside(self.handle_outside_tap_callback())
            .into_widget()
    }
}

impl State for RawMenuAnchorGroupState {
    type Widget = RawMenuAnchorGroup;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        RawMenuAnchorBaseMixin::init_state(self, app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        RawMenuAnchorBaseMixin::did_change_dependencies(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &RawMenuAnchorGroup) {
        let controller = self.widget(app).controller.handle();
        if old_widget.controller != controller {
            let anchor = self.as_raw_menu_anchor_base();
            old_widget.controller.detach(app, anchor);
            controller.attach(app, anchor);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        RawMenuAnchorBaseMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        RawMenuAnchorBaseMixin::build(self, app, context)
    }
}

// ---------------------------------------------------------------------------------------------
// MenuController

/// The fields of Dart's `MenuController`, held under the field `menu_controller`
/// ([`menu_controller_accessors!`](crate::menu_controller_accessors)).
pub struct MenuControllerData {
    // The anchor that this controller controls.
    //
    // This is set automatically when this `MenuController` is attached to an anchor.
    anchor: Option<AnyRawMenuAnchorBase>,
}

impl MenuControllerData {
    /// The bag of a controller attached to no anchor.
    pub fn new() -> MenuControllerData {
        MenuControllerData { anchor: None }
    }
}

impl Default for MenuControllerData {
    fn default() -> MenuControllerData {
        MenuControllerData::new()
    }
}

/// The accessors [`MenuControllerLeaf`] asks for, for a struct whose bag is the field
/// `menu_controller`.
#[macro_export]
macro_rules! menu_controller_accessors {
    () => {
        fn menu_controller_data(
            self: ::waft_foundation::Handle<Self>,
            app: &::waft_foundation::App,
        ) -> &$crate::MenuControllerData {
            &app.get(self).menu_controller
        }

        fn menu_controller_data_mut(
            self: ::waft_foundation::Handle<Self>,
            app: &mut ::waft_foundation::App,
        ) -> &mut $crate::MenuControllerData {
            &mut app.get_mut(self).menu_controller
        }
    };
}

/// The trait every menu-controller leaf implements: [`MenuController`], and a type an app
/// writes where Dart extends `MenuController`.
///
/// The members a subclass overrides are the ones with a default body here; the shared bodies
/// are associated functions on [`MenuController`], which an override calls where Dart writes
/// `super.…`.
pub trait MenuControllerLeaf: Sized + 'static {
    /// Dart's `MenuController` fields, held under the field `menu_controller`
    /// ([`menu_controller_accessors!`](crate::menu_controller_accessors)).
    fn menu_controller_data(self: Handle<Self>, app: &App) -> &MenuControllerData;

    /// See [`menu_controller_data`](Self::menu_controller_data).
    fn menu_controller_data_mut(self: Handle<Self>, app: &mut App) -> &mut MenuControllerData;

    /// Whether or not the menu associated with this [`MenuController`] is open.
    fn is_open(self: Handle<Self>, app: &App) -> bool {
        MenuController::is_open(self.as_menu_controller(), app)
    }

    /// Opens the menu that this [`MenuController`] is associated with.
    ///
    /// If `position` is given, then the menu will open at the position given, in the
    /// coordinate space of the [`RawMenuAnchor`] that this controller is attached to.
    ///
    /// If the menu's anchor point is scrolled by an ancestor, or the view changes size, then
    /// any open menu will automatically close.
    fn open(self: Handle<Self>, app: &mut App, position: Option<Offset>) {
        MenuController::open(self.as_menu_controller(), app, position);
    }

    /// Close the menu that this [`MenuController`] is associated with.
    ///
    /// Associating with a menu is done by passing a [`MenuController`] to a [`RawMenuAnchor`]
    /// or a [`RawMenuAnchorGroup`].
    ///
    /// If the menu's anchor point is scrolled by an ancestor, or the view changes size, then
    /// any open menu will automatically close.
    fn close(self: Handle<Self>, app: &mut App) {
        MenuController::close(self.as_menu_controller(), app);
    }

    /// Close the children of the menu associated with this [`MenuController`], without closing
    /// the menu itself.
    fn close_children(self: Handle<Self>, app: &mut App) {
        MenuController::close_children(self.as_menu_controller(), app);
    }

    /// This controller as the erased [`AnyMenuController`]: what to pass where a Dart API takes
    /// a `MenuController`.
    fn as_menu_controller(self: Handle<Self>) -> AnyMenuController {
        AnyMenuController {
            id: self.id(),
            vtable: const { &MenuControllerVTable::of::<Self>() },
        }
    }
}

/// A controller used to manage a menu created by a [`RawMenuAnchor`] or a
/// [`RawMenuAnchorGroup`], or by a widget built on them, such as Material's `MenuAnchor`,
/// `MenuBar` and `SubmenuButton`.
///
/// A [`MenuController`] is used to control and interrogate a menu after it has been created,
/// with methods such as [`open`](MenuControllerLeaf::open) and
/// [`close`](MenuControllerLeaf::close), and state accessors like
/// [`is_open`](MenuControllerLeaf::is_open).
///
/// [`MenuController::maybe_of`] can be used to retrieve a controller from the [`BuildContext`]
/// of a widget that is a descendant of a [`RawMenuAnchor`] or a [`RawMenuAnchorGroup`]. Doing
/// so will not establish a dependency relationship.
///
/// A controller is an arena object: whoever keeps the [`Ref`] [`new`](Self::new) answers holds
/// it, and so does every anchor widget it is given to. A type that overrides Dart's members is a
/// second [`MenuControllerLeaf`].
///
/// The associated functions on this type are the shared bodies of Dart's `MenuController`; a
/// leaf calls one where Dart writes `super.…`.
pub struct MenuController {
    menu_controller: MenuControllerData,
}

impl MenuController {
    /// Creates a [`MenuController`], attached to no anchor.
    pub fn new(app: &mut App) -> Ref<MenuController> {
        app.create(MenuController {
            menu_controller: MenuControllerData::new(),
        })
    }

    /// See [`MenuControllerLeaf::is_open`].
    pub fn is_open(controller: AnyMenuController, app: &App) -> bool {
        controller
            .anchor(app)
            .is_some_and(|anchor| anchor.is_open(app))
    }

    /// See [`MenuControllerLeaf::open`].
    pub fn open(controller: AnyMenuController, app: &mut App, position: Option<Offset>) {
        let anchor = controller.anchor(app);
        debug_assert!(anchor.is_some(), "_anchor != null");
        anchor
            .expect("a MenuController opened while attached to an anchor")
            .handle_open_request(app, position);
    }

    /// See [`MenuControllerLeaf::close`].
    pub fn close(controller: AnyMenuController, app: &mut App) {
        if let Some(anchor) = controller.anchor(app) {
            anchor.handle_close_request(app);
        }
    }

    /// See [`MenuControllerLeaf::close_children`].
    pub fn close_children(controller: AnyMenuController, app: &mut App) {
        let anchor = controller.anchor(app);
        debug_assert!(anchor.is_some(), "_anchor != null");
        anchor
            .expect("a MenuController's children closed while attached to an anchor")
            .request_children_close(app);
    }

    /// Returns the [`MenuController`] of the ancestor [`RawMenuAnchor`] nearest to the given
    /// `context`, if one exists. Otherwise, returns `None`.
    ///
    /// This method will not establish a dependency relationship, so the calling widget will not
    /// rebuild when the menu opens and closes, nor when the [`MenuController`] changes.
    pub fn maybe_of(app: &App, context: BuildContext) -> Option<AnyMenuController> {
        context
            .get_inherited_widget_of_exact_type::<MenuControllerScope>(app)
            .map(|scope| scope.controller.handle())
    }

    /// Returns the value of [`MenuControllerLeaf::is_open`] of the ancestor [`RawMenuAnchor`]
    /// or [`RawMenuAnchorGroup`] nearest to the given `context`, if one exists. Otherwise,
    /// returns `None`.
    ///
    /// This method will establish a dependency relationship, so the calling widget will rebuild
    /// when the menu opens and closes.
    pub fn maybe_is_open_of(app: &mut App, context: BuildContext) -> Option<bool> {
        context
            .depend_on_inherited_widget_of_exact_type::<MenuControllerScope>(app)
            .map(|scope| scope.is_open)
    }
}

impl MenuControllerLeaf for MenuController {
    crate::menu_controller_accessors!();
}

impl Debug for MenuController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MenuController")
    }
}

/// The vtable of an erased [`AnyMenuController`]: one `&'static` table per leaf type.
struct MenuControllerVTable {
    type_name: fn() -> &'static str,
    data: fn(&App, HandleId) -> &MenuControllerData,
    data_mut: fn(&mut App, HandleId) -> &mut MenuControllerData,
    is_open: fn(&App, HandleId) -> bool,
    open: fn(&mut App, HandleId, Option<Offset>),
    close: fn(&mut App, HandleId),
    close_children: fn(&mut App, HandleId),
}

impl MenuControllerVTable {
    const fn of<C: MenuControllerLeaf>() -> MenuControllerVTable {
        MenuControllerVTable {
            type_name: std::any::type_name::<C>,
            data: |app, id| C::menu_controller_data(resolve(id), app),
            data_mut: |app, id| C::menu_controller_data_mut(resolve(id), app),
            is_open: |app, id| C::is_open(resolve(id), app),
            open: |app, id, position| C::open(resolve(id), app, position),
            close: |app, id| C::close(resolve(id), app),
            close_children: |app, id| C::close_children(resolve(id), app),
        }
    }
}

/// Erased [`MenuControllerLeaf`]: what a field or parameter Dart types as `MenuController`
/// becomes.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyMenuController {
    id: HandleId,
    vtable: &'static MenuControllerVTable,
}

/// A field Dart types as the erased object, kept: AnyMenuController with its count.
pub type MenuControllerRef = Retained<AnyMenuController>;

waft_foundation::erased_handle!(AnyMenuController: MenuControllerLeaf => MenuControllerLeaf::as_menu_controller);

impl AnyMenuController {
    /// The controller's arena id, which is also its anchors' [`TapRegion::group_id`].
    pub fn id(self) -> HandleId {
        self.id
    }

    fn anchor(self, app: &App) -> Option<AnyRawMenuAnchorBase> {
        (self.vtable.data)(app, self.id).anchor
    }

    /// See [`MenuControllerLeaf::is_open`].
    pub fn is_open(self, app: &App) -> bool {
        (self.vtable.is_open)(app, self.id)
    }

    /// See [`MenuControllerLeaf::open`].
    pub fn open(self, app: &mut App, position: Option<Offset>) {
        (self.vtable.open)(app, self.id, position);
    }

    /// See [`MenuControllerLeaf::close`].
    pub fn close(self, app: &mut App) {
        (self.vtable.close)(app, self.id);
    }

    /// See [`MenuControllerLeaf::close_children`].
    pub fn close_children(self, app: &mut App) {
        (self.vtable.close_children)(app, self.id);
    }

    /// Dart's `_attach`.
    fn attach(self, app: &mut App, anchor: AnyRawMenuAnchorBase) {
        (self.vtable.data_mut)(app, self.id).anchor = Some(anchor);
    }

    /// Dart's `_detach`.
    fn detach(self, app: &mut App, anchor: AnyRawMenuAnchorBase) {
        let data = (self.vtable.data_mut)(app, self.id);
        if data.anchor == Some(anchor) {
            data.anchor = None;
        }
    }
}

impl Debug for AnyMenuController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = (self.vtable.type_name)();
        let short = name.rsplit("::").next().unwrap_or(name);
        write!(f, "{short}({:?})", self.id)
    }
}

// ---------------------------------------------------------------------------------------------
// DismissMenuAction

/// An action that closes all the menus associated with the given [`MenuController`].
///
/// See also:
///
///  * [`RawMenuAnchor`], a widget that hosts a cascading submenu.
///  * [`MenuController`], a controller used to manage menus created by a [`RawMenuAnchor`].
pub struct DismissMenuAction {
    action: ActionData,

    /// The [`MenuController`] that manages the menu which should be dismissed upon invocation.
    pub controller: MenuControllerRef,
}

impl DismissMenuAction {
    /// Creates a [`DismissMenuAction`].
    pub fn new(app: &mut App, controller: MenuControllerRef) -> Ref<DismissMenuAction> {
        app.create(DismissMenuAction {
            action: ActionData::new(),
            controller,
        })
    }
}

impl Action for DismissMenuAction {
    type Intent = DismissIntent;

    crate::action_accessors!();

    fn invoke(self: Handle<Self>, app: &mut App, _intent: &DismissIntent) -> Option<Rc<dyn Any>> {
        let anchor = app
            .get(self)
            .controller
            .anchor(app)
            .expect("a DismissMenuAction invoked while enabled has an anchor");
        anchor.root(app).handle_close_request(app);
        None
    }

    fn is_enabled(self: Handle<Self>, app: &mut App, _intent: &DismissIntent) -> bool {
        app.get(self).controller.anchor(app).is_some()
    }
}

impl DismissAction for DismissMenuAction {}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell, RefMut};
    use std::time::Duration;

    use waft_embedder::{Clip, Color, Matrix4, PointerDeviceKind, TextDirection};
    use waft_foundation::{ChangeNotifier, Timer, UniqueKey, ValueNotifier};
    use waft_gestures::{GestureBinding, PointerEvent, PointerScrollEvent};
    use waft_painting::{AlignmentGeometry, Axis, BoxDecoration, EdgeInsetsGeometry};
    use waft_rendering::{AnyRenderObject, BoxConstraints, MainAxisSize, RenderHandle};
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey};

    use super::*;
    use crate::framework::StatelessWidget;
    use crate::testing::{
        TestPointer, WidgetTester, expect, find, finds_n_widgets, finds_nothing, finds_one_widget,
        test_widgets,
    };
    use crate::widgets::actions::{
        ActionDispatcher, CallbackAction, Intent, IntentRef, PlainActionDispatcher,
        VoidCallbackIntent,
    };
    use crate::widgets::app::WidgetsApp;
    use crate::widgets::basic::{
        Align, Center, ColoredBox, Column, Directionality, IntrinsicWidth, ListBody, Padding,
        Positioned, Row, StateSetter, StatefulBuilder, Transform, UnconstrainedBox,
    };
    use crate::widgets::container::Container;
    use crate::widgets::focus_manager::{FocusNode, primary_focus};
    use crate::widgets::focus_scope::Focus;
    use crate::widgets::focus_traversal::{NextFocusIntent, PreviousFocusIntent};
    use crate::widgets::gesture_detector::GestureDetector;
    use crate::widgets::media_query::MediaQueryData;
    use crate::widgets::navigator::RouteSettingsRef;
    use crate::widgets::overlay::{Overlay, OverlayEntry, RenderTheater};
    use crate::widgets::pages::PageRouteBuilder;
    use crate::widgets::routes::RoutePageBuilder;
    use crate::widgets::scroll_controller::{ScrollController, ScrollControllerRef};
    use crate::widgets::shortcuts::{LogicalKeySet, ShortcutActivatorRef};
    use crate::widgets::single_child_scroll_view::SingleChildScrollView;
    use crate::widgets::text::Text;

    const BLACK: Color = Color::new(0xFF000000);

    // ---- the tester: Flutter's `WidgetTester` and what these cases read of it ----

    /// A case's [`WidgetTester`] with the view resizes, key events and focus reads Dart's cases
    /// use on it.
    struct Tester<'t> {
        tester: &'t mut WidgetTester,
    }

    impl std::ops::Deref for Tester<'_> {
        type Target = WidgetTester;

        fn deref(&self) -> &WidgetTester {
            self.tester
        }
    }

    impl std::ops::DerefMut for Tester<'_> {
        fn deref_mut(&mut self) -> &mut WidgetTester {
            self.tester
        }
    }

    impl<'t> Tester<'t> {
        fn new(tester: &'t mut WidgetTester) -> Tester<'t> {
            Tester { tester }
        }

        fn app(&self) -> RefMut<'_, App> {
            self.tester.cell.borrow_mut()
        }

        /// Dart's `changeSurfaceSize(tester, size)`: `tester.binding.setSurfaceSize(size)`,
        /// which each case's own `App` resets.
        fn change_surface_size(&self, size: Size) {
            self.set_surface_size(Some(size));
        }

        /// Dart's `MediaQueryData.fromView(tester.view)`.
        fn media_query_data(&self) -> MediaQueryData {
            MediaQueryData::from_view(&self.tester.cell.borrow(), &**self.view(), None)
        }

        /// The debug label of the primary focus: Dart's `focusedMenu`.
        fn focused_label(&self) -> Option<String> {
            let mut app = self.app();
            primary_focus(&mut app).and_then(|node| node.debug_label(&app))
        }
    }

    /// A list the callbacks of a test write to.
    fn log() -> Rc<RefCell<Vec<&'static str>>> {
        Rc::new(RefCell::new(Vec::new()))
    }

    /// A counter the callbacks of a test bump.
    fn counter() -> Rc<Cell<u32>> {
        Rc::new(Cell::new(0))
    }

    fn push(log: &Rc<RefCell<Vec<&'static str>>>, tag: &'static str) -> VoidCallback {
        let log = Rc::clone(log);
        Rc::new(move |_app: &mut App| log.borrow_mut().push(tag))
    }

    fn bump(counter: &Rc<Cell<u32>>) -> VoidCallback {
        let counter = Rc::clone(counter);
        Rc::new(move |_app: &mut App| counter.set(counter.get() + 1))
    }

    /// The focus label Dart's `Tag.focusNode` gives.
    fn focus_label(tag: &str) -> String {
        format!("Focus[{tag}]")
    }

    // ---- the test widgets of Flutter's file ----

    /// Dart's `Button`: a focusable button that calls `on_pressed` when tapped.
    struct Button {
        child: WidgetRef,
        on_pressed: Option<VoidCallback>,
        focus_node: Option<FocusNodeRef>,
        constraints: BoxConstraints,
        focus_node_label: Option<String>,
    }

    impl Button {
        fn new<K>(child: impl IntoWidget<K>) -> Button {
            Button {
                child: child.into_widget(),
                on_pressed: None,
                focus_node: None,
                constraints: BoxConstraints::tight_for(Some(225.0), Some(32.0)),
                focus_node_label: None,
            }
        }

        /// Dart's `Button.tag`.
        fn tag(tag: &str) -> Button {
            let mut button = Button::new(Text::new(tag));
            button.focus_node_label = Some(focus_label(tag));
            button
        }

        fn on_pressed(mut self, on_pressed: VoidCallback) -> Button {
            self.on_pressed = Some(on_pressed);
            self
        }

        fn focus_node(mut self, focus_node: FocusNodeRef) -> Button {
            self.focus_node = Some(focus_node);
            self
        }

        fn constraints(mut self, constraints: BoxConstraints) -> Button {
            self.constraints = constraints;
            self
        }
    }

    impl Debug for Button {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Button").finish_non_exhaustive()
        }
    }

    impl StatelessWidget for Button {
        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            let on_pressed = self.on_pressed.clone();
            let detector = GestureDetector::new()
                .on_tap(Listener::new(move |app| {
                    if let Some(on_pressed) = &on_pressed {
                        on_pressed(app);
                    }
                }))
                .child(
                    Container::new()
                        .constraints(self.constraints)
                        .padding(EdgeInsetsGeometry::symmetric(0.0, 12.0))
                        .child(
                            Align::new()
                                .alignment(AlignmentGeometry::CENTER_START)
                                .child(self.child.clone()),
                        ),
                );
            let mut focus = Focus::new(detector);
            if let Some(focus_node) = self.focus_node.clone() {
                focus = focus.focus_node(focus_node);
            }
            if let Some(label) = &self.focus_node_label {
                focus = focus.debug_label(label.clone());
            }
            focus.into_widget()
        }
    }

    /// Dart's `Panel`: the menu's surface, its items in a column.
    fn panel(children: Vec<WidgetRef>) -> WidgetRef {
        let body = SingleChildScrollView::new()
            .clip_behavior(Clip::None)
            .child(ListBody::new().children(children));
        // The menu's items can grow beyond the size of the overlay, but will be clipped by the
        // overlay's bounds.
        UnconstrainedBox::new()
            .clip_behavior(Clip::HardEdge)
            .alignment(AlignmentGeometry::CENTER_START)
            .constrained_axis(Axis::Vertical)
            .child(
                IntrinsicWidth::new().child(
                    Container::new()
                        .clip_behavior(Clip::AntiAlias)
                        .decoration(BoxDecoration::new().color(Color::new(0xFFFFFFFF)))
                        .child(body),
                ),
            )
            .into_widget()
    }

    fn texts(tags: &[&str]) -> Vec<WidgetRef> {
        tags.iter()
            .map(|tag| Text::new(*tag).into_widget())
            .collect()
    }

    /// Dart's `Menu`: a [`RawMenuAnchor`] that shows `menu_panel` below its anchor and makes
    /// its own controller when given none.
    #[derive(Default)]
    struct Menu {
        key: Option<KeyRef>,
        menu_panel: Option<WidgetRef>,
        controller: Option<MenuControllerRef>,
        child: Option<WidgetRef>,
        builder: Option<RawMenuAnchorChildBuilder>,
        focus_node: Option<FocusNodeRef>,
        on_open: Option<VoidCallback>,
        on_close: Option<VoidCallback>,
        use_root_overlay: bool,
        consume_outside_taps: bool,
        overlay_builder: Option<RawMenuAnchorOverlayBuilder>,
        on_open_requested: Option<RawMenuAnchorOpenRequestedCallback>,
        on_close_requested: Option<RawMenuAnchorCloseRequestedCallback>,
    }

    impl Menu {
        fn new() -> Menu {
            Menu::default()
        }

        fn key(mut self, key: KeyRef) -> Menu {
            self.key = Some(key);
            self
        }

        fn menu_panel(mut self, menu_panel: WidgetRef) -> Menu {
            self.menu_panel = Some(menu_panel);
            self
        }

        fn controller(mut self, controller: MenuControllerRef) -> Menu {
            self.controller = Some(controller);
            self
        }

        fn child<K>(mut self, child: impl IntoWidget<K>) -> Menu {
            self.child = Some(child.into_widget());
            self
        }

        fn builder(
            mut self,
            builder: impl Fn(&mut App, BuildContext, AnyMenuController, Option<&WidgetRef>) -> WidgetRef
            + 'static,
        ) -> Menu {
            self.builder = Some(Rc::new(builder));
            self
        }

        fn focus_node(mut self, focus_node: FocusNodeRef) -> Menu {
            self.focus_node = Some(focus_node);
            self
        }

        fn on_open(mut self, on_open: VoidCallback) -> Menu {
            self.on_open = Some(on_open);
            self
        }

        fn on_close(mut self, on_close: VoidCallback) -> Menu {
            self.on_close = Some(on_close);
            self
        }

        fn use_root_overlay(mut self, use_root_overlay: bool) -> Menu {
            self.use_root_overlay = use_root_overlay;
            self
        }

        fn consume_outside_taps(mut self, consume_outside_taps: bool) -> Menu {
            self.consume_outside_taps = consume_outside_taps;
            self
        }

        fn overlay_builder(
            mut self,
            overlay_builder: impl Fn(&mut App, BuildContext, RawMenuOverlayInfo) -> WidgetRef + 'static,
        ) -> Menu {
            self.overlay_builder = Some(Rc::new(overlay_builder));
            self
        }

        fn on_open_requested(
            mut self,
            on_open_requested: impl Fn(&mut App, Option<Offset>, VoidCallback) + 'static,
        ) -> Menu {
            self.on_open_requested = Some(Rc::new(on_open_requested));
            self
        }

        fn on_close_requested(
            mut self,
            on_close_requested: impl Fn(&mut App, VoidCallback) + 'static,
        ) -> Menu {
            self.on_close_requested = Some(Rc::new(on_close_requested));
            self
        }
    }

    impl Debug for Menu {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Menu").finish_non_exhaustive()
        }
    }

    impl StatefulWidget for Menu {
        type State = MenuState;

        fn key(&self) -> Option<&KeyRef> {
            self.key.as_ref()
        }

        fn create_state(&self) -> MenuState {
            MenuState {
                state: StateData::new(),
                controller: None,
            }
        }
    }

    struct MenuState {
        state: StateData<Menu>,
        controller: Option<MenuControllerRef>,
    }

    impl State for MenuState {
        type Widget = Menu;
        crate::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let controller = match self.widget(app).controller.clone() {
                Some(controller) => controller,
                None => match app.get(self).controller.clone() {
                    Some(controller) => controller,
                    None => {
                        let controller =
                            MenuController::new(app).into_erased::<AnyMenuController>();
                        app.get_mut(self).controller = Some(controller.clone());
                        controller
                    }
                },
            };
            let widget = self.widget(app);
            let overlay_builder = widget.overlay_builder.clone();
            let mut anchor =
                RawMenuAnchor::new(
                    controller,
                    move |app, context, info| match &overlay_builder {
                        Some(overlay_builder) => overlay_builder(app, context, info),
                        None => Positioned::new(
                            self.widget(app)
                                .menu_panel
                                .clone()
                                .expect("a menu without an overlay builder has a panel"),
                        )
                        .top(info.anchor_rect.bottom)
                        .left(info.anchor_rect.left)
                        .into_widget(),
                    },
                )
                .consume_outside_taps(widget.consume_outside_taps)
                .use_root_overlay(widget.use_root_overlay);
            if let Some(focus_node) = widget.focus_node.clone() {
                anchor = anchor.child_focus_node(focus_node);
            }
            if let Some(on_open) = widget.on_open.clone() {
                anchor = anchor.on_open(on_open);
            }
            if let Some(on_close) = widget.on_close.clone() {
                anchor = anchor.on_close(on_close);
            }
            if let Some(on_open_requested) = widget.on_open_requested.clone() {
                anchor = anchor.on_open_requested(on_open_requested);
            }
            if let Some(on_close_requested) = widget.on_close_requested.clone() {
                anchor = anchor.on_close_requested(on_close_requested);
            }
            if let Some(builder) = widget.builder.clone() {
                anchor = anchor.builder(move |app, context, controller, child| {
                    builder(app, context, controller, child)
                });
            }
            if let Some(child) = widget.child.clone() {
                anchor = anchor.child(child);
            }
            anchor.into_widget()
        }
    }

    /// Dart's `void Function(Tag)`.
    type TagCallback = Rc<dyn Fn(&mut App, &'static str)>;

    /// Dart's `AnchorButton`: a [`Button`] that toggles the nearest menu.
    struct AnchorButton {
        tag: &'static str,
        on_pressed: Option<TagCallback>,
        focus_node: Option<FocusNodeRef>,
        constraints: Option<BoxConstraints>,
    }

    impl AnchorButton {
        fn new(tag: &'static str) -> AnchorButton {
            AnchorButton {
                tag,
                on_pressed: None,
                focus_node: None,
                constraints: None,
            }
        }

        fn on_pressed(mut self, on_pressed: TagCallback) -> AnchorButton {
            self.on_pressed = Some(on_pressed);
            self
        }

        fn focus_node(mut self, focus_node: FocusNodeRef) -> AnchorButton {
            self.focus_node = Some(focus_node);
            self
        }

        fn constraints(mut self, constraints: BoxConstraints) -> AnchorButton {
            self.constraints = Some(constraints);
            self
        }
    }

    impl Debug for AnchorButton {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("AnchorButton")
                .field("tag", &self.tag)
                .finish()
        }
    }

    impl StatelessWidget for AnchorButton {
        fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
            let controller =
                MenuController::maybe_of(app, context).map(|controller| controller.keep(app));
            let tag = self.tag;
            let on_pressed = self.on_pressed.clone();
            let mut button = Button::tag(tag).on_pressed(Rc::new(move |app: &mut App| {
                if let Some(on_pressed) = &on_pressed {
                    on_pressed(app, tag);
                }
                if let Some(controller) = &controller {
                    if controller.is_open(app) {
                        controller.close(app);
                    } else {
                        controller.open(app, None);
                    }
                }
            }));
            if let Some(focus_node) = self.focus_node.clone() {
                button = button.focus_node(focus_node);
            }
            if let Some(constraints) = self.constraints {
                button = button.constraints(constraints);
            }
            button.into_widget()
        }
    }

    /// Dart's `App`: a [`WidgetsApp`] whose one page aligns `child` in the view.
    struct TestApp {
        child: WidgetRef,
        alignment: AlignmentGeometry,
    }

    impl TestApp {
        fn new<K>(child: impl IntoWidget<K>) -> TestApp {
            TestApp {
                child: child.into_widget(),
                alignment: AlignmentGeometry::CENTER,
            }
        }
    }

    impl Debug for TestApp {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("TestApp").finish_non_exhaustive()
        }
    }

    impl StatefulWidget for TestApp {
        type State = TestAppState;

        fn create_state(&self) -> TestAppState {
            TestAppState {
                state: StateData::new(),
            }
        }
    }

    struct TestAppState {
        state: StateData<TestApp>,
    }

    impl State for TestAppState {
        type Widget = TestApp;
        crate::state_accessors!();

        fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
            // Dart's `_buildPage` tear-off: the page reads the current `widget.child`, and the
            // navigator rebuilds it when the app changes.
            let page: RoutePageBuilder = Rc::new(move |app, _context, _animation, _secondary| {
                let widget = self.widget(app);
                Directionality::new(
                    TextDirection::Ltr,
                    Align::new()
                        .alignment(widget.alignment)
                        .child(widget.child.clone()),
                )
                .into_widget()
            });
            ColoredBox::new(BLACK)
                .child(
                    WidgetsApp::new(BLACK).on_generate_route(move |app, settings| {
                        let route = PageRouteBuilder::new(app, Rc::clone(&page))
                            .settings(app, RouteSettingsRef::Settings(settings.clone()));
                        Some(route.into_erased())
                    }),
                )
                .into_widget()
        }
    }

    // ---- tests ----

    #[test]
    fn menu_controller_is_open_while_the_overlay_is_shown() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .menu_panel(panel(texts(&["a"])))
                    .child(AnchorButton::new("anchor")),
            ));

            tester.tap(find::text("anchor"));
            tester.pump();

            assert!(controller.is_open(&tester.app()));
            expect(&tester, &find::text("a"), finds_one_widget());

            tester.tap(find::text("anchor"));
            tester.pump();

            assert!(!controller.is_open(&tester.app()));
            expect(&tester, &find::text("a"), finds_nothing());
        });
    }

    #[test]
    fn menu_controller_open_and_close_toggle_overlay_visibility() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let nested_controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .menu_panel(panel(vec![
                        Text::new("a").into_widget(),
                        Menu::new()
                            .controller(nested_controller.clone())
                            .menu_panel(panel(texts(&["b.a"])))
                            .child(AnchorButton::new("b"))
                            .into_widget(),
                    ]))
                    .child(AnchorButton::new("anchor")),
            ));

            // Create the menu. The menu is closed, so no menu items should be found in the widget
            // tree.
            assert!(!controller.is_open(&tester.app()));
            expect(&tester, &find::text("anchor"), finds_one_widget());
            expect(&tester, &find::text("a"), finds_nothing());

            // Open the menu.
            controller.open(&mut tester.app(), None);
            tester.pump();

            assert!(controller.is_open(&tester.app()));
            assert!(!nested_controller.is_open(&tester.app()));
            expect(&tester, &find::text("a"), finds_one_widget());
            expect(&tester, &find::text("b.a"), finds_nothing());

            // Open the nested menu.
            nested_controller.open(&mut tester.app(), None);
            tester.pump();

            assert!(controller.is_open(&tester.app()));
            assert!(nested_controller.is_open(&tester.app()));
            expect(&tester, &find::text("a"), finds_one_widget());
            expect(&tester, &find::text("b.a"), finds_one_widget());

            // Close the menu from the root controller.
            controller.close(&mut tester.app());
            tester.pump();

            // All menus should be closed.
            assert!(!controller.is_open(&tester.app()));
            expect(&tester, &find::text("a"), finds_nothing());
            expect(&tester, &find::text("b.a"), finds_nothing());

            // Open the nested menu.
            controller.open(&mut tester.app(), None);
            tester.pump();

            nested_controller.open(&mut tester.app(), None);
            tester.pump();

            assert!(controller.is_open(&tester.app()));
            assert!(nested_controller.is_open(&tester.app()));
            expect(&tester, &find::text("a"), finds_one_widget());
            expect(&tester, &find::text("b.a"), finds_one_widget());

            // Close the nested menu, but not the root menu.
            nested_controller.close(&mut tester.app());
            tester.pump();

            assert!(controller.is_open(&tester.app()));
            assert!(!nested_controller.is_open(&tester.app()));
            expect(&tester, &find::text("a"), finds_one_widget());
            expect(&tester, &find::text("b.a"), finds_nothing());
        });
    }

    #[test]
    fn close_children_closes_submenus_and_focus_stays_on_the_anchor_button() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let focus_node: FocusNodeRef = FocusNode::new(&mut tester.app()).into_erased();
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .menu_panel(panel(vec![
                        Text::new("a").into_widget(),
                        Menu::new()
                            .focus_node(focus_node.clone())
                            .menu_panel(panel(texts(&["b.a"])))
                            .child(AnchorButton::new("b").focus_node(focus_node.clone()))
                            .into_widget(),
                    ]))
                    .child(AnchorButton::new("anchor")),
            ));

            tester.tap(find::text("anchor"));
            tester.pump();

            tester.tap(find::text("b"));
            tester.pump();

            focus_node.request_focus(&mut tester.app(), None);
            tester.pump();

            expect(&tester, &find::text("b"), finds_one_widget());
            expect(&tester, &find::text("b.a"), finds_one_widget());

            controller.close_children(&mut tester.app());
            tester.pump();

            assert!(controller.is_open(&tester.app()));
            expect(&tester, &find::text("b"), finds_one_widget());
            expect(&tester, &find::text("b.a"), finds_nothing());

            // Focus should stay on the anchor button.
            assert_eq!(primary_focus(&mut tester.app()), Some(focus_node.handle()));
        });
    }

    #[test]
    fn only_one_child_anchor_is_open_at_a_time() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(
                RawMenuAnchor::new(controller, |_app, _context, _position| {
                    Column::new()
                        .children([
                            Menu::new()
                                .menu_panel(panel(texts(&["a.a"])))
                                .child(AnchorButton::new("a"))
                                .into_widget(),
                            Menu::new()
                                .menu_panel(panel(texts(&["b.a"])))
                                .child(AnchorButton::new("b"))
                                .into_widget(),
                        ])
                        .into_widget()
                })
                .child(AnchorButton::new("anchor")),
            ));

            tester.tap(find::text("anchor"));
            tester.pump();

            expect(&tester, &find::text("a"), finds_one_widget());
            expect(&tester, &find::text("b"), finds_one_widget());
            expect(&tester, &find::text("a.a"), finds_nothing());
            expect(&tester, &find::text("b.a"), finds_nothing());

            tester.tap(find::text("a"));
            tester.pump();

            expect(&tester, &find::text("a.a"), finds_one_widget());
            expect(&tester, &find::text("b.a"), finds_nothing());

            tester.tap(find::text("b"));
            tester.pump();

            expect(&tester, &find::text("a.a"), finds_nothing());
            expect(&tester, &find::text("b.a"), finds_one_widget());
        });
    }

    #[test]
    fn context_menus_can_be_nested() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .menu_panel(panel(vec![Button::tag("a.a").into_widget()]))
                    .builder(|_app, _context, _controller, _child| {
                        Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .children([
                                AnchorButton::new("a").into_widget(),
                                Menu::new()
                                    .menu_panel(panel(vec![Button::tag("b.a").into_widget()]))
                                    .child(AnchorButton::new("b"))
                                    .into_widget(),
                            ])
                            .into_widget()
                    }),
            ));

            tester.tap(find::text("a"));
            tester.pump();

            expect(&tester, &find::text("a.a"), finds_one_widget());

            tester.tap(find::text("b"));
            tester.pump();

            expect(&tester, &find::text("b.a"), finds_one_widget());
        });
    }

    #[test]
    fn a_group_is_open_while_a_descendant_menu_is_open() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
                Row::new().children([
                    Menu::new()
                        .menu_panel(panel(texts(&["a.a"])))
                        .child(AnchorButton::new("a"))
                        .into_widget(),
                    // Menu should not need to be a direct descendant.
                    Padding::new(EdgeInsetsGeometry::all(8.0))
                        .child(
                            Menu::new()
                                .menu_panel(panel(texts(&["b.a"])))
                                .child(AnchorButton::new("b")),
                        )
                        .into_widget(),
                ]),
                controller.clone(),
            )));

            assert!(!controller.is_open(&tester.app()));

            tester.tap(find::text("a"));
            tester.pump();

            assert!(controller.is_open(&tester.app()));
            expect(&tester, &find::text("a.a"), finds_one_widget());
            expect(&tester, &find::text("b.a"), finds_nothing());

            tester.tap(find::text("b"));
            tester.pump();
            tester.pump();

            assert!(controller.is_open(&tester.app()));
            expect(&tester, &find::text("a.a"), finds_nothing());
            expect(&tester, &find::text("b.a"), finds_one_widget());

            tester.tap(find::text("b"));
            tester.pump();

            assert!(!controller.is_open(&tester.app()));
            expect(&tester, &find::text("b.a"), finds_nothing());
        });
    }

    #[test]
    fn opening_a_group_does_nothing() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let nested_controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
                Column::new().children([Menu::new()
                    .controller(nested_controller.clone())
                    .menu_panel(panel(texts(&["b.a"])))
                    .child(AnchorButton::new("b"))
                    .into_widget()]),
                controller.clone(),
            )));

            // Create the menu. The menu is closed, so no menu items should be found in the widget
            // tree.
            assert!(!controller.is_open(&tester.app()));
            expect(&tester, &find::text("b"), finds_one_widget());
            expect(&tester, &find::text("b.a"), finds_nothing());

            // Open the menu (which should do nothing).
            controller.open(&mut tester.app(), None);
            tester.pump();

            assert!(!controller.is_open(&tester.app()));
            assert!(!nested_controller.is_open(&tester.app()));
            expect(&tester, &find::text("b"), finds_one_widget());
            expect(&tester, &find::text("b.a"), finds_nothing());
        });
    }

    /// Dart's `[Group] MenuController.close closes children` and `[Group]
    /// MenuController.closeChildren closes children`, which differ in the one call.
    fn a_group_closes_its_children_with(close: fn(AnyMenuController, &mut App)) {
        let tester = &mut WidgetTester::new();
        let mut tester = Tester::new(tester);
        let controller = MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
        let nested_controller =
            MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
        tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
            Column::new().children([Menu::new()
                .controller(nested_controller.clone())
                .menu_panel(panel(texts(&["b.a"])))
                .child(AnchorButton::new("b"))
                .into_widget()]),
            controller.clone(),
        )));

        // Open the nested anchor.
        nested_controller.open(&mut tester.app(), None);
        tester.pump();

        assert!(controller.is_open(&tester.app()));
        assert!(nested_controller.is_open(&tester.app()));
        expect(&tester, &find::text("b"), finds_one_widget());
        expect(&tester, &find::text("b.a"), finds_one_widget());

        // Close the root menu panel.
        close(controller.handle(), &mut tester.app());
        tester.pump();

        assert!(!controller.is_open(&tester.app()));
        assert!(!nested_controller.is_open(&tester.app()));
        expect(&tester, &find::text("b"), finds_one_widget());
        expect(&tester, &find::text("b.a"), finds_nothing());
    }

    #[test]
    fn closing_a_group_closes_its_children() {
        a_group_closes_its_children_with(AnyMenuController::close);
    }

    #[test]
    fn closing_a_groups_children_closes_them() {
        a_group_closes_its_children_with(AnyMenuController::close_children);
    }

    #[test]
    fn submenus_call_on_close_before_their_parents() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let close_order = log();
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_close(push(&close_order, "a"))
                    .menu_panel(panel(vec![
                        Menu::new()
                            .on_close(push(&close_order, "a.a"))
                            .on_close_requested(|_app, _hide_menu| {})
                            .menu_panel(panel(vec![SizedBox::new().into_widget()]))
                            .child(AnchorButton::new("a.a"))
                            .into_widget(),
                    ])),
            ));

            // Open parent menu.
            controller.open(&mut tester.app(), None);
            tester.pump();

            // Open child menu.
            tester.tap(find::text("a.a"));
            tester.pump();

            // Close the root menu panel, which should close all submenus.
            controller.close(&mut tester.app());
            tester.pump();

            assert_eq!(*close_order.borrow(), ["a.a", "a"]);
        });
    }

    #[test]
    fn a_groups_submenus_call_on_close_before_their_parents() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let close_order = log();
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
                Menu::new()
                    .on_close(push(&close_order, "a"))
                    .menu_panel(panel(vec![
                        Menu::new()
                            .on_close(push(&close_order, "a.a"))
                            .menu_panel(panel(vec![SizedBox::new().into_widget()]))
                            .child(AnchorButton::new("a.a"))
                            .into_widget(),
                    ]))
                    .child(AnchorButton::new("a")),
                controller.clone(),
            )));

            // Open parent menu.
            controller.open(&mut tester.app(), None);
            tester.pump();

            // Open child menu.
            tester.tap(find::text("a"));
            tester.pump();

            tester.tap(find::text("a.a"));
            tester.pump();

            // Close the group, which should close all submenus.
            controller.close(&mut tester.app());
            tester.pump();

            assert_eq!(*close_order.borrow(), ["a.a", "a"]);
        });
    }

    #[test]
    fn a_group_shows_one_open_child_anchor_at_a_time() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
                Row::new().children([
                    Menu::new()
                        .menu_panel(panel(texts(&["a.a"])))
                        .child(AnchorButton::new("a"))
                        .into_widget(),
                    Menu::new()
                        .menu_panel(panel(texts(&["b.a"])))
                        .child(AnchorButton::new("b"))
                        .into_widget(),
                ]),
                controller,
            )));

            expect(&tester, &find::text("a"), finds_one_widget());
            expect(&tester, &find::text("b"), finds_one_widget());
            expect(&tester, &find::text("a.a"), finds_nothing());
            expect(&tester, &find::text("b.a"), finds_nothing());

            tester.tap(find::text("a"));
            tester.pump();

            expect(&tester, &find::text("a.a"), finds_one_widget());
            expect(&tester, &find::text("b.a"), finds_nothing());

            tester.tap(find::text("b"));
            tester.pump();

            expect(&tester, &find::text("a.a"), finds_nothing());
            expect(&tester, &find::text("b.a"), finds_one_widget());
        });
    }

    #[test]
    fn maybe_is_open_of_notifies_dependents_when_is_open_changes() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let group_controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let nested_controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let panel_is_open: Rc<Cell<Option<bool>>> = Rc::default();
            let overlay_is_open: Rc<Cell<Option<bool>>> = Rc::default();
            let anchor_is_open: Rc<Cell<Option<bool>>> = Rc::default();
            let panel_builds = counter();
            let anchor_builds = counter();
            let overlay_builds = counter();

            let observer =
                |is_open: &Rc<Cell<Option<bool>>>, builds: &Rc<Cell<u32>>, text: &'static str| {
                    let (is_open, builds) = (Rc::clone(is_open), Rc::clone(builds));
                    Builder::new(move |app, context| {
                        is_open.set(MenuController::maybe_is_open_of(app, context));
                        builds.set(builds.get() + 1);
                        Text::new(text).into_widget()
                    })
                    .into_widget()
                };

            tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
                Column::new().children([
                    // Panel context.
                    observer(&panel_is_open, &panel_builds, "a"),
                    Menu::new()
                        .controller(controller.clone())
                        .menu_panel(panel(vec![
                            // Overlay context.
                            observer(&overlay_is_open, &overlay_builds, "b.a.a"),
                            Menu::new()
                                .controller(nested_controller.clone())
                                .menu_panel(panel(vec![Button::tag("b.a.b.a").into_widget()]))
                                .child(Button::tag("b.a.b"))
                                .into_widget(),
                        ]))
                        // Anchor context.
                        .child(observer(&anchor_is_open, &anchor_builds, "b.a"))
                        .into_widget(),
                ]),
                group_controller,
            )));

            assert_eq!(panel_is_open.get(), Some(false));
            assert_eq!(anchor_is_open.get(), Some(false));
            assert_eq!(panel_builds.get(), 1);
            assert_eq!(anchor_builds.get(), 1);
            assert_eq!(overlay_builds.get(), 0);

            controller.open(&mut tester.app(), None);
            tester.pump();

            assert_eq!(panel_is_open.get(), Some(true));
            assert_eq!(anchor_is_open.get(), Some(true));
            assert_eq!(overlay_is_open.get(), Some(true));
            assert_eq!(panel_builds.get(), 2);
            assert_eq!(anchor_builds.get(), 2);
            assert_eq!(overlay_builds.get(), 1);

            nested_controller.open(&mut tester.app(), None);
            tester.pump();

            // No new builds should have occurred since all controllers are already open.
            assert_eq!(panel_is_open.get(), Some(true));
            assert_eq!(anchor_is_open.get(), Some(true));
            assert_eq!(overlay_is_open.get(), Some(true));
            assert_eq!(panel_builds.get(), 2);
            assert_eq!(anchor_builds.get(), 2);
            assert_eq!(overlay_builds.get(), 1);

            controller.close(&mut tester.app());
            tester.pump();

            assert_eq!(panel_is_open.get(), Some(false));
            assert_eq!(anchor_is_open.get(), Some(false));

            // Will be true because builder cannot rebuild when the menu is closed.
            assert_eq!(overlay_is_open.get(), Some(true));
            assert_eq!(panel_builds.get(), 3);
            assert_eq!(anchor_builds.get(), 3);
            assert_eq!(overlay_builds.get(), 1);
        });
    }

    #[test]
    fn menu_controllers_can_be_changed() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let group_controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let new_controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let new_group_controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();

            let tree = |group: MenuControllerRef, menu: MenuControllerRef| {
                TestApp::new(RawMenuAnchorGroup::new(
                    Menu::new()
                        .controller(menu)
                        .menu_panel(panel(texts(&["a"])))
                        .child(AnchorButton::new("anchor")),
                    group,
                ))
            };
            tester.pump_widget(tree(controller.clone(), group_controller.clone()));

            tester.tap(find::text("anchor"));
            tester.pump();

            expect(&tester, &find::text("a"), finds_one_widget());
            assert!(controller.is_open(&tester.app()));
            assert!(group_controller.is_open(&tester.app()));
            assert!(!new_controller.is_open(&tester.app()));
            assert!(!new_group_controller.is_open(&tester.app()));

            // Swap the controllers.
            tester.pump_widget(tree(new_controller.clone(), new_group_controller.clone()));

            expect(&tester, &find::text("a"), finds_one_widget());
            assert!(!controller.is_open(&tester.app()));
            assert!(!group_controller.is_open(&tester.app()));
            assert!(new_controller.is_open(&tester.app()));
            assert!(new_group_controller.is_open(&tester.app()));

            // Close the new controller.
            new_controller.close(&mut tester.app());
            tester.pump();

            assert!(!new_controller.is_open(&tester.app()));
            assert!(!new_group_controller.is_open(&tester.app()));
            expect(&tester, &find::text("a"), finds_nothing());
        });
    }

    #[test]
    fn a_menu_controller_can_be_moved_to_a_different_menu() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let cell = Rc::clone(&tester.cell);
            let tree = |key: Option<KeyRef>| {
                let mut menu = Menu::new()
                    .controller(controller.clone())
                    .menu_panel(panel(texts(&["a"])))
                    .child(AnchorButton::new("anchor"));
                if let Some(key) = key {
                    menu = menu.key(key);
                }
                let group_controller =
                    MenuController::new(&mut cell.borrow_mut()).into_erased::<AnyMenuController>();
                TestApp::new(RawMenuAnchorGroup::new(menu, group_controller))
            };
            tester.pump_widget(tree(None));

            tester.tap(find::text("anchor"));
            tester.pump();

            expect(&tester, &find::text("a"), finds_one_widget());
            assert!(controller.is_open(&tester.app()));

            // Swap the menus.
            tester.pump_widget(tree(Some(UniqueKey::new().into())));

            expect(&tester, &find::text("a"), finds_nothing());
            assert!(!controller.is_open(&tester.app()));

            tester.tap(find::text("anchor"));
            tester.pump();

            expect(&tester, &find::text("a"), finds_one_widget());
            assert!(controller.is_open(&tester.app()));

            // Close the menu.
            controller.close(&mut tester.app());
            tester.pump();

            assert!(!controller.is_open(&tester.app()));
            expect(&tester, &find::text("a"), finds_nothing());
        });
    }

    #[test]
    fn a_groups_menu_controller_can_be_moved_to_a_different_group() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let tree = |key: Option<KeyRef>| {
                let mut group = RawMenuAnchorGroup::new(
                    Menu::new()
                        .menu_panel(panel(texts(&["a"])))
                        .child(AnchorButton::new("anchor")),
                    controller.clone(),
                );
                if let Some(key) = key {
                    group = group.key(key);
                }
                TestApp::new(group)
            };
            tester.pump_widget(tree(None));

            tester.tap(find::text("anchor"));
            tester.pump();

            expect(&tester, &find::text("a"), finds_one_widget());
            assert!(controller.is_open(&tester.app()));

            // Swap the controllers.
            tester.pump_widget(tree(Some(UniqueKey::new().into())));

            expect(&tester, &find::text("a"), finds_nothing());
            assert!(!controller.is_open(&tester.app()));

            tester.tap(find::text("anchor"));
            tester.pump();

            expect(&tester, &find::text("a"), finds_one_widget());
            assert!(controller.is_open(&tester.app()));

            // Close the menu.
            controller.close(&mut tester.app());
            tester.pump();

            assert!(!controller.is_open(&tester.app()));
            expect(&tester, &find::text("a"), finds_nothing());
        });
    }

    #[test]
    fn maybe_of_does_not_notify_dependents_when_the_controller_changes() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let anchor_key = GlobalKey::new();
            let demo_controller_one =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let demo_controller_two =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();

            let panel_controller: Rc<Cell<Option<AnyMenuController>>> = Rc::default();
            let overlay_controller: Rc<Cell<Option<AnyMenuController>>> = Rc::default();
            let anchor_controller: Rc<Cell<Option<AnyMenuController>>> = Rc::default();
            let panel_builds = counter();
            let anchor_builds = counter();
            let overlay_builds = counter();

            let observer = |found: &Rc<Cell<Option<AnyMenuController>>>,
                            builds: &Rc<Cell<u32>>,
                            text: &'static str| {
                let (found, builds) = (Rc::clone(found), Rc::clone(builds));
                Builder::new(move |app, context| {
                    found.set(MenuController::maybe_of(app, context));
                    builds.set(builds.get() + 1);
                    Text::new(text).into_widget()
                })
            };
            let build_anchor = |group: MenuControllerRef, overlay: MenuControllerRef| {
                TestApp::new(RawMenuAnchorGroup::new(
                    Column::new().children([
                        // Panel context.
                        observer(&panel_controller, &panel_builds, "a").into_widget(),
                        Menu::new()
                            .controller(overlay)
                            .menu_panel(panel(vec![
                                // Overlay context.
                                observer(&overlay_controller, &overlay_builds, "b.a.a")
                                    .into_widget(),
                            ]))
                            // Anchor context.
                            .child(
                                observer(&anchor_controller, &anchor_builds, "b.a")
                                    .key(anchor_key.clone().into()),
                            )
                            .into_widget(),
                    ]),
                    group,
                ))
            };

            tester.pump_widget(build_anchor(
                demo_controller_one.clone(),
                demo_controller_two.clone(),
            ));

            assert_ne!(panel_controller.get(), Some(controller.handle()));
            assert_ne!(anchor_controller.get(), Some(controller.handle()));

            tester.pump_widget(build_anchor(
                controller.clone(),
                demo_controller_two.clone(),
            ));

            assert_eq!(panel_controller.get(), Some(controller.handle()));
            assert_ne!(anchor_controller.get(), Some(controller.handle()));
            assert_eq!(panel_builds.get(), 2);
            assert_eq!(anchor_builds.get(), 2);

            {
                let mut app = tester.app();
                let context = anchor_key
                    .current_context(&app)
                    .expect("the anchor is mounted");
                MenuController::maybe_of(&app, context)
                    .expect("the anchor's controller")
                    .open(&mut app, None);
            }
            tester.pump();

            assert_eq!(panel_builds.get(), 2);
            assert_eq!(anchor_builds.get(), 2);
            assert_eq!(overlay_builds.get(), 1);

            tester.pump_widget(build_anchor(
                demo_controller_one.clone(),
                controller.clone(),
            ));

            assert_ne!(panel_controller.get(), Some(controller.handle()));
            assert_eq!(anchor_controller.get(), Some(controller.handle()));
            assert_eq!(overlay_controller.get(), Some(controller.handle()));
            assert_eq!(panel_builds.get(), 3);
            assert_eq!(anchor_builds.get(), 3);
            assert_eq!(overlay_builds.get(), 2);
        });
    }

    #[test]
    fn a_detached_menu_controller_closes_without_panicking() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            controller.close(&mut tester.app());
            tester.pump();
        });
    }

    #[test]
    fn a_detached_menu_controller_is_not_open() {
        test_widgets(|tester| {
            let tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            assert!(!controller.is_open(&tester.app()));
        });
    }

    /// The message a call panics with, or `None` if it returns: Dart's `runZonedGuarded` that
    /// keeps the exception's text.
    fn panic_message(call: impl FnOnce()) -> Option<String> {
        let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)).err()?;
        Some(
            payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    payload
                        .downcast_ref::<&str>()
                        .map(|message| message.to_string())
                })
                .unwrap_or_default(),
        )
    }

    #[test]
    fn a_menu_controller_is_detached_when_its_anchor_is_given_another() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .menu_panel(SizedBox::shrink().into_widget())
                    .child(SizedBox::shrink()),
            ));

            // Should not panic because the controller is attached to the menu.
            controller.close_children(&mut tester.app());

            tester.pump_widget(TestApp::new(
                Menu::new()
                    .menu_panel(SizedBox::shrink().into_widget())
                    .child(SizedBox::shrink()),
            ));

            let message = panic_message(|| controller.close_children(&mut tester.app()));
            assert!(message.is_some_and(|message| message.contains("_anchor != null")));
        });
    }

    #[test]
    fn a_menu_controller_is_detached_when_its_group_is_given_another() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
                SizedBox::shrink(),
                controller.clone(),
            )));

            // Should not panic because the controller is attached to the menu.
            controller.close_children(&mut tester.app());

            let other = MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
                SizedBox::shrink(),
                other,
            )));

            let message = panic_message(|| controller.close_children(&mut tester.app()));
            assert!(message.is_some_and(|message| message.contains("_anchor != null")));
        });
    }

    #[test]
    fn a_menu_controller_is_detached_when_its_anchor_is_disposed() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .menu_panel(SizedBox::new().into_widget())
                    .child(SizedBox::new()),
            ));

            // Should not panic because the controller is attached to the menu.
            controller.close_children(&mut tester.app());

            tester.pump_widget(TestApp::new(SizedBox::new()));

            let message = panic_message(|| controller.close_children(&mut tester.app()));
            assert!(message.is_some_and(|message| message.contains("_anchor != null")));
        });
    }

    #[test]
    fn a_menu_controller_is_detached_when_its_group_is_disposed() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
                SizedBox::new(),
                controller.clone(),
            )));

            // Should not panic because the controller is attached to the menu.
            controller.close_children(&mut tester.app());

            tester.pump_widget(TestApp::new(SizedBox::new()));

            let message = panic_message(|| controller.close_children(&mut tester.app()));
            assert!(message.is_some_and(|message| message.contains("_anchor != null")));
        });
    }

    #[test]
    fn an_anchor_given_a_new_controller_after_its_owner_dropped_the_old_one_does_not_panic() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let old = MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let new = MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let anchor = |controller| {
                TestApp::new(
                    Menu::new()
                        .controller(controller)
                        .menu_panel(panel(texts(&["a"])))
                        .child(AnchorButton::new("anchor")),
                )
            };
            tester.pump_widget(anchor(old.clone()));

            // Material's `MenuAnchor` drops its internal controller as it is given one; the old
            // widget still keeps it while the anchor detaches from it.
            drop(old);
            tester.pump_widget(anchor(new.clone()));

            tester.tap(find::text("anchor"));
            tester.pump();
            assert!(new.is_open(&tester.app()));
        });
    }

    #[test]
    fn a_group_given_a_new_controller_after_its_owner_dropped_the_old_one_does_not_panic() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let old = MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let new = MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
                SizedBox::shrink(),
                old.clone(),
            )));

            drop(old);
            tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
                SizedBox::shrink(),
                new.clone(),
            )));

            new.close_children(&mut tester.app());
        });
    }

    #[test]
    fn the_anchor_rect_carries_the_anchors_transform() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let builder_position: Rc<Cell<Option<RawMenuOverlayInfo>>> = Rc::default();
            let sink = Rc::clone(&builder_position);
            tester.pump_widget(TestApp::new(
                Transform::new(Matrix4::translation(-50.0, 50.0).then(&Matrix4::scale(1.2, 1.2)))
                    .child(
                        RawMenuAnchor::new(controller.clone(), move |_app, _context, position| {
                            sink.set(Some(position));
                            Positioned::from_rect(
                                position.anchor_rect,
                                Container::new().color(Color::new(0xFF0000FF)),
                            )
                            .into_widget()
                        })
                        .child(AnchorButton::new("b")),
                    ),
            ));

            controller.open(&mut tester.app(), None);
            tester.pump();

            assert_eq!(
                Some(tester.get_rect(find::by_type::<AnchorButton>())),
                builder_position.get().map(|info| info.anchor_rect)
            );
        });
    }

    #[test]
    fn escape_closes_menus() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let a_focus_node: FocusNodeRef = FocusNode::new(&mut tester.app()).into_erased();
            let baa_focus_node: FocusNodeRef = FocusNode::new(&mut tester.app()).into_erased();
            let menu_controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(RawMenuAnchorGroup::new(
                Row::new().children([
                    Button::tag("a")
                        .focus_node(a_focus_node.clone())
                        .into_widget(),
                    Menu::new()
                        .controller(menu_controller.clone())
                        .menu_panel(panel(vec![
                            Menu::new()
                                .menu_panel(panel(vec![
                                    Button::tag("b.a.a")
                                        .focus_node(baa_focus_node.clone())
                                        .into_widget(),
                                ]))
                                .child(AnchorButton::new("b.a"))
                                .into_widget(),
                        ]))
                        .child(AnchorButton::new("b"))
                        .into_widget(),
                ]),
                controller,
            )));

            menu_controller.open(&mut tester.app(), None);
            tester.pump();

            a_focus_node.request_focus(&mut tester.app(), None);
            tester.pump();

            assert_eq!(
                primary_focus(&mut tester.app()),
                Some(a_focus_node.handle())
            );
            expect(&tester, &find::text("b.a"), finds_one_widget());

            // Test panel child can close siblings with escape key.
            tester.send_key_event(PhysicalKeyboardKey::ESCAPE, LogicalKeyboardKey::ESCAPE);
            tester.pump();

            expect(&tester, &find::text("b.a"), finds_nothing());

            tester.tap(find::text("b"));
            tester.pump();
            tester.tap(find::text("b.a"));
            tester.pump();
            baa_focus_node.request_focus(&mut tester.app(), None);
            tester.pump();

            assert_eq!(
                primary_focus(&mut tester.app()),
                Some(baa_focus_node.handle())
            );

            // Test ancestors menus are closed with escape key.
            tester.send_key_event(PhysicalKeyboardKey::ESCAPE, LogicalKeyboardKey::ESCAPE);
            tester.pump();

            expect(&tester, &find::text("b.a"), finds_nothing());
        });
    }

    #[test]
    fn a_closed_anchor_lets_intents_through() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let a_focus_node: FocusNodeRef = FocusNode::new(&mut tester.app()).into_erased();
            let invoked = log();
            let actions = {
                let mut app = tester.app();
                let mut actions: HashMap<TypeId, ActionRef> = HashMap::new();
                let sink = Rc::clone(&invoked);
                actions.insert(
                    TypeId::of::<DirectionalFocusIntent>(),
                    Retained::into_erased(CallbackAction::<DirectionalFocusIntent>::new(
                        &mut app,
                        Rc::new(move |_app, _intent| {
                            sink.borrow_mut().push("DirectionalFocusIntent");
                            None
                        }),
                    )),
                );
                let sink = Rc::clone(&invoked);
                actions.insert(
                    TypeId::of::<NextFocusIntent>(),
                    Retained::into_erased(CallbackAction::<NextFocusIntent>::new(
                        &mut app,
                        Rc::new(move |_app, _intent| {
                            sink.borrow_mut().push("NextFocusIntent");
                            None
                        }),
                    )),
                );
                let sink = Rc::clone(&invoked);
                actions.insert(
                    TypeId::of::<PreviousFocusIntent>(),
                    Retained::into_erased(CallbackAction::<PreviousFocusIntent>::new(
                        &mut app,
                        Rc::new(move |_app, _intent| {
                            sink.borrow_mut().push("PreviousFocusIntent");
                            None
                        }),
                    )),
                );
                let sink = Rc::clone(&invoked);
                actions.insert(
                    TypeId::of::<DismissIntent>(),
                    Retained::into_erased(CallbackAction::<DismissIntent>::new(
                        &mut app,
                        Rc::new(move |_app, _intent| {
                            sink.borrow_mut().push("DismissIntent");
                            None
                        }),
                    )),
                );
                actions
            };
            tester.pump_widget(TestApp::new(Actions::new(
                actions,
                RawMenuAnchorGroup::new(
                    Row::new().children([Menu::new()
                        .menu_panel(panel(texts(&["a"])))
                        .child(AnchorButton::new("anchor").focus_node(a_focus_node.clone()))
                        .into_widget()]),
                    controller,
                ),
            )));

            a_focus_node.request_focus(&mut tester.app(), None);
            tester.pump();
            {
                let mut app = tester.app();
                let context = a_focus_node
                    .context(&app)
                    .expect("the focus node is attached");
                Actions::invoke(
                    &mut app,
                    context,
                    &DirectionalFocusIntent::new(TraversalDirection::Up),
                );
                Actions::invoke(&mut app, context, &NextFocusIntent::new());
                Actions::invoke(&mut app, context, &PreviousFocusIntent::new());
                Actions::invoke(&mut app, context, &DismissIntent::new());
            }
            tester.pump();

            assert_eq!(
                *invoked.borrow(),
                [
                    "DirectionalFocusIntent",
                    "NextFocusIntent",
                    "PreviousFocusIntent",
                    "DismissIntent"
                ]
            );
        });
    }

    #[test]
    fn dismiss_menu_action_closes_menus() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let focus_node: FocusNodeRef = FocusNode::new(&mut tester.app()).into_erased();
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .menu_panel(panel(vec![
                        Text::new("a").into_widget(),
                        Menu::new()
                            .menu_panel(panel(vec![
                                Text::new("b.a").into_widget(),
                                Menu::new()
                                    .controller(controller.clone())
                                    .menu_panel(panel(texts(&["b.b.a"])))
                                    .child(AnchorButton::new("b.b").focus_node(focus_node.clone()))
                                    .into_widget(),
                            ]))
                            .child(AnchorButton::new("b"))
                            .into_widget(),
                    ]))
                    .child(AnchorButton::new("anchor")),
            ));

            tester.tap(find::text("anchor"));
            tester.pump();
            tester.tap(find::text("b"));
            tester.pump();
            tester.tap(find::text("b.b"));
            tester.pump();

            assert!(controller.is_open(&tester.app()));

            focus_node.request_focus(&mut tester.app(), None);
            tester.pump();

            {
                let mut app = tester.app();
                let action = DismissMenuAction::new(&mut app, controller);
                let context = focus_node.context(&app);
                PlainActionDispatcher::new().invoke_action(
                    &mut app,
                    Action::as_action(action.handle()),
                    &DismissIntent::new(),
                    context,
                );
            }

            tester.pump();

            expect(&tester, &find::text("a"), finds_nothing());
        });
    }

    #[test]
    fn a_group_lays_its_child_out_as_its_own() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let key = GlobalKey::new();
            tester.pump_widget(TestApp {
                child: RawMenuAnchorGroup::new(
                    Padding::new(EdgeInsetsGeometry::all(8.0))
                        .key(key.clone().into())
                        .child(
                            Row::new().main_axis_size(MainAxisSize::Min).children([
                                Container::new()
                                    .width(100.0)
                                    .height(100.0)
                                    .color(Color::new(0xff0000ff))
                                    .into_widget(),
                                Container::new()
                                    .width(100.0)
                                    .height(100.0)
                                    .color(Color::new(0xFFFF00D4))
                                    .into_widget(),
                            ]),
                        ),
                    controller,
                )
                .into_widget(),
                alignment: AlignmentGeometry::TOP_START,
            });

            assert_eq!(
                tester.get_rect(&key),
                Rect::from_ltwh(0.0, 0.0, 216.0, 116.0)
            );
        });
    }

    #[test]
    fn the_overlay_builder_is_passed_the_anchor_rect() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let overlay_position: Rc<Cell<Option<RawMenuOverlayInfo>>> = Rc::default();
            let sink = Rc::clone(&overlay_position);
            tester.pump_widget(TestApp::new(
                RawMenuAnchor::new(controller, move |_app, _context, position| {
                    sink.set(Some(position));
                    SizedBox::new().into_widget()
                })
                .child(AnchorButton::new("anchor")),
            ));

            tester.tap(find::text("anchor"));
            tester.pump();

            assert_eq!(
                overlay_position.get().map(|info| info.anchor_rect),
                Some(tester.get_rect(find::by_type::<Button>()))
            );
        });
    }

    #[test]
    fn overlay_contents_can_be_positioned() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let key = GlobalKey::new();
            let content_key = key.clone();
            tester.pump_widget(TestApp::new(
                RawMenuAnchor::new(controller, move |_app, _context, position| {
                    Positioned::new(
                        Container::new()
                            .key(content_key.clone().into())
                            .width(200.0)
                            .height(200.0)
                            .color(Color::new(0xFF00FF00)),
                    )
                    .top(position.anchor_rect.top)
                    .left(position.anchor_rect.left)
                    .into_widget()
                })
                .child(AnchorButton::new("anchor")),
            ));

            tester.tap(find::text("anchor"));
            tester.pump();

            let anchor_corner = tester.get_top_left(find::by_type::<Button>());
            let content_rect = tester.get_rect(&key);

            assert_eq!(content_rect, anchor_corner & Size::new(200.0, 200.0));
        });
    }

    #[test]
    fn the_tap_region_group_id_is_passed_to_the_overlay() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let inside_tap: Rc<Cell<Option<bool>>> = Rc::default();
            let builder_tap = Rc::clone(&inside_tap);
            tester.pump_widget(TestApp::new(
                RawMenuAnchor::new(controller, move |_app, _context, position| {
                    let (on_inside, on_outside) =
                        (Rc::clone(&builder_tap), Rc::clone(&builder_tap));
                    let mut region = TapRegion::new(Button::tag("a"))
                        .on_tap_inside(Rc::new(move |_app, _event| on_inside.set(Some(true))))
                        .on_tap_outside(Rc::new(move |_app, _event| on_outside.set(Some(false))));
                    if !builder_tap.get().unwrap_or(false) {
                        region = region.group_id(position.tap_region_group_id);
                    }
                    Positioned::from_rect(position.anchor_rect.translate(200.0, 200.0), region)
                        .into_widget()
                })
                .child(AnchorButton::new("anchor")),
            ));

            tester.tap(find::text("anchor"));
            tester.pump();

            expect(&tester, &find::text("a"), finds_one_widget());

            // Start by testing that the tap region has the correct group ID.
            tester.tap(find::text("anchor"));
            tester.pump();

            assert_eq!(inside_tap.get(), Some(true));

            // The menu should close when the tap region is tapped, so we need to reopen.
            tester.tap(find::text("anchor"));
            tester.pump();

            expect(&tester, &find::text("a"), finds_one_widget());
            assert_eq!(inside_tap.get(), Some(true));

            // Now test that setting the tap region group ID to None will cause the tap to be
            // considered outside the tap region.
            tester.tap(find::text("anchor"));
            tester.pump();

            assert_eq!(inside_tap.get(), Some(false));
        });
    }

    /// Dart's two `Menus close and ... consume tap when consumesOutsideTap is ...` cases.
    fn outside_taps_close_menus(consume_outside_taps: bool) {
        let tester = &mut WidgetTester::new();
        let mut tester = Tester::new(tester);
        let controller = MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
        let selected = log();
        let opened = log();
        let closed = log();
        let on_pressed: TagCallback = {
            let selected = Rc::clone(&selected);
            Rc::new(move |_app, tag| selected.borrow_mut().push(tag))
        };
        let on_open = |tag: &'static str| push(&opened, tag);
        let on_close = |tag: &'static str| {
            let (opened, closed) = (Rc::clone(&opened), Rc::clone(&closed));
            Rc::new(move |_app: &mut App| {
                opened.borrow_mut().retain(|opened| *opened != tag);
                closed.borrow_mut().push(tag);
            }) as VoidCallback
        };
        tester.pump_widget(TestApp::new(
            Column::new().children([
                Button::tag("outside")
                    .on_pressed(push(&selected, "outside"))
                    .into_widget(),
                RawMenuAnchorGroup::new(
                    Column::new().children([Menu::new()
                        .consume_outside_taps(consume_outside_taps)
                        .on_open(on_open("anchor"))
                        .on_close(on_close("anchor"))
                        .menu_panel(panel(vec![
                            Menu::new()
                                .consume_outside_taps(consume_outside_taps)
                                .on_open(on_open("a"))
                                .on_close(on_close("a"))
                                .menu_panel(panel(texts(&["a.a"])))
                                .child(AnchorButton::new("a").on_pressed(Rc::clone(&on_pressed)))
                                .into_widget(),
                        ]))
                        .child(AnchorButton::new("anchor").on_pressed(Rc::clone(&on_pressed)))
                        .into_widget()]),
                    controller,
                )
                .into_widget(),
            ]),
        ));

        assert!(opened.borrow().is_empty());
        assert!(closed.borrow().is_empty());

        // Doesn't consume tap when the menu is closed.
        tester.tap(find::text("outside"));
        tester.pump();

        assert_eq!(*selected.borrow(), ["outside"]);
        selected.borrow_mut().clear();

        tester.tap(find::text("anchor"));
        tester.pump();
        tester.tap(find::text("a"));
        tester.pump();

        assert_eq!(*opened.borrow(), ["anchor", "a"]);
        assert!(closed.borrow().is_empty());
        assert_eq!(*selected.borrow(), ["anchor", "a"]);
        opened.borrow_mut().clear();
        closed.borrow_mut().clear();
        selected.borrow_mut().clear();

        tester.tap(find::text("outside"));
        tester.pump();

        assert!(opened.borrow().is_empty());
        assert_eq!(*closed.borrow(), ["a", "anchor"]);
        if consume_outside_taps {
            // When the menu is open, outside taps are consumed. As a result, tapping outside
            // the menu will close it and not select the outside button.
            assert!(selected.borrow().is_empty());
        } else {
            // Because consume_outside_taps is false, the outside button is expected to receive
            // a tap.
            assert_eq!(*selected.borrow(), ["outside"]);
        }
    }

    #[test]
    fn menus_close_and_consume_the_tap_when_consume_outside_taps_is_true() {
        outside_taps_close_menus(true);
    }

    #[test]
    fn menus_close_and_let_the_tap_through_when_consume_outside_taps_is_false() {
        outside_taps_close_menus(false);
    }

    #[test]
    fn on_open_is_called_when_the_menu_opens() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let opened = Rc::new(Cell::new(false));
            let sink = Rc::clone(&opened);
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_open(Rc::new(move |_app| sink.set(true)))
                    .menu_panel(panel(Vec::new()))
                    .child(AnchorButton::new("anchor")),
            ));
            tester.tap(find::text("anchor"));
            tester.pump();

            assert!(opened.get());

            opened.set(false);
            tester.tap(find::text("anchor"));
            tester.pump();

            // on_open should not be called again.
            assert!(!opened.get());

            controller.open(&mut tester.app(), None);
            tester.pump();

            assert!(opened.get());
        });
    }

    #[test]
    fn on_close_is_called_when_the_menu_closes() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let closed = Rc::new(Cell::new(true));
            let (on_open, on_close) = (Rc::clone(&closed), Rc::clone(&closed));
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_open(Rc::new(move |_app| on_open.set(false)))
                    .on_close(Rc::new(move |_app| on_close.set(true)))
                    .menu_panel(panel(Vec::new()))
                    .child(AnchorButton::new("anchor")),
            ));

            tester.tap(find::text("anchor"));
            tester.pump();

            assert!(!closed.get());

            tester.tap(find::text("anchor"));
            tester.pump();

            assert!(closed.get());

            controller.open(&mut tester.app(), None);
            tester.pump();

            assert!(!closed.get());

            controller.close(&mut tester.app());
            tester.pump();

            assert!(closed.get());
        });
    }

    #[test]
    fn a_menu_closes_when_the_view_changes_size() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let opened = Rc::new(Cell::new(false));
            let closed = Rc::new(Cell::new(false));
            let on_open = {
                let (opened, closed) = (Rc::clone(&opened), Rc::clone(&closed));
                Rc::new(move |_app: &mut App| {
                    opened.set(true);
                    closed.set(false);
                }) as VoidCallback
            };
            let on_close = {
                let (opened, closed) = (Rc::clone(&opened), Rc::clone(&closed));
                Rc::new(move |_app: &mut App| {
                    opened.set(false);
                    closed.set(true);
                }) as VoidCallback
            };
            let media_query_data = tester.media_query_data();
            let tree = |size: Size| -> WidgetRef {
                MediaQuery::new(
                    media_query_data.copy_with().size(size),
                    TestApp::new(
                        Menu::new()
                            .on_open(Rc::clone(&on_open))
                            .on_close(Rc::clone(&on_close))
                            .controller(controller.clone())
                            .menu_panel(panel(texts(&["a"])))
                            .child(AnchorButton::new("anchor")),
                    ),
                )
                .into_widget()
            };

            tester.pump_widget(tree(media_query_data.size));
            tester.tap(find::text("anchor"));
            tester.pump();

            assert!(opened.get());
            assert!(!closed.get());

            let small_size = Size::new(200.0, 200.0);
            tester.change_surface_size(small_size);
            tester.pump_widget(tree(small_size));

            assert!(!opened.get());
            assert!(closed.get());
        });
    }

    /// The scroll controller Dart's `ScrollController()` makes.
    fn scroll_controller(app: &mut App) -> ScrollControllerRef {
        ScrollController::new(app, 0.0, true, None, None, None).into_erased()
    }

    #[test]
    fn a_menu_closes_when_an_ancestor_scrolls() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let scroll_controller = scroll_controller(&mut tester.app());
            let opened = log();
            let closed = log();
            let on_close = {
                let (opened, closed) = (Rc::clone(&opened), Rc::clone(&closed));
                Rc::new(move |_app: &mut App| {
                    opened.borrow_mut().retain(|opened| *opened != "anchor");
                    closed.borrow_mut().push("anchor");
                }) as VoidCallback
            };
            tester.pump_widget(TestApp::new(
                SingleChildScrollView::new()
                    .controller(scroll_controller.clone())
                    .child(
                        Menu::new()
                            .on_open(push(&opened, "anchor"))
                            .on_close(on_close)
                            .menu_panel(panel(vec![
                                Button::tag("a").into_widget(),
                                Button::tag("b").into_widget(),
                                Button::tag("c").into_widget(),
                                Button::tag("d").into_widget(),
                            ]))
                            .child(AnchorButton::new("anchor")),
                    ),
            ));

            tester.tap(find::text("anchor"));
            tester.pump();

            assert!(!opened.borrow().is_empty());
            assert!(closed.borrow().is_empty());
            opened.borrow_mut().clear();

            scroll_controller.jump_to(&mut tester.app(), 1000.0);
            tester.pump();

            assert!(opened.borrow().is_empty());
            assert!(!closed.borrow().is_empty());
        });
    }

    #[test]
    fn the_ancestor_scroll_listener_is_removed_on_dispose() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let scroll_controller = scroll_controller(&mut tester.app());
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(Directionality::new(
                TextDirection::Ltr,
                SingleChildScrollView::new()
                    .controller(scroll_controller.clone())
                    .child(RawMenuAnchor::new(controller, |_app, _context, _info| {
                        SizedBox::new().into_widget()
                    })),
            ));

            let notifier = {
                let app = tester.app();
                scroll_controller.position(&app).is_scrolling_notifier(&app)
            };

            assert!(has_listeners(&tester.app(), notifier));

            tester.pump_widget(Directionality::new(
                TextDirection::Ltr,
                SingleChildScrollView::new()
                    .controller(scroll_controller)
                    .child(SizedBox::new()),
            ));

            assert!(!has_listeners(&tester.app(), notifier));
        });
    }

    #[test]
    fn the_menu_builder_rebuilds_when_is_open_changes() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let is_open = Rc::new(Cell::new(false));
            let open_count = counter();
            let close_count = counter();
            let builder_is_open = Rc::clone(&is_open);
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .menu_panel(panel(vec![
                        Button::new(Text::new("Menu Item")).into_widget(),
                    ]))
                    .builder(move |app, _context, controller, _child| {
                        builder_is_open.set(controller.is_open(app));
                        let label = if builder_is_open.get() {
                            "close"
                        } else {
                            "open"
                        };
                        Button::new(Text::new(label))
                            .on_pressed(Rc::new(move |app: &mut App| {
                                if controller.is_open(app) {
                                    controller.close(app);
                                } else {
                                    controller.open(app, None);
                                }
                            }))
                            .into_widget()
                    })
                    .on_open(bump(&open_count))
                    .on_close(bump(&close_count)),
            ));

            expect(&tester, &find::text("open"), finds_one_widget());
            assert!(!is_open.get());
            assert_eq!(open_count.get(), 0);
            assert_eq!(close_count.get(), 0);

            tester.tap(find::text("open"));
            tester.pump();

            expect(&tester, &find::text("close"), finds_one_widget());
            assert!(is_open.get());
            assert_eq!(open_count.get(), 1);
            assert_eq!(close_count.get(), 0);

            tester.tap(find::text("close"));
            tester.pump();

            expect(&tester, &find::text("open"), finds_one_widget());
            assert!(!is_open.get());
            assert_eq!(open_count.get(), 1);
            assert_eq!(close_count.get(), 1);
        });
    }

    /// The nearest [`RenderTheater`] above `object`.
    fn ancestor_theater(app: &App, object: AnyRenderObject) -> Option<RenderHandle<RenderTheater>> {
        let mut node = Some(object);
        while let Some(current) = node {
            if let Some(theater) = current.downcast::<RenderTheater>(app) {
                return Some(theater);
            }
            node = current.parent(app);
        }
        None
    }

    /// Dart's `Content is shown in the root overlay when useRootOverlay is true` and its
    /// `nearest ancestor overlay` twin: a menu inside an inner [`Overlay`], with the root and
    /// the nested anchor's `use_root_overlay` as given. Answers whether the menu's and the
    /// submenu's theater is the inner overlay's, after asserting that they share one.
    fn menu_is_shown_in_the_inner_overlay(root_uses_root: bool, nested_uses_root: bool) -> bool {
        let tester = &mut WidgetTester::new();
        let mut tester = Tester::new(tester);
        let controller = MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
        let menu_controller = controller.clone();
        let overlay_key = GlobalKey::new();
        let overlay_entry = OverlayEntry::new(
            &mut tester.app(),
            Rc::new(move |_app, _context| {
                Center::new()
                    .child(
                        Menu::new()
                            .use_root_overlay(root_uses_root)
                            .controller(menu_controller.clone())
                            .menu_panel(panel(vec![
                                Menu::new()
                                    .use_root_overlay(nested_uses_root)
                                    .menu_panel(panel(vec![Button::tag("a.a").into_widget()]))
                                    .child(AnchorButton::new("a"))
                                    .into_widget(),
                            ])),
                    )
                    .into_widget()
            }),
            false,
            false,
            false,
        );
        tester.pump_widget(TestApp::new(
            Overlay::new()
                .key(overlay_key.clone().into())
                .initial_entries(vec![overlay_entry.clone()]),
        ));

        expect(&tester, &find::text("a"), finds_nothing());

        // Open the menu.
        controller.open(&mut tester.app(), None);
        tester.pump();
        tester.tap(find::text("a"));
        tester.pump();

        expect(&tester, &find::text("a"), finds_one_widget());
        expect(&tester, &find::text("a.a"), finds_one_widget());

        // Expect two overlays: the root overlay created by WidgetsApp and the overlay created
        // by the boilerplate code.
        expect(&tester, &find::by_type::<Overlay>(), finds_n_widgets(2));

        let (a, aa) = (
            tester.render_object(find::text("a")),
            tester.render_object(find::text("a.a")),
        );
        let app = tester.app();
        let inner_theater = overlay_key
            .current_context(&app)
            .and_then(|context| context.find_render_object(&app))
            .and_then(|object| object.downcast::<RenderTheater>(&app))
            .expect("the inner overlay's theater");
        let menu_theater = ancestor_theater(&app, a).expect("a theater above the menu");
        let submenu_theater = ancestor_theater(&app, aa).expect("a theater above the submenu");
        assert_eq!(menu_theater, submenu_theater);
        drop(app);
        overlay_entry.remove(&mut tester.app());
        overlay_entry.dispose(&mut tester.app());
        menu_theater == inner_theater
    }

    #[test]
    fn content_is_shown_in_the_root_overlay_when_use_root_overlay_is_true() {
        assert!(!menu_is_shown_in_the_inner_overlay(true, false));
    }

    #[test]
    fn content_is_shown_in_the_nearest_overlay_when_use_root_overlay_is_false() {
        // Nested menus should be rendered in the same overlay as their parent, so
        // use_root_overlay should have no effect.
        assert!(menu_is_shown_in_the_inner_overlay(false, true));
    }

    #[test]
    fn parent_updates_are_not_triggered_during_builds() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let media_query_data = tester.media_query_data();
            let tree = |size: Size| {
                let child: WidgetRef = MediaQuery::new(
                    media_query_data.copy_with().size(size),
                    RawMenuAnchorGroup::new(
                        Menu::new()
                            .menu_panel(panel(Vec::new()))
                            .child(AnchorButton::new("anchor")),
                        controller.clone(),
                    ),
                )
                .into_widget();
                TestApp::new(child)
            };

            tester.pump_widget(tree(media_query_data.size));

            tester.tap(find::text("anchor"));
            tester.pump();

            let small_size = Size::new(200.0, 200.0);
            tester.change_surface_size(small_size);

            tester.pump_widget(tree(small_size));
            tester.pump();
        });
    }

    #[test]
    fn the_anchor_rect_follows_the_anchor_when_it_moves() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let overlay_position: Rc<Cell<Option<RawMenuOverlayInfo>>> = Rc::default();
            let set_state: Rc<RefCell<Option<StateSetter>>> = Rc::default();
            let show_additional_widget = Rc::new(Cell::new(true));
            let (sink, setter, show) = (
                Rc::clone(&overlay_position),
                Rc::clone(&set_state),
                Rc::clone(&show_additional_widget),
            );
            tester.pump_widget(TestApp::new(StatefulBuilder::new(
                move |_app, _context, set_state| {
                    *setter.borrow_mut() = Some(set_state);
                    let mut children = Vec::new();
                    if show.get() {
                        children.push(SizedBox::new().width(100.0).height(100.0).into_widget());
                    }
                    let sink = Rc::clone(&sink);
                    children.push(
                        RawMenuAnchor::new(controller.clone(), move |_app, _context, position| {
                            sink.set(Some(position));
                            SizedBox::new().into_widget()
                        })
                        .child(AnchorButton::new("anchor"))
                        .into_widget(),
                    );
                    Column::new().children(children).into_widget()
                },
            )));

            tester.tap(find::text("anchor"));
            tester.pump();

            assert_eq!(
                overlay_position.get().map(|info| info.anchor_rect),
                Some(tester.get_rect(find::by_type::<Button>()))
            );

            {
                let set_state = set_state.borrow().clone().expect("built");
                set_state(&mut tester.app(), &mut |_app| {
                    show_additional_widget.set(false)
                });
            }
            tester.pump();
            tester.pump();

            assert_eq!(
                overlay_position.get().map(|info| info.anchor_rect),
                Some(tester.get_rect(find::by_type::<Button>()))
            );
        });
    }

    #[test]
    fn an_anchor_of_zero_area_does_not_panic() {
        test_widgets(|tester| {
            tester.view().set_physical_size(Size::ZERO);
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(Directionality::new(
                TextDirection::Ltr,
                Center::new().child(RawMenuAnchor::new(
                    controller.clone(),
                    |_app, _context, _info| Text::new("X").into_widget(),
                )),
            ));
            assert_eq!(
                tester.get_size(find::by_type::<RawMenuAnchor>()),
                Size::ZERO
            );
            controller.open(&mut tester.app(), None);
        });
    }

    #[test]
    fn a_group_of_zero_area_does_not_panic() {
        test_widgets(|tester| {
            tester.view().set_physical_size(Size::ZERO);
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(Directionality::new(
                TextDirection::Ltr,
                Center::new().child(RawMenuAnchorGroup::new(Text::new("X"), controller.clone())),
            ));
            assert_eq!(
                tester.get_size(find::by_type::<RawMenuAnchorGroup>()),
                Size::ZERO
            );
            controller.open(&mut tester.app(), None);
        });
    }

    /// Dart's `CustomMenuController extends MenuController`, which counts its opens and closes.
    struct CustomMenuController {
        menu_controller: MenuControllerData,
        open_call_count: u32,
        close_call_count: u32,
    }

    impl MenuControllerLeaf for CustomMenuController {
        crate::menu_controller_accessors!();

        fn open(self: Handle<Self>, app: &mut App, position: Option<Offset>) {
            app.get_mut(self).open_call_count += 1;
            MenuController::open(self.as_menu_controller(), app, position);
        }

        fn close(self: Handle<Self>, app: &mut App) {
            app.get_mut(self).close_call_count += 1;
            MenuController::close(self.as_menu_controller(), app);
        }
    }

    #[test]
    fn an_app_can_override_the_open_and_close_of_its_menu_controller() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let custom_controller = tester.app().create(CustomMenuController {
                menu_controller: MenuControllerData::new(),
                open_call_count: 0,
                close_call_count: 0,
            });
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(custom_controller.erase())
                    .menu_panel(panel(texts(&["a"])))
                    .child(AnchorButton::new("anchor")),
            ));

            custom_controller
                .as_menu_controller()
                .open(&mut tester.app(), None);
            tester.pump();

            assert!(custom_controller.is_open(&tester.app()));
            assert_eq!(tester.app().get(&custom_controller).open_call_count, 1);
            expect(&tester, &find::text("a"), finds_one_widget());

            custom_controller
                .as_menu_controller()
                .close(&mut tester.app());
            tester.pump();

            assert!(!custom_controller.is_open(&tester.app()));
            assert_eq!(tester.app().get(&custom_controller).close_call_count, 1);
            expect(&tester, &find::text("a"), finds_nothing());
        });
    }

    // ---- onOpenRequested ----

    #[test]
    fn on_open_requested_is_triggered_by_open() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let on_open_called = counter();
            let on_open_requested_called = counter();
            let requested = Rc::clone(&on_open_requested_called);
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_open(bump(&on_open_called))
                    .on_open_requested(move |_app, _position, _show_menu| {
                        requested.set(requested.get() + 1);
                    })
                    .menu_panel(panel(texts(&["Button 1"])))
                    .child(AnchorButton::new("anchor")),
            ));

            assert_eq!(on_open_called.get(), 0);
            assert_eq!(on_open_requested_called.get(), 0);

            controller.open(&mut tester.app(), None);
            tester.pump();

            assert_eq!(on_open_called.get(), 0);
            assert_eq!(on_open_requested_called.get(), 1);
        });
    }

    #[test]
    fn on_open_requested_is_passed_the_position() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let menu_position: Rc<Cell<Option<Offset>>> = Rc::default();
            let sink = Rc::clone(&menu_position);
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_open_requested(move |_app, position, _show_overlay| sink.set(position))
                    .menu_panel(panel(texts(&["Button 1"])))
                    .child(AnchorButton::new("anchor")),
            ));

            assert_eq!(menu_position.get(), None);

            controller.open(&mut tester.app(), None);
            tester.pump();

            assert_eq!(menu_position.get(), None);

            controller.close(&mut tester.app());
            tester.pump();

            assert_eq!(menu_position.get(), None);

            controller.open(&mut tester.app(), Some(Offset::new(10.0, 15.0)));
            tester.pump();

            assert_eq!(menu_position.get(), Some(Offset::new(10.0, 15.0)));
        });
    }

    #[test]
    fn show_overlay_triggers_on_open() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let on_open_called = counter();
            let on_open_requested_called = counter();
            let show_menu_overlay: Rc<RefCell<Option<VoidCallback>>> = Rc::default();
            let (requested, sink) = (
                Rc::clone(&on_open_requested_called),
                Rc::clone(&show_menu_overlay),
            );
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_open(bump(&on_open_called))
                    .on_open_requested(move |_app, _position, show_overlay| {
                        requested.set(requested.get() + 1);
                        *sink.borrow_mut() = Some(show_overlay);
                    })
                    .menu_panel(panel(texts(&["Button 1"])))
                    .child(AnchorButton::new("anchor")),
            ));

            assert_eq!(on_open_requested_called.get(), 0);
            assert_eq!(on_open_called.get(), 0);

            controller.open(&mut tester.app(), None);
            tester.pump();

            assert_eq!(on_open_requested_called.get(), 1);
            assert_eq!(on_open_called.get(), 0);

            controller.open(&mut tester.app(), None);
            tester.pump();

            // Calling open again will trigger on_open_requested again.
            assert_eq!(on_open_requested_called.get(), 2);
            assert_eq!(on_open_called.get(), 0);

            let show = show_menu_overlay.borrow().clone().expect("requested");
            show(&mut tester.app());
            tester.pump();

            assert_eq!(on_open_requested_called.get(), 2);
            assert_eq!(on_open_called.get(), 1);

            // Calling show_overlay again will trigger on_open again.
            show(&mut tester.app());
            tester.pump();

            assert_eq!(on_open_requested_called.get(), 2);
            assert_eq!(on_open_called.get(), 2);
        });
    }

    #[test]
    fn show_overlay_shows_the_menu_at_the_position() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let key = GlobalKey::new();
            let content_key = key.clone();
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_open_requested(|app, _position, show_overlay| {
                        Timer::new(
                            app,
                            Duration::from_millis(100),
                            Listener::new(move |app| show_overlay(app)),
                        );
                    })
                    .overlay_builder(move |_app, _context, info| {
                        let position = info.position.unwrap_or(Offset::ZERO);
                        Positioned::new(
                            SizedBox::square(Some(300.0)).key(content_key.clone().into()),
                        )
                        .top(position.dy())
                        .left(position.dx())
                        .into_widget()
                    })
                    .child(AnchorButton::new("anchor")),
            ));

            assert!(key.current_context(&tester.app()).is_none());

            controller.open(&mut tester.app(), None);
            tester.pump();

            assert!(key.current_context(&tester.app()).is_none());

            tester.pump_by(Duration::from_millis(101));

            assert_eq!(tester.get_top_left(&key), Offset::ZERO);

            controller.open(&mut tester.app(), Some(Offset::new(50.0, 50.0)));
            tester.pump();
            tester.pump_by(Duration::from_millis(101));

            assert_eq!(tester.get_top_left(&key), Offset::new(50.0, 50.0));
        });
    }

    #[test]
    fn show_overlay_does_nothing_after_the_menu_is_disposed() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let show_menu_overlay: Rc<RefCell<Option<VoidCallback>>> = Rc::default();
            let on_open_called = counter();
            let sink = Rc::clone(&show_menu_overlay);
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_open(bump(&on_open_called))
                    .on_open_requested(move |_app, _position, show_overlay| {
                        *sink.borrow_mut() = Some(show_overlay);
                    })
                    .overlay_builder(|_app, _context, _info| {
                        SizedBox::square(Some(300.0))
                            .child(Text::new("Overlay"))
                            .into_widget()
                    })
                    .child(AnchorButton::new("anchor")),
            ));

            controller.open(&mut tester.app(), None);
            let show = show_menu_overlay.borrow().clone().expect("requested");
            show(&mut tester.app());
            tester.pump();

            assert_eq!(on_open_called.get(), 1);
            expect(&tester, &find::text("Overlay"), finds_one_widget());

            controller.close(&mut tester.app());
            tester.pump();
            tester.pump_widget(TestApp::new(SizedBox::new()));

            show(&mut tester.app());

            assert_eq!(on_open_called.get(), 1);
        });
    }

    // ---- onCloseRequested ----

    #[test]
    fn on_close_requested_is_triggered_by_close() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let on_close_requested_called = counter();
            let requested = Rc::clone(&on_close_requested_called);
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_close_requested(move |_app, _hide_overlay| {
                        requested.set(requested.get() + 1);
                    })
                    .menu_panel(panel(texts(&["Button 1"])))
                    .child(AnchorButton::new("anchor")),
            ));

            assert_eq!(on_close_requested_called.get(), 0);

            controller.open(&mut tester.app(), None);
            tester.pump();

            assert_eq!(on_close_requested_called.get(), 0);

            controller.close(&mut tester.app());
            tester.pump();

            assert_eq!(on_close_requested_called.get(), 1);
        });
    }

    /// An `on_close_requested` that hides the overlay `delay` later, as Dart's
    /// `Timer(delay, hideOverlay)`.
    fn hide_after(delay: Duration) -> impl Fn(&mut App, VoidCallback) + 'static {
        move |app, hide_overlay| {
            Timer::new(app, delay, Listener::new(move |app| hide_overlay(app)));
        }
    }

    #[test]
    fn hide_overlay_triggers_on_close() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let on_close_called = counter();
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_close(bump(&on_close_called))
                    .on_close_requested(hide_after(Duration::from_millis(100)))
                    .menu_panel(panel(texts(&["Button 1"])))
                    .child(AnchorButton::new("anchor")),
            ));

            controller.open(&mut tester.app(), None);
            tester.pump();

            controller.close(&mut tester.app());
            tester.pump();

            assert_eq!(on_close_called.get(), 0);

            tester.pump_by(Duration::from_millis(99));

            assert_eq!(on_close_called.get(), 0);

            tester.pump_by(Duration::from_millis(2));

            assert_eq!(on_close_called.get(), 1);
        });
    }

    #[test]
    fn hide_overlay_hides_the_menu_overlay() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_close_requested(hide_after(Duration::from_millis(100)))
                    .overlay_builder(|_app, _context, _info| {
                        SizedBox::square(Some(300.0))
                            .child(Text::new("Overlay"))
                            .into_widget()
                    })
                    .child(AnchorButton::new("anchor")),
            ));

            expect(&tester, &find::text("Overlay"), finds_nothing());

            controller.open(&mut tester.app(), None);
            tester.pump();

            expect(&tester, &find::text("Overlay"), finds_one_widget());

            controller.close(&mut tester.app());
            tester.pump();
            tester.pump_by(Duration::from_millis(99));

            expect(&tester, &find::text("Overlay"), finds_one_widget());

            tester.pump_by(Duration::from_millis(2));

            expect(&tester, &find::text("Overlay"), finds_nothing());
        });
    }

    #[test]
    fn hide_overlay_does_nothing_after_the_menu_is_disposed() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let hide_menu_overlay: Rc<RefCell<Option<VoidCallback>>> = Rc::default();
            let on_close_called = counter();
            let sink = Rc::clone(&hide_menu_overlay);
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_close(bump(&on_close_called))
                    .on_close_requested(move |_app, hide_overlay| {
                        *sink.borrow_mut() = Some(hide_overlay);
                    })
                    .overlay_builder(|_app, _context, _info| {
                        SizedBox::square(Some(300.0))
                            .child(Text::new("Overlay"))
                            .into_widget()
                    })
                    .child(AnchorButton::new("anchor")),
            ));

            controller.open(&mut tester.app(), None);
            tester.pump();

            expect(&tester, &find::text("Overlay"), finds_one_widget());

            controller.close(&mut tester.app());
            let hide = hide_menu_overlay.borrow().clone().expect("requested");
            hide(&mut tester.app());
            tester.pump();

            assert_eq!(on_close_called.get(), 1);
            expect(&tester, &find::text("Overlay"), finds_nothing());

            controller.open(&mut tester.app(), None);
            tester.pump();

            expect(&tester, &find::text("Overlay"), finds_one_widget());

            tester.pump_widget(TestApp::new(SizedBox::new()));

            hide(&mut tester.app());

            assert_eq!(on_close_called.get(), 1);
        });
    }

    #[test]
    fn on_close_requested_is_called_on_parents_before_descendants() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let close_order = log();
            let request = |tag: &'static str| {
                let close_order = Rc::clone(&close_order);
                move |_app: &mut App, _hide_overlay: VoidCallback| {
                    close_order.borrow_mut().push(tag)
                }
            };
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    .on_close_requested(request("a"))
                    .menu_panel(panel(vec![
                        Menu::new()
                            .on_close_requested(request("a.a"))
                            .menu_panel(panel(vec![
                                Menu::new()
                                    .on_close_requested(request("a.a.a"))
                                    .menu_panel(panel(vec![SizedBox::new().into_widget()]))
                                    .child(AnchorButton::new("a.a.a"))
                                    .into_widget(),
                            ]))
                            .child(AnchorButton::new("a.a"))
                            .into_widget(),
                    ])),
            ));

            // Open parent menu.
            controller.open(&mut tester.app(), None);
            tester.pump();

            // Open submenu.
            tester.tap(find::text("a.a"));
            tester.pump();

            // Open sub-submenu.
            tester.tap(find::text("a.a.a"));
            tester.pump();

            // Menus call on_close_requested on their children when opening, so clear previous
            // close events.
            close_order.borrow_mut().clear();

            controller.close(&mut tester.app());
            tester.pump();

            assert_eq!(*close_order.borrow(), ["a", "a.a", "a.a.a"]);
        });
    }

    #[test]
    fn a_delayed_on_close_is_called_on_descendants_before_parents() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let close_order = log();
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .controller(controller.clone())
                    // Called second.
                    .on_close_requested(hide_after(Duration::from_millis(35)))
                    .on_close(push(&close_order, "a"))
                    .menu_panel(panel(vec![
                        Menu::new()
                            .on_close_requested(hide_after(Duration::from_millis(50)))
                            .on_close(push(&close_order, "a.a"))
                            .menu_panel(panel(vec![
                                Menu::new()
                                    // Called first.
                                    .on_close_requested(hide_after(Duration::from_millis(10)))
                                    .on_close(push(&close_order, "a.a.a"))
                                    .menu_panel(panel(vec![
                                        Menu::new()
                                            .on_close_requested(hide_after(Duration::from_millis(
                                                25,
                                            )))
                                            .on_close(push(&close_order, "a.a.a.a"))
                                            .menu_panel(panel(texts(&["a.a.a.a.a"])))
                                            .child(AnchorButton::new("a.a.a.a"))
                                            .into_widget(),
                                    ]))
                                    .child(AnchorButton::new("a.a.a"))
                                    .into_widget(),
                            ]))
                            .child(AnchorButton::new("a.a"))
                            .into_widget(),
                    ])),
            ));

            // Open parent menu.
            controller.open(&mut tester.app(), None);
            tester.pump();

            // Open submenu.
            tester.tap(find::text("a.a"));
            tester.pump();

            // Open sub-submenu.
            tester.tap(find::text("a.a.a"));
            tester.pump();

            tester.tap(find::text("a.a.a.a"));
            tester.pump();

            // Menus call on_close_requested on their children when opening, so clear previous
            // close events.
            close_order.borrow_mut().clear();

            controller.close(&mut tester.app());
            tester.pump();

            tester.pump_by(Duration::from_millis(9));

            assert!(close_order.borrow().is_empty());

            // 11 milliseconds.
            tester.pump_by(Duration::from_millis(2));

            assert_eq!(*close_order.borrow(), ["a.a.a.a", "a.a.a"]);

            // 34 milliseconds.
            tester.pump_by(Duration::from_millis(23));

            assert_eq!(*close_order.borrow(), ["a.a.a.a", "a.a.a"]);

            // 36 milliseconds.
            tester.pump_by(Duration::from_millis(2));

            assert_eq!(*close_order.borrow(), ["a.a.a.a", "a.a.a", "a.a", "a"]);

            tester.pump_by(Duration::from_millis(20));
        });
    }

    #[test]
    fn dismiss_menu_action_triggers_on_close_requested() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let focus_node: FocusNodeRef = FocusNode::new(&mut tester.app()).into_erased();
            let close_requests = log();
            let request = |tag: &'static str| {
                let close_requests = Rc::clone(&close_requests);
                move |_app: &mut App, _hide_overlay: VoidCallback| {
                    close_requests.borrow_mut().push(tag);
                }
            };
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .on_close_requested(request("a"))
                    .menu_panel(panel(vec![
                        Menu::new()
                            .on_close_requested(request("a.a"))
                            .menu_panel(panel(vec![
                                Menu::new()
                                    .controller(controller.clone())
                                    .on_close_requested(request("a.a.a"))
                                    .menu_panel(panel(texts(&["a.a.a.a"])))
                                    .child(
                                        AnchorButton::new("a.a.a").focus_node(focus_node.clone()),
                                    )
                                    .into_widget(),
                            ]))
                            .child(AnchorButton::new("a.a"))
                            .into_widget(),
                    ]))
                    .child(AnchorButton::new("a")),
            ));

            tester.tap(find::text("a"));
            tester.pump();
            tester.tap(find::text("a.a"));
            tester.pump();
            tester.tap(find::text("a.a.a"));
            tester.pump();

            focus_node.request_focus(&mut tester.app(), None);
            tester.pump();

            expect(&tester, &find::text("a.a.a.a"), finds_one_widget());
            assert_eq!(*close_requests.borrow(), ["a.a", "a.a.a"]);

            close_requests.borrow_mut().clear();

            {
                let mut app = tester.app();
                let action = DismissMenuAction::new(&mut app, controller);
                let context = focus_node.context(&app);
                PlainActionDispatcher::new().invoke_action(
                    &mut app,
                    Action::as_action(action.handle()),
                    &DismissIntent::new(),
                    context,
                );
            }

            tester.pump();

            assert_eq!(*close_requests.borrow(), ["a", "a.a", "a.a.a"]);
            expect(&tester, &find::text("a.a.a.a"), finds_one_widget());
        });
    }

    #[test]
    fn an_outside_tap_triggers_on_close_requested() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let close_requests = counter();
            let requests = Rc::clone(&close_requests);
            tester.pump_widget(TestApp::new(
                Menu::new()
                    .on_close_requested(move |app, hide_overlay| {
                        requests.set(requests.get() + 1);
                        hide_overlay(app);
                    })
                    .child(AnchorButton::new("anchor"))
                    .overlay_builder(|_app, context, info| {
                        Positioned::new(
                            TapRegion::new(Text::new("a"))
                                .group_id(info.tap_region_group_id)
                                .on_tap_outside(Rc::new(move |app, _event| {
                                    if let Some(controller) = MenuController::maybe_of(app, context)
                                    {
                                        controller.close(app);
                                    }
                                })),
                        )
                        .left(info.anchor_rect.left)
                        .top(info.anchor_rect.bottom)
                        .into_widget()
                    }),
            ));

            tester.tap(find::text("anchor"));
            tester.pump();

            expect(&tester, &find::text("a"), finds_one_widget());
            assert_eq!(close_requests.get(), 0);

            // Tap outside the menu to trigger the close request.
            let pointer = tester.next_pointer();
            tester.tap_at(Offset::ZERO, pointer);
            tester.pump();

            assert_eq!(close_requests.get(), 1);
            expect(&tester, &find::text("a"), finds_nothing());
        });
    }

    #[test]
    fn a_view_size_change_triggers_on_close_requested_on_an_open_menu() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let on_close_requested_called = counter();
            let requests = Rc::clone(&on_close_requested_called);
            let on_close_requested: RawMenuAnchorCloseRequestedCallback =
                Rc::new(move |app, hide_overlay| {
                    requests.set(requests.get() + 1);
                    hide_overlay(app);
                });
            let media_query_data = tester.media_query_data();
            let tree = |size: Size| -> WidgetRef {
                let on_close_requested = Rc::clone(&on_close_requested);
                MediaQuery::new(
                    media_query_data.copy_with().size(size),
                    TestApp::new(
                        Menu::new()
                            .on_close_requested(move |app, hide_overlay| {
                                on_close_requested(app, hide_overlay)
                            })
                            .controller(controller.clone())
                            .menu_panel(panel(texts(&["a"])))
                            .child(AnchorButton::new("anchor")),
                    ),
                )
                .into_widget()
            };

            tester.pump_widget(tree(media_query_data.size));
            tester.pump();

            // Test menu resize before first open.
            tester.pump_widget(tree(Size::new(250.0, 250.0)));
            tester.tap(find::text("anchor"));
            tester.pump();

            assert_eq!(on_close_requested_called.get(), 0);
            expect(&tester, &find::text("a"), finds_one_widget());

            tester.pump_widget(tree(Size::new(200.0, 200.0)));

            assert_eq!(on_close_requested_called.get(), 1);

            tester.pump();

            assert!(!controller.is_open(&tester.app()));

            // Test menu resize after close.
            tester.pump_widget(tree(Size::new(300.0, 300.0)));

            assert_eq!(on_close_requested_called.get(), 1);
        });
    }

    #[test]
    fn an_ancestor_scroll_triggers_on_close_requested_on_an_open_menu() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let scroll_controller = scroll_controller(&mut tester.app());
            let on_close_requested_called = counter();
            let requests = Rc::clone(&on_close_requested_called);
            tester.pump_widget(TestApp::new(
                SingleChildScrollView::new()
                    .controller(scroll_controller.clone())
                    .child(
                        SizedBox::new().height(1000.0).child(
                            Menu::new()
                                .on_close_requested(move |app, hide_overlay| {
                                    requests.set(requests.get() + 1);
                                    hide_overlay(app);
                                })
                                .menu_panel(panel(vec![
                                    Button::tag("a").into_widget(),
                                    Button::tag("b").into_widget(),
                                    Button::tag("c").into_widget(),
                                    Button::tag("d").into_widget(),
                                ]))
                                .child(AnchorButton::new("anchor")),
                        ),
                    ),
            ));

            scroll_controller.jump_to(&mut tester.app(), 10.0);
            tester.pump();

            // The menu is not open, so on_close_requested should not be called.
            assert_eq!(on_close_requested_called.get(), 0);

            tester.tap(find::text("anchor"));
            tester.pump();

            expect(&tester, &find::text("a"), finds_one_widget());
            assert_eq!(on_close_requested_called.get(), 0);

            scroll_controller.jump_to(&mut tester.app(), 500.0);
            tester.pump();

            // Make sure we didn't just scroll the menu off-screen.
            expect(&tester, &find::text("anchor"), finds_one_widget());
            assert_eq!(on_close_requested_called.get(), 1);
            expect(&tester, &find::text("a"), finds_nothing());

            scroll_controller.jump_to(&mut tester.app(), 5.0);
            tester.pump();

            expect(&tester, &find::text("anchor"), finds_one_widget());
            assert_eq!(on_close_requested_called.get(), 1);
        });
    }

    /// A focus node labelled as Dart's `FocusNode(debugLabel: tag.focusNode)`.
    fn labelled_focus_node(app: &mut App, tag: &str) -> FocusNodeRef {
        let node: FocusNodeRef = FocusNode::new(app).into_erased();
        node.set_debug_label(app, Some(focus_label(tag)));
        node
    }

    /// A `CallbackAction<I>` that logs `name` when invoked.
    fn logging_action<I: Intent + 'static>(
        app: &mut App,
        log: &Rc<RefCell<Vec<&'static str>>>,
        name: &'static str,
    ) -> ActionRef {
        let log = Rc::clone(log);
        Retained::into_erased(CallbackAction::<I>::new(
            app,
            Rc::new(move |_app, _intent| {
                log.borrow_mut().push(name);
                None
            }),
        ))
    }

    /// Shift-Tab, as Dart's `sendKeyDownEvent(shift)`, `sendKeyEvent(tab)` and
    /// `sendKeyUpEvent(shift)`.
    fn send_shift_tab(tester: &mut Tester) {
        tester.send_key_down_event(
            PhysicalKeyboardKey::SHIFT_LEFT,
            LogicalKeyboardKey::SHIFT_LEFT,
        );
        tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
        tester.send_key_up_event(
            PhysicalKeyboardKey::SHIFT_LEFT,
            LogicalKeyboardKey::SHIFT_LEFT,
        );
    }

    #[test]
    fn focus_traversal_shortcuts_are_not_bound_to_actions() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let anchor_focus_node = labelled_focus_node(&mut tester.app(), "anchor");
            let b_focus_node = labelled_focus_node(&mut tester.app(), "b");
            let invoked = log();
            let actions = {
                let mut app = tester.app();
                let mut actions: HashMap<TypeId, ActionRef> = HashMap::new();
                actions.insert(
                    TypeId::of::<DirectionalFocusIntent>(),
                    logging_action::<DirectionalFocusIntent>(&mut app, &invoked, "left"),
                );
                actions.insert(
                    TypeId::of::<NextFocusIntent>(),
                    logging_action::<NextFocusIntent>(&mut app, &invoked, "next"),
                );
                actions.insert(
                    TypeId::of::<PreviousFocusIntent>(),
                    logging_action::<PreviousFocusIntent>(&mut app, &invoked, "previous"),
                );
                actions
            };
            // Web doesn't automatically handle directional traversal.
            let traversal_shortcuts = || -> ShortcutMap {
                vec![
                    (
                        Rc::new(LogicalKeySet::new(
                            LogicalKeyboardKey::TAB,
                            None,
                            None,
                            None,
                        )) as ShortcutActivatorRef,
                        Rc::new(NextFocusIntent::new()) as IntentRef,
                    ),
                    (
                        Rc::new(LogicalKeySet::new(
                            LogicalKeyboardKey::SHIFT,
                            Some(LogicalKeyboardKey::TAB),
                            None,
                            None,
                        )) as ShortcutActivatorRef,
                        Rc::new(PreviousFocusIntent::new()) as IntentRef,
                    ),
                    (
                        Rc::new(LogicalKeySet::new(
                            LogicalKeyboardKey::ARROW_LEFT,
                            None,
                            None,
                            None,
                        )) as ShortcutActivatorRef,
                        Rc::new(DirectionalFocusIntent::new(TraversalDirection::Left)) as IntentRef,
                    ),
                ]
            };
            let menu_b_focus_node = b_focus_node.clone();
            tester.pump_widget(TestApp::new(
                Column::new().children([
                    Button::tag("a").into_widget(),
                    Actions::new(
                        actions,
                        RawMenuAnchor::new(controller.clone(), move |_app, _context, _position| {
                            Column::new()
                                .children([
                                    Button::tag("a").into_widget(),
                                    Shortcuts::new(
                                        traversal_shortcuts(),
                                        Button::tag("b").focus_node(menu_b_focus_node.clone()),
                                    )
                                    .into_widget(),
                                    Button::tag("d").into_widget(),
                                ])
                                .into_widget()
                        })
                        .child(AnchorButton::new("anchor").focus_node(anchor_focus_node.clone())),
                    )
                    .into_widget(),
                    Button::tag("c").into_widget(),
                ]),
            ));

            controller.open(&mut tester.app(), None);
            tester.pump();

            anchor_focus_node.request_focus(&mut tester.app(), None);
            tester.pump();

            for focused in ["anchor", "b"] {
                tester.send_key_event(
                    PhysicalKeyboardKey::ARROW_LEFT,
                    LogicalKeyboardKey::ARROW_LEFT,
                );
                assert_eq!(tester.focused_label(), Some(focus_label(focused)));

                tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
                assert_eq!(tester.focused_label(), Some(focus_label(focused)));

                send_shift_tab(&mut tester);
                assert_eq!(tester.focused_label(), Some(focus_label(focused)));

                tester.send_key_event(PhysicalKeyboardKey::HOME, LogicalKeyboardKey::HOME);
                assert_eq!(tester.focused_label(), Some(focus_label(focused)));

                tester.send_key_event(PhysicalKeyboardKey::END, LogicalKeyboardKey::END);
                assert_eq!(tester.focused_label(), Some(focus_label(focused)));

                b_focus_node.request_focus(&mut tester.app(), None);
                tester.pump();
            }

            assert_eq!(
                *invoked.borrow(),
                ["left", "next", "previous", "left", "next", "previous"]
            );
        });
    }

    // Menu implementations differ as to whether tabbing traverses a closes a menu or traverses
    // its items. By default, we let the user choose whether to close the menu or traverse its
    // items.
    #[test]
    fn tab_traversal_is_not_handled() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let controller =
                MenuController::new(&mut tester.app()).into_erased::<AnyMenuController>();
            let b_focus_node = labelled_focus_node(&mut tester.app(), "b");
            let bb_focus_node = labelled_focus_node(&mut tester.app(), "b.b");
            let invoked = log();
            let actions = {
                let mut app = tester.app();
                let mut actions: HashMap<TypeId, ActionRef> = HashMap::new();
                actions.insert(
                    TypeId::of::<NextFocusIntent>(),
                    logging_action::<NextFocusIntent>(&mut app, &invoked, "next"),
                );
                actions.insert(
                    TypeId::of::<PreviousFocusIntent>(),
                    logging_action::<PreviousFocusIntent>(&mut app, &invoked, "previous"),
                );
                actions
            };
            tester.pump_widget(TestApp::new(
                Row::new().children([Actions::new(
                    actions,
                    RawMenuAnchorGroup::new(
                        Column::new().children([
                            Button::tag("a").into_widget(),
                            Menu::new()
                                .menu_panel(panel(vec![
                                    Button::tag("b.a").into_widget(),
                                    Button::tag("b.b")
                                        .focus_node(bb_focus_node.clone())
                                        .into_widget(),
                                    Button::tag("b.c").into_widget(),
                                ]))
                                .child(AnchorButton::new("b").focus_node(b_focus_node.clone()))
                                .into_widget(),
                            Button::tag("c").into_widget(),
                        ]),
                        controller,
                    ),
                )
                .into_widget()]),
            ));

            b_focus_node.request_focus(&mut tester.app(), None);
            tester.pump();

            assert_eq!(tester.focused_label(), Some(focus_label("b")));

            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump();

            assert_eq!(tester.focused_label(), Some(focus_label("b")));

            send_shift_tab(&mut tester);
            tester.pump();

            assert_eq!(tester.focused_label(), Some(focus_label("b")));

            // Open and move focus to nested menu
            tester.tap(find::text("b"));
            tester.pump();
            bb_focus_node.request_focus(&mut tester.app(), None);
            tester.pump();

            assert_eq!(tester.focused_label(), Some(focus_label("b.b")));

            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump();

            assert_eq!(tester.focused_label(), Some(focus_label("b.b")));

            send_shift_tab(&mut tester);
            tester.pump();

            assert_eq!(tester.focused_label(), Some(focus_label("b.b")));
            assert_eq!(*invoked.borrow(), ["next", "previous", "next", "previous"]);
        });
    }

    #[test]
    fn actions_around_a_menu_are_invoked_from_both_the_anchor_and_the_overlay() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let anchor_focus_node: FocusNodeRef = FocusNode::new(&mut tester.app()).into_erased();
            let a_focus_node: FocusNodeRef = FocusNode::new(&mut tester.app()).into_erased();
            let invoked_anchor = Rc::new(Cell::new(false));
            let invoked_overlay = Rc::new(Cell::new(false));
            let actions = {
                let mut app = tester.app();
                let mut actions: HashMap<TypeId, ActionRef> = HashMap::new();
                actions.insert(
                    TypeId::of::<VoidCallbackIntent>(),
                    Retained::into_erased(CallbackAction::<VoidCallbackIntent>::new(
                        &mut app,
                        Rc::new(|app, intent: &VoidCallbackIntent| {
                            intent.callback.call(app);
                            None
                        }),
                    )),
                );
                actions
            };
            tester.pump_widget(TestApp::new(Actions::new(
                actions,
                Menu::new()
                    .focus_node(anchor_focus_node.clone())
                    .menu_panel(panel(vec![
                        Button::tag("a")
                            .focus_node(a_focus_node.clone())
                            .into_widget(),
                    ]))
                    .child(AnchorButton::new("anchor").focus_node(anchor_focus_node.clone())),
            )));

            tester.tap(find::text("anchor"));
            tester.pump();

            {
                let mut app = tester.app();
                let (anchor, overlay) = (Rc::clone(&invoked_anchor), Rc::clone(&invoked_overlay));
                let anchor_context = anchor_focus_node.context(&app).expect("attached");
                Actions::invoke(
                    &mut app,
                    anchor_context,
                    &VoidCallbackIntent::new(Listener::new(move |_app| anchor.set(true))),
                );
                let overlay_context = a_focus_node.context(&app).expect("attached");
                Actions::invoke(
                    &mut app,
                    overlay_context,
                    &VoidCallbackIntent::new(Listener::new(move |_app| overlay.set(true))),
                );
            }

            tester.pump();

            assert!(invoked_anchor.get());
            assert!(invoked_overlay.get());
        });
    }

    #[test]
    fn menus_do_not_close_when_the_root_menus_own_panel_scrolls() {
        test_widgets(|tester| {
            let mut tester = Tester::new(tester);
            let scroll_controller = scroll_controller(&mut tester.app());
            let root_opened = Rc::new(Cell::new(false));
            let opened = log();
            let closed = log();
            let large_button_constraints = BoxConstraints::tight_for(Some(200.0), Some(300.0));
            let (on_root_open, on_root_close) = (Rc::clone(&root_opened), Rc::clone(&root_opened));
            let on_close = {
                let (opened, closed) = (Rc::clone(&opened), Rc::clone(&closed));
                Rc::new(move |_app: &mut App| {
                    opened.borrow_mut().retain(|opened| *opened != "a");
                    closed.borrow_mut().push("a");
                }) as VoidCallback
            };
            tester.pump_widget(TestApp::new(
                SingleChildScrollView::new()
                    .controller(scroll_controller.clone())
                    .child(
                        Container::new()
                            .height(700.0)
                            .alignment(AlignmentGeometry::TOP_LEFT)
                            .child(
                                Menu::new()
                                    .on_open(Rc::new(move |_app| on_root_open.set(true)))
                                    .on_close(Rc::new(move |_app| on_root_close.set(false)))
                                    .menu_panel(panel(vec![
                                        Menu::new()
                                            .on_open(push(&opened, "a"))
                                            .on_close(on_close)
                                            .menu_panel(panel(vec![
                                                Button::tag("a.a")
                                                    .constraints(large_button_constraints)
                                                    .into_widget(),
                                            ]))
                                            .child(
                                                AnchorButton::new("a")
                                                    .constraints(large_button_constraints),
                                            )
                                            .into_widget(),
                                        Button::tag("b")
                                            .constraints(large_button_constraints)
                                            .into_widget(),
                                        Button::tag("c")
                                            .constraints(large_button_constraints)
                                            .into_widget(),
                                        Button::tag("d")
                                            .constraints(large_button_constraints)
                                            .into_widget(),
                                    ]))
                                    .child(AnchorButton::new("anchor")),
                            ),
                    ),
            ));

            tester.tap(find::text("anchor"));
            tester.pump();
            assert!(root_opened.get());

            // Hover the first submenu anchor.
            tester.tap(find::text("a"));
            let a_center = tester.get_center(find::text("a"));
            {
                let mut mouse = TestPointer::mouse(2, 0);
                tester.move_to(&mut mouse, a_center, false);
            }
            tester.pump();
            assert_eq!(*opened.borrow(), ["a"]);

            // Menus do not close on internal scroll.
            {
                let mut app = tester.app();
                GestureBinding::instance(&mut app).handle_pointer_event(
                    &mut app,
                    PointerEvent::Scroll(PointerScrollEvent {
                        pointer: 2,
                        position: a_center,
                        kind: PointerDeviceKind::Mouse,
                        scroll_delta: Offset::new(0.0, 30.0),
                        ..PointerScrollEvent::default()
                    }),
                );
                app.drain_microtasks();
            }
            tester.pump();
            assert!(root_opened.get());
            assert!(closed.borrow().is_empty());

            // Menus close on external scroll.
            scroll_controller.jump_to(&mut tester.app(), 700.0);
            tester.pump();
            assert!(!root_opened.get());
            assert_eq!(*closed.borrow(), ["a"]);
        });
    }

    /// Dart's `notifier.hasListeners`.
    fn has_listeners(app: &App, notifier: Handle<ValueNotifier<bool>>) -> bool {
        app.get(notifier).change_notifier_data().has_listeners()
    }
}
