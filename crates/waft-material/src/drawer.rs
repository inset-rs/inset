//! Flutter counterpart: material_ui's `drawer.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationStatus, AnimationStatusListener,
};
use waft_embedder::{Clip, Color, TargetPlatform, TextDirection};
use waft_foundation::{App, Handle, Listener};
use waft_gestures::{DragDownDetails, DragEndDetails, DragStartBehavior, DragUpdateDetails};
use waft_painting::{AlignmentGeometry, ShapeBorder};
use waft_rendering::{BoxConstraints, HitTestBehavior};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    Align, AnyModalRoute, BlockSemantics, BuildContext, ColoredBox, ConstrainedBox, Directionality,
    ExcludeSemantics, FocusNodeLeaf, FocusScope, FocusScopeNode, GestureDetector, GlobalKey,
    InheritedWidget, IntoWidget, KeyRef, LimitedBox, LocalHistoryEntry, MediaQuery,
    RepaintBoundary, Semantics, SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData,
    SizedBox, Stack, State, StateData, StatefulWidget, StatelessWidget, WidgetRef, downcast_widget,
};

use crate::colors::Colors;
use crate::debug::debug_check_has_material_localizations;
use crate::drawer_theme::{DrawerTheme, DrawerThemeData};
use crate::generated::drawer_defaults_m3::drawer_defaults_m3;
use crate::list_tile::ListTileStyle;
use crate::list_tile_theme::ListTileTheme;
use crate::material::Material;
use crate::material_localizations::MaterialLocalizations;
use crate::theme::Theme;

/// The possible alignments of a [`Drawer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawerAlignment {
    /// Denotes that the [`Drawer`] is at the start side of the `Scaffold`.
    ///
    /// This corresponds to the left side when the text direction is left-to-right
    /// and the right side when the text direction is right-to-left.
    Start,

    /// Denotes that the [`Drawer`] is at the end side of the `Scaffold`.
    ///
    /// This corresponds to the right side when the text direction is left-to-right
    /// and the left side when the text direction is right-to-left.
    End,
}

// TODO(eseidel): Draw width should vary based on device size:
// https://material.io/design/components/navigation-drawer.html#specs

// Mobile:
// Width = Screen width − 56 dp
// Maximum width: 320dp
// Maximum width applies only when using a left nav. When using a right nav,
// the panel can cover the full width of the screen.

// Desktop/Tablet:
// Maximum width for a left nav is 400dp.
// The right nav can vary depending on content.

const K_WIDTH: f64 = 304.0;
const K_EDGE_DRAG_WIDTH: f64 = 20.0;
const K_MIN_FLING_VELOCITY: f64 = 365.0;
const K_BASE_SETTLE_DURATION: Duration = Duration::from_millis(246);

/// A Material Design panel that slides in horizontally from the edge of a
/// `Scaffold` to show navigation links in an application.
///
/// There is a Material 3 version of this component, `NavigationDrawer`,
/// that's preferred for applications that are configured for Material 3
/// (see `ThemeData::use_material3`).
///
/// Drawers are typically used with the `Scaffold::drawer` property. The child of
/// the drawer is usually a `ListView` whose first child is a `DrawerHeader`
/// that displays status information about the current user. The remaining
/// drawer children are often constructed with `ListTile`s, often concluding
/// with an `AboutListTile`.
///
/// The `AppBar` automatically displays an appropriate `IconButton` to show the
/// [`Drawer`] when a [`Drawer`] is available in the `Scaffold`. The `Scaffold`
/// automatically handles the edge-swipe gesture to show the drawer.
///
/// An open drawer may be closed with a swipe to close gesture, pressing the
/// escape key, by tapping the scrim, or by calling pop route function such as
/// `Navigator::pop`.
///
/// See also:
///
///  * `Scaffold::drawer`, where one specifies a [`Drawer`] so that it can be
///    shown.
///  * `Scaffold::of`, to obtain the current `ScaffoldState`, which manages the
///    display and animation of the drawer.
///  * `ScaffoldState::open_drawer`, which displays its [`Drawer`], if any.
///  * <https://material.io/design/components/navigation-drawer.html>
#[derive(Debug, Default)]
pub struct Drawer {
    pub key: Option<KeyRef>,

    /// Sets the color of the `Material` that holds all of the [`Drawer`]'s
    /// contents.
    ///
    /// If this is null, then [`DrawerThemeData::background_color`] is used. If that
    /// is also null, then it falls back to `Material`'s default.
    pub background_color: Option<Color>,

    /// The z-coordinate at which to place this drawer relative to its parent.
    ///
    /// This controls the size of the shadow below the drawer.
    ///
    /// If this is null, then [`DrawerThemeData::elevation`] is used. If that
    /// is also null, then it defaults to 16.0.
    pub elevation: Option<f64>,

    /// The color used to paint a drop shadow under the drawer's `Material`,
    /// which reflects the drawer's [`elevation`](Self::elevation).
    ///
    /// If null and `ThemeData::use_material3` is true then no drop shadow will
    /// be rendered.
    ///
    /// If null and `ThemeData::use_material3` is false then it will default to
    /// `ThemeData::shadow_color`.
    ///
    /// See also:
    ///   * `Material::shadow_color`, which describes how the drop shadow is painted.
    ///   * [`elevation`](Self::elevation), which affects how the drop shadow is painted.
    ///   * [`surface_tint_color`](Self::surface_tint_color), which can be used to indicate
    ///     elevation through tinting the background color.
    pub shadow_color: Option<Color>,

    /// The color used as a surface tint overlay on the drawer's background color,
    /// which reflects the drawer's [`elevation`](Self::elevation).
    ///
    /// This is not recommended for use. [Material 3 spec](https://m3.material.io/styles/color/the-color-system/color-roles)
    /// introduced a set of tone-based surfaces and surface containers in its `ColorScheme`,
    /// which provide more flexibility. The intention is to eventually remove surface tint color from
    /// the framework.
    ///
    /// To disable this feature, set [`surface_tint_color`](Self::surface_tint_color) to
    /// `Colors::TRANSPARENT`.
    ///
    /// Defaults to `Colors::TRANSPARENT`.
    ///
    /// See also:
    ///   * `Material::surface_tint_color`, which describes how the surface tint will
    ///     be applied to the background color of the drawer.
    ///   * [`elevation`](Self::elevation), which affects the opacity of the surface tint.
    ///   * [`shadow_color`](Self::shadow_color), which can be used to indicate elevation
    ///     through a drop shadow.
    pub surface_tint_color: Option<Color>,

    /// The shape of the drawer.
    ///
    /// Defines the drawer's `Material::shape`.
    ///
    /// If this is null, then [`DrawerThemeData::shape`] is used. If that
    /// is also null, then it falls back to `Material`'s default.
    pub shape: Option<Box<dyn ShapeBorder>>,

    /// The width of the drawer.
    ///
    /// If this is null, then [`DrawerThemeData::width`] is used. If that is also
    /// null, then it falls back to the Material spec's default (304.0).
    pub width: Option<f64>,

    /// The widget below this widget in the tree.
    ///
    /// Typically a `ListView`.
    pub child: Option<WidgetRef>,

    /// The semantic label of the drawer used by accessibility frameworks to
    /// announce screen transitions when the drawer is opened and closed.
    ///
    /// If this label is not provided, it will default to
    /// `MaterialLocalizations::drawer_label`.
    pub semantic_label: Option<String>,

    /// The `clip_behavior` argument specifies how to clip the drawer's
    /// [`shape`](Self::shape).
    ///
    /// If the drawer has a [`shape`](Self::shape), it defaults to [`Clip::HardEdge`].
    /// Otherwise, defaults to [`Clip::None`].
    pub clip_behavior: Option<Clip>,
}

impl Drawer {
    /// Creates a Material Design drawer; Dart's optional named arguments are the setters.
    ///
    /// Typically used in the `Scaffold::drawer` property.
    ///
    /// The [`elevation`](Self::elevation) must be non-negative.
    pub fn new() -> Drawer {
        Drawer::default()
    }

    /// Dart `Drawer(key:)`.
    pub fn key(mut self, key: KeyRef) -> Drawer {
        self.key = Some(key);
        self
    }

    /// Dart `Drawer(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> Drawer {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `Drawer(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> Drawer {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `Drawer(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> Drawer {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `Drawer(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> Drawer {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `Drawer(shape:)`.
    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> Drawer {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `Drawer(width:)`.
    pub fn width(mut self, width: f64) -> Drawer {
        self.width = Some(width);
        self
    }

    /// Dart `Drawer(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Drawer {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `Drawer(semanticLabel:)`.
    pub fn semantic_label(mut self, semantic_label: impl Into<String>) -> Drawer {
        self.semantic_label = Some(semantic_label.into());
        self
    }

    /// Dart `Drawer(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Drawer {
        self.clip_behavior = Some(clip_behavior);
        self
    }
}

impl StatelessWidget for Drawer {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material_localizations(app, context));
        let drawer_theme = DrawerTheme::of(app, context);
        let label = match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => self.semantic_label.clone(),
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                Some(self.semantic_label.clone().unwrap_or_else(|| {
                    <dyn MaterialLocalizations>::of(app, context).drawer_label()
                }))
            }
        };
        let use_material3 = Theme::of(app, context).use_material3;
        let is_drawer_start = DrawerController::maybe_of(app, context)
            .map(|controller| controller.alignment)
            != Some(DrawerAlignment::End);
        let defaults = if use_material3 {
            drawer_defaults_m3(app, context)
        } else {
            drawer_defaults_m2(app, context)
        };
        let effective_shape: Option<Box<dyn ShapeBorder>> = self.shape.clone().or_else(|| {
            if is_drawer_start {
                drawer_theme.shape.clone().or(defaults.shape.clone())
            } else {
                drawer_theme
                    .end_shape
                    .clone()
                    .or(defaults.end_shape.clone())
            }
        });
        let clip_behavior = if effective_shape.is_some() {
            self.clip_behavior
                .or(drawer_theme.clip_behavior)
                .or(defaults.clip_behavior)
                .expect("the defaults give a clip behavior")
        } else {
            Clip::None
        };
        let mut material = Material::new()
            .elevation(
                self.elevation
                    .or(drawer_theme.elevation)
                    .or(defaults.elevation)
                    .expect("the defaults give an elevation"),
            )
            .clip_behavior(clip_behavior);
        material.color = self
            .background_color
            .or(drawer_theme.background_color)
            .or(defaults.background_color);
        material.shadow_color = self
            .shadow_color
            .or(drawer_theme.shadow_color)
            .or(defaults.shadow_color);
        material.surface_tint_color = self
            .surface_tint_color
            .or(drawer_theme.surface_tint_color)
            .or(defaults.surface_tint_color);
        material.shape = effective_shape;
        material.child = self.child.clone();
        let mut semantics = Semantics::new()
            .scopes_route(true)
            .names_route(true)
            .explicit_child_nodes(true);
        if let Some(label) = label {
            semantics = semantics.label(label);
        }
        semantics
            .child(
                ConstrainedBox::new(BoxConstraints::expand(
                    Some(self.width.or(drawer_theme.width).unwrap_or(K_WIDTH)),
                    None,
                ))
                .child(material),
            )
            .into_widget()
    }
}

/// Signature for the callback that's called when a [`DrawerController`] is
/// opened or closed.
pub type DrawerCallback = Rc<dyn Fn(&mut App, bool)>;

/// Dart's `_DrawerControllerScope`.
#[derive(Debug)]
struct DrawerControllerScope {
    /// The [`DrawerController`] widget, held as the reference the element was given.
    controller: WidgetRef,
    child: WidgetRef,
}

impl DrawerControllerScope {
    fn controller(&self) -> &DrawerController {
        downcast_widget::<DrawerController>(&*self.controller)
            .expect("the scope holds a DrawerController")
    }
}

impl InheritedWidget for DrawerControllerScope {
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &DrawerControllerScope) -> bool {
        !Rc::ptr_eq(&self.controller, &old.controller)
    }
}

/// Provides interactive behavior for [`Drawer`] widgets.
///
/// Rarely used directly. Drawer controllers are typically created automatically
/// by `Scaffold` widgets.
///
/// The drawer controller provides the ability to open and close a drawer, either
/// via an animation or via user interaction. When closed, the drawer collapses
/// to a translucent gesture detector that can be used to listen for edge
/// swipes.
///
/// See also:
///
///  * [`Drawer`], a container with the default width of a drawer.
///  * `Scaffold::drawer`, the `Scaffold` slot for showing a drawer.
pub struct DrawerController {
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    ///
    /// Typically a [`Drawer`].
    pub child: WidgetRef,

    /// The alignment of the [`Drawer`].
    ///
    /// This controls the direction in which the user should swipe to open and
    /// close the drawer.
    pub alignment: DrawerAlignment,

    /// Optional callback that is called when a [`Drawer`] is opened or closed.
    pub drawer_callback: Option<DrawerCallback>,

    /// Whether tapping the barrier behind the [`Drawer`] dismisses it.
    ///
    /// Defaults to true.
    ///
    /// If false, tapping the barrier will not dismiss the drawer.
    pub drawer_barrier_dismissible: bool,

    /// Determines the way that drag start behavior is handled.
    ///
    /// If set to [`DragStartBehavior::Start`], the drag behavior used for opening
    /// and closing a drawer will begin at the position where the drag gesture won
    /// the arena. If set to [`DragStartBehavior::Down`] it will begin at the position
    /// where a down event is first detected.
    ///
    /// In general, setting this to [`DragStartBehavior::Start`] will make drag
    /// animation smoother and setting it to [`DragStartBehavior::Down`] will make
    /// drag behavior feel slightly more reactive.
    ///
    /// By default, the drag start behavior is [`DragStartBehavior::Start`].
    ///
    /// See also:
    ///
    ///  * `DragGestureRecognizer::drag_start_behavior`, which gives an example for
    ///    the different behaviors.
    pub drag_start_behavior: DragStartBehavior,

    /// The color to use for the scrim that obscures the underlying content while
    /// a drawer is open.
    ///
    /// If this is null, then [`DrawerThemeData::scrim_color`] is used. If that
    /// is also null, then it defaults to `Colors::BLACK54`.
    pub scrim_color: Option<Color>,

    /// Determines if the [`Drawer`] can be opened with a drag gesture.
    ///
    /// By default, the drag gesture is enabled.
    pub enable_open_drag_gesture: bool,

    /// The width of the area within which a horizontal swipe will open the
    /// drawer.
    ///
    /// By default, the value used is 20.0 added to the padding edge of
    /// `MediaQuery::padding_of(context)` that corresponds to [`alignment`](Self::alignment).
    /// This ensures that the drag area for notched devices is not obscured. For
    /// example, if [`alignment`](Self::alignment) is set to [`DrawerAlignment::Start`] and
    /// `Directionality::of(context)` is set to [`TextDirection::Ltr`],
    /// 20.0 will be added to `MediaQuery::padding_of(context).left`.
    pub edge_drag_width: Option<f64>,

    /// Whether or not the drawer is opened or closed.
    ///
    /// This parameter is primarily used by the state restoration framework
    /// to restore the drawer's animation controller to the open or closed state
    /// depending on what was last saved to the target platform before the
    /// application was killed.
    pub is_drawer_open: bool,
}

impl DrawerController {
    /// Creates a controller for a [`Drawer`]; Dart's optional named arguments are the
    /// setters.
    ///
    /// Rarely used directly.
    ///
    /// The [`child`](Self::child) argument is typically a [`Drawer`].
    pub fn new<K>(child: impl IntoWidget<K>, alignment: DrawerAlignment) -> DrawerController {
        DrawerController {
            key: None,
            child: child.into_widget(),
            alignment,
            drawer_callback: None,
            drawer_barrier_dismissible: true,
            drag_start_behavior: DragStartBehavior::Start,
            scrim_color: None,
            enable_open_drag_gesture: true,
            edge_drag_width: None,
            is_drawer_open: false,
        }
    }

    /// Dart `DrawerController(key:)`.
    pub fn key(mut self, key: GlobalKey) -> DrawerController {
        self.key = Some(key.into());
        self
    }

    /// Dart `DrawerController(isDrawerOpen:)`.
    pub fn is_drawer_open(mut self, is_drawer_open: bool) -> DrawerController {
        self.is_drawer_open = is_drawer_open;
        self
    }

    /// Dart `DrawerController(drawerCallback:)`.
    pub fn drawer_callback(mut self, drawer_callback: DrawerCallback) -> DrawerController {
        self.drawer_callback = Some(drawer_callback);
        self
    }

    /// Dart `DrawerController(dragStartBehavior:)`.
    pub fn drag_start_behavior(
        mut self,
        drag_start_behavior: DragStartBehavior,
    ) -> DrawerController {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `DrawerController(scrimColor:)`.
    pub fn scrim_color(mut self, scrim_color: Color) -> DrawerController {
        self.scrim_color = Some(scrim_color);
        self
    }

    /// Dart `DrawerController(edgeDragWidth:)`.
    pub fn edge_drag_width(mut self, edge_drag_width: f64) -> DrawerController {
        self.edge_drag_width = Some(edge_drag_width);
        self
    }

    /// Dart `DrawerController(enableOpenDragGesture:)`.
    pub fn enable_open_drag_gesture(mut self, enable_open_drag_gesture: bool) -> DrawerController {
        self.enable_open_drag_gesture = enable_open_drag_gesture;
        self
    }

    /// Dart `DrawerController(drawerBarrierDismissible:)`.
    pub fn drawer_barrier_dismissible(
        mut self,
        drawer_barrier_dismissible: bool,
    ) -> DrawerController {
        self.drawer_barrier_dismissible = drawer_barrier_dismissible;
        self
    }

    /// The closest instance of [`DrawerController`] that encloses the given
    /// context, or null if none is found.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let controller = DrawerController::maybe_of(app, context);
    /// ```
    ///
    /// Calling this method will create a dependency on the closest
    /// [`DrawerController`] in the `context`, if there is one.
    ///
    /// See also:
    ///
    /// * [`DrawerController::of`], which is similar to this method, but asserts
    ///   if no [`DrawerController`] ancestor is found.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<&DrawerController> {
        context
            .depend_on_inherited_widget_of_exact_type::<DrawerControllerScope>(app)
            .map(DrawerControllerScope::controller)
    }

    /// The closest instance of [`DrawerController`] that encloses the given
    /// context.
    ///
    /// If no instance is found, this method will assert in debug mode and throw
    /// an exception in release mode.
    ///
    /// Calling this method will create a dependency on the closest
    /// [`DrawerController`] in the `context`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let controller = DrawerController::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> &DrawerController {
        if DrawerController::maybe_of(app, context).is_none() {
            panic!(
                "DrawerController.of() was called with a context that does not contain a \
                 DrawerController widget.\nNo DrawerController widget ancestor could be found \
                 starting from the context that was passed to DrawerController.of(). This can \
                 happen because you are using a widget that looks for a DrawerController \
                 ancestor, but no such ancestor exists.\nThe context used was:\n  {}",
                context.widget(app).type_name(),
            );
        }
        DrawerController::maybe_of(app, context).expect("checked above")
    }
}

impl Debug for DrawerController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DrawerController")
            .field("key", &self.key)
            .field("alignment", &self.alignment)
            .field("is_drawer_open", &self.is_drawer_open)
            .field("drag_start_behavior", &self.drag_start_behavior)
            .field("scrim_color", &self.scrim_color)
            .field("edge_drag_width", &self.edge_drag_width)
            .field("enable_open_drag_gesture", &self.enable_open_drag_gesture)
            .field(
                "drawer_barrier_dismissible",
                &self.drawer_barrier_dismissible,
            )
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for DrawerController {
    type State = DrawerControllerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> DrawerControllerState {
        DrawerControllerState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            disposed: false,
            history_entry: None,
            focus_scope_node: None,
            controller: None,
            drawer_key: GlobalKey::new(),
            previously_opened: false,
            gesture_detector_key: GlobalKey::new(),
        }
    }
}

/// State for a [`DrawerController`].
///
/// Typically used by a `Scaffold` to [`open`](Self::open) and [`close`](Self::close) the
/// drawer.
pub struct DrawerControllerState {
    state: StateData<DrawerController>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    disposed: bool,
    history_entry: Option<Handle<LocalHistoryEntry>>,
    /// Dart's field initializer `FocusScopeNode()`, made where `init_state` starts.
    focus_scope_node: Option<Handle<FocusScopeNode>>,
    controller: Option<Handle<AnimationController>>,
    drawer_key: GlobalKey,
    previously_opened: bool,
    gesture_detector_key: GlobalKey,
}

impl DrawerControllerState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self).controller.expect("created in init_state")
    }

    fn focus_scope_node(self: Handle<Self>, app: &App) -> Handle<FocusScopeNode> {
        app.get(self)
            .focus_scope_node
            .expect("created in init_state")
    }

    fn animation_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {
            // The animation controller's state is our build state, and it changed already.
        });
    }

    fn ensure_history_entry(self: Handle<Self>, app: &mut App) {
        if app.get(self).history_entry.is_none() {
            let context = self.context(app);
            let route: Option<AnyModalRoute> = AnyModalRoute::of(app, context);
            if let Some(route) = route {
                let history_entry = LocalHistoryEntry::new(app)
                    .on_remove(
                        app,
                        Listener::handle_method(self, Self::handle_history_entry_removed),
                    )
                    .implies_app_bar_dismissal(app, false);
                app.get_mut(self).history_entry = Some(history_entry);
                route.add_local_history_entry(app, history_entry);
                let focus_scope_node = self.focus_scope_node(app);
                FocusScope::of(app, context, true).set_first_focus(app, focus_scope_node);
            }
        }
    }

    fn remove_history_entry(self: Handle<Self>, app: &mut App) {
        if let Some(history_entry) = app.get(self).history_entry {
            history_entry.remove(app);
        }
        app.get_mut(self).history_entry = None;
    }

    fn animation_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        match status {
            AnimationStatus::Forward => self.ensure_history_entry(app),
            AnimationStatus::Reverse | AnimationStatus::Dismissed => {
                self.remove_history_entry(app);
            }
            AnimationStatus::Completed => {}
        }
    }

    fn handle_history_entry_removed(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).history_entry = None;
        if app.get(self).disposed || !self.mounted(app) {
            return;
        }
        let controller = self.controller(app);
        if !controller.is_dismissed(app) && controller.status(app) != AnimationStatus::Reverse {
            self.close(app);
        }
    }

    fn handle_drag_down(self: Handle<Self>, app: &mut App, _details: DragDownDetails) {
        self.controller(app).stop(app, true);
        self.ensure_history_entry(app);
    }

    fn handle_drag_cancel(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        if controller.is_dismissed(app) || Animation::is_animating(controller, app) {
            return;
        }
        if controller.value(app) < 0.5 {
            self.close(app);
        } else {
            self.open(app);
        }
    }

    fn width(self: Handle<Self>, app: &mut App) -> f64 {
        let drawer_key = app.get(self).drawer_key.clone();
        let size = drawer_key
            .current_context(app)
            .and_then(|context| context.find_render_object(app))
            .and_then(|render_object| render_object.as_box())
            .map(|render_box| render_box.size(app));
        // return K_WIDTH if drawer not being shown currently
        size.map_or(K_WIDTH, |size| size.width())
    }

    fn direction_factor(self: Handle<Self>, app: &mut App) -> f64 {
        let context = self.context(app);
        match (Directionality::of(app, context), self.widget(app).alignment) {
            (TextDirection::Rtl, DrawerAlignment::Start) => -1.0,
            (TextDirection::Rtl, DrawerAlignment::End) => 1.0,
            (TextDirection::Ltr, DrawerAlignment::Start) => 1.0,
            (TextDirection::Ltr, DrawerAlignment::End) => -1.0,
        }
    }

    fn r#move(self: Handle<Self>, app: &mut App, details: DragUpdateDetails) {
        let controller = self.controller(app);
        let primary_delta = details
            .primary_delta
            .expect("a horizontal drag has a primary delta");
        let delta = primary_delta / self.width(app) * self.direction_factor(app);
        controller.set_value(app, controller.value(app) + delta);

        let opened = controller.value(app) > 0.5;
        if opened != app.get(self).previously_opened
            && let Some(drawer_callback) = self.widget(app).drawer_callback.clone()
        {
            drawer_callback(app, opened);
        }
        app.get_mut(self).previously_opened = opened;
    }

    fn settle(self: Handle<Self>, app: &mut App, details: DragEndDetails) {
        let controller = self.controller(app);
        if controller.is_dismissed(app) {
            return;
        }
        let x_velocity = details.velocity.pixels_per_second.dx();
        if x_velocity.abs() >= K_MIN_FLING_VELOCITY {
            let visual_velocity = x_velocity / self.width(app) * self.direction_factor(app);

            drop(controller.fling(app, visual_velocity, None, None));
            if visual_velocity < 0.0 {
                // We explicitly remove the history entry here because the drawer may already be
                // animating closed (status == AnimationStatus.reverse) when the drag begins.
                // In that case, flinging it closed again does not change the animation status,
                // so the status listener will not fire.
                self.remove_history_entry(app);
            }
            if let Some(drawer_callback) = self.widget(app).drawer_callback.clone() {
                drawer_callback(app, visual_velocity > 0.0);
            }
        } else if controller.value(app) < 0.5 {
            self.close(app);
        } else {
            self.open(app);
        }
    }

    /// Starts an animation to open the drawer.
    ///
    /// Typically called by `ScaffoldState::open_drawer`.
    pub fn open(self: Handle<Self>, app: &mut App) {
        drop(self.controller(app).fling(app, 1.0, None, None));
        if let Some(drawer_callback) = self.widget(app).drawer_callback.clone() {
            drawer_callback(app, true);
        }
    }

    /// Starts an animation to close the drawer.
    pub fn close(self: Handle<Self>, app: &mut App) {
        drop(self.controller(app).fling(app, -1.0, None, None));
        // Explicitly removed here for the same reason as in settle.
        self.remove_history_entry(app);
        if let Some(drawer_callback) = self.widget(app).drawer_callback.clone() {
            drawer_callback(app, false);
        }
    }

    fn drawer_outer_alignment(self: Handle<Self>, app: &App) -> AlignmentGeometry {
        match self.widget(app).alignment {
            DrawerAlignment::Start => AlignmentGeometry::CENTER_START,
            DrawerAlignment::End => AlignmentGeometry::CENTER_END,
        }
    }

    fn drawer_inner_alignment(self: Handle<Self>, app: &App) -> AlignmentGeometry {
        match self.widget(app).alignment {
            DrawerAlignment::Start => AlignmentGeometry::CENTER_END,
            DrawerAlignment::End => AlignmentGeometry::CENTER_START,
        }
    }

    fn build_drawer(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let is_desktop = match Theme::of(app, context).platform {
            TargetPlatform::Android | TargetPlatform::IOS | TargetPlatform::Fuchsia => false,
            TargetPlatform::MacOS | TargetPlatform::Linux | TargetPlatform::Windows => true,
        };

        let drag_area_width = match self.widget(app).edge_drag_width {
            Some(edge_drag_width) => edge_drag_width,
            None => {
                let alignment = self.widget(app).alignment;
                K_EDGE_DRAG_WIDTH
                    + match (alignment, Directionality::of(app, context)) {
                        (DrawerAlignment::Start, TextDirection::Ltr) => {
                            MediaQuery::padding_of(app, context).left
                        }
                        (DrawerAlignment::Start, TextDirection::Rtl) => {
                            MediaQuery::padding_of(app, context).right
                        }
                        (DrawerAlignment::End, TextDirection::Rtl) => {
                            MediaQuery::padding_of(app, context).left
                        }
                        (DrawerAlignment::End, TextDirection::Ltr) => {
                            MediaQuery::padding_of(app, context).right
                        }
                    }
            }
        };

        let controller = self.controller(app);
        let gesture_detector_key = app.get(self).gesture_detector_key.clone();
        if controller.is_dismissed(app) {
            if self.widget(app).enable_open_drag_gesture && !is_desktop {
                Align::new()
                    .alignment(self.drawer_outer_alignment(app))
                    .child(
                        GestureDetector::new()
                            .key(gesture_detector_key.into())
                            .on_horizontal_drag_update(Rc::new(
                                move |app: &mut App, details: DragUpdateDetails| {
                                    self.r#move(app, details);
                                },
                            ))
                            .on_horizontal_drag_end(Rc::new(
                                move |app: &mut App, details: DragEndDetails| {
                                    self.settle(app, details);
                                },
                            ))
                            .behavior(HitTestBehavior::Translucent)
                            .drag_start_behavior(self.widget(app).drag_start_behavior)
                            .child(LimitedBox::new().max_height(0.0).child(
                                SizedBox::new().width(drag_area_width).height(f64::INFINITY),
                            )),
                    )
                    .into_widget()
            } else {
                SizedBox::shrink().into_widget()
            }
        } else {
            let platform_has_back_button = match app.platform().target_platform() {
                TargetPlatform::Android => true,
                TargetPlatform::IOS
                | TargetPlatform::MacOS
                | TargetPlatform::Fuchsia
                | TargetPlatform::Linux
                | TargetPlatform::Windows => false,
            };

            let scrim_color = match self.widget(app).scrim_color {
                Some(scrim_color) => scrim_color,
                None => DrawerTheme::of(app, context)
                    .scrim_color
                    .unwrap_or(Colors::BLACK54),
            };
            let effective_scrim_color = scrim_color.with_values(
                Some(scrim_color.a * controller.value(app)),
                None,
                None,
                None,
                None,
            );
            let drawer_scrim = ColoredBox::new(effective_scrim_color).child(
                LimitedBox::new()
                    .max_width(0.0)
                    .max_height(0.0)
                    .child(SizedBox::expand()),
            );

            let mut barrier = GestureDetector::new().child(
                Semantics::new()
                    .label(
                        <dyn MaterialLocalizations>::of(app, context).modal_barrier_dismiss_label(),
                    )
                    .child(drawer_scrim),
            );
            if self.widget(app).drawer_barrier_dismissible {
                barrier = barrier.on_tap(Listener::handle_method(self, Self::close));
            }

            let drawer_key = app.get(self).drawer_key.clone();
            let focus_scope_node = self.focus_scope_node(app);
            let child = DrawerControllerScope {
                controller: context.widget(app).clone(),
                child: RepaintBoundary::new()
                    .child(
                        Stack::new().children([
                            BlockSemantics::new()
                                .child(
                                    ExcludeSemantics::new()
                                        // On Android, the back button is used to dismiss a modal.
                                        .excluding(platform_has_back_button)
                                        .child(barrier),
                                )
                                .into_widget(),
                            Align::new()
                                .alignment(self.drawer_outer_alignment(app))
                                .child(
                                    Align::new()
                                        .alignment(self.drawer_inner_alignment(app))
                                        .width_factor(controller.value(app))
                                        .child(
                                            RepaintBoundary::new().child(
                                                FocusScope::new(self.widget(app).child.clone())
                                                    .key(drawer_key.into())
                                                    .node(focus_scope_node),
                                            ),
                                        ),
                                )
                                .into_widget(),
                        ]),
                    )
                    .into_widget(),
            };

            if is_desktop {
                return child.into_widget();
            }

            GestureDetector::new()
                .key(gesture_detector_key.into())
                .on_horizontal_drag_down(Rc::new(move |app: &mut App, details: DragDownDetails| {
                    self.handle_drag_down(app, details);
                }))
                .on_horizontal_drag_update(Rc::new(
                    move |app: &mut App, details: DragUpdateDetails| {
                        self.r#move(app, details);
                    },
                ))
                .on_horizontal_drag_end(Rc::new(move |app: &mut App, details: DragEndDetails| {
                    self.settle(app, details);
                }))
                .on_horizontal_drag_cancel(Listener::handle_method(self, Self::handle_drag_cancel))
                .drag_start_behavior(self.widget(app).drag_start_behavior)
                .child(child)
                .into_widget()
        }
    }
}

impl SingleTickerProviderStateMixin for DrawerControllerState {
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

impl TickerProviderObject for DrawerControllerState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for DrawerControllerState {
    type Widget = DrawerController;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let focus_scope_node = FocusScopeNode::new(app);
        app.get_mut(self).focus_scope_node = Some(focus_scope_node);
        let value = if self.widget(app).is_drawer_open {
            1.0
        } else {
            0.0
        };
        let controller = AnimationController::create(
            app,
            Some(value),
            Some(K_BASE_SETTLE_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).controller = Some(controller);
        controller.add_listener(app, Listener::handle_method(self, Self::animation_changed));
        controller.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, Self::animation_status_changed),
        );
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).disposed = true;
        self.remove_history_entry(app);
        self.controller(app).dispose(app);
        self.focus_scope_node(app).dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &DrawerController) {
        let controller = self.controller(app);
        if controller.status(app).is_animating() {
            return; // Don't snap the drawer open or shut while the user is dragging.
        }
        let is_drawer_open = self.widget(app).is_drawer_open;
        if is_drawer_open != old_widget.is_drawer_open {
            controller.set_value(app, if is_drawer_open { 1.0 } else { 0.0 });
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material_localizations(app, context));
        ListTileTheme::merge(self.build_drawer(app, context))
            .style(ListTileStyle::Drawer)
            .into_widget()
    }
}

/// Dart's `_DrawerDefaultsM2(context)`: the [`DrawerThemeData`] whose fields are the
/// constructor's arguments and the value of that subclass's getter.
fn drawer_defaults_m2(app: &mut App, context: BuildContext) -> DrawerThemeData {
    DrawerThemeData::new()
        .elevation(16.0)
        .clip_behavior(Clip::HardEdge)
        .shadow_color(Theme::of(app, context).shadow_color)
}

#[cfg(test)]
mod tests {
    //! The cases of `test/drawer_test.dart` and `test/drawer_theme_test.dart`'s widget cases.
    //! scaffold_test.dart's drawer cases are in `scaffold.rs`.

    use std::cell::{Cell, RefCell};

    use waft_embedder::{Offset, Radius, Rect, Size};
    use waft_painting::{BorderRadius, BorderRadiusGeometry, BorderSide, RoundedRectangleBorder};
    use waft_rendering::AnyRenderBox;
    use waft_widgets::{
        AnyElement, Builder, Center, Container, ListView, ModalBarrier, MouseRegion,
        PreferredSizeWidgetRef, Text,
    };

    use super::*;
    use crate::app_bar::AppBar;
    use crate::drawer_header::DrawerHeader;
    use crate::icons::Icons;
    use crate::list_tile::ListTile;
    use crate::scaffold::{Scaffold, ScaffoldState};
    use crate::test_support::{
        TestPointer, WidgetTester, is_same_color_as, restorable_material_app,
        with_main_thread_stack,
    };
    use crate::text_button::TextButton;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3, pump_app, tester};

    /// Dart's `scaffoldKey.currentState!`, or `tester.firstState(find.byType(Scaffold))` of the
    /// one scaffold, which the case keys.
    fn scaffold_state(tester: &WidgetTester, key: &GlobalKey) -> Handle<ScaffoldState> {
        key.current_state::<ScaffoldState>(&mut tester.cell.borrow_mut())
            .expect("a mounted scaffold")
    }

    fn open_drawer(tester: &WidgetTester, key: &GlobalKey) {
        let state = scaffold_state(tester, key);
        state.open_drawer(&mut tester.cell.borrow_mut());
    }

    fn open_end_drawer(tester: &WidgetTester, key: &GlobalKey) {
        let state = scaffold_state(tester, key);
        state.open_end_drawer(&mut tester.cell.borrow_mut());
    }

    fn close_drawer(tester: &WidgetTester, key: &GlobalKey) {
        let state = scaffold_state(tester, key);
        state.close_drawer(&mut tester.cell.borrow_mut());
    }

    fn close_end_drawer(tester: &WidgetTester, key: &GlobalKey) {
        let state = scaffold_state(tester, key);
        state.close_end_drawer(&mut tester.cell.borrow_mut());
    }

    fn is_drawer_open(tester: &WidgetTester, key: &GlobalKey) -> bool {
        scaffold_state(tester, key).is_drawer_open(&tester.cell.borrow())
    }

    fn is_end_drawer_open(tester: &WidgetTester, key: &GlobalKey) -> bool {
        scaffold_state(tester, key).is_end_drawer_open(&tester.cell.borrow())
    }

    fn found(tester: &WidgetTester, text: &str) -> usize {
        tester.find_text(text).len()
    }

    fn drawers(tester: &WidgetTester) -> usize {
        tester.find_by_type::<Drawer>().len()
    }

    fn drawer_box(tester: &WidgetTester) -> AnyRenderBox {
        let drawer = *tester.find_by_type::<Drawer>().first().expect("a drawer");
        tester.render_box_of(drawer)
    }

    fn drawer_width(tester: &WidgetTester) -> f64 {
        drawer_box(tester).size(&tester.cell.borrow()).width()
    }

    /// `find.descendant(of: find.byType(Drawer), matching: find.byType(Material))`, first.
    fn drawer_material(tester: &WidgetTester) -> AnyElement {
        let drawer = *tester.find_by_type::<Drawer>().first().expect("a drawer");
        *tester
            .find_by_type::<Material>()
            .iter()
            .find(|material| tester.ancestors(**material).contains(&drawer))
            .expect("a material in the drawer")
    }

    fn material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        tester.widget(drawer_material(tester), read)
    }

    fn material_shape(tester: &WidgetTester) -> Option<Box<dyn ShapeBorder>> {
        material(tester, |material| material.shape.clone())
    }

    /// The scrim: the `ColoredBox` of a [`DrawerController`] outside its drawer, which Dart
    /// finds by its `Semantics` label.
    fn scrim_color(tester: &WidgetTester) -> Color {
        let controllers = tester.find_by_type::<DrawerController>();
        let drawers = tester.find_by_type::<Drawer>();
        let scrim = *tester
            .find_by_type::<ColoredBox>()
            .iter()
            .find(|colored_box| {
                let ancestors = tester.ancestors(**colored_box);
                controllers
                    .iter()
                    .any(|controller| ancestors.contains(controller))
                    && !drawers.iter().any(|drawer| ancestors.contains(drawer))
            })
            .expect("a scrim");
        tester.widget(scrim, |colored_box: &ColoredBox| colored_box.color.color())
    }

    fn top_left(tester: &WidgetTester, element: AnyElement) -> Offset {
        tester.global_rect(tester.render_box_of(element)).top_left()
    }

    fn text_rect(tester: &WidgetTester, text: &str) -> Rect {
        let text = *tester.find_text(text).first().expect("the text");
        tester.global_rect(tester.render_box_of(text))
    }

    fn rounded(border_radius: BorderRadius) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(BorderSide::NONE, border_radius.into())
    }

    fn assert_shape(tester: &WidgetTester, expected: Option<RoundedRectangleBorder>) {
        let shape = material_shape(tester);
        assert_eq!(
            shape.as_deref(),
            expected.as_ref().map(|shape| shape as &dyn ShapeBorder)
        );
    }

    /// `Scaffold(key: key, drawer: Text(drawer), body: Container())`.
    fn text_drawer_scaffold(key: &GlobalKey, drawer: &str) -> Scaffold {
        Scaffold::new()
            .key(key.clone().into())
            .drawer(Text::new(drawer))
            .body(Container::new())
    }

    // drawer_test.dart 'Material2 - Drawer control test' and 'Material3 - Drawer control test'.
    fn drawer_control_test(theme: fn(&App) -> ThemeData, header_divider: f64) {
        let mut tester = tester();
        let key = GlobalKey::new();
        let container_key = GlobalKey::new();
        let saved_context = Rc::new(Cell::new(None));
        let (scaffold, container, saved) = (
            key.clone(),
            container_key.clone(),
            Rc::clone(&saved_context),
        );
        pump_app(&mut tester, theme, move |_app| {
            Builder::new(move |_app, context| {
                saved.set(Some(context));

                Scaffold::new()
                    .key(scaffold.clone().into())
                    .drawer(
                        Drawer::new().child(
                            ListView::new().children([
                                DrawerHeader::new(Some(
                                    Container::new()
                                        .key(container.clone().into())
                                        .child(Text::new("header"))
                                        .into_widget(),
                                ))
                                .into_widget(),
                                ListTile::new()
                                    .leading(waft_widgets::Icon::new(Some(Icons::archive())))
                                    .title(Text::new("Archive"))
                                    .into_widget(),
                            ]),
                        ),
                    )
                    .into_widget()
            })
            .into_widget()
        });

        assert_eq!(found(&tester, "Archive"), 0);
        open_drawer(&tester, &key);

        tester.pump(Duration::ZERO);
        assert_eq!(found(&tester, "Archive"), 1);
        tester.pump(Duration::from_secs(1));
        assert_eq!(found(&tester, "Archive"), 1);

        let header = *tester
            .find_by_type::<DrawerHeader>()
            .first()
            .expect("a header");
        let header_size = tester.render_box_of(header).size(&tester.cell.borrow());
        assert_eq!(header_size.height(), 160.0 + 8.0 + 1.0); // height + bottom margin + bottom edge

        let container_size = tester
            .render_object(&container_key)
            .as_box()
            .expect("a render box")
            .size(&tester.cell.borrow());
        assert_eq!(container_size.width(), header_size.width() - 2.0 * 16.0);
        assert_eq!(
            container_size.height(),
            header_size.height() - 2.0 * 16.0 - header_divider
        );

        assert_eq!(found(&tester, "header"), 1);

        {
            let context = saved_context.get().expect("the builder built");
            let mut app = tester.cell.borrow_mut();
            waft_widgets::Navigator::pop(&mut app, context, None);
        }
        tester.pump(Duration::ZERO); // drawer should be starting to animate away
        assert_eq!(found(&tester, "Archive"), 1);
        tester.pump(Duration::from_secs(1)); // animation done
        assert_eq!(found(&tester, "Archive"), 0);
    }

    #[test]
    fn material2_drawer_control_test() {
        with_main_thread_stack(|| drawer_control_test(m2, 0.0));
    }

    #[test]
    fn material3_drawer_control_test() {
        // Header divider thickness is 1.0 in Material 3.
        with_main_thread_stack(|| drawer_control_test(m3, 1.0));
    }

    #[test]
    fn drawer_tap_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                text_drawer_scaffold(&key, "drawer").into_widget()
            });
            tester.pump(Duration::ZERO); // no effect
            assert_eq!(found(&tester, "drawer"), 0);
            open_drawer(&tester, &key);
            tester.pump(Duration::ZERO); // drawer should be starting to animate in
            assert_eq!(found(&tester, "drawer"), 1);
            tester.pump(Duration::from_secs(1)); // animation done
            assert_eq!(found(&tester, "drawer"), 1);
            let at = text_rect(&tester, "drawer").center();
            tester.tap(at, 1);
            tester.pump(Duration::ZERO); // nothing should have happened
            assert_eq!(found(&tester, "drawer"), 1);
            tester.pump(Duration::from_secs(1)); // ditto
            assert_eq!(found(&tester, "drawer"), 1);
            tester.tap(Offset::new(750.0, 100.0), 2); // on the mask
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(10));
            // drawer should be starting to animate away
            assert_eq!(found(&tester, "drawer"), 1);
            tester.pump(Duration::from_secs(1)); // animation done
            assert_eq!(found(&tester, "drawer"), 0);
        });
    }

    #[test]
    fn drawer_hover_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let logs = Rc::new(RefCell::new(Vec::<&'static str>::new()));
            let mut gesture = WidgetTester::mouse(1, 0);
            // Start out of hoverTarget
            tester.add_pointer(&mut gesture, Offset::new(100.0, 100.0));

            let log = Rc::clone(&logs);
            pump_app(&mut tester, m3, |_app| {
                let (enter, hover, exit) = (Rc::clone(&log), Rc::clone(&log), Rc::clone(&log));
                Scaffold::new()
                    .key(key.clone().into())
                    .drawer(Text::new("drawer"))
                    .body(
                        Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                            MouseRegion::new()
                                .on_enter(Rc::new(move |_app, _event| {
                                    enter.borrow_mut().push("enter");
                                }))
                                .on_hover(Rc::new(move |_app, _event| {
                                    hover.borrow_mut().push("hover");
                                }))
                                .on_exit(Rc::new(move |_app, _event| {
                                    exit.borrow_mut().push("exit");
                                }))
                                .child(SizedBox::new().width(10.0).height(10.0)),
                        ),
                    )
                    .into_widget()
            });
            assert!(logs.borrow().is_empty());
            assert_eq!(found(&tester, "drawer"), 0);

            // When drawer is closed, hover is interactable
            tester.move_to(&mut gesture, Offset::new(5.0, 5.0), false);
            tester.pump(Duration::ZERO); // no effect
            assert_eq!(*logs.borrow(), ["enter", "hover"]);
            logs.borrow_mut().clear();

            tester.move_to(&mut gesture, Offset::new(20.0, 20.0), false);
            tester.pump(Duration::ZERO); // no effect
            assert_eq!(*logs.borrow(), ["exit"]);
            logs.borrow_mut().clear();

            // When drawer is open, hover is uninteractable
            open_drawer(&tester, &key);
            tester.pump(Duration::from_secs(1)); // animation done
            assert_eq!(found(&tester, "drawer"), 1);

            tester.move_to(&mut gesture, Offset::new(5.0, 5.0), false);
            tester.pump(Duration::ZERO); // no effect
            assert!(logs.borrow().is_empty());
            logs.borrow_mut().clear();

            tester.move_to(&mut gesture, Offset::new(20.0, 20.0), false);
            tester.pump(Duration::ZERO); // no effect
            assert!(logs.borrow().is_empty());
            logs.borrow_mut().clear();

            // Close drawer, hover is interactable again
            tester.tap(Offset::new(750.0, 100.0), 2); // on the mask
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1)); // animation done
            assert_eq!(found(&tester, "drawer"), 0);

            tester.move_to(&mut gesture, Offset::new(5.0, 5.0), false);
            tester.pump(Duration::ZERO); // no effect
            assert_eq!(*logs.borrow(), ["enter", "hover"]);
            logs.borrow_mut().clear();

            tester.move_to(&mut gesture, Offset::new(20.0, 20.0), false);
            tester.pump(Duration::ZERO); // no effect
            assert_eq!(*logs.borrow(), ["exit"]);
            logs.borrow_mut().clear();
        });
    }

    #[test]
    fn scaffold_drawer_scrim_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let build_frame = |tester: &mut WidgetTester, drawer_scrim_color: Option<Color>| {
                pump_app(tester, m3, |_app| {
                    let mut scaffold =
                        Scaffold::new()
                            .key(key.clone().into())
                            .drawer(Drawer::new().child(Builder::new(|_app, context| {
                                GestureDetector::new()
                                    .on_tap(Listener::new(move |app| {
                                        waft_widgets::Navigator::pop(app, context, None);
                                    })) // close drawer
                                    .into_widget()
                            })));
                    if let Some(drawer_scrim_color) = drawer_scrim_color {
                        scaffold = scaffold.drawer_scrim_color(drawer_scrim_color);
                    }
                    scaffold.into_widget()
                });
            };

            let check_scrim = |tester: &mut WidgetTester, color: Color| {
                open_drawer(tester, &key);
                tester.pump(Duration::ZERO);
                assert!(is_same_color_as(
                    scrim_color(tester),
                    color.with_values(Some(0.0), None, None, None, None)
                ));

                tester.pump_and_settle();
                assert!(is_same_color_as(scrim_color(tester), color));

                let at = tester.global_rect(drawer_box(tester)).center();
                tester.tap(at, 1);
                tester.pump_and_settle();
                assert_eq!(drawers(tester), 0);
            };

            // Default drawerScrimColor
            build_frame(&mut tester, None);
            check_scrim(&mut tester, Colors::BLACK54);

            // Specific drawerScrimColor
            build_frame(&mut tester, Some(Color::new(0xFF323232)));
            check_scrim(&mut tester, Color::new(0xFF323232));
        });
    }

    #[test]
    fn open_close_drawers_by_flinging() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                Scaffold::new()
                    .key(key.clone().into())
                    .drawer(Drawer::new().child(Text::new("start drawer")))
                    .end_drawer(Drawer::new().child(Text::new("end drawer")))
                    .into_widget()
            });

            // In the beginning, drawers are closed
            assert!(!is_drawer_open(&tester, &key));
            assert!(!is_end_drawer_open(&tester, &key));
            let size = WidgetTester::SIZE;

            // A fling from the left opens the start drawer
            tester.fling_from(
                Offset::new(0.0, size.height() / 2.0),
                Offset::new(80.0, 0.0),
                500.0,
                1,
            );
            tester.pump_and_settle();
            assert!(is_drawer_open(&tester, &key));
            assert!(!is_end_drawer_open(&tester, &key));

            // Now, a fling from the right closes the drawer
            tester.fling_from(
                Offset::new(size.width() - 1.0, size.height() / 2.0),
                Offset::new(-80.0, 0.0),
                500.0,
                2,
            );
            tester.pump_and_settle();
            assert!(!is_drawer_open(&tester, &key));
            assert!(!is_end_drawer_open(&tester, &key));

            // Another fling from the right opens the end drawer
            tester.fling_from(
                Offset::new(size.width() - 1.0, size.height() / 2.0),
                Offset::new(-80.0, 0.0),
                500.0,
                3,
            );
            tester.pump_and_settle();
            assert!(!is_drawer_open(&tester, &key));
            assert!(is_end_drawer_open(&tester, &key));

            // And a fling from the left closes it
            tester.fling_from(
                Offset::new(0.0, size.height() / 2.0),
                Offset::new(80.0, 0.0),
                500.0,
                4,
            );
            tester.pump_and_settle();
            assert!(!is_drawer_open(&tester, &key));
            assert!(!is_end_drawer_open(&tester, &key));
        });
    }

    #[test]
    fn open_close_drawer_by_dragging() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .platform(TargetPlatform::Android)
                        .build(app)
                },
                |_app| Scaffold::new().drawer(Drawer::new()).into_widget(),
            );

            let mut gesture = WidgetTester::touch(1);
            let drawer_position = |tester: &WidgetTester| {
                assert_eq!(drawers(tester), 1);
                tester.global_rect(drawer_box(tester)).left
            };
            // `gesture.moveBy(by)`.
            let move_by = |tester: &mut WidgetTester, gesture: &mut TestPointer, by: Offset| {
                let at = gesture.position + by;
                tester.move_to(gesture, at, true);
            };

            // Pointer down (drawer is closed).
            tester.down(&mut gesture, Offset::new(2.0, 2.0));
            tester.pump(Duration::ZERO);
            assert_eq!(drawers(&tester), 0);

            // Open drawer slightly.
            move_by(&mut tester, &mut gesture, Offset::new(20.0, 0.0));
            tester.pump(Duration::ZERO);
            assert!(drawer_position(&tester) < 0.0);

            // Open drawer more than halfway.
            move_by(&mut tester, &mut gesture, Offset::new(200.0, 0.0));
            tester.pump(Duration::ZERO);
            assert!(drawer_position(&tester) < 0.0);

            // Drawer is fully open.
            move_by(&mut tester, &mut gesture, Offset::new(200.0, 0.0));
            tester.pump(Duration::ZERO);
            assert_eq!(drawer_position(&tester), 0.0);

            // Drawer is less than halfway closed.
            move_by(&mut tester, &mut gesture, Offset::new(-100.0, 0.0));
            tester.pump(Duration::ZERO);
            assert!((drawer_position(&tester) - -100.0).abs() < 1e-10);

            // Drawer is more than halfway closed.
            move_by(&mut tester, &mut gesture, Offset::new(-100.0, 0.0));
            tester.pump(Duration::ZERO);
            assert!((drawer_position(&tester) - -200.0).abs() < 1e-10);

            // Drawer is completely closed.
            tester.move_to(&mut gesture, Offset::ZERO, true);
            tester.pump(Duration::ZERO);
            assert_eq!(drawers(&tester), 0);
        });
    }

    #[test]
    fn close_drawer_with_navigator_pop() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let button_pressed = Rc::new(Cell::new(false));
            let (scaffold, pressed) = (key.clone(), Rc::clone(&button_pressed));
            pump_app(&mut tester, m3, move |_app| {
                Builder::new(move |_app, context| {
                    let pressed = Rc::clone(&pressed);
                    Scaffold::new()
                        .key(scaffold.clone().into())
                        .drawer(
                            Drawer::new().child(
                                ListView::new().children([
                                    Text::new("drawer").into_widget(),
                                    TextButton::new(
                                        Some(Listener::new(move |app| {
                                            waft_widgets::Navigator::pop(app, context, None);
                                        })),
                                        Some(Text::new("close")),
                                    )
                                    .into_widget(),
                                ]),
                            ),
                        )
                        .body(TextButton::new(
                            Some(Listener::new(move |_app| pressed.set(true))),
                            Some(Text::new("button")),
                        ))
                        .into_widget()
                })
                .into_widget()
            });

            // Open the drawer.
            open_drawer(&tester, &key);
            tester.pump(Duration::ZERO); // drawer should be starting to animate in
            assert_eq!(found(&tester, "drawer"), 1);

            // Tap the close button to pop the drawer route.
            tester.pump(Duration::from_millis(100));
            let at = text_rect(&tester, "close").center();
            tester.tap(at, 1);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));
            assert_eq!(found(&tester, "drawer"), 0);

            // Confirm that a button in the scaffold body is still clickable.
            let at = text_rect(&tester, "button").center();
            tester.tap(at, 2);
            assert!(button_pressed.get());
        });
    }

    fn drawer_closing_can_be_canceled_by_gesture(text_direction: TextDirection) {
        let mut tester = tester();
        let key = GlobalKey::new();
        pump_app(&mut tester, m3, |_app| {
            Directionality::new(
                text_direction,
                Scaffold::new()
                    .drawer_drag_start_behavior(DragStartBehavior::Down)
                    .key(key.clone().into())
                    .drawer(
                        Drawer::new().child(
                            ListView::new().children([
                                Text::new("drawer").into_widget(),
                                Container::new()
                                    .height(1000.0)
                                    .color(Colors::BLUE.shade500())
                                    .into_widget(),
                            ]),
                        ),
                    )
                    .body(Container::new()),
            )
            .into_widget()
        });
        let ltr = text_direction == TextDirection::Ltr;
        // The edge of the text facing the drawer's edge.
        let text_edge = |tester: &WidgetTester| {
            let rect = text_rect(tester, "drawer");
            if ltr { rect.left } else { rect.right }
        };
        assert_eq!(found(&tester, "drawer"), 0);
        open_drawer(&tester, &key);
        tester.pump(Duration::ZERO); // drawer should be starting to animate in
        assert_eq!(found(&tester, "drawer"), 1);
        tester.pump(Duration::from_secs(1)); // animation done
        assert_eq!(found(&tester, "drawer"), 1);

        tester.tap(Offset::new(if ltr { 750.0 } else { 50.0 }, 100.0), 1); // on the mask
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_millis(10));
        // drawer should be starting to animate away
        let text_start = text_edge(&tester);
        if ltr {
            assert!(text_start < 0.0);
        } else {
            assert!(text_start > 800.0);
        }

        let mut gesture = WidgetTester::touch(2);
        tester.down(
            &mut gesture,
            Offset::new(if ltr { 100.0 } else { 700.0 }, 100.0),
        );
        // drawer should be stopped.
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_millis(10));
        assert_eq!(text_edge(&tester), text_start);

        let at = gesture.position + Offset::new(if ltr { 50.0 } else { -50.0 }, 0.0);
        tester.move_to(&mut gesture, at, true);
        // drawer should be returning to visible
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_secs(1));
        assert_eq!(text_edge(&tester), if ltr { 0.0 } else { 800.0 });

        tester.up(&gesture);
    }

    #[test]
    fn drawer_closing_can_be_canceled_by_gesture_ltr() {
        with_main_thread_stack(|| drawer_closing_can_be_canceled_by_gesture(TextDirection::Ltr));
    }

    #[test]
    fn drawer_closing_can_be_canceled_by_gesture_rtl() {
        with_main_thread_stack(|| drawer_closing_can_be_canceled_by_gesture(TextDirection::Rtl));
    }

    /// Dart's `tester.pumpWidget(MaterialApp(restorationScopeId: 'app', home: scaffold()))` on
    /// a tester whose host restores `data`: a new one for each `restartAndRestore` and
    /// `restoreFrom`.
    fn restored(
        data: Option<waft_embedder::RestorationMap>,
        scaffold: &dyn Fn() -> Scaffold,
    ) -> WidgetTester {
        let mut tester = WidgetTester::restoring(data);
        tester.install_fonts();
        let theme = m3(&tester.cell.borrow());
        tester.pump_widget(restorable_material_app(
            "app",
            theme,
            scaffold().into_widget(),
        ));
        tester
    }

    /// drawer_test.dart's null-restorationId cases: the drawer of `text_drawer_scaffold(key,
    /// text)` opened, then a restart.
    fn null_restoration_id(text: &'static str) {
        let key = GlobalKey::new();
        let scaffold = || text_drawer_scaffold(&key, text);
        let mut tester = restored(None, &scaffold);
        tester.pump(Duration::ZERO); // no effect
        assert_eq!(found(&tester, text), 0);
        open_drawer(&tester, &key);
        tester.pump_and_settle();
        assert_eq!(found(&tester, text), 1);

        let tester = restored(tester.restoration_data(), &scaffold);
        // Drawer state should not have been saved.
        assert_eq!(found(&tester, text), 0);
    }

    #[test]
    fn scaffold_drawer_null_restoration_id() {
        with_main_thread_stack(|| null_restoration_id("drawer"));
    }

    #[test]
    fn scaffold_end_drawer_null_restoration_id() {
        // Dart's case gives its text as the start drawer.
        with_main_thread_stack(|| null_restoration_id("endDrawer"));
    }

    #[test]
    fn scaffold_drawer_state_restoration_test() {
        with_main_thread_stack(|| {
            let key = GlobalKey::new();
            let scaffold = || {
                Scaffold::new()
                    .key(key.clone().into())
                    .restoration_id("scaffold")
                    .drawer(Text::new("drawer"))
                    .body(Container::new())
            };
            let mut tester = restored(None, &scaffold);
            tester.pump(Duration::ZERO); // no effect
            assert_eq!(found(&tester, "drawer"), 0);
            open_drawer(&tester, &key);
            tester.pump_and_settle();
            assert_eq!(found(&tester, "drawer"), 1);

            let mut tester = restored(tester.restoration_data(), &scaffold);
            assert_eq!(found(&tester, "drawer"), 1);

            let data = tester.restoration_data();
            tester.tap(Offset::new(750.0, 100.0), 1); // on the mask
            tester.pump_and_settle();
            assert_eq!(found(&tester, "drawer"), 0);

            let tester = restored(data, &scaffold);
            assert_eq!(found(&tester, "drawer"), 1);
        });
    }

    #[test]
    fn scaffold_end_drawer_state_restoration_test() {
        with_main_thread_stack(|| {
            let key = GlobalKey::new();
            let scaffold = || {
                Scaffold::new()
                    .key(key.clone().into())
                    .restoration_id("scaffold")
                    .end_drawer(Text::new("endDrawer"))
                    .body(Container::new())
            };
            let mut tester = restored(None, &scaffold);
            tester.pump(Duration::ZERO); // no effect
            assert_eq!(found(&tester, "endDrawer"), 0);
            open_end_drawer(&tester, &key);
            tester.pump_and_settle();
            assert_eq!(found(&tester, "endDrawer"), 1);

            let mut tester = restored(tester.restoration_data(), &scaffold);
            assert_eq!(found(&tester, "endDrawer"), 1);

            let data = tester.restoration_data();
            tester.tap(Offset::new(750.0, 100.0), 1); // on the mask
            tester.pump_and_settle();
            assert_eq!(found(&tester, "endDrawer"), 0);

            let tester = restored(data, &scaffold);
            assert_eq!(found(&tester, "endDrawer"), 1);
        });
    }

    #[test]
    fn both_drawer_and_end_drawer_state_restoration_test() {
        with_main_thread_stack(|| {
            let key = GlobalKey::new();
            let scaffold = || {
                Scaffold::new()
                    .restoration_id("scaffold")
                    .key(key.clone().into())
                    .drawer(Text::new("drawer"))
                    .end_drawer(Text::new("endDrawer"))
                    .body(Container::new())
            };
            let shown =
                |tester: &WidgetTester| (found(tester, "drawer"), found(tester, "endDrawer"));
            let mut tester = restored(None, &scaffold);
            tester.pump(Duration::ZERO); // no effect
            assert_eq!(shown(&tester), (0, 0));
            open_drawer(&tester, &key);
            tester.pump_and_settle();
            assert_eq!(shown(&tester), (1, 0));

            let mut tester = restored(tester.restoration_data(), &scaffold);
            assert_eq!(shown(&tester), (1, 0));

            let data = tester.restoration_data();
            tester.tap(Offset::new(750.0, 100.0), 1); // on the mask
            tester.pump_and_settle();
            assert_eq!(shown(&tester), (0, 0));

            let mut tester = restored(data, &scaffold);
            assert_eq!(shown(&tester), (1, 0));

            tester.tap(Offset::new(750.0, 100.0), 1); // on the mask
            tester.pump_and_settle();
            assert_eq!(shown(&tester), (0, 0));

            open_end_drawer(&tester, &key);
            tester.pump_and_settle();
            assert_eq!(shown(&tester), (0, 1));

            let mut tester = restored(tester.restoration_data(), &scaffold);
            assert_eq!(shown(&tester), (0, 1));

            let data = tester.restoration_data();
            tester.tap(Offset::new(750.0, 100.0), 1); // on the mask
            tester.pump_and_settle();
            assert_eq!(shown(&tester), (0, 0));

            let tester = restored(data, &scaffold);
            assert_eq!(shown(&tester), (0, 1));
        });
    }

    #[test]
    fn scaffold_state_close_drawer() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                text_drawer_scaffold(&key, "Drawer").into_widget()
            });

            assert_eq!(found(&tester, "Drawer"), 0);

            open_drawer(&tester, &key);
            tester.pump_and_settle();
            assert_eq!(found(&tester, "Drawer"), 1);

            close_drawer(&tester, &key);
            tester.pump_and_settle();
            assert_eq!(found(&tester, "Drawer"), 0);
        });
    }

    #[test]
    fn scaffold_state_close_drawer_do_not_crash_if_drawer_is_already_closed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                text_drawer_scaffold(&key, "Drawer").into_widget()
            });

            assert_eq!(found(&tester, "Drawer"), 0);

            close_drawer(&tester, &key);
            tester.pump_and_settle();
            assert_eq!(found(&tester, "Drawer"), 0);
        });
    }

    #[test]
    fn scaffold_state_close_end_drawer() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                Scaffold::new()
                    .key(key.clone().into())
                    .end_drawer(Text::new("endDrawer"))
                    .body(Container::new())
                    .into_widget()
            });

            assert_eq!(found(&tester, "endDrawer"), 0);

            open_end_drawer(&tester, &key);
            tester.pump_and_settle();
            assert_eq!(found(&tester, "endDrawer"), 1);

            close_end_drawer(&tester, &key);
            tester.pump_and_settle();
            assert_eq!(found(&tester, "endDrawer"), 0);
        });
    }

    #[test]
    fn drawer_width_defaults_to_material_spec() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                Scaffold::new()
                    .key(key.clone().into())
                    .drawer(Drawer::new())
                    .into_widget()
            });

            open_drawer(&tester, &key);

            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));

            assert_eq!(drawer_width(&tester), 304.0);
        });
    }

    #[test]
    fn drawer_width_can_be_customized_by_parameter() {
        with_main_thread_stack(|| {
            const SMALL_WIDTH: f64 = 200.0;

            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, m3, |_app| {
                Scaffold::new()
                    .key(key.clone().into())
                    .drawer(Drawer::new().width(SMALL_WIDTH))
                    .into_widget()
            });

            open_drawer(&tester, &key);

            tester.pump_and_settle();

            assert_eq!(drawer_width(&tester), SMALL_WIDTH);
        });
    }

    /// drawer_test.dart's default shape cases: `Scaffold(drawer: Drawer(), endDrawer:
    /// Drawer())`, under `text_direction` where the case gives one; the start drawer opened,
    /// a tap at (750, 300), then the end drawer opened.
    fn drawer_default_shape(
        theme: fn(&App) -> ThemeData,
        text_direction: Option<TextDirection>,
        start_shape: Option<RoundedRectangleBorder>,
        end_shape: Option<RoundedRectangleBorder>,
    ) {
        let mut tester = tester();
        let key = GlobalKey::new();
        pump_app(&mut tester, theme, |_app| {
            let scaffold = Scaffold::new()
                .key(key.clone().into())
                .drawer(Drawer::new())
                .end_drawer(Drawer::new());
            match text_direction {
                Some(text_direction) => Directionality::new(text_direction, scaffold).into_widget(),
                None => scaffold.into_widget(),
            }
        });

        // Open the drawer.
        open_drawer(&tester, &key);
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_secs(1));

        // Test the drawer shape.
        assert_shape(&tester, start_shape);

        // Close the opened drawer.
        tester.tap(Offset::new(750.0, 300.0), 1);
        tester.pump_and_settle();

        // Open the end drawer.
        open_end_drawer(&tester, &key);
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_secs(1));

        // Test the end drawer shape.
        assert_shape(&tester, end_shape);
    }

    fn right_rounded() -> RoundedRectangleBorder {
        rounded(BorderRadius::only(
            Radius::ZERO,
            Radius::circular(16.0),
            Radius::ZERO,
            Radius::circular(16.0),
        ))
    }

    fn left_rounded() -> RoundedRectangleBorder {
        rounded(BorderRadius::only(
            Radius::circular(16.0),
            Radius::ZERO,
            Radius::circular(16.0),
            Radius::ZERO,
        ))
    }

    #[test]
    fn material3_drawer_default_shape_ltr() {
        with_main_thread_stack(|| {
            drawer_default_shape(
                m3,
                Some(TextDirection::Ltr),
                Some(right_rounded()),
                Some(left_rounded()),
            );
        });
    }

    #[test]
    fn material3_drawer_default_shape_rtl() {
        with_main_thread_stack(|| {
            drawer_default_shape(
                m3,
                Some(TextDirection::Rtl),
                Some(left_rounded()),
                Some(right_rounded()),
            );
        });
    }

    #[test]
    fn material2_drawer_default_shape() {
        with_main_thread_stack(|| drawer_default_shape(m2, None, None, None));
    }

    /// drawer_test.dart's clip behavior cases: the default, then the drawer `second` makes.
    fn drawer_clip_behavior(
        theme: fn(&App) -> ThemeData,
        default_clip: Clip,
        second: fn() -> Drawer,
        second_clip: Clip,
    ) {
        let mut tester = tester();
        let key = GlobalKey::new();
        pump_app(&mut tester, theme, |_app| {
            Scaffold::new()
                .key(key.clone().into())
                .drawer(Drawer::new())
                .into_widget()
        });

        // Open the drawer.
        open_drawer(&tester, &key);
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_secs(1));

        // Test default clip behavior.
        assert_eq!(
            material(&tester, |material| material.clip_behavior),
            default_clip
        );

        close_drawer(&tester, &key);
        tester.pump_and_settle();

        // Provide a custom clip behavior.
        pump_app(&mut tester, theme, |_app| {
            Scaffold::new()
                .key(key.clone().into())
                .drawer(second())
                .into_widget()
        });

        // Open the drawer again.
        open_drawer(&tester, &key);
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_secs(1));

        // Clip behavior is now updated.
        assert_eq!(
            material(&tester, |material| material.clip_behavior),
            second_clip
        );
    }

    #[test]
    fn material3_drawer_clip_behavior() {
        with_main_thread_stack(|| {
            drawer_clip_behavior(
                m3,
                Clip::HardEdge,
                || Drawer::new().clip_behavior(Clip::AntiAlias),
                Clip::AntiAlias,
            );
        });
    }

    #[test]
    fn material2_drawer_clip_behavior() {
        with_main_thread_stack(|| {
            drawer_clip_behavior(
                m2,
                Clip::None,
                || {
                    Drawer::new()
                        .clip_behavior(Clip::HardEdge)
                        .shape(RoundedRectangleBorder::default())
                },
                Clip::HardEdge,
            );
        });
    }

    /// drawer_test.dart's barrier cases: an end drawer opened, closed, opened again, and a
    /// tap on the route's barrier at (400, 300).
    fn drawer_barrier(drawer_barrier_dismissible: bool) {
        let mut tester = tester();
        let key = GlobalKey::new();
        pump_app(&mut tester, m2, |_app| {
            Scaffold::new()
                .key(key.clone().into())
                .drawer_barrier_dismissible(drawer_barrier_dismissible)
                .app_bar(PreferredSizeWidgetRef::new(
                    AppBar::new().title(Text::new("Drawer Dismissible")),
                ))
                .end_drawer(
                    Drawer::new()
                        .background_color(Colors::WHITE)
                        .width(300.0)
                        .child(Text::new("Drawer")),
                )
                .body(
                    Container::new()
                        .color(Colors::WHITE)
                        .width(600.0)
                        .height(600.0)
                        .child(Center::new().child(Text::new("Drawer Dismissible"))),
                )
                .into_widget()
        });

        // Check the flag is set at the Scaffold level.
        let scaffold = tester.find_by_type::<Scaffold>()[0];
        assert_eq!(
            tester.widget(scaffold, |scaffold: &Scaffold| scaffold
                .drawer_barrier_dismissible),
            drawer_barrier_dismissible
        );

        // Verify whether the drawer barrier is dismissible via the state
        let state = scaffold_state(&tester, &key);
        assert_eq!(
            state.is_drawer_barrier_dismissible(&tester.cell.borrow()),
            drawer_barrier_dismissible
        );

        // Open the drawer initially.
        open_end_drawer(&tester, &key);

        tester.pump_and_settle();

        // Check that the drawer open.
        assert_eq!(drawers(&tester), 1);

        // Close the drawer programmatically.
        close_end_drawer(&tester, &key);
        tester.pump_and_settle();

        assert_eq!(drawers(&tester), 0);

        // Open it again, and make sure the drawer is available.
        open_end_drawer(&tester, &key);
        tester.pump_and_settle();

        assert_eq!(drawers(&tester), 1);

        // Find the ModalBarrier.
        let modal_barriers = tester.find_by_type::<ModalBarrier>();
        assert_eq!(modal_barriers.len(), 1);

        // Calculate a point to tap outside the Drawer.
        // This example taps on the ModalBarrier somewhere outside its boundaries.
        let modal_barrier_center = Offset::new(400.0, 300.0);
        let tap_position = top_left(&tester, modal_barriers[0]) + modal_barrier_center;

        // Tap on the ModalBarrier.
        tester.tap(tap_position, 1);
        tester.pump_and_settle();

        assert_eq!(
            drawers(&tester),
            if drawer_barrier_dismissible { 0 } else { 1 }
        );
    }

    #[test]
    fn drawer_barrier_is_dismissible_by_default() {
        with_main_thread_stack(|| {
            assert!(Scaffold::new().drawer_barrier_dismissible);
            // Make sure the drawer is gone, since the drawerBarrierDismissible flag is set to
            // true by default.
            drawer_barrier(true);
        });
    }

    #[test]
    fn drawer_can_be_configured_as_not_dismissible() {
        with_main_thread_stack(|| {
            // Make sure the drawer is still present, and that tapping on the modal barrier
            // didn't dismiss it, since the drawerBarrierDismissible property is set to false.
            drawer_barrier(false);
        });
    }

    /// drawer_test.dart's escape key cases: the drawer opened, and the escape key sent.
    fn escape_key(drawer_barrier_dismissible: Option<bool>) -> usize {
        let mut tester = tester();
        let key = GlobalKey::new();
        pump_app(&mut tester, m3, |_app| {
            let mut scaffold = Scaffold::new()
                .key(key.clone().into())
                .drawer(Drawer::new().child(Text::new("drawer")));
            if let Some(drawer_barrier_dismissible) = drawer_barrier_dismissible {
                scaffold = scaffold.drawer_barrier_dismissible(drawer_barrier_dismissible);
            }
            scaffold.into_widget()
        });

        let state = scaffold_state(&tester, &key);
        assert_eq!(
            state.is_drawer_barrier_dismissible(&tester.cell.borrow()),
            drawer_barrier_dismissible.unwrap_or(true)
        );

        // Open the drawer.
        open_drawer(&tester, &key);
        tester.pump_and_settle();
        assert_eq!(found(&tester, "drawer"), 1);

        tester.send_key_event(
            waft_services::PhysicalKeyboardKey::ESCAPE,
            waft_services::LogicalKeyboardKey::ESCAPE,
        );
        tester.pump_and_settle();
        found(&tester, "drawer")
    }

    #[test]
    fn drawer_can_be_dismissed_with_the_escape_key_by_default() {
        // Close the drawer with the escape key.
        with_main_thread_stack(|| assert_eq!(escape_key(None), 0));
    }

    #[test]
    fn drawer_cannot_be_dismissed_with_the_escape_key_when_drawer_barrier_dismissible_is_false() {
        // Try to close the drawer with the escape key, and verify it does not close.
        with_main_thread_stack(|| assert_eq!(escape_key(Some(false)), 1));
    }

    // Regression test for https://github.com/flutter/flutter/issues/6537
    #[test]
    fn drawer_and_drawer_header_do_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, move |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(
                        Drawer::new().child(DrawerHeader::new(Some(Text::new("X").into_widget()))),
                    ))
                    .into_widget()
            });
            assert_eq!(drawer_box(&tester).size(&tester.cell.borrow()), Size::ZERO);
            let header = *tester
                .find_by_type::<DrawerHeader>()
                .first()
                .expect("a header");
            assert_eq!(
                tester.render_box_of(header).size(&tester.cell.borrow()),
                Size::ZERO
            );
        });
    }

    fn will_handle_pop_internally(tester: &WidgetTester, text: &str) -> bool {
        let element = *tester.find_text(text).first().expect("the text");
        let mut app = tester.cell.borrow_mut();
        let route = AnyModalRoute::of(&mut app, element).expect("a modal route");
        route.as_route().will_handle_pop_internally(&app)
    }

    /// Dart's `Scaffold.of(tester.element(find.text(text)))`.
    fn scaffold_of(tester: &WidgetTester, text: &str) -> Handle<ScaffoldState> {
        let element = *tester.find_text(text).first().expect("the text");
        Scaffold::of(&mut tester.cell.borrow_mut(), element)
    }

    /// The pushed `MaterialPageRoute`'s page of drawer_test.dart's leak cases, as the home
    /// page: `MaterialPageRoute` is not ported.
    fn second_page(drawer: impl FnOnce(Scaffold, Drawer) -> Scaffold) -> WidgetRef {
        drawer(
            Scaffold::new()
                .app_bar(PreferredSizeWidgetRef::new(
                    AppBar::new().title(Text::new("Second")),
                ))
                .body(Center::new().child(Text::new("Second Body"))),
            Drawer::new().child(Text::new("Drawer Item")),
        )
        .into_widget()
    }

    fn ios(app: &App) -> ThemeData {
        ThemeData::builder()
            .platform(TargetPlatform::IOS)
            .build(app)
    }

    // drawer_test.dart 'Drawer double tap to close does not leak LocalHistoryEntry', up to
    // its swipe back.
    #[test]
    fn drawer_double_tap_to_close_does_not_leak_local_history_entry() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, ios, |_app| {
                second_page(|scaffold, drawer| scaffold.end_drawer(drawer))
            });
            tester.pump_and_settle();

            // Open the end drawer.
            let scaffold = scaffold_of(&tester, "Second Body");
            scaffold.open_end_drawer(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();

            // Rapid double tap on the scrim.
            // Use manual pointer events to simulate tapping the scrim twice rapidly.
            tester.tap(Offset::new(10.0, 200.0), 1);

            // Pump once to start the closing animation ticker.
            tester.pump(Duration::ZERO);
            // Advance time by 150ms so the drawer is more than half closed.
            tester.pump(Duration::from_millis(150));

            tester.tap(Offset::new(10.0, 200.0), 2);

            tester.pump_and_settle();

            // The drawer should be closed.
            assert_eq!(found(&tester, "Drawer Item"), 0);

            // The history entry should have been removed, meaning it won't handle pop internally.
            assert!(!will_handle_pop_internally(&tester, "Second Body"));
        });
    }

    // drawer_test.dart 'Drawer drag while closing does not leak LocalHistoryEntry'.
    #[test]
    fn drawer_drag_while_closing_does_not_leak_local_history_entry() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, ios, |_app| {
                second_page(|scaffold, drawer| scaffold.drawer(drawer))
            });
            tester.pump_and_settle();

            // Open the drawer.
            let scaffold = scaffold_of(&tester, "Second Body");
            scaffold.open_drawer(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();

            assert!(will_handle_pop_internally(&tester, "Second Body"));

            // Start closing the drawer programmatically to begin the closing animation.
            let scaffold = scaffold_of(&tester, "Second Body");
            scaffold.close_drawer(&mut tester.cell.borrow_mut());

            // Pump once to start the closing animation ticker.
            tester.pump(Duration::ZERO);
            // Advance time so the drawer is in the middle of closing.
            tester.pump(Duration::from_millis(100));

            // While it is closing (status == AnimationStatus.reverse), grab the drawer.
            // This triggers _handleDragDown, which adds the history entry back.
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, Offset::new(50.0, 200.0));
            tester.pump(Duration::ZERO);

            // The history entry should have been re-added.
            assert!(will_handle_pop_internally(&tester, "Second Body"));

            // Drag it closed again by dragging left and releasing.
            let at = gesture.position + Offset::new(-20.0, 0.0);
            tester.move_to(&mut gesture, at, true);
            tester.pump(Duration::ZERO);
            tester.up(&gesture);

            tester.pump(Duration::ZERO);

            assert!(!will_handle_pop_internally(&tester, "Second Body"));

            tester.pump_and_settle();

            assert_eq!(found(&tester, "Drawer Item"), 0);
        });
    }

    /// drawer_theme_test.dart's `_drawerMaterial`, `_scrim` and `_drawerRenderBox` readings.
    struct Seen {
        color: Option<Color>,
        elevation: f64,
        shadow_color: Option<Color>,
        surface_tint_color: Option<Color>,
        shape: Option<Box<dyn ShapeBorder>>,
        scrim_color: Color,
        width: f64,
        clip_behavior: Clip,
    }

    /// The drawer `drawer` makes, as the start drawer or the end drawer of the `Scaffold` keyed
    /// `key` that `home` builds, opened and settled, as the theme cases read it.
    fn opened_drawer(
        theme: impl FnOnce(&App) -> ThemeData,
        alignment: DrawerAlignment,
        drawer: impl Fn() -> Drawer + 'static,
        home: impl Fn(Scaffold) -> WidgetRef + 'static,
    ) -> Seen {
        let mut tester = tester();
        let key = GlobalKey::new();
        let scaffold_key = key.clone();
        pump_app(&mut tester, theme, move |_app| {
            let scaffold = Scaffold::new().key(scaffold_key.clone().into());
            home(match alignment {
                DrawerAlignment::Start => scaffold.drawer(drawer()),
                DrawerAlignment::End => scaffold.end_drawer(drawer()),
            })
        });
        match alignment {
            DrawerAlignment::Start => open_drawer(&tester, &key),
            DrawerAlignment::End => open_end_drawer(&tester, &key),
        }
        tester.pump_and_settle();
        let (color, elevation, shadow_color, surface_tint_color, clip_behavior) =
            material(&tester, |material| {
                (
                    material.color,
                    material.elevation,
                    material.shadow_color,
                    material.surface_tint_color,
                    material.clip_behavior,
                )
            });
        Seen {
            color,
            elevation,
            shadow_color,
            surface_tint_color,
            shape: material_shape(&tester),
            scrim_color: scrim_color(&tester),
            width: drawer_width(&tester),
            clip_behavior,
        }
    }

    fn theme_defaults(
        theme: fn(&App) -> ThemeData,
        alignment: DrawerAlignment,
        shape: Option<RoundedRectangleBorder>,
    ) {
        let seen = opened_drawer(theme, alignment, Drawer::new, IntoWidget::into_widget);
        let theme = {
            let tester = tester();
            theme(&tester.cell.borrow())
        };
        if theme.use_material3 {
            assert_eq!(seen.color, Some(theme.color_scheme.surface_container_low()));
            assert_eq!(seen.elevation, 1.0);
            assert_eq!(seen.shadow_color, Some(Colors::TRANSPARENT));
            assert_eq!(seen.surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(seen.clip_behavior, Clip::HardEdge);
        } else {
            assert_eq!(seen.color, None);
            assert_eq!(seen.elevation, 16.0);
            assert_eq!(seen.shadow_color, Some(theme.shadow_color));
            assert_eq!(seen.surface_tint_color, None);
            assert_eq!(seen.clip_behavior, Clip::None);
        }
        assert_eq!(
            seen.shape.as_deref(),
            shape.as_ref().map(|shape| shape as &dyn ShapeBorder)
        );
        assert_eq!(seen.scrim_color, Colors::BLACK54);
        assert_eq!(seen.width, 304.0);
    }

    #[test]
    fn material2_default_values_are_used_when_no_drawer_or_drawer_theme_data_properties_are_specified()
     {
        with_main_thread_stack(|| theme_defaults(m2, DrawerAlignment::Start, None));
    }

    #[test]
    fn material3_default_values_are_used_when_no_drawer_or_drawer_theme_data_properties_are_specified()
     {
        with_main_thread_stack(|| {
            theme_defaults(
                m3,
                DrawerAlignment::Start,
                Some(rounded(BorderRadius::horizontal(
                    Radius::ZERO,
                    Radius::circular(16.0),
                ))),
            );
        });
    }

    #[test]
    fn material2_default_values_are_used_when_no_drawer_or_drawer_theme_data_properties_are_specified_in_end_drawer()
     {
        with_main_thread_stack(|| theme_defaults(m2, DrawerAlignment::End, None));
    }

    #[test]
    fn material3_default_values_are_used_when_no_drawer_or_drawer_theme_data_properties_are_specified_in_end_drawer()
     {
        with_main_thread_stack(|| {
            theme_defaults(
                m3,
                DrawerAlignment::End,
                Some(rounded(BorderRadius::horizontal(
                    Radius::circular(16.0),
                    Radius::ZERO,
                ))),
            );
        });
    }

    const BACKGROUND_COLOR: Color = Color::new(0x00000001);
    const SCRIM_COLOR: Color = Color::new(0x00000002);
    const ELEVATION: f64 = 7.0;
    const SHADOW_COLOR: Color = Color::new(0x00000003);
    const SURFACE_TINT_COLOR: Color = Color::new(0x00000004);
    const WIDTH: f64 = 200.0;
    const CLIP_BEHAVIOR: Clip = Clip::AntiAlias;

    fn shape() -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(BorderSide::NONE, BorderRadiusGeometry::circular(16.0))
    }

    fn assert_given_values(seen: &Seen) {
        assert_eq!(seen.color, Some(BACKGROUND_COLOR));
        assert_eq!(seen.elevation, ELEVATION);
        assert_eq!(seen.shadow_color, Some(SHADOW_COLOR));
        assert_eq!(seen.surface_tint_color, Some(SURFACE_TINT_COLOR));
        assert_eq!(seen.shape.as_deref(), Some(&shape() as &dyn ShapeBorder));
        assert_eq!(seen.scrim_color, SCRIM_COLOR);
        assert_eq!(seen.width, WIDTH);
        assert_eq!(seen.clip_behavior, CLIP_BEHAVIOR);
    }

    fn given_theme() -> DrawerThemeData {
        DrawerThemeData::new()
            .background_color(BACKGROUND_COLOR)
            .scrim_color(SCRIM_COLOR)
            .elevation(ELEVATION)
            .shadow_color(SHADOW_COLOR)
            .surface_tint_color(SURFACE_TINT_COLOR)
            .shape(shape())
            .width(WIDTH)
            .clip_behavior(CLIP_BEHAVIOR)
    }

    #[test]
    fn drawer_theme_data_values_are_used_when_no_drawer_properties_are_specified() {
        with_main_thread_stack(|| {
            let seen = opened_drawer(
                |app| ThemeData::builder().drawer_theme(given_theme()).build(app),
                DrawerAlignment::Start,
                Drawer::new,
                IntoWidget::into_widget,
            );
            assert_given_values(&seen);
        });
    }

    fn other_theme() -> DrawerThemeData {
        DrawerThemeData::new()
            .background_color(Color::new(0x00000005))
            .scrim_color(Color::new(0x00000006))
            .elevation(13.0)
            .shape(RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::circular(29.0),
            ))
            .width(400.0)
            .clip_behavior(Clip::AntiAliasWithSaveLayer)
    }

    #[test]
    fn drawer_values_take_priority_over_drawer_theme_data_values_when_both_properties_are_specified()
     {
        with_main_thread_stack(|| {
            let seen = opened_drawer(
                |app| ThemeData::builder().drawer_theme(other_theme()).build(app),
                DrawerAlignment::Start,
                || {
                    Drawer::new()
                        .background_color(BACKGROUND_COLOR)
                        .elevation(ELEVATION)
                        .shadow_color(SHADOW_COLOR)
                        .surface_tint_color(SURFACE_TINT_COLOR)
                        .shape(shape())
                        .width(WIDTH)
                        .clip_behavior(CLIP_BEHAVIOR)
                },
                |scaffold| scaffold.drawer_scrim_color(SCRIM_COLOR).into_widget(),
            );
            assert_given_values(&seen);
        });
    }

    #[test]
    fn drawer_theme_values_take_priority_over_theme_data_drawer_theme_values_when_both_properties_are_specified()
     {
        with_main_thread_stack(|| {
            let seen = opened_drawer(
                |app| {
                    ThemeData::builder()
                        .drawer_theme(
                            other_theme()
                                .shadow_color(Color::new(0x00000007))
                                .surface_tint_color(Color::new(0x00000007)),
                        )
                        .build(app)
                },
                DrawerAlignment::Start,
                Drawer::new,
                |scaffold| DrawerTheme::new(given_theme(), scaffold).into_widget(),
            );
            assert_given_values(&seen);
        });
    }

    /// `DrawerController(isDrawerOpen:)`, which the scaffold sets from its restored state: the
    /// drawer shows at once, fully open, and a rebuild with false closes it at once.
    #[test]
    fn is_drawer_open_opens_the_drawer_at_once() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let pump = |tester: &mut WidgetTester, is_drawer_open: bool| {
                pump_app(tester, m3, move |_app| {
                    Material::new()
                        .child(
                            DrawerController::new(Text::new("drawer"), DrawerAlignment::Start)
                                .is_drawer_open(is_drawer_open),
                        )
                        .into_widget()
                });
            };
            pump(&mut tester, true);
            assert_eq!(found(&tester, "drawer"), 1);
            assert_eq!(text_rect(&tester, "drawer").left, 0.0);

            pump(&mut tester, false);
            assert_eq!(found(&tester, "drawer"), 0);
        });
    }
}
