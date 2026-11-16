//! Flutter counterpart: material_ui's `dropdown.dart`.
//!
//! A route's result is erased (waft-widgets' navigator), so the menu pops a
//! `DropdownRouteResult<T>` behind an `Rc<dyn Any>`, which the button downcasts. The parts of
//! Dart's `_DropdownRoute` that its layout delegate and painter read during layout and paint,
//! where no `App` is at hand, live in a [`DropdownRouteMenu`] the route shares with them.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;
use waft_widgets::{
    ActionRef, Actions, ActivateIntent, AnyFormFieldState, AutovalidateMode, BuildContext,
    ButtonActivateIntent, CallbackAction, CapturedThemes, ClipRRect, Column, ConstrainedBox,
    Container, CustomPaint, CustomSingleChildLayout, DefaultTextStyle, DirectionalFocusIntent,
    Directionality, Expanded, FadeTransition, Focus, FocusHighlightMode, FocusManager, FocusNode,
    FocusNodeLeaf, FocusNodeRef, FormFieldData, FormFieldLeaf, FormFieldState, FormFieldStateData,
    FormFieldStateLeaf, GestureDetector, Icon, IconTheme, IconThemeData, IgnorePointer,
    IndexedStack, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, LayoutBuilder, ListView,
    LocalHistoryRoute, LocalHistoryRouteData, MediaQuery, ModalRoute, ModalRouteData, Navigator,
    Orientation, OverlayRoute, OverlayRouteData, Padding, PopupRoute, Positioned,
    PredictiveBackRoute, PrimaryScrollController, RenderObjectWidget, RestorationMixin,
    RestorationMixinData, Route, RouteData, Row, ScrollConfiguration, ScrollController,
    ScrollControllerLeaf, ScrollControllerRef, ScrollPhysicsRef, Semantics, Shortcuts,
    SingleActivator, SingleChildRenderObjectWidget, SizedBox, Stack, State, StateData,
    StatefulWidget, StatelessWidget, Text, TransitionRoute, TransitionRouteData,
    TraversalDirection, View, WidgetRef, WidgetState, WidgetStateMouseCursor, WidgetStateProperty,
    WidgetStates,
};

use waft_animation::{
    Animation, AnimationRef, AnyAnimation, CurvedAnimation, Curves, Interval, Threshold,
};
use waft_embedder::{
    Brightness, Clip, Color, Offset, Rect, SemanticsRole, Size, TextBaseline, TextDirection,
};
use waft_foundation::{
    App, Handle, K_IS_WEB, Listenable, Listener, OptionRetained, Ref, ValueChanged,
};
use waft_painting::{
    AlignmentGeometry, BorderRadius, BoxDecoration, BoxPainter, Decoration, EdgeInsets,
    EdgeInsetsGeometry, ImageConfiguration, TextStyle,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, CustomPainter,
    MainAxisAlignment, MainAxisSize, PaintingContext, RenderBox, RenderBoxData, RenderHandle,
    RenderObject, RenderObjectData, RenderObjectWithChildData, RenderObjectWithChildMixin,
    RenderProxyBoxMixin, SingleChildLayoutDelegate,
};
use waft_services::{LogicalKeyboardKey, MouseCursorRef};

use crate::button_theme::ButtonTheme;
use crate::colors::Colors;
use crate::constants::{K_MATERIAL_LIST_PADDING, K_MIN_INTERACTIVE_DIMENSION};
use crate::debug::{debug_check_has_material, debug_check_has_material_localizations};
use crate::icons::Icons;
use crate::ink_decoration::Ink;
use crate::ink_well::InkWell;
use crate::input_decorator::{InputDecoration, InputDecorationTheme, InputDecorator};
use crate::material::{Material, MaterialType};
use crate::material_localizations::MaterialLocalizations;
use crate::scrollbar::Scrollbar;
use crate::shadows::k_elevation_to_shadow;
use crate::theme::Theme;

const K_DROPDOWN_MENU_DURATION: Duration = Duration::from_millis(300);
const K_MENU_ITEM_HEIGHT: f64 = K_MIN_INTERACTIVE_DIMENSION;
const K_DENSE_BUTTON_HEIGHT: f64 = 24.0;
const K_MENU_ITEM_PADDING: EdgeInsets = EdgeInsets::symmetric(0.0, 16.0);
const K_ALIGNED_BUTTON_PADDING: EdgeInsetsGeometry =
    EdgeInsetsGeometry::directional(16.0, 0.0, 4.0, 0.0);
const K_UNALIGNED_BUTTON_PADDING: EdgeInsets = EdgeInsets::ZERO;
const K_ALIGNED_MENU_MARGIN: EdgeInsets = EdgeInsets::ZERO;
const K_UNALIGNED_MENU_MARGIN: EdgeInsetsGeometry =
    EdgeInsetsGeometry::directional(16.0, 0.0, 24.0, 0.0);

/// A builder to customize dropdown buttons.
///
/// Used by [`DropdownButton::selected_item_builder`].
///
/// The list of widgets returned by this builder must be exactly the same length as the
/// [`DropdownButton::items`] list.
pub type DropdownButtonBuilder = Rc<dyn Fn(&mut App, BuildContext) -> Vec<WidgetRef>>;

// ---------------------------------------------------------------------------------------------
// _DropdownMenuPainter

/// Dart's `_DropdownMenuPainter`.
struct DropdownMenuPainter {
    color: Option<Color>,
    elevation: Option<i32>,
    selected_index: Option<usize>,
    border_radius: Option<BorderRadius>,
    resize: Ref<CurvedAnimation>,
    repaint: Rc<dyn Listenable>,
    get_selected_item_offset: Rc<dyn Fn() -> f64>,
    painter: RefCell<Box<dyn BoxPainter>>,
}

impl DropdownMenuPainter {
    fn new(
        color: Option<Color>,
        elevation: Option<i32>,
        selected_index: Option<usize>,
        border_radius: Option<BorderRadius>,
        resize: Ref<CurvedAnimation>,
        get_selected_item_offset: Rc<dyn Fn() -> f64>,
    ) -> DropdownMenuPainter {
        // If you add an image here, you must provide a real
        // configuration in the paint() function and you must provide some sort
        // of onChanged callback here.
        let mut decoration = BoxDecoration::new().border_radius(
            border_radius.unwrap_or(BorderRadius::all(waft_embedder::Radius::circular(2.0))),
        );
        if let Some(color) = color {
            decoration = decoration.color(color);
        }
        if let Some(shadows) = elevation.and_then(k_elevation_to_shadow) {
            decoration = decoration.box_shadow(shadows.to_vec());
        }
        DropdownMenuPainter {
            color,
            elevation,
            selected_index,
            border_radius,
            repaint: Rc::new(resize.erase::<AnyAnimation<f64>>()) as Rc<dyn Listenable>,
            resize,
            get_selected_item_offset,
            painter: RefCell::new(decoration.create_box_painter(None)),
        }
    }
}

impl CustomPainter for DropdownMenuPainter {
    fn repaint(&self) -> Option<&Rc<dyn Listenable>> {
        Some(&self.repaint)
    }

    fn paint(&self, app: &mut App, canvas: &mut waft_painting::Canvas, size: Size) {
        let selected_item_offset = (self.get_selected_item_offset)();
        let top_begin =
            selected_item_offset.clamp(0.0, (size.height() - K_MENU_ITEM_HEIGHT).max(0.0));
        let top_end = 0.0;

        let bottom_begin = (top_begin + K_MENU_ITEM_HEIGHT)
            .clamp(K_MENU_ITEM_HEIGHT.min(size.height()), size.height());
        let bottom_end = size.height();

        let resize = self.resize.value(app);
        let rect = Rect::from_ltrb(
            0.0,
            top_begin + (top_end - top_begin) * resize,
            size.width(),
            bottom_begin + (bottom_end - bottom_begin) * resize,
        );

        let configuration = ImageConfiguration {
            size: Some(rect.size()),
            ..ImageConfiguration::EMPTY
        };
        self.painter
            .borrow_mut()
            .paint(canvas, rect.top_left(), &configuration);
    }

    fn should_repaint(&self, _app: &App, old_painter: &dyn CustomPainter) -> bool {
        let old_painter = old_painter
            .as_any()
            .downcast_ref::<DropdownMenuPainter>()
            .expect("compared with a painter of its own type");
        old_painter.color != self.color
            || old_painter.elevation != self.elevation
            || old_painter.selected_index != self.selected_index
            || old_painter.border_radius != self.border_radius
            || old_painter.resize != self.resize
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------------------------
// _DropdownMenuItemButton

/// The widget that is the button wrapping the menu items; Dart's `_DropdownMenuItemButton<T>`.
struct DropdownMenuItemButton<T: Clone + PartialEq + Debug + 'static> {
    route: Handle<DropdownRoute<T>>,
    scroll_controller: Ref<ScrollController>,
    padding: Option<EdgeInsets>,
    button_rect: Rect,
    constraints: BoxConstraints,
    item_index: usize,
    enable_feedback: bool,
    mouse_cursor: Option<MouseCursorRef>,
}

impl<T: Clone + PartialEq + Debug + 'static> Debug for DropdownMenuItemButton<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownMenuItemButton")
            .field("padding", &self.padding)
            .field("button_rect", &self.button_rect)
            .field("item_index", &self.item_index)
            .field("enable_feedback", &self.enable_feedback)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> StatefulWidget for DropdownMenuItemButton<T> {
    type State = DropdownMenuItemButtonState<T>;

    fn create_state(&self) -> DropdownMenuItemButtonState<T> {
        DropdownMenuItemButtonState {
            state: StateData::new(),
            opacity_animation: None,
        }
    }
}

/// Dart's `_DropdownMenuItemButtonState<T>`.
struct DropdownMenuItemButtonState<T: Clone + PartialEq + Debug + 'static> {
    state: StateData<DropdownMenuItemButton<T>>,
    opacity_animation: Option<Ref<CurvedAnimation>>,
}

impl<T: Clone + PartialEq + Debug + 'static> Debug for DropdownMenuItemButtonState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownMenuItemButtonState")
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> DropdownMenuItemButtonState<T> {
    fn opacity_animation(self: Handle<Self>, app: &App) -> Ref<CurvedAnimation> {
        app.get(self)
            .opacity_animation
            .clone()
            .expect("made in init_state")
    }

    fn set_opacity_animation(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let route = app.get(widget.route);
        let item_index = widget.item_index;
        let unit = 0.5 / (route.items.len() as f64 + 1.5);
        let selected_index = route.selected_index;
        let animation = ModalRoute::animation(widget.route, app)
            .expect("an installed route has an animation")
            .keep(app);
        let opacity_animation = if item_index == selected_index {
            CurvedAnimation::new(app, animation, Rc::new(Threshold::new(0.0)), None)
        } else {
            let start = (0.5 + (item_index as f64 + 1.0) * unit).clamp(0.0, 1.0);
            let end = (start + 1.5 * unit).clamp(0.0, 1.0);
            CurvedAnimation::new(
                app,
                animation,
                Rc::new(Interval::new(start, end, Curves::linear())),
                None,
            )
        };
        app.get_mut(self).opacity_animation = Some(opacity_animation);
    }

    fn handle_focus_change(self: Handle<Self>, app: &mut App, focused: bool) {
        let in_traditional_mode = match FocusManager::instance(app).highlight_mode(app) {
            FocusHighlightMode::Touch => false,
            FocusHighlightMode::Traditional => true,
        };

        if focused && in_traditional_mode {
            let widget = self.widget(app);
            let menu_limits = app.get(widget.route).menu.get_menu_limits(
                widget.button_rect,
                widget.constraints.max_height,
                widget.item_index,
            );
            drop(widget.scroll_controller.animate_to(
                app,
                menu_limits.scroll_offset,
                Duration::from_millis(100),
                Curves::ease_in_out(),
            ));
        }
    }

    fn handle_on_tap(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let dropdown_menu_item = app
            .get(widget.route)
            .menu_item(widget.item_index)
            .item
            .clone()
            .expect("a menu item has its dropdown item");

        if let Some(on_tap) = &dropdown_menu_item.on_tap {
            on_tap.call(app);
        }

        let context = self.context(app);
        Navigator::pop(
            app,
            context,
            Some(Rc::new(DropdownRouteResult {
                result: dropdown_menu_item.value.clone(),
            })),
        );
    }

    /// Dart's `_webShortcuts`.
    fn web_shortcuts() -> waft_widgets::ShortcutMap {
        // On the web, up/down don't change focus, *except* in a <select>
        // element, which is what a dropdown emulates.
        vec![
            (
                Rc::new(SingleActivator::new(LogicalKeyboardKey::ARROW_DOWN)),
                Rc::new(DirectionalFocusIntent::new(TraversalDirection::Down)),
            ),
            (
                Rc::new(SingleActivator::new(LogicalKeyboardKey::ARROW_UP)),
                Rc::new(DirectionalFocusIntent::new(TraversalDirection::Up)),
            ),
        ]
    }
}

impl<T: Clone + PartialEq + Debug + 'static> State for DropdownMenuItemButtonState<T> {
    type Widget = DropdownMenuItemButton<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.set_opacity_animation(app);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &DropdownMenuItemButton<T>,
    ) {
        let widget = self.widget(app);
        if old_widget.item_index != widget.item_index
            || ModalRoute::animation(old_widget.route, app)
                != ModalRoute::animation(widget.route, app)
            || app.get(old_widget.route).selected_index != app.get(widget.route).selected_index
            || app.get(widget.route).items.len() != app.get(old_widget.route).items.len()
        {
            let replaced = self.opacity_animation(app);
            replaced.dispose(app);
            self.set_opacity_animation(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let opacity_animation = self.opacity_animation(app);
        opacity_animation.dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let route = app.get(widget.route);
        let dropdown_menu_item = route
            .menu_item(widget.item_index)
            .item
            .clone()
            .expect("a menu item has its dropdown item");
        let mut child = route.items[widget.item_index].clone();
        if let Some(padding) = widget.padding {
            child = Padding::new(padding.into()).child(child).into_widget();
        }
        let mut sized = SizedBox::new();
        if let Some(item_height) = route.item_height {
            sized = sized.height(item_height);
        }
        child = sized.child(child).into_widget();

        let is_selected = widget.item_index == route.selected_index;
        let highlight_mode = FocusManager::instance(app).highlight_mode(app);
        // An [InkWell] is added to the item only if it is enabled
        if dropdown_menu_item.enabled {
            let widget = self.widget(app);
            let mut ink_well = InkWell::new()
                .autofocus(is_selected)
                .enable_feedback(widget.enable_feedback)
                .on_tap(Listener::handle_method(self, Self::handle_on_tap))
                .on_focus_change(Rc::new(move |app: &mut App, focused: bool| {
                    self.handle_focus_change(app, focused)
                }));
            if let Some(mouse_cursor) = widget.mouse_cursor.clone() {
                ink_well = ink_well.mouse_cursor(mouse_cursor);
            }
            // When highlightMode is traditional, the InkWell draws the selected item background color.
            // When highlightMode is touch, insert an Ink to force the selected item background color.
            child = match highlight_mode {
                FocusHighlightMode::Touch => {
                    let mut ink = Ink::new();
                    if is_selected {
                        ink = ink.color(Theme::of(app, context).focus_color);
                    }
                    ink_well.child(ink.child(child)).into_widget()
                }
                FocusHighlightMode::Traditional => ink_well.child(child).into_widget(),
            };
        }
        child = FadeTransition::new(self.opacity_animation(app).erase())
            .child(child)
            .into_widget();
        if K_IS_WEB && dropdown_menu_item.enabled {
            child = Shortcuts::new(Self::web_shortcuts(), child).into_widget();
        }
        Semantics::new()
            .role(SemanticsRole::MenuItem)
            .child(child)
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// _DropdownMenu

/// Dart's `_DropdownMenu<T>`.
struct DropdownMenu<T: Clone + PartialEq + Debug + 'static> {
    route: Handle<DropdownRoute<T>>,
    padding: Option<EdgeInsets>,
    button_rect: Rect,
    constraints: BoxConstraints,
    dropdown_color: Option<Color>,
    enable_feedback: bool,
    border_radius: Option<BorderRadius>,
    scroll_controller: Ref<ScrollController>,
    mouse_cursor: Option<MouseCursorRef>,
}

impl<T: Clone + PartialEq + Debug + 'static> Debug for DropdownMenu<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownMenu")
            .field("padding", &self.padding)
            .field("button_rect", &self.button_rect)
            .field("dropdown_color", &self.dropdown_color)
            .field("enable_feedback", &self.enable_feedback)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> StatefulWidget for DropdownMenu<T> {
    type State = DropdownMenuState<T>;

    fn create_state(&self) -> DropdownMenuState<T> {
        DropdownMenuState {
            state: StateData::new(),
            fade_opacity: None,
            resize: None,
        }
    }
}

/// Dart's `_DropdownMenuState<T>`.
struct DropdownMenuState<T: Clone + PartialEq + Debug + 'static> {
    state: StateData<DropdownMenu<T>>,
    fade_opacity: Option<Ref<CurvedAnimation>>,
    resize: Option<Ref<CurvedAnimation>>,
}

impl<T: Clone + PartialEq + Debug + 'static> Debug for DropdownMenuState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownMenuState").finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> State for DropdownMenuState<T> {
    type Widget = DropdownMenu<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        // We need to hold these animations as state because of their curve
        // direction. When the route's animation reverses, if we were to recreate
        // the CurvedAnimation objects in build, we'd lose
        // CurvedAnimation._curveDirection.
        let route = self.widget(app).route;
        let animation: AnimationRef<f64> = ModalRoute::animation(route, app)
            .expect("an installed route has an animation")
            .keep(app);
        let fade_opacity = CurvedAnimation::new(
            app,
            animation.clone(),
            Rc::new(Interval::new(0.0, 0.25, Curves::linear())),
            Some(Rc::new(Interval::new(0.75, 1.0, Curves::linear()))),
        );
        let resize = CurvedAnimation::new(
            app,
            animation,
            Rc::new(Interval::new(0.25, 0.5, Curves::linear())),
            Some(Rc::new(Threshold::new(0.0))),
        );
        let state = app.get_mut(self);
        state.fade_opacity = Some(fade_opacity);
        state.resize = Some(resize);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let state = app.get(self);
        let fade_opacity = state.fade_opacity.handle().expect("made in init_state");
        let resize = state.resize.handle().expect("made in init_state");
        fade_opacity.dispose(app);
        resize.dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // The menu is shown in three stages (unit timing in brackets):
        // [0s - 0.25s] - Fade in a rect-sized menu container with the selected item.
        // [0.25s - 0.5s] - Grow the otherwise empty menu container from the center
        //   until it's big enough for as many items as we're going to show.
        // [0.5s - 1.0s] Fade in the remaining visible items from top to bottom.
        //
        // When the menu is dismissed we just fade the entire thing out
        // in the first 0.25s.
        debug_assert!(debug_check_has_material_localizations(app, context));
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let theme = Theme::of(app, context);
        let scroll_behavior = ScrollConfiguration::of(app, context);
        let widget = self.widget(app);
        let route_handle = widget.route;
        let item_count = app.get(route_handle).items.len();
        let children: Vec<WidgetRef> = (0..item_count)
            .map(|item_index| {
                DropdownMenuItemButton::<T> {
                    route: route_handle,
                    padding: widget.padding,
                    button_rect: widget.button_rect,
                    constraints: widget.constraints,
                    item_index,
                    enable_feedback: widget.enable_feedback,
                    scroll_controller: widget.scroll_controller.clone(),
                    mouse_cursor: widget.mouse_cursor.clone(),
                }
                .into_widget()
            })
            .collect();

        let route = app.get(route_handle);
        let state = app.get(self);
        let menu = Rc::clone(&route.menu);
        let selected_index = route.selected_index;
        let painter = DropdownMenuPainter::new(
            Some(widget.dropdown_color.unwrap_or(theme.canvas_color)),
            Some(route.elevation),
            Some(route.selected_index),
            widget.border_radius,
            state.resize.clone().expect("made in init_state"),
            // This offset is passed as a callback, not a value, because it must
            // be retrieved at paint time (after layout), not at build time.
            Rc::new(move || menu.get_item_offset(selected_index)),
        );
        let route_style = route.style.clone();
        let fade_opacity = state.fade_opacity.clone().expect("made in init_state");

        let behavior = scroll_behavior
            .copy_with()
            .scrollbars(false)
            .overscroll(false)
            .physics(Rc::new(waft_widgets::ClampingScrollPhysics::new()) as ScrollPhysicsRef)
            .platform(theme.platform);
        let scroll_controller: ScrollControllerRef = widget.scroll_controller.erase();

        FadeTransition::new(fade_opacity.erase())
            .child(
                CustomPaint::new().painter(painter).child(
                    Semantics::new()
                        .role(SemanticsRole::Menu)
                        .scopes_route(true)
                        .names_route(true)
                        .explicit_child_nodes(true)
                        .label(localizations.popup_menu_label())
                        .child(
                            ClipRRect::new()
                                .border_radius(
                                    widget.border_radius.unwrap_or(BorderRadius::ZERO).into(),
                                )
                                .clip_behavior(if widget.border_radius.is_some() {
                                    Clip::AntiAlias
                                } else {
                                    Clip::None
                                })
                                .child(
                                    Material::new()
                                        .r#type(MaterialType::Transparency)
                                        .text_style(route_style)
                                        .child(ScrollConfiguration::new(
                                            // Dropdown menus should never overscroll or display an overscroll indicator.
                                            // Scrollbars are built-in below.
                                            // Platform must use Theme and ScrollPhysics must be Clamping.
                                            waft_widgets::ScrollBehaviorRef::new(behavior),
                                            PrimaryScrollController::new(
                                                scroll_controller,
                                                Scrollbar::new(
                                                    ListView::new()
                                                        // Ensure this always inherits the PrimaryScrollController
                                                        .primary(true)
                                                        .padding(K_MATERIAL_LIST_PADDING.into())
                                                        .shrink_wrap(true)
                                                        .children(children),
                                                )
                                                .thumb_visibility(true),
                                            ),
                                        )),
                                ),
                        ),
                ),
            )
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// _DropdownMenuRouteLayout

/// Dart's `_DropdownMenuRouteLayout<T>`; it reads the route's [`DropdownRouteMenu`].
struct DropdownMenuRouteLayout {
    button_rect: Rect,
    route: Rc<DropdownRouteMenu>,
    text_direction: Option<TextDirection>,
    menu_width: Option<f64>,
}

impl Debug for DropdownMenuRouteLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownMenuRouteLayout")
            .field("button_rect", &self.button_rect)
            .field("text_direction", &self.text_direction)
            .field("menu_width", &self.menu_width)
            .finish_non_exhaustive()
    }
}

impl SingleChildLayoutDelegate for DropdownMenuRouteLayout {
    fn get_constraints_for_child(&self, constraints: BoxConstraints) -> BoxConstraints {
        // The maximum height of a simple menu should be one or more rows less than
        // the view height. This ensures a tappable area outside of the simple menu
        // with which to dismiss the menu.
        //   -- https://material.io/design/components/menus.html#usage
        let mut max_height = (constraints.max_height - 2.0 * K_MENU_ITEM_HEIGHT).max(0.0);
        if let Some(menu_max_height) = self.route.menu_max_height
            && menu_max_height <= max_height
        {
            max_height = menu_max_height;
        }
        // The width of a menu should be at most the view width. This ensures that
        // the menu does not extend past the left and right edges of the screen.
        let width = constraints
            .max_width
            .min(self.menu_width.unwrap_or(self.button_rect.width()));
        BoxConstraints::new()
            .min_width(width)
            .max_width(width)
            .max_height(max_height)
    }

    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        let menu_limits =
            self.route
                .get_menu_limits(self.button_rect, size.height(), self.route.selected_index);

        if cfg!(debug_assertions) {
            let container = Offset::ZERO & size;
            if container.intersect(self.button_rect) == self.button_rect {
                // If the button was entirely on-screen, then verify
                // that the menu is also on-screen.
                // If the button was a bit off-screen, then, oh well.
                debug_assert!(menu_limits.top >= 0.0);
                debug_assert!(menu_limits.top + menu_limits.height <= size.height());
            }
        }
        debug_assert!(self.text_direction.is_some());
        let left = match self.text_direction.expect("asserted above") {
            TextDirection::Rtl => {
                self.button_rect.right.clamp(0.0, size.width()) - child_size.width()
            }
            TextDirection::Ltr => self
                .button_rect
                .left
                .clamp(0.0, size.width() - child_size.width()),
        };

        Offset::new(left, menu_limits.top)
    }

    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
        let Some(old_delegate) = old_delegate
            .as_any()
            .downcast_ref::<DropdownMenuRouteLayout>()
        else {
            return true;
        };
        self.button_rect != old_delegate.button_rect
            || self.text_direction != old_delegate.text_direction
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------------------------
// _DropdownRouteResult, _MenuLimits

// We box the return value so that the return value can be null. Otherwise,
// canceling the route (which returns null) would get confused with actually
// returning a real null value.
/// Dart's `_DropdownRouteResult<T>`.
#[derive(Clone, PartialEq)]
struct DropdownRouteResult<T> {
    result: Option<T>,
}

/// Dart's `_MenuLimits`.
#[derive(Clone, Copy, Debug)]
struct MenuLimits {
    top: f64,
    #[expect(dead_code, reason = "Dart's field, which no caller reads")]
    bottom: f64,
    height: f64,
    scroll_offset: f64,
}

// ---------------------------------------------------------------------------------------------
// _DropdownRoute

/// What Dart's `_DropdownRoute` reads in `getItemOffset` and `getMenuLimits`, shared with the
/// route's layout delegate and painter: its `itemHeights`, which the items fill as they lay
/// out, its `menuMaxHeight` and its `selectedIndex`.
struct DropdownRouteMenu {
    item_heights: RefCell<Vec<f64>>,
    menu_max_height: Option<f64>,
    selected_index: usize,
}

impl DropdownRouteMenu {
    fn get_item_offset(&self, index: usize) -> f64 {
        let item_heights = self.item_heights.borrow();
        let mut offset = K_MATERIAL_LIST_PADDING.top;
        if !item_heights.is_empty() && index > 0 {
            offset += item_heights[..index]
                .iter()
                .copied()
                .reduce(|total, height| total + height)
                .expect("index > 0");
        }
        offset
    }

    // Returns the vertical extent of the menu and the initial scrollOffset
    // for the ListView that contains the menu items. The vertical center of the
    // selected item is aligned with the button's vertical center, as far as
    // that's possible given availableHeight.
    fn get_menu_limits(
        &self,
        button_rect: Rect,
        available_height: f64,
        index: usize,
    ) -> MenuLimits {
        let mut computed_max_height = available_height - 2.0 * K_MENU_ITEM_HEIGHT;
        if let Some(menu_max_height) = self.menu_max_height {
            computed_max_height = computed_max_height.min(menu_max_height);
        }
        let button_top = button_rect.top;
        let button_bottom = button_rect.bottom.min(available_height);
        let selected_item_offset = self.get_item_offset(index);

        // If the button is placed on the bottom or top of the screen, its top or
        // bottom may be less than [_kMenuItemHeight] from the edge of the screen.
        // In this case, we want to change the menu limits to align with the top
        // or bottom edge of the button.
        let top_limit = K_MENU_ITEM_HEIGHT.min(button_top);
        let bottom_limit = (available_height - K_MENU_ITEM_HEIGHT).max(button_bottom);

        let item_heights = self.item_heights.borrow();
        let selected_item_height = item_heights[self.selected_index];
        let mut menu_top = (button_top - selected_item_offset)
            - (selected_item_height - button_rect.height()) / 2.0;
        let mut preferred_menu_height = K_MATERIAL_LIST_PADDING.vertical();
        if !item_heights.is_empty() {
            preferred_menu_height += item_heights
                .iter()
                .copied()
                .reduce(|total, height| total + height)
                .expect("not empty");
        }

        // If there are too many elements in the menu, we need to shrink it down
        // so it is at most the computedMaxHeight.
        let menu_height = computed_max_height.min(preferred_menu_height);
        let mut menu_bottom = menu_top + menu_height;

        // If the computed top or bottom of the menu are outside of the range
        // specified, we need to bring them into range. If the item height is larger
        // than the button height and the button is at the very bottom or top of the
        // screen, the menu will be aligned with the bottom or top of the button
        // respectively.
        if menu_top < top_limit {
            menu_top = button_top.min(top_limit);
            menu_bottom = menu_top + menu_height;
        }

        if menu_bottom > bottom_limit {
            menu_bottom = button_bottom.max(bottom_limit);
            menu_top = menu_bottom - menu_height;
        }

        if menu_bottom - selected_item_height / 2.0 < button_bottom - button_rect.height() / 2.0 {
            menu_bottom = button_bottom - button_rect.height() / 2.0 + selected_item_height / 2.0;
            menu_top = menu_bottom - menu_height;
        }

        let mut scroll_offset = 0.0;
        // If all of the menu items will not fit within availableHeight then
        // compute the scroll offset that will line the selected menu item up
        // with the select item. This is only done when the menu is first
        // shown - subsequently we leave the scroll offset where the user left
        // it. This scroll offset is only accurate for fixed height menu items
        // (the default).
        if preferred_menu_height > computed_max_height {
            // The offset should be zero if the selected item is in view at the beginning
            // of the menu. Otherwise, the scroll offset should center the item if possible.
            scroll_offset = (selected_item_offset - (button_top - menu_top)).max(0.0);
            // If the selected item's scroll offset is greater than the maximum scroll offset,
            // set it instead to the maximum allowed scroll offset.
            scroll_offset = scroll_offset.min(preferred_menu_height - menu_height);
        }

        debug_assert!(
            (menu_bottom - menu_top - menu_height).abs()
                < waft_foundation::PRECISION_ERROR_TOLERANCE
        );
        MenuLimits {
            top: menu_top,
            bottom: menu_bottom,
            height: menu_height,
            scroll_offset,
        }
    }
}

/// Dart's `_DropdownRoute<T>`.
struct DropdownRoute<T: Clone + PartialEq + Debug + 'static> {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    items: Vec<WidgetRef>,
    padding: EdgeInsetsGeometry,
    button_rect: Rect,
    selected_index: usize,
    elevation: i32,
    captured_themes: Rc<CapturedThemes>,
    style: TextStyle,
    item_height: Option<f64>,
    menu_width: Option<f64>,
    dropdown_color: Option<Color>,
    enable_feedback: bool,
    border_radius: Option<BorderRadius>,
    dropdown_menu_item_mouse_cursor: Option<MouseCursorRef>,
    /// Dart's `itemHeights`, `menuMaxHeight` and `selectedIndex`, with the methods that read
    /// them.
    menu: Rc<DropdownRouteMenu>,
    barrier_dismissible: bool,
    barrier_label: Option<String>,
    /// The value type of the items, which the menu pops wrapped in a `DropdownRouteResult`.
    value_type: PhantomData<fn() -> T>,
}

/// The arguments of Dart's `_DropdownRoute` constructor.
struct DropdownRouteArguments<T: Clone + PartialEq + Debug + 'static> {
    items: Vec<WidgetRef>,
    padding: EdgeInsetsGeometry,
    button_rect: Rect,
    selected_index: usize,
    elevation: i32,
    captured_themes: CapturedThemes,
    style: TextStyle,
    barrier_label: Option<String>,
    item_height: Option<f64>,
    menu_width: Option<f64>,
    dropdown_color: Option<Color>,
    menu_max_height: Option<f64>,
    enable_feedback: bool,
    border_radius: Option<BorderRadius>,
    barrier_dismissible: bool,
    dropdown_menu_item_mouse_cursor: Option<MouseCursorRef>,
    value_type: PhantomData<fn() -> T>,
}

impl<T: Clone + PartialEq + Debug + 'static> DropdownRoute<T> {
    fn new(app: &mut App, arguments: DropdownRouteArguments<T>) -> Ref<DropdownRoute<T>> {
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let item_heights = vec![
            arguments.item_height.unwrap_or(K_MIN_INTERACTIVE_DIMENSION);
            arguments.items.len()
        ];
        app.create(DropdownRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            items: arguments.items,
            padding: arguments.padding,
            button_rect: arguments.button_rect,
            selected_index: arguments.selected_index,
            elevation: arguments.elevation,
            captured_themes: Rc::new(arguments.captured_themes),
            style: arguments.style,
            item_height: arguments.item_height,
            menu_width: arguments.menu_width,
            dropdown_color: arguments.dropdown_color,
            enable_feedback: arguments.enable_feedback,
            border_radius: arguments.border_radius,
            dropdown_menu_item_mouse_cursor: arguments.dropdown_menu_item_mouse_cursor,
            menu: Rc::new(DropdownRouteMenu {
                item_heights: RefCell::new(item_heights),
                menu_max_height: arguments.menu_max_height,
                selected_index: arguments.selected_index,
            }),
            barrier_dismissible: arguments.barrier_dismissible,
            barrier_label: arguments.barrier_label,
            value_type: PhantomData,
        })
    }

    /// The [`MenuItem`] `items[index]` is.
    fn menu_item(&self, index: usize) -> &MenuItem<T> {
        waft_widgets::downcast_widget::<MenuItem<T>>(&*self.items[index])
            .expect("the route's items are menu items")
    }

    fn dismiss(self: Handle<Self>, app: &mut App) {
        if Route::is_active(self, app)
            && let Some(navigator) = Route::navigator(self, app)
        {
            navigator.remove_route(app, Route::as_route(self), None);
        }
    }
}

impl<T: Clone + PartialEq + Debug + 'static> Route for DropdownRoute<T> {
    waft_widgets::modal_route_overrides!();
}

impl<T: Clone + PartialEq + Debug + 'static> OverlayRoute for DropdownRoute<T> {
    waft_widgets::modal_overlay_route_overrides!();
}

impl<T: Clone + PartialEq + Debug + 'static> PredictiveBackRoute for DropdownRoute<T> {
    waft_widgets::modal_predictive_back_route_overrides!(ModalRoute::pop_gesture_enabled);
}

impl<T: Clone + PartialEq + Debug + 'static> TransitionRoute for DropdownRoute<T> {
    waft_widgets::modal_transition_route_overrides!();

    fn transition_duration(self: Handle<Self>, _app: &mut App) -> Duration {
        K_DROPDOWN_MENU_DURATION
    }

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::opaque(self, app)
    }

    fn allow_snapshotting(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::allow_snapshotting(self, app)
    }
}

impl<T: Clone + PartialEq + Debug + 'static> LocalHistoryRoute for DropdownRoute<T> {
    waft_widgets::local_history_route_accessors!();
}

impl<T: Clone + PartialEq + Debug + 'static> ModalRoute for DropdownRoute<T> {
    waft_widgets::modal_route_accessors!();

    fn barrier_dismissible(self: Handle<Self>, app: &App) -> bool {
        app.get(self).barrier_dismissible
    }

    fn barrier_color(self: Handle<Self>, _app: &App) -> Option<Color> {
        None
    }

    fn barrier_label(self: Handle<Self>, app: &App) -> Option<String> {
        app.get(self).barrier_label.clone()
    }

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::maintain_state(self, app)
    }

    fn build_page(
        self: Handle<Self>,
        _app: &mut App,
        _context: BuildContext,
        _animation: AnimationRef<f64>,
        _secondary_animation: AnimationRef<f64>,
    ) -> WidgetRef {
        LayoutBuilder::new(move |app, _context, constraints| {
            let route = app.get(self);
            DropdownRoutePage::<T> {
                route: self,
                constraints: constraints.constraints(),
                padding: route.padding,
                button_rect: route.button_rect,
                selected_index: route.selected_index,
                elevation: route.elevation,
                captured_themes: Rc::clone(&route.captured_themes),
                style: Some(route.style.clone()),
                dropdown_color: route.dropdown_color,
                enable_feedback: route.enable_feedback,
                border_radius: route.border_radius,
                menu_width: route.menu_width,
                mouse_cursor: route.dropdown_menu_item_mouse_cursor.clone(),
            }
            .into_widget()
        })
        .into_widget()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> PopupRoute for DropdownRoute<T> {}

// ---------------------------------------------------------------------------------------------
// _DropdownRoutePage

/// Dart's `_DropdownRoutePage<T>`; Dart's `items` argument, which the page never reads, is
/// the route's.
struct DropdownRoutePage<T: Clone + PartialEq + Debug + 'static> {
    route: Handle<DropdownRoute<T>>,
    constraints: BoxConstraints,
    padding: EdgeInsetsGeometry,
    button_rect: Rect,
    selected_index: usize,
    #[expect(dead_code, reason = "Dart's field, which the page never reads")]
    elevation: i32,
    captured_themes: Rc<CapturedThemes>,
    #[expect(dead_code, reason = "Dart's field, which the page never reads")]
    style: Option<TextStyle>,
    dropdown_color: Option<Color>,
    enable_feedback: bool,
    border_radius: Option<BorderRadius>,
    menu_width: Option<f64>,
    mouse_cursor: Option<MouseCursorRef>,
}

impl<T: Clone + PartialEq + Debug + 'static> Debug for DropdownRoutePage<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownRoutePage")
            .field("constraints", &self.constraints)
            .field("button_rect", &self.button_rect)
            .field("selected_index", &self.selected_index)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> StatefulWidget for DropdownRoutePage<T> {
    type State = DropdownRoutePageState<T>;

    fn create_state(&self) -> DropdownRoutePageState<T> {
        DropdownRoutePageState {
            state: StateData::new(),
            scroll_controller: None,
        }
    }
}

/// Dart's `_DropdownRoutePageState<T>`.
struct DropdownRoutePageState<T: Clone + PartialEq + Debug + 'static> {
    state: StateData<DropdownRoutePage<T>>,
    scroll_controller: Option<Ref<ScrollController>>,
}

impl<T: Clone + PartialEq + Debug + 'static> Debug for DropdownRoutePageState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownRoutePageState")
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> State for DropdownRoutePageState<T> {
    type Widget = DropdownRoutePage<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        // Computing the initialScrollOffset now, before the items have been laid
        // out. This only works if the item heights are effectively fixed, i.e. either
        // DropdownButton.itemHeight is specified or DropdownButton.itemHeight is null
        // and all of the items' intrinsic heights are less than kMinInteractiveDimension.
        // Otherwise the initialScrollOffset is just a rough approximation based on
        // treating the items as if their heights were all equal to kMinInteractiveDimension.
        let widget = self.widget(app);
        let menu_limits = app.get(widget.route).menu.get_menu_limits(
            widget.button_rect,
            widget.constraints.max_height,
            widget.selected_index,
        );
        let scroll_controller =
            ScrollController::new(app, menu_limits.scroll_offset, true, None, None, None);
        app.get_mut(self).scroll_controller = Some(scroll_controller);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(waft_widgets::debug_check_has_directionality(
            app, context, None, None, None
        ));

        let text_direction = Directionality::maybe_of(app, context);
        let widget = self.widget(app);
        let menu = DropdownMenu::<T> {
            route: widget.route,
            padding: Some(widget.padding.resolve(text_direction)),
            button_rect: widget.button_rect,
            constraints: widget.constraints,
            dropdown_color: widget.dropdown_color,
            enable_feedback: widget.enable_feedback,
            border_radius: widget.border_radius,
            scroll_controller: app
                .get(self)
                .scroll_controller
                .clone()
                .expect("made in init_state"),
            mouse_cursor: widget.mouse_cursor.clone(),
        }
        .into_widget();

        let button_rect = widget.button_rect;
        let route_menu = Rc::clone(&app.get(widget.route).menu);
        let menu_width = widget.menu_width;
        let captured_themes = Rc::clone(&widget.captured_themes);
        MediaQuery::remove_padding(
            app,
            context,
            true,
            true,
            true,
            true,
            waft_widgets::Builder::new(move |_app, _context| {
                CustomSingleChildLayout::new(Rc::new(DropdownMenuRouteLayout {
                    button_rect,
                    route: Rc::clone(&route_menu),
                    text_direction,
                    menu_width,
                }))
                .child(captured_themes.wrap(menu.clone()))
                .into_widget()
            })
            .into_widget(),
        )
        .into_widget()
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(scroll_controller) = app.get(self).scroll_controller.handle() {
            scroll_controller.dispose(app);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// _MenuItem

// This widget enables _DropdownRoute to look up the sizes of
// each menu item. These sizes are used to compute the offset of the selected
// item so that _DropdownRoutePage can align the vertical center of the
// selected item lines up with the vertical center of the dropdown button,
// as closely as possible.
/// Dart's `_MenuItem<T>`.
struct MenuItem<T: Clone + PartialEq + Debug + 'static> {
    on_layout: OnLayout,
    item: Option<DropdownMenuItem<T>>,
    child: Option<WidgetRef>,
}

impl<T: Clone + PartialEq + Debug + 'static> MenuItem<T> {
    fn new(on_layout: ValueChanged<Size>, item: DropdownMenuItem<T>) -> MenuItem<T> {
        MenuItem {
            on_layout: OnLayout(on_layout),
            child: Some(item.clone().into_widget()),
            item: Some(item),
        }
    }
}

impl<T: Clone + PartialEq + Debug + 'static> Debug for MenuItem<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuItem")
            .field("on_layout", &self.on_layout)
            .finish_non_exhaustive()
    }
}

/// Dart's `ValueChanged<Size> onLayout`, which `Debug` prints by name.
#[derive(Clone)]
struct OnLayout(ValueChanged<Size>);

impl Debug for OnLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("onLayout")
    }
}

impl<T: Clone + PartialEq + Debug + 'static> RenderObjectWidget for MenuItem<T> {
    type RenderObject = RenderMenuItem;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderMenuItem::new(app, self.on_layout.clone()).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderMenuItem>,
    ) {
        render_object.get_mut(app).on_layout = self.on_layout.clone();
    }
}

impl<T: Clone + PartialEq + Debug + 'static> SingleChildRenderObjectWidget for MenuItem<T> {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderMenuItem`.
struct RenderMenuItem {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    on_layout: OnLayout,
}

impl RenderMenuItem {
    fn new(app: &mut App, on_layout: OnLayout) -> RenderHandle<RenderMenuItem> {
        RenderHandle::new_box(
            app,
            RenderMenuItem {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                on_layout,
            },
        )
    }
}

impl RenderObjectWithChildMixin for RenderMenuItem {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderProxyBoxMixin for RenderMenuItem {}

impl RenderObject for RenderMenuItem {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
        let size = self.size(app);
        let on_layout = Rc::clone(&self.get(app).on_layout.0);
        on_layout(app, size);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderProxyBoxMixin::paint(self, app, context, offset);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderMenuItem {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut waft_embedder::Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }
}

// ---------------------------------------------------------------------------------------------
// _DropdownMenuItemContainer

// The container widget for a menu item created by a [DropdownButton]. It
// provides the default configuration for [DropdownMenuItem]s, as well as a
// [DropdownButton]'s hint and disabledHint widgets.
/// Dart's `_DropdownMenuItemContainer`; [`DropdownMenuItem`] builds what it builds.
#[derive(Debug)]
struct DropdownMenuItemContainer {
    key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    ///
    /// Typically a `Text` widget.
    child: WidgetRef,
    /// Defines how the item is positioned within the container.
    ///
    /// Defaults to `AlignmentDirectional::CENTER_START`.
    alignment: AlignmentGeometry,
}

impl DropdownMenuItemContainer {
    /// Creates an item for a dropdown menu.
    fn new<K>(child: impl IntoWidget<K>) -> DropdownMenuItemContainer {
        DropdownMenuItemContainer {
            key: None,
            child: child.into_widget(),
            alignment: AlignmentGeometry::CENTER_START,
        }
    }

    fn alignment(mut self, alignment: AlignmentGeometry) -> DropdownMenuItemContainer {
        self.alignment = alignment;
        self
    }

    /// Dart's `_DropdownMenuItemContainer.build`.
    fn build_container(alignment: AlignmentGeometry, child: &WidgetRef) -> WidgetRef {
        Semantics::new()
            .button(true)
            .child(
                ConstrainedBox::new(BoxConstraints::new().min_height(K_MENU_ITEM_HEIGHT)).child(
                    waft_widgets::Align::new()
                        .alignment(alignment)
                        .child(child.clone()),
                ),
            )
            .into_widget()
    }
}

impl StatelessWidget for DropdownMenuItemContainer {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        DropdownMenuItemContainer::build_container(self.alignment, &self.child)
    }
}

// ---------------------------------------------------------------------------------------------
// DropdownMenuItem

/// An item in a menu created by a [`DropdownButton`].
///
/// The type `T` is the type of the value the entry represents. All the entries in a given menu
/// must represent values with consistent types.
pub struct DropdownMenuItem<T: 'static> {
    pub key: Option<KeyRef>,
    /// Called when the dropdown menu item is tapped.
    pub on_tap: Option<Listener>,
    /// The value to return if the user selects this menu item.
    ///
    /// Eventually returned in a call to [`DropdownButton::on_changed`].
    pub value: Option<T>,
    /// Whether or not a user can select this menu item.
    ///
    /// Defaults to `true`.
    pub enabled: bool,
    /// Defines how the item is positioned within the container.
    ///
    /// Defaults to `AlignmentDirectional::CENTER_START`.
    pub alignment: AlignmentGeometry,
    /// The widget below this widget in the tree.
    ///
    /// Typically a `Text` widget.
    pub child: WidgetRef,
}

impl<T: 'static> DropdownMenuItem<T> {
    /// Creates an item for a dropdown menu.
    pub fn new<K>(child: impl IntoWidget<K>) -> DropdownMenuItem<T> {
        DropdownMenuItem {
            key: None,
            on_tap: None,
            value: None,
            enabled: true,
            alignment: AlignmentGeometry::CENTER_START,
            child: child.into_widget(),
        }
    }

    /// Dart `DropdownMenuItem(key:)`.
    pub fn key(mut self, key: KeyRef) -> DropdownMenuItem<T> {
        self.key = Some(key);
        self
    }

    /// Dart `DropdownMenuItem(onTap:)`.
    pub fn on_tap(mut self, on_tap: Listener) -> DropdownMenuItem<T> {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `DropdownMenuItem(value:)`.
    pub fn value(mut self, value: T) -> DropdownMenuItem<T> {
        self.value = Some(value);
        self
    }

    /// Dart `DropdownMenuItem(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> DropdownMenuItem<T> {
        self.enabled = enabled;
        self
    }

    /// Dart `DropdownMenuItem(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> DropdownMenuItem<T> {
        self.alignment = alignment;
        self
    }
}

impl<T: Clone + 'static> Clone for DropdownMenuItem<T> {
    fn clone(&self) -> DropdownMenuItem<T> {
        DropdownMenuItem {
            key: self.key.clone(),
            on_tap: self.on_tap.clone(),
            value: self.value.clone(),
            enabled: self.enabled,
            alignment: self.alignment,
            child: self.child.clone(),
        }
    }
}

impl<T: Debug + 'static> Debug for DropdownMenuItem<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownMenuItem")
            .field("value", &self.value)
            .field("enabled", &self.enabled)
            .field("alignment", &self.alignment)
            .finish_non_exhaustive()
    }
}

impl<T: Debug + 'static> StatelessWidget for DropdownMenuItem<T> {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        DropdownMenuItemContainer::build_container(self.alignment, &self.child)
    }
}

// ---------------------------------------------------------------------------------------------
// DropdownButtonHideUnderline

/// An inherited widget that causes any descendant [`DropdownButton`] widgets to not include
/// their regular underline.
///
/// This is used by `DataTable` to remove the underline from any [`DropdownButton`] widgets
/// placed within material data tables, as required by the Material Design specification.
#[derive(Debug)]
pub struct DropdownButtonHideUnderline {
    pub key: Option<KeyRef>,
    pub child: WidgetRef,
}

impl DropdownButtonHideUnderline {
    /// Creates a [`DropdownButtonHideUnderline`]. A non-null `child` must be given.
    pub fn new<K>(child: impl IntoWidget<K>) -> DropdownButtonHideUnderline {
        DropdownButtonHideUnderline {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `DropdownButtonHideUnderline(key:)`.
    pub fn key(mut self, key: KeyRef) -> DropdownButtonHideUnderline {
        self.key = Some(key);
        self
    }

    /// Returns whether the underline of [`DropdownButton`] widgets should be hidden.
    pub fn at(app: &mut App, context: BuildContext) -> bool {
        context
            .depend_on_inherited_widget_of_exact_type::<DropdownButtonHideUnderline>(app)
            .is_some()
    }
}

impl InheritedWidget for DropdownButtonHideUnderline {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, _old_widget: &DropdownButtonHideUnderline) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------------------------
// DropdownButton

/// A Material Design button for selecting from a list of items.
///
/// A dropdown button lets the user select from a number of items. The button shows the
/// currently selected item as well as an arrow that opens a menu for selecting another item.
///
/// There is a Material 3 version of this component, `DropdownMenu`, that is preferred for
/// applications that are configured for Material 3 (see `ThemeData::use_material3`).
///
/// One ancestor must be a [`Material`] widget and typically this is provided by the app's
/// `Scaffold`.
///
/// The type `T` is the type of the [`value`](Self::value) that each dropdown item represents.
/// All the entries in a given menu must represent values with consistent types. Typically, an
/// enum is used. Each [`DropdownMenuItem`] in [`items`](Self::items) must be specialized with
/// that same type argument.
///
/// The [`on_changed`](Self::on_changed) callback should update a state variable that defines
/// the dropdown's value. It should also call `State::set_state` to rebuild the dropdown with
/// the new value.
///
/// If the [`on_changed`](Self::on_changed) callback is null or the list of
/// [`items`](Self::items) is null then the dropdown button will be disabled, i.e. its arrow
/// will be displayed in grey and it will not respond to input. A disabled button will display
/// the [`disabled_hint`](Self::disabled_hint) widget if it is non-null. However, if
/// [`disabled_hint`](Self::disabled_hint) is null and [`hint`](Self::hint) is non-null, the
/// [`hint`](Self::hint) widget will instead be displayed.
///
/// See also:
///
///  * [`DropdownButtonFormField`], which integrates with the `Form` widget.
///  * [`DropdownMenuItem`], the class used to represent the [`items`](Self::items).
///  * [`DropdownButtonHideUnderline`], which prevents its descendant dropdown buttons from
///    displaying their underlines.
///  * `ElevatedButton`, `TextButton`, ordinary buttons that trigger a single action.
///  * <https://material.io/design/components/menus.html#dropdown-menu>
pub struct DropdownButton<T: 'static> {
    pub key: Option<KeyRef>,
    /// The list of items the user can select.
    ///
    /// If the [`on_changed`](Self::on_changed) callback is null or the list of items is null
    /// then the dropdown button will be disabled, i.e. its arrow will be displayed in grey and
    /// it will not respond to input.
    pub items: Option<Vec<DropdownMenuItem<T>>>,
    /// The value of the currently selected [`DropdownMenuItem`].
    ///
    /// If [`value`](Self::value) is null and the button is enabled, [`hint`](Self::hint) will
    /// be displayed if it is non-null.
    ///
    /// If [`value`](Self::value) is null and the button is disabled,
    /// [`disabled_hint`](Self::disabled_hint) will be displayed if it is non-null. If
    /// [`disabled_hint`](Self::disabled_hint) is null, then [`hint`](Self::hint) will be
    /// displayed if it is non-null.
    pub value: Option<T>,
    /// A placeholder widget that is displayed by the dropdown button.
    ///
    /// If [`value`](Self::value) is null and the dropdown is enabled ([`items`](Self::items)
    /// and [`on_changed`](Self::on_changed) are non-null), this widget is displayed as a
    /// placeholder for the dropdown button's value.
    ///
    /// If [`value`](Self::value) is null and the dropdown is disabled and
    /// [`disabled_hint`](Self::disabled_hint) is null, this widget is used as the placeholder.
    pub hint: Option<WidgetRef>,
    /// A preferred placeholder widget that is displayed when the dropdown is disabled.
    ///
    /// If [`value`](Self::value) is null, the dropdown is disabled ([`items`](Self::items) or
    /// [`on_changed`](Self::on_changed) is null), this widget is displayed as a placeholder
    /// for the dropdown button's value.
    pub disabled_hint: Option<WidgetRef>,
    /// Called when the user selects an item.
    ///
    /// If the [`on_changed`](Self::on_changed) callback is null or the list of
    /// [`DropdownButton::items`] is null then the dropdown button will be disabled, i.e. its
    /// arrow will be displayed in grey and it will not respond to input. A disabled button will
    /// display the [`DropdownButton::disabled_hint`] widget if it is non-null. If
    /// [`DropdownButton::disabled_hint`] is also null but [`DropdownButton::hint`] is
    /// non-null, [`DropdownButton::hint`] will instead be displayed.
    pub on_changed: Option<ValueChanged<Option<T>>>,
    /// Called when the dropdown button is tapped.
    ///
    /// This is distinct from [`on_changed`](Self::on_changed), which is called when the user
    /// selects an item from the dropdown.
    ///
    /// The callback will not be invoked if the dropdown button is disabled.
    pub on_tap: Option<Listener>,
    /// A builder to customize the dropdown buttons corresponding to the
    /// [`DropdownMenuItem`]s in [`items`](Self::items).
    ///
    /// When a [`DropdownMenuItem`] is selected, the widget that will be displayed from the list
    /// corresponds to the [`DropdownMenuItem`] of the same index in [`items`](Self::items).
    ///
    /// If this callback is null, the [`DropdownMenuItem`] from [`items`](Self::items) that
    /// matches [`value`](Self::value) will be displayed.
    ///
    /// The list of widgets returned by this builder must be exactly the same length as the
    /// [`items`](Self::items) list.
    pub selected_item_builder: Option<DropdownButtonBuilder>,
    /// The z-coordinate at which to place the menu when open.
    ///
    /// The following elevations have defined shadows: 1, 2, 3, 4, 6, 8, 9, 12, 16, and 24. See
    /// [`k_elevation_to_shadow`].
    ///
    /// Defaults to 8, the appropriate elevation for dropdown buttons.
    pub elevation: i32,
    /// The text style to use for text in the dropdown button and the dropdown menu that
    /// appears when you tap the button.
    ///
    /// To use a separate text style for selected item when it's displayed within the dropdown
    /// button, consider using [`selected_item_builder`](Self::selected_item_builder).
    ///
    /// Defaults to the `TextTheme::title_medium` value of the current `ThemeData::text_theme`
    /// of the current [`Theme`].
    pub style: Option<TextStyle>,
    /// The widget to use for drawing the drop-down button's underline.
    ///
    /// Defaults to a 0.0 width bottom border with color 0xFFBDBDBD.
    pub underline: Option<WidgetRef>,
    /// The widget to use for the drop-down button's icon.
    ///
    /// Defaults to an [`Icon`] with the [`Icons::ARROW_DROP_DOWN`] glyph.
    pub icon: Option<WidgetRef>,
    /// The color of any [`Icon`] descendant of [`icon`](Self::icon) if this button is
    /// disabled, i.e. if [`on_changed`](Self::on_changed) is null.
    ///
    /// Defaults to `MaterialColor::shade400` of `Colors::GREY` when the theme's
    /// `ThemeData::brightness` is [`Brightness::Light`] and to `Colors::WHITE10` when it is
    /// [`Brightness::Dark`].
    pub icon_disabled_color: Option<Color>,
    /// The color of any [`Icon`] descendant of [`icon`](Self::icon) if this button is enabled,
    /// i.e. if [`on_changed`](Self::on_changed) is defined.
    ///
    /// Defaults to `MaterialColor::shade700` of `Colors::GREY` when the theme's
    /// `ThemeData::brightness` is [`Brightness::Light`] and to `Colors::WHITE70` when it is
    /// [`Brightness::Dark`].
    pub icon_enabled_color: Option<Color>,
    /// The size to use for the drop-down button's down arrow icon button.
    ///
    /// Defaults to 24.0.
    pub icon_size: f64,
    /// Reduce the button's height.
    ///
    /// By default this button's height is the same as its menu items' heights. If
    /// [`is_dense`](Self::is_dense) is true, the button's height is reduced by about half.
    /// This can be useful when the button is embedded in a container that adds its own
    /// decorations, like [`InputDecorator`].
    pub is_dense: bool,
    /// Set the dropdown's inner contents to horizontally fill its parent.
    ///
    /// By default this button's inner width is the minimum size of its contents. If
    /// [`is_expanded`](Self::is_expanded) is true, the inner width is expanded to fill its
    /// surrounding container.
    ///
    /// See also:
    ///
    ///  * [`DropdownButton::is_vertically_expanded`], which expands the inner height of the
    ///    dropdown to fill the available vertical space.
    pub is_expanded: bool,
    /// If null, then the menu item heights will vary according to each menu item's intrinsic
    /// height.
    ///
    /// The default value is [`K_MIN_INTERACTIVE_DIMENSION`], which is also the minimum height
    /// for menu items.
    ///
    /// If this value is null and there isn't enough vertical room for the menu, then the
    /// menu's initial scroll offset may not align the selected item with the dropdown button.
    /// That's because, in this case, the initial scroll offset is computed as if all of the
    /// menu item heights were [`K_MIN_INTERACTIVE_DIMENSION`].
    pub item_height: Option<f64>,
    /// The width of the menu.
    ///
    /// If it is not provided, the width of the menu is the width of the dropdown button.
    pub menu_width: Option<f64>,
    /// The color for the button's [`Material`] when it has the input focus.
    pub focus_color: Option<Color>,
    /// {@macro flutter.widgets.Focus.focusNode}
    pub focus_node: Option<FocusNodeRef>,
    /// {@macro flutter.widgets.Focus.autofocus}
    pub autofocus: bool,
    /// The background color of the dropdown.
    ///
    /// If it is not provided, the theme's `ThemeData::canvas_color` will be used instead.
    pub dropdown_color: Option<Color>,
    /// The maximum height of the menu.
    ///
    /// The maximum height of the menu must be at least one row shorter than the height of the
    /// app's view. This ensures that a tappable area outside of the simple menu is present so
    /// the user can dismiss the menu.
    ///
    /// If this property is set above the maximum allowable height threshold mentioned above,
    /// then the menu defaults to being padded at the top and bottom of the menu by at one menu
    /// item's height.
    pub menu_max_height: Option<f64>,
    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a long-press will
    /// produce a short vibration, when feedback is enabled.
    ///
    /// By default, platform-specific feedback is enabled.
    pub enable_feedback: Option<bool>,
    /// Defines how the hint or the selected item is positioned within the button.
    ///
    /// Defaults to `AlignmentDirectional::CENTER_START`.
    pub alignment: AlignmentGeometry,
    /// Defines the corner radii of the menu's rounded rectangle shape.
    pub border_radius: Option<BorderRadius>,
    /// Padding around the visible portion of the dropdown widget.
    ///
    /// As the padding increases, the size of the [`DropdownButton`] will also increase. The
    /// padding is included in the clickable area of the dropdown widget, so this can make the
    /// widget easier to click.
    ///
    /// Padding can be useful when used with a custom border. The clickable area will stay
    /// flush with the border, as opposed to an external [`Padding`] widget which will leave a
    /// non-clickable gap.
    pub padding: Option<EdgeInsetsGeometry>,
    /// Determines whether tapping outside the dropdown will close it.
    ///
    /// Defaults to `true`.
    pub barrier_dismissible: bool,
    /// The cursor for a mouse pointer when it enters or is hovering over this button.
    ///
    /// If this property is null, [`WidgetStateMouseCursor::adaptive_clickable`] will be used.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// The cursor for a mouse pointer when it enters or is hovering over the this button's
    /// [`items`](Self::items).
    ///
    /// If this property is null, [`WidgetStateMouseCursor::adaptive_clickable`] will be used.
    pub dropdown_menu_item_mouse_cursor: Option<MouseCursorRef>,
    /// Whether the dropdown's inner contents expand to fill the available vertical space.
    ///
    /// When true, the inner height of the dropdown is expanded to match its parent container.
    /// When false, the inner height is determined by the height of the selected item (its
    /// intrinsic height).
    ///
    /// This is particularly useful when the dropdown is used inside a fixed-height container,
    /// ensuring that the underline and items align properly within the allocated space.
    ///
    /// Defaults to false for [`DropdownButtonFormField`] to ensure the input decorator handles
    /// the vertical constraints correctly by default.
    ///
    /// See also:
    ///
    ///  * [`DropdownButton::is_expanded`], which expands the inner width of the dropdown to
    ///    fill the available horizontal space.
    pub is_vertically_expanded: bool,
    input_decoration: Option<InputDecoration>,
    is_empty: bool,
}

impl<T: Clone + PartialEq + Debug + 'static> DropdownButton<T> {
    /// Creates a dropdown button.
    ///
    /// The [`items`](Self::items) must have distinct values. If [`value`](Self::value) isn't
    /// null then it must be equal to one of the [`DropdownMenuItem`] values. If
    /// [`items`](Self::items) or [`on_changed`](Self::on_changed) is null, the button will be
    /// disabled, the down arrow will be greyed out.
    ///
    /// If [`value`](Self::value) is null and the button is enabled, [`hint`](Self::hint) will
    /// be displayed if it is non-null.
    ///
    /// If [`value`](Self::value) is null and the button is disabled,
    /// [`disabled_hint`](Self::disabled_hint) will be displayed if it is non-null. If
    /// [`disabled_hint`](Self::disabled_hint) is null, then [`hint`](Self::hint) will be
    /// displayed if it is non-null.
    ///
    /// The [`dropdown_color`](Self::dropdown_color) argument specifies the background color of
    /// the dropdown when it is open. If it is null, the current theme's
    /// `ThemeData::canvas_color` will be used instead.
    pub fn new(
        items: Option<Vec<DropdownMenuItem<T>>>,
        on_changed: Option<ValueChanged<Option<T>>>,
    ) -> DropdownButton<T> {
        DropdownButton {
            key: None,
            items,
            value: None,
            hint: None,
            disabled_hint: None,
            on_changed,
            on_tap: None,
            selected_item_builder: None,
            elevation: 8,
            style: None,
            underline: None,
            icon: None,
            icon_disabled_color: None,
            icon_enabled_color: None,
            icon_size: 24.0,
            is_dense: false,
            is_expanded: false,
            item_height: Some(K_MIN_INTERACTIVE_DIMENSION),
            menu_width: None,
            focus_color: None,
            focus_node: None,
            autofocus: false,
            dropdown_color: None,
            menu_max_height: None,
            enable_feedback: None,
            alignment: AlignmentGeometry::CENTER_START,
            border_radius: None,
            padding: None,
            barrier_dismissible: true,
            mouse_cursor: None,
            dropdown_menu_item_mouse_cursor: None,
            is_vertically_expanded: true,
            input_decoration: None,
            is_empty: false,
        }
    }

    /// Dart's `DropdownButton._formField`: what [`DropdownButtonFormField`]'s builder makes
    /// from the arguments it captured, which its `template` holds with Dart's defaults of this
    /// constructor (`underline` and `menu_width` null).
    fn form_field(
        template: &DropdownButton<T>,
        value: Option<T>,
        hint: Option<WidgetRef>,
        disabled_hint: Option<WidgetRef>,
        on_changed: Option<ValueChanged<Option<T>>>,
        input_decoration: InputDecoration,
        is_empty: bool,
    ) -> DropdownButton<T> {
        DropdownButton {
            key: None,
            items: template.items.clone(),
            value,
            hint,
            disabled_hint,
            on_changed,
            on_tap: template.on_tap.clone(),
            selected_item_builder: template.selected_item_builder.clone(),
            elevation: template.elevation,
            style: template.style.clone(),
            underline: None,
            icon: template.icon.clone(),
            icon_disabled_color: template.icon_disabled_color,
            icon_enabled_color: template.icon_enabled_color,
            icon_size: template.icon_size,
            is_dense: template.is_dense,
            is_expanded: template.is_expanded,
            item_height: template.item_height,
            menu_width: None,
            focus_color: template.focus_color,
            focus_node: template.focus_node.clone(),
            autofocus: template.autofocus,
            dropdown_color: template.dropdown_color,
            menu_max_height: template.menu_max_height,
            enable_feedback: template.enable_feedback,
            alignment: template.alignment,
            border_radius: template.border_radius,
            padding: template.padding,
            barrier_dismissible: template.barrier_dismissible,
            mouse_cursor: template.mouse_cursor.clone(),
            dropdown_menu_item_mouse_cursor: template.dropdown_menu_item_mouse_cursor.clone(),
            is_vertically_expanded: template.is_vertically_expanded,
            input_decoration: Some(input_decoration),
            is_empty,
        }
    }

    /// Dart `DropdownButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> DropdownButton<T> {
        self.key = Some(key);
        self
    }

    /// Dart `DropdownButton(selectedItemBuilder:)`.
    pub fn selected_item_builder(
        mut self,
        selected_item_builder: impl Fn(&mut App, BuildContext) -> Vec<WidgetRef> + 'static,
    ) -> DropdownButton<T> {
        self.selected_item_builder = Some(Rc::new(selected_item_builder));
        self
    }

    /// Dart `DropdownButton(value:)`.
    pub fn value(mut self, value: Option<T>) -> DropdownButton<T> {
        self.value = value;
        self
    }

    /// Dart `DropdownButton(hint:)`.
    pub fn hint<K>(mut self, hint: impl IntoWidget<K>) -> DropdownButton<T> {
        self.hint = Some(hint.into_widget());
        self
    }

    /// Dart `DropdownButton(disabledHint:)`.
    pub fn disabled_hint<K>(mut self, disabled_hint: impl IntoWidget<K>) -> DropdownButton<T> {
        self.disabled_hint = Some(disabled_hint.into_widget());
        self
    }

    /// Dart `DropdownButton(onTap:)`.
    pub fn on_tap(mut self, on_tap: Listener) -> DropdownButton<T> {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `DropdownButton(elevation:)`.
    pub fn elevation(mut self, elevation: i32) -> DropdownButton<T> {
        self.elevation = elevation;
        self
    }

    /// Dart `DropdownButton(style:)`.
    pub fn style(mut self, style: TextStyle) -> DropdownButton<T> {
        self.style = Some(style);
        self
    }

    /// Dart `DropdownButton(underline:)`.
    pub fn underline<K>(mut self, underline: impl IntoWidget<K>) -> DropdownButton<T> {
        self.underline = Some(underline.into_widget());
        self
    }

    /// Dart `DropdownButton(icon:)`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> DropdownButton<T> {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Dart `DropdownButton(iconDisabledColor:)`.
    pub fn icon_disabled_color(mut self, icon_disabled_color: Color) -> DropdownButton<T> {
        self.icon_disabled_color = Some(icon_disabled_color);
        self
    }

    /// Dart `DropdownButton(iconEnabledColor:)`.
    pub fn icon_enabled_color(mut self, icon_enabled_color: Color) -> DropdownButton<T> {
        self.icon_enabled_color = Some(icon_enabled_color);
        self
    }

    /// Dart `DropdownButton(iconSize:)`.
    pub fn icon_size(mut self, icon_size: f64) -> DropdownButton<T> {
        self.icon_size = icon_size;
        self
    }

    /// Dart `DropdownButton(isDense:)`.
    pub fn is_dense(mut self, is_dense: bool) -> DropdownButton<T> {
        self.is_dense = is_dense;
        self
    }

    /// Dart `DropdownButton(isExpanded:)`.
    pub fn is_expanded(mut self, is_expanded: bool) -> DropdownButton<T> {
        self.is_expanded = is_expanded;
        self
    }

    /// Dart `DropdownButton(itemHeight:)`; `None` lets each item take its intrinsic height.
    pub fn item_height(mut self, item_height: Option<f64>) -> DropdownButton<T> {
        debug_assert!(
            item_height.is_none_or(|item_height| item_height >= K_MIN_INTERACTIVE_DIMENSION)
        );
        self.item_height = item_height;
        self
    }

    /// Dart `DropdownButton(menuWidth:)`.
    pub fn menu_width(mut self, menu_width: f64) -> DropdownButton<T> {
        self.menu_width = Some(menu_width);
        self
    }

    /// Dart `DropdownButton(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> DropdownButton<T> {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `DropdownButton(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> DropdownButton<T> {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `DropdownButton(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> DropdownButton<T> {
        self.autofocus = autofocus;
        self
    }

    /// Dart `DropdownButton(dropdownColor:)`.
    pub fn dropdown_color(mut self, dropdown_color: Color) -> DropdownButton<T> {
        self.dropdown_color = Some(dropdown_color);
        self
    }

    /// Dart `DropdownButton(menuMaxHeight:)`.
    pub fn menu_max_height(mut self, menu_max_height: f64) -> DropdownButton<T> {
        self.menu_max_height = Some(menu_max_height);
        self
    }

    /// Dart `DropdownButton(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> DropdownButton<T> {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    /// Dart `DropdownButton(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> DropdownButton<T> {
        self.alignment = alignment;
        self
    }

    /// Dart `DropdownButton(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> DropdownButton<T> {
        self.border_radius = Some(border_radius);
        self
    }

    /// Dart `DropdownButton(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> DropdownButton<T> {
        self.padding = Some(padding);
        self
    }

    /// Dart `DropdownButton(barrierDismissible:)`.
    pub fn barrier_dismissible(mut self, barrier_dismissible: bool) -> DropdownButton<T> {
        self.barrier_dismissible = barrier_dismissible;
        self
    }

    /// Dart `DropdownButton(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> DropdownButton<T> {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `DropdownButton(dropdownMenuItemMouseCursor:)`.
    pub fn dropdown_menu_item_mouse_cursor(
        mut self,
        dropdown_menu_item_mouse_cursor: MouseCursorRef,
    ) -> DropdownButton<T> {
        self.dropdown_menu_item_mouse_cursor = Some(dropdown_menu_item_mouse_cursor);
        self
    }

    /// Dart's constructor assert across `items` and `value`.
    fn debug_check_constructor(&self) {
        let message = if self.input_decoration.is_some() {
            "There should be exactly one item with [DropdownButtonFormField]'s value"
        } else {
            "There should be exactly one item with [DropdownButton]'s value"
        };
        debug_assert!(
            self.items.as_ref().is_none_or(|items| items.is_empty()
                || self.value.is_none()
                || items.iter().filter(|item| item.value == self.value).count() == 1),
            "{message}. \nEither zero or 2 or more [DropdownMenuItem]s were detected with the same value"
        );
    }
}

impl<T: 'static> Debug for DropdownButton<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownButton")
            .field("elevation", &self.elevation)
            .field("icon_size", &self.icon_size)
            .field("is_dense", &self.is_dense)
            .field("is_expanded", &self.is_expanded)
            .field("item_height", &self.item_height)
            .field("menu_width", &self.menu_width)
            .field("alignment", &self.alignment)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> StatefulWidget for DropdownButton<T> {
    type State = DropdownButtonState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        DropdownButton::debug_check_constructor(self);
    }

    fn create_state(&self) -> DropdownButtonState<T> {
        DropdownButtonState {
            state: StateData::new(),
            selected_index: None,
            dropdown_route: None,
            last_orientation: None,
            internal_node: None,
            action_map: HashMap::new(),
            is_hovering: false,
            has_primary_focus: false,
            is_menu_expanded: false,
        }
    }
}

/// Dart's `_DropdownButtonState<T>`.
pub struct DropdownButtonState<T: Clone + PartialEq + Debug + 'static> {
    state: StateData<DropdownButton<T>>,
    selected_index: Option<usize>,
    dropdown_route: Option<Ref<DropdownRoute<T>>>,
    last_orientation: Option<Orientation>,
    internal_node: Option<Ref<FocusNode>>,
    action_map: HashMap<TypeId, ActionRef>,
    is_hovering: bool,
    has_primary_focus: bool,
    is_menu_expanded: bool,
}

impl<T: Clone + PartialEq + Debug + 'static> Debug for DropdownButtonState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownButtonState")
            .field("selected_index", &self.selected_index)
            .field("is_hovering", &self.is_hovering)
            .field("has_primary_focus", &self.has_primary_focus)
            .field("is_menu_expanded", &self.is_menu_expanded)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> DropdownButtonState<T> {
    fn focus_node(self: Handle<Self>, app: &App) -> FocusNodeRef {
        self.widget(app)
            .focus_node
            .clone()
            .or_else(|| app.get(self).internal_node.as_ref().map(Ref::erase))
            .expect("an internal node exists without a widget's node")
    }

    // Only used if needed to create _internalNode.
    fn create_focus_node(self: Handle<Self>, app: &mut App) -> Ref<FocusNode> {
        let node = FocusNode::new(app);
        let debug_label = waft_widgets::short_type_name::<DropdownButton<T>>().to_owned();
        node.set_debug_label(app, Some(debug_label));
        node
    }

    fn handle_focus_changed(self: Handle<Self>, app: &mut App) {
        let has_primary_focus = self.focus_node(app).has_primary_focus(app);
        if app.get(self).has_primary_focus != has_primary_focus {
            self.set_state(app, |state| {
                state.has_primary_focus = has_primary_focus;
            });
        }
    }

    fn remove_dropdown_route(self: Handle<Self>, app: &mut App) {
        if let Some(route) = app.get(self).dropdown_route.handle() {
            route.dismiss(app);
        }
        let state = app.get_mut(self);
        state.dropdown_route = None;
        state.last_orientation = None;
    }

    fn update_selected_index(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let Some(items) = widget.items.as_ref() else {
            app.get_mut(self).selected_index = None;
            return;
        };
        if items.is_empty()
            || (widget.value.is_none()
                && !items
                    .iter()
                    .any(|item| item.enabled && item.value == widget.value))
        {
            app.get_mut(self).selected_index = None;
            return;
        }

        debug_assert_eq!(
            items
                .iter()
                .filter(|item| item.value == widget.value)
                .count(),
            1
        );
        let selected_index = items.iter().position(|item| item.value == widget.value);
        if let Some(selected_index) = selected_index {
            app.get_mut(self).selected_index = Some(selected_index);
        }
    }

    fn text_style(self: Handle<Self>, app: &mut App) -> Option<TextStyle> {
        let context = self.context(app);
        self.widget(app)
            .style
            .clone()
            .or_else(|| Theme::of(app, context).text_theme.title_medium.clone())
    }

    fn handle_tap(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let text_direction = Directionality::maybe_of(app, context);
        let menu_margin: EdgeInsetsGeometry = if ButtonTheme::of(app, context).aligned_dropdown {
            K_ALIGNED_MENU_MARGIN.into()
        } else {
            K_UNALIGNED_MENU_MARGIN
        };

        let items = self
            .widget(app)
            .items
            .clone()
            .expect("a tappable button has items");
        let menu_items: Vec<WidgetRef> = items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                MenuItem::new(
                    Rc::new(move |app: &mut App, size: Size| {
                        // If [_dropdownRoute] is null and onLayout is called, this means
                        // that performLayout was called on a _DropdownRoute that has not
                        // left the widget tree but is already on its way out.
                        //
                        // Since onLayout is used primarily to collect the desired heights
                        // of each menu item before laying them out, not having the _DropdownRoute
                        // collect each item's height to lay out is fine since the route is
                        // already on its way out.
                        if !app.contains(self) {
                            return;
                        }
                        let Some(route) = app.get(self).dropdown_route.handle() else {
                            return;
                        };

                        app.get(route).menu.item_heights.borrow_mut()[index] = size.height();
                    }),
                    item,
                )
                .into_widget()
            })
            .collect();

        let navigator = Navigator::of(app, context, false);
        debug_assert!(app.get(self).dropdown_route.is_none());
        let item_box = context
            .find_render_object(app)
            .and_then(|object| object.as_box())
            .expect("a dropdown button has a render box");
        let navigator_context = navigator.context(app);
        let navigator_object = navigator_context.find_render_object(app);
        let item_rect =
            item_box.local_to_global(app, Offset::ZERO, navigator_object) & item_box.size(app);
        let captured_themes = <dyn InheritedTheme>::capture(app, context, Some(navigator_context));
        let style = self
            .text_style(app)
            .expect("the theme has a title medium style");
        let barrier_label =
            <dyn MaterialLocalizations>::of(app, context).modal_barrier_dismiss_label();
        let widget = self.widget(app);
        let route = DropdownRoute::new(
            app,
            DropdownRouteArguments {
                items: menu_items,
                button_rect: menu_margin.resolve(text_direction).inflate_rect(item_rect),
                padding: K_MENU_ITEM_PADDING.into(),
                selected_index: app.get(self).selected_index.unwrap_or(0),
                elevation: widget.elevation,
                captured_themes,
                style,
                barrier_label: Some(barrier_label),
                item_height: widget.item_height,
                menu_width: widget.menu_width,
                dropdown_color: widget.dropdown_color,
                menu_max_height: widget.menu_max_height,
                enable_feedback: widget.enable_feedback.unwrap_or(true),
                border_radius: widget.border_radius,
                barrier_dismissible: widget.barrier_dismissible,
                dropdown_menu_item_mouse_cursor: widget.dropdown_menu_item_mouse_cursor.clone(),
                value_type: PhantomData,
            },
        );
        app.get_mut(self).dropdown_route = Some(route.clone());

        self.focus_node(app).request_focus(app, None);
        let popped = navigator.push(app, route.into_erased());
        drop(app.spawn(async move |cx| {
            let new_value = popped.await;
            cx.update(|app| {
                if !self.mounted(app) {
                    return;
                }
                self.remove_dropdown_route(app);
                self.set_state(app, |state| {
                    state.is_menu_expanded = false;
                });
                let Some(new_value) = new_value.and_then(|new_value| {
                    new_value.downcast_ref::<DropdownRouteResult<T>>().cloned()
                }) else {
                    return;
                };
                if let Some(on_changed) = self.widget(app).on_changed.clone() {
                    on_changed(app, new_value.result);
                }
            });
        }));

        if let Some(on_tap) = self.widget(app).on_tap.clone() {
            on_tap.call(app);
        }
        self.set_state(app, |state| {
            state.is_menu_expanded = true;
        });
    }

    // When isDense is true, reduce the height of this button from _kMenuItemHeight to
    // _kDenseButtonHeight, but don't make it smaller than the text that it contains.
    // Similarly, we don't reduce the height of the button so much that its icon
    // would be clipped.
    fn dense_button_height(self: Handle<Self>, app: &mut App) -> f64 {
        let context = self.context(app);
        let text_style = self.text_style(app).expect("a text style");
        let title_medium = Theme::of(app, context)
            .text_theme
            .title_medium
            .clone()
            .expect("the theme has a title medium style");
        let font_size = text_style.font_size.unwrap_or_else(|| {
            title_medium
                .font_size
                .expect("title medium has a font size")
        });
        let line_height = text_style.height.or(title_medium.height).unwrap_or(1.0);
        let scaled_font_size =
            MediaQuery::text_scaler_of(app, context).scale(font_size * line_height);
        scaled_font_size.max(self.widget(app).icon_size.max(K_DENSE_BUTTON_HEIGHT))
    }

    fn icon_color(self: Handle<Self>, app: &mut App) -> Color {
        // These colors are not defined in the Material Design spec.
        let context = self.context(app);
        let brightness = Theme::brightness_of(app, context);
        let enabled = self.enabled(app);
        let widget = self.widget(app);
        if enabled {
            widget.icon_enabled_color.unwrap_or(match brightness {
                Brightness::Light => Colors::GREY.shade700(),
                Brightness::Dark => Colors::WHITE70,
            })
        } else {
            widget.icon_disabled_color.unwrap_or(match brightness {
                Brightness::Light => Colors::GREY.shade400(),
                Brightness::Dark => Colors::WHITE10,
            })
        }
    }

    fn enabled(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.items.as_ref().is_some_and(|items| !items.is_empty()) && widget.on_changed.is_some()
    }

    fn get_orientation(app: &mut App, context: BuildContext) -> Orientation {
        let mut result = MediaQuery::maybe_orientation_of(app, context);
        if result.is_none() {
            // If there's no MediaQuery, then use the view aspect to determine
            // orientation.
            let size = View::of(app, context).metrics().physical_size;
            result = Some(if size[0] > size[1] {
                Orientation::Landscape
            } else {
                Orientation::Portrait
            });
        }
        result.expect("set above")
    }
}

impl<T: Clone + PartialEq + Debug + 'static> State for DropdownButtonState<T> {
    type Widget = DropdownButton<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.update_selected_index(app);
        if self.widget(app).focus_node.is_none() && app.get(self).internal_node.is_none() {
            let node = self.create_focus_node(app);
            app.get_mut(self).internal_node = Some(node);
        }
        let activate = CallbackAction::<ActivateIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &ActivateIntent| {
                self.handle_tap(app);
                None
            }),
        );
        let button_activate = CallbackAction::<ButtonActivateIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &ButtonActivateIntent| {
                self.handle_tap(app);
                None
            }),
        );
        let action_map = &mut app.get_mut(self).action_map;
        action_map.insert(TypeId::of::<ActivateIntent>(), activate.into_erased());
        action_map.insert(
            TypeId::of::<ButtonActivateIntent>(),
            button_activate.into_erased(),
        );
        self.focus_node(app).add_listener(
            app,
            Listener::handle_method(self, Self::handle_focus_changed),
        );
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.remove_dropdown_route(app);
        self.focus_node(app).remove_listener(
            app,
            &Listener::handle_method(self, Self::handle_focus_changed),
        );
        if let Some(internal_node) = app.get(self).internal_node.handle() {
            internal_node.dispose(app);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &DropdownButton<T>) {
        if self.widget(app).focus_node != old_widget.focus_node {
            if let Some(old_node) = old_widget.focus_node.clone() {
                old_node.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::handle_focus_changed),
                );
            }
            if let Some(internal_node) = app.get(self).internal_node.handle()
                && self.widget(app).focus_node.is_some()
            {
                internal_node.dispose(app);
                app.get_mut(self).internal_node = None;
            }

            if self.widget(app).focus_node.is_none() && app.get(self).internal_node.is_none() {
                let node = self.create_focus_node(app);
                app.get_mut(self).internal_node = Some(node);
            }
            let has_primary_focus = self.focus_node(app).has_primary_focus(app);
            app.get_mut(self).has_primary_focus = has_primary_focus;
            self.focus_node(app).add_listener(
                app,
                Listener::handle_method(self, Self::handle_focus_changed),
            );
        }
        self.update_selected_index(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(debug_check_has_material_localizations(app, context));
        let new_orientation = Self::get_orientation(app, context);
        if app.get(self).last_orientation.is_none() {
            app.get_mut(self).last_orientation = Some(new_orientation);
        }
        if Some(new_orientation) != app.get(self).last_orientation {
            self.remove_dropdown_route(app);
            app.get_mut(self).last_orientation = Some(new_orientation);
        }

        // The width of the button and the menu are defined by the widest
        // item and the width of the hint.
        // We should explicitly type the items list to be a list of <Widget>,
        // otherwise, no explicit type adding items maybe trigger a crash/failure
        // when hint and selectedItemBuilder are provided.
        let mut items: Vec<WidgetRef> = match self.widget(app).selected_item_builder.clone() {
            Some(selected_item_builder) => {
                let selected_items = selected_item_builder(app, context);
                let widget = self.widget(app);
                debug_assert!(
                    widget
                        .items
                        .as_ref()
                        .is_none_or(|items| selected_items.len() == items.len()),
                    "The selectedItemBuilder must return a list of widgets with the same length as the items list.\n\
                     Currently, selectedItemBuilder returns a list of length {}, but items has length {}.",
                    selected_items.len(),
                    widget.items.as_ref().map_or(0, Vec::len),
                );
                selected_items
            }
            None => match &self.widget(app).items {
                Some(items) => items
                    .iter()
                    .map(|item| item.clone().into_widget())
                    .collect(),
                None => Vec::new(),
            },
        };

        let enabled = self.enabled(app);
        let text_style = self.text_style(app).expect("a text style");
        let theme = Theme::of(app, context);
        let mut hint_index = None;
        let widget = self.widget(app);
        if widget.hint.is_some() || (!enabled && widget.disabled_hint.is_some()) {
            let displayed_hint = if enabled {
                widget.hint.clone().expect("checked above")
            } else {
                widget
                    .disabled_hint
                    .clone()
                    .or_else(|| widget.hint.clone())
                    .expect("checked above")
            };

            hint_index = Some(items.len());
            items.push(
                DefaultTextStyle::new(
                    text_style.copy_with().color(theme.hint_color),
                    IgnorePointer::new().child(
                        DropdownMenuItemContainer::new(displayed_hint).alignment(widget.alignment),
                    ),
                )
                .into_widget(),
            );
        }

        let padding: EdgeInsetsGeometry = if ButtonTheme::of(app, context).aligned_dropdown
            && self.widget(app).input_decoration.is_none()
        {
            K_ALIGNED_BUTTON_PADDING
        } else {
            K_UNALIGNED_BUTTON_PADDING.into()
        };

        // If value is null (then _selectedIndex is null) then we
        // display the hint or nothing at all.
        let widget = self.widget(app);
        let inner_items_widget: WidgetRef = if items.is_empty() {
            SizedBox::shrink().into_widget()
        } else {
            let children: Vec<WidgetRef> = if widget.is_dense {
                items
            } else {
                items
                    .into_iter()
                    .map(|item| match widget.item_height {
                        Some(item_height) => SizedBox::new()
                            .height(item_height)
                            .child(item)
                            .into_widget(),
                        None => Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .children(vec![item])
                            .into_widget(),
                    })
                    .collect()
            };
            IndexedStack::new()
                .index(
                    app.get(self)
                        .selected_index
                        .or(hint_index)
                        .map(|index| index as i32),
                )
                .alignment(widget.alignment)
                .children(children)
                .into_widget()
        };

        let icon_color = self.icon_color(app);
        let widget = self.widget(app);
        let default_icon = Icon::new(Some(Icons::arrow_drop_down())).into_widget();
        let effective_suffix_icon = IconTheme::new(
            IconThemeData::new()
                .color(icon_color)
                .size(widget.icon_size),
            widget
                .icon
                .clone()
                .or_else(|| {
                    widget
                        .input_decoration
                        .as_ref()
                        .and_then(|decoration| decoration.suffix_icon.clone())
                })
                .unwrap_or(default_icon),
        )
        .into_widget();

        let dense_height = if widget.is_dense {
            Some(self.dense_button_height(app))
        } else {
            None
        };
        let widget = self.widget(app);
        let mut row_children = vec![if widget.is_expanded {
            Expanded::new(inner_items_widget).into_widget()
        } else {
            inner_items_widget
        }];
        if widget.input_decoration.is_none() {
            row_children.push(effective_suffix_icon.clone());
        }
        let mut sized = SizedBox::new();
        if let Some(dense_height) = dense_height {
            sized = sized.height(dense_height);
        }
        let text_direction = Directionality::of(app, context);
        let mut result: WidgetRef = DefaultTextStyle::new(
            if enabled {
                text_style.clone()
            } else {
                text_style.copy_with().color(theme.disabled_color)
            },
            sized.child(
                Padding::new(padding.resolve(Some(text_direction)).into()).child(
                    Row::new()
                        .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                        .main_axis_size(MainAxisSize::Min)
                        .children(row_children),
                ),
            ),
        )
        .into_widget();

        if !DropdownButtonHideUnderline::at(app, context) {
            let widget = self.widget(app);
            let bottom = if widget.is_dense || widget.item_height.is_none() {
                0.0
            } else {
                8.0
            };
            let underline = widget.underline.clone().unwrap_or_else(|| {
                Container::new()
                    .height(1.0)
                    .decoration(BoxDecoration::new().border(waft_painting::Border {
                        bottom: waft_painting::BorderSide {
                            color: Color::new(0xFFBDBDBD),
                            width: 0.0,
                            ..waft_painting::BorderSide::default()
                        },
                        ..waft_painting::Border::from_border_side(waft_painting::BorderSide::NONE)
                    }))
                    .into_widget()
            });
            result = Stack::new()
                .children(vec![
                    result,
                    Positioned::new(underline)
                        .left(0.0)
                        .right(0.0)
                        .bottom(bottom)
                        .into_widget(),
                ])
                .into_widget();
        }

        let mut states = WidgetStates::new();
        if !enabled {
            states.insert(WidgetState::Disabled);
        }
        let effective_mouse_cursor: MouseCursorRef = match self.widget(app).mouse_cursor.clone() {
            Some(mouse_cursor) => {
                <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&mouse_cursor, &states)
            }
            None => WidgetStateMouseCursor::adaptive_clickable().resolve(&states),
        };

        // When an InputDecoration is provided, use it instead of using an InkWell
        // that overflows in some cases (such as showing an errorText) and requires
        // additional logic to manage clipping properly.
        // A filled InputDecoration is able to fill the InputDecorator container
        // without overflowing. It also supports blending the hovered color.
        // According to the Material specification, the overlay colors should be
        // visible only for filled dropdown button, see:
        // https://m2.material.io/components/menus#dropdown-menu
        if let Some(input_decoration) = self.widget(app).input_decoration.clone() {
            let input_decoration_theme = InputDecorationTheme::of(app, context);
            let filled = input_decoration
                .filled
                .unwrap_or(input_decoration_theme.filled);
            let outlined = input_decoration
                .border
                .as_ref()
                .map(|border| border.is_outline())
                .or_else(|| {
                    input_decoration_theme
                        .border
                        .as_ref()
                        .map(|border| border.is_outline())
                })
                .unwrap_or(false);

            let suffix_icon_end_margin = if filled || outlined { 12.0 } else { 0.0 };
            let widget = self.widget(app);
            let mut effective_decoration = input_decoration
                .copy_with()
                // The decorator only uses its disabled colors and borders, and ignores
                // hovering, when the decoration is disabled, as for a disabled TextField.
                .enabled(enabled && input_decoration.enabled)
                // Override the suffix icon constraints to allow the
                // icon alignment to match the regular dropdown button.
                .suffix_icon_constraints(
                    BoxConstraints::new()
                        .min_width(widget.icon_size + suffix_icon_end_margin)
                        .min_height(widget.icon_size),
                )
                .suffix_icon(
                    Padding::new(EdgeInsetsGeometry::directional(
                        0.0,
                        0.0,
                        suffix_icon_end_margin,
                        0.0,
                    ))
                    .child(effective_suffix_icon),
                );
            if app.get(self).has_primary_focus {
                let focus_color = widget.focus_color.or(effective_decoration.focus_color);
                // For compatibility, override the fill color when focusColor is set.
                if let Some(focus_color) = focus_color {
                    effective_decoration = effective_decoration.copy_with().fill_color(focus_color);
                }
            }
            let state = app.get(self);
            let mut input_decorator = InputDecorator::new(effective_decoration)
                .is_empty(widget.is_empty)
                .is_focused(state.has_primary_focus)
                .is_hovering(state.is_hovering)
                .expands(widget.is_vertically_expanded);
            input_decorator = match widget.padding {
                None => input_decorator.child(result),
                Some(padding) => input_decorator.child(Padding::new(padding).child(result)),
            };
            let mut gesture_detector = GestureDetector::new()
                .behavior(waft_rendering::HitTestBehavior::Opaque)
                .child(input_decorator);
            if enabled {
                gesture_detector =
                    gesture_detector.on_tap(Listener::handle_method(self, Self::handle_tap));
            }
            result = Focus::new(
                waft_widgets::MouseRegion::new()
                    .on_enter(Rc::new(move |app: &mut App, _event| {
                        if !app.get(self).is_hovering {
                            self.set_state(app, |state| {
                                state.is_hovering = true;
                            });
                        }
                    }))
                    .on_exit(Rc::new(move |app: &mut App, _event| {
                        if app.get(self).is_hovering {
                            self.set_state(app, |state| {
                                state.is_hovering = false;
                            });
                        }
                    }))
                    .cursor(effective_mouse_cursor)
                    .child(gesture_detector),
            )
            .can_request_focus(enabled)
            .focus_node(self.focus_node(app))
            .autofocus(widget.autofocus)
            .into_widget();

            // When vertical expansion is disabled, wrap with Align
            // to prevent the widget from stretching to fill the parent's
            // available height.
            if !self.widget(app).is_vertically_expanded {
                result = waft_widgets::Align::new()
                    .alignment(AlignmentGeometry::TOP_START)
                    .height_factor(1.0)
                    .child(result)
                    .into_widget();
            }
        } else {
            let widget = self.widget(app);
            let mut ink_well = InkWell::new()
                .mouse_cursor(effective_mouse_cursor)
                .can_request_focus(enabled)
                .focus_node(self.focus_node(app))
                .autofocus(widget.autofocus)
                .focus_color(widget.focus_color.unwrap_or(theme.focus_color))
                .enable_feedback(false);
            if enabled {
                ink_well = ink_well.on_tap(Listener::handle_method(self, Self::handle_tap));
            }
            if let Some(border_radius) = widget.border_radius {
                ink_well = ink_well.border_radius(border_radius);
            }
            result = match widget.padding {
                None => ink_well.child(result),
                Some(padding) => ink_well.child(Padding::new(padding).child(result)),
            }
            .into_widget();
        }

        let state = app.get(self);
        let child_has_button_semantic = hint_index.is_some()
            || (state.selected_index.is_some() && self.widget(app).selected_item_builder.is_none());

        Semantics::new()
            .button(!child_has_button_semantic)
            .expanded(state.is_menu_expanded)
            .child(Actions::new(state.action_map.clone(), result))
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// DropdownButtonFormField

/// A `FormField` that contains a [`DropdownButton`].
///
/// This is a convenience widget that wraps a [`DropdownButton`] widget in a `FormField`.
///
/// A `Form` ancestor is not required. The `Form` allows one to save, reset, or validate
/// multiple fields at once. To use without a `Form`, pass a `GlobalKey` to the constructor and
/// use `GlobalKey::current_state` to save or reset the form field.
///
/// The `value` parameter maps to `FormField::initial_value`.
///
/// See also:
///
///  * [`DropdownButton`], which is the underlying text field without the `Form` integration.
pub struct DropdownButtonFormField<T: 'static> {
    form_field: FormFieldData<T>,
    /// Called when the user selects an item.
    ///
    /// This callback is invoked after the parent `Form`'s `Form::on_changed` callback. The
    /// field's updated value is available in the `Form::on_changed` callback via
    /// `FormFieldState::value`.
    ///
    /// If the constructor's `enabled` argument is true, the field is interactive even when
    /// [`on_changed`](Self::on_changed) is null.
    pub on_changed: Option<ValueChanged<Option<T>>>,
    /// The decoration to show around the dropdown button form field.
    ///
    /// By default, draws a horizontal line under the dropdown button field but can be
    /// configured to show an icon, label, hint text, and error text.
    ///
    /// If not specified, an [`InputDecorator`] with the `focus_color` set to the supplied
    /// `focus_color` (if any) will be used.
    pub decoration: InputDecoration,
    /// Determines whether tapping outside the dropdown will close it.
    ///
    /// Defaults to `true`.
    pub barrier_dismissible: bool,
    /// The cursor for a mouse pointer when it enters or is hovering over the dropdown button
    /// and its [`DropdownMenuItem`]s.
    ///
    /// If this property is null, [`WidgetStateMouseCursor::adaptive_clickable`] will be used.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// The cursor for a mouse pointer when it enters or is hovering over the this button's
    /// [`DropdownMenuItem`]s.
    ///
    /// If this property is null, [`WidgetStateMouseCursor::adaptive_clickable`] will be used.
    pub dropdown_menu_item_mouse_cursor: Option<MouseCursorRef>,
    /// Dart's constructor arguments `value` (deprecated) and `initialValue`, which fill the
    /// form field's [`initial_value`](FormFieldData::initial_value) as `initialValue ?? value`.
    value: Option<T>,
    initial_value: Option<T>,
    /// Dart's constructor argument `enabled`, which the builder closure reads; the form field's
    /// [`enabled`](FormFieldData::enabled) is `enabled ?? true`.
    enabled: Option<bool>,
    /// The constructor's other arguments, which Dart's builder closure captures and hands
    /// `DropdownButton._formField`: kept as that button, with this constructor's defaults.
    button: DropdownButton<T>,
}

impl<T: Clone + PartialEq + Debug + 'static> DropdownButtonFormField<T> {
    /// Creates a [`DropdownButton`] widget that is a `FormField`, wrapped in an
    /// [`InputDecorator`].
    ///
    /// The `enabled` argument controls whether the user can interact with the field:
    ///
    ///  * If true, the field is interactive even if [`on_changed`](Self::on_changed) is null.
    ///    The selected value is stored in the `FormFieldState`.
    ///  * If false, the field is not interactive and its own `autovalidate_mode` is ignored,
    ///    even if [`on_changed`](Self::on_changed) is non-null. `FormState::validate` and the
    ///    parent `Form`'s `Form::autovalidate_mode` still validate it.
    ///  * If null, the field is interactive if and only if [`on_changed`](Self::on_changed) is
    ///    non-null. In either case, it is still validated. This matches the behavior before
    ///    `enabled` was added.
    ///
    /// The resolved `FormField::enabled` is true unless `enabled` is explicitly false.
    ///
    /// For a description of the `on_saved`, `validator`, or `autovalidate_mode` parameters, see
    /// `FormField`. For the rest (other than [`decoration`](Self::decoration)), see
    /// [`DropdownButton`].
    pub fn new(items: Option<Vec<DropdownMenuItem<T>>>) -> DropdownButtonFormField<T> {
        let mut form_field = FormFieldData::new(Rc::new(DropdownButtonFormField::build_field));
        form_field.autovalidate_mode = AutovalidateMode::Disabled;
        form_field.enabled = true;
        let mut button = DropdownButton::new(items, None)
            .is_dense(true)
            .item_height(None);
        button.is_vertically_expanded = false;
        DropdownButtonFormField {
            form_field,
            on_changed: None,
            decoration: InputDecoration::new(),
            barrier_dismissible: true,
            mouse_cursor: None,
            dropdown_menu_item_mouse_cursor: None,
            value: None,
            initial_value: None,
            enabled: None,
            button,
        }
    }

    /// Dart `DropdownButtonFormField(key:)`.
    pub fn key(mut self, key: KeyRef) -> DropdownButtonFormField<T> {
        self.form_field.key = Some(key);
        self
    }

    /// Dart `DropdownButtonFormField(selectedItemBuilder:)`.
    pub fn selected_item_builder(
        mut self,
        selected_item_builder: impl Fn(&mut App, BuildContext) -> Vec<WidgetRef> + 'static,
    ) -> DropdownButtonFormField<T> {
        self.button = self.button.selected_item_builder(selected_item_builder);
        self
    }

    /// Dart `DropdownButtonFormField(value:)`, deprecated: use
    /// [`initial_value`](Self::initial_value) instead. This will set the initial value for the
    /// form field.
    #[deprecated(note = "Use initial_value instead.")]
    pub fn value(mut self, value: Option<T>) -> DropdownButtonFormField<T> {
        self.value = value;
        self.form_field.initial_value = self.initial_value.clone().or_else(|| self.value.clone());
        self
    }

    /// Dart `DropdownButtonFormField(initialValue:)`.
    pub fn initial_value(mut self, initial_value: Option<T>) -> DropdownButtonFormField<T> {
        self.initial_value = initial_value;
        self.form_field.initial_value = self.initial_value.clone().or_else(|| self.value.clone());
        self
    }

    /// Dart `DropdownButtonFormField(hint:)`.
    pub fn hint<K>(mut self, hint: impl IntoWidget<K>) -> DropdownButtonFormField<T> {
        self.button = self.button.hint(hint);
        self
    }

    /// Dart `DropdownButtonFormField(disabledHint:)`.
    pub fn disabled_hint<K>(
        mut self,
        disabled_hint: impl IntoWidget<K>,
    ) -> DropdownButtonFormField<T> {
        self.button = self.button.disabled_hint(disabled_hint);
        self
    }

    /// Dart `DropdownButtonFormField(onChanged:)`.
    pub fn on_changed(
        mut self,
        on_changed: impl Fn(&mut App, Option<T>) + 'static,
    ) -> DropdownButtonFormField<T> {
        self.on_changed = Some(Rc::new(on_changed));
        self
    }

    /// Dart `DropdownButtonFormField(onTap:)`.
    pub fn on_tap(mut self, on_tap: Listener) -> DropdownButtonFormField<T> {
        self.button = self.button.on_tap(on_tap);
        self
    }

    /// Dart `DropdownButtonFormField(elevation:)`.
    pub fn elevation(mut self, elevation: i32) -> DropdownButtonFormField<T> {
        self.button = self.button.elevation(elevation);
        self
    }

    /// Dart `DropdownButtonFormField(style:)`.
    pub fn style(mut self, style: TextStyle) -> DropdownButtonFormField<T> {
        self.button = self.button.style(style);
        self
    }

    /// Dart `DropdownButtonFormField(icon:)`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> DropdownButtonFormField<T> {
        self.button = self.button.icon(icon);
        self
    }

    /// Dart `DropdownButtonFormField(iconDisabledColor:)`.
    pub fn icon_disabled_color(mut self, icon_disabled_color: Color) -> DropdownButtonFormField<T> {
        self.button = self.button.icon_disabled_color(icon_disabled_color);
        self
    }

    /// Dart `DropdownButtonFormField(iconEnabledColor:)`.
    pub fn icon_enabled_color(mut self, icon_enabled_color: Color) -> DropdownButtonFormField<T> {
        self.button = self.button.icon_enabled_color(icon_enabled_color);
        self
    }

    /// Dart `DropdownButtonFormField(iconSize:)`.
    pub fn icon_size(mut self, icon_size: f64) -> DropdownButtonFormField<T> {
        self.button = self.button.icon_size(icon_size);
        self
    }

    /// Dart `DropdownButtonFormField(isDense:)`.
    pub fn is_dense(mut self, is_dense: bool) -> DropdownButtonFormField<T> {
        self.button = self.button.is_dense(is_dense);
        self
    }

    /// Dart `DropdownButtonFormField(isExpanded:)`.
    pub fn is_expanded(mut self, is_expanded: bool) -> DropdownButtonFormField<T> {
        self.button = self.button.is_expanded(is_expanded);
        self
    }

    /// Dart `DropdownButtonFormField(itemHeight:)`.
    pub fn item_height(mut self, item_height: Option<f64>) -> DropdownButtonFormField<T> {
        self.button = self.button.item_height(item_height);
        self
    }

    /// Dart `DropdownButtonFormField(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> DropdownButtonFormField<T> {
        self.button = self.button.focus_color(focus_color);
        self
    }

    /// Dart `DropdownButtonFormField(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> DropdownButtonFormField<T> {
        self.button = self.button.focus_node(focus_node);
        self
    }

    /// Dart `DropdownButtonFormField(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> DropdownButtonFormField<T> {
        self.button = self.button.autofocus(autofocus);
        self
    }

    /// Dart `DropdownButtonFormField(dropdownColor:)`.
    pub fn dropdown_color(mut self, dropdown_color: Color) -> DropdownButtonFormField<T> {
        self.button = self.button.dropdown_color(dropdown_color);
        self
    }

    /// Dart `DropdownButtonFormField(decoration:)`.
    pub fn decoration(mut self, decoration: InputDecoration) -> DropdownButtonFormField<T> {
        self.decoration = decoration;
        self
    }

    /// Dart `DropdownButtonFormField(onSaved:)`.
    pub fn on_saved(
        mut self,
        on_saved: impl Fn(&mut App, Option<T>) + 'static,
    ) -> DropdownButtonFormField<T> {
        self.form_field.on_saved = Some(Rc::new(on_saved));
        self
    }

    /// Dart `DropdownButtonFormField(validator:)`.
    pub fn validator(
        mut self,
        validator: impl Fn(&mut App, Option<&T>) -> Option<String> + 'static,
    ) -> DropdownButtonFormField<T> {
        self.form_field.validator = Some(Rc::new(validator));
        self
    }

    /// Dart `DropdownButtonFormField(errorBuilder:)`.
    pub fn error_builder(
        mut self,
        error_builder: impl Fn(&mut App, BuildContext, String) -> WidgetRef + 'static,
    ) -> DropdownButtonFormField<T> {
        self.form_field.error_builder = Some(Rc::new(error_builder));
        self
    }

    /// Dart `DropdownButtonFormField(forceErrorText:)`.
    pub fn force_error_text(
        mut self,
        force_error_text: impl Into<String>,
    ) -> DropdownButtonFormField<T> {
        self.form_field.force_error_text = Some(force_error_text.into());
        self
    }

    /// Dart `DropdownButtonFormField(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> DropdownButtonFormField<T> {
        self.enabled = Some(enabled);
        self.form_field.enabled = enabled;
        self
    }

    /// Dart `DropdownButtonFormField(autovalidateMode:)`.
    pub fn autovalidate_mode(
        mut self,
        autovalidate_mode: AutovalidateMode,
    ) -> DropdownButtonFormField<T> {
        self.form_field.autovalidate_mode = autovalidate_mode;
        self
    }

    /// Dart `DropdownButtonFormField(menuMaxHeight:)`.
    pub fn menu_max_height(mut self, menu_max_height: f64) -> DropdownButtonFormField<T> {
        self.button = self.button.menu_max_height(menu_max_height);
        self
    }

    /// Dart `DropdownButtonFormField(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> DropdownButtonFormField<T> {
        self.button = self.button.enable_feedback(enable_feedback);
        self
    }

    /// Dart `DropdownButtonFormField(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> DropdownButtonFormField<T> {
        self.button = self.button.alignment(alignment);
        self
    }

    /// Dart `DropdownButtonFormField(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> DropdownButtonFormField<T> {
        self.button = self.button.border_radius(border_radius);
        self
    }

    /// Dart `DropdownButtonFormField(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> DropdownButtonFormField<T> {
        self.button = self.button.padding(padding);
        self
    }

    /// Dart `DropdownButtonFormField(barrierDismissible:)`.
    pub fn barrier_dismissible(mut self, barrier_dismissible: bool) -> DropdownButtonFormField<T> {
        self.barrier_dismissible = barrier_dismissible;
        self.button = self.button.barrier_dismissible(barrier_dismissible);
        self
    }

    /// Dart `DropdownButtonFormField(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> DropdownButtonFormField<T> {
        self.mouse_cursor = Some(mouse_cursor.clone());
        self.button = self.button.mouse_cursor(mouse_cursor);
        self
    }

    /// Dart `DropdownButtonFormField(dropdownMenuItemMouseCursor:)`.
    pub fn dropdown_menu_item_mouse_cursor(
        mut self,
        dropdown_menu_item_mouse_cursor: MouseCursorRef,
    ) -> DropdownButtonFormField<T> {
        self.dropdown_menu_item_mouse_cursor = Some(dropdown_menu_item_mouse_cursor.clone());
        self.button = self
            .button
            .dropdown_menu_item_mouse_cursor(dropdown_menu_item_mouse_cursor);
        self
    }

    /// Dart `DropdownButtonFormField(isVerticallyExpanded:)`.
    pub fn is_vertically_expanded(
        mut self,
        is_vertically_expanded: bool,
    ) -> DropdownButtonFormField<T> {
        self.button.is_vertically_expanded = is_vertically_expanded;
        self
    }

    /// Dart's builder closure: the [`DropdownButton`] for the form field's current state.
    fn build_field(app: &mut App, field: AnyFormFieldState<T>) -> WidgetRef {
        let state = field
            .downcast::<DropdownButtonFormFieldState<T>>(app)
            .expect("a DropdownButtonFormField's state is a DropdownButtonFormFieldState");
        let context = field.context(app);
        let input_decoration_theme = InputDecorationTheme::of(app, context);
        let widget = state.widget(app);
        let mut effective_decoration = widget.decoration.apply_defaults(input_decoration_theme);

        let items = &widget.button.items;
        let value = field.value(app).cloned();
        let show_selected_item = items
            .as_ref()
            .is_some_and(|items| items.iter().any(|item| item.value == value));
        let is_dropdown_enabled = widget.enabled.unwrap_or(widget.on_changed.is_some())
            && items.as_ref().is_some_and(|items| !items.is_empty());
        // If decoration hintText is provided, use it as the default value for both hint and disabledHint.
        let decoration_hint = effective_decoration
            .hint_text
            .clone()
            .map(|hint_text| Text::new(hint_text).into_widget());
        let effective_hint = widget.button.hint.clone().or(decoration_hint);
        let effective_disabled_hint = widget
            .button
            .disabled_hint
            .clone()
            .or_else(|| effective_hint.clone());
        let is_hint_or_disabled_hint_available = if is_dropdown_enabled {
            effective_hint.is_some()
        } else {
            effective_hint.is_some() || effective_disabled_hint.is_some()
        };
        let is_empty = !show_selected_item && !is_hint_or_disabled_hint_available;

        let error_text = field.error_text(app);
        if error_text.is_some() || effective_decoration.hint_text.is_some() {
            let error_builder = state.widget(app).form_field.error_builder.clone();
            let error = match (&error_text, error_builder) {
                (Some(error_text), Some(error_builder)) => {
                    Some(error_builder(app, state.context(app), error_text.clone()))
                }
                _ => None,
            };
            let error_text = if error.is_none() { error_text } else { None };
            // Clear the decoration hintText because DropdownButton has its own hint logic.
            let hint_text = effective_decoration
                .hint_text
                .as_ref()
                .map(|_| String::new());

            let mut decoration = effective_decoration.copy_with();
            if let Some(error) = error {
                decoration = decoration.error(error);
            }
            if let Some(error_text) = error_text {
                decoration = decoration.error_text(error_text);
            }
            if let Some(hint_text) = hint_text {
                decoration = decoration.hint_text(hint_text);
            }
            effective_decoration = decoration;
        }

        let on_changed: Option<ValueChanged<Option<T>>> = if is_dropdown_enabled {
            Some(Rc::new(move |app: &mut App, value: Option<T>| {
                field.did_change(app, value);
            }))
        } else {
            None
        };
        let widget = state.widget(app);
        // An unfocusable Focus widget so that this widget can detect if its
        // descendants have focus or not.
        Focus::new(DropdownButtonHideUnderline::new(
            DropdownButton::form_field(
                &widget.button,
                value,
                effective_hint,
                effective_disabled_hint,
                on_changed,
                effective_decoration,
                is_empty,
            ),
        ))
        .can_request_focus(false)
        .skip_traversal(true)
        .into_widget()
    }

    /// Dart's constructor asserts across fields.
    fn debug_check_constructor(&self) {
        let value = self.initial_value.clone().or_else(|| self.value.clone());
        debug_assert!(
            self.button
                .items
                .as_ref()
                .is_none_or(|items| items.is_empty()
                    || value.is_none()
                    || items.iter().filter(|item| item.value == value).count() == 1),
            "There should be exactly one item with [DropdownButton]'s value: {value:?}. \n\
             Either zero or 2 or more [DropdownMenuItem]s were detected with the same value"
        );
        debug_assert!(
            self.form_field.error_builder.is_none() || self.decoration.error_text.is_none(),
            "Declaring both errorBuilder and decoration.errorText is not supported."
        );
    }
}

impl<T: 'static> Debug for DropdownButtonFormField<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownButtonFormField")
            .field("button", &self.button)
            .field("barrier_dismissible", &self.barrier_dismissible)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> FormFieldLeaf for DropdownButtonFormField<T> {
    type Value = T;
    waft_widgets::form_field_accessors!();
}

impl<T: Clone + PartialEq + Debug + 'static> StatefulWidget for DropdownButtonFormField<T> {
    type State = DropdownButtonFormFieldState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.form_field.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        DropdownButtonFormField::debug_check_constructor(self);
    }

    fn create_state(&self) -> DropdownButtonFormFieldState<T> {
        DropdownButtonFormFieldState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
            form_field_state: FormFieldStateData::new(&self.form_field),
        }
    }
}

/// Dart's `_DropdownButtonFormFieldState<T>`.
pub struct DropdownButtonFormFieldState<T: Clone + PartialEq + Debug + 'static> {
    state: StateData<DropdownButtonFormField<T>>,
    restoration: RestorationMixinData,
    form_field_state: FormFieldStateData<T>,
}

impl<T: Clone + PartialEq + Debug + 'static> Debug for DropdownButtonFormFieldState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropdownButtonFormFieldState")
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> RestorationMixin for DropdownButtonFormFieldState<T> {
    waft_widgets::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        FormFieldState::restoration_id(self, app)
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        old_bucket: Option<Handle<waft_services::RestorationBucket>>,
        initial_restore: bool,
    ) {
        FormFieldState::restore_state(self, app, old_bucket, initial_restore);
    }
}

impl<T: Clone + PartialEq + Debug + 'static> FormFieldStateLeaf
    for DropdownButtonFormFieldState<T>
{
    waft_widgets::form_field_state_accessors!();

    fn did_change(self: Handle<Self>, app: &mut App, value: Option<T>) {
        FormFieldState::did_change(self, app, value.clone());
        if let Some(on_changed) = self.widget(app).on_changed.clone() {
            on_changed(app, value);
        }
    }

    fn reset(self: Handle<Self>, app: &mut App) {
        FormFieldState::reset(self, app);
        if let Some(on_changed) = self.widget(app).on_changed.clone() {
            let value = self.value(app).cloned();
            on_changed(app, value);
        }
    }
}

impl<T: Clone + PartialEq + Debug + 'static> State for DropdownButtonFormFieldState<T> {
    type Widget = DropdownButtonFormField<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        FormFieldState::init_state(self, app);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &DropdownButtonFormField<T>,
    ) {
        FormFieldState::did_update_widget(self, app, old_widget);
        let initial_value = self.widget(app).form_field.initial_value.clone();
        if old_widget.form_field.initial_value != initial_value {
            self.set_value(app, initial_value);
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        FormFieldState::did_change_dependencies(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        FormFieldState::deactivate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        FormFieldState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        FormFieldState::build(self, app, context)
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/dropdown_test.dart` and `test/dropdown_form_field_test.dart`. Text lays
    //! out in flutter_test's FlutterTest font, so sizes are Flutter's.

    use std::cell::{Cell, RefCell};

    use waft_embedder::{Locale, TargetPlatform};
    use waft_foundation::ValueKey;
    use waft_widgets::{
        AnyElement, Builder, Center, DefaultWidgetsLocalizations, Form, FormState, GlobalKey,
        Localizations, MediaQueryData, RepaintBoundary, RouteSettingsRef, Scrollable,
        StatefulBuilder,
    };

    use super::*;
    use crate::material_localizations::DefaultMaterialLocalizations;
    use crate::page::MaterialPageRoute;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, tester, tester_on};
    use waft_widgets::AnyFocusNode;

    const MENU_ITEMS: [&str; 4] = ["one", "two", "three", "four"];

    /// Dart's `moreOrLessEquals` on positions that went through a render object's transform:
    /// `Matrix4` is valo's f32 matrix, so they carry f32 rounding that Dart's f64 has not, and
    /// nothing more.
    fn assert_near(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= f64::from(f32::EPSILON) * expected.abs().max(1.0),
            "{actual} is not {expected}"
        );
    }

    fn key(text: &str) -> KeyRef {
        ValueKey::new(text.to_owned()).into()
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    fn numbers(count: usize) -> Vec<String> {
        (0..count).map(|index| index.to_string()).collect()
    }

    /// Dart's `void onChanged<T>(T _) {}`.
    fn on_changed() -> ValueChanged<Option<String>> {
        Rc::new(|_app, _value| {})
    }

    /// The arguments of Dart's `buildFrame` and `buildDropdown`, with their defaults.
    #[derive(Clone)]
    struct Frame {
        button_key: Option<GlobalKey>,
        initial_value: Option<String>,
        on_changed: Option<ValueChanged<Option<String>>>,
        on_tap: Option<Listener>,
        icon: Option<WidgetRef>,
        icon_disabled_color: Option<Color>,
        icon_enabled_color: Option<Color>,
        icon_size: f64,
        is_dense: bool,
        is_expanded: bool,
        hint: Option<WidgetRef>,
        disabled_hint: Option<WidgetRef>,
        underline: Option<WidgetRef>,
        items: Option<Vec<String>>,
        selected_item_builder: Option<DropdownButtonBuilder>,
        item_height: Option<f64>,
        menu_width: Option<f64>,
        alignment: AlignmentGeometry,
        text_direction: TextDirection,
        media_size: Option<Size>,
        focus_node: Option<FocusNodeRef>,
        autofocus: bool,
        focus_color: Option<Color>,
        dropdown_color: Option<Color>,
        is_form_field: bool,
        menu_max_height: Option<f64>,
        padding: Option<EdgeInsetsGeometry>,
        dropdown_alignment: AlignmentGeometry,
        use_material3: Option<bool>,
        decoration: Option<InputDecoration>,
    }

    fn frame() -> Frame {
        Frame {
            button_key: None,
            initial_value: Some("two".to_owned()),
            on_changed: None,
            on_tap: None,
            icon: None,
            icon_disabled_color: None,
            icon_enabled_color: None,
            icon_size: 24.0,
            is_dense: false,
            is_expanded: false,
            hint: None,
            disabled_hint: None,
            underline: None,
            items: Some(strings(&MENU_ITEMS)),
            selected_item_builder: None,
            item_height: Some(K_MIN_INTERACTIVE_DIMENSION),
            menu_width: None,
            alignment: AlignmentGeometry::CENTER_START,
            text_direction: TextDirection::Ltr,
            media_size: None,
            focus_node: None,
            autofocus: false,
            focus_color: None,
            dropdown_color: None,
            is_form_field: false,
            menu_max_height: None,
            padding: None,
            dropdown_alignment: AlignmentGeometry::CENTER,
            use_material3: None,
            decoration: None,
        }
    }

    impl Frame {
        /// Dart's `buildDropdown`.
        fn dropdown(&self) -> WidgetRef {
            let list_items = self.items.as_ref().map(|items| {
                items
                    .iter()
                    .map(|item| {
                        DropdownMenuItem::new(
                            Text::new(item.clone()).key(key(&format!("{item}Text"))),
                        )
                        .key(key(item))
                        .value(item.clone())
                    })
                    .collect::<Vec<_>>()
            });

            if self.is_form_field {
                let mut field = DropdownButtonFormField::new(list_items)
                    .initial_value(self.initial_value.clone())
                    .icon_size(self.icon_size)
                    .is_dense(self.is_dense)
                    .is_expanded(self.is_expanded)
                    .autofocus(self.autofocus)
                    .item_height(self.item_height)
                    .alignment(self.alignment);
                if let Some(button_key) = &self.button_key {
                    field = field.key(button_key.clone().into());
                }
                if let Some(hint) = &self.hint {
                    field = field.hint(hint.clone());
                }
                if let Some(disabled_hint) = &self.disabled_hint {
                    field = field.disabled_hint(disabled_hint.clone());
                }
                if let Some(on_changed) = self.on_changed.clone() {
                    field = field.on_changed(move |app, value| on_changed(app, value));
                }
                if let Some(on_tap) = self.on_tap.clone() {
                    field = field.on_tap(on_tap);
                }
                if let Some(icon) = &self.icon {
                    field = field.icon(icon.clone());
                }
                if let Some(color) = self.icon_disabled_color {
                    field = field.icon_disabled_color(color);
                }
                if let Some(color) = self.icon_enabled_color {
                    field = field.icon_enabled_color(color);
                }
                if let Some(focus_node) = self.focus_node.clone() {
                    field = field.focus_node(focus_node);
                }
                if let Some(color) = self.focus_color {
                    field = field.focus_color(color);
                }
                if let Some(color) = self.dropdown_color {
                    field = field.dropdown_color(color);
                }
                if let Some(builder) = self.selected_item_builder.clone() {
                    field = field.selected_item_builder(move |app, context| builder(app, context));
                }
                if let Some(menu_max_height) = self.menu_max_height {
                    field = field.menu_max_height(menu_max_height);
                }
                if let Some(padding) = self.padding {
                    field = field.padding(padding);
                }
                if let Some(decoration) = &self.decoration {
                    field = field.decoration(decoration.clone());
                }
                return Form::new(field).into_widget();
            }
            let mut button = DropdownButton::new(list_items, self.on_changed.clone())
                .value(self.initial_value.clone())
                .icon_size(self.icon_size)
                .is_dense(self.is_dense)
                .is_expanded(self.is_expanded)
                .autofocus(self.autofocus)
                .item_height(self.item_height)
                .alignment(self.alignment);
            if let Some(button_key) = &self.button_key {
                button = button.key(button_key.clone().into());
            }
            if let Some(hint) = &self.hint {
                button = button.hint(hint.clone());
            }
            if let Some(disabled_hint) = &self.disabled_hint {
                button = button.disabled_hint(disabled_hint.clone());
            }
            if let Some(on_tap) = self.on_tap.clone() {
                button = button.on_tap(on_tap);
            }
            if let Some(icon) = &self.icon {
                button = button.icon(icon.clone());
            }
            if let Some(color) = self.icon_disabled_color {
                button = button.icon_disabled_color(color);
            }
            if let Some(color) = self.icon_enabled_color {
                button = button.icon_enabled_color(color);
            }
            if let Some(underline) = &self.underline {
                button = button.underline(underline.clone());
            }
            if let Some(focus_node) = self.focus_node.clone() {
                button = button.focus_node(focus_node);
            }
            if let Some(color) = self.focus_color {
                button = button.focus_color(color);
            }
            if let Some(color) = self.dropdown_color {
                button = button.dropdown_color(color);
            }
            if let Some(builder) = self.selected_item_builder.clone() {
                button = button.selected_item_builder(move |app, context| builder(app, context));
            }
            if let Some(menu_width) = self.menu_width {
                button = button.menu_width(menu_width);
            }
            if let Some(menu_max_height) = self.menu_max_height {
                button = button.menu_max_height(menu_max_height);
            }
            if let Some(padding) = self.padding {
                button = button.padding(padding);
            }
            button.into_widget()
        }

        /// Dart's `buildFrame`.
        fn build(&self, app: &App) -> WidgetRef {
            let mut theme = ThemeData::builder();
            if let Some(use_material3) = self.use_material3 {
                theme = theme.use_material3(use_material3);
            }
            let dropdown = self.dropdown();
            let dropdown_alignment = self.dropdown_alignment;
            Theme::new(
                theme.build(app),
                test_app(self.text_direction, self.media_size, move || {
                    Material::new()
                        .child(
                            waft_widgets::Align::new()
                                .alignment(dropdown_alignment)
                                .child(
                                    RepaintBoundary::new()
                                        .child(InputDecorationTheme::new().child(dropdown.clone())),
                                ),
                        )
                        .into_widget()
                }),
            )
            .into_widget()
        }

        fn button_key(mut self, button_key: &GlobalKey) -> Frame {
            self.button_key = Some(button_key.clone());
            self
        }

        fn initial_value(mut self, initial_value: Option<&str>) -> Frame {
            self.initial_value = initial_value.map(str::to_owned);
            self
        }

        fn on_changed(mut self, on_changed: ValueChanged<Option<String>>) -> Frame {
            self.on_changed = Some(on_changed);
            self
        }

        fn items(mut self, items: Option<Vec<String>>) -> Frame {
            self.items = items;
            self
        }
    }

    /// Dart's `TestApp(textDirection:, mediaSize:, child:)`: a navigator whose one page is
    /// `child`, under the view's media query with `media_size` as its size.
    fn test_app(
        text_direction: TextDirection,
        media_size: Option<Size>,
        child: impl Fn() -> WidgetRef + 'static,
    ) -> WidgetRef {
        TestApp {
            text_direction,
            media_size,
            media_from_view: true,
            child: child(),
        }
        .into_widget()
    }

    /// dropdown_form_field_test.dart's `TestApp`, whose media query is `MediaQueryData()` with
    /// `media_size` as its size, not the view's.
    fn form_test_app(text_direction: TextDirection, child: WidgetRef) -> WidgetRef {
        TestApp {
            text_direction,
            media_size: None,
            media_from_view: false,
            child,
        }
        .into_widget()
    }

    /// Dart's `TestApp`: its page reads the current widget's `child`, so a new `TestApp`
    /// rebuilds the page with the new child.
    #[derive(Debug)]
    struct TestApp {
        text_direction: TextDirection,
        media_size: Option<Size>,
        media_from_view: bool,
        child: WidgetRef,
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

    impl Debug for TestAppState {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("TestAppState").finish_non_exhaustive()
        }
    }

    impl State for TestAppState {
        type Widget = TestApp;
        waft_widgets::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
            let view = View::of(app, context);
            let mut data = if self.widget(app).media_from_view {
                MediaQueryData::from_view(app, &*view, None)
            } else {
                MediaQueryData::new()
            };
            let widget = self.widget(app);
            if let Some(media_size) = widget.media_size {
                data = data.copy_with().size(media_size);
            }
            Localizations::new(
                Locale::new("en").country_code("US"),
                vec![
                    DefaultWidgetsLocalizations::delegate(),
                    DefaultMaterialLocalizations::delegate(),
                ],
            )
            .child(
                MediaQuery::new(
                    data,
                    Directionality::new(
                        widget.text_direction,
                        Navigator::new().on_generate_route(move |app, settings| {
                            let route = MaterialPageRoute::new(
                                app,
                                Rc::new(move |app, _context| self.widget(app).child.clone()),
                            );
                            route
                                .clone()
                                .settings(app, RouteSettingsRef::Settings(settings.clone()));
                            Some(route.erase())
                        }),
                    ),
                )
                .into_widget(),
            )
            .into_widget()
        }
    }

    fn pump_frame(tester: &mut WidgetTester, frame: &Frame) {
        let root = frame.build(&tester.cell.borrow());
        tester.pump_widget(root);
    }

    /// Dart's `find.byKey(key, skipOffstage: false)`.
    fn all_by_key(tester: &WidgetTester, key: &KeyRef) -> Vec<AnyElement> {
        let found = tester.elements();
        let app = tester.cell.borrow();
        found
            .into_iter()
            .filter(|element| element.widget(&app).key() == Some(key))
            .collect()
    }

    /// Dart's `tester.renderObjectList<RenderBox>(find.byKey(ValueKey(text)))`.
    fn boxes_of(tester: &WidgetTester, text: &str) -> Vec<AnyRenderBox> {
        all_by_key(tester, &key(text))
            .into_iter()
            .map(|element| tester.render_box_of(element))
            .collect()
    }

    fn only(found: Vec<AnyElement>) -> AnyElement {
        assert_eq!(found.len(), 1, "one widget found");
        found[0]
    }

    fn global_top_left(tester: &WidgetTester, render: AnyRenderBox) -> Offset {
        render.local_to_global(&tester.cell.borrow(), Offset::ZERO, None)
    }

    fn size_of(tester: &WidgetTester, render: AnyRenderBox) -> Size {
        render.size(&tester.cell.borrow())
    }

    fn rect_of(tester: &WidgetTester, render: AnyRenderBox) -> Rect {
        tester.global_rect(render)
    }

    fn key_box(tester: &WidgetTester, key: &GlobalKey) -> AnyRenderBox {
        tester.render_object(key).as_box().expect("a render box")
    }

    /// Dart's `tester.tap(finder)` on `render`.
    fn tap_box(tester: &mut WidgetTester, render: AnyRenderBox) {
        let center = rect_of(tester, render).center();
        tester.tap(center, 1);
    }

    /// Dart's `tester.tap(find.text(text).last)`.
    fn tap_last_text(tester: &mut WidgetTester, text: &str) {
        let found = tester.find_onstage_text(text);
        let element = *found.last().expect("the text is shown");
        let render = tester.render_box_of(element);
        tap_box(tester, render);
    }

    /// Dart's `tester.tap(find.text(text))` of the only one shown.
    fn tap_text(tester: &mut WidgetTester, text: &str) {
        let element = only(tester.find_onstage_text(text));
        let render = tester.render_box_of(element);
        tap_box(tester, render);
    }

    /// Dart's `tester.tap(find.text(text, skipOffstage: false), warnIfMissed: false)`: a tap
    /// on the first one, shown or not.
    fn tap_first_text_anywhere(tester: &mut WidgetTester, text: &str) {
        let element = tester.find_text(text)[0];
        let render = tester.render_box_of(element);
        tap_box(tester, render);
    }

    fn tap_key(tester: &mut WidgetTester, key: &GlobalKey) {
        let center = tester.center_of(key);
        tester.tap(center, 1);
    }

    /// `pump()` then `pump(1s)`: finish the menu animation.
    fn settle_menu(tester: &mut WidgetTester) {
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_secs(1));
    }

    /// Dart's `checkSelectedItemTextGeometry`: the two paragraphs of the selected item's text,
    /// in the button and in the menu, are aligned.
    fn check_selected_item_text_geometry(tester: &WidgetTester, value: &str) {
        let boxes = boxes_of(tester, &format!("{value}Text"));
        assert_eq!(boxes.len(), 2);
        assert_eq!(
            global_top_left(tester, boxes[0]),
            global_top_left(tester, boxes[1])
        );
        assert_eq!(size_of(tester, boxes[0]), size_of(tester, boxes[1]));
    }

    fn value_log() -> (Rc<RefCell<Option<String>>>, ValueChanged<Option<String>>) {
        let value = Rc::new(RefCell::new(None));
        let set = Rc::clone(&value);
        (
            value,
            Rc::new(move |_app, new_value| *set.borrow_mut() = new_value),
        )
    }

    #[test]
    fn dropdown_button_control_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (value, did_change_value) = value_log();
            *value.borrow_mut() = Some("one".to_owned());
            let build = |value: &Rc<RefCell<Option<String>>>| {
                frame()
                    .initial_value(value.borrow().as_deref())
                    .on_changed(Rc::clone(&did_change_value))
            };

            pump_frame(&mut tester, &build(&value));

            tap_text(&mut tester, "one");
            settle_menu(&mut tester);
            assert_eq!(value.borrow().as_deref(), Some("one"));

            tap_last_text(&mut tester, "three");
            settle_menu(&mut tester);
            assert_eq!(value.borrow().as_deref(), Some("three"));

            tap_first_text_anywhere(&mut tester, "three");
            settle_menu(&mut tester);
            assert_eq!(value.borrow().as_deref(), Some("three"));

            pump_frame(&mut tester, &build(&value));

            tap_last_text(&mut tester, "two");
            settle_menu(&mut tester);
            assert_eq!(value.borrow().as_deref(), Some("two"));
        });
    }

    impl Frame {
        fn text_direction(mut self, text_direction: TextDirection) -> Frame {
            self.text_direction = text_direction;
            self
        }

        fn use_material3(mut self, use_material3: bool) -> Frame {
            self.use_material3 = Some(use_material3);
            self
        }

        fn expanded(mut self, is_expanded: bool) -> Frame {
            self.is_expanded = is_expanded;
            self
        }

        fn dense(mut self, is_dense: bool) -> Frame {
            self.is_dense = is_dense;
            self
        }

        fn icon(mut self, icon: WidgetRef) -> Frame {
            self.icon = Some(icon);
            self
        }

        fn icon_size(mut self, icon_size: f64) -> Frame {
            self.icon_size = icon_size;
            self
        }

        fn icon_colors(mut self, enabled: Color, disabled: Color) -> Frame {
            self.icon_enabled_color = Some(enabled);
            self.icon_disabled_color = Some(disabled);
            self
        }

        fn hint(mut self, hint: WidgetRef) -> Frame {
            self.hint = Some(hint);
            self
        }

        fn disabled_hint(mut self, disabled_hint: WidgetRef) -> Frame {
            self.disabled_hint = Some(disabled_hint);
            self
        }

        fn dropdown_alignment(mut self, dropdown_alignment: AlignmentGeometry) -> Frame {
            self.dropdown_alignment = dropdown_alignment;
            self
        }

        fn media_size(mut self, media_size: Size) -> Frame {
            self.media_size = Some(media_size);
            self
        }

        fn form_field(mut self, is_form_field: bool) -> Frame {
            self.is_form_field = is_form_field;
            self
        }

        fn item_height(mut self, item_height: Option<f64>) -> Frame {
            self.item_height = item_height;
            self
        }

        fn menu_max_height(mut self, menu_max_height: f64) -> Frame {
            self.menu_max_height = Some(menu_max_height);
            self
        }

        fn menu_width(mut self, menu_width: f64) -> Frame {
            self.menu_width = Some(menu_width);
            self
        }

        fn alignment(mut self, alignment: AlignmentGeometry) -> Frame {
            self.alignment = alignment;
            self
        }

        fn selected_item_builder(mut self, builder: DropdownButtonBuilder) -> Frame {
            self.selected_item_builder = Some(builder);
            self
        }

        fn underline(mut self, underline: WidgetRef) -> Frame {
            self.underline = Some(underline);
            self
        }

        fn padding(mut self, padding: EdgeInsetsGeometry) -> Frame {
            self.padding = Some(padding);
            self
        }

        fn on_tap(mut self, on_tap: Listener) -> Frame {
            self.on_tap = Some(on_tap);
            self
        }

        fn focus_node(mut self, focus_node: FocusNodeRef) -> Frame {
            self.focus_node = Some(focus_node);
            self
        }

        fn autofocus(mut self, autofocus: bool) -> Frame {
            self.autofocus = autofocus;
            self
        }

        fn focus_color(mut self, focus_color: Color) -> Frame {
            self.focus_color = Some(focus_color);
            self
        }
    }

    /// Dart's `getMenuRect`: the rectangle of the `_DropdownMenu` above the menu's `ListView`.
    fn menu_rect(tester: &WidgetTester) -> Rect {
        let list_view = only(tester.find_by_type::<ListView>());
        let menu = tester.ancestor_of_type::<DropdownMenu<String>>(list_view);
        let render = tester.render_box_of(menu);
        rect_of(tester, render)
    }

    fn the_button(tester: &WidgetTester) -> AnyRenderBox {
        let button = only(tester.find_by_type::<DropdownButton<String>>());
        tester.render_box_of(button)
    }

    fn icon_box(tester: &WidgetTester, key: &GlobalKey) -> AnyRenderBox {
        key_box(tester, key)
    }

    fn icon_color(tester: &WidgetTester, key: &GlobalKey) -> Option<Color> {
        let icon = key
            .current_context(&tester.cell.borrow())
            .expect("the icon is mounted");
        tester.glyph_style(icon).color.map(|color| color.color())
    }

    #[test]
    fn dropdown_button_with_no_app() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (value, did_change_value) = value_log();
            *value.borrow_mut() = Some("one".to_owned());
            let build = |tester: &WidgetTester| {
                let frame = frame()
                    .initial_value(Some("one"))
                    .on_changed(Rc::clone(&did_change_value));
                let page = frame.build(&tester.cell.borrow());
                Directionality::new(
                    TextDirection::Ltr,
                    Builder::new(move |app, context| {
                        let view = View::of(app, context);
                        let data = MediaQueryData::from_view(app, &*view, None);
                        let page = page.clone();
                        MediaQuery::new(
                            data,
                            Navigator::new().initial_route("/").on_generate_route(
                                move |app, settings| {
                                    let page = page.clone();
                                    let route = MaterialPageRoute::new(
                                        app,
                                        Rc::new(move |_app, _context| {
                                            Material::new().child(page.clone()).into_widget()
                                        }),
                                    );
                                    route.clone().settings(
                                        app,
                                        RouteSettingsRef::Settings(settings.clone()),
                                    );
                                    Some(route.erase())
                                },
                            ),
                        )
                        .into_widget()
                    }),
                )
                .into_widget()
            };

            let root = build(&tester);
            tester.pump_widget(root);

            tap_text(&mut tester, "one");
            settle_menu(&mut tester);
            assert_eq!(value.borrow().as_deref(), Some("one"));

            tap_last_text(&mut tester, "three");
            settle_menu(&mut tester);
            assert_eq!(value.borrow().as_deref(), Some("three"));

            tap_first_text_anywhere(&mut tester, "three");
            settle_menu(&mut tester);
            assert_eq!(value.borrow().as_deref(), Some("three"));

            let root = build(&tester);
            tester.pump_widget(root);

            tap_last_text(&mut tester, "two");
            settle_menu(&mut tester);
            assert_eq!(value.borrow().as_deref(), Some("two"));
        });
    }

    fn items_of(values: &[&str]) -> Vec<DropdownMenuItem<String>> {
        values
            .iter()
            .map(|value| DropdownMenuItem::new(Text::new(*value)).value((*value).to_owned()))
            .collect()
    }

    #[test]
    #[should_panic(expected = "There should be exactly one item with [DropdownButton]'s value")]
    fn dropdown_button_does_not_allow_duplicate_item_values() {
        let _ = DropdownButton::new(Some(items_of(&["a", "b", "c", "c"])), Some(on_changed()))
            .value(Some("c".to_owned()))
            .into_widget();
    }

    #[test]
    #[should_panic(expected = "There should be exactly one item with [DropdownButton]'s value")]
    fn dropdown_button_value_should_only_appear_in_one_menu_item() {
        let _ = DropdownButton::new(Some(items_of(&["a", "b", "c", "d"])), Some(on_changed()))
            .value(Some("e".to_owned()))
            .into_widget();
    }

    fn indexed_stack_index(tester: &WidgetTester) -> Option<i32> {
        let stack = only(tester.find_by_type::<IndexedStack>());
        tester.widget(stack, |stack: &IndexedStack| stack.index)
    }

    #[test]
    fn dropdown_form_field_uses_form_field_state() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let form_key = GlobalKey::new();
            let value: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
            let saved = Rc::clone(&value);
            let field_key = form_key.clone();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let saved = Rc::clone(&saved);
                    let initial_value = saved.borrow().clone();
                    let set_state = set_state.clone();
                    Material::new()
                        .child(
                            Form::new(
                                DropdownButtonFormField::new(Some(items_of(&MENU_ITEMS)))
                                    .initial_value(initial_value)
                                    .hint(Text::new("Select Value"))
                                    .decoration(
                                        InputDecoration::new()
                                            .prefix_icon(Icon::new(Some(Icons::fastfood()))),
                                    )
                                    .validator(|_app, value| {
                                        value.is_none().then(|| "Must select value".to_owned())
                                    })
                                    .on_changed(|_app, _value| {})
                                    .on_saved(move |app, value| {
                                        let saved = Rc::clone(&saved);
                                        set_state(app, &mut |_app| {
                                            *saved.borrow_mut() = value.clone();
                                        });
                                    }),
                            )
                            .key(field_key.clone().into()),
                        )
                        .into_widget()
                })
                .into_widget(),
            );
            tester.pump_widget(root);

            // Initial value of null displays hint
            assert_eq!(*value.borrow(), None);
            assert_eq!(indexed_stack_index(&tester), Some(4));
            tap_first_text_anywhere(&mut tester, "Select Value");
            tester.pump_and_settle();
            tap_last_text(&mut tester, "three");
            tester.pump_and_settle();
            assert_eq!(indexed_stack_index(&tester), Some(2));
            // Changes only made to FormField state until form saved
            assert_eq!(*value.borrow(), None);
            let form = form_key
                .current_state::<FormState>(&tester.cell.borrow())
                .expect("the form is mounted");
            form.save(&mut tester.cell.borrow_mut());
            tester.cell.checkpoint();
            assert_eq!(value.borrow().as_deref(), Some("three"));
        });
    }

    fn form_field_state(tester: &WidgetTester) -> Handle<DropdownButtonFormFieldState<String>> {
        only(tester.find_by_type::<DropdownButtonFormField<String>>())
            .state_handle::<DropdownButtonFormFieldState<String>>(&tester.cell.borrow())
            .expect("the form field's state")
    }

    #[test]
    fn dropdown_form_field_only_uses_initial_value_parameter_when_first_built_and_when_reset() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let set_state = set_state.clone();
                    Material::new()
                        .child(
                            DropdownButtonFormField::new(Some(items_of(&MENU_ITEMS)))
                                .initial_value(Some("one".to_owned()))
                                .hint(Text::new("Select Value"))
                                .on_changed(move |app, _value| {
                                    // Do nothing, just to trigger a rebuild.
                                    set_state(app, &mut |_app| {});
                                }),
                        )
                        .into_widget()
                })
                .into_widget(),
            );
            tester.pump_widget(root);
            let state = form_field_state(&tester);
            assert_eq!(
                state.value(&tester.cell.borrow()).map(String::as_str),
                Some("one")
            );

            // Open the dropdown menu.
            tap_text(&mut tester, "one");
            tester.pump_and_settle();

            tap_last_text(&mut tester, "three");
            tester.pump_and_settle();

            // The value should update to selected, not the initial value.
            assert_eq!(tester.find_onstage_text("three").len(), 1);
            assert_eq!(
                state.value(&tester.cell.borrow()).map(String::as_str),
                Some("three")
            );

            state.reset(&mut tester.cell.borrow_mut());
            tester.pump(Duration::ZERO);

            // Reset to the initial value.
            assert_eq!(tester.find_onstage_text("one").len(), 1);
            assert_eq!(
                state.value(&tester.cell.borrow()).map(String::as_str),
                Some("one")
            );
        });
    }

    #[test]
    fn dropdown_in_list_view() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/12053
            // Positions a DropdownButton at the left and right edges of the screen,
            // forcing it to be sized down to the viewport width
            let mut tester = tester();
            let item_key = key("item");
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Material::new()
                    .child(ListView::new().children(vec![
                        DropdownButton::new(
                            Some(vec![
                                DropdownMenuItem::new(Text::new("foo"))
                                    .key(item_key.clone())
                                    .value("foo".to_owned()),
                            ]),
                            Some(on_changed()),
                        )
                        .value(Some("foo".to_owned()))
                        .into_widget(),
                    ]))
                    .into_widget(),
            );
            tester.pump_widget(root);
            tap_text(&mut tester, "foo");
            tester.pump(Duration::ZERO);
            let item_boxes: Vec<AnyRenderBox> = all_by_key(&tester, &item_key)
                .into_iter()
                .map(|element| tester.render_box_of(element))
                .collect();
            assert_eq!(global_top_left(&tester, item_boxes[0]).dx(), 0.0);
            assert_eq!(global_top_left(&tester, item_boxes[1]).dx(), 16.0);
            assert_eq!(size_of(&tester, item_boxes[1]).width(), 800.0 - 16.0 * 2.0);
        });
    }

    #[test]
    fn dropdown_screen_edges() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = Rc::new(Cell::new(Some(4)));
            let set_value = Rc::clone(&value);
            let handle_changed: ValueChanged<Option<i32>> =
                Rc::new(move |_app, new_value| set_value.set(new_value));
            let items: Vec<DropdownMenuItem<i32>> = (0..20)
                .map(|i| DropdownMenuItem::new(Text::new(i.to_string())).value(i))
                .collect();
            let button_key = GlobalKey::new();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Material::new()
                    .child(
                        waft_widgets::Align::new()
                            .alignment(AlignmentGeometry::TOP_CENTER)
                            .child(
                                DropdownButton::new(Some(items), Some(handle_changed))
                                    .key(button_key.clone().into())
                                    .value(Some(4)),
                            ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            tap_text(&mut tester, "4");
            settle_menu(&mut tester);

            // We should have two copies of item 5, one in the menu and one in the
            // button itself.
            assert_eq!(tester.find_text("5").len(), 2);

            assert_eq!(value.get(), Some(4));
            tap_key(&mut tester, &button_key);
            assert_eq!(value.get(), Some(4));
            // this waits for the route's completer to complete, which calls handleChanged
            tester.pump(Duration::ZERO);
            assert_eq!(value.get(), Some(4));
        });
    }

    fn dropdown_button_aligns_selected_menu_item(text_direction: TextDirection) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let build = frame()
                .button_key(&button_key)
                .text_direction(text_direction)
                .on_changed(on_changed())
                .use_material3(false);

            pump_frame(&mut tester, &build);
            let button_box = key_box(&tester, &button_key);
            let button_origin_before_tap = global_top_left(&tester, button_box);

            tap_text(&mut tester, "two");
            settle_menu(&mut tester);

            // Tapping the dropdown button should not cause it to move.
            assert_eq!(
                global_top_left(&tester, button_box),
                button_origin_before_tap
            );

            // The selected dropdown item is both in menu we just popped up, and in
            // the IndexedStack contained by the dropdown button. Both of them should
            // have the same origin and height as the dropdown button.
            let item_boxes = boxes_of(&tester, "two");
            assert_eq!(item_boxes.len(), 2);
            for item_box in item_boxes {
                match text_direction {
                    TextDirection::Rtl => {
                        assert_eq!(
                            rect_of(&tester, button_box).bottom_right(),
                            rect_of(&tester, item_box).bottom_right()
                        );
                    }
                    TextDirection::Ltr => {
                        assert_eq!(
                            global_top_left(&tester, button_box),
                            global_top_left(&tester, item_box)
                        );
                    }
                }
                assert_eq!(
                    size_of(&tester, button_box).height(),
                    size_of(&tester, item_box).height()
                );
            }

            // The two RenderParagraph objects, for the 'two' items' Text children,
            // should have the same size and location.
            check_selected_item_text_geometry(&tester, "two");
        });
    }

    #[test]
    fn dropdown_button_aligns_selected_menu_item_rtl() {
        dropdown_button_aligns_selected_menu_item(TextDirection::Rtl);
    }

    #[test]
    fn dropdown_button_aligns_selected_menu_item_ltr() {
        dropdown_button_aligns_selected_menu_item(TextDirection::Ltr);
    }

    #[test]
    fn arrow_icon_aligns_with_the_edge_of_button_when_expanded() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    .expanded(true)
                    .on_changed(on_changed()),
            );
            let button_box = key_box(&tester, &button_key);
            let arrow_icon = tester.render_box_of(tester.find_by_icon(&Icons::arrow_drop_down()));

            // Arrow icon should be aligned with far right of button when expanded
            let button_size = size_of(&tester, button_box);
            let arrow_size = size_of(&tester, arrow_icon);
            assert_eq!(
                global_top_left(&tester, arrow_icon).dx(),
                button_size
                    .center_right(Offset::new(-arrow_size.width(), 0.0))
                    .dx()
            );
        });
    }

    #[test]
    fn dropdown_button_icon_will_accept_widgets_as_icons() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let custom_key = GlobalKey::new();
            let custom_widget = Container::new()
                .key(custom_key.clone().into())
                .decoration(waft_painting::ShapeDecoration::new(
                    waft_painting::CircleBorder::new(
                        waft_painting::BorderSide {
                            width: 5.0,
                            color: Colors::GREY.shade700(),
                            ..waft_painting::BorderSide::default()
                        },
                        0.0,
                    ),
                ))
                .into_widget();

            pump_frame(
                &mut tester,
                &frame().icon(custom_widget).on_changed(on_changed()),
            );

            assert!(custom_key.current_context(&tester.cell.borrow()).is_some());
            assert!(icons_showing(&tester, &Icons::arrow_drop_down()).is_empty());

            pump_frame(
                &mut tester,
                &frame()
                    .icon(Icon::new(Some(Icons::assessment())).into_widget())
                    .on_changed(on_changed()),
            );

            assert_eq!(icons_showing(&tester, &Icons::assessment()).len(), 1);
            assert!(icons_showing(&tester, &Icons::arrow_drop_down()).is_empty());
        });
    }

    /// Dart's `find.byIcon(icon)`.
    fn icons_showing(tester: &WidgetTester, icon: &waft_widgets::IconData) -> Vec<AnyElement> {
        tester
            .find_by_type::<Icon>()
            .into_iter()
            .filter(|element| {
                tester.widget(*element, |widget: &Icon| widget.icon.as_ref() == Some(icon))
            })
            .collect()
    }

    #[test]
    fn dropdown_button_icon_should_have_default_size_and_colors_when_not_defined() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let icon_key = GlobalKey::new();
            let custom_icon = Icon::new(Some(Icons::assessment()))
                .key(icon_key.clone().into())
                .into_widget();

            pump_frame(
                &mut tester,
                &frame().icon(custom_icon.clone()).on_changed(on_changed()),
            );

            // test for size
            assert_eq!(
                size_of(&tester, icon_box(&tester, &icon_key)),
                Size::new(24.0, 24.0)
            );

            // test for enabled color
            assert_eq!(
                icon_color(&tester, &icon_key),
                Some(Colors::GREY.shade700())
            );

            // test for disabled color
            pump_frame(&mut tester, &frame().icon(custom_icon));
            assert_eq!(
                icon_color(&tester, &icon_key),
                Some(Colors::GREY.shade400())
            );
        });
    }

    #[test]
    fn dropdown_button_icon_should_have_the_passed_in_size_and_color_instead_of_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let icon_key = GlobalKey::new();
            let custom_icon = Icon::new(Some(Icons::assessment()))
                .key(icon_key.clone().into())
                .into_widget();
            let build = frame()
                .icon(custom_icon)
                .icon_size(30.0)
                .icon_colors(Colors::PINK.primary(), Colors::ORANGE.primary());

            pump_frame(&mut tester, &build.clone().on_changed(on_changed()));

            // test for size
            assert_eq!(
                size_of(&tester, icon_box(&tester, &icon_key)),
                Size::new(30.0, 30.0)
            );

            // test for enabled color
            assert_eq!(icon_color(&tester, &icon_key), Some(Colors::PINK.primary()));

            // test for disabled color
            pump_frame(&mut tester, &build);
            assert_eq!(
                icon_color(&tester, &icon_key),
                Some(Colors::ORANGE.primary())
            );
        });
    }

    #[test]
    fn dropdown_button_should_use_its_own_size_and_color_properties_over_those_defined_by_the_theme()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let icon_key = GlobalKey::new();
            let custom_icon = Icon::new(Some(Icons::assessment()))
                .key(icon_key.clone().into())
                .size(40.0)
                .color(Colors::YELLOW.primary())
                .into_widget();
            let build = frame()
                .icon(custom_icon)
                .icon_size(30.0)
                .icon_colors(Colors::PINK.primary(), Colors::ORANGE.primary());

            pump_frame(&mut tester, &build.clone().on_changed(on_changed()));

            // test for size
            assert_eq!(
                size_of(&tester, icon_box(&tester, &icon_key)),
                Size::new(40.0, 40.0)
            );

            // test for enabled color
            assert_eq!(
                icon_color(&tester, &icon_key),
                Some(Colors::YELLOW.primary())
            );

            // test for disabled color
            pump_frame(&mut tester, &build);
            assert_eq!(
                icon_color(&tester, &icon_key),
                Some(Colors::YELLOW.primary())
            );
        });
    }

    #[test]
    fn dropdown_button_with_is_dense_true_aligns_selected_menu_item() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    .dense(true)
                    .on_changed(on_changed()),
            );
            let button_box = key_box(&tester, &button_key);

            tap_text(&mut tester, "two");
            settle_menu(&mut tester);

            // The selected dropdown item is both in menu we just popped up, and in
            // the IndexedStack contained by the dropdown button. Both of them should
            // have the same vertical center as the button.
            let item_boxes = boxes_of(&tester, "two");
            assert_eq!(item_boxes.len(), 2);

            // When isDense is true, the button's height is reduced. The menu items'
            // heights are not.
            let menu_item_height = item_boxes
                .iter()
                .map(|item_box| size_of(&tester, *item_box).height())
                .fold(f64::MIN, f64::max);
            assert!(menu_item_height > size_of(&tester, button_box).height());

            for item_box in item_boxes {
                assert_eq!(
                    rect_of(&tester, button_box).center().dy(),
                    rect_of(&tester, item_box).center().dy()
                );
            }

            // The two RenderParagraph objects, for the 'two' items' Text children,
            // should have the same size and location.
            check_selected_item_text_geometry(&tester, "two");
        });
    }

    #[test]
    fn dropdown_button_can_have_a_text_style_with_no_font_size_specified() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/33425
            let mut tester = tester();
            let root = test_app(TextDirection::Ltr, None, || {
                Material::new()
                    .child(
                        DropdownButton::new(
                            Some(vec![
                                DropdownMenuItem::new(Text::new("foo"))
                                    .key(key("item"))
                                    .value("foo".to_owned()),
                            ]),
                            Some(on_changed()),
                        )
                        .value(Some("foo".to_owned()))
                        .is_dense(true)
                        .style(TextStyle::new().color(Colors::BLUE.primary())),
                    )
                    .into_widget()
            });
            // Building and laying it out does not panic.
            tester.pump_widget(root);
        });
    }

    fn scrollable_box(tester: &WidgetTester) -> AnyRenderBox {
        tester.render_box_of(only(tester.find_by_type::<Scrollable>()))
    }

    /// Dart's `find.descendant(of: find.byType(Scrollable), matching: find.byKey(ValueKey(text)))`.
    fn menu_item_box(tester: &WidgetTester, text: &str) -> AnyRenderBox {
        let scrollable = only(tester.find_by_type::<Scrollable>());
        let element = only(
            all_by_key(tester, &key(text))
                .into_iter()
                .filter(|element| tester.ancestors(*element).contains(&scrollable))
                .collect(),
        );
        tester.render_box_of(element)
    }

    #[test]
    fn dropdown_menu_scrolls_to_first_item_in_long_lists() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // Open the dropdown menu
            let button_key = GlobalKey::new();
            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    // nothing selected
                    .initial_value(None)
                    .items(Some(numbers(100)))
                    .on_changed(on_changed()),
            );
            tap_key(&mut tester, &button_key);
            tester.pump(Duration::ZERO);
            tester.pump_and_settle(); // finish the menu animation

            // Find the first item in the scrollable dropdown list
            let menu_item_container = scrollable_box(&tester);
            let first_item = menu_item_box(&tester, "0");

            // List should be scrolled so that the first item is at the top. Menu items
            // are offset 8.0 from the top edge of the scrollable menu.
            let first_top = {
                let app = tester.cell.borrow();
                first_item
                    .local_to_global(&app, Offset::new(0.0, -8.0), None)
                    .dy()
            };
            assert_eq!(first_top, rect_of(&tester, menu_item_container).top);
        });
    }

    #[test]
    fn dropdown_menu_aligns_selected_item_with_button_in_long_lists() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // Open the dropdown menu
            let button_key = GlobalKey::new();
            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    .initial_value(Some("50"))
                    .items(Some(numbers(100)))
                    .on_changed(on_changed()),
            );
            let button_box = key_box(&tester, &button_key);
            tap_key(&mut tester, &button_key);
            tester.pump_and_settle(); // finish the menu animation

            // Find the selected item in the scrollable dropdown list
            let selected_item = menu_item_box(&tester, "50");

            // List should be scrolled so that the selected item is in line with the button
            assert_eq!(
                rect_of(&tester, selected_item).center().dy(),
                rect_of(&tester, button_box).center().dy()
            );
        });
    }

    #[test]
    fn dropdown_menu_scrolls_to_last_item_in_long_lists() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    .initial_value(Some("99"))
                    .items(Some(numbers(100)))
                    .on_changed(on_changed()),
            );
            tap_key(&mut tester, &button_key);
            tester.pump(Duration::ZERO);

            let list_view = only(tester.find_by_type::<ListView>());
            {
                let mut app = tester.cell.borrow_mut();
                let scroll_controller = PrimaryScrollController::of(&mut app, list_view);
                // Make sure there is no overscroll
                assert_eq!(
                    scroll_controller.offset(&app),
                    scroll_controller.position(&app).max_scroll_extent(&app)
                );
            }

            // Find the selected item in the scrollable dropdown list
            let menu_item_container = scrollable_box(&tester);
            let selected_item = menu_item_box(&tester, "99");

            // kMaterialListPadding.vertical is 8.
            let container_bottom = {
                let app = tester.cell.borrow();
                let size = menu_item_container.size(&app);
                menu_item_container
                    .local_to_global(&app, Offset::new(0.0, -8.0), None)
                    .dy()
                    + size.height()
            };
            // Selected item should be aligned to the bottom of the dropdown menu.
            assert_eq!(rect_of(&tester, selected_item).bottom, container_bottom);
        });
    }

    #[test]
    fn size_of_dropdown_button_with_null_value() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let build = |value: Option<&str>| {
                frame()
                    .button_key(&button_key)
                    .initial_value(value)
                    .on_changed(on_changed())
            };

            pump_frame(&mut tester, &build(None));
            let button_box_null_value = key_box(&tester, &button_key);

            pump_frame(&mut tester, &build(Some("three")));
            let button_box = key_box(&tester, &button_key);

            // A Dropdown button with a null value should be the same size as a
            // one with a non-null value.
            assert_eq!(
                rect_of(&tester, button_box),
                rect_of(&tester, button_box_null_value)
            );
        });
    }

    #[test]
    fn size_of_dropdown_button_with_no_items() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/26419
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let build = |items: Option<Vec<String>>| {
                frame()
                    .button_key(&button_key)
                    .items(items)
                    .on_changed(on_changed())
            };

            pump_frame(&mut tester, &build(None));
            let button_box_null_items = key_box(&tester, &button_key);

            pump_frame(&mut tester, &build(Some(Vec::new())));
            let _button_box_empty_items = key_box(&tester, &button_key);

            pump_frame(&mut tester, &build(Some(strings(&MENU_ITEMS))));
            let button_box = key_box(&tester, &button_key);

            // A Dropdown button with a null value should be the same size as a
            // one with a non-null value.
            assert_eq!(
                rect_of(&tester, button_box),
                rect_of(&tester, button_box_null_items)
            );
        });
    }

    #[test]
    fn layout_of_a_dropdown_button_with_null_value() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let (value, set_value) = value_log();
            let build = |value: &Rc<RefCell<Option<String>>>| {
                frame()
                    .button_key(&button_key)
                    .initial_value(value.borrow().as_deref())
                    .on_changed(Rc::clone(&set_value))
            };

            pump_frame(&mut tester, &build(&value));

            // Show the menu.
            tap_key(&mut tester, &button_key);
            settle_menu(&mut tester);

            // Tap on item 'one', which must appear over the button.
            tap_key(&mut tester, &button_key);
            settle_menu(&mut tester);

            pump_frame(&mut tester, &build(&value));
            assert_eq!(value.borrow().as_deref(), Some("one"));
        });
    }

    #[test]
    fn size_of_dropdown_button_with_null_value_and_a_hint() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            // The hint will define the dropdown's width
            let build = |value: Option<&str>| {
                frame()
                    .button_key(&button_key)
                    .initial_value(value)
                    .hint(Text::new("onetwothree").into_widget())
            };

            pump_frame(&mut tester, &build(None));
            assert_eq!(tester.find_onstage_text("onetwothree").len(), 1);
            let button_box_hint_value = key_box(&tester, &button_key);

            pump_frame(&mut tester, &build(Some("three")));
            let button_box = key_box(&tester, &button_key);

            // A Dropdown button with a null value and a hint should be the same size as a
            // one with a non-null value.
            assert_eq!(
                rect_of(&tester, button_box),
                rect_of(&tester, button_box_hint_value)
            );
        });
    }

    #[test]
    fn dropdown_menus_must_fit_within_the_screen() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // In all of the tests that follow we're assuming that the dropdown menu
            // is horizontally aligned with the center of the dropdown button and padded
            // on the top, left, and right.
            let button_padding = EdgeInsets::only(16.0, 8.0, 24.0, 0.0);

            let mut pop_up_and_down = |frame: Frame| -> (Rect, Rect) {
                pump_frame(&mut tester, &frame);
                let button = the_button(&tester);
                tap_box(&mut tester, button);
                tester.pump_and_settle();
                let menu_rect = menu_rect(&tester);
                let button_rect =
                    button_padding.inflate_rect(rect_of(&tester, the_button(&tester)));
                tap_box(&mut tester, button);
                (menu_rect, button_rect)
            };
            let last = Some(*MENU_ITEMS.last().expect("items"));
            let first = Some(MENU_ITEMS[0]);

            // Dropdown button is along the top of the app. The top of the menu is
            // aligned with the top of the expanded button and shifted horizontally
            // so that it fits within the frame.

            let (menu_rect, _) = pop_up_and_down(
                frame()
                    .dropdown_alignment(AlignmentGeometry::TOP_LEFT)
                    .initial_value(last)
                    .on_changed(on_changed()),
            );
            assert_eq!(menu_rect.top_left(), Offset::ZERO);
            assert_eq!(menu_rect.top_right(), Offset::new(menu_rect.width(), 0.0));

            let (menu_rect, button_rect) = pop_up_and_down(
                frame()
                    .dropdown_alignment(AlignmentGeometry::TOP_CENTER)
                    .initial_value(last)
                    .on_changed(on_changed()),
            );
            assert_eq!(menu_rect.top_left(), Offset::new(button_rect.left, 0.0));
            assert_eq!(menu_rect.top_right(), Offset::new(button_rect.right, 0.0));

            let (menu_rect, _) = pop_up_and_down(
                frame()
                    .dropdown_alignment(AlignmentGeometry::TOP_RIGHT)
                    .initial_value(last)
                    .on_changed(on_changed()),
            );
            assert_eq!(
                menu_rect.top_left(),
                Offset::new(800.0 - menu_rect.width(), 0.0)
            );
            assert_eq!(menu_rect.top_right(), Offset::new(800.0, 0.0));

            // Dropdown button is along the middle of the app. The top of the menu is
            // aligned with the top of the expanded button (because the 1st item
            // is selected) and shifted horizontally so that it fits within the frame.

            let (menu_rect, button_rect) = pop_up_and_down(
                frame()
                    .dropdown_alignment(AlignmentGeometry::CENTER_LEFT)
                    .initial_value(first)
                    .on_changed(on_changed()),
            );
            assert_eq!(menu_rect.top_left(), Offset::new(0.0, button_rect.top));
            assert_eq!(
                menu_rect.top_right(),
                Offset::new(menu_rect.width(), button_rect.top)
            );

            let (menu_rect, button_rect) =
                pop_up_and_down(frame().initial_value(first).on_changed(on_changed()));
            assert_eq!(menu_rect.top_left(), button_rect.top_left());
            assert_eq!(menu_rect.top_right(), button_rect.top_right());

            let (menu_rect, button_rect) = pop_up_and_down(
                frame()
                    .dropdown_alignment(AlignmentGeometry::CENTER_RIGHT)
                    .initial_value(first)
                    .on_changed(on_changed()),
            );
            assert_eq!(
                menu_rect.top_left(),
                Offset::new(800.0 - menu_rect.width(), button_rect.top)
            );
            assert_eq!(menu_rect.top_right(), Offset::new(800.0, button_rect.top));

            // Dropdown button is along the bottom of the app. The bottom of the menu is
            // aligned with the bottom of the expanded button and shifted horizontally
            // so that it fits within the frame.

            let (menu_rect, _) = pop_up_and_down(
                frame()
                    .dropdown_alignment(AlignmentGeometry::BOTTOM_LEFT)
                    .initial_value(first)
                    .on_changed(on_changed()),
            );
            assert_eq!(menu_rect.bottom_left(), Offset::new(0.0, 600.0));
            assert_eq!(
                menu_rect.bottom_right(),
                Offset::new(menu_rect.width(), 600.0)
            );

            let (menu_rect, button_rect) = pop_up_and_down(
                frame()
                    .dropdown_alignment(AlignmentGeometry::BOTTOM_CENTER)
                    .initial_value(first)
                    .on_changed(on_changed()),
            );
            assert_eq!(
                menu_rect.bottom_left(),
                Offset::new(button_rect.left, 600.0)
            );
            assert_eq!(
                menu_rect.bottom_right(),
                Offset::new(button_rect.right, 600.0)
            );

            let (menu_rect, _) = pop_up_and_down(
                frame()
                    .dropdown_alignment(AlignmentGeometry::BOTTOM_RIGHT)
                    .initial_value(first)
                    .on_changed(on_changed()),
            );
            assert_eq!(
                menu_rect.bottom_left(),
                Offset::new(800.0 - menu_rect.width(), 600.0)
            );
            assert_eq!(menu_rect.bottom_right(), Offset::new(800.0, 600.0));
        });
    }

    #[test]
    fn dropdown_menus_are_dismissed_on_screen_orientation_changes_but_not_on_keyboard_hide() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build = |size: Size| frame().on_changed(on_changed()).media_size(size);
            pump_frame(&mut tester, &build(Size::new(800.0, 600.0)));
            let button = the_button(&tester);
            tap_box(&mut tester, button);
            tester.pump_and_settle();
            assert_eq!(tester.find_onstage_by_type::<ListView>().len(), 1);

            // Show a keyboard (simulate by shortening the height).
            pump_frame(&mut tester, &build(Size::new(800.0, 300.0)));
            tester.pump(Duration::ZERO);
            assert_eq!(tester.find_by_type::<ListView>().len(), 1);

            // Hide a keyboard again (simulate by increasing the height).
            pump_frame(&mut tester, &build(Size::new(800.0, 600.0)));
            tester.pump(Duration::ZERO);
            assert_eq!(tester.find_by_type::<ListView>().len(), 1);

            // Rotate the device (simulate by changing the aspect ratio).
            pump_frame(&mut tester, &build(Size::new(600.0, 800.0)));
            tester.pump(Duration::ZERO);
            assert!(tester.find_by_type::<ListView>().is_empty());
        });
    }

    fn shows(tester: &WidgetTester, text: &str) -> usize {
        tester.find_onstage_text(text).len()
    }

    fn center_dy_of(tester: &WidgetTester, element: AnyElement) -> f64 {
        rect_of(tester, tester.render_box_of(element)).center().dy()
    }

    fn text_center_dy(tester: &WidgetTester, text: &str) -> f64 {
        center_dy_of(tester, only(tester.find_onstage_text(text)))
    }

    fn icon_center_dy(tester: &WidgetTester) -> f64 {
        center_dy_of(tester, only(tester.find_by_type::<Icon>()))
    }

    #[test]
    fn disabled_hint_displays_on_empty_items_or_on_changed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let build = |items: Option<Vec<String>>,
                         on_changed: Option<ValueChanged<Option<String>>>| {
                let mut frame = frame()
                    .items(items)
                    .button_key(&button_key)
                    .initial_value(None)
                    .hint(Text::new("enabled").into_widget())
                    .disabled_hint(Text::new("disabled").into_widget());
                frame.on_changed = on_changed;
                frame
            };

            // [disabledHint] should display when [items] is null
            pump_frame(&mut tester, &build(None, Some(on_changed())));
            assert_eq!(shows(&tester, "enabled"), 0);
            assert_eq!(shows(&tester, "disabled"), 1);

            // [disabledHint] should display when [items] is an empty list.
            pump_frame(&mut tester, &build(Some(Vec::new()), Some(on_changed())));
            assert_eq!(shows(&tester, "enabled"), 0);
            assert_eq!(shows(&tester, "disabled"), 1);

            // [disabledHint] should display when [onChanged] is null
            pump_frame(&mut tester, &build(Some(strings(&MENU_ITEMS)), None));
            assert_eq!(shows(&tester, "enabled"), 0);
            assert_eq!(shows(&tester, "disabled"), 1);
            let disabled_hint_box = key_box(&tester, &button_key);

            // A Dropdown button with a disabled hint should be the same size as a
            // one with a regular enabled hint.
            pump_frame(
                &mut tester,
                &build(Some(strings(&MENU_ITEMS)), Some(on_changed())),
            );
            assert_eq!(shows(&tester, "disabled"), 0);
            assert_eq!(shows(&tester, "enabled"), 1);
            let enabled_hint_box = key_box(&tester, &button_key);
            assert_eq!(
                rect_of(&tester, enabled_hint_box),
                rect_of(&tester, disabled_hint_box)
            );
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/70177
    #[test]
    fn disabled_hint_behavior_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build = |on_changed: Option<ValueChanged<Option<String>>>,
                         value: Option<&str>,
                         hint: Option<&str>,
                         disabled_hint: Option<&str>| {
                let mut frame = frame()
                    .items(Some(strings(&MENU_ITEMS)))
                    .initial_value(value);
                frame.on_changed = on_changed;
                frame.hint = hint.map(|hint| Text::new(hint).into_widget());
                frame.disabled_hint = disabled_hint.map(|hint| Text::new(hint).into_widget());
                frame
            };

            // The selected value should be displayed when the button is disabled.
            pump_frame(&mut tester, &build(None, Some("two"), None, None));
            // The dropdown icon and the selected menu item are vertically aligned.
            assert_eq!(text_center_dy(&tester, "two"), icon_center_dy(&tester));

            // If [value] is null, the button is enabled, hint is displayed.
            pump_frame(
                &mut tester,
                &build(Some(on_changed()), None, Some("hint"), Some("disabledHint")),
            );
            assert_eq!(text_center_dy(&tester, "hint"), icon_center_dy(&tester));

            // If [value] is null, the button is disabled, [disabledHint] is displayed when [disabledHint] is non-null.
            pump_frame(
                &mut tester,
                &build(None, None, Some("hint"), Some("disabledHint")),
            );
            assert_eq!(
                text_center_dy(&tester, "disabledHint"),
                icon_center_dy(&tester)
            );

            // If [value] is null, the button is disabled, [hint] is displayed when [disabledHint] is null.
            pump_frame(&mut tester, &build(None, None, Some("hint"), None));
            assert_eq!(text_center_dy(&tester, "hint"), icon_center_dy(&tester));

            // If [value], [hint] and [disabledHint] are null, the button is disabled, nothing displayed.
            pump_frame(&mut tester, &build(None, None, None, None));
            assert_eq!(indexed_stack_index(&tester), None);

            // If [value], [hint] and [disabledHint] are null, the button is enabled, nothing displayed.
            pump_frame(&mut tester, &build(Some(on_changed()), None, None, None));
            assert_eq!(indexed_stack_index(&tester), None);
        });
    }

    #[test]
    fn dropdown_button_selected_item_color_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build =
                |tester: &WidgetTester, on_changed: Option<ValueChanged<Option<String>>>| {
                    let mut field = DropdownButtonFormField::new(Some(items_of(&["one", "two"])))
                        .style(TextStyle::new().color(Colors::YELLOW.primary()))
                        .initial_value(Some("two".to_owned()));
                    if let Some(on_changed) = on_changed {
                        field = field.on_changed(move |app, value| on_changed(app, value));
                    }
                    material_app(
                        ThemeData::builder()
                            .disabled_color(Colors::PINK.primary())
                            .build(&tester.cell.borrow()),
                        crate::scaffold::Scaffold::new()
                            .body(Center::new().child(
                                waft_widgets::Column::new().children(vec![field.into_widget()]),
                            ))
                            .into_widget(),
                    )
                };

            // The selected value should be displayed when the button is enabled.
            let root = build(&tester, Some(on_changed()));
            tester.pump_widget(root);
            // The dropdown icon and the selected menu item are vertically aligned.
            assert_eq!(text_center_dy(&tester, "two"), icon_center_dy(&tester));
            // Selected item has a normal color from [DropdownButtonFormField.style]
            // when the button is enabled.
            assert_eq!(tester.text_color("two"), Colors::YELLOW.primary());

            // The selected value should be displayed when the button is disabled.
            let root = build(&tester, None);
            tester.pump_widget(root);
            assert_eq!(text_center_dy(&tester, "two"), icon_center_dy(&tester));
            // Selected item has a disabled color from [theme.disabledColor]
            // when the button is disable.
            assert_eq!(tester.text_color("two"), Colors::PINK.primary());
        });
    }

    #[test]
    fn dropdown_button_hint_displays_when_the_items_list_is_empty_items_is_null_and_disabled_hint_is_null()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let build = |items: Option<Vec<String>>| {
                frame()
                    .items(items)
                    .button_key(&button_key)
                    .initial_value(None)
                    .hint(Text::new("hint used when disabled").into_widget())
            };

            // [hint] should display when [items] is null and [disabledHint] is not defined
            pump_frame(&mut tester, &build(None));
            assert_eq!(shows(&tester, "hint used when disabled"), 1);

            // [hint] should display when [items] is an empty list and [disabledHint] is not defined.
            pump_frame(&mut tester, &build(Some(Vec::new())));
            assert_eq!(shows(&tester, "hint used when disabled"), 1);
        });
    }

    /// Dart's `(BuildContext context) => [for (final item in items) SizedBox.square(dimension:
    /// double.parse(item), child: Center(child: Text(item)))]`.
    fn square_items_builder(items: &[&str]) -> DropdownButtonBuilder {
        let items = strings(items);
        Rc::new(move |_app, _context| {
            items
                .iter()
                .map(|item| {
                    SizedBox::square(Some(item.parse::<f64>().expect("a number")))
                        .child(Center::new().child(Text::new(item.clone())))
                        .into_widget()
                })
                .collect()
        })
    }

    /// Dart's `tester.renderObject<RenderBox>(find.widgetWithText(Row, text))`: the first `Row`
    /// with a `Text` showing `text` below it.
    fn row_with_text(tester: &WidgetTester, text: &str, skip_offstage: bool) -> Size {
        let texts = if skip_offstage {
            tester.find_onstage_text(text)
        } else {
            tester.find_text(text)
        };
        let row = tester
            .find_by_type::<Row>()
            .into_iter()
            .find(|row| {
                texts
                    .iter()
                    .any(|text| tester.ancestors(*text).contains(row))
            })
            .expect("a Row with the text");
        size_of(tester, tester.render_box_of(row))
    }

    const SQUARE_ITEMS: [&str; 3] = ["25", "50", "100"];

    fn square_frame() -> Frame {
        frame()
            .items(Some(strings(&SQUARE_ITEMS)))
            .item_height(None)
            .selected_item_builder(square_items_builder(&SQUARE_ITEMS))
    }

    fn sized_hint(dimension: f64) -> WidgetRef {
        SizedBox::new()
            .height(dimension)
            .width(dimension)
            .child(Text::new("hint"))
            .into_widget()
    }

    #[test]
    fn size_of_largest_widget_is_used_dropdown_button_when_selected_item_builder_is_non_null() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_frame(
                &mut tester,
                // To test the size constraints, the selected item should not be the
                // largest item. This validates that the button sizes itself according
                // to the largest item regardless of which one is selected.
                &square_frame()
                    .initial_value(Some("25"))
                    .on_changed(on_changed()),
            );

            let size = row_with_text(&tester, "25", true);
            // DropdownButton should be the height of the largest item
            assert_eq!(size.height(), 100.0);
            // DropdownButton should be width of largest item added to the icon size
            assert_eq!(size.width(), 100.0 + 24.0);
        });
    }

    #[test]
    fn enabled_button_size_of_largest_widget_is_used_when_the_hint_is_smaller_than_the_largest_item()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_frame(
                &mut tester,
                &square_frame()
                    .initial_value(None)
                    // [hint] widget is smaller than largest selected item widget
                    .hint(sized_hint(50.0))
                    .on_changed(on_changed()),
            );

            let size = row_with_text(&tester, "hint", true);
            assert_eq!(size.height(), 100.0);
            assert_eq!(size.width(), 100.0 + 24.0);
        });
    }

    #[test]
    fn enabled_button_size_of_largest_widget_is_used_when_the_hint_is_larger_than_the_largest_item()
    {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_frame(
                &mut tester,
                &square_frame()
                    .initial_value(Some("25"))
                    // [hint] widget is larger than largest selected item widget
                    .hint(sized_hint(125.0))
                    .on_changed(on_changed()),
            );

            let size = row_with_text(&tester, "25", true);
            // DropdownButton should be the height of the largest item (hint inclusive)
            assert_eq!(size.height(), 125.0);
            // DropdownButton should be width of largest item (hint inclusive) added to the icon size
            assert_eq!(size.width(), 125.0 + 24.0);
        });
    }

    #[test]
    fn disabled_button_size_of_largest_widget_is_used_when_the_hint_is_smaller_than_the_largest_item()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_frame(
                &mut tester,
                &square_frame().initial_value(None).hint(sized_hint(50.0)),
            );

            let size = row_with_text(&tester, "hint", true);
            assert_eq!(size.height(), 100.0);
            assert_eq!(size.width(), 100.0 + 24.0);
        });
    }

    #[test]
    fn disabled_button_size_of_largest_widget_is_used_when_the_hint_is_larger_than_the_largest_item()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_frame(
                &mut tester,
                &square_frame().initial_value(None).hint(sized_hint(125.0)),
            );

            let size = row_with_text(&tester, "25", false);
            assert_eq!(size.height(), 125.0);
            assert_eq!(size.width(), 125.0 + 24.0);
        });
    }

    #[test]
    fn disabled_button_size_of_largest_widget_is_used_when_the_disabled_hint_is_smaller_than_the_largest_item()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_frame(
                &mut tester,
                &square_frame()
                    .initial_value(None)
                    .disabled_hint(sized_hint(50.0)),
            );

            let size = row_with_text(&tester, "hint", true);
            assert_eq!(size.height(), 100.0);
            assert_eq!(size.width(), 100.0 + 24.0);
        });
    }

    #[test]
    fn disabled_button_size_of_largest_widget_is_used_when_the_disabled_hint_is_larger_than_the_largest_item()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_frame(
                &mut tester,
                &square_frame()
                    .initial_value(None)
                    .disabled_hint(sized_hint(125.0)),
            );

            let size = row_with_text(&tester, "25", false);
            assert_eq!(size.height(), 125.0);
            assert_eq!(size.width(), 125.0 + 24.0);
        });
    }

    #[test]
    fn menu_width_is_correct_when_set() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/133267.
            let mut tester = tester();
            pump_frame(
                &mut tester,
                &frame()
                    .initial_value(Some("25"))
                    .items(Some(strings(&SQUARE_ITEMS)))
                    .menu_width(200.0)
                    .selected_item_builder(square_items_builder(&SQUARE_ITEMS))
                    .on_changed(on_changed()),
            );

            tap_text(&mut tester, "25");
            tester.pump_and_settle();

            assert_eq!(menu_rect(&tester).width(), 200.0);
        });
    }

    fn int_items(count: i32) -> Vec<DropdownMenuItem<i32>> {
        (0..count)
            .map(|i| DropdownMenuItem::new(Text::new(i.to_string())).value(i))
            .collect()
    }

    /// Dart's `PrimaryScrollController.of(tester.element(find.byType(ListView))).position.pixels`.
    fn menu_scroll(tester: &WidgetTester) -> f64 {
        let list_view = only(tester.find_by_type::<ListView>());
        let mut app = tester.cell.borrow_mut();
        let scroll_controller = PrimaryScrollController::of(&mut app, list_view);
        scroll_controller.position(&app).pixels(&app)
    }

    fn menu_scroll_case(value: i32, alignment: AlignmentGeometry, expected: f64) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Material::new()
                    .child(
                        waft_widgets::Align::new().alignment(alignment).child(
                            DropdownButton::new(
                                Some(int_items(100)),
                                Some(Rc::new(|_app, _value| {})),
                            )
                            .value(Some(value)),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            tap_text(&mut tester, &value.to_string());
            tester.pump_and_settle();
            assert_eq!(menu_scroll(&tester), expected);
        });
    }

    #[test]
    fn dropdown_in_middle_showing_middle_item() {
        menu_scroll_case(50, AlignmentGeometry::CENTER, 2180.0);
    }

    #[test]
    fn dropdown_in_top_showing_bottom_item() {
        menu_scroll_case(99, AlignmentGeometry::TOP_CENTER, 4312.0);
    }

    #[test]
    fn dropdown_in_bottom_showing_top_item() {
        menu_scroll_case(0, AlignmentGeometry::BOTTOM_CENTER, 0.0);
    }

    #[test]
    fn dropdown_in_center_showing_bottom_item() {
        menu_scroll_case(99, AlignmentGeometry::CENTER, 4312.0);
    }

    #[test]
    fn dropdown_menu_respects_parent_size_limits() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/24417
            let mut tester = tester();
            let selected_index = Rc::new(Cell::new(None));
            let set_index = Rc::clone(&selected_index);
            let on_changed: ValueChanged<Option<i32>> = Rc::new(move |_app, i| set_index.set(i));
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .bottom_navigation_bar(SizedBox::new().height(200.0))
                    .body(Navigator::new().on_generate_route(move |app, _settings| {
                        let on_changed = Rc::clone(&on_changed);
                        let route = MaterialPageRoute::new(
                            app,
                            Rc::new(move |_app, _context| {
                                waft_widgets::SafeArea::new(
                                    Container::new()
                                        .alignment(AlignmentGeometry::TOP_LEFT)
                                        // From material/dropdown.dart (menus are unaligned by default):
                                        //  _kUnalignedMenuMargin = EdgeInsetsDirectional.only(start: 16.0, end: 24.0)
                                        // This padding ensures that the entire menu will be visible
                                        .padding(EdgeInsetsGeometry::directional(
                                            16.0, 0.0, 24.0, 0.0,
                                        ))
                                        .child(
                                            DropdownButton::new(
                                                Some(int_items(100)),
                                                Some(Rc::clone(&on_changed)),
                                            )
                                            .value(Some(12)),
                                        ),
                                )
                                .into_widget()
                            }),
                        );
                        Some(route.erase())
                    }))
                    .into_widget(),
            );
            tester.pump_widget(root);

            tap_text(&mut tester, "12");
            tester.pump_and_settle();
            assert_eq!(selected_index.get(), None);

            tap_last_text(&mut tester, "13");
            tester.pump_and_settle();
            assert_eq!(selected_index.get(), Some(13));
        });
    }

    fn bottom_border(color: Color, width: f64) -> BoxDecoration {
        BoxDecoration::new().border(waft_painting::Border {
            bottom: waft_painting::BorderSide {
                color,
                width,
                ..waft_painting::BorderSide::default()
            },
            ..waft_painting::Border::from_border_side(waft_painting::BorderSide::NONE)
        })
    }

    #[test]
    fn dropdown_button_will_accept_widgets_as_its_underline() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let decoration = bottom_border(Color::new(0xFFCCBB00), 4.0);
            let default_decoration = bottom_border(Color::new(0xFFBDBDBD), 0.0);

            let custom_underline = Container::new()
                .height(4.0)
                .decoration(decoration.clone())
                .into_widget();
            let button_key = GlobalKey::new();

            let last_decoration = |tester: &WidgetTester| -> BoxDecoration {
                let button = button_key
                    .current_context(&tester.cell.borrow())
                    .expect("the button is mounted");
                let decorated = *tester
                    .descendants_of_type::<waft_widgets::DecoratedBox>(button)
                    .last()
                    .expect("a DecoratedBox");
                tester.widget(decorated, |decorated: &waft_widgets::DecoratedBox| {
                    decorated
                        .decoration
                        .as_any()
                        .downcast_ref::<BoxDecoration>()
                        .expect("a BoxDecoration")
                        .clone()
                })
            };

            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    .underline(custom_underline)
                    .on_changed(on_changed()),
            );
            assert_eq!(last_decoration(&tester), decoration);

            pump_frame(
                &mut tester,
                &frame().button_key(&button_key).on_changed(on_changed()),
            );
            assert_eq!(last_decoration(&tester), default_decoration);
        });
    }

    #[test]
    fn dropdown_button_selected_item_builder_builds_custom_buttons() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let items = ["One", "Two", "Three"];
            let selected_item = Rc::new(RefCell::new(Some(items[0].to_owned())));
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let set_state = set_state.clone();
                    let selected = Rc::clone(&selected_item);
                    crate::scaffold::Scaffold::new()
                        .body(
                            DropdownButton::new(
                                Some(items_of(&items)),
                                Some(Rc::new(move |app, string| {
                                    let selected = Rc::clone(&selected);
                                    set_state(app, &mut |_app| {
                                        *selected.borrow_mut() = string.clone();
                                    });
                                })),
                            )
                            .value(selected_item.borrow().clone())
                            .selected_item_builder(
                                move |_app, _context| {
                                    let mut index = 0;
                                    items
                                        .iter()
                                        .map(|string| {
                                            index += 1;
                                            Text::new(format!(
                                                "{string} as an Arabic numeral: {index}"
                                            ))
                                            .into_widget()
                                        })
                                        .collect()
                                },
                            ),
                        )
                        .into_widget()
                })
                .into_widget(),
            );
            tester.pump_widget(root);

            assert_eq!(shows(&tester, "One as an Arabic numeral: 1"), 1);
            tap_text(&mut tester, "One as an Arabic numeral: 1");
            tester.pump_and_settle();
            tap_text(&mut tester, "Two");
            tester.pump_and_settle();
            assert_eq!(shows(&tester, "Two as an Arabic numeral: 2"), 1);
        });
    }

    /// Dart's `checkDropdownColor`: the menu's painter fills its rounded rectangle, after the
    /// elevation's three shadows, in `color`, or else the theme's canvas colour.
    fn check_dropdown_color(color: Option<Color>, is_form_field: bool) {
        use waft_embedder::valo::Op;
        with_main_thread_stack(move || {
            let mut tester = tester();
            let text = "foo";
            let items = || {
                Some(vec![
                    DropdownMenuItem::new(Text::new(text)).value(text.to_owned()),
                ])
            };
            let home = if is_form_field {
                let mut field = DropdownButtonFormField::new(items())
                    .initial_value(Some(text.to_owned()))
                    .on_changed(|_app, _value| {});
                if let Some(color) = color {
                    field = field.dropdown_color(color);
                }
                Form::new(field).into_widget()
            } else {
                let mut button =
                    DropdownButton::new(items(), Some(on_changed())).value(Some(text.to_owned()));
                if let Some(color) = color {
                    button = button.dropdown_color(color);
                }
                button.into_widget()
            };
            let root = material_app(
                m2(&tester.cell.borrow()),
                Material::new().child(home).into_widget(),
            );
            tester.pump_widget(root);
            tap_text(&mut tester, text);
            tester.pump(Duration::ZERO);

            let menu = only(tester.find_by_type::<DropdownMenu<String>>());
            let custom_paint = only(
                tester
                    .descendants_of_type::<CustomPaint>(menu)
                    .into_iter()
                    .take(1)
                    .collect(),
            );
            let render = tester.render_box_of(custom_paint).as_object();
            let ops = tester.painted_ops(render);
            assert!(matches!(ops.first(), Some(Op::Save)));
            // Dart's `rrect` calls: a blurred shadow rect is Valo's `RRectBlur`, where Dart's is
            // an `rrect` call with a mask filter.
            let rrects: Vec<(bool, waft_embedder::valo::Color)> = ops
                .iter()
                .filter_map(|op| match op {
                    Op::DrawPath { paint, .. } => Some((false, paint.color)),
                    Op::RRectBlur { paint, .. } => Some((true, paint.color)),
                    _ => None,
                })
                .collect();
            let expected = color.unwrap_or(Colors::GREY.shade50());
            assert_eq!(rrects[3], (false, expected.into()));
        });
    }

    #[test]
    fn dropdown_button_uses_default_color_when_expanded() {
        check_dropdown_color(None, false);
    }

    #[test]
    fn dropdown_button_uses_dropdown_color_when_expanded() {
        check_dropdown_color(Some(Color::from_rgbo(120, 220, 70, 0.8)), false);
    }

    #[test]
    fn dropdown_button_form_field_uses_dropdown_color_when_expanded() {
        check_dropdown_color(Some(Color::from_rgbo(120, 220, 70, 0.8)), true);
    }

    #[test]
    fn dropdown_button_hint_displays_properly_when_selected_item_builder_is_defined() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/42340
            let mut tester = tester();
            let items = ["1", "2", "3"];
            let selected_item: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let set_state = set_state.clone();
                    let selected = Rc::clone(&selected_item);
                    crate::scaffold::Scaffold::new()
                        .body(
                            DropdownButton::new(
                                Some(items_of(&items)),
                                Some(Rc::new(move |app, string| {
                                    let selected = Rc::clone(&selected);
                                    set_state(app, &mut |_app| {
                                        *selected.borrow_mut() = string.clone();
                                    });
                                })),
                            )
                            .hint(Text::new("Please select an item"))
                            .value(selected_item.borrow().clone())
                            .selected_item_builder(
                                move |_app, _context| {
                                    items
                                        .iter()
                                        .map(|item| {
                                            Text::new(format!("You have selected: {item}"))
                                                .into_widget()
                                        })
                                        .collect()
                                },
                            ),
                        )
                        .into_widget()
                })
                .into_widget(),
            );
            tester.pump_widget(root);

            // Initially shows the hint text
            assert_eq!(shows(&tester, "Please select an item"), 1);
            tap_first_text_anywhere(&mut tester, "Please select an item");
            tester.pump_and_settle();
            tap_text(&mut tester, "1");
            tester.pump_and_settle();
            // Selecting an item should display its corresponding item builder
            assert_eq!(shows(&tester, "You have selected: 1"), 1);
        });
    }

    #[test]
    fn variable_size_and_oversized_menu_items() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let item_heights = [30.0, 40.0, 50.0, 60.0];
            let dropdown_value = Rc::new(Cell::new(Some(item_heights[0])));
            let value_key = |value: f64| -> KeyRef { ValueKey::new(value.to_bits()).into() };
            let value = Rc::clone(&dropdown_value);
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(Center::new().child(StatefulBuilder::new(
                        move |_app, _context, set_state| {
                            let set_state = set_state.clone();
                            let set_value = Rc::clone(&value);
                            DropdownButton::new(
                                Some(
                                    item_heights
                                        .iter()
                                        .map(|value| {
                                            DropdownMenuItem::new(
                                                Center::new().child(
                                                    Container::new()
                                                        .width(100.0)
                                                        .height(*value)
                                                        .color(Colors::BLUE.primary()),
                                                ),
                                            )
                                            .key(value_key(*value))
                                            .value(*value)
                                        })
                                        .collect(),
                                ),
                                Some(Rc::new(move |app, value| {
                                    let set_value = Rc::clone(&set_value);
                                    set_state(app, &mut |_app| set_value.set(value));
                                })),
                            )
                            .value(value.get())
                            .item_height(None)
                            .into_widget()
                        },
                    )))
                    .into_widget(),
            );

            let boxes = |tester: &WidgetTester, value: f64| -> Vec<Rect> {
                all_by_key(tester, &value_key(value))
                    .into_iter()
                    .map(|element| rect_of(tester, tester.render_box_of(element)))
                    .collect()
            };
            let icon_rect = |tester: &WidgetTester| {
                rect_of(
                    tester,
                    tester.render_box_of(only(tester.find_by_type::<Icon>())),
                )
            };

            // Only the DropdownButton is visible. It contains the selected item
            // and a dropdown arrow icon.
            tester.pump_widget(root);
            assert_eq!(tester.find_by_type::<Icon>().len(), 1);
            assert_eq!(boxes(&tester, 30.0).len(), 1);

            // All menu items have a minimum height of 48. The centers of the
            // dropdown icon and the selected menu item are vertically aligned
            // and horizontally adjacent.
            let item30 = boxes(&tester, 30.0)[0];
            assert_eq!(item30.size(), Size::new(100.0, 48.0));
            assert_eq!(item30.center().dy(), icon_rect(&tester).center().dy());
            assert_eq!(item30.top_right().dx(), icon_rect(&tester).top_left().dx());

            // Show the popup menu.
            tester.tap(item30.center(), 1);
            tester.pump_and_settle();

            // Each item appears twice, once in the menu and once
            // in the dropdown button's IndexedStack.
            for value in item_heights {
                assert_eq!(boxes(&tester, value).len(), 2);
            }

            // Verify that the items have the expected sizes. The width of the items
            // that appear in the menu is padded by 16 on the left and right.
            assert_eq!(boxes(&tester, 30.0)[0].size(), Size::new(100.0, 48.0));
            assert_eq!(boxes(&tester, 40.0)[0].size(), Size::new(100.0, 48.0));
            assert_eq!(boxes(&tester, 50.0)[0].size(), Size::new(100.0, 50.0));
            assert_eq!(boxes(&tester, 60.0)[0].size(), Size::new(100.0, 60.0));
            assert_eq!(boxes(&tester, 30.0)[1].size(), Size::new(132.0, 48.0));
            assert_eq!(boxes(&tester, 40.0)[1].size(), Size::new(132.0, 48.0));
            assert_eq!(boxes(&tester, 50.0)[1].size(), Size::new(132.0, 50.0));
            assert_eq!(boxes(&tester, 60.0)[1].size(), Size::new(132.0, 60.0));

            // The vertical center of the selectedItem (item30) should
            // line up with its button counterpart.
            assert_eq!(
                boxes(&tester, 30.0)[0].center().dy(),
                boxes(&tester, 30.0)[1].center().dy()
            );

            // The menu items should be arranged in a column.
            assert_eq!(
                boxes(&tester, 30.0)[1].bottom_left(),
                boxes(&tester, 40.0)[1].top_left()
            );
            assert_eq!(
                boxes(&tester, 40.0)[1].bottom_left(),
                boxes(&tester, 50.0)[1].top_left()
            );
            assert_eq!(
                boxes(&tester, 50.0)[1].bottom_left(),
                boxes(&tester, 60.0)[1].top_left()
            );

            // Dismiss the menu by selecting item40 and then show the menu again.
            tester.tap(boxes(&tester, 40.0)[1].center(), 1);
            tester.pump_and_settle();
            assert_eq!(dropdown_value.get(), Some(40.0));
            tester.tap(boxes(&tester, 40.0)[0].center(), 1);
            tester.pump_and_settle();

            // The vertical center of the selectedItem (item40) should
            // line up with its button counterpart.
            assert_eq!(
                boxes(&tester, 40.0)[0].center().dy(),
                boxes(&tester, 40.0)[1].center().dy()
            );
        });
    }

    #[test]
    fn dropdown_button_menu_items_do_not_resize_when_its_route_is_popped() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/44877.
            let mut tester = tester();
            let items = ["one", "two", "three"];
            let item = Rc::new(RefCell::new(Some(items[0].to_owned())));
            let text_scale = Rc::new(Cell::new(1.0));
            let root = StatefulBuilder::new(move |app, _context, set_state| {
                let set_state = set_state.clone();
                let scale = Rc::clone(&text_scale);
                let set_scale = Rc::clone(&text_scale);
                let set_item = Rc::clone(&item);
                crate::app::MaterialApp::new()
                    .theme(ThemeData::new(app))
                    .builder(move |app, context, child| {
                        scale.set(MediaQuery::of(app, context).text_scaler.scale(14.0) / 14.0);
                        MediaQuery::new(
                            MediaQueryData::new()
                                .text_scaler(waft_painting::TextScaler::linear(scale.get())),
                            child.cloned().expect("the navigator"),
                        )
                        .into_widget()
                    })
                    .home(
                        crate::scaffold::Scaffold::new().body(
                            DropdownButton::new(
                                Some(items_of(&items)),
                                Some(Rc::new(move |app, new_item| {
                                    let set_item = Rc::clone(&set_item);
                                    let set_scale = Rc::clone(&set_scale);
                                    set_state(app, &mut |_app| {
                                        *set_item.borrow_mut() = new_item.clone();
                                        set_scale.set(set_scale.get() + 0.1);
                                    });
                                })),
                            )
                            .value(item.borrow().clone()),
                        ),
                    )
                    .into_widget()
            })
            .into_widget();
            tester.pump_widget(root);

            // Verify that the first item is showing.
            assert_eq!(shows(&tester, "one"), 1);

            // Select a different item to trigger setState, which updates mediaQuery
            // and forces a performLayout on the popped _DropdownRoute. This operation
            // should not cause an exception.
            tap_text(&mut tester, "one");
            tester.pump_and_settle();
            tap_last_text(&mut tester, "two");
            tester.pump_and_settle();
            assert_eq!(shows(&tester, "two"), 1);
        });
    }

    fn press(tester: &mut WidgetTester, logical: LogicalKeyboardKey) {
        use waft_services::PhysicalKeyboardKey;
        let physical = if logical == LogicalKeyboardKey::ENTER {
            PhysicalKeyboardKey::ENTER
        } else if logical == LogicalKeyboardKey::SPACE {
            PhysicalKeyboardKey::SPACE
        } else if logical == LogicalKeyboardKey::TAB {
            PhysicalKeyboardKey::TAB
        } else if logical == LogicalKeyboardKey::ARROW_DOWN {
            PhysicalKeyboardKey::ARROW_DOWN
        } else if logical == LogicalKeyboardKey::ARROW_UP {
            PhysicalKeyboardKey::ARROW_UP
        } else {
            panic!("a key these cases press")
        };
        tester.send_key_event(physical, logical);
    }

    fn focus_node(tester: &WidgetTester) -> FocusNodeRef {
        FocusNode::new(&mut tester.cell.borrow_mut()).into_erased()
    }

    fn has_primary_focus(tester: &WidgetTester, node: AnyFocusNode) -> bool {
        node.has_primary_focus(&tester.cell.borrow())
    }

    #[test]
    fn dropdown_button_can_be_focused_and_has_focus_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let button_key = GlobalKey::new();
            let focus_node = focus_node(&tester);

            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    .on_changed(on_changed())
                    .focus_node(focus_node.clone())
                    .autofocus(true)
                    .use_material3(false),
            );
            tester.pump_and_settle(); // Pump a frame for autofocus to take effect.
            assert!(has_primary_focus(&tester, focus_node.handle()));
            assert!(
                tester
                    .ink_drawn()
                    .contains(&crate::test_support::Drawn::Rect(
                        0x1f000000,
                        Rect::from_ltrb(348.0, 276.0, 452.0, 324.0)
                    ))
            );

            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    .on_changed(on_changed())
                    .focus_node(focus_node.clone())
                    .focus_color(Color::new(0xff00ff00))
                    .use_material3(false),
            );
            tester.pump_and_settle(); // Pump a frame for autofocus to take effect.
            assert!(
                tester
                    .ink_drawn()
                    .contains(&crate::test_support::Drawn::Rect(
                        0x1f00ff00,
                        Rect::from_ltrb(348.0, 276.0, 452.0, 324.0)
                    ))
            );
        });
    }

    /// Dart's `findInputDecoratorBorderPainter()` with `paints..rrect(style: fill, color:)`:
    /// whether the decorator's border container fills a path in `color`.
    fn input_decorator_fills(tester: &WidgetTester, color: Color) -> bool {
        let decorator = only(tester.find_by_type::<InputDecorator>());
        let render = tester.render_box_of(decorator).as_object();
        tester.painted(render).iter().any(|drawn| {
            matches!(drawn, crate::test_support::Drawn::Path(fill, _) if *fill == color.to_argb32())
        })
    }

    fn decorator_theme(tester: &WidgetTester) -> Rc<ThemeData> {
        let decorator = only(tester.find_by_type::<InputDecorator>());
        Theme::of(&mut tester.cell.borrow_mut(), decorator)
    }

    #[test]
    fn dropdown_button_form_field_can_be_focused_and_has_focus_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let button_key = GlobalKey::new();
            let focus_node = focus_node(&tester);
            let form_field = || {
                frame()
                    .form_field(true)
                    .button_key(&button_key)
                    .on_changed(on_changed())
                    .focus_node(focus_node.clone())
            };

            let mut first = form_field().autofocus(true);
            first.decoration = Some(InputDecoration::new().filled(true));
            pump_frame(&mut tester, &first);

            tester.pump(Duration::ZERO); // Pump a frame for autofocus to take effect.
            assert!(has_primary_focus(&tester, focus_node.handle()));

            // Default focus Color from InputDecorator defaults.
            let theme = decorator_theme(&tester);
            assert!(input_decorator_fills(
                &tester,
                theme.color_scheme.surface_container_highest()
            ));

            // Focus color from Decoration.
            let mut second = form_field();
            second.decoration = Some(
                InputDecoration::new()
                    .filled(true)
                    .focus_color(Color::new(0xff00ffff)),
            );
            pump_frame(&mut tester, &second);
            assert!(input_decorator_fills(&tester, Color::new(0xff00ffff)));

            // Focus color from focusColor property.
            let mut third = form_field().focus_color(Color::new(0xff00ff00));
            third.decoration = Some(
                InputDecoration::new()
                    .filled(true)
                    .focus_color(Color::new(0xff00ffff)),
            );
            pump_frame(&mut tester, &third);
            assert!(input_decorator_fills(&tester, Color::new(0xff00ff00)));
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/166642.
    #[test]
    fn dropdown_button_form_field_can_replace_focus_node_properly() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let button_key = GlobalKey::new();
            let build_form_field = |focus_node: FocusNodeRef| {
                let mut frame = frame()
                    .form_field(true)
                    .button_key(&button_key)
                    .on_changed(on_changed())
                    .focus_node(focus_node.clone())
                    .focus_color(Color::new(0xff00ff00));
                frame.decoration = Some(InputDecoration::new().filled(true));
                frame
            };
            let mut node = focus_node(&tester);

            pump_frame(&mut tester, &build_form_field(node.clone()));
            let default_border_color = decorator_theme(&tester)
                .color_scheme
                .surface_container_highest();
            assert!(input_decorator_fills(&tester, default_border_color));

            // Replace focusNode and request focus.
            node.dispose(&mut tester.cell.borrow_mut());
            node = focus_node(&tester);
            node.request_focus(&mut tester.cell.borrow_mut(), None);

            pump_frame(&mut tester, &build_form_field(node.clone()));
            tester.pump(Duration::ZERO); // Wait for requestFocus to take effect.
            assert!(input_decorator_fills(&tester, Color::new(0xff00ff00)));

            // Replace focusNode and request focus.
            node.dispose(&mut tester.cell.borrow_mut());
            node = focus_node(&tester);
            node.request_focus(&mut tester.cell.borrow_mut(), None);

            pump_frame(&mut tester, &build_form_field(node.clone()));
            {
                let mut app = tester.cell.borrow_mut();
                let manager = FocusManager::instance(&mut app);
                if let Some(primary_focus) = manager.primary_focus(&app) {
                    primary_focus.unfocus(&mut app, waft_widgets::UnfocusDisposition::Scope);
                }
            }
            tester.pump(Duration::ZERO); // Wait for unfocus to take effect.
            assert!(input_decorator_fills(&tester, default_border_color));
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/147069.
    #[test]
    fn dropdown_button_form_field_can_be_hovered() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let button_key = GlobalKey::new();

            pump_frame(
                &mut tester,
                &frame()
                    .form_field(true)
                    .button_key(&button_key)
                    .on_changed(on_changed()),
            );
            tester.pump(Duration::ZERO);

            // Check inputDecorator.isHovering value because DropdownButtonFormField
            // delegates to the InputDecorator which manages hover overlay.
            let is_hovering = |tester: &WidgetTester| {
                tester.widget(
                    only(tester.find_by_type::<InputDecorator>()),
                    |decorator: &InputDecorator| decorator.is_hovering,
                )
            };
            assert!(!is_hovering(&tester));

            let mut mouse = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            let center = tester.center_of(&button_key);
            tester.move_to(&mut mouse, center, false);
            tester.pump(Duration::ZERO);

            assert!(is_hovering(&tester));
        });
    }

    #[test]
    fn dropdown_button_wont_be_focused_if_not_enabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let focus_node = focus_node(&tester);

            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    .focus_node(focus_node.clone())
                    .autofocus(true)
                    .focus_color(Color::new(0xff00ff00)),
            );
            tester.pump(Duration::ZERO); // Pump a frame for autofocus to take effect (although it shouldn't).
            assert!(!has_primary_focus(&tester, focus_node.handle()));
            let button = tester.render_object(&button_key);
            assert!(!tester.paints_rrect(
                button,
                waft_embedder::RRect::from_ltrbxy(0.0, 0.0, 104.0, 48.0, 4.0, 4.0),
                Some(Color::new(0xff00ff00)),
            ));
        });
    }

    /// Dart's `buildFrame` of the keyboard cases: a centred button with `focus_node`, autofocused,
    /// whose `on_changed` sets `value`.
    fn keyboard_app(
        tester: &WidgetTester,
        focus_node: FocusNodeRef,
        value: &Rc<RefCell<Option<String>>>,
        text_key: fn(&str) -> String,
    ) -> WidgetRef {
        let value = Rc::clone(value);
        material_app(
            ThemeData::new(&tester.cell.borrow()),
            crate::scaffold::Scaffold::new()
                .body(Center::new().child(StatefulBuilder::new(
                    move |_app, _context, set_state| {
                        let set_state = set_state.clone();
                        let set_value = Rc::clone(&value);
                        DropdownButton::new(
                            Some(
                                MENU_ITEMS
                                    .iter()
                                    .map(|item| {
                                        DropdownMenuItem::new(
                                            Text::new(*item).key(key(&text_key(item))),
                                        )
                                        .key(key(item))
                                        .value((*item).to_owned())
                                    })
                                    .collect(),
                            ),
                            Some(Rc::new(move |app, new_value| {
                                let set_value = Rc::clone(&set_value);
                                set_state(app, &mut |_app| {
                                    *set_value.borrow_mut() = new_value.clone()
                                });
                            })),
                        )
                        .focus_node(focus_node.clone())
                        .autofocus(true)
                        .value(value.borrow().clone())
                        .item_height(None)
                        .into_widget()
                    },
                )))
                .into_widget(),
        )
    }

    fn dropdown_button_is_activated_with(activator: LogicalKeyboardKey) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            let focus_node = focus_node(&tester);
            let value = Rc::new(RefCell::new(Some("one".to_owned())));
            let root = keyboard_app(&tester, focus_node.clone(), &value, |item| {
                format!("{item}Text")
            });

            tester.pump_widget(root);
            tester.pump(Duration::ZERO); // Pump a frame for autofocus to take effect.
            assert!(has_primary_focus(&tester, focus_node.handle()));

            press(&mut tester, activator);
            settle_menu(&mut tester);
            assert_eq!(value.borrow().as_deref(), Some("one"));

            press(&mut tester, LogicalKeyboardKey::TAB); // Focus 'two'
            tester.pump(Duration::ZERO);
            press(&mut tester, activator); // Select 'two'.
            tester.pump(Duration::ZERO);

            settle_menu(&mut tester);

            assert_eq!(value.borrow().as_deref(), Some("two"));
        });
    }

    #[test]
    fn dropdown_button_is_activated_with_the_enter_key() {
        dropdown_button_is_activated_with(LogicalKeyboardKey::ENTER);
    }

    #[test]
    fn dropdown_button_is_activated_with_the_space_key() {
        dropdown_button_is_activated_with(LogicalKeyboardKey::SPACE);
    }

    // Regression test for https://github.com/flutter/flutter/issues/77655.
    #[test]
    fn dropdown_button_selecting_a_null_valued_item_should_be_selected() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let items: [(Option<&str>, &str); 3] =
                [(None, "None"), (Some("one"), "One"), (Some("two"), "Two")];
            let selected_item = Rc::new(RefCell::new(Some("one".to_owned())));
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let set_state = set_state.clone();
                    let selected = Rc::clone(&selected_item);
                    crate::scaffold::Scaffold::new()
                        .body(
                            DropdownButton::new(
                                Some(
                                    items
                                        .iter()
                                        .map(|(value, label)| {
                                            let item = DropdownMenuItem::new(Text::new(*label));
                                            match value {
                                                Some(value) => item.value((*value).to_owned()),
                                                None => item,
                                            }
                                        })
                                        .collect(),
                                ),
                                Some(Rc::new(move |app, string| {
                                    let selected = Rc::clone(&selected);
                                    set_state(app, &mut |_app| {
                                        *selected.borrow_mut() = string.clone();
                                    });
                                })),
                            )
                            .value(selected_item.borrow().clone()),
                        )
                        .into_widget()
                })
                .into_widget(),
            );
            tester.pump_widget(root);

            tap_text(&mut tester, "One");
            tester.pump_and_settle();
            tap_last_text(&mut tester, "None");
            tester.pump_and_settle();
            assert_eq!(shows(&tester, "None"), 1);
        });
    }

    /// Dart's `Focus.of(tester.element(finder))`.
    fn focus_of(tester: &WidgetTester, element: AnyElement) -> AnyFocusNode {
        Focus::of(&mut tester.cell.borrow_mut(), element, false, false)
    }

    #[test]
    fn selected_element_is_focused_when_dropdown_is_opened() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node = focus_node(&tester);
            let value = Rc::new(RefCell::new(Some("one".to_owned())));
            let root = keyboard_app(&tester, focus_node.clone(), &value, |item| {
                format!("Text {item}")
            });
            tester.pump_widget(root);

            tester.pump(Duration::ZERO); // Pump a frame for autofocus to take effect.
            assert!(has_primary_focus(&tester, focus_node.handle()));

            press(&mut tester, LogicalKeyboardKey::ENTER);
            settle_menu(&mut tester); // finish the menu open animation
            assert_eq!(value.borrow().as_deref(), Some("one"));
            let one = *all_by_key(&tester, &key("one"))
                .last()
                .expect("the menu's 'one'");
            let node = focus_of(&tester, one);
            assert!(has_primary_focus(&tester, node));

            press(&mut tester, LogicalKeyboardKey::TAB); // Focus 'two'
            tester.pump(Duration::ZERO);
            press(&mut tester, LogicalKeyboardKey::ENTER); // Select 'two' and close the dropdown.

            settle_menu(&mut tester); // finish the menu close animation

            assert_eq!(value.borrow().as_deref(), Some("two"));

            // Now make sure that "two" is focused when we re-open the dropdown.
            press(&mut tester, LogicalKeyboardKey::ENTER);
            settle_menu(&mut tester); // finish the menu open animation
            assert_eq!(value.borrow().as_deref(), Some("two"));
            let two = *all_by_key(&tester, &key("two"))
                .last()
                .expect("the menu's 'two'");
            let node = focus_of(&tester, two);
            assert!(node.has_focus(&mut tester.cell.borrow_mut()));
        });
    }

    /// Whether a hit test at the centre of `element` reaches its render box: Dart's
    /// `hitTestable()` finder.
    fn hit_testable(tester: &WidgetTester, element: AnyElement) -> bool {
        let render = tester.render_box_of(element);
        let center = tester.global_rect(render).center();
        let mut app = tester.cell.borrow_mut();
        let view_id = app.platform().views()[0].id();
        let mut result = waft_gestures::HitTestResult::new();
        waft_gestures::GestureBinding::instance(&mut app).hit_test_in_view(
            &mut app,
            &mut result,
            center,
            view_id,
        );
        result.path().iter().any(|entry| {
            (entry.target() as &dyn std::any::Any)
                .downcast_ref::<waft_rendering::BoxHitTestEntry>()
                .is_some_and(|entry| entry.target() == render)
        })
    }

    /// Dart's `find.text(text).hitTestable()`.
    fn hit_testable_texts(tester: &WidgetTester, text: &str) -> Vec<AnyElement> {
        tester
            .find_onstage_text(text)
            .into_iter()
            .filter(|element| hit_testable(tester, *element))
            .collect()
    }

    fn tap_hit_testable_text(tester: &mut WidgetTester, text: &str) {
        let element = *hit_testable_texts(tester, text)
            .first()
            .expect("a hit-testable text");
        let render = tester.render_box_of(element);
        tap_box(tester, render);
    }

    #[test]
    fn selected_element_is_correctly_focused_with_dropdown_that_more_items_than_fit_on_the_screen()
    {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node = focus_node(&tester);
            let value = Rc::new(Cell::new(Some(1)));
            let int_key = |item: i32| -> KeyRef { ValueKey::new(item).into() };
            let state_value = Rc::clone(&value);
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(Center::new().child(StatefulBuilder::new({
                        let focus_node = focus_node.clone();
                        move |_app, _context, set_state| {
                            let set_state = set_state.clone();
                            let set_value = Rc::clone(&state_value);
                            DropdownButton::new(
                                Some(
                                    (0..50)
                                        .map(|item| {
                                            DropdownMenuItem::new(
                                                Text::new(item.to_string())
                                                    .key(key(&format!("Text {item}"))),
                                            )
                                            .key(int_key(item))
                                            .value(item)
                                        })
                                        .collect(),
                                ),
                                Some(Rc::new(move |app, new_value| {
                                    let set_value = Rc::clone(&set_value);
                                    set_state(app, &mut |_app| set_value.set(new_value));
                                })),
                            )
                            .focus_node(focus_node.clone())
                            .autofocus(true)
                            .value(state_value.get())
                            .item_height(None)
                            .into_widget()
                        }
                    })))
                    .into_widget(),
            );
            tester.pump_widget(root);

            tester.pump(Duration::ZERO); // Pump a frame for autofocus to take effect.
            assert!(has_primary_focus(&tester, focus_node.handle()));

            press(&mut tester, LogicalKeyboardKey::ENTER);
            settle_menu(&mut tester); // finish the menu open animation
            assert_eq!(value.get(), Some(1));
            let one = *all_by_key(&tester, &int_key(1))
                .last()
                .expect("the menu's 1");
            let node = focus_of(&tester, one);
            assert!(has_primary_focus(&tester, node));

            for _ in 0..41 {
                press(&mut tester, LogicalKeyboardKey::TAB); // Move to the next one.
                tester.pump_and_settle_by(Duration::from_millis(200)); // Wait for it to animate the menu.
            }
            press(&mut tester, LogicalKeyboardKey::ENTER); // Select '42' and close the dropdown.
            tester.pump_and_settle_by(Duration::from_secs(1)); // Finish the menu close animation
            assert_eq!(value.get(), Some(42));

            // Now make sure that "42" is focused when we re-open the dropdown.
            press(&mut tester, LogicalKeyboardKey::ENTER);
            settle_menu(&mut tester); // finish the menu open animation
            assert_eq!(value.get(), Some(42));
            let forty_two = *all_by_key(&tester, &int_key(42))
                .last()
                .expect("the menu's 42");
            let node = focus_of(&tester, forty_two);
            assert!(node.has_focus(&mut tester.cell.borrow_mut()));
        });
    }

    #[test]
    fn having_a_focused_element_does_not_interrupt_scroll_when_flung_by_touch() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node = focus_node(&tester);
            let value = Rc::new(Cell::new(Some(1)));
            let int_key = |item: i32| -> KeyRef { ValueKey::new(item).into() };
            let state_value = Rc::clone(&value);
            let root = material_app(
                m2(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(Center::new().child(StatefulBuilder::new({
                        let focus_node = focus_node.clone();
                        move |_app, _context, set_state| {
                            let set_state = set_state.clone();
                            let set_value = Rc::clone(&state_value);
                            DropdownButton::new(
                                Some(
                                    (0..100)
                                        .map(|item| {
                                            DropdownMenuItem::new(
                                                Text::new(item.to_string())
                                                    .key(key(&format!("Text {item}"))),
                                            )
                                            .key(int_key(item))
                                            .value(item)
                                        })
                                        .collect(),
                                ),
                                Some(Rc::new(move |app, new_value| {
                                    let set_value = Rc::clone(&set_value);
                                    set_state(app, &mut |_app| set_value.set(new_value));
                                })),
                            )
                            .focus_node(focus_node.clone())
                            .autofocus(true)
                            .value(state_value.get())
                            .item_height(None)
                            .into_widget()
                        }
                    })))
                    .into_widget(),
            );
            tester.pump_widget(root);

            tester.pump(Duration::ZERO); // Pump a frame for autofocus to take effect.
            assert!(has_primary_focus(&tester, focus_node.handle()));

            press(&mut tester, LogicalKeyboardKey::ENTER);
            tester.pump_and_settle();
            assert_eq!(value.get(), Some(1));
            let one = *tester
                .find_onstage_by_key(&int_key(1))
                .last()
                .expect("the menu's 1");
            let node = focus_of(&tester, one);
            assert!(has_primary_focus(&tester, node));

            // Move to an item very far down the menu.
            for _ in 0..90 {
                press(&mut tester, LogicalKeyboardKey::TAB); // Move to the next one.
                tester.pump_and_settle(); // Wait for it to animate the menu.
            }
            let ninety_one = *tester
                .find_onstage_by_key(&int_key(91))
                .last()
                .expect("the menu's 91");
            let node = focus_of(&tester, ninety_one);
            assert!(has_primary_focus(&tester, node));

            // Scroll back to the top using touch, and make sure we end up there.
            let menu = tester.render_box_of(only(tester.find_by_type::<DropdownMenu<i32>>()));
            let menu_rect = rect_of(&tester, menu);
            let menu_rect = menu_rect.shift(menu_rect.top_left());
            for _ in 0..10 {
                let center = rect_of(&tester, menu).center();
                tester.fling_from(center, Offset::new(0.0, menu_rect.height()), 10.0, 1);
            }
            tester.pump_and_settle();

            // Make sure that we made it to the top and something didn't stop the
            // scroll.
            let ones = tester.find_onstage_by_key(&int_key(1));
            assert_eq!(ones.len(), 2);
            let last_one = tester.render_box_of(*ones.last().expect("two found"));
            assert_eq!(
                rect_of(&tester, last_one),
                Rect::from_ltrb(372.0, 104.0, 436.0, 152.0)
            );

            // Scrolling to the top again has removed the one the focus was on from the
            // tree, causing it to lose focus.
            let ninety_one = *all_by_key(&tester, &int_key(91))
                .last()
                .expect("the menu's 91, offstage");
            let node = focus_of(&tester, ninety_one);
            assert!(!has_primary_focus(&tester, node));
        });
    }

    #[test]
    fn dropdown_button_on_tap_callback_can_request_focus() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node = focus_node(&tester);
            focus_node.add_listener(&mut tester.cell.borrow_mut(), Listener::new(|_app| {}));
            let value = Rc::new(Cell::new(Some(1)));
            let state_value = Rc::clone(&value);
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(Center::new().child(StatefulBuilder::new({
                        let focus_node = focus_node.clone();
                        move |_app, _context, set_state| {
                            let set_state = set_state.clone();
                            let set_value = Rc::clone(&state_value);
                            DropdownButton::new(
                                Some(
                                    (0..100)
                                        .map(|item| {
                                            DropdownMenuItem::new(Text::new(item.to_string()))
                                                .key(ValueKey::new(item).into())
                                                .value(item)
                                        })
                                        .collect(),
                                ),
                                Some(Rc::new(move |app, new_value| {
                                    let set_value = Rc::clone(&set_value);
                                    set_state(app, &mut |_app| set_value.set(new_value));
                                })),
                            )
                            .focus_node(focus_node.clone())
                            .value(state_value.get())
                            .item_height(None)
                            .into_widget()
                        }
                    })))
                    .into_widget(),
            );
            tester.pump_widget(root);

            tester.pump(Duration::ZERO); // Pump a frame for autofocus to take effect.
            assert!(!has_primary_focus(&tester, focus_node.handle()));

            tap_text(&mut tester, "1");
            tester.pump_and_settle();

            // Close the dropdown menu.
            tester.tap(Offset::new(1.0, 1.0), 1);
            tester.pump_and_settle();

            assert!(has_primary_focus(&tester, focus_node.handle()));
        });
    }

    #[test]
    fn dropdown_button_changes_selected_item_with_arrow_keys() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node = focus_node(&tester);
            let value = Rc::new(RefCell::new(Some("one".to_owned())));
            let root = keyboard_app(&tester, focus_node.clone(), &value, |item| {
                format!("{item}Text")
            });

            tester.pump_widget(root);
            tester.pump(Duration::ZERO); // Pump a frame for autofocus to take effect.
            assert!(has_primary_focus(&tester, focus_node.handle()));

            press(&mut tester, LogicalKeyboardKey::ENTER);
            settle_menu(&mut tester);
            assert_eq!(value.borrow().as_deref(), Some("one"));

            press(&mut tester, LogicalKeyboardKey::ARROW_DOWN); // Focus 'two'.
            tester.pump(Duration::ZERO);
            press(&mut tester, LogicalKeyboardKey::ARROW_DOWN); // Focus 'three'.
            tester.pump(Duration::ZERO);
            press(&mut tester, LogicalKeyboardKey::ARROW_UP); // Back to 'two'.
            tester.pump(Duration::ZERO);
            press(&mut tester, LogicalKeyboardKey::ENTER); // Select 'two'.
            tester.pump(Duration::ZERO);

            settle_menu(&mut tester);

            assert_eq!(value.borrow().as_deref(), Some("two"));
        });
    }

    #[test]
    fn dropdown_button_on_tap_callback_is_called_when_defined() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dropdown_button_tap_counter = Rc::new(Cell::new(0));
            let (value, set_value) = value_log();
            *value.borrow_mut() = Some("one".to_owned());
            let counter = Rc::clone(&dropdown_button_tap_counter);
            let on_tap = Listener::new(move |_app| counter.set(counter.get() + 1));
            pump_frame(
                &mut tester,
                &frame()
                    .initial_value(Some("one"))
                    .on_changed(set_value)
                    .on_tap(on_tap),
            );

            assert_eq!(dropdown_button_tap_counter.get(), 0);

            // Tap dropdown button.
            tap_text(&mut tester, "one");
            tester.pump_and_settle();

            assert_eq!(value.borrow().as_deref(), Some("one"));
            assert_eq!(dropdown_button_tap_counter.get(), 1); // Should update counter.

            // Tap dropdown menu item.
            tap_last_text(&mut tester, "three");
            tester.pump_and_settle();

            assert_eq!(value.borrow().as_deref(), Some("three"));
            assert_eq!(dropdown_button_tap_counter.get(), 1); // Should not change.

            // Tap dropdown button again.
            tap_first_text_anywhere(&mut tester, "three");
            tester.pump_and_settle();

            assert_eq!(value.borrow().as_deref(), Some("three"));
            assert_eq!(dropdown_button_tap_counter.get(), 2); // Should update counter.

            // Tap dropdown menu item.
            tap_last_text(&mut tester, "two");
            tester.pump_and_settle();

            assert_eq!(value.borrow().as_deref(), Some("two"));
            assert_eq!(dropdown_button_tap_counter.get(), 2); // Should not change.
        });
    }

    #[test]
    fn dropdown_menu_item_on_tap_callback_is_called_when_defined() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (value, set_value) = value_log();
            *value.borrow_mut() = Some("one".to_owned());
            let menu_item_tap_counters = Rc::new(RefCell::new(vec![0, 0, 0, 0]));
            let counters = Rc::clone(&menu_item_tap_counters);
            let root = test_app(TextDirection::Ltr, None, move || {
                let items = MENU_ITEMS
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        let counters = Rc::clone(&counters);
                        DropdownMenuItem::new(Text::new(*item))
                            .value((*item).to_owned())
                            .on_tap(Listener::new(move |_app| counters.borrow_mut()[index] += 1))
                    })
                    .collect();
                Material::new()
                    .child(
                        RepaintBoundary::new().child(
                            DropdownButton::new(Some(items), Some(Rc::clone(&set_value)))
                                .value(Some("one".to_owned())),
                        ),
                    )
                    .into_widget()
            });
            tester.pump_widget(root);

            // Tap dropdown button.
            tap_text(&mut tester, "one");
            tester.pump_and_settle();

            assert_eq!(value.borrow().as_deref(), Some("one"));
            // Counters should still be zero.
            assert_eq!(*menu_item_tap_counters.borrow(), vec![0, 0, 0, 0]);

            // Tap dropdown menu item.
            tap_last_text(&mut tester, "three");
            tester.pump_and_settle();

            // Should update the counter for the third item (second index).
            assert_eq!(value.borrow().as_deref(), Some("three"));
            assert_eq!(*menu_item_tap_counters.borrow(), vec![0, 0, 1, 0]);

            // Tap dropdown button again.
            tap_first_text_anywhere(&mut tester, "three");
            tester.pump_and_settle();

            // Should not change.
            assert_eq!(value.borrow().as_deref(), Some("three"));
            assert_eq!(*menu_item_tap_counters.borrow(), vec![0, 0, 1, 0]);

            // Tap dropdown menu item.
            tap_last_text(&mut tester, "two");
            tester.pump_and_settle();

            // Should update the counter for the second item (first index).
            assert_eq!(value.borrow().as_deref(), Some("two"));
            assert_eq!(*menu_item_tap_counters.borrow(), vec![0, 1, 1, 0]);

            // Tap dropdown button again.
            tap_first_text_anywhere(&mut tester, "two");
            tester.pump_and_settle();

            // Should not change.
            assert_eq!(value.borrow().as_deref(), Some("two"));
            assert_eq!(*menu_item_tap_counters.borrow(), vec![0, 1, 1, 0]);

            // Tap the already selected menu item
            tap_last_text(&mut tester, "two");
            tester.pump_and_settle();

            // Should update the counter for the second item (first index), even
            // though it was already selected.
            assert_eq!(value.borrow().as_deref(), Some("two"));
            assert_eq!(*menu_item_tap_counters.borrow(), vec![0, 2, 1, 0]);
        });
    }

    #[test]
    fn does_not_crash_when_option_is_selected_without_waiting_for_opening_animation_to_complete() {
        with_main_thread_stack(|| {
            // Regression test for b/171846624.
            let mut tester = tester();
            let options = ["first", "second", "third"];
            let value = Rc::new(RefCell::new(Some(options[0].to_owned())));
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(StatefulBuilder::new(move |_app, _context, set_state| {
                        let set_state = set_state.clone();
                        let set_value = Rc::clone(&value);
                        DropdownButton::new(
                            Some(items_of(&options)),
                            Some(Rc::new(move |app, v| {
                                let set_value = Rc::clone(&set_value);
                                set_state(app, &mut |_app| *set_value.borrow_mut() = v.clone());
                            })),
                        )
                        .value(value.borrow().clone())
                        .into_widget()
                    }))
                    .into_widget(),
            );
            tester.pump_widget(root);
            assert_eq!(hit_testable_texts(&tester, "first").len(), 1);
            assert_eq!(hit_testable_texts(&tester, "second").len(), 0);
            assert_eq!(hit_testable_texts(&tester, "third").len(), 0);

            // Open dropdown.
            tap_hit_testable_text(&mut tester, "first");
            tester.pump(Duration::ZERO);

            assert_eq!(hit_testable_texts(&tester, "third").len(), 1);
            assert_eq!(hit_testable_texts(&tester, "first").len(), 1);
            assert_eq!(hit_testable_texts(&tester, "second").len(), 1);

            // Deliberately not waiting for opening animation to complete!

            // Select an option in dropdown.
            tap_hit_testable_text(&mut tester, "third");
            tester.pump(Duration::ZERO);
            assert_eq!(hit_testable_texts(&tester, "third").len(), 1);
            assert_eq!(hit_testable_texts(&tester, "first").len(), 0);
            assert_eq!(hit_testable_texts(&tester, "second").len(), 0);
        });
    }

    #[test]
    fn dropdown_menu_should_persistently_show_a_scrollbar_if_it_is_scrollable() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scrollbar_paints_rect = |tester: &WidgetTester| {
                // The scrollbar's painter is on the first `CustomPaint` below it; the
                // `Scrollbar`'s own render object is a repaint boundary.
                let scrollbar = only(tester.find_by_type::<Scrollbar>());
                let custom_paint = *tester
                    .descendants_of_type::<CustomPaint>(scrollbar)
                    .first()
                    .expect("the scrollbar's painter");
                let render = tester.render_box_of(custom_paint).as_object();
                tester
                    .painted(render)
                    .iter()
                    .any(|drawn| matches!(drawn, crate::test_support::Drawn::Rect(..)))
            };
            let max_scroll_extent = |tester: &WidgetTester| {
                let list_view = only(tester.find_by_type::<ListView>());
                let mut app = tester.cell.borrow_mut();
                let scroll_controller = PrimaryScrollController::of(&mut app, list_view);
                scroll_controller.position(&app).max_scroll_extent(&app)
            };
            pump_frame(
                &mut tester,
                &frame()
                    .initial_value(Some("0"))
                    // menu is short enough to fit onto the screen.
                    .items(Some(numbers(10)))
                    .on_changed(on_changed()),
            );
            tap_text(&mut tester, "0");
            tester.pump_and_settle();

            // The scrollbar shouldn't show if the list fits into the screen.
            assert_eq!(max_scroll_extent(&tester), 0.0);
            assert!(!scrollbar_paints_rect(&tester));

            tap_last_text(&mut tester, "0");
            tester.pump_and_settle();
            pump_frame(
                &mut tester,
                &frame()
                    .initial_value(Some("0"))
                    // menu is too long to fit onto the screen.
                    .items(Some(numbers(100)))
                    .on_changed(on_changed()),
            );
            tap_text(&mut tester, "0");
            tester.pump_and_settle();

            // The scrollbar is shown when the list is longer than the height of the screen.
            assert!(max_scroll_extent(&tester) > 0.0);
            assert!(scrollbar_paints_rect(&tester));
        });
    }

    #[test]
    fn dropdown_menus_maximum_height_should_be_influenced_by_dropdown_button_menu_max_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let list_view_height = |tester: &WidgetTester| {
                size_of(
                    tester,
                    tester.render_box_of(only(tester.find_by_type::<ListView>())),
                )
                .height()
            };
            let base = || {
                frame()
                    .initial_value(Some("0"))
                    .items(Some(numbers(64)))
                    .on_changed(on_changed())
            };
            pump_frame(&mut tester, &base());
            tap_text(&mut tester, "0");
            tester.pump_and_settle();

            let list_view = only(tester.find_by_type::<ListView>());
            let mut menu_height = list_view_height(&tester);
            // The default maximum height should be one item height from the edge.
            // https://material.io/design/components/menus.html#usage
            let media_height = MediaQuery::of(&mut tester.cell.borrow_mut(), list_view)
                .size
                .height();
            let default_menu_height = media_height - (2.0 * K_MIN_INTERACTIVE_DIMENSION);
            assert_eq!(menu_height, default_menu_height);

            tap_last_text(&mut tester, "0");
            tester.pump_and_settle();

            // Set menuMaxHeight which is less than defaultMenuHeight
            pump_frame(
                &mut tester,
                &base().menu_max_height(7.0 * K_MIN_INTERACTIVE_DIMENSION),
            );
            tap_text(&mut tester, "0");
            tester.pump_and_settle();

            menu_height = list_view_height(&tester);

            assert!(menu_height != default_menu_height);
            assert_eq!(menu_height, K_MIN_INTERACTIVE_DIMENSION * 7.0);

            tap_last_text(&mut tester, "0");
            tester.pump_and_settle();

            // Set menuMaxHeight which is greater than defaultMenuHeight
            pump_frame(&mut tester, &base().menu_max_height(media_height));

            tap_text(&mut tester, "0");
            tester.pump_and_settle();

            menu_height = list_view_height(&tester);
            assert_eq!(menu_height, default_menu_height);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/89029
    #[test]
    fn menu_position_test_with_menu_max_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    .initial_value(Some("6"))
                    .items(Some(numbers(64)))
                    .on_changed(on_changed())
                    .menu_max_height(2.0 * K_MIN_INTERACTIVE_DIMENSION),
            );

            tap_text(&mut tester, "6");
            tester.pump_and_settle();

            let menu_box = tester.render_box_of(only(tester.find_by_type::<ListView>()));
            let button_box = key_box(&tester, &button_key);
            // The menu's bottom should align with the drop-button's bottom.
            assert_eq!(
                rect_of(&tester, menu_box).bottom,
                rect_of(&tester, button_box).bottom
            );
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/76614
    #[test]
    fn do_not_crash_if_used_in_very_short_screen() {
        use waft_embedder::valo::Op;
        with_main_thread_stack(|| {
            // The default item height is 48.0 pixels and needs two items padding since
            // the menu requires empty space surrounding the menu. Finally, the constraint height
            // is 47.0 pixels for the menu rendering.
            let mut tester = WidgetTester::with_physical_size(
                waft_test::TestPlatform::new().on(TargetPlatform::Android),
                Size::new(1600.0, (48.0 * 3.0 - 1.0) * 2.0),
            );
            tester.install_platform_fonts();

            let item_key = key("item");
            let root = material_app(
                m2(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(
                        Center::new().child(
                            DropdownButton::new(
                                Some(vec![
                                    DropdownMenuItem::new(Text::new("foo"))
                                        .key(item_key.clone())
                                        .value("foo".to_owned()),
                                ]),
                                Some(on_changed()),
                            )
                            .value(Some("foo".to_owned())),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            tap_text(&mut tester, "foo");
            tester.pump_and_settle();

            let item_boxes: Vec<AnyRenderBox> = all_by_key(&tester, &item_key)
                .into_iter()
                .map(|element| tester.render_box_of(element))
                .collect();
            assert_eq!(global_top_left(&tester, item_boxes[0]).dx(), 364.0);
            assert_eq!(global_top_left(&tester, item_boxes[0]).dy(), 47.5);

            assert_eq!(global_top_left(&tester, item_boxes[1]).dx(), 364.0);
            assert_eq!(global_top_left(&tester, item_boxes[1]).dy(), 47.5);

            let menu = only(tester.find_by_type::<DropdownMenu<String>>());
            let custom_paint = *tester
                .descendants_of_type::<CustomPaint>(menu)
                .first()
                .expect("the menu's painter");
            let ops = tester.painted_ops(tester.render_box_of(custom_paint).as_object());
            assert!(matches!(ops.first(), Some(Op::Save)));
            let rrects: Vec<&Op> = ops
                .iter()
                .filter(|op| matches!(op, Op::DrawPath { .. } | Op::RRectBlur { .. }))
                .collect();
            // The height of menu is 47.0.
            let Op::DrawPath { paint, .. } = rrects[3] else {
                panic!("the fourth rrect is the menu's fill, with no mask filter");
            };
            assert_eq!(paint.color, Colors::GREY.shade50().into());
            assert!(tester.paints_rrect(
                tester.render_box_of(custom_paint).as_object(),
                waft_embedder::RRect::from_ltrbxy(0.0, 0.0, 112.0, 47.0, 2.0, 2.0),
                Some(Colors::GREY.shade50()),
            ));
        });
    }

    #[test]
    fn tapping_a_disabled_item_should_not_close_dropdown_button() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = Rc::new(RefCell::new(Some("first".to_owned())));
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(StatefulBuilder::new(move |_app, _context, set_state| {
                        let set_state = set_state.clone();
                        let set_value = Rc::clone(&value);
                        DropdownButton::new(
                            Some(vec![
                                DropdownMenuItem::new(Text::new("disabled")).enabled(false),
                                DropdownMenuItem::new(Text::new("first")).value("first".to_owned()),
                                DropdownMenuItem::new(Text::new("second"))
                                    .value("second".to_owned()),
                            ]),
                            Some(Rc::new(move |app, new_value| {
                                let set_value = Rc::clone(&set_value);
                                set_state(app, &mut |_app| {
                                    *set_value.borrow_mut() = new_value.clone();
                                });
                            })),
                        )
                        .value(value.borrow().clone())
                        .into_widget()
                    }))
                    .into_widget(),
            );
            tester.pump_widget(root);

            // Open dropdown.
            tap_hit_testable_text(&mut tester, "first");
            tester.pump_and_settle();

            // Tap on a disabled item.
            tap_hit_testable_text(&mut tester, "disabled");
            tester.pump_and_settle();

            // The dropdown should still be open, i.e., there should be one widget with 'second' text.
            assert_eq!(hit_testable_texts(&tester, "second").len(), 1);
        });
    }

    #[test]
    fn disabled_item_should_not_be_focusable() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(
                        DropdownButton::new(
                            Some(vec![
                                DropdownMenuItem::new(Text::new("disabled")).enabled(false),
                                DropdownMenuItem::new(Text::new("enabled"))
                                    .value("enabled".to_owned()),
                            ]),
                            Some(on_changed()),
                        )
                        .value(Some("enabled".to_owned())),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            // Open dropdown.
            tap_hit_testable_text(&mut tester, "enabled");
            tester.pump_and_settle();

            // The `FocusNode` of [disabledItem] should be `null` as enabled is false.
            let disabled_item = *hit_testable_texts(&tester, "disabled")
                .first()
                .expect("the menu's disabled item");
            assert_eq!(
                Focus::maybe_of(&mut tester.cell.borrow_mut(), disabled_item, false, false),
                None,
                "Disabled menu item should not be able to request focus"
            );
        });
    }

    #[test]
    fn alignment_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let build = |tester: &WidgetTester, button_alignment: Option<AlignmentGeometry>| {
                let alignment = button_alignment.unwrap_or(AlignmentGeometry::CENTER_START);
                material_app(
                    ThemeData::new(&tester.cell.borrow()),
                    crate::scaffold::Scaffold::new()
                        .body(
                            Center::new().child(
                                DropdownButton::new(
                                    Some(vec![
                                        DropdownMenuItem::new(Text::new("disabled"))
                                            .alignment(alignment)
                                            .enabled(false),
                                        DropdownMenuItem::new(Text::new("enabled"))
                                            .alignment(alignment)
                                            .value("enabled".to_owned()),
                                    ]),
                                    Some(on_changed()),
                                )
                                .key(button_key.clone().into())
                                .alignment(alignment)
                                .value(Some("enabled".to_owned())),
                            ),
                        )
                        .into_widget(),
                )
            };
            let local_to_global = |tester: &WidgetTester, render: AnyRenderBox, offset: Offset| {
                render.local_to_global(&tester.cell.borrow(), offset, None)
            };

            let root = build(&tester, None);
            tester.pump_widget(root);

            let button_box = key_box(&tester, &button_key);
            let mut selected_item_box =
                tester.render_box_of(only(tester.find_onstage_text("enabled")));
            // Default to center-start aligned.
            assert_eq!(
                local_to_global(
                    &tester,
                    button_box,
                    Offset::new(0.0, size_of(&tester, button_box).height() / 2.0)
                ),
                local_to_global(
                    &tester,
                    selected_item_box,
                    Offset::new(0.0, size_of(&tester, selected_item_box).height() / 2.0)
                )
            );

            let root = build(&tester, Some(AlignmentGeometry::CENTER));
            tester.pump_widget(root);

            selected_item_box = tester.render_box_of(only(tester.find_onstage_text("enabled")));
            // Should be center-center aligned, the icon size is 24.0 pixels.
            let button_size = size_of(&tester, button_box);
            let item_size = size_of(&tester, selected_item_box);
            let expected = local_to_global(
                &tester,
                button_box,
                Offset::new(
                    (button_size.width() - 24.0) / 2.0,
                    button_size.height() / 2.0,
                ),
            );
            let actual = local_to_global(
                &tester,
                selected_item_box,
                Offset::new(item_size.width() / 2.0, item_size.height() / 2.0),
            );
            // Dart's `offsetMoreOrLessEquals`.
            assert_near(actual.dx(), expected.dx());
            assert_near(actual.dy(), expected.dy());

            // Open dropdown.
            tap_hit_testable_text(&mut tester, "enabled");
            tester.pump_and_settle();

            let selected_item_box_in_menu = tester.render_box_of(tester.find_text("enabled")[1]);
            let menu_rect = menu_rect_any(&tester);
            let in_menu_size = size_of(&tester, selected_item_box_in_menu);
            let center = local_to_global(
                &tester,
                selected_item_box_in_menu,
                Offset::new(in_menu_size.width() / 2.0, in_menu_size.height() / 2.0),
            );

            assert_near(center.dx(), menu_rect.top_center().dx());
            let item_size = size_of(&tester, selected_item_box);
            let button_item_center = local_to_global(
                &tester,
                selected_item_box,
                Offset::new(item_size.width() / 2.0, item_size.height() / 2.0),
            );
            assert_near(center.dy(), button_item_center.dy());
        });
    }

    /// Dart's `tester.getRect(find.byWidgetPredicate(runtimeType starts with '_DropdownMenu<'))`.
    fn menu_rect_any(tester: &WidgetTester) -> Rect {
        let menu = only(tester.find_by_type::<DropdownMenu<String>>());
        tester.get_rect(tester.render_box_of(menu))
    }

    fn cursor(name: waft_services::SystemMouseCursor) -> MouseCursorRef {
        name.into()
    }

    fn one_item_app(tester: &WidgetTester, button: DropdownButton<String>) -> WidgetRef {
        material_app(
            ThemeData::new(&tester.cell.borrow()),
            Material::new().child(button).into_widget(),
        )
    }

    #[test]
    fn dropdown_menu_item_has_expected_default_mouse_cursor_on_hover() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let menu_key = GlobalKey::new();
            let item_key = key("testDropdownMenuItem");
            let root = one_item_app(
                &tester,
                DropdownButton::new(
                    Some(vec![
                        DropdownMenuItem::new(Text::new("TestDropdownMenuItem"))
                            .key(item_key.clone())
                            .value("testDropdownMenuItem".to_owned()),
                    ]),
                    Some(on_changed()),
                )
                .key(menu_key.clone().into()),
            );
            tester.pump_widget(root);

            // Open DropdownButton.
            tap_key(&mut tester, &menu_key);
            tester.pump(Duration::ZERO);

            // Find DropdownMenuItem.
            let item = tester.render_box_of(only(tester.find_onstage_by_key(&item_key)));
            let on_menu_item = rect_of(&tester, item).center();
            let off_menu_item = rect_of(&tester, item).bottom_right() + Offset::new(1.0, 1.0);
            let mut gesture = WidgetTester::mouse(1, 0);

            tester.add_pointer(&mut gesture, on_menu_item);
            tester.pump(Duration::ZERO);

            let expected = if K_IS_WEB {
                cursor(waft_services::SystemMouseCursors::CLICK)
            } else {
                cursor(waft_services::SystemMouseCursors::BASIC)
            };
            assert!(*tester.active_cursor(1) == *expected);

            tester.move_to(&mut gesture, off_menu_item, false);

            assert!(*tester.active_cursor(1) == *cursor(waft_services::SystemMouseCursors::BASIC));
        });
    }

    #[test]
    fn dropdown_button_changes_mouse_cursor_when_hovered_as_expected() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let button = |on_changed: Option<ValueChanged<Option<String>>>| {
                DropdownButton::new(Some(items_of(&["One", "Two", "Three", "Four"])), on_changed)
                    .key(key.clone().into())
            };
            let root = one_item_app(&tester, button(Some(super::tests::on_changed())));
            tester.pump_widget(root);

            let button_rect = rect_of(&tester, key_box(&tester, &key));
            let on_dropdown_button = button_rect.center();
            let off_dropdown_button = button_rect.bottom_right() + Offset::new(1.0, 1.0);
            let mut gesture = WidgetTester::mouse(1, 0);

            tester.add_pointer(&mut gesture, on_dropdown_button);

            tester.pump(Duration::ZERO);

            let expected = if K_IS_WEB {
                cursor(waft_services::SystemMouseCursors::CLICK)
            } else {
                cursor(waft_services::SystemMouseCursors::BASIC)
            };
            assert!(*tester.active_cursor(1) == *expected);

            tester.move_to(&mut gesture, off_dropdown_button, false);
            assert!(*tester.active_cursor(1) == *cursor(waft_services::SystemMouseCursors::BASIC));

            // Test that mouse cursor doesn't change when button is disabled
            let root = one_item_app(&tester, button(None));
            tester.pump_widget(root);

            tester.move_to(&mut gesture, on_dropdown_button, false);
            assert!(*tester.active_cursor(1) == *cursor(waft_services::SystemMouseCursors::BASIC));
            tester.move_to(&mut gesture, off_dropdown_button, false);
            assert!(*tester.active_cursor(1) == *cursor(waft_services::SystemMouseCursors::BASIC));
        });
    }

    #[test]
    fn dropdown_button_has_expected_mouse_cursor_when_explicitly_configured() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let menu_key = GlobalKey::new();
            let root = one_item_app(
                &tester,
                DropdownButton::new(Some(items_of(&["One"])), Some(on_changed()))
                    .key(menu_key.clone().into())
                    .mouse_cursor(cursor(waft_services::SystemMouseCursors::CELL))
                    .dropdown_menu_item_mouse_cursor(cursor(
                        waft_services::SystemMouseCursors::GRAB,
                    )),
            );
            tester.pump_widget(root);

            let on_dropdown_button = tester.center_of(&menu_key);
            let mut gesture = WidgetTester::mouse(1, 0);

            tester.add_pointer(&mut gesture, on_dropdown_button);
            tester.pump(Duration::ZERO);

            assert!(*tester.active_cursor(1) == *cursor(waft_services::SystemMouseCursors::CELL));
        });
    }

    #[test]
    fn dropdown_menu_item_has_expected_mouse_cursor_when_explicitly_configured() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let menu_key = GlobalKey::new();
            let item_key = key("testDropdownMenuItem");
            let root = one_item_app(
                &tester,
                DropdownButton::new(
                    Some(vec![
                        DropdownMenuItem::new(Text::new("One"))
                            .key(item_key.clone())
                            .value("One".to_owned()),
                    ]),
                    Some(on_changed()),
                )
                .key(menu_key.clone().into())
                .dropdown_menu_item_mouse_cursor(cursor(waft_services::SystemMouseCursors::GRAB)),
            );
            tester.pump_widget(root);

            let mut gesture = WidgetTester::mouse(1, 0);

            // Open DropdownButton.
            tap_key(&mut tester, &menu_key);
            tester.pump_and_settle();

            // Find DropdownMenuItem.
            let item = tester.render_box_of(only(tester.find_onstage_by_key(&item_key)));
            let on_menu_item = rect_of(&tester, item).center();

            tester.add_pointer(&mut gesture, on_menu_item);
            tester.pump(Duration::ZERO);

            assert!(*tester.active_cursor(1) == *cursor(waft_services::SystemMouseCursors::GRAB));
        });
    }

    /// Dart's `paints..save()..rrect()..rrect()..rrect()..rrect(rrect: rrect)` on the menu's
    /// painter: after the elevation's three blurred shadows it fills `rrect`.
    fn menu_painter_fills(tester: &WidgetTester, rrect: waft_embedder::RRect) {
        use waft_embedder::valo::Op;
        let menu = only(tester.find_by_type::<DropdownMenu<String>>());
        let custom_paint = *tester
            .descendants_of_type::<CustomPaint>(menu)
            .first()
            .expect("the menu's painter");
        let render = tester.render_box_of(custom_paint).as_object();
        let ops = tester.painted_ops(render);
        assert!(matches!(ops.first(), Some(Op::Save)));
        let rrects = ops
            .iter()
            .filter(|op| matches!(op, Op::DrawPath { .. } | Op::RRectBlur { .. }))
            .count();
        assert!(rrects >= 4);
        assert!(tester.paints_rrect(render, rrect, None));
    }

    #[test]
    fn border_radius_property_works_properly() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let radius = 20.0;
            let root = material_app(
                m2(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(
                        Center::new().child(
                            DropdownButton::new(
                                Some(items_of(&["One", "Two", "Three", "Four"])),
                                Some(on_changed()),
                            )
                            .border_radius(BorderRadius::all(waft_embedder::Radius::circular(
                                radius,
                            )))
                            .value(Some("One".to_owned())),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            tap_text(&mut tester, "One");
            tester.pump_and_settle();

            menu_painter_fills(
                &tester,
                waft_embedder::RRect::from_ltrbxy(0.0, 0.0, 144.0, 208.0, radius, radius),
            );
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/88574
    #[test]
    fn specifying_item_height_affects_popup_menu_items_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let item_height = 80.0;
            let keys: Vec<KeyRef> = ["One", "Two", "Free", "Four"]
                .iter()
                .map(|value| key(value))
                .collect();
            let menu_items: Vec<DropdownMenuItem<String>> = ["One", "Two", "Free", "Four"]
                .iter()
                .zip(&keys)
                .map(|(value, key)| {
                    DropdownMenuItem::new(Text::new(*value))
                        .key(key.clone())
                        .value((*value).to_owned())
                })
                .collect();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(
                        Center::new().child(
                            DropdownButton::new(Some(menu_items), Some(on_changed()))
                                .value(Some("One".to_owned()))
                                .item_height(Some(item_height)),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            tap_text(&mut tester, "One");
            tester.pump_and_settle();

            for key in &keys {
                for element in all_by_key(&tester, key) {
                    let height = size_of(&tester, tester.render_box_of(element)).height();
                    assert_eq!(height, item_height);
                }
            }
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/92438
    #[test]
    fn do_not_throw_due_to_the_double_precision() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(
                        Center::new().child(
                            DropdownButton::new(
                                Some(items_of(&["One", "Two", "Free"])),
                                Some(on_changed()),
                            )
                            .value(Some("One".to_owned()))
                            .item_height(Some(77.701)),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            // Opening the menu and settling does not panic.
            tap_text(&mut tester, "One");
            tester.pump_and_settle();
        });
    }

    fn border_radius_form_field_app(tester: &WidgetTester, radius: f64) -> WidgetRef {
        material_app(
            ThemeData::new(&tester.cell.borrow()),
            crate::scaffold::Scaffold::new()
                .body(
                    Center::new().child(
                        DropdownButtonFormField::new(Some(items_of(&[
                            "One", "Two", "Three", "Four",
                        ])))
                        .border_radius(BorderRadius::all(waft_embedder::Radius::circular(radius)))
                        .initial_value(Some("One".to_owned()))
                        .on_changed(|_app, _value| {}),
                    ),
                )
                .into_widget(),
        )
    }

    #[test]
    fn border_radius_property_works_properly_for_dropdown_button_form_field() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let radius = 20.0;
            let root = border_radius_form_field_app(&tester, radius);
            tester.pump_widget(root);

            tap_text(&mut tester, "One");
            tester.pump_and_settle();

            menu_painter_fills(
                &tester,
                waft_embedder::RRect::from_ltrbxy(0.0, 0.0, 800.0, 208.0, radius, radius),
            );
        });
    }

    #[test]
    fn dropdown_button_hint_alignment() {
        hint_alignment_case(false);
    }

    #[test]
    fn dropdown_button_hint_alignment_with_selected_item_builder() {
        hint_alignment_case(true);
    }

    /// An alignment, whether the button is expanded, the corner of the hint's rect a case
    /// reads, and where that corner is.
    type HintAlignmentCase = (AlignmentGeometry, bool, fn(Rect) -> Offset, Offset);

    fn hint_alignment_case(enable_selected_item_builder: bool) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            let hint_text = "hint";
            // Dart's `buildDropdownWithHint`.
            let build = |alignment: AlignmentGeometry, is_expanded: bool| {
                let mut frame = frame()
                    .use_material3(false)
                    .media_size(Size::new(800.0, 600.0))
                    .item_height(Some(100.0))
                    .alignment(alignment)
                    .expanded(is_expanded)
                    .hint(Text::new(hint_text).into_widget());
                if enable_selected_item_builder {
                    frame = frame.selected_item_builder(Rc::new(|_app, _context| {
                        MENU_ITEMS
                            .iter()
                            .map(|item| {
                                waft_widgets::ColoredBox::new(Color::new(0xff00ff00))
                                    .child(Text::new(*item))
                                    .into_widget()
                            })
                            .collect()
                    }));
                }
                frame
            };
            let hint_rect = |tester: &WidgetTester| {
                tester.get_rect(tester.render_box_of(tester.find_text(hint_text)[0]))
            };
            let cases: [HintAlignmentCase; 14] = [
                // AlignmentDirectional.centerStart (default)
                (
                    AlignmentGeometry::CENTER_START,
                    false,
                    |r| r.top_left(),
                    Offset::new(348.0, 292.0),
                ),
                (
                    AlignmentGeometry::TOP_START,
                    false,
                    |r| r.top_left(),
                    Offset::new(348.0, 250.0),
                ),
                (
                    AlignmentGeometry::BOTTOM_START,
                    false,
                    |r| r.bottom_left(),
                    Offset::new(348.0, 350.0),
                ),
                (
                    AlignmentGeometry::CENTER,
                    false,
                    |r| r.center(),
                    Offset::new(388.0, 300.0),
                ),
                (
                    AlignmentGeometry::TOP_END,
                    false,
                    |r| r.top_right(),
                    Offset::new(428.0, 250.0),
                ),
                (
                    AlignmentGeometry::CENTER_END,
                    false,
                    |r| r.top_right(),
                    Offset::new(428.0, 292.0),
                ),
                (
                    AlignmentGeometry::BOTTOM_END,
                    false,
                    |r| r.top_right(),
                    Offset::new(428.0, 334.0),
                ),
                // DropdownButton with `isExpanded: true`
                (
                    AlignmentGeometry::CENTER_START,
                    true,
                    |r| r.top_left(),
                    Offset::new(0.0, 292.0),
                ),
                (
                    AlignmentGeometry::TOP_START,
                    true,
                    |r| r.top_left(),
                    Offset::new(0.0, 250.0),
                ),
                (
                    AlignmentGeometry::BOTTOM_START,
                    true,
                    |r| r.bottom_left(),
                    Offset::new(0.0, 350.0),
                ),
                (
                    AlignmentGeometry::CENTER,
                    true,
                    |r| r.center(),
                    Offset::new(388.0, 300.0),
                ),
                (
                    AlignmentGeometry::TOP_END,
                    true,
                    |r| r.top_right(),
                    Offset::new(776.0, 250.0),
                ),
                (
                    AlignmentGeometry::CENTER_END,
                    true,
                    |r| r.top_right(),
                    Offset::new(776.0, 292.0),
                ),
                (
                    AlignmentGeometry::BOTTOM_END,
                    true,
                    |r| r.bottom_right(),
                    Offset::new(776.0, 350.0),
                ),
            ];
            for (alignment, is_expanded, corner, expected) in cases {
                pump_frame(&mut tester, &build(alignment, is_expanded));
                assert_eq!(
                    corner(hint_rect(&tester)),
                    expected,
                    "{alignment:?} {is_expanded}"
                );
            }
        });
    }

    fn decoration_hint_frame() -> Frame {
        let mut frame = frame().form_field(true);
        frame.decoration = Some(InputDecoration::new().hint_text("Decoration Hint text"));
        frame
    }

    #[test]
    fn dropdown_button_form_field_decoration_hint_text_is_the_fallback_value_for_hint() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_frame(
                &mut tester,
                &decoration_hint_frame().on_changed(on_changed()),
            );

            assert_eq!(tester.find_text("Decoration Hint text").len(), 1);
        });
    }

    #[test]
    fn dropdown_button_form_field_decoration_hint_text_does_not_override_hint() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_frame(
                &mut tester,
                &decoration_hint_frame()
                    .hint(Text::new("Hint text").into_widget())
                    .on_changed(on_changed()),
            );

            assert_eq!(tester.find_text("Hint text").len(), 1);
            assert!(tester.find_text("Decoration Hint text").is_empty());
        });
    }

    #[test]
    fn dropdown_button_form_field_decoration_hint_text_is_the_fallback_value_for_disabled_hint() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // The Dropdown is disabled because onChanged is not defined.
            pump_frame(&mut tester, &decoration_hint_frame());

            assert_eq!(tester.find_text("Decoration Hint text").len(), 1);
        });
    }

    #[test]
    fn dropdown_button_form_field_decoration_hint_text_does_not_override_disabled_hint() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // The Dropdown is disabled because onChanged is not defined.
            pump_frame(
                &mut tester,
                &decoration_hint_frame()
                    .disabled_hint(Text::new("Disabled Hint text").into_widget()),
            );

            assert_eq!(tester.find_text("Disabled Hint text").len(), 1);
            assert!(tester.find_text("Decoration Hint text").is_empty());
        });
    }

    #[test]
    fn dropdown_button_form_field_decoration_hint_text_is_not_used_for_disabled_hint_if_hint_is_provided()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // The Dropdown is disabled because onChanged is not defined.
            pump_frame(
                &mut tester,
                &decoration_hint_frame().hint(Text::new("Hint text").into_widget()),
            );

            assert_eq!(tester.find_text("Hint text").len(), 1);
            assert!(tester.find_text("Decoration Hint text").is_empty());
        });
    }

    #[test]
    fn border_radius_property_clips_dropdown_menu() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let radius = 20.0;
            let root = border_radius_form_field_app(&tester, radius);
            tester.pump_widget(root);

            tap_text(&mut tester, "One");
            tester.pump_and_settle();

            let render_clip = tester.first::<waft_rendering::RenderClipRRect>();
            assert_eq!(
                render_clip.border_radius(&tester.cell.borrow()),
                BorderRadius::all(waft_embedder::Radius::circular(radius)).into()
            );
        });
    }

    #[test]
    fn size_of_dropdown_button_with_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let pad_vertical = 5.0;
            let pad_horizontal = 10.0;
            let button_key = GlobalKey::new();

            pump_frame(
                &mut tester,
                &frame().button_key(&button_key).on_changed(on_changed()),
            );
            let no_padding_size = size_of(&tester, key_box(&tester, &button_key));

            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    .on_changed(on_changed())
                    .padding(EdgeInsetsGeometry::symmetric(pad_vertical, pad_horizontal)),
            );
            let padded_size = size_of(&tester, key_box(&tester, &button_key));

            // dropdowns with padding should be that much larger than with no padding
            assert_eq!(
                no_padding_size.height(),
                padded_size.height() - pad_vertical * 2.0
            );
            assert_eq!(
                no_padding_size.width(),
                padded_size.width() - pad_horizontal * 2.0
            );
        });
    }

    fn barrier_case(barrier_dismissible: Option<bool>, expected_second: usize) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            let mut button = DropdownButton::new(
                Some(vec![
                    DropdownMenuItem::new(Text::new("disabled")).enabled(false),
                    DropdownMenuItem::new(Text::new("first")).value("first".to_owned()),
                    DropdownMenuItem::new(Text::new("second")).value("second".to_owned()),
                ]),
                Some(on_changed()),
            )
            .value(Some("first".to_owned()));
            if let Some(barrier_dismissible) = barrier_dismissible {
                button = button.barrier_dismissible(barrier_dismissible);
            }
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new().body(button).into_widget(),
            );
            tester.pump_widget(root);

            // Open dropdown.
            tap_hit_testable_text(&mut tester, "first");
            tester.pump_and_settle();

            // Tap on the barrier.
            tester.tap(Offset::new(400.0, 400.0), 1);
            tester.pump_and_settle();

            assert_eq!(shows(&tester, "second"), expected_second);
        });
    }

    #[test]
    fn dropdown_closes_when_barrier_is_tapped_by_default() {
        // The dropdown should be closed, i.e., there should be no widget with 'second' text.
        barrier_case(None, 0);
    }

    #[test]
    fn dropdown_does_not_close_when_barrier_dismissible_set_to_false() {
        // The dropdown should still be open, i.e., there should be one widget with 'second' text.
        barrier_case(Some(false), 1);
    }

    // This is a regression test for https://github.com/flutter/flutter/issues/70294.
    fn the_previous_selected_item_should_be_highlighted_when_reopening_dropdown(
        platform: TargetPlatform,
    ) {
        with_main_thread_stack(move || {
            let mut tester = tester_on(platform);
            let selected_color = Colors::BLACK.with_values(Some(0.12), None, None, None, None);
            let current_value = Rc::new(RefCell::new("one".to_owned()));
            let root = material_app(
                ThemeData::builder()
                    .focus_color(selected_color)
                    .build(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(Center::new().child(StatefulBuilder::new(
                        move |_app, _context, set_state| {
                            let set_state = set_state.clone();
                            let set_value = Rc::clone(&current_value);
                            DropdownButton::new(
                                Some(items_of(&MENU_ITEMS)),
                                Some(Rc::new(move |app, new_value: Option<String>| {
                                    let set_value = Rc::clone(&set_value);
                                    set_state(app, &mut |_app| {
                                        *set_value.borrow_mut() =
                                            new_value.clone().expect("a value");
                                    });
                                })),
                            )
                            .value(Some(current_value.borrow().clone()))
                            .into_widget()
                        },
                    )))
                    .into_widget(),
            );
            tester.pump_widget(root);

            // Make sure the current value of dropdown is the first one of items list menuItems.
            assert_eq!(shows(&tester, "one"), 1);

            // Tap to open the dropdown.
            tap_text(&mut tester, "one");
            tester.pump_and_settle();

            // Select the second item from the dropdown list.
            tap_text(&mut tester, "two");
            tester.pump_and_settle();

            // Make sure the current item of dropdown is the second item of items list menuItems.
            assert_eq!(shows(&tester, "two"), 1);

            // Tap to reopen the dropdown.
            tap_text(&mut tester, "two");
            tester.pump_and_settle();

            // Make sure the current selected item is highlighted with selectedColor.
            let ink = tester
                .find_onstage_text("two")
                .into_iter()
                .find_map(|two| {
                    let ancestors = tester.ancestors(two);
                    let app = tester.cell.borrow();
                    ancestors.into_iter().find(|ancestor| {
                        waft_widgets::downcast_widget::<Ink>(&**ancestor.widget(&app)).is_some()
                    })
                })
                .expect("an Ink above a 'two'");
            let color = tester.widget(ink, |ink: &Ink| {
                ink.effective_decoration()
                    .as_ref()
                    .and_then(|decoration| decoration.as_any().downcast_ref::<BoxDecoration>())
                    .and_then(|decoration| decoration.color.clone())
                    .map(|color| color.color())
            });
            assert_eq!(color, Some(selected_color));
        });
    }

    #[test]
    fn the_previous_selected_item_should_be_highlighted_when_reopening_dropdown_on_android() {
        the_previous_selected_item_should_be_highlighted_when_reopening_dropdown(
            TargetPlatform::Android,
        );
    }

    #[test]
    fn the_previous_selected_item_should_be_highlighted_when_reopening_dropdown_on_ios() {
        the_previous_selected_item_should_be_highlighted_when_reopening_dropdown(
            TargetPlatform::IOS,
        );
    }

    #[test]
    fn the_previous_selected_item_should_be_highlighted_when_reopening_dropdown_on_fuchsia() {
        the_previous_selected_item_should_be_highlighted_when_reopening_dropdown(
            TargetPlatform::Fuchsia,
        );
    }

    #[test]
    #[allow(deprecated)]
    fn dropdown_button_form_field_deprecated_value_parameter_can_still_be_used_to_set_the_initial_value()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Material::new()
                    .child(
                        DropdownButtonFormField::new(Some(items_of(&MENU_ITEMS)))
                            .value(Some("one".to_owned()))
                            .hint(Text::new("Select Value"))
                            .on_changed(|_app, _value| {}),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);
            let state = form_field_state(&tester);
            assert_eq!(
                state.value(&tester.cell.borrow()).map(String::as_str),
                Some("one")
            );
        });
    }

    fn zero_area_app(tester: &WidgetTester, child: WidgetRef) -> WidgetRef {
        material_app(
            ThemeData::new(&tester.cell.borrow()),
            crate::scaffold::Scaffold::new()
                .body(Center::new().child(SizedBox::shrink().child(child)))
                .into_widget(),
        )
    }

    #[test]
    fn dropdown_button_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = zero_area_app(
                &tester,
                DropdownButton::new(Some(items_of(&["a"])), Some(on_changed()))
                    .value(Some("a".to_owned()))
                    .into_widget(),
            );
            tester.pump_widget(root);
            let button = only(tester.find_by_type::<DropdownButton<String>>());
            assert_eq!(size_of(&tester, tester.render_box_of(button)), Size::ZERO);
        });
    }

    #[test]
    fn dropdown_button_form_field_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = zero_area_app(
                &tester,
                DropdownButtonFormField::new(Some(items_of(&["a"])))
                    .on_changed(|_app, _value| {})
                    .into_widget(),
            );
            tester.pump_widget(root);
            let field = only(tester.find_by_type::<DropdownButtonFormField<String>>());
            assert_eq!(size_of(&tester, tester.render_box_of(field)), Size::ZERO);
        });
    }

    #[test]
    fn dropdown_menu_item_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = zero_area_app(
                &tester,
                DropdownMenuItem::new(Text::new("a"))
                    .value("a".to_owned())
                    .into_widget(),
            );
            tester.pump_widget(root);
            let item = only(tester.find_by_type::<DropdownMenuItem<String>>());
            assert_eq!(size_of(&tester, tester.render_box_of(item)), Size::ZERO);
        });
    }

    #[test]
    fn dropdown_button_form_field_can_inherit_from_local_input_decoration_theme_data() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let label_text = "Label";
            let label_color = Colors::GREEN.primary();
            let decoration = InputDecoration::new().label_text(label_text);
            let decoration_theme = crate::input_decorator::InputDecorationThemeData::new()
                .label_style(TextStyle::new().color(label_color));

            let mut frame = frame().form_field(true).on_changed(on_changed());
            frame.decoration = Some(decoration);
            let dropdown = frame.dropdown();
            let root = Theme::new(
                ThemeData::new(&tester.cell.borrow()),
                test_app(TextDirection::Ltr, None, move || {
                    Material::new()
                        .child(
                            waft_widgets::Align::new().child(
                                RepaintBoundary::new().child(
                                    InputDecorationTheme::new()
                                        .with_data(decoration_theme.clone())
                                        .child(dropdown.clone()),
                                ),
                            ),
                        )
                        .into_widget()
                }),
            )
            .into_widget();
            tester.pump_widget(root);

            let label = tester.find_text(label_text)[0];
            let label_style =
                waft_widgets::DefaultTextStyle::of(&mut tester.cell.borrow_mut(), label).style;
            assert_eq!(
                label_style.color.map(|color| color.color()),
                Some(label_color)
            );
        });
    }

    #[test]
    #[should_panic(
        expected = "The selectedItemBuilder must return a list of widgets with the same length as the items list.\nCurrently, selectedItemBuilder returns a list of length 1, but items has length 2."
    )]
    fn dropdown_button_selected_item_builder_length_must_match_items_length() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/92773
            let mut tester = tester();
            let root = zero_area_app(
                &tester,
                DropdownButtonFormField::new(Some(items_of(&["a", "b"])))
                    .on_changed(|_app, _value| {})
                    .selected_item_builder(|_app, _context| vec![Text::new("a").into_widget()])
                    .into_widget(),
            );
            tester.pump_widget(root);
        });
    }

    #[test]
    #[should_panic(
        expected = "Declaring both errorBuilder and decoration.errorText is not supported."
    )]
    fn dropdown_button_form_field_asserts_when_both_error_builder_and_decoration_error_text_are_provided()
     {
        let _ = DropdownButtonFormField::<String>::new(Some(Vec::new()))
            .on_changed(|_app, _value| {})
            .decoration(InputDecoration::new().error_text("Decoration error"))
            .error_builder(|_app, _context, error_text| Text::new(error_text).into_widget())
            .into_widget();
    }

    // -----------------------------------------------------------------------------------------
    // dropdown_form_field_test.dart

    /// The arguments of dropdown_form_field_test.dart's `buildFormFrame`, with its defaults.
    #[derive(Clone)]
    struct FormFrame {
        button_key: Option<GlobalKey>,
        autovalidate_mode: AutovalidateMode,
        elevation: i32,
        value: Option<String>,
        on_changed: Option<ValueChanged<Option<String>>>,
        on_tap: Option<Listener>,
        icon: Option<WidgetRef>,
        icon_disabled_color: Option<Color>,
        icon_enabled_color: Option<Color>,
        icon_size: f64,
        is_dense: bool,
        is_expanded: bool,
        hint: Option<WidgetRef>,
        disabled_hint: Option<WidgetRef>,
        items: Option<Vec<String>>,
        alignment: AlignmentGeometry,
        text_direction: TextDirection,
        button_alignment: AlignmentGeometry,
    }

    fn form_frame() -> FormFrame {
        FormFrame {
            button_key: None,
            autovalidate_mode: AutovalidateMode::Disabled,
            elevation: 8,
            value: Some("two".to_owned()),
            on_changed: None,
            on_tap: None,
            icon: None,
            icon_disabled_color: None,
            icon_enabled_color: None,
            icon_size: 24.0,
            is_dense: true,
            is_expanded: false,
            hint: None,
            disabled_hint: None,
            items: Some(strings(&MENU_ITEMS)),
            alignment: AlignmentGeometry::CENTER,
            text_direction: TextDirection::Ltr,
            button_alignment: AlignmentGeometry::CENTER_START,
        }
    }

    impl FormFrame {
        /// Dart's `buildFormFrame`.
        fn build(&self) -> WidgetRef {
            let items = self.items.as_ref().map(|items| {
                items
                    .iter()
                    .map(|item| {
                        DropdownMenuItem::new(
                            Text::new(item.clone()).key(key(&format!("{item}Text"))),
                        )
                        .key(key(item))
                        .value(item.clone())
                    })
                    .collect()
            });
            let mut field = DropdownButtonFormField::new(items)
                .autovalidate_mode(self.autovalidate_mode)
                .elevation(self.elevation)
                .initial_value(self.value.clone())
                .icon_size(self.icon_size)
                .is_dense(self.is_dense)
                .is_expanded(self.is_expanded)
                .alignment(self.button_alignment);
            if let Some(button_key) = &self.button_key {
                field = field.key(button_key.clone().into());
            }
            if let Some(hint) = &self.hint {
                field = field.hint(hint.clone());
            }
            if let Some(disabled_hint) = &self.disabled_hint {
                field = field.disabled_hint(disabled_hint.clone());
            }
            if let Some(on_changed) = self.on_changed.clone() {
                field = field.on_changed(move |app, value| on_changed(app, value));
            }
            if let Some(on_tap) = self.on_tap.clone() {
                field = field.on_tap(on_tap);
            }
            if let Some(icon) = &self.icon {
                field = field.icon(icon.clone());
            }
            if let Some(color) = self.icon_disabled_color {
                field = field.icon_disabled_color(color);
            }
            if let Some(color) = self.icon_enabled_color {
                field = field.icon_enabled_color(color);
            }
            form_test_app(
                self.text_direction,
                Material::new()
                    .child(
                        waft_widgets::Align::new()
                            .alignment(self.alignment)
                            .child(RepaintBoundary::new().child(field)),
                    )
                    .into_widget(),
            )
        }
    }

    fn pump_form_frame(tester: &mut WidgetTester, frame: &FormFrame) {
        tester.pump_widget(frame.build());
    }

    fn label_top_left(tester: &WidgetTester) -> Offset {
        tester
            .get_rect(tester.render_box_of(only(tester.find_onstage_text("labelText"))))
            .top_left()
    }

    fn int_option_items(values: &[(Option<i32>, &str)]) -> Vec<DropdownMenuItem<i32>> {
        values
            .iter()
            .map(|(value, label)| {
                let item = DropdownMenuItem::new(Text::new(*label));
                match value {
                    Some(value) => item.value(*value),
                    None => item,
                }
            })
            .collect()
    }

    const ONE_TWO_THREE: [(Option<i32>, &str); 3] =
        [(Some(1), "One"), (Some(2), "Two"), (Some(3), "Three")];

    /// A label-position case's field, under the form tests' `TestApp`.
    fn label_field(field: DropdownButtonFormField<i32>) -> WidgetRef {
        form_test_app(
            TextDirection::Ltr,
            Material::new()
                .child(field.decoration(InputDecoration::new().label_text("labelText")))
                .into_widget(),
        )
    }

    // Regression test for https://github.com/flutter/flutter/issues/87102
    #[test]
    fn label_position_test_show_hint() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = Rc::new(Cell::new(None));
            let set_value = Rc::clone(&value);
            tester.pump_widget(label_field(
                DropdownButtonFormField::new(Some(int_option_items(&ONE_TWO_THREE)))
                    .initial_value(None)
                    .hint(Text::new("Hint"))
                    .on_changed(move |_app, new_value| set_value.set(new_value)),
            ));

            assert_eq!(value.get(), None);
            let hint_empty_label = label_top_left(&tester);

            // Select a item.
            tap_text(&mut tester, "Hint");
            tester.pump_and_settle();
            tap_last_text(&mut tester, "One");
            tester.pump_and_settle();

            assert_eq!(value.get(), Some(1));
            let one_value_label = label_top_left(&tester);

            // The position of the label does not change.
            assert_eq!(hint_empty_label, one_value_label);
        });
    }

    fn label_position_case(field: fn() -> DropdownButtonFormField<i32>, expected: Offset) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            tester.pump_widget(label_field(field()));
            assert_eq!(label_top_left(&tester), expected);
        });
    }

    #[test]
    fn label_position_test_show_disabled_hint_disable() {
        // onChanged is null by default, which disables the menu and shows the disabledHint.
        label_position_case(
            || {
                DropdownButtonFormField::new(Some(int_option_items(&ONE_TWO_THREE)))
                    .initial_value(None)
                    .disabled_hint(Text::new("disabledHint"))
            },
            Offset::new(0.0, 8.0),
        );
    }

    #[test]
    fn label_position_test_show_disabled_hint_enable_and_null_item() {
        label_position_case(
            || {
                DropdownButtonFormField::new(None)
                    .initial_value(None)
                    .disabled_hint(Text::new("disabledHint"))
                    .on_changed(|_app, _value| {})
            },
            Offset::new(0.0, 8.0),
        );
    }

    #[test]
    fn label_position_test_show_disabled_hint_enable_and_empty_item() {
        label_position_case(
            || {
                DropdownButtonFormField::new(Some(Vec::new()))
                    .initial_value(None)
                    .disabled_hint(Text::new("disabledHint"))
                    .on_changed(|_app, _value| {})
            },
            Offset::new(0.0, 8.0),
        );
    }

    #[test]
    fn label_position_test_show_hint_enable_and_empty_item() {
        label_position_case(
            || {
                DropdownButtonFormField::new(Some(Vec::new()))
                    .initial_value(None)
                    .hint(Text::new("hint"))
                    .on_changed(|_app, _value| {})
            },
            Offset::new(0.0, 8.0),
        );
    }

    #[test]
    fn label_position_test_no_hint_shown_enable_and_no_selected_and_disabled_hint() {
        label_position_case(
            || {
                DropdownButtonFormField::new(Some(int_option_items(&ONE_TWO_THREE)))
                    .initial_value(None)
                    .disabled_hint(Text::new("disabledHint"))
                    .on_changed(|_app, _value| {})
            },
            Offset::new(0.0, 20.0),
        );
    }

    #[test]
    fn label_position_test_show_selected_item_disabled_and_hint_and_disabled_hint() {
        // onChanged is null by default, so disabled
        label_position_case(
            || {
                DropdownButtonFormField::new(Some(int_option_items(&ONE_TWO_THREE)))
                    .initial_value(Some(1))
                    .hint(Text::new("hint"))
                    .disabled_hint(Text::new("disabledHint"))
            },
            Offset::new(0.0, 8.0),
        );
    }

    // Regression test for https://github.com/flutter/flutter/issues/82910
    #[test]
    fn null_value_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = Rc::new(Cell::new(Some(1)));
            let set_value = Rc::clone(&value);
            tester.pump_widget(label_field(
                DropdownButtonFormField::new(Some(int_option_items(&[
                    (None, "None"),
                    (Some(1), "One"),
                    (Some(2), "Two"),
                    (Some(3), "Three"),
                ])))
                .initial_value(Some(1))
                .on_changed(move |_app, new_value| set_value.set(new_value)),
            ));

            assert_eq!(value.get(), Some(1));
            let non_empty_label = label_top_left(&tester);

            // Switch to `null` value item from value 1.
            tap_text(&mut tester, "One");
            tester.pump_and_settle();
            tap_last_text(&mut tester, "None");
            tester.pump(Duration::ZERO);

            assert_eq!(value.get(), None);
            let null_value_label = label_top_left(&tester);
            // The position of the label does not change.
            assert_eq!(non_empty_label, null_value_label);
        });
    }

    #[test]
    fn dropdown_button_form_field_with_autovalidation_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = Rc::new(RefCell::new(Some("one".to_owned())));
            let validate_called = Rc::new(Cell::new(0));
            let state_value = Rc::clone(&value);
            let validated = Rc::clone(&validate_called);
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let set_state = set_state.clone();
                    let set_value = Rc::clone(&state_value);
                    let validated = Rc::clone(&validated);
                    Material::new()
                        .child(
                            DropdownButtonFormField::new(Some(items_of(&MENU_ITEMS)))
                                .initial_value(state_value.borrow().clone())
                                .hint(Text::new("Select Value"))
                                .decoration(
                                    InputDecoration::new()
                                        .prefix_icon(Icon::new(Some(Icons::fastfood()))),
                                )
                                .on_changed(move |app, new_value| {
                                    let set_value = Rc::clone(&set_value);
                                    set_state(app, &mut |_app| {
                                        *set_value.borrow_mut() = new_value.clone();
                                    });
                                })
                                .validator(move |_app, current_value| {
                                    validated.set(validated.get() + 1);
                                    current_value
                                        .is_none()
                                        .then(|| "Must select value".to_owned())
                                })
                                .autovalidate_mode(AutovalidateMode::Always),
                        )
                        .into_widget()
                })
                .into_widget(),
            );
            tester.pump_widget(root);

            assert_eq!(validate_called.get(), 1);
            assert_eq!(value.borrow().as_deref(), Some("one"));
            tap_text(&mut tester, "one");
            tester.pump_and_settle();
            tap_last_text(&mut tester, "three");
            tester.pump(Duration::ZERO);
            assert_eq!(validate_called.get(), 2);
            tester.pump_and_settle();
            assert_eq!(value.borrow().as_deref(), Some("three"));
        });
    }

    #[test]
    fn dropdown_button_form_field_arrow_icon_aligns_with_the_edge_of_button_when_expanded() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();

            // There shouldn't be overflow when expanded although list contains longer items.
            let items = strings(&[
                "1234567890",
                "abcdefghijklmnopqrstuvwxyz1234567890abcdefghijklmnopqrstuvwxyz1234567890",
            ]);

            let mut frame = form_frame();
            frame.button_key = Some(button_key.clone());
            frame.value = Some("1234567890".to_owned());
            frame.is_expanded = true;
            frame.on_changed = Some(on_changed());
            frame.items = Some(items);
            pump_form_frame(&mut tester, &frame);
            let button_box = key_box(&tester, &button_key);
            let arrow_icon = tester.render_box_of(tester.find_by_icon(&Icons::arrow_drop_down()));

            // Arrow icon should be aligned with far right of button when expanded
            let button_size = size_of(&tester, button_box);
            let arrow_size = size_of(&tester, arrow_icon);
            assert_eq!(
                global_top_left(&tester, arrow_icon).dx(),
                button_size
                    .center_right(Offset::new(-arrow_size.width(), 0.0))
                    .dx()
            );
        });
    }

    #[test]
    fn dropdown_button_form_field_with_is_dense_true_aligns_selected_menu_item() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let mut frame = form_frame();
            frame.button_key = Some(button_key.clone());
            frame.on_changed = Some(on_changed());
            pump_form_frame(&mut tester, &frame);
            let button_box = key_box(&tester, &button_key);

            tap_text(&mut tester, "two");
            settle_menu(&mut tester);

            // The selected dropdown item is both in menu we just popped up, and in
            // the IndexedStack contained by the dropdown button. Both of them should
            // have the same vertical center as the button.
            let item_boxes = boxes_of(&tester, "two");
            assert_eq!(item_boxes.len(), 2);

            // When isDense is true, the button's height is reduced. The menu items'
            // heights are not.
            let menu_item_height = item_boxes
                .iter()
                .map(|item_box| size_of(&tester, *item_box).height())
                .fold(f64::MIN, f64::max);
            assert!(menu_item_height >= size_of(&tester, button_box).height());

            for item_box in item_boxes {
                assert_eq!(
                    rect_of(&tester, button_box).center().dy(),
                    rect_of(&tester, item_box).center().dy()
                );
            }
        });
    }

    fn dropdown_button_height(tester: &WidgetTester) -> f64 {
        let button = only(tester.find_by_type::<DropdownButton<String>>());
        size_of(tester, tester.render_box_of(button)).height()
    }

    fn scaled_field(field: DropdownButtonFormField<String>, scale_factor: f64) -> WidgetRef {
        form_test_app(
            TextDirection::Ltr,
            Builder::new(move |_app, _context| {
                MediaQuery::with_clamped_text_scaling(
                    None,
                    scale_factor,
                    scale_factor,
                    Material::new().child(Center::new().child(field_clone(&field))),
                )
            })
            .into_widget(),
        )
    }

    /// A field built afresh from the same arguments, as Dart's builder closure rebuilds it.
    fn field_clone(field: &DropdownButtonFormField<String>) -> WidgetRef {
        let mut copy = DropdownButtonFormField::new(field.button.items.clone())
            .initial_value(field.initial_value.clone());
        copy.on_changed = field.on_changed.clone();
        copy.button = DropdownButton {
            key: field.button.key.clone(),
            items: field.button.items.clone(),
            value: field.button.value.clone(),
            hint: field.button.hint.clone(),
            disabled_hint: field.button.disabled_hint.clone(),
            on_changed: field.button.on_changed.clone(),
            on_tap: field.button.on_tap.clone(),
            selected_item_builder: field.button.selected_item_builder.clone(),
            elevation: field.button.elevation,
            style: field.button.style.clone(),
            underline: field.button.underline.clone(),
            icon: field.button.icon.clone(),
            icon_disabled_color: field.button.icon_disabled_color,
            icon_enabled_color: field.button.icon_enabled_color,
            icon_size: field.button.icon_size,
            is_dense: field.button.is_dense,
            is_expanded: field.button.is_expanded,
            item_height: field.button.item_height,
            menu_width: field.button.menu_width,
            focus_color: field.button.focus_color,
            focus_node: field.button.focus_node.clone(),
            autofocus: field.button.autofocus,
            dropdown_color: field.button.dropdown_color,
            menu_max_height: field.button.menu_max_height,
            enable_feedback: field.button.enable_feedback,
            alignment: field.button.alignment,
            border_radius: field.button.border_radius,
            padding: field.button.padding,
            barrier_dismissible: field.button.barrier_dismissible,
            mouse_cursor: field.button.mouse_cursor.clone(),
            dropdown_menu_item_mouse_cursor: field.button.dropdown_menu_item_mouse_cursor.clone(),
            is_vertically_expanded: field.button.is_vertically_expanded,
            input_decoration: None,
            is_empty: false,
        };
        copy.form_field.key = field.form_field.key.clone();
        copy.into_widget()
    }

    fn keyed_items() -> Vec<DropdownMenuItem<String>> {
        MENU_ITEMS
            .iter()
            .map(|item| {
                DropdownMenuItem::new(Text::new(*item).key(key(&format!("{item}Text"))))
                    .key(key(item))
                    .value((*item).to_owned())
            })
            .collect()
    }

    // Regression test for https://github.com/flutter/flutter/issues/159971.
    #[test]
    fn dropdown_button_form_field_does_not_clip_large_scale_text() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scale_factor = 3.0;
            tester.pump_widget(scaled_field(
                DropdownButtonFormField::new(Some(keyed_items()))
                    .initial_value(Some("two".to_owned()))
                    .on_changed(|_app, _value| {}),
                scale_factor,
            ));

            let button = only(tester.find_by_type::<DropdownButton<String>>());
            let style = Theme::of(&mut tester.cell.borrow_mut(), button)
                .text_theme
                .title_medium
                .clone()
                .expect("title medium");
            let line_height = style.font_size.expect("a size") * style.height.expect("a height"); // 16 * 1.5 = 24
            let label_height = line_height * scale_factor; // 24 * 3.0 = 72
            let decoration_vertical_padding = 16.0;
            assert_eq!(
                dropdown_button_height(&tester),
                label_height + decoration_vertical_padding
            );
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/159971.
    #[test]
    fn dropdown_button_form_field_with_custom_text_style_does_not_clip_large_scale_text() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scale_factor = 3.0;
            let font_size = 22.0;
            let font_height = 1.5;
            tester.pump_widget(scaled_field(
                DropdownButtonFormField::new(Some(keyed_items()))
                    .initial_value(Some("two".to_owned()))
                    .on_changed(|_app, _value| {})
                    .style(TextStyle::new().font_size(font_size).height(font_height)),
                scale_factor,
            ));

            let line_height = font_size * font_height; // 22 * 1.5 = 33
            let label_height = line_height * scale_factor; // 33 * 3.0 = 99
            let decoration_vertical_padding = 16.0;
            assert_eq!(
                dropdown_button_height(&tester),
                label_height + decoration_vertical_padding
            );
        });
    }

    #[test]
    fn dropdown_button_form_field_is_dense_is_true_by_default() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/46844
            let mut tester = tester();
            tester.pump_widget(form_test_app(
                TextDirection::Ltr,
                Material::new()
                    .child(
                        Center::new().child(
                            DropdownButtonFormField::new(Some(keyed_items()))
                                .initial_value(Some("two".to_owned()))
                                .on_changed(|_app, _value| {}),
                        ),
                    )
                    .into_widget(),
            ));

            assert_eq!(dropdown_button_height(&tester), 48.0);
        });
    }

    #[test]
    fn dropdown_button_form_field_custom_text_style() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let item_key = GlobalKey::new();
            tester.pump_widget(form_test_app(
                TextDirection::Ltr,
                Material::new()
                    .child(
                        DropdownButtonFormField::new(Some(vec![
                            DropdownMenuItem::new(Text::new("foo"))
                                .key(item_key.clone().into())
                                .value("foo".to_owned()),
                        ]))
                        .initial_value(Some("foo".to_owned()))
                        .on_changed(|_app, _value| {})
                        .style(
                            TextStyle::new()
                                .color(Colors::AMBER.primary())
                                .font_size(20.0),
                        ),
                    )
                    .into_widget(),
            ));

            let item = item_key
                .current_context(&tester.cell.borrow())
                .expect("the item is mounted");
            let rich_text = only(tester.descendants_of_type::<waft_widgets::RichText>(item));
            let style = tester.widget(rich_text, |rich_text: &waft_widgets::RichText| {
                rich_text.text.style().cloned().expect("a style")
            });
            assert_eq!(
                style.color.map(|color| color.color()),
                Some(Colors::AMBER.primary())
            );
            assert_eq!(style.font_size, Some(20.0));
        });
    }

    #[test]
    fn dropdown_button_form_field_disabled_hint_displays_when_the_items_list_is_empty_when_items_is_null()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let build = |items: Option<Vec<String>>| {
                let mut frame = form_frame();
                frame.items = items;
                frame.button_key = Some(button_key.clone());
                frame.value = None;
                frame.hint = Some(Text::new("enabled").into_widget());
                frame.disabled_hint = Some(Text::new("disabled").into_widget());
                frame
            };

            // [disabledHint] should display when [items] is null
            pump_form_frame(&mut tester, &build(None));
            assert_eq!(shows(&tester, "enabled"), 0);
            assert_eq!(shows(&tester, "disabled"), 1);

            // [disabledHint] should display when [items] is an empty list.
            pump_form_frame(&mut tester, &build(Some(Vec::new())));
            assert_eq!(shows(&tester, "enabled"), 0);
            assert_eq!(shows(&tester, "disabled"), 1);
        });
    }

    #[test]
    fn dropdown_button_form_field_hint_displays_when_the_items_list_is_empty_items_is_null_and_disabled_hint_is_null()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let build = |items: Option<Vec<String>>| {
                let mut frame = form_frame();
                frame.items = items;
                frame.button_key = Some(button_key.clone());
                frame.value = None;
                frame.hint = Some(Text::new("hint used when disabled").into_widget());
                frame
            };

            // [hint] should display when [items] is null and [disabledHint] is not defined
            pump_form_frame(&mut tester, &build(None));
            assert_eq!(shows(&tester, "hint used when disabled"), 1);

            // [hint] should display when [items] is an empty list and [disabledHint] is not defined.
            pump_form_frame(&mut tester, &build(Some(Vec::new())));
            assert_eq!(shows(&tester, "hint used when disabled"), 1);
        });
    }

    #[test]
    fn dropdown_button_form_field_disabled_hint_displays_when_on_changed_is_null() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let mut frame = form_frame();
            frame.button_key = Some(GlobalKey::new());
            frame.value = None;
            frame.hint = Some(Text::new("enabled").into_widget());
            frame.disabled_hint = Some(Text::new("disabled").into_widget());
            pump_form_frame(&mut tester, &frame);
            assert_eq!(shows(&tester, "enabled"), 0);
            assert_eq!(shows(&tester, "disabled"), 1);
        });
    }

    #[test]
    fn dropdown_button_form_field_disabled_hint_should_be_of_same_size_as_enabled_hint() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let build = |items: Option<Vec<String>>| {
                let mut frame = form_frame();
                frame.items = items;
                frame.button_key = Some(button_key.clone());
                frame.value = None;
                frame.hint = Some(Text::new("enabled").into_widget());
                frame.disabled_hint = Some(Text::new("disabled").into_widget());
                frame
            };

            pump_form_frame(&mut tester, &build(None));
            let disabled_hint_box = key_box(&tester, &button_key);

            pump_form_frame(&mut tester, &build(Some(strings(&MENU_ITEMS))));
            let enabled_hint_box = key_box(&tester, &button_key);
            assert_eq!(
                rect_of(&tester, enabled_hint_box),
                rect_of(&tester, disabled_hint_box)
            );
        });
    }

    #[test]
    fn dropdown_button_form_field_custom_icon_size_and_colors() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let icon_key = GlobalKey::new();
            let custom_icon = Icon::new(Some(Icons::assessment()))
                .key(icon_key.clone().into())
                .into_widget();
            let mut frame = form_frame();
            frame.icon = Some(custom_icon);
            frame.icon_size = 30.0;
            frame.icon_enabled_color = Some(Colors::PINK.primary());
            frame.icon_disabled_color = Some(Colors::ORANGE.primary());

            let mut enabled = frame.clone();
            enabled.on_changed = Some(on_changed());
            pump_form_frame(&mut tester, &enabled);

            // test for size
            assert_eq!(
                size_of(&tester, icon_box(&tester, &icon_key)),
                Size::new(30.0, 30.0)
            );

            // test for enabled color
            assert_eq!(icon_color(&tester, &icon_key), Some(Colors::PINK.primary()));

            // test for disabled color
            let mut disabled = frame;
            disabled.items = None;
            pump_form_frame(&mut tester, &disabled);
            assert_eq!(
                icon_color(&tester, &icon_key),
                Some(Colors::ORANGE.primary())
            );
        });
    }

    /// Dart's `verifyPaintedShadow`: the menu's painter draws the first three shadows of
    /// `elevation` as blurred rounded rects, each the menu's 800x208 rect shifted by its
    /// offset and inflated by its spread. A blurred rect is Valo's `RRectBlur`, where Dart's
    /// is an `rrect` call with a mask filter.
    fn verify_painted_shadow(tester: &WidgetTester, elevation: i32) {
        use waft_embedder::valo::Op;
        let original_rectangle = Rect::from_ltrb(0.0, 0.0, 800.0, 208.0);
        let box_shadows = &k_elevation_to_shadow(elevation).expect("a defined elevation")[..3];

        let menu = only(tester.find_by_type::<DropdownMenu<String>>());
        let custom_paint = *tester
            .descendants_of_type::<CustomPaint>(menu)
            .first()
            .expect("the menu's painter");
        let ops = tester.painted_ops(tester.render_box_of(custom_paint).as_object());
        assert!(matches!(ops.first(), Some(Op::Save)));
        for (index, op) in ops[1..4].iter().enumerate() {
            let shadow = &box_shadows[index];
            let expected = original_rectangle
                .shift(shadow.offset)
                .inflate(shadow.spread_radius);
            let Op::RRectBlur {
                rect, radii, paint, ..
            } = op
            else {
                panic!("a blurred shadow rect");
            };
            assert_eq!(
                (rect.x, rect.y, rect.width, rect.height),
                (
                    expected.left as f32,
                    expected.top as f32,
                    expected.width() as f32,
                    expected.height() as f32
                )
            );
            assert_eq!(*radii, [2.0; 4]);
            assert_eq!(paint.color, shadow.color.into());
        }
    }

    #[test]
    fn dropdown_button_form_field_default_elevation() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let mut frame = form_frame();
            frame.button_key = Some(button_key.clone());
            frame.on_changed = Some(on_changed());
            pump_form_frame(&mut tester, &frame);
            tap_key(&mut tester, &button_key);
            tester.pump_and_settle();

            // Verifying whether or not default elevation(i.e. 8) paints desired shadow
            verify_painted_shadow(&tester, 8);
        });
    }

    #[test]
    fn dropdown_button_form_field_custom_elevation() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key_one = GlobalKey::new();
            let button_key_two = GlobalKey::new();

            let mut frame = form_frame();
            frame.button_key = Some(button_key_one.clone());
            frame.elevation = 16;
            frame.on_changed = Some(on_changed());
            pump_form_frame(&mut tester, &frame);
            tap_key(&mut tester, &button_key_one);
            tester.pump_and_settle();

            verify_painted_shadow(&tester, 16);
            tap_last_text(&mut tester, "one");
            let mut frame = form_frame();
            frame.button_key = Some(button_key_two.clone());
            frame.elevation = 24;
            frame.on_changed = Some(on_changed());
            pump_form_frame(&mut tester, &frame);
            tap_key(&mut tester, &button_key_two);
            tester.pump_and_settle();

            verify_painted_shadow(&tester, 24);
        });
    }

    #[test]
    #[should_panic(expected = "There should be exactly one item with [DropdownButton]'s value")]
    fn dropdown_button_form_field_does_not_allow_duplicate_item_values() {
        let _ = DropdownButtonFormField::new(Some(items_of(&["a", "b", "c", "c"])))
            .initial_value(Some("c".to_owned()))
            .on_changed(|_app, _value| {})
            .into_widget();
    }

    #[test]
    fn dropdown_button_form_field_selected_item_builder_builds_custom_buttons() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let items = ["One", "Two", "Three"];
            let selected_item = Rc::new(RefCell::new(Some(items[0].to_owned())));
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let set_state = set_state.clone();
                    let selected = Rc::clone(&selected_item);
                    crate::scaffold::Scaffold::new()
                        .body(
                            DropdownButtonFormField::new(Some(items_of(&items)))
                                .initial_value(selected_item.borrow().clone())
                                .on_changed(move |app, string| {
                                    let selected = Rc::clone(&selected);
                                    set_state(app, &mut |_app| {
                                        *selected.borrow_mut() = string.clone();
                                    });
                                })
                                .selected_item_builder(move |_app, _context| {
                                    let mut index = 0;
                                    items
                                        .iter()
                                        .map(|string| {
                                            index += 1;
                                            Text::new(format!(
                                                "{string} as an Arabic numeral: {index}"
                                            ))
                                            .into_widget()
                                        })
                                        .collect()
                                }),
                        )
                        .into_widget()
                })
                .into_widget(),
            );
            tester.pump_widget(root);

            assert_eq!(shows(&tester, "One as an Arabic numeral: 1"), 1);
            tap_text(&mut tester, "One as an Arabic numeral: 1");
            tester.pump_and_settle();
            tap_text(&mut tester, "Two");
            tester.pump_and_settle();
            assert_eq!(shows(&tester, "Two as an Arabic numeral: 2"), 1);
        });
    }

    #[test]
    fn dropdown_button_form_field_on_tap_callback_is_called_when_defined() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dropdown_button_tap_counter = Rc::new(Cell::new(0));
            let (value, set_value) = value_log();
            *value.borrow_mut() = Some("one".to_owned());
            let counter = Rc::clone(&dropdown_button_tap_counter);
            let mut frame = form_frame();
            frame.value = Some("one".to_owned());
            frame.on_changed = Some(set_value);
            frame.on_tap = Some(Listener::new(move |_app| counter.set(counter.get() + 1)));
            pump_form_frame(&mut tester, &frame);

            assert_eq!(dropdown_button_tap_counter.get(), 0);

            // Tap dropdown button.
            tap_text(&mut tester, "one");
            tester.pump_and_settle();

            assert_eq!(value.borrow().as_deref(), Some("one"));
            assert_eq!(dropdown_button_tap_counter.get(), 1); // Should update counter.

            // Tap dropdown menu item.
            tap_last_text(&mut tester, "three");
            tester.pump_and_settle();

            assert_eq!(value.borrow().as_deref(), Some("three"));
            assert_eq!(dropdown_button_tap_counter.get(), 1); // Should not change.

            // Tap dropdown button again.
            tap_text(&mut tester, "three");
            tester.pump_and_settle();

            assert_eq!(value.borrow().as_deref(), Some("three"));
            assert_eq!(dropdown_button_tap_counter.get(), 2); // Should update counter.

            // Tap dropdown menu item.
            tap_last_text(&mut tester, "two");
            tester.pump_and_settle();

            assert_eq!(value.borrow().as_deref(), Some("two"));
            assert_eq!(dropdown_button_tap_counter.get(), 2); // Should not change.
        });
    }

    #[test]
    fn dropdown_button_form_field_should_re_render_if_initial_value_parameter_changes() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let current_value = Rc::new(RefCell::new("two".to_owned()));
            let state_value = Rc::clone(&current_value);
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let items = MENU_ITEMS
                        .iter()
                        .map(|value| {
                            let set_state = set_state.clone();
                            let set_value = Rc::clone(&state_value);
                            DropdownMenuItem::new(Text::new(*value))
                                .value((*value).to_owned())
                                .on_tap(Listener::new(move |app| {
                                    let set_value = Rc::clone(&set_value);
                                    set_state(app, &mut |_app| {
                                        *set_value.borrow_mut() = (*value).to_owned();
                                    });
                                }))
                        })
                        .collect();
                    Material::new()
                        .child(
                            DropdownButtonFormField::new(Some(items))
                                .initial_value(Some(state_value.borrow().clone()))
                                .on_changed(|_app, _value| {}),
                        )
                        .into_widget()
                })
                .into_widget(),
            );
            tester.pump_widget(root);

            // Make sure the rendered text value matches the initial state value.
            assert_eq!(current_value.borrow().as_str(), "two");
            assert_eq!(shows(&tester, "two"), 1);

            // Tap the DropdownButtonFormField widget
            tap_text(&mut tester, "two");
            tester.pump_and_settle();

            // Tap the first dropdown menu item.
            tap_last_text(&mut tester, "one");
            tester.pump_and_settle();

            // Make sure the rendered text value matches the updated state value.
            assert_eq!(current_value.borrow().as_str(), "one");
            assert_eq!(shows(&tester, "one"), 1);
        });
    }

    #[test]
    fn autovalidate_mode_is_passed_to_super() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let validate_called = Rc::new(Cell::new(0));
            let validated = Rc::clone(&validate_called);
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Material::new()
                    .child(
                        Center::new().child(
                            DropdownButtonFormField::new(Some(items_of(&MENU_ITEMS)))
                                .autovalidate_mode(AutovalidateMode::Always)
                                .on_changed(|_app, _value| {})
                                .validator(move |_app, _value| {
                                    validated.set(validated.get() + 1);
                                    None
                                }),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            assert_eq!(validate_called.get(), 1);
        });
    }

    #[test]
    fn dropdown_button_form_field_custom_button_alignment() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let mut frame = form_frame();
            frame.button_alignment = AlignmentGeometry::CENTER;
            frame.items = Some(strings(&["one"]));
            frame.value = Some("one".to_owned());
            pump_form_frame(&mut tester, &frame);

            let button_box = tester.render_box_of(only(tester.find_by_type::<IndexedStack>()));
            let selected_item_box = tester.render_box_of(only(tester.find_onstage_text("one")));

            // Should be center-center aligned.
            assert_eq!(
                rect_of(&tester, button_box).center(),
                rect_of(&tester, selected_item_box).center()
            );
        });
    }

    /// A form whose field starts at 'One' and reports to `on_changed` when given one.
    fn reset_form(
        tester: &WidgetTester,
        form_key: &GlobalKey,
        on_changed: Option<ValueChanged<Option<String>>>,
    ) -> WidgetRef {
        let mut field =
            DropdownButtonFormField::new(Some(items_of(&["One", "Two", "Free", "Four"])))
                .initial_value(Some("One".to_owned()));
        if let Some(on_changed) = on_changed {
            field = field.on_changed(move |app, value| on_changed(app, value));
        }
        material_app(
            ThemeData::new(&tester.cell.borrow()),
            Material::new()
                .child(Form::new(field).key(form_key.clone().into()))
                .into_widget(),
        )
    }

    #[test]
    fn dropdown_button_form_field_on_changed_is_called_when_the_form_is_reset() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/123009.
            let mut tester = tester();
            let form_key = GlobalKey::new();
            let (value, set_value) = value_log();
            let root = reset_form(&tester, &form_key, Some(set_value));
            tester.pump_widget(root);
            let state = form_field_state(&tester);

            // Initial value is 'One'.
            assert_eq!(*value.borrow(), None);
            assert_eq!(
                state.value(&tester.cell.borrow()).map(String::as_str),
                Some("One")
            );

            // Select 'Two'.
            tap_text(&mut tester, "One");
            tester.pump_and_settle();
            tap_last_text(&mut tester, "Two");
            tester.pump_and_settle();
            assert_eq!(value.borrow().as_deref(), Some("Two"));
            assert_eq!(
                state.value(&tester.cell.borrow()).map(String::as_str),
                Some("Two")
            );

            // Should be back to 'One' when the form is reset.
            let form = form_key
                .current_state::<FormState>(&tester.cell.borrow())
                .expect("the form is mounted");
            form.reset(&mut tester.cell.borrow_mut());
            assert_eq!(value.borrow().as_deref(), Some("One"));
            assert_eq!(
                state.value(&tester.cell.borrow()).map(String::as_str),
                Some("One")
            );
        });
    }

    #[test]
    fn dropdown_button_form_field_with_on_changed_set_to_null_does_not_throw_on_form_reset() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/146335.
            let mut tester = tester();
            let form_key = GlobalKey::new();
            let root = reset_form(&tester, &form_key, None);
            tester.pump_widget(root);

            // Reset the form: it does not panic.
            let form = form_key
                .current_state::<FormState>(&tester.cell.borrow())
                .expect("the form is mounted");
            form.reset(&mut tester.cell.borrow_mut());
            tester.cell.checkpoint();
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/106659.
    #[test]
    fn error_visual_logic_is_delegated_to_input_decorator() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let form_key = GlobalKey::new();
            let validate_key = form_key.clone();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Material::new()
                    .child(
                        Form::new(
                            DropdownButtonFormField::new(Some(items_of(&MENU_ITEMS)))
                                .on_changed(|_app, _value| {})
                                .validator(|_app, _value| Some("Required".to_owned()))
                                .on_tap(Listener::new(move |app| {
                                    let form = validate_key
                                        .current_state::<FormState>(app)
                                        .expect("the form is mounted");
                                    form.validate(app);
                                })),
                        )
                        .key(form_key.clone().into()),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            let decorator = tester.render_box_of(only(tester.find_by_type::<InputDecorator>()));
            tap_box(&mut tester, decorator);
            tester.pump(Duration::ZERO);

            // Check InputDecorator state because DropdownButtonFormField delegates
            // visual logic to the InputDecorator.
            let (is_focused, error_text) = tester.widget(
                only(tester.find_by_type::<InputDecorator>()),
                |decorator: &InputDecorator| {
                    (
                        decorator.is_focused,
                        decorator.decoration.error_text.clone(),
                    )
                },
            );
            assert!(is_focused);
            assert_eq!(error_text.as_deref(), Some("Required"));
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/135292.
    #[test]
    fn widget_returned_by_error_builder_is_shown() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                Material::new()
                    .child(
                        DropdownButtonFormField::new(Some(items_of(&MENU_ITEMS)))
                            .on_changed(|_app, _value| {})
                            .autovalidate_mode(AutovalidateMode::Always)
                            .validator(|_app, _value| Some("Required".to_owned()))
                            .error_builder(|_app, _context, error_text| {
                                Text::new(format!("**{error_text}**")).into_widget()
                            }),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            tester.pump(Duration::ZERO);

            assert_eq!(shows(&tester, "**Required**"), 1);
        });
    }

    fn form_field_state_of(
        tester: &WidgetTester,
        key: &GlobalKey,
    ) -> Handle<DropdownButtonFormFieldState<String>> {
        key.current_state::<DropdownButtonFormFieldState<String>>(&tester.cell.borrow())
            .expect("the field is mounted")
    }

    #[test]
    fn is_valid_returns_false_when_force_error_text_is_set_and_changes_error_display() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key1 = GlobalKey::new();
            let field_key2 = GlobalKey::new();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                MediaQuery::new(
                    MediaQueryData::new(),
                    Directionality::new(
                        TextDirection::Ltr,
                        Center::new().child(Material::new().child(Form::new(
                            ListView::new().children(vec![
                            DropdownButtonFormField::new(Some(items_of(&MENU_ITEMS)))
                                .key(field_key1.clone().into())
                                .autovalidate_mode(AutovalidateMode::Disabled)
                                .into_widget(),
                            DropdownButtonFormField::new(Some(items_of(&MENU_ITEMS)))
                                .key(field_key2.clone().into())
                                .force_error_text("Forcing error.")
                                .on_changed(|_app, _value| {})
                                .autovalidate_mode(AutovalidateMode::Disabled)
                                .into_widget(),
                        ]),
                        ))),
                    ),
                )
                .into_widget(),
            );
            tester.pump_widget(root);

            let field1 = form_field_state_of(&tester, &field_key1);
            let field2 = form_field_state_of(&tester, &field_key2);
            assert!(field1.is_valid(&mut tester.cell.borrow_mut()));
            assert!(!field1.has_error(&tester.cell.borrow()));
            assert!(!field2.is_valid(&mut tester.cell.borrow_mut()));
            assert!(field2.has_error(&tester.cell.borrow()));
        });
    }

    #[test]
    fn force_error_text_overrides_input_decoration_error_when_both_are_provided() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                MediaQuery::new(
                    MediaQueryData::new(),
                    Directionality::new(
                        TextDirection::Ltr,
                        Center::new().child(
                            Material::new().child(Form::new(
                                DropdownButtonFormField::new(Some(items_of(&MENU_ITEMS)))
                                    .decoration(InputDecoration::new().error_text("Decoration"))
                                    .force_error_text("Forcing error"),
                            )),
                        ),
                    ),
                )
                .into_widget(),
            );
            tester.pump_widget(root);

            assert_eq!(shows(&tester, "Forcing error"), 1);
            assert_eq!(shows(&tester, "Decoration"), 0);
        });
    }

    fn form_barrier_case(barrier_dismissible: Option<bool>, expected_second: usize) {
        with_main_thread_stack(move || {
            let mut tester = tester();
            let mut field = DropdownButtonFormField::new(Some(vec![
                DropdownMenuItem::new(Text::new("disabled")).enabled(false),
                DropdownMenuItem::new(Text::new("first")).value("first".to_owned()),
                DropdownMenuItem::new(Text::new("second")).value("second".to_owned()),
            ]))
            .initial_value(Some("first".to_owned()))
            .on_changed(|_app, _value| {});
            if let Some(barrier_dismissible) = barrier_dismissible {
                field = field.barrier_dismissible(barrier_dismissible);
            }
            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new().body(field).into_widget(),
            );
            tester.pump_widget(root);

            // Open dropdown.
            tap_hit_testable_text(&mut tester, "first");
            tester.pump_and_settle();

            // Tap on the barrier.
            tester.tap(Offset::new(400.0, 400.0), 1);
            tester.pump_and_settle();

            assert_eq!(shows(&tester, "second"), expected_second);
        });
    }

    #[test]
    fn form_field_dropdown_closes_when_barrier_is_tapped_by_default() {
        // The dropdown should be closed, i.e., there should be no widget with 'second' text.
        form_barrier_case(None, 0);
    }

    #[test]
    fn form_field_dropdown_does_not_close_when_barrier_dismissible_set_to_false() {
        // The dropdown should still be open, i.e., there should be one widget with 'second' text.
        form_barrier_case(Some(false), 1);
    }

    fn one_two_field_app(
        tester: &WidgetTester,
        field: DropdownButtonFormField<String>,
    ) -> WidgetRef {
        material_app(
            ThemeData::new(&tester.cell.borrow()),
            crate::scaffold::Scaffold::new()
                .body(Center::new().child(field))
                .into_widget(),
        )
    }

    // Regression test for https://github.com/flutter/flutter/issues/157074.
    #[test]
    fn dropdown_button_form_field_icon_is_aligned_with_label_text() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = one_two_field_app(
                &tester,
                DropdownButtonFormField::new(Some(items_of(&["One", "Two"])))
                    .decoration(InputDecoration::new().label_text("Label Text"))
                    .on_changed(|_app, _value| {}),
            );
            tester.pump_widget(root);

            let icon = *tester.find_by_type::<Icon>().last().expect("an icon");
            assert_eq!(
                text_center_dy(&tester, "Label Text"),
                center_dy_of(&tester, icon)
            );
        });
    }

    #[test]
    fn dropdown_form_field_has_expected_default_mouse_cursor_on_hover() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = one_two_field_app(
                &tester,
                DropdownButtonFormField::new(Some(items_of(&["One", "Two"])))
                    .on_changed(|_app, _value| {}),
            );
            tester.pump_widget(root);

            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);

            assert!(*tester.active_cursor(1) == *cursor(waft_services::SystemMouseCursors::BASIC));

            let field = only(tester.find_by_type::<DropdownButtonFormField<String>>());
            let center = rect_of(&tester, tester.render_box_of(field)).center();
            tester.move_to(&mut gesture, center, false);
            tester.pump(Duration::ZERO);

            let expected = if K_IS_WEB {
                cursor(waft_services::SystemMouseCursors::CLICK)
            } else {
                cursor(waft_services::SystemMouseCursors::BASIC)
            };
            assert!(*tester.active_cursor(1) == *expected);
        });
    }

    #[test]
    fn dropdown_button_form_field_has_expected_mouse_cursor_when_explicitly_configured() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = one_two_field_app(
                &tester,
                DropdownButtonFormField::new(Some(items_of(&["One", "Two"])))
                    .mouse_cursor(cursor(waft_services::SystemMouseCursors::CELL))
                    .on_changed(|_app, _value| {}),
            );
            tester.pump_widget(root);

            let field = only(tester.find_by_type::<DropdownButtonFormField<String>>());
            let center = rect_of(&tester, tester.render_box_of(field)).center();
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, center);

            assert!(*tester.active_cursor(1) == *cursor(waft_services::SystemMouseCursors::CELL));
        });
    }

    fn int_field_app(tester: &WidgetTester, field: DropdownButtonFormField<i32>) -> WidgetRef {
        material_app(
            ThemeData::new(&tester.cell.borrow()),
            Material::new().child(field).into_widget(),
        )
    }

    fn one_two_int_items() -> Vec<DropdownMenuItem<i32>> {
        vec![
            DropdownMenuItem::new(Text::new("one")).value(1),
            DropdownMenuItem::new(Text::new("two")).value(2),
        ]
    }

    #[test]
    fn dropdown_button_form_field_enabled_true_without_on_changed_updates_form_field_state_value() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_key = GlobalKey::new();
            let root = int_field_app(
                &tester,
                DropdownButtonFormField::new(Some(one_two_int_items()))
                    .key(field_key.clone().into())
                    .initial_value(Some(1))
                    .enabled(true),
            );
            tester.pump_widget(root);

            // Open the menu,
            tap_text(&mut tester, "one");
            tester.pump_and_settle();

            // Select the second item,
            tap_last_text(&mut tester, "two");
            tester.pump_and_settle();

            // Menu should be closed and value updated.
            let state = field_key
                .current_state::<DropdownButtonFormFieldState<i32>>(&tester.cell.borrow())
                .expect("the field is mounted");
            assert_eq!(state.value(&tester.cell.borrow()).copied(), Some(2));
            assert_eq!(shows(&tester, "two"), 1);
        });
    }

    #[test]
    fn dropdown_button_form_field_enabled_false_with_on_changed_does_not_open_menu() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = int_field_app(
                &tester,
                DropdownButtonFormField::new(Some(one_two_int_items()))
                    .initial_value(Some(1))
                    .enabled(false)
                    .on_changed(|_app, _value| {}),
            );
            tester.pump_widget(root);

            // Attempt to open the menu.
            tap_text(&mut tester, "one");
            tester.pump_and_settle();

            // Menu should not open, so 'two' should not be visible.
            assert_eq!(shows(&tester, "two"), 0);
        });
    }

    /// Dart's `paints..line(p1:, p2:)`: whether `render`'s pictures stroke a two-point path
    /// from `p1` to `p2`, read from the path's bounds.
    fn paints_line(tester: &WidgetTester, render: AnyRenderBox, p1: Offset, p2: Offset) -> bool {
        use waft_embedder::valo::{Op, PaintStyle as ValoPaintStyle};
        let lines: Vec<(Offset, Offset)> = tester
            .painted_ops(render.as_object())
            .into_iter()
            .filter_map(|op| match op {
                Op::DrawPath { path, paint, .. }
                    if matches!(paint.style, ValoPaintStyle::Stroke(_))
                        && path.elements().count() == 2 =>
                {
                    let bounds = path.bounds();
                    Some((
                        Offset::new(f64::from(bounds.x), f64::from(bounds.y)),
                        Offset::new(
                            f64::from(bounds.x + bounds.width),
                            f64::from(bounds.y + bounds.height),
                        ),
                    ))
                }
                _ => None,
            })
            .collect();
        let found = lines.contains(&(p1, p2));
        if !found {
            eprintln!("lines painted: {lines:?}");
        }
        found
    }

    #[test]
    fn dropdown_button_form_field_underline_is_at_the_bottom_of_the_expanded_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const CONTAINER_HEIGHT: f64 = 200.0;

            let root = material_app(
                ThemeData::new(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .body(
                        SizedBox::new().height(CONTAINER_HEIGHT).child(
                            DropdownButtonFormField::new(Some(vec![
                                DropdownMenuItem::new(Text::new("Option 1")).value(1),
                            ]))
                            .is_expanded(true)
                            .is_vertically_expanded(true)
                            .on_changed(|_app, _value| {}),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump_widget(root);

            let decorator = tester.render_box_of(only(tester.find_by_type::<InputDecorator>()));
            let width = size_of(&tester, decorator).width();

            assert!(paints_line(
                &tester,
                decorator,
                Offset::new(0.0, CONTAINER_HEIGHT - 0.5),
                Offset::new(width, CONTAINER_HEIGHT - 0.5),
            ));
        });
    }

    /// Not a Dart case: what Flutter's leak tracking checks, that opening and closing the menu
    /// leaves nothing in the arena.
    #[test]
    fn opening_and_closing_the_menu_leaves_nothing_behind() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_frame(&mut tester, &frame().on_changed(on_changed()));
            tester.assert_leaves_nothing_behind(|tester| {
                tap_text(tester, "two");
                tester.pump_and_settle();
                assert!(!tester.find_by_type::<DropdownMenu<String>>().is_empty());
                tester.tap(Offset::ZERO, 1);
                tester.pump_and_settle();
                assert!(tester.find_by_type::<DropdownMenu<String>>().is_empty());
            });
        });
    }

    #[test]
    fn dropdown_button_hint_is_selected_item() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const HINT_PADDING_OFFSET: f64 = 8.0;
            let item_values = ["item0", "item1", "item2", "item3"];
            let selected_item = Rc::new(RefCell::new(Some("item0".to_owned())));

            let build_frame = |tester: &WidgetTester| {
                let selected_item = Rc::clone(&selected_item);
                material_app(
                    ThemeData::new(&tester.cell.borrow()),
                    crate::scaffold::Scaffold::new()
                        .body(
                            ButtonTheme::new(DropdownButtonHideUnderline::new(
                                Center::new().child(StatefulBuilder::new(
                                    move |_app, _context, set_state| {
                                        let set_state = set_state.clone();
                                        let hint_item = Rc::clone(&selected_item);
                                        let set_item = Rc::clone(&selected_item);
                                        // The pretzel below is from an actual app. The price
                                        // of limited configurability is keeping this working.
                                        DropdownButton::new(
                                            Some(
                                                item_values
                                                    .iter()
                                                    .map(|value| {
                                                        DropdownMenuItem::new(Text::new(*value))
                                                            .value((*value).to_owned())
                                                    })
                                                    .collect(),
                                            ),
                                            Some(Rc::new(move |app, value| {
                                                let set_item = Rc::clone(&set_item);
                                                set_state(app, &mut |_app| {
                                                    *set_item.borrow_mut() = value.clone();
                                                });
                                            })),
                                        )
                                        .is_expanded(true)
                                        .elevation(2)
                                        .hint(
                                            LayoutBuilder::new(
                                                move |_app, _context, constraints| {
                                                    let label = format!(
                                                        "-{}-",
                                                        hint_item
                                                            .borrow()
                                                            .as_deref()
                                                            .unwrap_or("null")
                                                    );
                                                    // Stack with a positioned widget is used to override the
                                                    // hard coded 16px margin in the dropdown code, so that
                                                    // this hint aligns "properly" with the menu.
                                                    Stack::new()
                                                        .clip_behavior(Clip::None)
                                                        .alignment(AlignmentGeometry::TOP_CENTER)
                                                        .children(vec![
                                                waft_widgets::PositionedDirectional::new(
                                                    Text::new(label),
                                                )
                                                .width(
                                                    constraints.constraints().max_width
                                                        + HINT_PADDING_OFFSET,
                                                )
                                                .start(-HINT_PADDING_OFFSET)
                                                .top(4.0)
                                                .into_widget(),
                                            ])
                                                        .into_widget()
                                                },
                                            )
                                            .into_widget(),
                                        )
                                        .icon(Container::new())
                                        .into_widget()
                                    },
                                )),
                            ))
                            .aligned_dropdown(true),
                        )
                        .into_widget(),
                )
            };

            let root = build_frame(&tester);
            tester.pump_widget(root);
            let hint = tester.render_box_of(only(tester.find_onstage_text("-item0-")));
            assert_eq!(global_top_left(&tester, hint).dx(), 8.0);

            // Show the popup menu.
            let hint = tester.render_box_of(tester.find_text("-item0-")[0]);
            tap_box(&mut tester, hint);
            tester.pump_and_settle();

            let hint = tester.render_box_of(only(tester.find_onstage_text("-item0-")));
            assert_eq!(global_top_left(&tester, hint).dx(), 8.0);
        });
    }

    // button_theme_test.dart 'ButtonTheme alignedDropdown'.
    #[test]
    fn button_theme_aligned_dropdown() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dropdown_key = GlobalKey::new();

            let build_frame =
                |tester: &WidgetTester, aligned_dropdown: bool, text_direction: TextDirection| {
                    let dropdown_key = dropdown_key.clone();
                    crate::app::MaterialApp::new()
                        .theme(ThemeData::new(&tester.cell.borrow()))
                        .builder(move |_app, _context, child| {
                            Directionality::new(
                                text_direction,
                                child.cloned().expect("the app's navigator"),
                            )
                            .into_widget()
                        })
                        .home(
                            ButtonTheme::new(Material::new().child(Builder::new(
                                move |_app, _context| {
                                    Container::new()
                                        .alignment(AlignmentGeometry::CENTER)
                                        .child(DropdownButtonHideUnderline::new(
                                            SizedBox::new().width(200.0).child(
                                                DropdownButton::new(
                                                    Some(vec![
                                                        DropdownMenuItem::new(Text::new("foo"))
                                                            .value("foo".to_owned()),
                                                        DropdownMenuItem::new(Text::new("bar"))
                                                            .value("bar".to_owned()),
                                                    ]),
                                                    Some(Rc::new(|_app, _value| {})),
                                                )
                                                .key(dropdown_key.clone().into())
                                                .value(Some("foo".to_owned())),
                                            ),
                                        ))
                                        .into_widget()
                                },
                            )))
                            .aligned_dropdown(aligned_dropdown)
                            .into_widget(),
                        )
                        .into_widget()
                };
            let menus = |tester: &WidgetTester| tester.find_by_type::<DropdownMenu<String>>();
            let width_of = |tester: &WidgetTester, element: AnyElement| {
                size_of(tester, tester.render_box_of(element)).width()
            };
            let foo_rects = |tester: &WidgetTester| -> Vec<Rect> {
                tester
                    .find_onstage_text("foo")
                    .into_iter()
                    .map(|element| rect_of(tester, tester.render_box_of(element)))
                    .collect()
            };

            let root = build_frame(&tester, false, TextDirection::Ltr);
            tester.pump_widget(root);
            tap_key(&mut tester, &dropdown_key);
            tester.pump_and_settle();

            // 240 = 200.0 (button width) + _kUnalignedMenuMargin (20.0 left and right)
            assert_eq!(
                size_of(&tester, key_box(&tester, &dropdown_key)).width(),
                200.0
            );
            assert_eq!(width_of(&tester, only(menus(&tester))), 240.0);

            // Dismiss the menu.
            tester.tap(Offset::ZERO, 1);
            tester.pump_and_settle();
            assert!(menus(&tester).is_empty());

            let root = build_frame(&tester, true, TextDirection::Ltr);
            tester.pump_widget(root);
            tap_key(&mut tester, &dropdown_key);
            tester.pump_and_settle();

            // Aligneddropdown: true means the button and menu widths match
            assert_eq!(
                size_of(&tester, key_box(&tester, &dropdown_key)).width(),
                200.0
            );
            assert_eq!(width_of(&tester, only(menus(&tester))), 200.0);

            // There are two 'foo' widgets: the selected menu item's label and the drop
            // down button's label. The should both appear at the same location.
            let rects = foo_rects(&tester);
            assert_eq!(rects.len(), 2);
            assert_eq!(rects[0], rects[1]);

            // Dismiss the menu.
            tester.tap(Offset::ZERO, 1);
            tester.pump_and_settle();
            assert!(menus(&tester).is_empty());

            // Same test as above except RTL
            let root = build_frame(&tester, true, TextDirection::Rtl);
            tester.pump_widget(root);
            tap_key(&mut tester, &dropdown_key);
            tester.pump_and_settle();

            let rects = foo_rects(&tester);
            assert_eq!(rects.len(), 2);
            assert_eq!(rects[0], rects[1]);
        });
    }

    #[test]
    fn button_theme_aligned_dropdown_does_not_affect_the_field_content_position() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_frame =
                |tester: &WidgetTester, aligned_dropdown: bool, text_direction: TextDirection| {
                    material_app(
                        ThemeData::new(&tester.cell.borrow()),
                        Directionality::new(
                            text_direction,
                            ButtonTheme::new(
                                Material::new().child(
                                    DropdownButtonFormField::new(Some(
                                        MENU_ITEMS
                                            .iter()
                                            .map(|value| {
                                                DropdownMenuItem::new(Text::new(*value))
                                                    .value((*value).to_owned())
                                            })
                                            .collect(),
                                    ))
                                    .initial_value(Some(MENU_ITEMS[0].to_owned()))
                                    .on_changed(|_app, _value| {}),
                                ),
                            )
                            .aligned_dropdown(aligned_dropdown),
                        )
                        .into_widget(),
                    )
                };
            let selected_value_rect = |tester: &WidgetTester| {
                let element = tester.find_onstage_text(MENU_ITEMS[0])[0];
                rect_of(tester, tester.render_box_of(element))
            };

            for text_direction in [TextDirection::Ltr, TextDirection::Rtl] {
                let root = build_frame(&tester, false, text_direction);
                tester.pump_widget(root);
                let content_rect_for_unaligned_dropdown = selected_value_rect(&tester);

                // When alignedDropdown is true, the content should be at the same position.
                let root = build_frame(&tester, true, text_direction);
                tester.pump_widget(root);
                assert_eq!(
                    selected_value_rect(&tester),
                    content_rect_for_unaligned_dropdown,
                    "{text_direction:?}"
                );
            }
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/195192.
    #[test]
    fn dropdown_button_form_field_decoration_is_disabled_when_the_field_is_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build = |tester: &WidgetTester, enabled: Option<bool>, with_on_changed: bool| {
                let mut field =
                    DropdownButtonFormField::new(Some(one_two_int_items())).initial_value(Some(1));
                if let Some(enabled) = enabled {
                    field = field.enabled(enabled);
                }
                if with_on_changed {
                    field = field.on_changed(|_app, _value| {});
                }
                int_field_app(tester, field)
            };
            let is_decoration_enabled = |tester: &WidgetTester| {
                tester.widget(
                    only(tester.find_by_type::<InputDecorator>()),
                    |decorator: &InputDecorator| decorator.decoration.enabled,
                )
            };

            let root = build(&tester, None, true);
            tester.pump_widget(root);
            assert!(is_decoration_enabled(&tester));

            let root = build(&tester, Some(false), true);
            tester.pump_widget(root);
            assert!(!is_decoration_enabled(&tester));

            // Without `enabled`, a null onChanged disables the field.
            let root = build(&tester, None, false);
            tester.pump_widget(root);
            assert!(!is_decoration_enabled(&tester));
        });
    }

    #[test]
    fn dropdown_button_form_field_with_on_changed_null_still_autovalidates() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = int_field_app(
                &tester,
                DropdownButtonFormField::new(Some(one_two_int_items()))
                    .initial_value(Some(1))
                    .autovalidate_mode(AutovalidateMode::Always)
                    .validator(|_app, _value| Some("error text".to_owned())),
            );
            tester.pump_widget(root);

            assert_eq!(shows(&tester, "error text"), 1);
        });
    }

    #[test]
    fn dropdown_menu_can_position_correctly_inside_a_nested_navigator() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/66870
            let mut tester = tester();
            let root = material_app(
                m2(&tester.cell.borrow()),
                crate::scaffold::Scaffold::new()
                    .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                        crate::app_bar::AppBar::new(),
                    ))
                    .body(waft_widgets::Column::new().children(vec![
                            ConstrainedBox::new(
                                BoxConstraints::new().max_width(500.0).max_height(200.0),
                            )
                            .child(Navigator::new().on_generate_route(|app, _settings| {
                                let route = MaterialPageRoute::new(
                                    app,
                                    Rc::new(|_app, _context| {
                                        Center::new()
                                            .child(
                                                DropdownButton::new(
                                                    Some(vec![
                                                        DropdownMenuItem::new(Text::new(
                                                            "First Item",
                                                        ))
                                                        .value(1),
                                                        DropdownMenuItem::new(Text::new(
                                                            "Second Item",
                                                        ))
                                                        .value(2),
                                                    ]),
                                                    Some(Rc::new(|_app, _value| {})),
                                                )
                                                .value(Some(1)),
                                            )
                                            .into_widget()
                                    }),
                                );
                                Some(route.erase())
                            }))
                            .into_widget(),
                        ]))
                    .into_widget(),
            );
            tester.pump_widget(root);
            tap_text(&mut tester, "First Item");
            tester.pump(Duration::ZERO);
            let second_item = tester.render_box_of(tester.find_text("Second Item")[1]);
            assert_eq!(global_top_left(&tester, second_item).dx(), 150.0);
            assert_eq!(global_top_left(&tester, second_item).dy(), 176.0);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/151460.
    #[test]
    fn dropdown_button_form_field_has_hover_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
            let button_key = GlobalKey::new();
            let build = |decoration: InputDecoration| {
                let mut frame = frame()
                    .form_field(true)
                    .button_key(&button_key)
                    .on_changed(on_changed());
                // Setting InputDecoration.filled to true is required to get overlay showing.
                frame.decoration = Some(decoration);
                frame
            };

            pump_frame(&mut tester, &build(InputDecoration::new().filled(true)));
            tester.pump(Duration::ZERO);

            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            let center = tester.center_of(&button_key);
            tester.move_to(&mut gesture, center, false);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(15)); // Hover animation.

            // Default hover color.
            let theme = decorator_theme(&tester);
            assert!(input_decorator_fills(
                &tester,
                Color::alpha_blend(
                    theme.hover_color,
                    theme.color_scheme.surface_container_highest()
                )
            ));

            // Custom hover color.
            let hover_color = Color::new(0xaa00ff00);
            pump_frame(
                &mut tester,
                &build(InputDecoration::new().filled(true).hover_color(hover_color)),
            );
            assert!(input_decorator_fills(
                &tester,
                Color::alpha_blend(hover_color, theme.color_scheme.surface_container_highest())
            ));
        });
    }

    fn conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown(
        platform: TargetPlatform,
    ) {
        with_main_thread_stack(move || {
            // Regression test for https://github.com/flutter/flutter/issues/83819
            let mut tester = tester_on(platform);
            // Open the dropdown menu
            let button_key = GlobalKey::new();
            pump_frame(
                &mut tester,
                &frame()
                    .button_key(&button_key)
                    // nothing selected
                    .initial_value(None)
                    .items(Some(numbers(100)))
                    .on_changed(on_changed()),
            );
            tap_key(&mut tester, &button_key);
            tester.pump(Duration::ZERO);
            tester.pump_and_settle(); // finish the menu animation

            // The inherited ScrollBehavior should not apply Scrollbars since they are
            // already built in to the widget. For iOS platform, ScrollBar directly returns
            // CupertinoScrollbar
            let cupertino = tester
                .find_onstage_by_type::<waft_cupertino::CupertinoScrollbar>()
                .len();
            assert_eq!(cupertino, usize::from(platform == TargetPlatform::IOS));
            assert_eq!(tester.find_onstage_by_type::<Scrollbar>().len(), 1);
            assert!(
                tester
                    .find_onstage_by_type::<waft_widgets::RawScrollbar>()
                    .is_empty()
            );
        });
    }

    #[test]
    fn conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown_on_android() {
        conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown(
            TargetPlatform::Android,
        );
    }

    #[test]
    fn conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown_on_ios() {
        conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown(TargetPlatform::IOS);
    }

    #[test]
    fn conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown_on_fuchsia() {
        conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown(
            TargetPlatform::Fuchsia,
        );
    }

    #[test]
    fn conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown_on_linux() {
        conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown(
            TargetPlatform::Linux,
        );
    }

    #[test]
    fn conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown_on_macos() {
        conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown(
            TargetPlatform::MacOS,
        );
    }

    #[test]
    fn conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown_on_windows() {
        conflicting_scrollbars_are_not_applied_by_scroll_behavior_to_dropdown(
            TargetPlatform::Windows,
        );
    }
}
