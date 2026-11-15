//! Flutter counterpart: material_ui's `popup_menu.dart`.
//!
//! A [`PopupMenuEntry`] is a widget that also answers its height and whether it represents a
//! value; the menu holds them erased as [`PopupMenuEntryRef`]s. A route's result is erased
//! (waft-widgets' navigator), so [`show_menu`] resolves to the `Rc<dyn Any>` an item popped
//! with, which holds its `T`.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, Animation, AnimationStyle, AnyAnimation, CurveTween, CurvedAnimation, Curves,
    Interval,
};
use waft_embedder::{
    Brightness, Clip, Color, Offset, Rect, SemanticsRole, Size, TargetPlatform, TextBaseline,
    TextDirection,
};
use waft_foundation::{App, CompleterFuture, Handle, Listener, ValueChanged};
use waft_painting::{
    AlignmentGeometry, BorderRadius, BorderRadiusGeometry, EdgeInsets, EdgeInsetsGeometry,
    ShapeBorderRef, TextStyle,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, BoxParentData,
    PaintingContext, RelativeRect, RenderBox, RenderBoxData, RenderHandle, RenderObject,
    RenderObjectData, RenderObjectWithChildData, RenderObjectWithChildMixin, RenderShiftedBox,
    SingleChildLayoutDelegate,
};
use waft_scheduler::{FrameCallback, SchedulerBinding};
use waft_services::MouseCursorRef;
use waft_widgets::{
    Align, AnimatedBuilder, AnimatedDefaultTextStyle, BuildContext, CapturedThemes, ColoredBox,
    ConstrainedBox, CustomSingleChildLayout, Directionality, FadeTransition, GlobalKey, Icon,
    IconTheme, IconThemeData, IgnorePointer, InheritedTheme, IntoWidget, IntrinsicWidth, KeyRef,
    LayoutBuilder, ListBody, LocalHistoryRoute, LocalHistoryRouteData, MediaQuery, MergeSemantics,
    ModalRoute, ModalRouteData, NavigationMode, Navigator, OverlayRoute, OverlayRouteData, Padding,
    PopupRoute, PredictiveBackRoute, RenderObjectWidget, Route, RouteData, RouteResult,
    RouteSettingsRef, ScrollPositionAlignmentPolicy, Scrollable, Semantics,
    SingleChildRenderObjectWidget, SingleChildScrollView, SingleTickerProviderStateMixin,
    SingleTickerProviderStateMixinData, StandardComponentType, State, StateData, StatefulWidget,
    TransitionRoute, TransitionRouteData, TraversalEdgeBehavior, WidgetRef, WidgetState,
    WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
    downcast_widget,
};

use crate::button_style::ButtonStyle;
use crate::constants::{K_MIN_INTERACTIVE_DIMENSION, K_THEME_CHANGE_DURATION};
use crate::debug::debug_check_has_material_localizations;
use crate::divider::Divider;
use crate::generated::popup_menu_defaults_m3::{MENU_ITEM_PADDING, PopupMenuDefaultsM3};
use crate::icon_button::IconButton;
use crate::icons::Icons;
use crate::ink_well::InkWell;
use crate::list_tile::ListTile;
use crate::list_tile_theme::ListTileTheme;
use crate::material::{Material, MaterialType};
use crate::material_localizations::MaterialLocalizations;
use crate::popup_menu_theme::{
    PopupMenuPosition, PopupMenuTheme, PopupMenuThemeData, PopupMenuThemeDataGetters,
};
use crate::theme::Theme;
use crate::theme_data::MaterialTapTargetSize;
use crate::theme_data_getters::DefaultsContext;
use crate::tooltip::Tooltip;

const K_MENU_DURATION: Duration = Duration::from_millis(300);
const K_MENU_CLOSE_INTERVAL_END: f64 = 2.0 / 3.0;
const K_MENU_DIVIDER_HEIGHT: f64 = 16.0;
const K_MENU_MAX_WIDTH: f64 = 5.0 * K_MENU_WIDTH_STEP;
const K_MENU_MIN_WIDTH: f64 = 2.0 * K_MENU_WIDTH_STEP;
const K_MENU_WIDTH_STEP: f64 = 56.0;
const K_MENU_SCREEN_PADDING: f64 = 8.0;

// ---------------------------------------------------------------------------------------------
// PopupMenuEntry

/// A base class for entries in a Material Design popup menu.
///
/// The popup menu widget uses this interface to interact with the menu items. To show a popup
/// menu, use the [`show_menu`] function. To create a button that shows a popup menu, consider
/// using [`PopupMenuButton`].
///
/// The type `T` is the type of the value(s) the entry represents. All the entries in a given
/// menu must represent values with consistent types.
///
/// A [`PopupMenuEntry`] may represent multiple values, for example a row with several icons,
/// or a single entry, for example a menu item with an icon (see [`PopupMenuItem`]), or no value
/// at all (for example, [`PopupMenuDivider`]).
///
/// A menu holds its entries as [`PopupMenuEntryRef`]s.
///
/// See also:
///
///  * [`PopupMenuItem`], a popup menu entry for a single value.
///  * [`PopupMenuDivider`], a popup menu entry that is just a horizontal line.
///  * [`CheckedPopupMenuItem`], a popup menu item with a checkmark.
///  * [`show_menu`], a method to dynamically show a popup menu at a given location.
///  * [`PopupMenuButton`], an [`IconButton`] that automatically shows a menu when it is tapped.
pub trait PopupMenuEntry<T: 'static>: StatefulWidget {
    /// The amount of vertical space occupied by this entry.
    ///
    /// This value is used at the time the [`show_menu`] method is called, if the
    /// `initial_value` argument is provided, to determine the position of this entry when
    /// aligning the selected entry over the given `position`. It is otherwise ignored.
    fn height(&self) -> f64;

    /// Whether this entry represents a particular value.
    ///
    /// This method is used by [`show_menu`], when it is called, to align the entry representing
    /// the `initial_value`, if any, to the given `position`, and then later is called on each
    /// entry to determine if it should be highlighted (if the method returns true, the entry
    /// will have its background color set to the ambient `ThemeData::highlight_color`). If
    /// `initial_value` is `None`, then this method is not called.
    ///
    /// If the [`PopupMenuEntry`] represents a single value, this should return true if the
    /// argument matches that value. If it represents multiple values, it should return true if
    /// the argument matches any of them.
    fn represents(&self, value: Option<&T>) -> bool;
}

/// A [`PopupMenuEntry`] held as a value: the erased widget, whose height and values are still
/// answered by the entry it holds. Dart types such a value `PopupMenuEntry<T>`.
pub struct PopupMenuEntryRef<T: 'static> {
    widget: WidgetRef,
    height: fn(&WidgetRef) -> f64,
    represents: fn(&WidgetRef, Option<&T>) -> bool,
}

impl<T: 'static> PopupMenuEntryRef<T> {
    /// Erases `entry`.
    pub fn new<K, E: PopupMenuEntry<T> + IntoWidget<K>>(entry: E) -> PopupMenuEntryRef<T> {
        PopupMenuEntryRef {
            widget: entry.into_widget(),
            height: |widget| {
                downcast_widget::<E>(&**widget)
                    .expect("an erased widget keeps its type")
                    .height()
            },
            represents: |widget, value| {
                downcast_widget::<E>(&**widget)
                    .expect("an erased widget keeps its type")
                    .represents(value)
            },
        }
    }

    /// See [`PopupMenuEntry::height`].
    pub fn height(&self) -> f64 {
        (self.height)(&self.widget)
    }

    /// See [`PopupMenuEntry::represents`].
    pub fn represents(&self, value: Option<&T>) -> bool {
        (self.represents)(&self.widget, value)
    }

    /// The widget itself.
    pub fn widget(&self) -> &WidgetRef {
        &self.widget
    }
}

impl<T: 'static> Clone for PopupMenuEntryRef<T> {
    fn clone(&self) -> PopupMenuEntryRef<T> {
        PopupMenuEntryRef {
            widget: self.widget.clone(),
            height: self.height,
            represents: self.represents,
        }
    }
}

impl<T: 'static> Debug for PopupMenuEntryRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.widget, f)
    }
}

/// The kind tag of [`IntoWidget`] for a [`PopupMenuEntryRef`].
pub struct PopupMenuEntryRefKind;

impl<T: 'static> IntoWidget<PopupMenuEntryRefKind> for PopupMenuEntryRef<T> {
    fn into_widget(self) -> WidgetRef {
        self.widget
    }
}

// ---------------------------------------------------------------------------------------------
// PopupMenuDivider

/// A horizontal divider in a Material Design popup menu.
///
/// This widget adapts the [`Divider`] for use in popup menus.
///
/// It is a [`PopupMenuEntry`] of every `T`, as Dart's `PopupMenuEntry<Never>` is.
///
/// See also:
///
///  * [`PopupMenuItem`], for the kinds of items that this widget divides.
///  * [`show_menu`], a method to dynamically show a popup menu at a given location.
///  * [`PopupMenuButton`], an [`IconButton`] that automatically shows a menu when it is tapped.
#[derive(Debug)]
pub struct PopupMenuDivider {
    pub key: Option<KeyRef>,

    /// The height of the divider entry.
    ///
    /// Defaults to 16 pixels.
    pub height: f64,

    /// The thickness of the divider line.
    ///
    /// If this is `None`, then the `DividerThemeData::thickness` is used. If that is also
    /// `None`, then this defaults to 1.0.
    pub thickness: Option<f64>,

    /// The amount of empty space to the leading edge of the divider line.
    ///
    /// If this is `None`, then the `DividerThemeData::indent` is used. If that is also `None`,
    /// then this defaults to 0.0.
    pub indent: Option<f64>,

    /// The amount of empty space to the trailing edge of the divider line.
    ///
    /// If this is `None`, then the `DividerThemeData::end_indent` is used. If that is also
    /// `None`, then this defaults to 0.0.
    pub end_indent: Option<f64>,

    /// The amount of radius for the border of the divider line.
    ///
    /// If this is `None`, then the default radius of [`Divider`] will be used.
    pub radius: Option<BorderRadiusGeometry>,

    /// The color of the divider line.
    ///
    /// If this is `None`, then the `DividerThemeData::color` is used. If that is also `None`,
    /// then `ColorScheme::outline_variant` is used.
    // TODO(framework): Add unit tests to this code snippet.
    // https://github.com/flutter/flutter/issues/188530
    pub color: Option<Color>,
}

impl PopupMenuDivider {
    /// Creates a horizontal divider for a popup menu.
    ///
    /// By default, the divider has a height of 16 logical pixels.
    pub fn new() -> PopupMenuDivider {
        PopupMenuDivider {
            key: None,
            height: K_MENU_DIVIDER_HEIGHT,
            thickness: None,
            indent: None,
            end_indent: None,
            radius: None,
            color: None,
        }
    }

    /// Dart `PopupMenuDivider(key:)`.
    pub fn key(mut self, key: KeyRef) -> PopupMenuDivider {
        self.key = Some(key);
        self
    }

    /// Dart `PopupMenuDivider(height:)`.
    pub fn height(mut self, height: f64) -> PopupMenuDivider {
        self.height = height;
        self
    }

    /// Dart `PopupMenuDivider(thickness:)`.
    pub fn thickness(mut self, thickness: f64) -> PopupMenuDivider {
        self.thickness = Some(thickness);
        self
    }

    /// Dart `PopupMenuDivider(indent:)`.
    pub fn indent(mut self, indent: f64) -> PopupMenuDivider {
        self.indent = Some(indent);
        self
    }

    /// Dart `PopupMenuDivider(endIndent:)`.
    pub fn end_indent(mut self, end_indent: f64) -> PopupMenuDivider {
        self.end_indent = Some(end_indent);
        self
    }

    /// Dart `PopupMenuDivider(radius:)`.
    pub fn radius(mut self, radius: impl Into<BorderRadiusGeometry>) -> PopupMenuDivider {
        self.radius = Some(radius.into());
        self
    }

    /// Dart `PopupMenuDivider(color:)`.
    pub fn color(mut self, color: Color) -> PopupMenuDivider {
        self.color = Some(color);
        self
    }
}

impl Default for PopupMenuDivider {
    fn default() -> PopupMenuDivider {
        PopupMenuDivider::new()
    }
}

impl<T: 'static> PopupMenuEntry<T> for PopupMenuDivider {
    fn height(&self) -> f64 {
        self.height
    }

    fn represents(&self, _value: Option<&T>) -> bool {
        false
    }
}

impl StatefulWidget for PopupMenuDivider {
    type State = PopupMenuDividerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> PopupMenuDividerState {
        PopupMenuDividerState {
            state: StateData::new(),
        }
    }
}

/// Dart's `_PopupMenuDividerState`.
pub struct PopupMenuDividerState {
    state: StateData<PopupMenuDivider>,
}

impl Debug for PopupMenuDividerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PopupMenuDividerState")
            .finish_non_exhaustive()
    }
}

impl State for PopupMenuDividerState {
    type Widget = PopupMenuDivider;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let mut divider = Divider::new().height(widget.height);
        if let Some(thickness) = widget.thickness {
            divider = divider.thickness(thickness);
        }
        if let Some(indent) = widget.indent {
            divider = divider.indent(indent);
        }
        if let Some(color) = widget.color {
            divider = divider.color(color);
        }
        if let Some(end_indent) = widget.end_indent {
            divider = divider.end_indent(end_indent);
        }
        if let Some(radius) = widget.radius {
            divider = divider.radius(radius);
        }
        divider.into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// _MenuItem

// This widget only exists to enable _PopupMenuRoute to save the sizes of
// each menu item. The sizes are used by _PopupMenuRouteLayout to compute the
// y coordinate of the menu's origin so that the center of selected menu
// item lines up with the center of its PopupMenuButton.
/// Dart's `_MenuItem`.
#[derive(Debug)]
struct MenuItem {
    on_layout: OnLayout,
    child: Option<WidgetRef>,
}

/// Dart's `ValueChanged<Size> onLayout`, which `Debug` prints by name.
#[derive(Clone)]
struct OnLayout(ValueChanged<Size>);

impl Debug for OnLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("onLayout")
    }
}

impl RenderObjectWidget for MenuItem {
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

impl SingleChildRenderObjectWidget for MenuItem {
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

impl RenderShiftedBox for RenderMenuItem {}

impl RenderObject for RenderMenuItem {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        match self.child(app) {
            None => self.set_size(app, Size::ZERO),
            Some(child) => {
                child.layout(app, constraints, true);
                let size = constraints.constrain(child.size(app));
                self.set_size(app, size);
                child.parent_data_of_mut::<BoxParentData>(app).offset = Offset::ZERO;
            }
        }
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
        RenderShiftedBox::paint(self, app, context, offset);
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

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderShiftedBox::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderShiftedBox::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderShiftedBox::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderShiftedBox::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        match self.child(app) {
            Some(child) => child.get_dry_layout(app, constraints),
            None => Size::ZERO,
        }
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.child(app)
            .and_then(|child| child.get_dry_baseline(app, constraints, baseline))
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }
}

// ---------------------------------------------------------------------------------------------
// PopupMenuItem

/// The fields of Dart's `PopupMenuItem<T>`, which [`PopupMenuItem`] and its subclasses, such
/// as [`CheckedPopupMenuItem`], carry under the field `popup_menu_item`.
pub struct PopupMenuItemData<T> {
    pub key: Option<KeyRef>,

    /// The value that will be returned by [`show_menu`] if this entry is selected.
    pub value: Option<T>,

    /// Called when the menu item is tapped.
    pub on_tap: Option<Listener>,

    /// Whether the user is permitted to select this item.
    ///
    /// Defaults to true. If this is false, then the item will not react to touches.
    pub enabled: bool,

    /// The minimum height of the menu item.
    ///
    /// Defaults to [`K_MIN_INTERACTIVE_DIMENSION`] pixels.
    pub height: f64,

    /// The padding of the menu item.
    ///
    /// The [`height`](Self::height) property may interact with the applied padding. For
    /// example, If a [`height`](Self::height) greater than the height of the sum of the padding
    /// and [`child`](Self::child) is provided, then the padding's effect will not be visible.
    ///
    /// If this is `None` and `ThemeData::use_material3` is true, the horizontal padding
    /// defaults to 12.0 on both sides.
    ///
    /// If this is `None` and `ThemeData::use_material3` is false, the horizontal padding
    /// defaults to 16.0 on both sides.
    pub padding: Option<EdgeInsets>,

    /// The text style of the popup menu item.
    ///
    /// If this property is `None`, then [`PopupMenuThemeData::text_style`] is used. If
    /// [`PopupMenuThemeData::text_style`] is also `None`, then `TextTheme::title_medium` of
    /// `ThemeData::text_theme` is used.
    pub text_style: Option<TextStyle>,

    /// The label style of the popup menu item.
    ///
    /// When `ThemeData::use_material3` is true, this styles the text of the popup menu item.
    ///
    /// If this property is `None`, then [`PopupMenuThemeData::label_text_style`] is used. If
    /// [`PopupMenuThemeData::label_text_style`] is also `None`, then `TextTheme::label_large`
    /// is used with the `ColorScheme::on_surface` color when popup menu item is enabled and the
    /// `ColorScheme::on_surface` color with 0.38 opacity when the popup menu item is disabled.
    pub label_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,

    /// {@template material_ui.popupmenu.mouseCursor}
    /// The cursor for a mouse pointer when it enters or is hovering over the widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a [`WidgetStateMouseCursor`],
    /// [`WidgetStateProperty::resolve`] is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// {@endtemplate}
    ///
    /// If `None`, then the value of [`PopupMenuThemeData::mouse_cursor`] is used. If that is
    /// also `None`, then [`WidgetStateMouseCursor::adaptive_clickable`] is used.
    pub mouse_cursor: Option<MouseCursorRef>,

    /// The widget below this widget in the tree.
    ///
    /// Typically a single-line [`ListTile`] (for menus with icons) or a `Text`. An appropriate
    /// `DefaultTextStyle` is put in scope for the child. In either case, the text should be
    /// short enough that it won't wrap.
    pub child: Option<WidgetRef>,
}

impl<T> PopupMenuItemData<T> {
    /// The fields of an item showing `child`, with Dart's defaults.
    pub fn new(child: Option<WidgetRef>) -> PopupMenuItemData<T> {
        PopupMenuItemData {
            key: None,
            value: None,
            on_tap: None,
            enabled: true,
            height: K_MIN_INTERACTIVE_DIMENSION,
            padding: None,
            text_style: None,
            label_text_style: None,
            mouse_cursor: None,
            child,
        }
    }
}

impl<T: Debug> Debug for PopupMenuItemData<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PopupMenuItem")
            .field("value", &self.value)
            .field("enabled", &self.enabled)
            .field("height", &self.height)
            .field("padding", &self.padding)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

/// The fluent setters of Dart's `PopupMenuItem` named arguments, defined on [`PopupMenuItem`]
/// and on [`CheckedPopupMenuItem`], whose constructor passes them on as `super.` arguments.
macro_rules! popup_menu_item_setters {
    ($name:ident) => {
        #[doc = concat!("Dart `", stringify!($name), "(key:)`.")]
        pub fn key(mut self, key: KeyRef) -> $name<T> {
            self.popup_menu_item.key = Some(key);
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(value:)`.")]
        pub fn value(mut self, value: T) -> $name<T> {
            self.popup_menu_item.value = Some(value);
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(onTap:)`.")]
        pub fn on_tap(mut self, on_tap: Listener) -> $name<T> {
            self.popup_menu_item.on_tap = Some(on_tap);
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(enabled:)`.")]
        pub fn enabled(mut self, enabled: bool) -> $name<T> {
            self.popup_menu_item.enabled = enabled;
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(height:)`.")]
        pub fn height(mut self, height: f64) -> $name<T> {
            self.popup_menu_item.height = height;
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(padding:)`.")]
        pub fn padding(mut self, padding: EdgeInsets) -> $name<T> {
            self.popup_menu_item.padding = Some(padding);
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(labelTextStyle:)`.")]
        pub fn label_text_style(
            mut self,
            label_text_style: WidgetStatePropertyRef<Option<TextStyle>>,
        ) -> $name<T> {
            self.popup_menu_item.label_text_style = Some(label_text_style);
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(mouseCursor:)`.")]
        pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> $name<T> {
            self.popup_menu_item.mouse_cursor = Some(mouse_cursor);
            self
        }
    };
}

/// The trait every popup-menu-item widget implements: [`PopupMenuItem`] and its subclasses,
/// such as [`CheckedPopupMenuItem`]. Dart's `W extends PopupMenuItem<T>`.
///
/// Its [`State`] is a [`PopupMenuItemStateLeaf`].
pub trait PopupMenuItemLeaf: StatefulWidget {
    /// Dart's `T` of `PopupMenuItem<T>`.
    type Value: Clone + PartialEq + 'static;

    /// The [`PopupMenuItemData`] bag, held under the field `popup_menu_item`.
    fn popup_menu_item_data(&self) -> &PopupMenuItemData<Self::Value>;
}

/// An item in a Material Design popup menu.
///
/// To show a popup menu, use the [`show_menu`] function. To create a button that shows a popup
/// menu, consider using [`PopupMenuButton`].
///
/// To show a checkmark next to a popup menu item, consider using [`CheckedPopupMenuItem`].
///
/// Typically the [`child`](PopupMenuItemData::child) of a [`PopupMenuItem`] is a `Text`
/// widget. More elaborate menus with icons can use a [`ListTile`]. By default, a
/// [`PopupMenuItem`] is [`K_MIN_INTERACTIVE_DIMENSION`] pixels high. If you use a widget with
/// a different height, it must be specified in the [`height`](PopupMenuItemData::height)
/// property.
///
/// See also:
///
///  * [`PopupMenuDivider`], which can be used to divide items from each other.
///  * [`CheckedPopupMenuItem`], a variant of [`PopupMenuItem`] with a checkmark.
///  * [`show_menu`], a method to dynamically show a popup menu at a given location.
///  * [`PopupMenuButton`], an [`IconButton`] that automatically shows a menu when it is tapped.
// TODO(framework): Add unit tests to this code snippet.
// https://github.com/flutter/flutter/issues/188530
pub struct PopupMenuItem<T: 'static> {
    pub popup_menu_item: PopupMenuItemData<T>,
}

impl<T: Clone + PartialEq + 'static> PopupMenuItem<T> {
    /// Creates an item for a popup menu.
    ///
    /// By default, the item is [`enabled`](PopupMenuItemData::enabled).
    pub fn new<K>(child: Option<impl IntoWidget<K>>) -> PopupMenuItem<T> {
        PopupMenuItem {
            popup_menu_item: PopupMenuItemData::new(child.map(IntoWidget::into_widget)),
        }
    }

    popup_menu_item_setters!(PopupMenuItem);

    /// Dart `PopupMenuItem(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> PopupMenuItem<T> {
        self.popup_menu_item.text_style = Some(text_style);
        self
    }
}

impl<T: 'static> Debug for PopupMenuItem<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PopupMenuItem")
            .field("enabled", &self.popup_menu_item.enabled)
            .field("height", &self.popup_menu_item.height)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> PopupMenuItemLeaf for PopupMenuItem<T> {
    type Value = T;

    fn popup_menu_item_data(&self) -> &PopupMenuItemData<T> {
        &self.popup_menu_item
    }
}

impl<T: Clone + PartialEq + 'static> PopupMenuEntry<T> for PopupMenuItem<T> {
    fn height(&self) -> f64 {
        self.popup_menu_item.height
    }

    fn represents(&self, value: Option<&T>) -> bool {
        value == self.popup_menu_item.value.as_ref()
    }
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for PopupMenuItem<T> {
    type State = PopupMenuItemState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.popup_menu_item.key.as_ref()
    }

    fn create_state(&self) -> PopupMenuItemState<T> {
        PopupMenuItemState {
            state: StateData::new(),
        }
    }
}

/// The value type of a popup-menu-item state: its widget's `T`.
pub type PopupMenuItemValue<S> = <<S as State>::Widget as PopupMenuItemLeaf>::Value;

/// The trait every popup-menu-item state implements: [`PopupMenuItemState`] and its
/// subclasses, such as [`CheckedPopupMenuItem`]'s state.
///
/// The virtuals a subclass overrides are the members here, each defaulting to the body of
/// [`PopupMenuItemState`] of its name, which an override calls where Dart writes `super.…`;
/// a leaf's own [`State::build`] runs [`PopupMenuItemState::build`].
pub trait PopupMenuItemStateLeaf: State<Widget: PopupMenuItemLeaf> {
    /// The menu item contents.
    ///
    /// Used by the [`State::build`] method.
    ///
    /// By default, this returns [`PopupMenuItemData::child`]. Override this to put something
    /// else in the menu entry.
    fn build_child(self: Handle<Self>, app: &mut App, context: BuildContext) -> Option<WidgetRef> {
        PopupMenuItemState::<PopupMenuItemValue<Self>>::build_child(self, app, context)
    }

    /// The handler for when the user selects the menu item.
    ///
    /// Used by the `InkWell` inserted by the [`State::build`] method.
    ///
    /// By default, uses [`Navigator::pop`] to return the [`PopupMenuItemData::value`] from the
    /// menu route.
    fn handle_tap(self: Handle<Self>, app: &mut App) {
        PopupMenuItemState::<PopupMenuItemValue<Self>>::handle_tap(self, app);
    }

    /// Builds the semantics for the menu item.
    fn build_semantics(self: Handle<Self>, app: &mut App, child: WidgetRef) -> WidgetRef {
        PopupMenuItemState::<PopupMenuItemValue<Self>>::build_semantics(self, app, child)
    }
}

/// The [`State`] for [`PopupMenuItem`] subclasses.
///
/// By default this implements the basic styling and layout of Material Design popup menu
/// items.
///
/// The [`PopupMenuItemStateLeaf::build_child`] method can be overridden to adjust exactly
/// what gets placed in the menu. By default it returns [`PopupMenuItemData::child`].
///
/// The [`PopupMenuItemStateLeaf::handle_tap`] method can be overridden to adjust exactly what
/// happens when the item is tapped. By default, it uses [`Navigator::pop`] to return the
/// [`PopupMenuItemData::value`] from the menu route.
///
/// This struct is the state of [`PopupMenuItem`] itself, Dart's
/// `PopupMenuItemState<T, PopupMenuItem<T>>`; its associated functions are the bodies Dart's
/// `PopupMenuItemState` gives its subclasses.
pub struct PopupMenuItemState<T: Clone + PartialEq + 'static> {
    state: StateData<PopupMenuItem<T>>,
}

impl<T: Clone + PartialEq + 'static> Debug for PopupMenuItemState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PopupMenuItemState").finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> PopupMenuItemState<T> {
    /// See [`PopupMenuItemStateLeaf::build_child`].
    pub fn build_child<S: PopupMenuItemStateLeaf<Widget: PopupMenuItemLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
        _context: BuildContext,
    ) -> Option<WidgetRef> {
        this.widget(app).popup_menu_item_data().child.clone()
    }

    /// See [`PopupMenuItemStateLeaf::handle_tap`].
    pub fn handle_tap<S: PopupMenuItemStateLeaf<Widget: PopupMenuItemLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
    ) {
        // Need to pop the navigator first in case onTap may push new route onto navigator.
        let context = this.context(app);
        let value = this.widget(app).popup_menu_item_data().value.clone();
        Navigator::pop(
            app,
            context,
            value.map(|value| Rc::new(value) as Rc<dyn Any>),
        );

        if let Some(on_tap) = this.widget(app).popup_menu_item_data().on_tap.clone() {
            on_tap.call(app);
        }
    }

    /// Dart's `PopupMenuItemState.build`, which a leaf's [`State::build`] runs.
    pub fn build<S: PopupMenuItemStateLeaf<Widget: PopupMenuItemLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
        context: BuildContext,
    ) -> WidgetRef {
        let theme = Theme::of(app, context);
        let popup_menu_theme = PopupMenuTheme::of(app, context);
        let defaults: Box<dyn PopupMenuThemeDataGetters> = if theme.use_material3 {
            Box::new(PopupMenuDefaultsM3::new(context))
        } else {
            Box::new(PopupMenuDefaultsM2::new(context))
        };
        let widget_ref = context.widget(app).clone();
        let widget = waft_widgets::downcast_widget::<S::Widget>(&*widget_ref)
            .expect("a popup menu item's state builds its item")
            .popup_menu_item_data();
        let enabled = widget.enabled;
        let mut states = WidgetStates::new();
        if !enabled {
            states.insert(WidgetState::Disabled);
        }

        let mut style = if theme.use_material3 {
            widget
                .label_text_style
                .as_ref()
                .and_then(|label_text_style| label_text_style.resolve(&states))
                .or_else(|| {
                    popup_menu_theme
                        .label_text_style
                        .as_ref()
                        .map(|label_text_style| {
                            label_text_style
                                .resolve(&states)
                                .expect("PopupMenuThemeData.labelTextStyle resolves a style")
                        })
                })
                .unwrap_or_else(|| {
                    defaults
                        .label_text_style(app)
                        .as_ref()
                        .expect("the defaults have a labelTextStyle")
                        .resolve(&states)
                        .expect("the defaults' labelTextStyle resolves a style")
                })
        } else {
            widget
                .text_style
                .clone()
                .or_else(|| popup_menu_theme.text_style.clone())
                .or_else(|| defaults.text_style(app))
                .expect("the defaults have a textStyle")
        };

        if !enabled && !theme.use_material3 {
            style = style.copy_with().color(theme.disabled_color);
        }
        let padding: EdgeInsetsGeometry = widget
            .padding
            .unwrap_or(if theme.use_material3 {
                MENU_ITEM_PADDING
            } else {
                POPUP_MENU_DEFAULTS_M2_MENU_ITEM_PADDING
            })
            .into();
        let height = widget.height;
        let mouse_cursor = widget.mouse_cursor.clone();

        let mut align = Align::new().alignment(AlignmentGeometry::CENTER_START);
        if let Some(child) = this.build_child(app, context) {
            align = align.child(child);
        }
        let mut item = AnimatedDefaultTextStyle::new(
            ConstrainedBox::new(BoxConstraints::new().min_height(height))
                .child(Padding::new(padding).child(align)),
            style.clone(),
            K_THEME_CHANGE_DURATION,
        )
        .into_widget();

        if !enabled {
            let is_dark = theme.brightness() == Brightness::Dark;
            item = IconTheme::merge(
                None,
                IconThemeData::new().opacity(if is_dark { 0.5 } else { 0.38 }),
                item,
            );
        }

        let mut ink_well = InkWell::new()
            .can_request_focus(enabled)
            .mouse_cursor(
                effective_mouse_cursor(mouse_cursor, popup_menu_theme.mouse_cursor.clone()).into(),
            )
            .child(
                ListTileTheme::merge(item)
                    .content_padding(EdgeInsetsGeometry::ZERO)
                    .title_text_style(style),
            );
        if enabled {
            ink_well = ink_well.on_tap(Listener::handle_method(this, S::handle_tap));
        }
        let semantics = this.build_semantics(app, ink_well.into_widget());
        MergeSemantics::new().child(semantics).into_widget()
    }

    /// See [`PopupMenuItemStateLeaf::build_semantics`].
    pub fn build_semantics<S: PopupMenuItemStateLeaf<Widget: PopupMenuItemLeaf<Value = T>>>(
        this: Handle<S>,
        app: &mut App,
        child: WidgetRef,
    ) -> WidgetRef {
        Semantics::new()
            .role(SemanticsRole::MenuItem)
            .enabled(this.widget(app).popup_menu_item_data().enabled)
            .button(true)
            .child(child)
            .into_widget()
    }
}

impl<T: Clone + PartialEq + 'static> PopupMenuItemStateLeaf for PopupMenuItemState<T> {}

impl<T: Clone + PartialEq + 'static> State for PopupMenuItemState<T> {
    type Widget = PopupMenuItem<T>;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        PopupMenuItemState::<T>::build(self, app, context)
    }
}

// ---------------------------------------------------------------------------------------------
// CheckedPopupMenuItem

/// An item with a checkmark in a Material Design popup menu.
///
/// To show a popup menu, use the [`show_menu`] function. To create a button that shows a popup
/// menu, consider using [`PopupMenuButton`].
///
/// A [`CheckedPopupMenuItem`] is [`K_MIN_INTERACTIVE_DIMENSION`] pixels high, which matches
/// the default minimum height of a [`PopupMenuItem`]. The horizontal layout uses [`ListTile`];
/// the checkmark is an `Icons::done` icon, shown in the `ListTile::leading` position.
///
/// See also:
///
///  * [`PopupMenuItem`], a popup menu entry for picking a command (as opposed to toggling a
///    value).
///  * [`PopupMenuDivider`], a popup menu entry that is just a horizontal line.
///  * [`show_menu`], a method to dynamically show a popup menu at a given location.
///  * [`PopupMenuButton`], an [`IconButton`] that automatically shows a menu when it is tapped.
// TODO(framework): Add unit tests to this code snippet.
// https://github.com/flutter/flutter/issues/188530
pub struct CheckedPopupMenuItem<T: 'static> {
    pub popup_menu_item: PopupMenuItemData<T>,

    /// Whether to display a checkmark next to the menu item.
    ///
    /// Defaults to false.
    ///
    /// When true, an `Icons::done` checkmark is displayed.
    ///
    /// When this popup menu item is selected, the checkmark will fade in or out as appropriate
    /// to represent the implied new state.
    pub checked: bool,
}

impl<T: Clone + PartialEq + 'static> CheckedPopupMenuItem<T> {
    /// Creates a popup menu item with a checkmark.
    ///
    /// By default, the menu item is [`enabled`](PopupMenuItemData::enabled) but unchecked. To
    /// mark the item as checked, set [`checked`](Self::checked) to true.
    pub fn new() -> CheckedPopupMenuItem<T> {
        CheckedPopupMenuItem {
            popup_menu_item: PopupMenuItemData::new(None),
            checked: false,
        }
    }

    popup_menu_item_setters!(CheckedPopupMenuItem);

    /// Dart `CheckedPopupMenuItem(checked:)`.
    pub fn checked(mut self, checked: bool) -> CheckedPopupMenuItem<T> {
        self.checked = checked;
        self
    }

    /// Dart `CheckedPopupMenuItem(child:)`.
    ///
    /// The widget below this widget in the tree.
    ///
    /// Typically a `Text`. An appropriate `DefaultTextStyle` is put in scope for the child. The
    /// text should be short enough that it won't wrap.
    ///
    /// This widget is placed in the `ListTile::title` slot of a [`ListTile`] whose
    /// `ListTile::leading` slot is an `Icons::done` icon.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> CheckedPopupMenuItem<T> {
        self.popup_menu_item.child = Some(child.into_widget());
        self
    }
}

impl<T: Clone + PartialEq + 'static> Default for CheckedPopupMenuItem<T> {
    fn default() -> CheckedPopupMenuItem<T> {
        CheckedPopupMenuItem::new()
    }
}

impl<T: 'static> Debug for CheckedPopupMenuItem<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheckedPopupMenuItem")
            .field("checked", &self.checked)
            .field("enabled", &self.popup_menu_item.enabled)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> PopupMenuItemLeaf for CheckedPopupMenuItem<T> {
    type Value = T;

    fn popup_menu_item_data(&self) -> &PopupMenuItemData<T> {
        &self.popup_menu_item
    }
}

impl<T: Clone + PartialEq + 'static> PopupMenuEntry<T> for CheckedPopupMenuItem<T> {
    fn height(&self) -> f64 {
        self.popup_menu_item.height
    }

    fn represents(&self, value: Option<&T>) -> bool {
        value == self.popup_menu_item.value.as_ref()
    }
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for CheckedPopupMenuItem<T> {
    type State = CheckedPopupMenuItemState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.popup_menu_item.key.as_ref()
    }

    fn create_state(&self) -> CheckedPopupMenuItemState<T> {
        CheckedPopupMenuItemState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            controller: None,
        }
    }
}

/// Dart's `_CheckedPopupMenuItemState`.
pub struct CheckedPopupMenuItemState<T: Clone + PartialEq + 'static> {
    state: StateData<CheckedPopupMenuItem<T>>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    /// Created in `init_state`, because a state is created without the [`App`].
    controller: Option<Handle<waft_animation::AnimationController>>,
}

impl<T: Clone + PartialEq + 'static> CheckedPopupMenuItemState<T> {
    const FADE_DURATION: Duration = Duration::from_millis(150);

    fn controller(self: Handle<Self>, app: &App) -> Handle<waft_animation::AnimationController> {
        app.get(self).controller.expect("created in init_state")
    }

    fn opacity(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        self.controller(app).view()
    }

    fn animation_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {
            // animation changed
        });
    }
}

impl<T: Clone + PartialEq + 'static> Debug for CheckedPopupMenuItemState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheckedPopupMenuItemState")
            .field("controller", &self.controller)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> SingleTickerProviderStateMixin
    for CheckedPopupMenuItemState<T>
{
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
impl<T: Clone + PartialEq + 'static> waft_scheduler::TickerProviderObject
    for CheckedPopupMenuItemState<T>
{
    fn create_ticker(
        self: Handle<Self>,
        app: &mut App,
        on_tick: waft_scheduler::TickerCallback,
    ) -> Handle<waft_scheduler::Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl<T: Clone + PartialEq + 'static> PopupMenuItemStateLeaf for CheckedPopupMenuItemState<T> {
    fn handle_tap(self: Handle<Self>, app: &mut App) {
        // This fades the checkmark in or out when tapped.
        let controller = self.controller(app);
        if self.widget(app).checked {
            controller.reverse(app, None);
        } else {
            controller.forward(app, None);
        }
        PopupMenuItemState::<T>::handle_tap(self, app);
    }

    fn build_semantics(self: Handle<Self>, app: &mut App, child: WidgetRef) -> WidgetRef {
        let widget = self.widget(app);
        Semantics::new()
            .role(SemanticsRole::MenuItemCheckbox)
            .enabled(widget.popup_menu_item.enabled)
            .checked(widget.checked)
            .button(true)
            .child(child)
            .into_widget()
    }

    fn build_child(self: Handle<Self>, app: &mut App, context: BuildContext) -> Option<WidgetRef> {
        let theme = Theme::of(app, context);
        let popup_menu_theme = PopupMenuTheme::of(app, context);
        let defaults: Box<dyn PopupMenuThemeDataGetters> = if theme.use_material3 {
            Box::new(PopupMenuDefaultsM3::new(context))
        } else {
            Box::new(PopupMenuDefaultsM2::new(context))
        };
        let widget_ref = context.widget(app).clone();
        let widget = waft_widgets::downcast_widget::<<Self as State>::Widget>(&*widget_ref)
            .expect("a checked popup menu item's state builds its item");
        let mut states = WidgetStates::new();
        if widget.checked {
            states.insert(WidgetState::Selected);
        }
        let effective_label_text_style = widget
            .popup_menu_item
            .label_text_style
            .clone()
            .or(popup_menu_theme.label_text_style)
            .or_else(|| defaults.label_text_style(app));
        let enabled = widget.popup_menu_item.enabled;
        let title = widget.popup_menu_item.child.clone();
        let controller = self.controller(app);
        let icon = if controller.status(app).is_dismissed() {
            None
        } else {
            Some(Icons::done())
        };
        let mut list_tile = ListTile::new()
            .enabled(enabled)
            .leading(FadeTransition::new(self.opacity(app)).child(Icon::new(icon)));
        if let Some(title_text_style) = effective_label_text_style
            .as_ref()
            .and_then(|label_text_style| label_text_style.resolve(&states))
        {
            list_tile = list_tile.title_text_style(title_text_style);
        }
        if let Some(title) = title {
            list_tile = list_tile.title(title);
        }
        Some(
            IgnorePointer::new()
                .child(ListTileTheme::merge(list_tile).content_padding(EdgeInsetsGeometry::ZERO))
                .into_widget(),
        )
    }
}

impl<T: Clone + PartialEq + 'static> State for CheckedPopupMenuItemState<T> {
    type Widget = CheckedPopupMenuItem<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = waft_animation::AnimationController::create(
            app,
            None,
            Some(Self::FADE_DURATION),
            None,
            0.0,
            1.0,
            waft_animation::AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).controller = Some(controller);
        let value = if self.widget(app).checked { 1.0 } else { 0.0 };
        controller.set_value(app, value);
        controller.add_listener(app, Listener::handle_method(self, Self::animation_changed));
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
        app.get_mut(self).controller = None;
        app.destroy(controller);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        PopupMenuItemState::<T>::build(self, app, context)
    }
}

/// Dart's `_EffectiveMouseCursor`: a [`WidgetStateMouseCursor`] passed along to the menu
/// item's `InkWell`, which resolves the property against [`WidgetState::Disabled`],
/// [`WidgetState::Hovered`] and [`WidgetState::Focused`].
fn effective_mouse_cursor(
    widget_cursor: Option<MouseCursorRef>,
    theme_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
) -> WidgetStateMouseCursor {
    WidgetStateMouseCursor::resolve_with(
        move |states| {
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&widget_cursor, states)
                .or_else(|| {
                    theme_cursor
                        .as_ref()
                        .and_then(|theme_cursor| theme_cursor.resolve(states))
                })
                .unwrap_or_else(|| WidgetStateMouseCursor::adaptive_clickable().resolve(states))
        },
        Some("WidgetStateMouseCursor(PopupMenuItemState)"),
    )
}

/// Hand coded defaults based on Material Design 2.
///
/// Dart's `_PopupMenuDefaultsM2`.
struct PopupMenuDefaultsM2 {
    /// Dart's `super(..)` arguments.
    popup_menu_theme_data: PopupMenuThemeData,
    context: DefaultsContext,
}

impl PopupMenuDefaultsM2 {
    fn new(context: BuildContext) -> PopupMenuDefaultsM2 {
        PopupMenuDefaultsM2 {
            popup_menu_theme_data: PopupMenuThemeData::new().elevation(8.0),
            context: DefaultsContext::new(context),
        }
    }
}

impl PopupMenuThemeDataGetters for PopupMenuDefaultsM2 {
    fn popup_menu_theme_data(&self) -> &PopupMenuThemeData {
        &self.popup_menu_theme_data
    }

    fn text_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).title_medium.clone()
    }

    fn menu_padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        Some(EdgeInsetsGeometry::symmetric(8.0, 0.0))
    }
}

/// Dart's `_PopupMenuDefaultsM2.menuItemPadding`.
const POPUP_MENU_DEFAULTS_M2_MENU_ITEM_PADDING: EdgeInsets = EdgeInsets::symmetric(0.0, 16.0);

// ---------------------------------------------------------------------------------------------
// _PopupMenu

/// Dart's `_PopupMenu<T>`.
struct PopupMenu<T: Clone + PartialEq + 'static> {
    item_keys: Vec<GlobalKey>,
    route: Handle<PopupMenuRoute<T>>,
    semantic_label: Option<String>,
    constraints: Option<BoxConstraints>,
    clip_behavior: Clip,
}

impl<T: Clone + PartialEq + 'static> Debug for PopupMenu<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PopupMenu")
            .field("semantic_label", &self.semantic_label)
            .field("constraints", &self.constraints)
            .field("clip_behavior", &self.clip_behavior)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for PopupMenu<T> {
    type State = PopupMenuState<T>;

    fn create_state(&self) -> PopupMenuState<T> {
        PopupMenuState {
            state: StateData::new(),
            opacities: Vec::new(),
        }
    }
}

/// Dart's `_PopupMenuState<T>`.
struct PopupMenuState<T: Clone + PartialEq + 'static> {
    state: StateData<PopupMenu<T>>,
    opacities: Vec<Handle<CurvedAnimation>>,
}

impl<T: Clone + PartialEq + 'static> Debug for PopupMenuState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PopupMenuState")
            .field("opacities", &self.opacities.len())
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> PopupMenuState<T> {
    fn route_animation(app: &App, route: Handle<PopupMenuRoute<T>>) -> AnyAnimation<f64> {
        ModalRoute::animation(route, app).expect("an installed route has an animation")
    }

    fn set_opacities(self: Handle<Self>, app: &mut App) {
        for opacity in std::mem::take(&mut app.get_mut(self).opacities) {
            opacity.dispose(app);
            app.destroy_lingering(opacity);
        }
        let route = self.widget(app).route;
        let count = app.get(route).items.len();
        let animation = Self::route_animation(app, route);
        let mut new_opacities = Vec::new();
        // 1.0 for the width and 0.5 for the last item's fade.
        let unit = 1.0 / (count as f64 + 1.5);
        for i in 0..count {
            let start = (i as f64 + 1.0) * unit;
            let end = (start + 1.5 * unit).clamp(0.0, 1.0);
            let opacity = CurvedAnimation::create(
                app,
                animation,
                Rc::new(Interval::new(start, end, Curves::linear())),
                None,
            );
            new_opacities.push(opacity);
        }
        app.get_mut(self).opacities = new_opacities;
    }
}

impl<T: Clone + PartialEq + 'static> State for PopupMenuState<T> {
    type Widget = PopupMenu<T>;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.set_opacities(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &PopupMenu<T>) {
        let route = self.widget(app).route;
        if app.get(old_widget.route).items.len() != app.get(route).items.len()
            || ModalRoute::animation(old_widget.route, app) != ModalRoute::animation(route, app)
        {
            self.set_opacities(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for opacity in std::mem::take(&mut app.get_mut(self).opacities) {
            opacity.dispose(app);
            app.destroy_lingering(opacity);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let route = self.widget(app).route;
        let count = app.get(route).items.len();
        // 1.0 for the width and 0.5 for the last item's fade.
        let unit = 1.0 / (count as f64 + 1.5);
        let mut children = Vec::new();
        let theme = Theme::of(app, context);
        let popup_menu_theme = PopupMenuTheme::of(app, context);
        let defaults: Box<dyn PopupMenuThemeDataGetters> = if theme.use_material3 {
            Box::new(PopupMenuDefaultsM3::new(context))
        } else {
            Box::new(PopupMenuDefaultsM2::new(context))
        };

        for i in 0..count {
            let opacity = app.get(self).opacities[i].as_animation();
            let menu_route = app.get(route);
            let mut item = menu_route.items[i].widget().clone();
            if let Some(initial_value) = &menu_route.initial_value
                && menu_route.items[i].represents(Some(initial_value))
            {
                item = ColoredBox::new(Theme::of(app, context).highlight_color)
                    .child(item)
                    .into_widget();
            }
            let item_sizes = Rc::clone(&app.get(route).item_sizes);
            let item_key: KeyRef = self.widget(app).item_keys[i].clone().into();
            children.push(
                MenuItem {
                    on_layout: OnLayout(Rc::new(move |_app, size| {
                        item_sizes.borrow_mut()[i] = Some(size);
                    })),
                    child: Some(
                        FadeTransition::new(opacity)
                            .key(item_key)
                            .child(item)
                            .into_widget(),
                    ),
                }
                .into_widget(),
            );
        }

        let animation = PopupMenuState::<T>::route_animation(app, route);
        let opacity = CurveTween::new(
            app,
            Rc::new(Interval::new(0.0, 1.0 / 3.0, Curves::linear())),
        );
        let width = CurveTween::new(app, Rc::new(Interval::new(0.0, unit, Curves::linear())));
        let height = CurveTween::new(
            app,
            Rc::new(Interval::new(0.0, unit * count as f64, Curves::linear())),
        );

        let widget = self.widget(app);
        let (semantic_label, constraints, clip_behavior) = (
            widget.semantic_label.clone(),
            widget.constraints,
            widget.clip_behavior,
        );
        let menu_route = app.get(route);
        let (menu_padding, route_shape, route_color, route_elevation) = (
            menu_route.menu_padding,
            menu_route.shape.clone(),
            menu_route.color,
            menu_route.elevation,
        );
        let (route_shadow_color, route_surface_tint_color) =
            (menu_route.shadow_color, menu_route.surface_tint_color);
        let mut scroll_view =
            SingleChildScrollView::new().child(ListBody::new().children(children));
        if let Some(padding) = menu_padding
            .or(popup_menu_theme.menu_padding)
            .or_else(|| defaults.menu_padding(app))
        {
            scroll_view = scroll_view.padding(padding);
        }
        let mut semantics = Semantics::new()
            .role(SemanticsRole::Menu)
            .scopes_route(true)
            .names_route(true)
            .explicit_child_nodes(true);
        if let Some(semantic_label) = semantic_label {
            semantics = semantics.label(semantic_label);
        }
        let child = ConstrainedBox::new(
            constraints.unwrap_or(
                BoxConstraints::new()
                    .min_width(K_MENU_MIN_WIDTH)
                    .max_width(K_MENU_MAX_WIDTH),
            ),
        )
        .child(
            IntrinsicWidth::new()
                .step_width(K_MENU_WIDTH_STEP)
                .child(semantics.child(scroll_view)),
        )
        .into_widget();

        let shape = route_shape
            .or(popup_menu_theme.shape)
            .or_else(|| defaults.shape(app));
        let color = route_color
            .or(popup_menu_theme.color)
            .or_else(|| defaults.color(app));
        let elevation = route_elevation
            .or(popup_menu_theme.elevation)
            .or_else(|| defaults.elevation(app))
            .expect("the defaults have an elevation");
        let shadow_color = route_shadow_color
            .or(popup_menu_theme.shadow_color)
            .or_else(|| defaults.shadow_color(app));
        let surface_tint_color = route_surface_tint_color
            .or(popup_menu_theme.surface_tint_color)
            .or_else(|| defaults.surface_tint_color(app));
        let opacity = opacity.animate(app, animation);

        AnimatedBuilder::new(Rc::new(animation), move |app, _context, child| {
            let mut material = Material::new()
                .clip_behavior(clip_behavior)
                .r#type(MaterialType::Card)
                .elevation(elevation);
            if let Some(shape) = &shape {
                material = material.shape(shape.clone());
            }
            if let Some(color) = color {
                material = material.color(color);
            }
            if let Some(shadow_color) = shadow_color {
                material = material.shadow_color(shadow_color);
            }
            if let Some(surface_tint_color) = surface_tint_color {
                material = material.surface_tint_color(surface_tint_color);
            }
            let mut align = Align::new()
                .alignment(AlignmentGeometry::TOP_END)
                .width_factor(width.evaluate(app, animation))
                .height_factor(height.evaluate(app, animation));
            if let Some(child) = child {
                align = align.child(child.clone());
            }
            FadeTransition::new(opacity)
                .child(material.child(align))
                .into_widget()
        })
        .child(child)
        .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// _PopupMenuRouteLayout

/// Positioning of the menu on the screen; Dart's `_PopupMenuRouteLayout`.
struct PopupMenuRouteLayout {
    /// Rectangle of underlying button, relative to the overlay's dimensions.
    position: RelativeRect,

    /// The sizes of each item are computed when the menu is laid out, and before the route is
    /// laid out. The route's own list: Dart hands the delegate the route's `itemSizes`.
    item_sizes: Rc<RefCell<Vec<Option<Size>>>>,

    /// The index of the selected item, or `None` if `PopupMenuButton::initial_value` was not
    /// specified.
    selected_item_index: Option<usize>,

    /// Whether to prefer going to the left or to the right.
    text_direction: TextDirection,

    /// The padding of unsafe area.
    padding: EdgeInsets,
}

impl Debug for PopupMenuRouteLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PopupMenuRouteLayout")
            .field("selected_item_index", &self.selected_item_index)
            .field("text_direction", &self.text_direction)
            .field("padding", &self.padding)
            .finish_non_exhaustive()
    }
}

impl PopupMenuRouteLayout {
    fn closest_screen(screens: &[Rect], point: Offset) -> Rect {
        let mut closest = screens[0];
        for screen in screens {
            if (screen.center() - point).distance() < (closest.center() - point).distance() {
                closest = *screen;
            }
        }
        closest
    }

    fn fit_inside_screen(&self, screen: Rect, child_size: Size, wanted_position: Offset) -> Offset {
        let padding = self.padding;
        let mut x = wanted_position.dx();
        let mut y = wanted_position.dy();
        // Avoid going outside an area defined as the rectangle 8.0 pixels from the
        // edge of the screen in every direction.
        if x < screen.left + K_MENU_SCREEN_PADDING + padding.left {
            x = screen.left + K_MENU_SCREEN_PADDING + padding.left;
        } else if x + child_size.width() > screen.right - K_MENU_SCREEN_PADDING - padding.right {
            x = screen.right - child_size.width() - K_MENU_SCREEN_PADDING - padding.right;
        }
        if y < screen.top + K_MENU_SCREEN_PADDING + padding.top {
            y = K_MENU_SCREEN_PADDING + padding.top;
        } else if y + child_size.height() > screen.bottom - K_MENU_SCREEN_PADDING - padding.bottom {
            y = screen.bottom - child_size.height() - K_MENU_SCREEN_PADDING - padding.bottom;
        }

        Offset::new(x, y)
    }
}

// We put the child wherever position specifies, so long as it will fit within
// the specified parent size padded (inset) by 8. If necessary, we adjust the
// child's position so that it fits.
impl SingleChildLayoutDelegate for PopupMenuRouteLayout {
    fn get_constraints_for_child(&self, constraints: BoxConstraints) -> BoxConstraints {
        // The menu can be at most the size of the overlay minus 8.0 pixels in each
        // direction.
        BoxConstraints::loose(constraints.biggest())
            .deflate((EdgeInsets::all(K_MENU_SCREEN_PADDING) + self.padding).into())
    }

    fn get_position_for_child(&self, size: Size, child_size: Size) -> Offset {
        let position = self.position;
        let y = position.top;

        // Find the ideal horizontal position.
        // size: The size of the overlay.
        // childSize: The size of the menu, when fully open, as determined by
        // getConstraintsForChild.
        let x = if position.left > position.right {
            // Menu button is closer to the right edge, so grow to the left, aligned to the right edge.
            size.width() - position.right - child_size.width()
        } else if position.left < position.right {
            // Menu button is closer to the left edge, so grow to the right, aligned to the left edge.
            position.left
        } else {
            // Menu button is equidistant from both edges, so grow in reading direction.
            match self.text_direction {
                TextDirection::Rtl => size.width() - position.right - child_size.width(),
                TextDirection::Ltr => position.left,
            }
        };
        let wanted_position = Offset::new(x, y);
        let origin_center = position.to_rect(Offset::ZERO & size).center();
        let sub_screens = [Offset::ZERO & size];
        let sub_screen = Self::closest_screen(&sub_screens, origin_center);
        self.fit_inside_screen(sub_screen, child_size, wanted_position)
    }

    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
        let Some(old_delegate) = old_delegate.as_any().downcast_ref::<PopupMenuRouteLayout>()
        else {
            return true;
        };
        // If called when the old and new itemSizes have been initialized then
        // we expect them to have the same length because there's no practical
        // way to change length of the items list once the menu has been shown.
        debug_assert_eq!(
            self.item_sizes.borrow().len(),
            old_delegate.item_sizes.borrow().len()
        );

        self.position != old_delegate.position
            || self.selected_item_index != old_delegate.selected_item_index
            || self.text_direction != old_delegate.text_direction
            || *self.item_sizes.borrow() != *old_delegate.item_sizes.borrow()
            || self.padding != old_delegate.padding
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------------------------
// _PopupMenuRoute

/// Dart's `_PopupMenuRoute<T>`.
struct PopupMenuRoute<T: Clone + PartialEq + 'static> {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    position: Option<RelativeRect>,
    position_builder: Option<PopupMenuPositionBuilder>,
    items: Vec<PopupMenuEntryRef<T>>,
    item_keys: Vec<GlobalKey>,
    item_sizes: Rc<RefCell<Vec<Option<Size>>>>,
    initial_value: Option<T>,
    elevation: Option<f64>,
    surface_tint_color: Option<Color>,
    shadow_color: Option<Color>,
    semantic_label: Option<String>,
    shape: Option<ShapeBorderRef>,
    menu_padding: Option<EdgeInsetsGeometry>,
    color: Option<Color>,
    captured_themes: Rc<CapturedThemes>,
    constraints: Option<BoxConstraints>,
    clip_behavior: Clip,
    pop_up_animation_style: Option<AnimationStyle>,
    barrier_label: String,
    animation: Option<Handle<CurvedAnimation>>,
}

/// The arguments of Dart's `_PopupMenuRoute` constructor.
struct PopupMenuRouteArguments<T: 'static> {
    position: Option<RelativeRect>,
    position_builder: Option<PopupMenuPositionBuilder>,
    items: Vec<PopupMenuEntryRef<T>>,
    item_keys: Vec<GlobalKey>,
    initial_value: Option<T>,
    elevation: Option<f64>,
    surface_tint_color: Option<Color>,
    shadow_color: Option<Color>,
    barrier_label: String,
    semantic_label: Option<String>,
    shape: Option<ShapeBorderRef>,
    menu_padding: Option<EdgeInsetsGeometry>,
    color: Option<Color>,
    captured_themes: CapturedThemes,
    constraints: Option<BoxConstraints>,
    clip_behavior: Clip,
    settings: Option<RouteSettingsRef>,
    request_focus: Option<bool>,
    pop_up_animation_style: Option<AnimationStyle>,
}

impl<T: Clone + PartialEq + 'static> PopupMenuRoute<T> {
    fn new(app: &mut App, arguments: PopupMenuRouteArguments<T>) -> Handle<PopupMenuRoute<T>> {
        debug_assert!(
            arguments.position.is_some() != arguments.position_builder.is_some(),
            "Either position or positionBuilder must be provided."
        );
        let route = RouteData::new(app, arguments.settings, arguments.request_focus);
        let transition_route = TransitionRouteData::new(app);
        let mut modal_route = ModalRouteData::new(app);
        // Menus always cycle focus through their items irrespective of the
        // focus traversal edge behavior set in the Navigator.
        modal_route.traversal_edge_behavior = Some(TraversalEdgeBehavior::ClosedLoop);
        let item_count = arguments.items.len();
        app.create(PopupMenuRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            position: arguments.position,
            position_builder: arguments.position_builder,
            items: arguments.items,
            item_keys: arguments.item_keys,
            item_sizes: Rc::new(RefCell::new(vec![None; item_count])),
            initial_value: arguments.initial_value,
            elevation: arguments.elevation,
            surface_tint_color: arguments.surface_tint_color,
            shadow_color: arguments.shadow_color,
            semantic_label: arguments.semantic_label,
            shape: arguments.shape,
            menu_padding: arguments.menu_padding,
            color: arguments.color,
            captured_themes: Rc::new(arguments.captured_themes),
            constraints: arguments.constraints,
            clip_behavior: arguments.clip_behavior,
            pop_up_animation_style: arguments.pop_up_animation_style,
            barrier_label: arguments.barrier_label,
            animation: None,
        })
    }

    fn scroll_to(self: Handle<Self>, app: &mut App, selected_item_index: usize) {
        let item_key = app.get(self).item_keys[selected_item_index].clone();
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _duration| {
                if let Some(context) = item_key.current_context(app) {
                    drop(Scrollable::ensure_visible(
                        app,
                        context,
                        0.0,
                        Duration::ZERO,
                        Curves::ease(),
                        ScrollPositionAlignmentPolicy::Explicit,
                    ));
                }
            }),
            None,
        );
    }

    /// Dart's `TransitionRoute.createAnimation`, which the override runs as `super`: the
    /// trait's default body.
    fn transition_route_create_animation(self: Handle<Self>, app: &mut App) -> AnyAnimation<f64> {
        debug_assert!(
            !TransitionRoute::debug_transition_completed(self, app),
            "cannot reuse a route after disposing it"
        );
        let controller =
            TransitionRoute::controller(self, app).expect("createAnimationController ran first");
        controller.view()
    }
}

// Dart's `_PopupMenuRoute.dispose` disposes its curved animation and runs `super.dispose()`,
// so this impl writes out `modal_route_overrides!` with that `dispose`.
impl<T: Clone + PartialEq + 'static> Route for PopupMenuRoute<T> {
    waft_widgets::route_accessors!();

    fn as_transition_route(self: Handle<Self>) -> Option<waft_widgets::AnyTransitionRoute> {
        Some(TransitionRoute::as_transition_route(self))
    }

    fn as_modal_route(self: Handle<Self>) -> Option<waft_widgets::AnyModalRoute> {
        Some(ModalRoute::as_modal_route(self))
    }

    fn overlay_entries(self: Handle<Self>, app: &App) -> Vec<Handle<waft_widgets::OverlayEntry>> {
        OverlayRoute::overlay_entries(self, app)
    }

    fn install(self: Handle<Self>, app: &mut App) {
        ModalRoute::install(self, app)
    }

    fn did_push(self: Handle<Self>, app: &mut App) -> waft_scheduler::TickerFuture {
        ModalRoute::did_push(self, app)
    }

    fn did_add(self: Handle<Self>, app: &mut App) {
        ModalRoute::did_add(self, app)
    }

    fn did_replace(self: Handle<Self>, app: &mut App, old_route: Option<waft_widgets::AnyRoute>) {
        TransitionRoute::did_replace(self, app, old_route)
    }

    fn will_pop(self: Handle<Self>, app: &mut App) -> waft_widgets::RoutePopDisposition {
        LocalHistoryRoute::will_pop(self, app)
    }

    fn pop_disposition(self: Handle<Self>, app: &mut App) -> waft_widgets::RoutePopDisposition {
        ModalRoute::pop_disposition(self, app)
    }

    fn on_pop_invoked_with_result(
        self: Handle<Self>,
        app: &mut App,
        did_pop: bool,
        result: RouteResult,
    ) {
        ModalRoute::on_pop_invoked_with_result(self, app, did_pop, result)
    }

    fn will_handle_pop_internally(self: Handle<Self>, app: &App) -> bool {
        LocalHistoryRoute::will_handle_pop_internally(self, app)
    }

    fn did_pop(self: Handle<Self>, app: &mut App, result: RouteResult) -> bool {
        LocalHistoryRoute::did_pop(self, app, result)
    }

    fn did_pop_next(self: Handle<Self>, app: &mut App, next_route: waft_widgets::AnyRoute) {
        ModalRoute::did_pop_next(self, app, next_route)
    }

    fn did_change_next(
        self: Handle<Self>,
        app: &mut App,
        next_route: Option<waft_widgets::AnyRoute>,
    ) {
        ModalRoute::did_change_next(self, app, next_route)
    }

    fn did_change_previous(
        self: Handle<Self>,
        app: &mut App,
        previous_route: Option<waft_widgets::AnyRoute>,
    ) {
        ModalRoute::did_change_previous(self, app, previous_route)
    }

    fn changed_internal_state(self: Handle<Self>, app: &mut App) {
        ModalRoute::changed_internal_state(self, app)
    }

    fn changed_external_state(self: Handle<Self>, app: &mut App) {
        ModalRoute::changed_external_state(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(animation) = app.get(self).animation {
            animation.dispose(app);
        }
        TransitionRoute::dispose(self, app)
    }
}

impl<T: Clone + PartialEq + 'static> OverlayRoute for PopupMenuRoute<T> {
    waft_widgets::modal_overlay_route_overrides!();
}

impl<T: Clone + PartialEq + 'static> PredictiveBackRoute for PopupMenuRoute<T> {
    waft_widgets::modal_predictive_back_route_overrides!(ModalRoute::pop_gesture_enabled);
}

impl<T: Clone + PartialEq + 'static> TransitionRoute for PopupMenuRoute<T> {
    waft_widgets::modal_transition_route_overrides!();

    fn transition_duration(self: Handle<Self>, app: &mut App) -> Duration {
        app.get(self)
            .pop_up_animation_style
            .as_ref()
            .and_then(|style| style.duration)
            .unwrap_or(K_MENU_DURATION)
    }

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::opaque(self, app)
    }

    fn allow_snapshotting(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::allow_snapshotting(self, app)
    }

    fn create_animation(self: Handle<Self>, app: &mut App) -> AnyAnimation<f64> {
        let style = app.get(self).pop_up_animation_style.clone();
        if style.as_ref() != Some(&AnimationStyle::NO_ANIMATION) {
            if let Some(animation) = app.get(self).animation {
                return animation.as_animation();
            }
            let parent = self.transition_route_create_animation(app);
            let curve = style
                .as_ref()
                .and_then(|style| style.curve.clone())
                .unwrap_or_else(Curves::linear);
            let reverse_curve = style
                .as_ref()
                .and_then(|style| style.reverse_curve.clone())
                .unwrap_or_else(|| {
                    Rc::new(Interval::new(
                        0.0,
                        K_MENU_CLOSE_INTERVAL_END,
                        Curves::linear(),
                    ))
                });
            let animation = CurvedAnimation::create(app, parent, curve, Some(reverse_curve));
            app.get_mut(self).animation = Some(animation);
            return animation.as_animation();
        }
        self.transition_route_create_animation(app)
    }
}

impl<T: Clone + PartialEq + 'static> LocalHistoryRoute for PopupMenuRoute<T> {
    waft_widgets::local_history_route_accessors!();
}

impl<T: Clone + PartialEq + 'static> ModalRoute for PopupMenuRoute<T> {
    waft_widgets::modal_route_accessors!();

    fn barrier_dismissible(self: Handle<Self>, _app: &App) -> bool {
        true
    }

    fn barrier_color(self: Handle<Self>, _app: &App) -> Option<Color> {
        None
    }

    fn barrier_label(self: Handle<Self>, app: &App) -> Option<String> {
        Some(app.get(self).barrier_label.clone())
    }

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::maintain_state(self, app)
    }

    fn build_page(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
        secondary_animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        let _ = (animation, secondary_animation);
        let mut selected_item_index = None;
        let route = app.get(self);
        if let Some(initial_value) = &route.initial_value {
            let mut index = 0;
            while selected_item_index.is_none() && index < route.items.len() {
                if route.items[index].represents(Some(initial_value)) {
                    selected_item_index = Some(index);
                }
                index += 1;
            }
        }
        if let Some(selected_item_index) = selected_item_index {
            self.scroll_to(app, selected_item_index);
        }

        let route = app.get(self);
        let menu = PopupMenu {
            route: self,
            item_keys: route.item_keys.clone(),
            semantic_label: route.semantic_label.clone(),
            constraints: route.constraints,
            clip_behavior: route.clip_behavior,
        }
        .into_widget();
        let position_builder = route.position_builder.clone();
        let position = route.position;
        let item_sizes = Rc::clone(&route.item_sizes);
        let captured_themes = Rc::clone(&route.captured_themes);
        let media_query = MediaQuery::of(app, context);
        let padding = media_query.padding;
        MediaQuery::remove_padding(
            app,
            context,
            true,
            true,
            true,
            true,
            LayoutBuilder::new(move |app, context, constraints| {
                let position = match &position_builder {
                    Some(position_builder) => {
                        position_builder(app, context, constraints.constraints())
                    }
                    None => position.expect("Either position or positionBuilder"),
                };
                CustomSingleChildLayout::new(Rc::new(PopupMenuRouteLayout {
                    position,
                    item_sizes: Rc::clone(&item_sizes),
                    selected_item_index,
                    text_direction: Directionality::of(app, context),
                    padding,
                }))
                .child(captured_themes.wrap(menu.clone()))
                .into_widget()
            })
            .into_widget(),
        )
        .into_widget()
    }
}

impl<T: Clone + PartialEq + 'static> PopupRoute for PopupMenuRoute<T> {}

/// A callback that returns a [`RelativeRect`] that describes the position of the popup menu.
///
/// The `constraints` argument describes the constraints of the menu's overlay, as a
/// `LayoutBuilder`'s builder receives them.
pub type PopupMenuPositionBuilder =
    Rc<dyn Fn(&mut App, BuildContext, BoxConstraints) -> RelativeRect>;

/// Show a popup menu that contains the `items` at `position`.
///
/// The `items` parameter must not be empty.
///
/// Only one of `position` or `position_builder` should be provided. Providing both throws an
/// assertion error. The `position_builder` is called whenever the menu's overlay is laid out,
/// with the overlay's constraints, and it should return the position of the menu.
///
/// If `initial_value` is specified then the first item with a matching value will be
/// highlighted and the value of `position` gives the rectangle whose vertical center will be
/// aligned with the vertical center of the highlighted item (when possible).
///
/// If `initial_value` is not specified then the top of the menu will be aligned with the top
/// of the `position` rectangle.
///
/// In both cases, the menu position will be adjusted if necessary to fit on the screen.
///
/// Horizontally, the menu is positioned so that it grows in the direction that has the most
/// room. For example, if the `position` describes a rectangle on the left edge of the screen,
/// then the left edge of the menu is aligned with the left edge of the `position`, and the
/// menu grows to the right. If both edges of the `position` are equidistant from the opposite
/// edge of the screen, then the ambient `Directionality` is used as a tie-breaker, preferring
/// to grow in the reading direction.
///
/// The positioning of the `initial_value` at the `position` is implemented by iterating over
/// the `items` to find the first whose [`PopupMenuEntry::represents`] method returns true for
/// `initial_value`, and then summing the values of [`PopupMenuEntry::height`] for all the
/// preceding widgets in the list.
///
/// The `elevation` argument specifies the z-coordinate at which to place the menu. The
/// elevation defaults to 8, the appropriate elevation for popup menus.
///
/// The `context` argument is used to look up the [`Navigator`] and [`Theme`] for the menu. It
/// is only used when the method is called. Its corresponding widget can be safely removed from
/// the tree before the popup menu is closed.
///
/// The `use_root_navigator` argument is used to determine whether to push the menu to the
/// [`Navigator`] furthest from or nearest to the given `context`. It is `false` by default.
///
/// The `semantic_label` argument is used by accessibility frameworks to announce screen
/// transitions when the menu is opened and closed. If this label is not provided, it will
/// default to `MaterialLocalizations::popup_menu_label`.
///
/// The `clip_behavior` argument is used to clip the shape of the menu. Defaults to
/// [`Clip::None`].
///
/// The `request_focus` argument specifies whether the menu should request focus when it
/// appears. If it is `None`, `Navigator::request_focus` is used instead.
///
/// The future resolves to the item's value the menu was popped with, erased as a route's
/// result is, or `None` if the menu was dismissed.
///
/// See also:
///
///  * [`PopupMenuItem`], a popup menu entry for a single value.
///  * [`PopupMenuDivider`], a popup menu entry that is just a horizontal line.
///  * [`CheckedPopupMenuItem`], a popup menu item with a checkmark.
///  * [`PopupMenuButton`], which provides an [`IconButton`] that shows a menu by calling this
///    method automatically.
///  * `SemanticsConfiguration::names_route`, for a description of edge triggered semantics.
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments; a free function has no receiver for the fluent setters"
)]
pub fn show_menu<T: Clone + PartialEq + 'static>(
    app: &mut App,
    context: BuildContext,
    position: Option<RelativeRect>,
    position_builder: Option<PopupMenuPositionBuilder>,
    items: Vec<PopupMenuEntryRef<T>>,
    initial_value: Option<T>,
    elevation: Option<f64>,
    shadow_color: Option<Color>,
    surface_tint_color: Option<Color>,
    semantic_label: Option<String>,
    shape: Option<ShapeBorderRef>,
    menu_padding: Option<EdgeInsetsGeometry>,
    color: Option<Color>,
    use_root_navigator: bool,
    constraints: Option<BoxConstraints>,
    clip_behavior: Clip,
    route_settings: Option<RouteSettingsRef>,
    pop_up_animation_style: Option<AnimationStyle>,
    request_focus: Option<bool>,
) -> CompleterFuture<RouteResult> {
    debug_assert!(!items.is_empty());
    debug_assert!(debug_check_has_material_localizations(app, context));
    debug_assert!(
        position.is_some() != position_builder.is_some(),
        "Either position or positionBuilder must be provided."
    );

    let mut semantic_label = semantic_label;
    match app.platform().target_platform() {
        TargetPlatform::IOS | TargetPlatform::MacOS => {}
        TargetPlatform::Android
        | TargetPlatform::Fuchsia
        | TargetPlatform::Linux
        | TargetPlatform::Windows => {
            if semantic_label.is_none() {
                semantic_label =
                    Some(<dyn MaterialLocalizations>::of(app, context).popup_menu_label());
            }
        }
    }

    let menu_item_keys: Vec<GlobalKey> = (0..items.len()).map(|_| GlobalKey::new()).collect();
    let navigator = Navigator::of(app, context, use_root_navigator);
    let barrier_label = <dyn MaterialLocalizations>::of(app, context).menu_dismiss_label();
    let navigator_context = navigator.context(app);
    let captured_themes = <dyn InheritedTheme>::capture(app, context, Some(navigator_context));
    let route = PopupMenuRoute::new(
        app,
        PopupMenuRouteArguments {
            position,
            position_builder,
            items,
            item_keys: menu_item_keys,
            initial_value,
            elevation,
            shadow_color,
            surface_tint_color,
            semantic_label,
            barrier_label,
            shape,
            menu_padding,
            color,
            captured_themes,
            constraints,
            clip_behavior,
            settings: route_settings,
            pop_up_animation_style,
            request_focus,
        },
    );
    navigator.push(app, Route::as_route(route))
}

/// Signature for the callback invoked when a menu item is selected. The argument is the value
/// of the [`PopupMenuItem`] that caused its menu to be dismissed.
///
/// Used by [`PopupMenuButton::on_selected`].
pub type PopupMenuItemSelected<T> = Rc<dyn Fn(&mut App, T)>;

/// Signature for the callback invoked when a [`PopupMenuButton`] is dismissed without
/// selecting an item.
///
/// Used by [`PopupMenuButton::on_canceled`].
pub type PopupMenuCanceled = Listener;

/// Signature used by [`PopupMenuButton`] to lazily construct the items shown when the button
/// is pressed.
///
/// Used by [`PopupMenuButton::item_builder`].
pub type PopupMenuItemBuilder<T> = Rc<dyn Fn(&mut App, BuildContext) -> Vec<PopupMenuEntryRef<T>>>;

// ---------------------------------------------------------------------------------------------
// PopupMenuButton

/// Displays a menu when pressed and calls [`on_selected`](Self::on_selected) when the menu is
/// dismissed because an item was selected. The value passed to
/// [`on_selected`](Self::on_selected) is the value of the selected menu item.
///
/// One of [`child`](Self::child) or [`icon`](Self::icon) may be provided, but not both. If
/// [`icon`](Self::icon) is provided, then [`PopupMenuButton`] behaves like an [`IconButton`].
///
/// If both are `None`, then a standard overflow icon is created (depending on the platform).
///
/// ## Updating to `MenuAnchor`
///
/// There is a Material 3 component, `MenuAnchor` that is preferred for applications that are
/// configured for Material 3 (see `ThemeData::use_material3`). The `MenuAnchor` widget's
/// visuals are a little bit different, see the Material 3 spec at
/// <https://m3.material.io/components/menus/guidelines> for more details.
///
/// The `MenuAnchor` widget's API is also slightly different. `MenuAnchor` can be used to wrap
/// any widget that should have a menu associated with it, and it uses `MenuItemButton` for the
/// items it shows.
///
/// See also:
///
///  * [`PopupMenuItem`], a popup menu entry for a single value.
///  * [`PopupMenuDivider`], a popup menu entry that is just a horizontal line.
///  * [`CheckedPopupMenuItem`], a popup menu item with a checkmark.
///  * [`show_menu`], a method to dynamically show a popup menu at a given location.
pub struct PopupMenuButton<T: 'static> {
    pub key: Option<KeyRef>,

    /// Called when the button is pressed to create the items to show in the menu.
    pub item_builder: PopupMenuItemBuilder<T>,

    /// The value of the menu item, if any, that should be highlighted when the menu opens.
    pub initial_value: Option<T>,

    /// Called when the popup menu is shown.
    pub on_opened: Option<Listener>,

    /// Called when the user selects a value from the popup menu created by this button.
    ///
    /// If the popup menu is dismissed without selecting a value,
    /// [`on_canceled`](Self::on_canceled) is called instead.
    pub on_selected: Option<PopupMenuItemSelected<T>>,

    /// Called when the user dismisses the popup menu without selecting an item.
    ///
    /// If the user selects a value, [`on_selected`](Self::on_selected) is called instead.
    pub on_canceled: Option<PopupMenuCanceled>,

    /// Text that describes the action that will occur when the button is pressed.
    ///
    /// This text is displayed when the user long-presses on the button and is used for
    /// accessibility.
    pub tooltip: Option<String>,

    /// The z-coordinate at which to place the menu when open. This controls the size of the
    /// shadow below the menu.
    ///
    /// Defaults to 8, the appropriate elevation for popup menus.
    pub elevation: Option<f64>,

    /// The color used to paint the shadow below the menu.
    ///
    /// If `None` then the ambient [`PopupMenuThemeData::shadow_color`] is used. If that is
    /// `None` too, then the overall theme's `ThemeData::shadow_color` (default black) is used.
    pub shadow_color: Option<Color>,

    /// The color used as an overlay on [`color`](Self::color) to indicate elevation.
    ///
    /// This is not recommended for use. [Material 3 spec](https://m3.material.io/styles/color/the-color-system/color-roles)
    /// introduced a set of tone-based surfaces and surface containers in its `ColorScheme`,
    /// which provide more flexibility. The intention is to eventually remove surface tint color
    /// from the framework.
    ///
    /// If `None`, [`PopupMenuThemeData::surface_tint_color`] is used. If that is also `None`,
    /// the default value is `Colors::TRANSPARENT`.
    ///
    /// See `Material::surface_tint_color` for more details on how this overlay is applied.
    pub surface_tint_color: Option<Color>,

    /// Matches IconButton's 8 dps padding by default. In some cases, notably where this button
    /// appears as the trailing element of a list item, it's useful to be able to set the
    /// padding to zero.
    pub padding: EdgeInsetsGeometry,

    /// If provided, menu padding is used for empty space around the outside of the popup menu.
    ///
    /// If this property is `None`, then [`PopupMenuThemeData::menu_padding`] is used. If
    /// [`PopupMenuThemeData::menu_padding`] is also `None`, then vertical padding of 8 pixels
    /// is used.
    pub menu_padding: Option<EdgeInsetsGeometry>,

    /// The splash radius.
    ///
    /// If `None`, default splash radius of [`InkWell`] or [`IconButton`] is used.
    pub splash_radius: Option<f64>,

    /// If provided, [`child`](Self::child) is the widget used for this button and the button
    /// will utilize an [`InkWell`] for taps.
    pub child: Option<WidgetRef>,

    /// The border radius for the [`InkWell`] that wraps the [`child`](Self::child).
    ///
    /// Defaults to `None`, which indicates no border radius should be applied.
    pub border_radius: Option<BorderRadius>,

    /// If provided, the [`icon`](Self::icon) is used for this button and the button will
    /// behave like an [`IconButton`].
    pub icon: Option<WidgetRef>,

    /// The offset is applied relative to the initial position set by the
    /// [`position`](Self::position).
    ///
    /// When not set, the offset defaults to [`Offset::ZERO`].
    pub offset: Offset,

    /// Whether this popup menu button is interactive.
    ///
    /// Defaults to true.
    ///
    /// If true, the button will respond to presses by displaying the menu.
    ///
    /// If false, the button is styled with the disabled color from the current [`Theme`] and
    /// will not respond to presses or show the popup menu and
    /// [`on_selected`](Self::on_selected), [`on_canceled`](Self::on_canceled) and
    /// [`item_builder`](Self::item_builder) will not be called.
    ///
    /// This can be useful in situations where the app needs to show the button, but doesn't
    /// currently have anything to show in the menu.
    pub enabled: bool,

    /// If provided, the shape used for the menu.
    ///
    /// If this property is `None`, then [`PopupMenuThemeData::shape`] is used. If
    /// [`PopupMenuThemeData::shape`] is also `None`, then the default shape for
    /// [`MaterialType::Card`] is used. This default shape is a rectangle with rounded edges of
    /// `BorderRadius::circular(2.0)`.
    pub shape: Option<ShapeBorderRef>,

    /// If provided, the background color used for the menu.
    ///
    /// If this property is `None`, then [`PopupMenuThemeData::color`] is used. If
    /// [`PopupMenuThemeData::color`] is also `None`, then `ThemeData::card_color` is used in
    /// Material 2. In Material3, defaults to `ColorScheme::surface_container`.
    pub color: Option<Color>,

    /// If provided, this color is used for the button icon.
    ///
    /// If this property is `None`, then [`PopupMenuThemeData::icon_color`] is used. If
    /// [`PopupMenuThemeData::icon_color`] is also `None` then defaults to
    /// `IconThemeData::color`.
    pub icon_color: Option<Color>,

    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a long-press will
    /// produce a short vibration, when feedback is enabled.
    ///
    /// See also:
    ///
    ///  * `Feedback` for providing platform-specific feedback to certain actions.
    pub enable_feedback: Option<bool>,

    /// If provided, the size of the [`Icon`].
    ///
    /// If this property is `None`, then `IconThemeData::size` is used. If
    /// `IconThemeData::size` is also `None`, then default size is 24.0 pixels.
    pub icon_size: Option<f64>,

    /// Optional size constraints for the menu.
    ///
    /// When unspecified, defaults to:
    ///
    /// ```text
    /// BoxConstraints::new().min_width(2.0 * 56.0).max_width(5.0 * 56.0)
    /// ```
    ///
    /// The default constraints ensure that the menu width matches maximum width recommended by
    /// the Material Design guidelines. Specifying this parameter enables creation of menu wider
    /// than the default maximum width.
    pub constraints: Option<BoxConstraints>,

    /// Whether the popup menu is positioned over or under the popup menu button.
    ///
    /// [`offset`](Self::offset) is used to change the position of the popup menu relative to
    /// the position set by this parameter.
    ///
    /// If this property is `None`, then [`PopupMenuThemeData::position`] is used. If
    /// [`PopupMenuThemeData::position`] is also `None`, then the position defaults to
    /// [`PopupMenuPosition::Over`] which makes the popup menu appear directly over the button
    /// that was used to create it.
    pub position: Option<PopupMenuPosition>,

    /// {@macro cupertino_ui.Material.clipBehavior}
    ///
    /// The [`clip_behavior`](Self::clip_behavior) argument is used the clip shape of the menu.
    ///
    /// Defaults to [`Clip::None`].
    pub clip_behavior: Clip,

    /// Used to determine whether to push the menu to the [`Navigator`] furthest from or nearest
    /// to the given `context`.
    ///
    /// Defaults to false.
    pub use_root_navigator: bool,

    /// Used to override the default animation curves and durations of the popup menu's open
    /// and close transitions.
    ///
    /// If `AnimationStyle::curve` is provided, it will be used to override the default popup
    /// animation curve. Otherwise, defaults to `Curves::linear`.
    ///
    /// If `AnimationStyle::reverse_curve` is provided, it will be used to override the default
    /// popup animation reverse curve. Otherwise, defaults to `Interval(0.0, 2.0 / 3.0)`.
    ///
    /// If `AnimationStyle::duration` is provided, it will be used to override the default popup
    /// animation duration. Otherwise, defaults to 300ms.
    ///
    /// To disable the theme animation, use [`AnimationStyle::NO_ANIMATION`].
    ///
    /// If this is `None`, then the default animation will be used.
    pub pop_up_animation_style: Option<AnimationStyle>,

    /// Optional route settings for the menu.
    ///
    /// See `RouteSettings` for details.
    pub route_settings: Option<RouteSettingsRef>,

    /// Customizes this icon button's appearance.
    ///
    /// The [`style`](Self::style) is only used for Material 3 [`IconButton`]s. If
    /// `ThemeData::use_material3` is set to true, [`style`](Self::style) is preferred for icon
    /// button customization, and any parameters defined in [`style`](Self::style) will
    /// override the same parameters in [`IconButton`].
    ///
    /// `None` by default.
    pub style: Option<ButtonStyle>,

    /// Whether to request focus when the menu appears.
    ///
    /// If `None`, `Navigator::request_focus` will be used instead.
    pub request_focus: Option<bool>,
}

macro_rules! popup_menu_button_setters {
    ($($field:ident: $ty:ty => $dart:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `PopupMenuButton(", $dart, ":)`.")]
            pub fn $field(mut self, $field: $ty) -> PopupMenuButton<T> {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl<T: Clone + PartialEq + 'static> PopupMenuButton<T> {
    /// Creates a button that shows a popup menu.
    pub fn new(
        item_builder: impl Fn(&mut App, BuildContext) -> Vec<PopupMenuEntryRef<T>> + 'static,
    ) -> PopupMenuButton<T> {
        PopupMenuButton {
            key: None,
            item_builder: Rc::new(item_builder),
            initial_value: None,
            on_opened: None,
            on_selected: None,
            on_canceled: None,
            tooltip: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            padding: EdgeInsetsGeometry::all(8.0),
            menu_padding: None,
            child: None,
            border_radius: None,
            splash_radius: None,
            icon: None,
            icon_size: None,
            offset: Offset::ZERO,
            enabled: true,
            shape: None,
            color: None,
            icon_color: None,
            enable_feedback: None,
            constraints: None,
            position: None,
            clip_behavior: Clip::None,
            use_root_navigator: false,
            pop_up_animation_style: None,
            route_settings: None,
            style: None,
            request_focus: None,
        }
    }

    popup_menu_button_setters!(
        key: KeyRef => "key",
        initial_value: T => "initialValue",
        on_opened: Listener => "onOpened",
        on_canceled: PopupMenuCanceled => "onCanceled",
        elevation: f64 => "elevation",
        shadow_color: Color => "shadowColor",
        surface_tint_color: Color => "surfaceTintColor",
        menu_padding: EdgeInsetsGeometry => "menuPadding",
        border_radius: BorderRadius => "borderRadius",
        splash_radius: f64 => "splashRadius",
        icon_size: f64 => "iconSize",
        color: Color => "color",
        icon_color: Color => "iconColor",
        enable_feedback: bool => "enableFeedback",
        constraints: BoxConstraints => "constraints",
        position: PopupMenuPosition => "position",
        pop_up_animation_style: AnimationStyle => "popUpAnimationStyle",
        route_settings: RouteSettingsRef => "routeSettings",
        style: ButtonStyle => "style",
        request_focus: bool => "requestFocus",
    );

    /// Dart `PopupMenuButton(onSelected:)`.
    pub fn on_selected(
        mut self,
        on_selected: impl Fn(&mut App, T) + 'static,
    ) -> PopupMenuButton<T> {
        self.on_selected = Some(Rc::new(on_selected));
        self
    }

    /// Dart `PopupMenuButton(tooltip:)`.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> PopupMenuButton<T> {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Dart `PopupMenuButton(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> PopupMenuButton<T> {
        self.padding = padding;
        self
    }

    /// Dart `PopupMenuButton(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> PopupMenuButton<T> {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `PopupMenuButton(icon:)`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> PopupMenuButton<T> {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Dart `PopupMenuButton(offset:)`.
    pub fn offset(mut self, offset: Offset) -> PopupMenuButton<T> {
        self.offset = offset;
        self
    }

    /// Dart `PopupMenuButton(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> PopupMenuButton<T> {
        self.enabled = enabled;
        self
    }

    /// Dart `PopupMenuButton(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> PopupMenuButton<T> {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `PopupMenuButton(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> PopupMenuButton<T> {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `PopupMenuButton(useRootNavigator:)`.
    pub fn use_root_navigator(mut self, use_root_navigator: bool) -> PopupMenuButton<T> {
        self.use_root_navigator = use_root_navigator;
        self
    }
}

impl<T: 'static> Debug for PopupMenuButton<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PopupMenuButton")
            .field("tooltip", &self.tooltip)
            .field("enabled", &self.enabled)
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for PopupMenuButton<T> {
    type State = PopupMenuButtonState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(
            !(self.child.is_some() && self.icon.is_some()),
            "You can only pass [child] or [icon], not both."
        );
    }

    fn create_state(&self) -> PopupMenuButtonState<T> {
        PopupMenuButtonState {
            state: StateData::new(),
            is_menu_expanded: false,
            last_position: Rc::default(),
            popup_menu_theme: None,
            cached_button_render_box: None,
            cached_overlay_render_box: None,
        }
    }
}

/// The [`State`] for a [`PopupMenuButton`].
///
/// See [`show_button_menu`](Self::show_button_menu) for a way to programmatically open the
/// popup menu of your button state.
pub struct PopupMenuButtonState<T: Clone + PartialEq + 'static> {
    state: StateData<PopupMenuButton<T>>,
    is_menu_expanded: bool,
    /// Dart's `_lastPosition`, shared with the position builder of the menu this button
    /// shows, which reads it after the state is disposed, as Dart's menu keeps the state
    /// alive.
    last_position: Rc<Cell<Option<RelativeRect>>>,
    /// Dart's `late PopupMenuThemeData _popupMenuTheme`.
    popup_menu_theme: Option<PopupMenuThemeData>,
    cached_button_render_box: Option<AnyRenderBox>,
    cached_overlay_render_box: Option<AnyRenderBox>,
}

impl<T: Clone + PartialEq + 'static> Debug for PopupMenuButtonState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PopupMenuButtonState")
            .field("is_menu_expanded", &self.is_menu_expanded)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> PopupMenuButtonState<T> {
    fn update_cached_objects(self: Handle<Self>, app: &mut App) {
        if self.mounted(app) {
            let context = self.context(app);
            let popup_menu_theme = PopupMenuTheme::of(app, context);
            app.get_mut(self).popup_menu_theme = Some(popup_menu_theme);

            let button_render_object = context.find_render_object(app);
            if let Some(button) = button_render_object.and_then(|object| object.as_box()) {
                app.get_mut(self).cached_button_render_box = Some(button);
            }
            let use_root_navigator = self.widget(app).use_root_navigator;
            match Navigator::maybe_of(app, context, use_root_navigator) {
                Some(navigator) => {
                    let overlay_render_object = navigator
                        .overlay(app)
                        .and_then(|overlay| overlay.context(app).find_render_object(app));
                    if let Some(overlay) = overlay_render_object.and_then(|object| object.as_box())
                    {
                        app.get_mut(self).cached_overlay_render_box = Some(overlay);
                    }
                }
                None => {
                    let state = app.get_mut(self);
                    state.cached_button_render_box = None;
                    state.cached_overlay_render_box = None;
                }
            }
        }
    }

    fn get_default_position(
        last_position: &Cell<Option<RelativeRect>>,
        constraints: BoxConstraints,
    ) -> RelativeRect {
        last_position
            .get()
            .unwrap_or_else(|| RelativeRect::from_size(Rect::ZERO, constraints.biggest()))
    }

    /// Dart's `_positionBuilder`; `last_position` is the state's `last_position`, which it
    /// reads after the state is gone.
    fn position_builder(
        self: Handle<Self>,
        last_position: &Cell<Option<RelativeRect>>,
        app: &mut App,
        _context: BuildContext,
        constraints: BoxConstraints,
    ) -> RelativeRect {
        if !self.mounted(app) {
            // When the route is displayed, the `_positionBuilder` closure is stored.
            // Even after the button has been unmounted and the context becomes invalid,
            // the route might keep displaying, and `_positionBuilder` must continue to
            // work in that case.
            return Self::get_default_position(last_position, constraints);
        }

        let popup_menu_theme = app
            .get(self)
            .popup_menu_theme
            .clone()
            .expect("set in did_change_dependencies");

        // Use cached render objects if available.
        let button = app.get(self).cached_button_render_box;
        let overlay = app.get(self).cached_overlay_render_box;

        // Check if cached render objects are available and still attached.
        let (Some(button), Some(overlay)) = (button, overlay) else {
            // Render objects are not available or detached, return cached position or default.
            return Self::get_default_position(last_position, constraints);
        };
        if !button.as_object().attached(app) || !overlay.as_object().attached(app) {
            return Self::get_default_position(last_position, constraints);
        }
        let widget = self.widget(app);
        let popup_menu_position = widget
            .position
            .or(popup_menu_theme.position)
            .unwrap_or(PopupMenuPosition::Over);
        let offset = match popup_menu_position {
            PopupMenuPosition::Over => widget.offset,
            PopupMenuPosition::Under => {
                let mut offset = Offset::new(0.0, button.size(app).height()) + widget.offset;
                if widget.child.is_none() {
                    // Remove the padding of the icon button.
                    offset = offset - Offset::new(0.0, widget.padding.vertical() / 2.0);
                }
                offset
            }
        };
        let button_size = button.size(app);
        let position = RelativeRect::from_rect(
            Rect::from_points(
                button.local_to_global(app, offset, Some(overlay.as_object())),
                button.local_to_global(
                    app,
                    button_size.bottom_right(Offset::ZERO) + offset,
                    Some(overlay.as_object()),
                ),
            ),
            Offset::ZERO & overlay.size(app),
        );

        last_position.set(Some(position));
        position
    }

    /// A method to show a popup menu with the items supplied to
    /// [`PopupMenuButton::item_builder`] at the position of your [`PopupMenuButton`].
    ///
    /// By default, it is called when the user taps the button and
    /// [`PopupMenuButton::enabled`] is set to `true`. Moreover, you can open the button by
    /// calling the method manually.
    ///
    /// You would access your [`PopupMenuButtonState`] using a `GlobalKey` and show the menu of
    /// the button with `global_key.current_state::<PopupMenuButtonState<T>>(app)
    /// .unwrap().show_button_menu(app)`.
    pub fn show_button_menu(self: Handle<Self>, app: &mut App) {
        // Ensure cached render objects are initialized
        self.update_cached_objects(app);
        let context = self.context(app);
        let item_builder = Rc::clone(&self.widget(app).item_builder);
        let items = item_builder(app, context);
        // Only show the menu if there is something to show
        if !items.is_empty() {
            if let Some(on_opened) = self.widget(app).on_opened.clone() {
                on_opened.call(app);
            }
            self.set_state(app, |state| {
                state.is_menu_expanded = true;
            });
            let widget = self.widget(app);
            let initial_value = widget.initial_value.clone();
            let elevation = widget.elevation;
            let shadow_color = widget.shadow_color;
            let surface_tint_color = widget.surface_tint_color;
            let shape = widget.shape.clone();
            let menu_padding = widget.menu_padding;
            let color = widget.color;
            let constraints = widget.constraints;
            let clip_behavior = widget.clip_behavior;
            let use_root_navigator = widget.use_root_navigator;
            let pop_up_animation_style = widget.pop_up_animation_style.clone();
            let route_settings = widget.route_settings.clone();
            let request_focus = widget.request_focus;
            let last_position = Rc::clone(&app.get(self).last_position);
            let popped = show_menu(
                app,
                context,
                None,
                Some(Rc::new(move |app, context, constraints| {
                    self.position_builder(&last_position, app, context, constraints)
                })),
                items,
                initial_value,
                elevation,
                shadow_color,
                surface_tint_color,
                None,
                shape,
                menu_padding,
                color,
                use_root_navigator,
                constraints,
                clip_behavior,
                route_settings,
                pop_up_animation_style,
                request_focus,
            );
            drop(app.spawn(async move |cx| {
                let new_value = popped.await;
                cx.update(|app| {
                    if !self.mounted(app) {
                        return;
                    }
                    self.set_state(app, |state| {
                        state.is_menu_expanded = false;
                    });
                    let new_value =
                        new_value.and_then(|new_value| new_value.downcast_ref::<T>().cloned());
                    let Some(new_value) = new_value else {
                        if let Some(on_canceled) = self.widget(app).on_canceled.clone() {
                            on_canceled.call(app);
                        }
                        return;
                    };
                    if let Some(on_selected) = self.widget(app).on_selected.clone() {
                        on_selected(app, new_value);
                    }
                });
            }));
        }
    }

    fn can_request_focus(self: Handle<Self>, app: &mut App) -> bool {
        let context = self.context(app);
        let mode = MediaQuery::maybe_navigation_mode_of(app, context)
            .unwrap_or(NavigationMode::Traditional);
        match mode {
            NavigationMode::Traditional => self.widget(app).enabled,
            NavigationMode::Directional => true,
        }
    }
}

impl<T: Clone + PartialEq + 'static> State for PopupMenuButtonState<T> {
    type Widget = PopupMenuButton<T>;
    waft_widgets::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.update_cached_objects(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let icon_theme = IconTheme::of(app, context);
        let popup_menu_theme = PopupMenuTheme::of(app, context);
        let enable_feedback = self
            .widget(app)
            .enable_feedback
            .or(PopupMenuTheme::of(app, context).enable_feedback)
            .unwrap_or(true);

        debug_assert!(debug_check_has_material_localizations(app, context));
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let can_request_focus = self.can_request_focus(app);
        let is_menu_expanded = app.get(self).is_menu_expanded;
        let widget = self.widget(app);
        let tooltip = widget
            .tooltip
            .clone()
            .unwrap_or_else(|| localizations.show_menu_tooltip());
        let on_pressed = widget
            .enabled
            .then(|| Listener::handle_method(self, Self::show_button_menu));

        if let Some(child) = &widget.child {
            let mut ink_well = InkWell::new()
                .can_request_focus(can_request_focus)
                .enable_feedback(enable_feedback)
                .child(child.clone());
            if let Some(border_radius) = widget.border_radius {
                ink_well = ink_well.border_radius(border_radius);
            }
            if let Some(on_pressed) = on_pressed {
                ink_well = ink_well.on_tap(on_pressed);
            }
            if let Some(splash_radius) = widget.splash_radius {
                ink_well = ink_well.radius(splash_radius);
            }
            let child = Tooltip::new()
                .message(tooltip)
                .child(ink_well)
                .into_widget();
            let tap_target_size = widget
                .style
                .as_ref()
                .and_then(|style| style.tap_target_size)
                .unwrap_or(MaterialTapTargetSize::ShrinkWrap);
            let result = if tap_target_size == MaterialTapTargetSize::Padded {
                ConstrainedBox::new(
                    BoxConstraints::new()
                        .min_width(K_MIN_INTERACTIVE_DIMENSION)
                        .min_height(K_MIN_INTERACTIVE_DIMENSION),
                )
                .child(child)
                .into_widget()
            } else {
                child
            };
            // The button semantics are added here rather than by the [InkWell] so
            // that assistive technologies describe the popup menu button the same way
            // regardless of whether it is built from [child] or from [icon], in which
            // case the semantics come from the [IconButton].
            return Semantics::new()
                .button(true)
                .enabled(widget.enabled)
                .expanded(is_menu_expanded)
                .child(result)
                .into_widget();
        }

        let icon = widget
            .icon
            .clone()
            .unwrap_or_else(|| Icon::new(Some(Icons::adaptive().more(app))).into_widget());
        let mut icon_button = IconButton::new(
            on_pressed,
            Semantics::new().expanded(is_menu_expanded).child(icon),
        )
        .key(StandardComponentType::MoreButton.key())
        .padding(widget.padding)
        .tooltip(tooltip)
        .enable_feedback(enable_feedback);
        if let Some(splash_radius) = widget.splash_radius {
            icon_button = icon_button.splash_radius(splash_radius);
        }
        if let Some(icon_size) = widget
            .icon_size
            .or(popup_menu_theme.icon_size)
            .or(icon_theme.size)
        {
            icon_button = icon_button.icon_size(icon_size);
        }
        if let Some(color) = widget
            .icon_color
            .or(popup_menu_theme.icon_color)
            .or(icon_theme.color.as_deref().copied())
        {
            icon_button = icon_button.color(color);
        }
        if let Some(style) = widget.style.clone() {
            icon_button = icon_button.style(style);
        }
        Semantics::new().child(icon_button).into_widget()
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/popup_menu_test.dart`; the semantics-tree cases wait for the semantics
    //! tree, the goldens are not ported, and the display-feature case waits for
    //! `DisplayFeatureSubScreen`.

    use std::cell::{Cell, RefCell};

    use waft_embedder::Locale;
    use waft_widgets::{
        AnyElement, Builder, Center, Column, Container, DefaultWidgetsLocalizations, Localizations,
        MediaQueryData, RouteSettings, SizedBox, Text,
    };

    use super::*;
    use crate::material_localizations::DefaultMaterialLocalizations;
    use crate::page::MaterialPageRoute;
    use crate::test_support::{WidgetTester, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3, pump_app, tester, tester_on};

    /// Dart's `PopupMenuItem<T>(value: value, child: Text(text))`.
    fn item<T: Clone + PartialEq + 'static>(value: T, text: &str) -> PopupMenuEntryRef<T> {
        PopupMenuEntryRef::new(PopupMenuItem::new(Some(Text::new(text))).value(value))
    }

    fn only(found: Vec<AnyElement>) -> AnyElement {
        assert_eq!(found.len(), 1, "one widget found");
        found[0]
    }

    /// Dart's `tester.getRect(find.text(text))`.
    fn text_rect(tester: &WidgetTester, text: &str) -> Rect {
        tester.get_rect(tester.render_box_of(only(tester.find_onstage_text(text))))
    }

    /// Dart's `find.text(text)` with `findsOneWidget` or `findsNothing`.
    fn shows(tester: &WidgetTester, text: &str) -> bool {
        !tester.find_onstage_text(text).is_empty()
    }

    /// Dart's `tester.tap(find.text(text))`.
    fn tap_text(tester: &mut WidgetTester, text: &str) {
        let center = text_rect(tester, text).center();
        tester.tap(center, 1);
    }

    /// Dart's `tester.tap(find.byKey(key))`.
    fn tap_key(tester: &mut WidgetTester, key: &GlobalKey) {
        let center = tester.center_of(key);
        tester.tap(center, 1);
    }

    /// Dart's `tester.getRect(find.byType(T))` of the only one.
    fn rect_of<W: 'static>(tester: &WidgetTester) -> Rect {
        tester.get_rect(tester.render_box_of(only(tester.find_onstage_by_type::<W>())))
    }

    /// Dart's `tester.getRect(find.byKey(key))`.
    fn rect_of_key(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        tester.get_rect(tester.render_object(key).as_box().expect("a render box"))
    }

    fn context_of(tester: &WidgetTester, key: &GlobalKey) -> BuildContext {
        key.current_context(&tester.cell.borrow())
            .expect("the keyed widget is mounted")
    }

    /// Dart's `Navigator.of(context).pop()`.
    fn pop(tester: &mut WidgetTester, context: BuildContext) {
        let mut app = tester.cell.borrow_mut();
        Navigator::of(&mut app, context, false).pop(&mut app, None);
        drop(app);
        tester.cell.checkpoint();
    }

    /// A counter a callback bumps: Dart's `var opens = 0; () => opens++`.
    fn counter() -> (Rc<Cell<u32>>, Listener) {
        let count = Rc::new(Cell::new(0));
        let bump = Rc::clone(&count);
        (count, Listener::new(move |_app| bump.set(bump.get() + 1)))
    }

    /// Dart's `TestApp(textDirection: text_direction, child: child)`: a navigator whose one
    /// page is `Material(child: child)`, under the view's media query.
    fn test_app(
        text_direction: TextDirection,
        child: impl Fn() -> WidgetRef + 'static,
    ) -> WidgetRef {
        let child = Rc::new(child);
        Localizations::new(
            Locale::new("en").country_code("US"),
            vec![
                DefaultWidgetsLocalizations::delegate(),
                DefaultMaterialLocalizations::delegate(),
            ],
        )
        .child(Builder::new(move |app, context| {
            let view = waft_widgets::View::of(app, context);
            let data = MediaQueryData::from_view(app, &*view, None);
            let child = Rc::clone(&child);
            MediaQuery::new(
                data,
                Directionality::new(
                    text_direction,
                    Navigator::new().on_generate_route(move |app, settings| {
                        let child = Rc::clone(&child);
                        let route = MaterialPageRoute::new(
                            app,
                            Rc::new(move |_app, _context| {
                                Material::new().child(child()).into_widget()
                            }),
                        );
                        route.settings(app, RouteSettingsRef::Settings(settings.clone()));
                        Some(Route::as_route(route))
                    }),
                ),
            )
            .into_widget()
        }))
        .into_widget()
    }

    #[test]
    fn navigator_push_works_within_a_popup_menu_button() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let target_key = GlobalKey::new();
            let key = target_key.clone();
            let root = {
                let app = tester.cell.borrow();
                crate::app::MaterialApp::new()
                    .theme(m3(&app))
                    .routes([(
                        "/next".to_owned(),
                        Rc::new(|_app: &mut App, _context: BuildContext| {
                            Text::new("Next").into_widget()
                        }) as waft_widgets::WidgetBuilder,
                    )])
                    .home(
                        Material::new()
                            .child(
                                Center::new().child(
                                    Builder::new(move |_app, context| {
                                        PopupMenuButton::new(|_app, _context| vec![item(1, "One")])
                                            .on_selected(move |app, _value: i32| {
                                                drop(Navigator::push_named(
                                                    app, context, "/next", None,
                                                ));
                                            })
                                            .into_widget()
                                    })
                                    .key(key.clone().into()),
                                ),
                            )
                            .into_widget(),
                    )
                    .into_widget()
            };
            tester.pump_widget(root);

            tap_key(&mut tester, &target_key);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1)); // finish the menu animation

            assert!(shows(&tester, "One"));
            assert!(!shows(&tester, "Next"));

            tap_text(&mut tester, "One");
            tester.pump(Duration::ZERO); // return the future
            tester.pump(Duration::ZERO); // start the navigation
            tester.pump(Duration::from_secs(1)); // end the navigation

            assert!(!shows(&tester, "One"));
            assert!(shows(&tester, "Next"));
        });
    }

    #[test]
    fn popup_menu_button_calls_on_opened_callback_when_the_menu_is_opened() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (opens, on_opened) = counter();
            let popup_context = Rc::new(Cell::new(None));
            let no_items_key = GlobalKey::new();
            let no_callback_key = GlobalKey::new();
            let with_callback_key = GlobalKey::new();
            let (a, b, c) = (
                no_items_key.clone(),
                no_callback_key.clone(),
                with_callback_key.clone(),
            );
            let saved = Rc::clone(&popup_context);
            pump_app(&mut tester, m3, move |_app| {
                Material::new()
                    .child(Column::new().children(vec![
                        PopupMenuButton::<i32>::new(|_app, _context| Vec::new())
                            .key(a.into())
                            .on_opened(on_opened.clone())
                            .into_widget(),
                        PopupMenuButton::new(move |_app, context| {
                            saved.set(Some(context));
                            vec![item(1, "Tap me please!")]
                        })
                        .key(b.into())
                        .into_widget(),
                        PopupMenuButton::new(|_app, _context| vec![item(1, "Tap me, too!")])
                            .key(c.into())
                            .on_opened(on_opened)
                            .into_widget(),
                    ]))
                    .into_widget()
            });

            // Make sure callback is not called when the menu is not shown
            tap_key(&mut tester, &no_items_key);
            tester.pump(Duration::ZERO);
            assert_eq!(opens.get(), 0);

            // Make sure everything works if no callback is provided
            tap_key(&mut tester, &no_callback_key);
            tester.pump(Duration::ZERO);
            assert_eq!(opens.get(), 0);

            // Close the opened menu
            pop(
                &mut tester,
                popup_context.get().expect("the item builder ran"),
            );
            tester.pump(Duration::ZERO);

            // Make sure callback is called when the button is tapped
            tap_key(&mut tester, &with_callback_key);
            tester.pump(Duration::ZERO);
            assert_eq!(opens.get(), 1);
        });
    }

    #[test]
    fn popup_menu_button_calls_on_canceled_callback_when_an_item_is_not_selected() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (cancels, on_canceled) = counter();
            let popup_context = Rc::new(Cell::new(None));
            let no_callback_key = GlobalKey::new();
            let with_callback_key = GlobalKey::new();
            let (a, b) = (no_callback_key.clone(), with_callback_key.clone());
            let saved = Rc::clone(&popup_context);
            pump_app(&mut tester, m3, move |_app| {
                Material::new()
                    .child(Column::new().children(vec![
                        PopupMenuButton::new(|_app, _context| vec![item(1, "Tap me please!")])
                            .key(a.into())
                            .into_widget(),
                        PopupMenuButton::new(move |_app, context| {
                            saved.set(Some(context));
                            vec![item(1, "Tap me, too!")]
                        })
                        .key(b.into())
                        .on_canceled(on_canceled)
                        .into_widget(),
                    ]))
                    .into_widget()
            });

            // Make sure everything works if no callback is provided
            tap_key(&mut tester, &no_callback_key);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));
            tester.tap(Offset::ZERO, 1);
            tester.pump(Duration::ZERO);
            assert_eq!(cancels.get(), 0);

            // Make sure callback is called when a non-selection tap occurs
            tap_key(&mut tester, &with_callback_key);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));
            tester.tap(Offset::ZERO, 1);
            tester.pump(Duration::ZERO);
            assert_eq!(cancels.get(), 1);

            // Make sure callback is called when back navigation occurs
            tap_key(&mut tester, &with_callback_key);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));
            pop(
                &mut tester,
                popup_context.get().expect("the item builder ran"),
            );
            tester.pump(Duration::ZERO);
            assert_eq!(cancels.get(), 2);
        });
    }

    /// Dart's `find.widgetWithText(W, text)`: the `W` nearest above the `Text` showing `text`.
    fn widget_with_text<W: 'static>(tester: &WidgetTester, text: &str) -> AnyElement {
        tester.ancestor_of_type::<W>(only(tester.find_onstage_text(text)))
    }

    /// Dart's `tester.getRect(find.widgetWithText(W, text))`.
    fn rect_with_text<W: 'static>(tester: &WidgetTester, text: &str) -> Rect {
        tester.get_rect(tester.render_box_of(widget_with_text::<W>(tester, text)))
    }

    /// Dart's `expect(actual, expected)` on positions that went through a render object's
    /// transform: `Matrix4` is valo's f32 matrix, so they carry f32 rounding that Dart's f64
    /// has not, and nothing more.
    fn assert_near(actual: f64, expected: f64) {
        assert!(near(actual, expected), "{actual} is not {expected}");
    }

    fn near(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() <= f64::from(f32::EPSILON) * expected.abs().max(1.0)
    }

    fn assert_rect_near(actual: Rect, expected: Rect) {
        for (actual, expected) in [
            (actual.left, expected.left),
            (actual.top, expected.top),
            (actual.right, expected.right),
            (actual.bottom, expected.bottom),
        ] {
            assert_near(actual, expected);
        }
    }

    fn assert_offset_near(actual: Offset, expected: Offset) {
        assert_near(actual.dx(), expected.dx());
        assert_near(actual.dy(), expected.dy());
    }

    /// Dart's `MaterialApp(home: Scaffold(body: Center(child: button)))`.
    fn centered_in_scaffold(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        button: impl FnOnce() -> WidgetRef,
    ) {
        let body = Center::new().child(button()).into_widget();
        pump_app(tester, theme, move |_app| {
            crate::scaffold::Scaffold::new().body(body).into_widget()
        });
    }

    fn has_scheduled_frame(tester: &WidgetTester) -> bool {
        SchedulerBinding::has_scheduled_frame(&mut tester.cell.borrow_mut())
    }

    #[test]
    fn disabled_popup_menu_button_will_not_call_item_builder_on_opened_on_selected_or_on_canceled()
    {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let popup_button_key = GlobalKey::new();
            let item_builder_called = Rc::new(Cell::new(false));
            let on_opened_called = Rc::new(Cell::new(false));
            let on_selected_called = Rc::new(Cell::new(false));
            let on_canceled_called = Rc::new(Cell::new(false));

            let build_app = |tester: &mut WidgetTester| {
                let key = popup_button_key.clone();
                let (builder_called, opened, selected, canceled) = (
                    Rc::clone(&item_builder_called),
                    Rc::clone(&on_opened_called),
                    Rc::clone(&on_selected_called),
                    Rc::clone(&on_canceled_called),
                );
                pump_app(tester, m3, move |_app| {
                    Builder::new(move |app, context| {
                        let (builder_called, opened, selected, canceled) = (
                            Rc::clone(&builder_called),
                            Rc::clone(&opened),
                            Rc::clone(&selected),
                            Rc::clone(&canceled),
                        );
                        MediaQuery::new(
                            MediaQuery::of(app, context)
                                .copy_with()
                                .navigation_mode(NavigationMode::Directional),
                            Material::new().child(Column::new().children(vec![
                                PopupMenuButton::new(move |_app, _context| {
                                    builder_called.set(true);
                                    vec![item(1, "Tap me please!")]
                                })
                                .enabled(false)
                                .child(Text::new("Tap Me").key(key.clone().into()))
                                .on_opened(Listener::new(move |_app| opened.set(true)))
                                .on_selected(move |_app, _selected: i32| selected.set(true))
                                .on_canceled(Listener::new(move |_app| canceled.set(true)))
                                .into_widget(),
                            ])),
                        )
                        .into_widget()
                    })
                    .into_widget()
                });
            };

            build_app(&mut tester);

            for directional in [false, true] {
                if directional {
                    // Test again, with directional navigation mode and after focusing the
                    // button.
                    build_app(&mut tester);
                    let context = context_of(&tester, &popup_button_key);
                    let mut app = tester.cell.borrow_mut();
                    let node = waft_widgets::Focus::of(&mut app, context, false, false);
                    node.request_focus(&mut app, None);
                    drop(app);
                    tester.pump_and_settle();
                }

                // Try to bring up the popup menu and select the first item from it
                tap_key(&mut tester, &popup_button_key);
                tester.pump_and_settle();
                tap_key(&mut tester, &popup_button_key);
                tester.pump_and_settle();
                assert!(!item_builder_called.get());
                assert!(!on_opened_called.get());
                assert!(!on_selected_called.get());

                // Try to bring up the popup menu and tap outside it to cancel the menu
                tap_key(&mut tester, &popup_button_key);
                tester.pump_and_settle();
                tester.tap(Offset::ZERO, 1);
                tester.pump_and_settle();
                assert!(!item_builder_called.get());
                assert!(!on_opened_called.get());
                assert!(!on_canceled_called.get());
            }
        });
    }

    /// Dart's 'disabled PopupMenuButton is not focusable' and 'Disabled PopupMenuButton is
    /// focusable with directional navigation'.
    fn disabled_button_takes_focus(navigation_mode: Option<NavigationMode>) -> bool {
        let mut tester = tester();
        let child_key = GlobalKey::new();
        let key = child_key.clone();
        pump_app(&mut tester, m3, move |_app| {
            Builder::new(move |app, context| {
                let button = Material::new().child(Column::new().children(vec![
                    PopupMenuButton::new(|_app, _context| vec![item(1, "Tap me please!")])
                        .enabled(false)
                        .child(Container::new().key(key.clone().into()))
                        .on_selected(|_app, _selected: i32| {})
                        .into_widget(),
                ]));
                match navigation_mode {
                    Some(navigation_mode) => MediaQuery::new(
                        MediaQuery::of(app, context)
                            .copy_with()
                            .navigation_mode(navigation_mode),
                        button,
                    )
                    .into_widget(),
                    None => button.into_widget(),
                }
            })
            .into_widget()
        });
        let context = context_of(&tester, &child_key);
        let node = {
            let mut app = tester.cell.borrow_mut();
            let node = waft_widgets::Focus::of(&mut app, context, false, false);
            node.request_focus(&mut app, None);
            node
        };
        tester.pump(Duration::ZERO);
        node.has_primary_focus(&tester.cell.borrow())
    }

    #[test]
    fn disabled_popup_menu_button_is_not_focusable() {
        with_main_thread_stack(|| assert!(!disabled_button_takes_focus(None)));
    }

    #[test]
    fn disabled_popup_menu_button_is_focusable_with_directional_navigation() {
        with_main_thread_stack(|| {
            assert!(disabled_button_takes_focus(Some(
                NavigationMode::Directional
            )));
        });
    }

    #[test]
    fn popup_menu_item_on_tap_callback_is_called_when_defined() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let counters = Rc::new(RefCell::new([0, 0]));
            let taps = Rc::clone(&counters);
            tester.pump_widget(test_app(TextDirection::Ltr, move || {
                let taps = Rc::clone(&taps);
                Material::new()
                    .child(
                        PopupMenuButton::<()>::new(move |_app, _context| {
                            let (first, second) = (Rc::clone(&taps), Rc::clone(&taps));
                            vec![
                                PopupMenuEntryRef::new(
                                    PopupMenuItem::new(Some(Text::new("First option"))).on_tap(
                                        Listener::new(move |_app| first.borrow_mut()[0] += 1),
                                    ),
                                ),
                                PopupMenuEntryRef::new(
                                    PopupMenuItem::new(Some(Text::new("Second option"))).on_tap(
                                        Listener::new(move |_app| second.borrow_mut()[1] += 1),
                                    ),
                                ),
                                PopupMenuEntryRef::new(PopupMenuItem::new(Some(Text::new(
                                    "Option without onTap",
                                )))),
                            ]
                        })
                        .child(Text::new("Actions")),
                    )
                    .into_widget()
            }));

            for (option, expected) in [
                ("First option", [1, 0]),
                ("First option", [2, 0]),
                ("Second option", [2, 1]),
                ("Option without onTap", [2, 1]),
            ] {
                tap_text(&mut tester, "Actions");
                tester.pump_and_settle();
                tap_text(&mut tester, option);
                tester.pump_and_settle();
                assert_eq!(*counters.borrow(), expected);
            }
        });
    }

    #[test]
    fn popup_menu_item_can_have_both_on_tap_and_value() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let counters = Rc::new(RefCell::new([0, 0]));
            let selected: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
            let (taps, chosen) = (Rc::clone(&counters), Rc::clone(&selected));
            tester.pump_widget(test_app(TextDirection::Ltr, move || {
                let (taps, chosen) = (Rc::clone(&taps), Rc::clone(&chosen));
                Material::new()
                    .child(
                        PopupMenuButton::new(move |_app, _context| {
                            let (first, second) = (Rc::clone(&taps), Rc::clone(&taps));
                            vec![
                                PopupMenuEntryRef::new(
                                    PopupMenuItem::new(Some(Text::new("First option")))
                                        .value("first".to_owned())
                                        .on_tap(Listener::new(move |_app| {
                                            first.borrow_mut()[0] += 1
                                        })),
                                ),
                                PopupMenuEntryRef::new(
                                    PopupMenuItem::new(Some(Text::new("Second option")))
                                        .value("second".to_owned())
                                        .on_tap(Listener::new(move |_app| {
                                            second.borrow_mut()[1] += 1
                                        })),
                                ),
                                item("third".to_owned(), "Option without onTap"),
                            ]
                        })
                        .on_selected(move |_app, value: String| *chosen.borrow_mut() = Some(value))
                        .child(Text::new("Actions")),
                    )
                    .into_widget()
            }));

            for (option, expected, value) in [
                ("First option", [1, 0], "first"),
                ("First option", [2, 0], "first"),
                ("Second option", [2, 1], "second"),
                ("Option without onTap", [2, 1], "third"),
            ] {
                tap_text(&mut tester, "Actions");
                tester.pump_and_settle();
                tap_text(&mut tester, option);
                tester.pump_and_settle();
                assert_eq!(*counters.borrow(), expected);
                assert_eq!(selected.borrow().as_deref(), Some(value));
            }
        });
    }

    #[test]
    fn popup_menu_item_is_only_focusable_when_enabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let popup_button_key = GlobalKey::new();
            let child_key = GlobalKey::new();
            let item_builder_called = Rc::new(Cell::new(false));
            let build = |tester: &mut WidgetTester, enabled: bool| {
                let (key, child_key, called) = (
                    popup_button_key.clone(),
                    child_key.clone(),
                    Rc::clone(&item_builder_called),
                );
                pump_app(tester, m3, move |_app| {
                    Material::new()
                        .child(Column::new().children(vec![
                            PopupMenuButton::new(move |_app, _context| {
                                called.set(true);
                                vec![PopupMenuEntryRef::new(
                                    PopupMenuItem::new(Some(
                                        Text::new("Tap me please!").key(child_key.clone().into()),
                                    ))
                                    .enabled(enabled)
                                    .value(1),
                                )]
                            })
                            .key(key.into())
                            .into_widget(),
                        ]))
                        .into_widget()
                });
            };
            let child_node = |tester: &WidgetTester| {
                let context = context_of(tester, &child_key);
                waft_widgets::Focus::of(&mut tester.cell.borrow_mut(), context, false, false)
            };

            build(&mut tester, true);
            // Open the popup to build and show the menu contents.
            tap_key(&mut tester, &popup_button_key);
            tester.pump_and_settle();
            let node = child_node(&tester);
            // Now that the contents are shown, request focus on the child text.
            node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert!(item_builder_called.get());

            // Make sure that the focus went where we expected it to.
            assert!(node.has_primary_focus(&tester.cell.borrow()));
            item_builder_called.set(false);

            // Close the popup.
            tap_key(&mut tester, &popup_button_key);
            tester.pump_and_settle();

            build(&mut tester, false);
            tester.pump_and_settle();
            // Open the popup again to rebuild the contents with enabled == false.
            tap_key(&mut tester, &popup_button_key);
            tester.pump_and_settle();

            assert!(item_builder_called.get());
            let node = child_node(&tester);
            assert!(!node.has_primary_focus(&tester.cell.borrow()));
        });
    }

    /// Dart's `testPositioningDown` and `testPositioningDownThenUp` of 'PopupMenu
    /// positioning'.
    fn test_positioning(
        tester: &mut WidgetTester,
        text_direction: TextDirection,
        alignment: AlignmentGeometry,
        growth_direction: TextDirection,
        start_rect: Rect,
        then_up: bool,
    ) {
        // reset in case we had a menu up already
        tester.pump_widget(Container::new().into_widget());
        tester.pump_widget(test_app(text_direction, move || {
            Align::new()
                .alignment(alignment)
                .child(
                    PopupMenuButton::new(|_app, _context| {
                        vec![item(1, "AAA"), item(2, "BBB"), item(3, "CCC")]
                    })
                    .child(
                        SizedBox::new()
                            .height(100.0)
                            .width(100.0)
                            .child(Text::new("XXX")),
                    ),
                )
                .into_widget()
        }));
        tap_text(tester, "XXX");
        tester.pump(Duration::ZERO);

        let mut rect = rect_of::<PopupMenu<i32>>(tester);
        assert_rect_near(rect, start_rect);
        let mut done_vertically = false;
        let mut vertical_stage = 0; // 0=down, 1=up, 2=done
        let mut done_horizontally = false;
        loop {
            tester.pump(Duration::from_millis(20));
            let new_rect = rect_of::<PopupMenu<i32>>(tester);
            if then_up {
                match vertical_stage {
                    0 if new_rect.top < rect.top => {
                        vertical_stage = 1;
                        assert!(new_rect.bottom >= rect.bottom);
                    }
                    0 => {
                        assert_near(new_rect.top, rect.top);
                        assert!(new_rect.bottom > rect.bottom);
                    }
                    1 if near(new_rect.top, rect.top) => {
                        vertical_stage = 2;
                        assert_near(new_rect.bottom, rect.bottom);
                    }
                    1 => {
                        assert!(new_rect.top < rect.top);
                        assert_near(new_rect.bottom, rect.bottom);
                    }
                    _ => {
                        assert_near(new_rect.bottom, rect.bottom);
                        assert_near(new_rect.top, rect.top);
                    }
                }
            } else {
                assert_near(new_rect.top, rect.top);
                if done_vertically {
                    assert_near(new_rect.bottom, rect.bottom);
                } else if near(new_rect.bottom, rect.bottom) {
                    done_vertically = true;
                } else {
                    assert!(new_rect.bottom > rect.bottom);
                }
            }
            match growth_direction {
                TextDirection::Rtl => {
                    assert_near(new_rect.right, rect.right);
                    if done_horizontally {
                        assert_near(new_rect.left, rect.left);
                    } else if near(new_rect.left, rect.left) {
                        done_horizontally = true;
                    } else {
                        assert!(new_rect.left < rect.left);
                    }
                }
                TextDirection::Ltr => {
                    assert_near(new_rect.left, rect.left);
                    if done_horizontally {
                        assert_near(new_rect.right, rect.right);
                    } else if near(new_rect.right, rect.right) {
                        done_horizontally = true;
                    } else {
                        assert!(new_rect.right > rect.right);
                    }
                }
            }
            rect = new_rect;
            if !has_scheduled_frame(tester) {
                break;
            }
        }
    }

    #[test]
    fn popup_menu_positioning() {
        use TextDirection::{Ltr, Rtl};
        with_main_thread_stack(|| {
            let mut tester = tester();
            let cases = [
                (Ltr, AlignmentGeometry::TOP_RIGHT, Rtl, 792.0, 8.0, false),
                (Rtl, AlignmentGeometry::TOP_RIGHT, Rtl, 792.0, 8.0, false),
                (Ltr, AlignmentGeometry::TOP_LEFT, Ltr, 8.0, 8.0, false),
                (Rtl, AlignmentGeometry::TOP_LEFT, Ltr, 8.0, 8.0, false),
                (Ltr, AlignmentGeometry::TOP_CENTER, Ltr, 350.0, 8.0, false),
                (Rtl, AlignmentGeometry::TOP_CENTER, Rtl, 450.0, 8.0, false),
                (
                    Ltr,
                    AlignmentGeometry::CENTER_RIGHT,
                    Rtl,
                    792.0,
                    250.0,
                    false,
                ),
                (
                    Rtl,
                    AlignmentGeometry::CENTER_RIGHT,
                    Rtl,
                    792.0,
                    250.0,
                    false,
                ),
                (Ltr, AlignmentGeometry::CENTER_LEFT, Ltr, 8.0, 250.0, false),
                (Rtl, AlignmentGeometry::CENTER_LEFT, Ltr, 8.0, 250.0, false),
                (Ltr, AlignmentGeometry::CENTER, Ltr, 350.0, 250.0, false),
                (Rtl, AlignmentGeometry::CENTER, Rtl, 450.0, 250.0, false),
                (
                    Ltr,
                    AlignmentGeometry::BOTTOM_RIGHT,
                    Rtl,
                    792.0,
                    500.0,
                    true,
                ),
                (
                    Rtl,
                    AlignmentGeometry::BOTTOM_RIGHT,
                    Rtl,
                    792.0,
                    500.0,
                    true,
                ),
                (Ltr, AlignmentGeometry::BOTTOM_LEFT, Ltr, 8.0, 500.0, true),
                (Rtl, AlignmentGeometry::BOTTOM_LEFT, Ltr, 8.0, 500.0, true),
                (
                    Ltr,
                    AlignmentGeometry::BOTTOM_CENTER,
                    Ltr,
                    350.0,
                    500.0,
                    true,
                ),
                (
                    Rtl,
                    AlignmentGeometry::BOTTOM_CENTER,
                    Rtl,
                    450.0,
                    500.0,
                    true,
                ),
            ];
            for (text_direction, alignment, growth_direction, x, y, then_up) in cases {
                test_positioning(
                    &mut tester,
                    text_direction,
                    alignment,
                    growth_direction,
                    Rect::from_ltwh(x, y, 0.0, 0.0),
                    then_up,
                );
            }
        });
    }

    #[test]
    fn popup_menu_offset_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_menu_button = |offset: Offset| {
                Material::new()
                    .child(
                        PopupMenuButton::new(|_app, _context| {
                            vec![PopupMenuEntryRef::new(
                                PopupMenuItem::new(Some(Builder::new(|_app, _context| {
                                    Text::new("AAA").into_widget()
                                })))
                                .value(1),
                            )]
                        })
                        .offset(offset),
                    )
                    .into_widget()
            };
            let in_scaffold = |tester: &mut WidgetTester, body: WidgetRef| {
                pump_app(tester, m3, move |_app| {
                    crate::scaffold::Scaffold::new().body(body).into_widget()
                });
            };
            let icon_button_center = |tester: &WidgetTester| {
                tester
                    .get_rect(tester.render_box_of(only(tester.find_by_type::<IconButton>())))
                    .center()
            };

            // Popup a menu without any offset.
            in_scaffold(&mut tester, build_menu_button(Offset::ZERO));

            // Popup the menu.
            let center = icon_button_center(&tester);
            tester.tap(center, 1);
            tester.pump_and_settle();

            // Initial state, the menu start at Offset(8.0, 8.0), the 8 pixels is edge padding
            // when offset.dx < 8.0.
            assert_offset_near(
                rect_of::<PopupMenu<i32>>(&tester).top_left(),
                Offset::new(8.0, 8.0),
            );

            // Collapse the menu.
            tester.tap(center, 1);
            tester.pump_and_settle();

            // Popup a new menu with Offset(50.0, 50.0).
            in_scaffold(&mut tester, build_menu_button(Offset::new(50.0, 50.0)));

            tester.tap(center, 1);
            tester.pump_and_settle();

            // This time the menu should start at Offset(50.0, 50.0), the padding only added
            // when offset.dx < 8.0.
            assert_offset_near(
                rect_of::<PopupMenu<i32>>(&tester).top_left(),
                Offset::new(50.0, 50.0),
            );
        });
    }

    #[test]
    fn popup_menu_button_popup_menu_divider() {
        // Regression test for https://github.com/flutter/flutter/issues/27072
        with_main_thread_stack(|| {
            let mut tester = tester();
            let selected_value: Rc<RefCell<String>> = Rc::default();
            let chosen = Rc::clone(&selected_value);
            centered_in_scaffold(&mut tester, m3, move || {
                PopupMenuButton::new(|_app, _context| {
                    vec![
                        item("1".to_owned(), "1"),
                        PopupMenuEntryRef::new(PopupMenuDivider::new()),
                        item("2".to_owned(), "2"),
                    ]
                })
                .on_selected(move |_app, result: String| *chosen.borrow_mut() = result)
                .initial_value("1".to_owned())
                .child(Text::new("Menu Button"))
                .into_widget()
            });

            for value in ["1", "2"] {
                tap_text(&mut tester, "Menu Button");
                tester.pump_and_settle();
                assert!(shows(&tester, "1"));
                assert_eq!(tester.find_onstage_by_type::<PopupMenuDivider>().len(), 1);
                assert!(shows(&tester, "2"));

                tap_text(&mut tester, value);
                tester.pump_and_settle();
                assert_eq!(*selected_value.borrow(), value);
            }
        });
    }

    #[test]
    fn popup_menu_item_child_height_is_a_minimum_child_is_vertically_centered() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let popup_menu_button_key = GlobalKey::new();
            let key = popup_menu_button_key.clone();
            centered_in_scaffold(&mut tester, m3, move || {
                PopupMenuButton::new(|_app, _context| {
                    vec![
                        // This menu item's height will be 48 because the default minimum height
                        // is 48 and the height of the text is less than 48.
                        item("0".to_owned(), "Item 0"),
                        // This menu item's height parameter specifies its minimum height. The
                        // overall height of the menu item will be 50 because the child's
                        // height 40, is less than 50.
                        PopupMenuEntryRef::new(
                            PopupMenuItem::new(Some(
                                SizedBox::new().height(40.0).child(Text::new("Item 1")),
                            ))
                            .height(50.0)
                            .value("1".to_owned()),
                        ),
                        // This menu item's height parameter specifies its minimum height, so
                        // the overall height of the menu item will be 75.
                        PopupMenuEntryRef::new(
                            PopupMenuItem::new(Some(SizedBox::new().child(Text::new("Item 2"))))
                                .height(75.0)
                                .value("2".to_owned()),
                        ),
                        // This menu item's height will be 100.
                        PopupMenuEntryRef::new(
                            PopupMenuItem::new(Some(
                                SizedBox::new().height(100.0).child(Text::new("Item 3")),
                            ))
                            .value("3".to_owned()),
                        ),
                    ]
                })
                .key(key.into())
                .child(Text::new("button"))
                .on_selected(|_app, _result: String| {})
                .into_widget()
            });

            // Show the menu
            tap_key(&mut tester, &popup_menu_button_key);
            tester.pump_and_settle();

            // The menu items and their InkWells should have the expected vertical size
            for (text, height) in [
                ("Item 0", 48.0),
                ("Item 1", 50.0),
                ("Item 2", 75.0),
                ("Item 3", 100.0),
            ] {
                assert_eq!(
                    rect_with_text::<PopupMenuItem<String>>(&tester, text).height(),
                    height
                );
                assert_eq!(rect_with_text::<InkWell>(&tester, text).height(), height);
            }

            // Menu item children which whose height is less than the PopupMenuItem
            // are vertically centered.
            for text in ["Item 0", "Item 2"] {
                assert_near(
                    rect_with_text::<PopupMenuItem<String>>(&tester, text)
                        .center()
                        .dy(),
                    text_rect(&tester, text).center().dy(),
                );
            }
        });
    }

    /// A button showing `items` in the scaffold's centre, keyed `key`, with a `Text('button')`
    /// child: the tree of Dart's padding and height cases.
    fn keyed_text_button(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        key: &GlobalKey,
        items: impl Fn() -> Vec<PopupMenuEntryRef<String>> + 'static,
    ) {
        let key = key.clone();
        centered_in_scaffold(tester, theme, move || {
            PopupMenuButton::new(move |_app, _context| items())
                .key(key.into())
                .child(Text::new("button"))
                .on_selected(|_app, _result: String| {})
                .into_widget()
        });
    }

    fn disabled_and_enabled_items() -> Vec<PopupMenuEntryRef<String>> {
        vec![
            PopupMenuEntryRef::new(
                PopupMenuItem::new(Some(Text::new("Item 0")))
                    .value("0".to_owned())
                    .enabled(false),
            ),
            item("1".to_owned(), "Item 1"),
        ]
    }

    /// Dart's `tester.widget<Padding>(find.widgetWithText(Padding, text).first).padding`: the
    /// `Padding` nearest above the text, as Dart's ancestor finder lists the nearest first.
    fn nearest_padding_of(tester: &WidgetTester, text: &str) -> EdgeInsetsGeometry {
        let padding = widget_with_text::<Padding>(tester, text);
        tester.widget(padding, |padding: &Padding| padding.padding)
    }

    /// Dart's padding of the `ConstrainedBox` whose child is a `Padding`, above `text`.
    fn item_padding_of<W: 'static>(tester: &WidgetTester, text: &str) -> EdgeInsetsGeometry {
        let text_element = only(tester.find_onstage_text(text));
        let item = tester.ancestor_of_type::<W>(text_element);
        let constrained = tester
            .descendants_of_type::<ConstrainedBox>(item)
            .into_iter()
            .find(|constrained| {
                let app = tester.cell.borrow();
                let widget =
                    waft_widgets::downcast_widget::<ConstrainedBox>(&**constrained.widget(&app))
                        .expect("found by type");
                widget.child.as_ref().is_some_and(|child| {
                    waft_widgets::downcast_widget::<Padding>(&**child).is_some()
                })
            })
            .expect("a ConstrainedBox around the item's Padding");
        let app = tester.cell.borrow();
        let widget = waft_widgets::downcast_widget::<ConstrainedBox>(&**constrained.widget(&app))
            .expect("found");
        waft_widgets::downcast_widget::<Padding>(&**widget.child.as_ref().expect("a child"))
            .expect("a Padding")
            .padding
    }

    /// Dart's `tester.widget<SingleChildScrollView>(find.byType(SingleChildScrollView)).padding`.
    fn menu_padding(tester: &WidgetTester) -> Option<EdgeInsetsGeometry> {
        let scroll_view = only(tester.find_by_type::<SingleChildScrollView>());
        tester.widget(scroll_view, |scroll_view: &SingleChildScrollView| {
            scroll_view.padding
        })
    }

    #[test]
    fn material3_popup_menu_item_default_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            keyed_text_button(&mut tester, m3, &key, disabled_and_enabled_items);

            // Show the menu.
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            // Dart's `paddingFor` reads 'Item 0' for both.
            assert_eq!(
                nearest_padding_of(&tester, "Item 0"),
                EdgeInsetsGeometry::symmetric(0.0, 12.0)
            );
            assert_eq!(
                nearest_padding_of(&tester, "Item 0"),
                EdgeInsetsGeometry::symmetric(0.0, 12.0)
            );
        });
    }

    #[test]
    fn material2_popup_menu_item_default_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            keyed_text_button(&mut tester, m2, &key, disabled_and_enabled_items);

            // Show the menu.
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            assert_eq!(
                nearest_padding_of(&tester, "Item 0"),
                EdgeInsetsGeometry::symmetric(0.0, 16.0)
            );
            assert_eq!(
                nearest_padding_of(&tester, "Item 0"),
                EdgeInsetsGeometry::symmetric(0.0, 16.0)
            );
        });
    }

    /// Dart's 'PopupMenu default padding' and the second 'Material2 - PopupMenuItem default
    /// padding'.
    fn default_menu_padding(theme: fn(&App) -> ThemeData) {
        let mut tester = tester();
        let key = GlobalKey::new();
        keyed_text_button(&mut tester, theme, &key, disabled_and_enabled_items);

        // Show the menu.
        tap_key(&mut tester, &key);
        tester.pump(Duration::from_millis(300));

        // Check popup menu padding.
        assert_eq!(
            menu_padding(&tester),
            Some(EdgeInsetsGeometry::symmetric(8.0, 0.0))
        );
    }

    #[test]
    fn popup_menu_default_padding() {
        with_main_thread_stack(|| default_menu_padding(m3));
    }

    #[test]
    fn material2_popup_menu_default_padding() {
        with_main_thread_stack(|| default_menu_padding(m2));
    }

    #[test]
    fn popup_menu_item_custom_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            keyed_text_button(&mut tester, m2, &key, || {
                let custom = |text: &str, padding: EdgeInsets, height: Option<f64>| {
                    let item = PopupMenuItem::new(Some(Text::new(text)))
                        .padding(padding)
                        .value("0".to_owned());
                    PopupMenuEntryRef::new(match height {
                        Some(height) => item.height(height),
                        None => item,
                    })
                };
                vec![
                    custom("Item 0", EdgeInsets::ZERO, None),
                    custom("Item 1", EdgeInsets::ZERO, Some(0.0)),
                    custom("Item 2", EdgeInsets::all(20.0), None),
                    custom("Item 3", EdgeInsets::all(20.0), Some(100.0)),
                ]
            });

            // Show the menu
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            // The menu items and their InkWells should have the expected vertical size
            // given the interactions between heights and padding.
            for (text, height) in [
                ("Item 0", 48.0),  // Minimum interactive height (48)
                ("Item 1", 16.0),  // Height of text (16)
                ("Item 2", 56.0),  // Padding (20.0 + 20.0) + Height of text (16) = 56
                ("Item 3", 100.0), // Height value of 100, since child (16) + padding (40) < 100
            ] {
                assert_eq!(
                    rect_with_text::<PopupMenuItem<String>>(&tester, text).height(),
                    height,
                    "{text}"
                );
            }

            for (text, padding) in [
                ("Item 0", EdgeInsetsGeometry::ZERO),
                ("Item 1", EdgeInsetsGeometry::ZERO),
                ("Item 2", EdgeInsetsGeometry::all(20.0)),
                ("Item 3", EdgeInsetsGeometry::all(20.0)),
            ] {
                assert_eq!(
                    item_padding_of::<PopupMenuItem<String>>(&tester, text),
                    padding
                );
            }
        });
    }

    #[test]
    fn checked_popup_menu_item_child_height_is_a_minimum_child_is_vertically_centered() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            keyed_text_button(&mut tester, m3, &key, || {
                let checked = |text: &str, value: &str, height: Option<f64>, child: SizedBox| {
                    let item = CheckedPopupMenuItem::new()
                        .checked(true)
                        .value(value.to_owned())
                        .child(child.child(Text::new(text)));
                    PopupMenuEntryRef::new(match height {
                        Some(height) => item.height(height),
                        None => item,
                    })
                };
                vec![
                    // This menu item's height will be 56.0 because the default minimum height
                    // is 48, but the contents of PopupMenuItem are 56.0 tall.
                    PopupMenuEntryRef::new(
                        CheckedPopupMenuItem::new()
                            .checked(true)
                            .value("0".to_owned())
                            .child(Text::new("Item 0")),
                    ),
                    // This menu item's height parameter specifies its minimum height. The
                    // overall height of the menu item will be 60 because the child's
                    // height 56, is less than 60.
                    checked("Item 1", "1", Some(60.0), SizedBox::new().height(40.0)),
                    // This menu item's height parameter specifies its minimum height, so the
                    // overall height of the menu item will be 75.
                    checked("Item 2", "2", Some(75.0), SizedBox::new()),
                    // This menu item's height will be 100.
                    checked("Item 3", "3", Some(100.0), SizedBox::new()),
                ]
            });

            // Show the menu
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            // The menu items and their InkWells should have the expected vertical size
            for (text, height) in [
                ("Item 0", 56.0),
                ("Item 1", 60.0),
                ("Item 2", 75.0),
                ("Item 3", 100.0),
            ] {
                assert_eq!(
                    rect_with_text::<CheckedPopupMenuItem<String>>(&tester, text).height(),
                    height,
                    "{text}"
                );
                // We evaluate the InkWell at the first index because that is the ListTile's
                // InkWell, which wins in the gesture arena over the child's InkWell and
                // is the one of interest.
                let ink_wells: Vec<AnyElement> = tester
                    .ancestors(only(tester.find_onstage_text(text)))
                    .into_iter()
                    .filter(|ancestor| {
                        waft_widgets::downcast_widget::<InkWell>(
                            &**ancestor.widget(&tester.cell.borrow()),
                        )
                        .is_some()
                    })
                    .collect();
                assert_eq!(ink_wells.len(), 2);
                let outer = ink_wells[1];
                assert_eq!(
                    tester.get_rect(tester.render_box_of(outer)).height(),
                    height,
                    "{text}"
                );
            }

            // Menu item children which whose height is less than the PopupMenuItem
            // are vertically centered.
            for text in ["Item 0", "Item 2"] {
                assert_near(
                    rect_with_text::<CheckedPopupMenuItem<String>>(&tester, text)
                        .center()
                        .dy(),
                    text_rect(&tester, text).center().dy(),
                );
            }
        });
    }

    #[test]
    fn checked_popup_menu_item_custom_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            keyed_text_button(&mut tester, m3, &key, || {
                let custom = |text: &str, padding: EdgeInsets, height: Option<f64>| {
                    let item = CheckedPopupMenuItem::new()
                        .padding(padding)
                        .value("0".to_owned())
                        .child(Text::new(text));
                    PopupMenuEntryRef::new(match height {
                        Some(height) => item.height(height),
                        None => item,
                    })
                };
                vec![
                    custom("Item 0", EdgeInsets::ZERO, None),
                    custom("Item 1", EdgeInsets::ZERO, Some(0.0)),
                    custom("Item 2", EdgeInsets::all(20.0), None),
                    custom("Item 3", EdgeInsets::all(20.0), Some(100.0)),
                ]
            });

            // Show the menu
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            // The menu items and their InkWells should have the expected vertical size
            // given the interactions between heights and padding.
            for (text, height) in [
                ("Item 0", 56.0),  // Minimum ListTile height (56)
                ("Item 1", 56.0),  // Minimum ListTile height (56)
                ("Item 2", 96.0),  // Padding (20.0 + 20.0) + Height of ListTile (56) = 96
                ("Item 3", 100.0), // Height value of 100, since child (56) + padding (40) < 100
            ] {
                assert_eq!(
                    rect_with_text::<CheckedPopupMenuItem<String>>(&tester, text).height(),
                    height,
                    "{text}"
                );
            }

            for (text, padding) in [
                ("Item 0", EdgeInsetsGeometry::ZERO),
                ("Item 1", EdgeInsetsGeometry::ZERO),
                ("Item 2", EdgeInsetsGeometry::all(20.0)),
                ("Item 3", EdgeInsetsGeometry::all(20.0)),
            ] {
                assert_eq!(
                    item_padding_of::<CheckedPopupMenuItem<String>>(&tester, text),
                    padding
                );
            }
        });
    }

    #[test]
    fn update_popup_menu_item_layout_while_the_menu_is_visible() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let build_frame =
                |tester: &mut WidgetTester, text_direction: TextDirection, font_size: f64| {
                    let key = key.clone();
                    let root = {
                        let app = tester.cell.borrow();
                        crate::app::MaterialApp::new()
                            .theme(m2(&app))
                            .builder(move |app, context, child| {
                                let title_medium = Theme::of(app, context)
                                    .text_theme
                                    .title_medium
                                    .clone()
                                    .expect("titleMedium");
                                Directionality::new(
                                    text_direction,
                                    PopupMenuTheme::new(
                                        PopupMenuTheme::of(app, context).copy_with().text_style(
                                            title_medium.copy_with().font_size(font_size),
                                        ),
                                        child.expect("a navigator").clone(),
                                    ),
                                )
                                .into_widget()
                            })
                            .home(
                                crate::scaffold::Scaffold::new()
                                    .body(
                                        PopupMenuButton::new(|_app, _context| {
                                            vec![
                                                item("0".to_owned(), "Item 0"),
                                                item("1".to_owned(), "Item 1"),
                                            ]
                                        })
                                        .key(key.into())
                                        .child(Text::new("button"))
                                        .on_selected(|_app, _result: String| {}),
                                    )
                                    .into_widget(),
                            )
                            .into_widget()
                    };
                    tester.pump_widget(root);
                };

            // Show the menu
            build_frame(&mut tester, TextDirection::Ltr, 24.0);
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            // The menu items should have their default heights and horizontal alignment.
            for text in ["Item 0", "Item 1"] {
                assert_eq!(
                    rect_with_text::<PopupMenuItem<String>>(&tester, text).height(),
                    48.0
                );
                assert_near(text_rect(&tester, text).left, 24.0);
            }

            // While the menu is up, change its font size to 64 (default is 16).
            build_frame(&mut tester, TextDirection::Ltr, 64.0);
            tester.pump_and_settle(); // Theme changes are animated.
            for text in ["Item 0", "Item 1"] {
                assert_eq!(
                    rect_with_text::<PopupMenuItem<String>>(&tester, text).height(),
                    128.0
                );
                assert_eq!(text_rect(&tester, text).height(), 128.0);
                assert_near(text_rect(&tester, text).left, 24.0);
            }

            // While the menu is up, change the textDirection to rtl. Now menu items
            // will be aligned right.
            build_frame(&mut tester, TextDirection::Rtl, 24.0);
            tester.pump_and_settle(); // Theme changes are animated.
            for text in ["Item 0", "Item 1"] {
                assert_eq!(
                    rect_with_text::<PopupMenuItem<String>>(&tester, text).height(),
                    48.0
                );
                assert_near(text_rect(&tester, text).left, 72.0);
            }
        });
    }

    #[test]
    #[should_panic(expected = "You can only pass [child] or [icon], not both.")]
    fn popup_menu_buttons_child_and_icon_properties_cannot_be_simultaneously_defined() {
        PopupMenuButton::<i32>::new(|_app, _context| Vec::new())
            .icon(Icon::new(Some(Icons::error())))
            .child(Container::new())
            .into_widget();
    }

    /// Dart's 'PopupMenuButton default tooltip' and 'custom tooltip'.
    fn three_buttons_tooltips(tooltip: Option<&'static str>) -> Vec<Option<String>> {
        let mut tester = tester();
        pump_app(&mut tester, m3, move |_app| {
            let with_tooltip = |button: PopupMenuButton<i32>| match tooltip {
                Some(tooltip) => button.tooltip(tooltip),
                None => button,
            };
            let items = |_app: &mut App, _context: BuildContext| vec![item(1, "Tap me please!")];
            Material::new()
                .child(Column::new().children(vec![
                    // Tooltip should be present when [PopupMenuButton.icon]
                    // and [PopupMenuButton.child] are undefined.
                    with_tooltip(PopupMenuButton::new(items)).into_widget(),
                    // Tooltip should be present when [PopupMenuButton.child] is defined.
                    with_tooltip(PopupMenuButton::new(items))
                        .child(Text::new("Test text"))
                        .into_widget(),
                    // Tooltip should be present when [PopupMenuButton.icon] is defined.
                    with_tooltip(PopupMenuButton::new(items))
                        .icon(Icon::new(Some(Icons::check())))
                        .into_widget(),
                ]))
                .into_widget()
        });
        tester
            .find_by_type::<Tooltip>()
            .into_iter()
            .map(|tooltip| tester.widget(tooltip, |tooltip: &Tooltip| tooltip.message.clone()))
            .collect()
    }

    #[test]
    fn popup_menu_button_default_tooltip() {
        with_main_thread_stack(|| {
            // The default tooltip is defined as [MaterialLocalizations.showMenuTooltip]
            // and it is used when no tooltip is provided.
            let expected = DefaultMaterialLocalizations.show_menu_tooltip();
            assert_eq!(three_buttons_tooltips(None), vec![Some(expected); 3]);
        });
    }

    #[test]
    fn popup_menu_button_custom_tooltip() {
        with_main_thread_stack(|| {
            assert_eq!(
                three_buttons_tooltips(Some("Test tooltip")),
                vec![Some("Test tooltip".to_owned()); 3]
            );
        });
    }

    #[test]
    fn allow_widget_for_popup_menu_button_icon() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Material::new()
                    .child(
                        PopupMenuButton::new(|_app, _context| vec![item(1, "Tap me please!")])
                            .tooltip("Test tooltip")
                            .icon(Text::new("PopupMenuButton icon")),
                    )
                    .into_widget()
            });

            assert_eq!(tester.find_onstage_text("PopupMenuButton icon").len(), 1);
        });
    }

    /// Dart's 'showMenu uses nested navigator by default' and 'uses root navigator if
    /// useRootNavigator is true': whether the menu's route is the nested navigator's, which
    /// Dart's `MenuObserver`s count.
    fn menu_is_in_nested_navigator(use_root_navigator: bool) -> bool {
        let mut tester = tester();
        let nested_key = GlobalKey::new();
        let button_key = GlobalKey::new();
        let (nested, button) = (nested_key.clone(), button_key.clone());
        pump_app(&mut tester, m3, move |_app| {
            Navigator::new()
                .key(nested.into())
                .on_generate_route(move |app, _settings| {
                    let button = button.clone();
                    let route = MaterialPageRoute::new(
                        app,
                        Rc::new(move |_app, context| {
                            crate::elevated_button::ElevatedButton::new(
                                Some(Listener::new(move |app| {
                                    drop(show_menu(
                                        app,
                                        context,
                                        Some(RelativeRect::FILL),
                                        None,
                                        vec![item(1, "1")],
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        None,
                                        use_root_navigator,
                                        None,
                                        Clip::None,
                                        None,
                                        None,
                                        None,
                                    ));
                                })),
                                Some(Text::new("Show Menu")),
                            )
                            .key(button.clone().into())
                            .into_widget()
                        }),
                    );
                    Some(Route::as_route(route))
                })
                .into_widget()
        });

        // Open the dialog.
        tap_key(&mut tester, &button_key);
        tester.pump(Duration::ZERO);

        let menu = only(tester.find_by_type::<PopupMenu<i32>>());
        let nested = nested_key
            .current_context(&tester.cell.borrow())
            .expect("the nested navigator is mounted");
        tester.ancestors(menu).contains(&nested)
    }

    #[test]
    fn show_menu_uses_nested_navigator_by_default() {
        with_main_thread_stack(|| assert!(menu_is_in_nested_navigator(false)));
    }

    #[test]
    fn show_menu_uses_root_navigator_if_use_root_navigator_is_true() {
        with_main_thread_stack(|| assert!(!menu_is_in_nested_navigator(true)));
    }

    #[test]
    fn popup_menu_button_calling_show_button_menu_manually() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let global_key = GlobalKey::new();
            let key = global_key.clone();
            pump_app(&mut tester, m3, move |_app| {
                Material::new()
                    .child(Column::new().children(vec![
                        PopupMenuButton::new(|_app, _context| vec![item(1, "Tap me please!")])
                            .key(key.into())
                            .into_widget(),
                    ]))
                    .into_widget()
            });

            assert!(!shows(&tester, "Tap me please!"));

            let state = global_key
                .current_state::<PopupMenuButtonState<i32>>(&tester.cell.borrow())
                .expect("the button is mounted");
            state.show_button_menu(&mut tester.cell.borrow_mut());
            // The PopupMenuItem will appear after an animation, hence,
            // we have to first wait for the tester to settle.
            tester.pump_and_settle();

            assert!(shows(&tester, "Tap me please!"));
        });
    }

    /// Dart's `MaterialApp(builder: (context, child) => MediaQuery(data: data, child: child!),
    /// theme: theme, home: home)`.
    fn app_under_media_query(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        data: MediaQueryData,
        home: WidgetRef,
    ) {
        let root = {
            let app = tester.cell.borrow();
            crate::app::MaterialApp::new()
                .theme(theme(&app))
                .builder(move |_app, _context, child| {
                    MediaQuery::new(data.clone(), child.expect("a navigator").clone()).into_widget()
                })
                .home(home)
                .into_widget()
        };
        tester.pump_widget(root);
    }

    /// Dart's `tester.getRect(find.byType(SingleChildScrollView))`.
    fn scroll_view_rect(tester: &WidgetTester) -> Rect {
        rect_of::<SingleChildScrollView>(tester)
    }

    #[test]
    fn popup_menu_button_has_expected_default_mouse_cursor_on_hover() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let button_key = key.clone();
            centered_in_scaffold(&mut tester, m3, move || {
                PopupMenuButton::new(|_app, _context| {
                    vec![item("a".to_owned(), "A"), item("b".to_owned(), "B")]
                })
                .key(button_key.into())
                .into_widget()
            });

            let mut gesture = WidgetTester::mouse(1, 0);
            let center = tester.center_of(&key);
            tester.add_pointer(&mut gesture, center);
            tester.pump(Duration::ZERO);

            let basic: MouseCursorRef = waft_services::SystemMouseCursors::BASIC.into();
            assert!(*tester.active_cursor(1) == *basic);
        });
    }

    /// Dart's tree for 'PopupMenuItem changes mouse cursor when hovered' and its checked
    /// twin: `item` at the top left of a scaffold, under a `MouseRegion` that would show the
    /// forbidden cursor.
    fn item_under_forbidden_cursor(tester: &mut WidgetTester, item: WidgetRef) {
        pump_app(tester, m3, move |_app| {
            crate::scaffold::Scaffold::new()
                .body(
                    Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                        Material::new().child(
                            waft_widgets::MouseRegion::new()
                                .cursor(waft_services::SystemMouseCursors::FORBIDDEN.into())
                                .child(
                                    // The [SemanticsRole.menu] is added here to make sure the
                                    // item's parent role is menu.
                                    Semantics::new().role(SemanticsRole::Menu).child(item),
                                ),
                        ),
                    ),
                )
                .into_widget()
        });
    }

    /// Dart's 'PopupMenuItem changes mouse cursor when hovered' and 'CheckedPopupMenuItem
    /// changes mouse cursor when hovered': the cursor of an item with a text cursor, of one
    /// with none, and of a disabled one.
    fn item_cursors(item: impl Fn(&GlobalKey, Option<MouseCursorRef>, bool) -> WidgetRef) {
        let mut tester = tester();
        let key = GlobalKey::new();
        let cursor = |name: waft_services::SystemMouseCursor| -> MouseCursorRef { name.into() };
        item_under_forbidden_cursor(
            &mut tester,
            item(
                &key,
                Some(cursor(waft_services::SystemMouseCursors::TEXT)),
                true,
            ),
        );

        let mut gesture = WidgetTester::mouse(1, 0);
        let center = tester.center_of(&key);
        tester.add_pointer(&mut gesture, center);
        tester.pump(Duration::ZERO);

        assert!(*tester.active_cursor(1) == *cursor(waft_services::SystemMouseCursors::TEXT));

        // Test default cursor
        item_under_forbidden_cursor(&mut tester, item(&key, None, true));
        assert!(*tester.active_cursor(1) == *cursor(waft_services::SystemMouseCursors::BASIC));

        // Test default cursor when disabled
        item_under_forbidden_cursor(&mut tester, item(&key, None, false));
        assert!(*tester.active_cursor(1) == *cursor(waft_services::SystemMouseCursors::BASIC));
    }

    #[test]
    fn popup_menu_item_changes_mouse_cursor_when_hovered() {
        with_main_thread_stack(|| {
            item_cursors(|key, mouse_cursor, enabled| {
                let item = PopupMenuItem::new(Some(Container::new()))
                    .key(key.clone().into())
                    .value(1)
                    .enabled(enabled);
                match mouse_cursor {
                    Some(mouse_cursor) => item.mouse_cursor(mouse_cursor),
                    None => item,
                }
                .into_widget()
            });
        });
    }

    #[test]
    fn checked_popup_menu_item_changes_mouse_cursor_when_hovered() {
        with_main_thread_stack(|| {
            item_cursors(|key, mouse_cursor, enabled| {
                let item = CheckedPopupMenuItem::new()
                    .key(key.clone().into())
                    .value(1)
                    .enabled(enabled)
                    .child(Container::new());
                match mouse_cursor {
                    Some(mouse_cursor) => item.mouse_cursor(mouse_cursor),
                    None => item,
                }
                .into_widget()
            });
        });
    }

    #[test]
    fn popup_menu_in_app_bar_does_not_overlap_with_the_status_bar() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let status_bar_height = 24.0;
            let first_item_key = GlobalKey::new();
            let selected_value = Rc::new(Cell::new(1));
            let first_key = first_item_key.clone();
            let home = waft_widgets::StatefulBuilder::new(move |_app, _context, set_state| {
                let selected = Rc::clone(&selected_value);
                let first_key = first_key.clone();
                crate::scaffold::Scaffold::new()
                    .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                        crate::app_bar::AppBar::new()
                            .title(Text::new("PopupMenu Test"))
                            .actions(vec![
                                PopupMenuButton::new(move |_app, _context| {
                                    vec![
                                        PopupMenuEntryRef::new(
                                            PopupMenuItem::new(Some(Text::new("Item 1")))
                                                .key(first_key.clone().into())
                                                .value(1),
                                        ),
                                        item(2, "Item 2"),
                                        item(3, "Item 3"),
                                    ]
                                })
                                .on_selected({
                                    let selected = Rc::clone(&selected);
                                    let set_state = Rc::clone(&set_state);
                                    move |app, result: i32| {
                                        let selected = Rc::clone(&selected);
                                        set_state(app, &mut |_app| selected.set(result));
                                    }
                                })
                                .initial_value(selected.get())
                                .into_widget(),
                            ]),
                    ))
                    .into_widget()
            })
            .into_widget();
            app_under_media_query(
                &mut tester,
                m3,
                MediaQueryData::new().padding(EdgeInsets::only(0.0, status_bar_height, 0.0, 0.0)),
                home,
            );

            let more_vert = || {
                let icon = Icons::more_vert();
                let element = tester.find_by_icon(&icon);
                tester.get_rect(tester.render_box_of(element)).center()
            };
            let center = more_vert();
            tester.tap(center, 1);
            tester.pump_and_settle();

            // Tap third item.
            tap_text(&mut tester, "Item 3");
            tester.pump_and_settle();

            // Open popupMenu again.
            tester.tap(center, 1);
            tester.pump_and_settle();

            // Check whether the first item is not overlapping with status bar.
            assert!(rect_of_key(&tester, &first_item_key).top > status_bar_height);
        });
    }

    #[test]
    fn vertically_long_popup_menu_does_not_overlap_with_the_status_bar_and_bottom_notch() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let window_padding_top = 44.0;
            let window_padding_bottom = 34.0;
            let home = crate::scaffold::Scaffold::new()
                .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                    crate::app_bar::AppBar::new().title(Text::new("PopupMenu Test")),
                ))
                .body(
                    PopupMenuButton::new(|_app, _context| {
                        (0..20).map(|i| item(i, &format!("Item {i}"))).collect()
                    })
                    .child(Text::new("Show Menu")),
                )
                .into_widget();
            app_under_media_query(
                &mut tester,
                m3,
                MediaQueryData::new().padding(EdgeInsets::only(
                    0.0,
                    window_padding_top,
                    0.0,
                    window_padding_bottom,
                )),
                home,
            );

            tap_text(&mut tester, "Show Menu");
            tester.pump_and_settle();

            let menu = scroll_view_rect(&tester);
            assert_near(menu.top, window_padding_top + 8.0);
            // Screen height is 600.
            assert_near(menu.bottom, 600.0 - window_padding_bottom - 8.0);
        });
    }

    /// Dart's `buildFrame` of the two 'PopupMenu position test when have unsafe area' cases:
    /// a pink 20x20 button among the app bar's actions, under a media query of `padding`.
    fn unsafe_area_frame(
        tester: &mut WidgetTester,
        padding: EdgeInsets,
        button_key: &GlobalKey,
        items: impl Fn() -> Vec<PopupMenuEntryRef<i32>> + 'static,
    ) {
        let home = crate::scaffold::Scaffold::new()
            .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                crate::app_bar::AppBar::new()
                    .title(Text::new("PopupMenu Test"))
                    .actions(vec![
                        PopupMenuButton::new(move |_app, _context| items())
                            .child(
                                SizedBox::new()
                                    .key(button_key.clone().into())
                                    .height(20.0)
                                    .width(20.0)
                                    .child(ColoredBox::new(crate::colors::Colors::PINK.primary())),
                            )
                            .into_widget(),
                    ]),
            ))
            .body(SizedBox::shrink())
            .into_widget();
        app_under_media_query(tester, m2, MediaQueryData::new().padding(padding), home);
    }

    #[test]
    fn popup_menu_position_test_when_have_unsafe_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            unsafe_area_frame(
                &mut tester,
                EdgeInsets::only(0.0, 32.0, 0.0, 32.0),
                &button_key,
                || vec![item(1, "-1-"), item(2, "-2-")],
            );

            tap_key(&mut tester, &button_key);
            tester.pump_and_settle();

            let button = rect_of_key(&tester, &button_key).top_right();
            assert_offset_near(button, Offset::new(800.0, 32.0)); // The topPadding is 32.0.

            let popup_menu = scroll_view_rect(&tester).top_right();

            // The menu should be positioned directly next to the top of the button.
            // The 8.0 pixels is [_kMenuScreenPadding].
            assert_offset_near(
                popup_menu,
                Offset::new(button.dx() - 8.0, button.dy() + 8.0),
            );
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/82874
    #[test]
    fn popup_menu_position_test_when_have_unsafe_area_left_right_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let padding = EdgeInsets::only(300.0, 32.0, 310.0, 64.0);
            let media_query_padding: Rc<Cell<Option<EdgeInsets>>> = Rc::default();
            let seen = Rc::clone(&media_query_padding);
            unsafe_area_frame(&mut tester, padding, &button_key, move || {
                let seen = Rc::clone(&seen);
                vec![
                    PopupMenuEntryRef::new(
                        PopupMenuItem::new(Some(Builder::new(move |app, context| {
                            seen.set(Some(MediaQuery::padding_of(app, context)));
                            // A long long text string.
                            Text::new("-1-".repeat(500)).into_widget()
                        })))
                        .value(1),
                    ),
                    item(2, "-2-"),
                ]
            });

            tap_key(&mut tester, &button_key);
            tester.pump_and_settle();

            let button = rect_of_key(&tester, &button_key).top_right();
            // The topPadding is 32.0.
            assert_offset_near(button, Offset::new(800.0 - padding.right, padding.top));

            let menu = scroll_view_rect(&tester);

            // The menu should be positioned directly next to the top of the button.
            // The 8.0 pixels is [_kMenuScreenPadding].
            assert_offset_near(
                menu.top_right(),
                Offset::new(800.0 - padding.right - 8.0, padding.top + 8.0),
            );
            assert_offset_near(
                menu.top_left(),
                Offset::new(padding.left + 8.0, padding.top + 8.0),
            );
            // Dart's next expectation, that the menu's bottom left is at
            // `Offset(padding.left + 8.0, 600.0 - padding.bottom - 8.0)`, waits on Valo: its
            // paragraph breaks a line only where UAX #14 allows one, where SkParagraph breaks
            // a word wider than the line, so the long text stays one line and the menu short.

            // The `MediaQueryData.padding` should be removed.
            assert_eq!(media_query_padding.get(), Some(EdgeInsets::ZERO));
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/163477
    #[test]
    fn popup_menus_overlay_can_be_rebuilt_even_when_the_button_is_unmounted() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let set_state: Rc<RefCell<Option<waft_widgets::StateSetter>>> = Rc::default();
            let show_button = Rc::new(Cell::new(true));

            let widget = |view_size: Size| {
                let (button_key, set_state, show_button) = (
                    button_key.clone(),
                    Rc::clone(&set_state),
                    Rc::clone(&show_button),
                );
                let app = crate::app::MaterialApp::new().home(
                    Material::new()
                        .child(waft_widgets::StatefulBuilder::new(
                            move |_app, _context, inner_set_state| {
                                *set_state.borrow_mut() = Some(inner_set_state);
                                if show_button.get() {
                                    PopupMenuButton::new(|_app, _context| {
                                        vec![PopupMenuEntryRef::new(
                                            PopupMenuItem::new(Some(Text::new("ACTION")))
                                                .value(1)
                                                .on_tap(Listener::new(|_app| {})),
                                        )]
                                    })
                                    .key(button_key.clone().into())
                                    .pop_up_animation_style(AnimationStyle::new(
                                        None,
                                        None,
                                        None,
                                        Some(Duration::from_millis(400)),
                                    ))
                                    .into_widget()
                                } else {
                                    Container::new().into_widget()
                                }
                            },
                        ))
                        .into_widget(),
                );
                Center::new()
                    .child(
                        SizedBox::new()
                            .width(view_size.width())
                            .height(view_size.height())
                            .child(app),
                    )
                    .into_widget()
            };

            // Pump a button
            tester.pump_widget(widget(Size::new(500.0, 500.0)));

            // Tap the button to show the menu
            tap_key(&mut tester, &button_key);
            tester.pump_and_settle();
            assert_eq!(tester.find_onstage_text("ACTION").len(), 1);
            assert!(context_of_maybe(&tester, &button_key).is_some());

            // Hide the button. The menu still shows since it's placed on a separate route.
            let setter = set_state.borrow().clone().expect("built");
            let flag = Rc::clone(&show_button);
            setter(&mut tester.cell.borrow_mut(), &mut |_app| flag.set(false));
            tester.pump(Duration::ZERO);
            assert_eq!(tester.find_onstage_text("ACTION").len(), 1);
            assert!(context_of_maybe(&tester, &button_key).is_none());

            // Resize the view, causing the menu to rebuild. Before the fix, this
            // rebuild would lead to a crash, because it relies on context of the button,
            // which has been unmounted.
            tester.pump_widget(widget(Size::new(300.0, 300.0)));
        });
    }

    fn context_of_maybe(tester: &WidgetTester, key: &GlobalKey) -> Option<BuildContext> {
        key.current_context(&tester.cell.borrow())
    }

    /// Dart's `show_menu(context: context, position: RelativeRect.fill, items: items,
    /// routeSettings: route_settings)` with the other arguments' defaults.
    fn show_menu_at_fill<T: Clone + PartialEq + 'static>(
        app: &mut App,
        context: BuildContext,
        items: Vec<PopupMenuEntryRef<T>>,
        route_settings: Option<RouteSettingsRef>,
    ) -> CompleterFuture<RouteResult> {
        show_menu(
            app,
            context,
            Some(RelativeRect::FILL),
            None,
            items,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            Clip::None,
            route_settings,
            None,
            None,
        )
    }

    /// Dart's `feedback` group, 'PopupMenuButton enableFeedback works properly': the click
    /// sound waits with `SystemSound`, so what decides it is checked instead, the
    /// `enableFeedback` the button's `InkWell` is built with, and no haptic is asked for.
    #[test]
    fn popup_menu_button_enable_feedback_works_properly() {
        #[derive(Default)]
        struct FeedbackTester {
            haptics: RefCell<Vec<waft_embedder::HapticFeedbackType>>,
        }

        impl waft_embedder::Haptics for FeedbackTester {
            fn feedback(&self, kind: waft_embedder::HapticFeedbackType) {
                self.haptics.borrow_mut().push(kind);
            }
        }

        with_main_thread_stack(|| {
            let feedback = Rc::new(FeedbackTester::default());
            let platform = waft_test::TestPlatform::new()
                .on(waft_embedder::TargetPlatform::Android)
                .with_haptics(Rc::clone(&feedback) as Rc<dyn waft_embedder::Haptics>);
            let mut tester = WidgetTester::with_platform(platform);
            tester.install_platform_fonts();
            for (widget_enable_feedback, theme_enable_feedback, expected) in [
                // PopupMenuButton with enabled feedback.
                (Some(true), None, true),
                // PopupMenuButton with disabled feedback.
                (Some(false), None, false),
                // PopupMenuButton with enabled feedback by default.
                (None, None, true),
                // PopupMenu with disabled feedback using PopupMenuButtonTheme.
                (None, Some(false), false),
                // PopupMenu enableFeedback property overrides PopupMenuButtonTheme.
                (Some(false), Some(true), false),
            ] {
                pump_app(&mut tester, m3, move |_app| {
                    let mut theme = PopupMenuThemeData::new();
                    theme.enable_feedback = theme_enable_feedback;
                    let mut button = PopupMenuButton::new(|_app, _context| vec![item(1, "One")])
                        .child(Text::new("Show Menu"));
                    button.enable_feedback = widget_enable_feedback;
                    crate::scaffold::Scaffold::new()
                        .body(PopupMenuTheme::new(theme, button))
                        .into_widget()
                });
                let ink_well = only(tester.find_by_type::<InkWell>());
                assert_eq!(
                    tester.widget(ink_well, |ink_well: &InkWell| ink_well
                        .ink_response
                        .enable_feedback),
                    expected
                );
                tap_text(&mut tester, "Show Menu");
                tester.pump_and_settle();
                assert!(shows(&tester, "One"));
                assert!(feedback.haptics.borrow().is_empty());
                tester.pump_widget(Container::new().into_widget());
            }
        });
    }

    #[test]
    fn can_customize_popup_menu_button_icon() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let icon_color = Color::new(0xffffff00);
            let icon_size = 29.5;
            centered_in_scaffold(&mut tester, m3, move || {
                PopupMenuButton::new(|_app, _context| vec![item("value".to_owned(), "child")])
                    .icon_color(icon_color)
                    .icon_size(icon_size)
                    .into_widget()
            });

            let more = Icons::adaptive().more(&tester.cell.borrow());
            let icon = tester.find_by_icon(&more);
            assert_eq!(
                tester.glyph_style(icon).color.as_deref().copied(),
                Some(icon_color)
            );
            assert_eq!(
                tester.get_rect(tester.render_box_of(icon)).size(),
                Size::new(icon_size, icon_size)
            );
        });
    }

    #[test]
    fn does_not_crash_in_small_overlay() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let navigator = GlobalKey::new();
            let key = navigator.clone();
            pump_app(&mut tester, m3, move |_app| {
                let navigator = key.clone();
                crate::scaffold::Scaffold::new()
                    .body(Column::new().children(vec![
                        crate::outlined_button::OutlinedButton::new(
                            Some(Listener::new(move |app| {
                                let context = navigator
                                    .current_context(app)
                                    .expect("the navigator is mounted");
                                drop(show_menu_at_fill::<()>(
                                    app,
                                    context,
                                    vec![PopupMenuEntryRef::new(PopupMenuItem::new(Some(
                                        Text::new("foo"),
                                    )))],
                                    None,
                                ));
                            })),
                            Some(Text::new("press")),
                        )
                        .into_widget(),
                        SizedBox::square(Some(10.0))
                            .child(Navigator::new().key(key.into()).on_generate_route(
                                |app, _settings| {
                                    let route = MaterialPageRoute::new(
                                        app,
                                        Rc::new(|_app, _context| {
                                            Container::new()
                                                .color(crate::colors::Colors::RED.primary())
                                                .into_widget()
                                        }),
                                    );
                                    Some(Route::as_route(route))
                                },
                            ))
                            .into_widget(),
                    ]))
                    .into_widget()
            });

            tap_text(&mut tester, "press");
            tester.pump_and_settle();
            assert_eq!(tester.find_onstage_text("foo").len(), 1);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/80869
    #[test]
    fn the_menu_position_test_in_the_scrollable_widget() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let key = button_key.clone();
            pump_app(&mut tester, m3, move |_app| {
                crate::scaffold::Scaffold::new()
                    .body(
                        SingleChildScrollView::new().child(Column::new().children(vec![
                        SizedBox::new().height(100.0).into_widget(),
                        PopupMenuButton::new(|_app, _context| vec![item(1, "-1-"), item(2, "-2-")])
                            .child(
                                SizedBox::new()
                                    .key(key.into())
                                    .height(10.0)
                                    .width(10.0)
                                    .child(ColoredBox::new(crate::colors::Colors::PINK.primary())),
                            )
                            .into_widget(),
                        SizedBox::new().height(600.0).into_widget(),
                    ])),
                    )
                    .into_widget()
            });
            let menu_top_left = |tester: &WidgetTester| {
                let scroll_views = tester.find_onstage_by_type::<SingleChildScrollView>();
                let last = *scroll_views.last().expect("the menu's scroll view");
                tester.get_rect(tester.render_box_of(last)).top_left()
            };

            // Open the menu.
            tap_key(&mut tester, &button_key);
            tester.pump_and_settle();

            let button = rect_of_key(&tester, &button_key).top_left();
            assert_offset_near(button, Offset::new(0.0, 100.0));

            // The menu should be positioned directly next to the top of the button.
            // The 8.0 pixels is [_kMenuScreenPadding].
            assert_offset_near(menu_top_left(&tester), Offset::new(8.0, 100.0));

            // Close the menu.
            tap_key(&mut tester, &button_key);
            tester.pump_and_settle();

            // Scroll a little bit.
            let scroll_view = only(tester.find_onstage_by_type::<SingleChildScrollView>());
            let start = tester.get_rect(tester.render_box_of(scroll_view)).center();
            tester.drag_from(start, Offset::new(0.0, -50.0), 1);

            let button = rect_of_key(&tester, &button_key).top_left();
            assert_offset_near(button, Offset::new(0.0, 50.0));

            // Open the menu again.
            tap_key(&mut tester, &button_key);
            tester.pump_and_settle();

            // The menu should be positioned directly next to the top of the button.
            // The 8.0 pixels is [_kMenuScreenPadding].
            assert_offset_near(menu_top_left(&tester), Offset::new(8.0, 50.0));
        });
    }

    #[test]
    fn popup_menu_button_custom_splash_radius() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_frame_without_child =
                |tester: &mut WidgetTester, splash_radius: Option<f64>| {
                    centered_in_scaffold(tester, m2, move || {
                        let mut button = PopupMenuButton::new(|_app, _context| {
                            vec![item("value".to_owned(), "child")]
                        });
                        button.splash_radius = splash_radius;
                        button.into_widget()
                    });
                };
            let build_frame_with_child = |tester: &mut WidgetTester, splash_radius: Option<f64>| {
                centered_in_scaffold(tester, m3, move || {
                    let mut button = PopupMenuButton::<String>::new(|_app, _context| {
                        vec![PopupMenuEntryRef::new(PopupMenuDivider::new())]
                    })
                    .child(Text::new("An item"));
                    button.splash_radius = splash_radius;
                    button.into_widget()
                });
            };
            let ink_response_radius = |tester: &WidgetTester| {
                let ink_response = only(tester.find_by_type::<crate::ink_well::InkResponse>());
                tester.widget(
                    ink_response,
                    |ink_response: &crate::ink_well::InkResponse| ink_response.radius,
                )
            };
            let ink_well_radius = |tester: &WidgetTester| {
                let ink_well = only(tester.find_by_type::<InkWell>());
                tester.widget(ink_well, |ink_well: &InkWell| ink_well.ink_response.radius)
            };

            build_frame_without_child(&mut tester, None);
            assert_eq!(
                ink_response_radius(&tester),
                Some(Material::DEFAULT_SPLASH_RADIUS)
            );
            build_frame_with_child(&mut tester, None);
            assert_eq!(ink_well_radius(&tester), None);

            let test_splash_radius = 50.0;

            build_frame_without_child(&mut tester, Some(test_splash_radius));
            assert_eq!(ink_response_radius(&tester), Some(test_splash_radius));
            build_frame_with_child(&mut tester, Some(test_splash_radius));
            assert_eq!(ink_well_radius(&tester), Some(test_splash_radius));
        });
    }

    #[test]
    fn can_override_menu_size_constraints() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let button_key = key.clone();
            centered_in_scaffold(&mut tester, m3, move || {
                PopupMenuButton::new(|_app, _context| vec![item("value".to_owned(), "Item 0")])
                    .key(button_key.into())
                    .constraints(BoxConstraints::new().min_width(500.0))
                    .into_widget()
            });

            // Show the menu
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            let item = rect_with_text::<PopupMenuItem<String>>(&tester, "Item 0");
            assert_eq!(item.height(), 48.0);
            assert_eq!(item.width(), 500.0);
        });
    }

    #[test]
    fn can_change_menu_position_and_offset() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_menu_button = |position: Option<PopupMenuPosition>, offset: Offset| {
                let button = PopupMenuButton::new(|_app, _context| {
                    vec![PopupMenuEntryRef::new(
                        PopupMenuItem::new(Some(Builder::new(|_app, _context| {
                            Text::new("AAA").into_widget()
                        })))
                        .value(1),
                    )]
                })
                .offset(offset);
                match position {
                    Some(position) => button.position(position),
                    None => button,
                }
            };
            for (position, offset, expected) in [
                // Popup menu with `MenuPosition.over (default) with default offset`.
                (
                    Some(PopupMenuPosition::Over),
                    Offset::ZERO,
                    Offset::new(8.0, 8.0),
                ),
                // Popup menu with `MenuPosition.under`(custom) with default offset`.
                (
                    Some(PopupMenuPosition::Under),
                    Offset::ZERO,
                    Offset::new(8.0, 40.0),
                ),
                // Popup menu with `MenuPosition.over (default) with custom offset`.
                (None, Offset::new(0.0, 50.0), Offset::new(8.0, 50.0)),
                // Popup menu with `MenuPosition.under (custom) with custom offset`.
                (
                    Some(PopupMenuPosition::Under),
                    Offset::new(0.0, 50.0),
                    Offset::new(8.0, 90.0),
                ),
            ] {
                let button = build_menu_button(position, offset);
                pump_app(&mut tester, m3, move |_app| {
                    crate::scaffold::Scaffold::new()
                        .body(Material::new().child(button))
                        .into_widget()
                });

                // Open the popup menu.
                let icon_button = only(tester.find_by_type::<IconButton>());
                let center = tester.get_rect(tester.render_box_of(icon_button)).center();
                tester.tap(center, 1);
                tester.pump_and_settle();

                assert_offset_near(rect_of::<PopupMenu<i32>>(&tester).top_left(), expected);

                // Close the popup menu.
                tester.tap(Offset::ZERO, 1);
                tester.pump(Duration::ZERO);
            }
        });
    }

    #[test]
    fn popup_menu_button_icon_inherits_icon_themes_size() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_popup_menu = |tester: &mut WidgetTester,
                                    theme_icon_size: Option<f64>,
                                    icon_size: Option<f64>| {
                centered_in_scaffold(
                    tester,
                    move |app| {
                        let mut icon_theme = waft_widgets::IconThemeData::new();
                        icon_theme.size = theme_icon_size;
                        ThemeData::builder().icon_theme(icon_theme).build(app)
                    },
                    move || {
                        let mut button = PopupMenuButton::new(|_app, _context| {
                            vec![item("value".to_owned(), "Item 0")]
                        });
                        button.icon_size = icon_size;
                        button.into_widget()
                    },
                );
            };
            let icon_size = |tester: &WidgetTester| {
                let icon = tester.find_by_icon(&Icons::more_vert());
                tester.get_rect(tester.render_box_of(icon)).size()
            };

            // Popup menu with default icon size.
            build_popup_menu(&mut tester, None, None);
            // Default PopupMenuButton icon size is 24.0.
            assert_eq!(icon_size(&tester), Size::new(24.0, 24.0));

            // Popup menu with custom theme icon size.
            build_popup_menu(&mut tester, Some(30.0), None);
            tester.pump_and_settle();
            // PopupMenuButton icon inherits IconTheme's size.
            assert_eq!(icon_size(&tester), Size::new(30.0, 30.0));

            // Popup menu with custom icon size.
            build_popup_menu(&mut tester, Some(30.0), Some(50.0));
            tester.pump_and_settle();
            // PopupMenuButton icon size overrides IconTheme's size.
            assert_eq!(icon_size(&tester), Size::new(50.0, 50.0));
        });
    }

    #[test]
    fn popup_menu_clip_behavior() {
        // Regression test for https://github.com/flutter/flutter/issues/107215
        with_main_thread_stack(|| {
            let mut tester = tester();
            let popup_button_key = GlobalKey::new();
            let radius = 20.0;
            for clip_behavior in [Clip::None, Clip::HardEdge] {
                let key = popup_button_key.clone();
                centered_in_scaffold(&mut tester, m3, move || {
                    PopupMenuButton::new(|_app, _context| vec![item("value".to_owned(), "Item 0")])
                        .key(key.into())
                        .shape(waft_painting::RoundedRectangleBorder::new(
                            waft_painting::BorderSide::NONE,
                            BorderRadiusGeometry::circular(radius),
                        ))
                        .clip_behavior(clip_behavior)
                        .into_widget()
                });

                // Open the popup to build and show the menu contents.
                tap_key(&mut tester, &popup_button_key);
                tester.pump_and_settle();

                let materials = tester.find_by_type::<Material>();
                let material = *materials.last().expect("the menu's Material");
                assert_eq!(
                    tester.widget(material, |material: &Material| material.clip_behavior),
                    clip_behavior
                );

                // Close the popup menu.
                tester.tap(Offset::ZERO, 1);
                tester.pump_and_settle();
            }
        });
    }

    #[test]
    fn popup_menu_scrollbar_inherits_scrollbar_theme() {
        use crate::scrollbar::Scrollbar;
        use crate::scrollbar::tests::painted;
        use crate::scrollbar_theme::{ScrollbarTheme, ScrollbarThemeData};
        use crate::test_support::Drawn;

        fn menu_button(key: &GlobalKey) -> PopupMenuButton<()> {
            PopupMenuButton::new(|_app, _context| {
                vec![PopupMenuEntryRef::new(
                    PopupMenuItem::<()>::new(Some(Text::new("Example"))).height(1000.0),
                )]
            })
            .key(key.clone().into())
        }

        /// Dart's `expect(find.byType(Scrollbar), findsOneWidget)` and
        /// `expect(find.byType(Scrollbar), paints..rrect(color: thumb))`.
        fn expect_one_scrollbar_painting(tester: &WidgetTester, thumb: u32) {
            only(tester.find_by_type::<Scrollbar>());
            assert!(
                painted(tester)
                    .iter()
                    .any(|drawn| matches!(drawn, Drawn::Path(color, _) if *color == thumb)),
                "the scrollbar paints a {thumb:#x} rrect"
            );
        }

        for platform in [
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ] {
            with_main_thread_stack(move || {
                let mut tester = tester_on(platform);
                let popup_button_key = GlobalKey::new();
                let scrollbar_theme = ScrollbarThemeData::new()
                    .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                        Color::new(0xffff0000),
                    )))
                    .thumb_visibility(<dyn WidgetStateProperty<Option<bool>>>::all(Some(true)));
                let theme = |scrollbar_theme: ScrollbarThemeData| {
                    move |app: &App| {
                        ThemeData::builder()
                            .scrollbar_theme(scrollbar_theme)
                            .build(app)
                    }
                };

                let key = popup_button_key.clone();
                pump_app(&mut tester, theme(scrollbar_theme.clone()), move |_app| {
                    Material::new()
                        .child(Column::new().children([menu_button(&key).into_widget()]))
                        .into_widget()
                });

                tap_key(&mut tester, &popup_button_key);
                tester.pump_and_settle();

                // Test Scrollbar thumb color.
                expect_one_scrollbar_painting(&tester, 0xffff0000);

                // Close the menu.
                tester.tap(Offset::new(20.0, 20.0), 1);
                tester.pump_and_settle();

                // Test local ScrollbarTheme overrides global ScrollbarTheme.
                let key = popup_button_key.clone();
                let local_theme = scrollbar_theme.copy_with().thumb_color(
                    <dyn WidgetStateProperty<Option<Color>>>::all(Some(Color::new(0xff0000ff))),
                );
                pump_app(&mut tester, theme(scrollbar_theme), move |_app| {
                    Material::new()
                        .child(Column::new().children([
                            ScrollbarTheme::new(local_theme, menu_button(&key)).into_widget(),
                        ]))
                        .into_widget()
                });
                tap_key(&mut tester, &popup_button_key);
                tester.pump_and_settle();

                // Scrollbar thumb color should be updated.
                expect_one_scrollbar_painting(&tester, 0xff0000ff);
            });
        }
    }

    #[test]
    fn uses_closed_loop_focus_traversal() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let popup_button_key = GlobalKey::new();
            let key = popup_button_key.clone();
            centered_in_scaffold(&mut tester, m3, move || {
                PopupMenuButton::new(|_app, _context| {
                    vec![item("a".to_owned(), "A"), item("b".to_owned(), "B")]
                })
                .key(key.into())
                .into_widget()
            });

            // Open the popup to build and show the menu contents.
            tap_key(&mut tester, &popup_button_key);
            tester.pump_and_settle();

            let node = |tester: &WidgetTester, text: &str| {
                let element = only(tester.find_onstage_text(text));
                waft_widgets::Focus::of(&mut tester.cell.borrow_mut(), element, false, false)
            };
            let invoke = |tester: &mut WidgetTester, intent: &dyn waft_widgets::Intent| {
                let result = {
                    let mut app = tester.cell.borrow_mut();
                    let context = waft_widgets::primary_focus(&mut app)
                        .and_then(|focus| focus.context(&app))
                        .expect("a focused node with a context");
                    waft_widgets::Actions::invoke(&mut app, context, intent)
                };
                tester.pump(Duration::ZERO);
                result
                    .and_then(|result| result.downcast_ref::<bool>().copied())
                    .expect("the focus actions answer whether focus moved")
            };
            let has_focus = |tester: &WidgetTester, text: &str| {
                node(tester, text).has_focus(&mut tester.cell.borrow_mut())
            };

            // Start at A
            let a = node(&tester, "A");
            a.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            assert!(has_focus(&tester, "A"));
            assert!(!has_focus(&tester, "B"));

            for (intent, a_focused) in [
                // A -> B
                (
                    &waft_widgets::NextFocusIntent as &dyn waft_widgets::Intent,
                    false,
                ),
                // B -> A (wrap around)
                (&waft_widgets::NextFocusIntent, true),
                // B <- A
                (&waft_widgets::PreviousFocusIntent, false),
                // A <- B (wrap around)
                (&waft_widgets::PreviousFocusIntent, true),
            ] {
                assert!(invoke(&mut tester, intent));
                assert_eq!(has_focus(&tester, "A"), a_focused);
                assert_eq!(has_focus(&tester, "B"), !a_focused);
            }
        });
    }

    #[test]
    fn popup_menu_with_route_settings() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Material::new()
                    .child(
                        Center::new().child(crate::elevated_button::ElevatedButton::new(
                            None,
                            Some(Text::new("Go")),
                        )),
                    )
                    .into_widget()
            });

            let context = only(tester.find_onstage_text("Go"));
            let example_setting = RouteSettings::default().name("simple");

            drop(show_menu_at_fill::<()>(
                &mut tester.cell.borrow_mut(),
                context,
                vec![PopupMenuEntryRef::new(PopupMenuItem::new(Some(Text::new(
                    "foo",
                ))))],
                Some(RouteSettingsRef::Settings(example_setting)),
            ));

            // Dart's `_ClosureNavigatorObserver` records the settings of the route on top; the
            // settings of the route the item is in say the same.
            tester.pump_and_settle();
            assert_eq!(tester.find_onstage_text("foo").len(), 1);
            let foo = only(tester.find_onstage_text("foo"));
            let settings =
                waft_widgets::AnyModalRoute::settings_of(&mut tester.cell.borrow_mut(), foo);
            assert_eq!(
                settings.as_ref().and_then(|settings| settings.name()),
                Some("simple")
            );

            tap_text(&mut tester, "foo");
            tester.pump_and_settle();
            let settings =
                waft_widgets::AnyModalRoute::settings_of(&mut tester.cell.borrow_mut(), context);
            assert_eq!(
                settings.as_ref().and_then(|settings| settings.name()),
                Some("/")
            );
        });
    }

    #[test]
    fn popup_menu_is_positioned_under_the_child() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let popup_button_key = GlobalKey::new();
            let child_key = GlobalKey::new();
            let (key, child) = (popup_button_key.clone(), child_key.clone());
            pump_app(&mut tester, m3, move |_app| {
                Material::new()
                    .child(Column::new().children(vec![
                        PopupMenuButton::<()>::new(|_app, _context| {
                            vec![PopupMenuEntryRef::new(PopupMenuItem::new(Some(Text::new(
                                "Example",
                            ))))]
                        })
                        .key(key.into())
                        .position(PopupMenuPosition::Under)
                        .child(SizedBox::new().key(child.into()).height(50.0).width(50.0))
                        .into_widget(),
                    ]))
                    .into_widget()
            });

            tap_key(&mut tester, &popup_button_key);
            tester.pump_and_settle();

            let child_bottom_left = rect_of_key(&tester, &child_key).bottom_left();
            // Dart's `find.bySemanticsLabel('Popup menu')`: the menu's `Semantics` node, the
            // `_PopupMenu`'s box.
            let menu_top_left = rect_of::<PopupMenu<()>>(&tester).top_left();
            assert_offset_near(child_bottom_left, menu_top_left);
        });
    }

    #[test]
    fn popup_menu_item_on_tap_should_be_calling_after_navigator_pop() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                crate::scaffold::Scaffold::new()
                    .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                        crate::app_bar::AppBar::new().actions(vec![
                            PopupMenuButton::new(|_app, context| {
                                vec![PopupMenuEntryRef::new(
                                    PopupMenuItem::new(Some(Text::new("ACTION")))
                                        .on_tap(Listener::new(move |app| {
                                            drop(crate::bottom_sheet::show_modal_bottom_sheet(
                                                app,
                                                context,
                                                Rc::new(|_app, _context| {
                                                    SizedBox::new()
                                                        .height(200.0)
                                                        .child(
                                                            Center::new().child(Text::new(
                                                                "ModalBottomSheet",
                                                            )),
                                                        )
                                                        .into_widget()
                                                }),
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                false,
                                                9.0 / 16.0,
                                                false,
                                                true,
                                                true,
                                                None,
                                                false,
                                                None,
                                                None,
                                                None,
                                                None,
                                            ));
                                        }))
                                        .value(10),
                                )]
                            })
                            .into_widget(),
                        ]),
                    ))
                    .into_widget()
            });

            let button = only(tester.find_by_type::<PopupMenuButton<i32>>());
            let center = tester.get_rect(tester.render_box_of(button)).center();
            tester.tap(center, 1);
            tester.pump_and_settle();

            tap_text(&mut tester, "ACTION");
            tester.pump_and_settle();

            // Verify that the ModalBottomSheet is displayed
            assert_eq!(tester.find_onstage_text("ModalBottomSheet").len(), 1);
        });
    }

    /// Dart's `_labelStyle(tester, label)`: the style of the `RichText` below the text.
    fn label_style(tester: &WidgetTester, label: &str) -> TextStyle {
        tester.glyph_style(only(tester.find_onstage_text(label)))
    }

    /// The menu's `Material`, the last one: Dart's `find.byType(Material).last`.
    fn last_material_size(tester: &WidgetTester) -> Size {
        let materials = tester.find_by_type::<Material>();
        let material = *materials.last().expect("a Material");
        tester.get_rect(tester.render_box_of(material)).size()
    }

    /// An app bar whose action is a button keyed `key` showing `items`.
    fn app_bar_button<T: Clone + PartialEq + 'static>(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        key: &GlobalKey,
        items: impl Fn() -> Vec<PopupMenuEntryRef<T>> + 'static,
    ) {
        let key = key.clone();
        pump_app(tester, theme, move |_app| {
            crate::scaffold::Scaffold::new()
                .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                    crate::app_bar::AppBar::new().actions(vec![
                        PopupMenuButton::new(move |_app, _context| items())
                            .key(key.into())
                            .into_widget(),
                    ]),
                ))
                .into_widget()
        });
    }

    fn custom_text_style() -> TextStyle {
        TextStyle::new()
            .font_size(20.0)
            .font_weight(waft_embedder::FontWeight::BOLD)
            .font_style(waft_embedder::FontStyle::Italic)
    }

    /// Dart's `const CheckedPopupMenuItem<void>(child: Text('Item 1'))` and its checked
    /// twin; Dart's second is a `CheckedPopupMenuItem<int>` in a list of `void` items, which
    /// a Rust list of one item type cannot hold.
    fn checked_items(
        label_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,
    ) -> Vec<PopupMenuEntryRef<()>> {
        let with_style = |item: CheckedPopupMenuItem<()>| match &label_text_style {
            Some(label_text_style) => item.label_text_style(Rc::clone(label_text_style)),
            None => item,
        };
        vec![
            PopupMenuEntryRef::new(with_style(
                CheckedPopupMenuItem::new().child(Text::new("Item 1")),
            )),
            PopupMenuEntryRef::new(with_style(
                CheckedPopupMenuItem::new()
                    .checked(true)
                    .child(Text::new("Item 2")),
            )),
        ]
    }

    #[test]
    fn material3_checked_popup_menu_item_label_text_style_uses_correct_text_style() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            app_bar_button(&mut tester, m3, &key, || checked_items(None));

            // Show the menu
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            // Test default text style.
            let on_surface = m3(&tester.cell.borrow()).color_scheme.on_surface;
            assert_eq!(label_style(&tester, "Item 1").font_size, Some(14.0));
            assert_eq!(
                label_style(&tester, "Item 1").color.as_deref().copied(),
                Some(on_surface)
            );

            // Close the menu.
            tester.tap(Offset::new(20.0, 20.0), 1);
            tester.pump_and_settle();

            // Test custom text theme.
            app_bar_button(
                &mut tester,
                |app| {
                    m3(app)
                        .copy_with()
                        .text_theme(
                            crate::text_theme::TextTheme::new().label_large(custom_text_style()),
                        )
                        .build()
                },
                &key,
                || checked_items(None),
            );

            // Show the menu.
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            // Test custom text theme.
            let style = label_style(&tester, "Item 1");
            assert_eq!(style.font_size, custom_text_style().font_size);
            assert_eq!(style.font_weight, custom_text_style().font_weight);
            assert_eq!(style.font_style, custom_text_style().font_style);
        });
    }

    #[test]
    fn checked_popup_menu_item_label_text_style_resolve_material_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let label_text_style =
                <dyn WidgetStateProperty<Option<TextStyle>>>::resolve_with(|states| {
                    if states.contains(&WidgetState::Selected) {
                        return Some(
                            TextStyle::new()
                                .color(crate::colors::Colors::RED.primary())
                                .font_size(24.0),
                        );
                    }
                    Some(
                        TextStyle::new()
                            .color(crate::colors::Colors::AMBER.primary())
                            .font_size(20.0),
                    )
                });
            let style = Rc::clone(&label_text_style);
            app_bar_button(&mut tester, m3, &key, move || {
                checked_items(Some(Rc::clone(&style)))
            });

            // Show the menu.
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            let mut selected = WidgetStates::new();
            selected.insert(WidgetState::Selected);
            let merged = |tester: &WidgetTester, label: &str| {
                let style = label_style(tester, label);
                (style.color.as_deref().copied(), style.font_size)
            };
            let expected = |states: &WidgetStates| {
                let style = label_text_style.resolve(states).expect("a style");
                (style.color.as_deref().copied(), style.font_size)
            };
            assert_eq!(merged(&tester, "Item 1"), expected(&WidgetStates::new()));
            assert_eq!(merged(&tester, "Item 2"), expected(&selected));
        });
    }

    /// Dart's `getItemSafeArea(label).minimum` of the two 'overrides redundant
    /// ListTile.contentPadding' cases: the `ListTile`'s `SafeArea` above the label.
    fn item_safe_area_minimum(tester: &WidgetTester, label: &str) -> EdgeInsets {
        let safe_area = widget_with_text::<waft_widgets::SafeArea>(tester, label);
        tester.widget(safe_area, |safe_area: &waft_widgets::SafeArea| {
            safe_area.minimum
        })
    }

    #[test]
    fn checked_popup_menu_item_overrides_redundant_list_tile_content_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            keyed_text_button(&mut tester, m2, &key, || {
                vec![
                    PopupMenuEntryRef::new(
                        CheckedPopupMenuItem::new()
                            .value("0".to_owned())
                            .child(Text::new("Item 0")),
                    ),
                    PopupMenuEntryRef::new(
                        CheckedPopupMenuItem::new()
                            .value("1".to_owned())
                            .checked(true)
                            .child(Text::new("Item 1")),
                    ),
                ]
            });

            // Show the menu.
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            assert_eq!(item_safe_area_minimum(&tester, "Item 0"), EdgeInsets::ZERO);
            assert_eq!(item_safe_area_minimum(&tester, "Item 1"), EdgeInsets::ZERO);
        });
    }

    #[test]
    fn popup_menu_item_overrides_redundant_list_tile_content_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            keyed_text_button(&mut tester, m2, &key, || {
                vec![
                    PopupMenuEntryRef::new(
                        PopupMenuItem::new(Some(ListTile::new().title(Text::new("Item 0"))))
                            .value("0".to_owned())
                            .enabled(false),
                    ),
                    PopupMenuEntryRef::new(
                        PopupMenuItem::new(Some(ListTile::new().title(Text::new("Item 1"))))
                            .value("1".to_owned()),
                    ),
                ]
            });

            // Show the menu.
            tap_key(&mut tester, &key);
            tester.pump_and_settle();

            assert_eq!(item_safe_area_minimum(&tester, "Item 0"), EdgeInsets::ZERO);
            assert_eq!(item_safe_area_minimum(&tester, "Item 1"), EdgeInsets::ZERO);
        });
    }

    fn text_and_list_tile_items() -> Vec<PopupMenuEntryRef<String>> {
        vec![
            // Popup menu item with a Text widget.
            item("0".to_owned(), "Item 0"),
            // Popup menu item with a ListTile widget.
            PopupMenuEntryRef::new(
                PopupMenuItem::new(Some(ListTile::new().title(Text::new("Item 1"))))
                    .value("1".to_owned()),
            ),
        ]
    }

    /// Dart's 'Material3 - PopupMenuItem overrides ListTile.titleTextStyle' and its
    /// Material 2 twin: the default font size and colour, then the text theme's style that
    /// `custom` puts in its place.
    fn item_overrides_title_text_style(
        theme: fn(&App) -> ThemeData,
        default_font_size: f64,
        default_color: impl Fn(&ThemeData) -> Option<Color>,
        custom: fn(crate::text_theme::TextTheme, TextStyle) -> crate::text_theme::TextTheme,
    ) {
        let mut tester = tester();
        let key = GlobalKey::new();
        keyed_text_button(&mut tester, theme, &key, text_and_list_tile_items);

        // Show the menu.
        tap_key(&mut tester, &key);
        tester.pump_and_settle();

        let expected_color = default_color(&theme(&tester.cell.borrow()));
        for label in ["Item 0", "Item 1"] {
            let style = label_style(&tester, label);
            assert_eq!(style.font_size, Some(default_font_size), "{label}");
            assert_eq!(style.color.as_deref().copied(), expected_color, "{label}");
        }

        // Close the menu.
        tester.tap(Offset::new(20.0, 20.0), 1);
        tester.pump_and_settle();

        // Test custom text theme.
        keyed_text_button(
            &mut tester,
            move |app| {
                theme(app)
                    .copy_with()
                    .text_theme(custom(
                        crate::text_theme::TextTheme::new(),
                        custom_text_style(),
                    ))
                    .build()
            },
            &key,
            text_and_list_tile_items,
        );

        // Show the menu.
        tap_key(&mut tester, &key);
        tester.pump_and_settle();

        for label in ["Item 0", "Item 1"] {
            let style = label_style(&tester, label);
            assert_eq!(style.font_size, custom_text_style().font_size, "{label}");
            assert_eq!(
                style.font_weight,
                custom_text_style().font_weight,
                "{label}"
            );
            assert_eq!(style.font_style, custom_text_style().font_style, "{label}");
        }
    }

    #[test]
    fn material3_popup_menu_item_overrides_list_tile_title_text_style() {
        with_main_thread_stack(|| {
            item_overrides_title_text_style(
                m3,
                14.0,
                |theme| Some(theme.color_scheme.on_surface),
                crate::text_theme::TextTheme::label_large,
            );
        });
    }

    #[test]
    fn material2_popup_menu_item_overrides_list_tile_title_text_style() {
        with_main_thread_stack(|| {
            item_overrides_title_text_style(
                m2,
                16.0,
                |theme| {
                    theme
                        .text_theme
                        .title_medium
                        .as_ref()
                        .and_then(|style| style.color.as_deref().copied())
                },
                crate::text_theme::TextTheme::title_medium,
            );
        });
    }

    #[test]
    fn checked_popup_menu_item_on_tap_callback_is_called_when_defined() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (count, on_tap) = counter();
            tester.pump_widget(test_app(TextDirection::Ltr, move || {
                let on_tap = on_tap.clone();
                Material::new()
                    .child(
                        PopupMenuButton::new(move |_app, _context| {
                            vec![
                                PopupMenuEntryRef::new(
                                    CheckedPopupMenuItem::new()
                                        .on_tap(on_tap.clone())
                                        .value("item1".to_owned())
                                        .child(Text::new("Item with onTap")),
                                ),
                                PopupMenuEntryRef::new(
                                    CheckedPopupMenuItem::new()
                                        .value("item2".to_owned())
                                        .child(Text::new("Item without onTap")),
                                ),
                            ]
                        })
                        .child(Text::new("button")),
                    )
                    .into_widget()
            }));

            for (label, expected) in [("Item with onTap", 1), ("Item without onTap", 1)] {
                tap_text(&mut tester, "button");
                tester.pump_and_settle();
                let center =
                    rect_with_text::<CheckedPopupMenuItem<String>>(&tester, label).center();
                tester.tap(center, 1);
                tester.pump_and_settle();
                assert_eq!(count.get(), expected);
            }
        });
    }

    /// Dart's 'PopupMenuButton uses root navigator if useRootNavigator is true' and 'does not
    /// use root navigator if useRootNavigator is false': whether the menu's route is the
    /// nested navigator's, which Dart's `MenuObserver`s count.
    fn button_menu_is_in_nested_navigator(use_root_navigator: bool) -> bool {
        let mut tester = tester();
        let nested_key = GlobalKey::new();
        let nested = nested_key.clone();
        pump_app(&mut tester, m3, move |_app| {
            Navigator::new()
                .key(nested.into())
                .on_generate_route(move |app, _settings| {
                    let route = MaterialPageRoute::new(
                        app,
                        Rc::new(move |_app, _context| {
                            Material::new()
                                .child(
                                    PopupMenuButton::new(|_app, _context| {
                                        vec![
                                            PopupMenuEntryRef::new(
                                                CheckedPopupMenuItem::new()
                                                    .value("item1".to_owned())
                                                    .child(Text::new("item 1")),
                                            ),
                                            PopupMenuEntryRef::new(
                                                CheckedPopupMenuItem::new()
                                                    .value("item2".to_owned())
                                                    .child(Text::new("item 2")),
                                            ),
                                        ]
                                    })
                                    .use_root_navigator(use_root_navigator)
                                    .child(Text::new("button")),
                                )
                                .into_widget()
                        }),
                    );
                    Some(Route::as_route(route))
                })
                .into_widget()
        });

        // Open the dialog.
        tap_text(&mut tester, "button");
        tester.pump(Duration::ZERO);

        let menu = only(tester.find_by_type::<PopupMenu<String>>());
        let nested = nested_key
            .current_context(&tester.cell.borrow())
            .expect("the nested navigator is mounted");
        tester.ancestors(menu).contains(&nested)
    }

    #[test]
    fn popup_menu_button_uses_root_navigator_if_use_root_navigator_is_true() {
        with_main_thread_stack(|| assert!(!button_menu_is_in_nested_navigator(true)));
    }

    #[test]
    fn popup_menu_button_does_not_use_root_navigator_if_use_root_navigator_is_false() {
        with_main_thread_stack(|| assert!(button_menu_is_in_nested_navigator(false)));
    }

    #[test]
    fn override_popup_menu_animation_using_animation_style() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let target_key = GlobalKey::new();
            let build_popup_menu =
                |tester: &mut WidgetTester, animation_style: Option<AnimationStyle>| {
                    let key = target_key.clone();
                    pump_app(tester, m3, move |_app| {
                        let mut button = PopupMenuButton::new(|_app, _context| {
                            vec![item(1, "One"), item(2, "Two"), item(3, "Three")]
                        })
                        .key(key.into());
                        button.pop_up_animation_style = animation_style.clone();
                        Material::new()
                            .child(Center::new().child(button))
                            .into_widget()
                    });
                };
            let assert_size_within = |tester: &WidgetTester, expected: Size| {
                let size = last_material_size(tester);
                assert!(
                    (size - expected).distance() <= 0.1,
                    "{size:?} is not within 0.1 of {expected:?}"
                );
            };

            // Test default animation.
            build_popup_menu(&mut tester, None);

            tap_key(&mut tester, &target_key);
            tester.pump(Duration::ZERO);
            // Advance the animation by 1/3 of its duration.
            tester.pump(Duration::from_millis(100));
            assert_size_within(&tester, Size::new(112.0, 80.0));

            // Advance the animation by 2/3 of its duration.
            tester.pump(Duration::from_millis(100));
            assert_size_within(&tester, Size::new(112.0, 160.0));

            // Advance the animation to the end.
            tester.pump_and_settle();
            assert_size_within(&tester, Size::new(112.0, 160.0));

            // Tap outside to dismiss the menu.
            tester.tap(Offset::new(20.0, 20.0), 1);
            tester.pump_and_settle();

            // Override the animation duration.
            build_popup_menu(
                &mut tester,
                Some(AnimationStyle::new(None, Some(Duration::ZERO), None, None)),
            );

            tap_key(&mut tester, &target_key);
            tester.pump(Duration::ZERO);
            // Advance the animation by 1 millisecond.
            tester.pump(Duration::from_millis(1));
            assert_size_within(&tester, Size::new(112.0, 160.0));

            // Tap outside to dismiss the menu.
            tester.tap(Offset::new(20.0, 20.0), 1);
            tester.pump_and_settle();

            // Override the animation curve.
            build_popup_menu(
                &mut tester,
                Some(AnimationStyle::new(
                    Some(crate::motion::Easing::emphasized_accelerate()),
                    None,
                    None,
                    None,
                )),
            );

            tap_key(&mut tester, &target_key);
            tester.pump(Duration::ZERO);
            // Advance the animation by 1/3 of its duration.
            tester.pump(Duration::from_millis(100));
            assert_size_within(&tester, Size::new(32.4, 15.4));

            // Advance the animation by 2/3 of its duration.
            tester.pump(Duration::from_millis(100));
            assert_size_within(&tester, Size::new(112.0, 72.2));

            // Advance the animation to the end.
            tester.pump_and_settle();
            assert_size_within(&tester, Size::new(112.0, 160.0));
        });
    }

    /// Dart's `tester.scrollUntilVisible(finder, delta)` on the menu's scroll view: drags it
    /// a step at a time until `text` is inside the view.
    fn scroll_menu_until_visible(tester: &mut WidgetTester, text: &str, delta: f64) {
        for _ in 0..50 {
            let viewport = scroll_view_rect(tester);
            let target = text_rect(tester, text);
            if target.top >= viewport.top && target.bottom <= viewport.bottom {
                return;
            }
            let step = if target.top < viewport.top {
                delta
            } else {
                -delta
            };
            let start = viewport.center();
            tester.drag_from(
                start,
                Offset::new(
                    0.0,
                    step.clamp(-viewport.height() / 2.0, viewport.height() / 2.0),
                ),
                1,
            );
            tester.pump(Duration::ZERO);
        }
        panic!("{text} did not come into view");
    }

    #[test]
    fn popup_menu_button_scrolls_initial_value_selected_value_to_visible() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let length = 50;
            let selected_value = length - 1;
            pump_app(&mut tester, m3, move |_app| {
                crate::scaffold::Scaffold::new()
                    .body(
                        Align::new()
                            .alignment(AlignmentGeometry::BOTTOM_CENTER)
                            .child(
                                PopupMenuButton::new(move |_app, _context| {
                                    (0..length)
                                        .map(|index| item(index, &format!("item #{index}")))
                                        .collect()
                                })
                                .pop_up_animation_style(AnimationStyle::NO_ANIMATION)
                                .initial_value(selected_value)
                                .child(Text::new("click here")),
                            ),
                    )
                    .into_widget()
            });
            tap_text(&mut tester, "click here");
            tester.pump(Duration::ZERO);

            // Set up finder and verify basic widget structure.
            assert_eq!(tester.find_onstage_text("item #49").len(), 1);

            // The initially selected menu item should be positioned on screen.
            let window_height = WidgetTester::SIZE.height();
            assert!(text_rect(&tester, "item #49").bottom <= window_height);

            // Select item 20.
            scroll_menu_until_visible(&mut tester, "item #20", 500.0);
            assert_eq!(tester.find_onstage_text("item #20").len(), 1);
            tap_text(&mut tester, "item #20");
            tester.pump(Duration::ZERO);

            // Open menu again.
            tap_text(&mut tester, "click here");
            tester.pump(Duration::ZERO);
            assert_eq!(tester.find_onstage_text("item #20").len(), 1);

            // The selected menu item should be positioned on screen.
            assert!(text_rect(&tester, "item #20").bottom <= window_height);
        });
    }

    #[test]
    fn popup_menu_button_properly_positions_a_constrained_size_popup() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let window_size = WidgetTester::SIZE;
            let length = 50;
            pump_app(&mut tester, m3, move |_app| {
                crate::scaffold::Scaffold::new()
                    .body(
                        Padding::new(EdgeInsetsGeometry::all(50.0)).child(
                            Align::new()
                                .alignment(AlignmentGeometry::BOTTOM_CENTER)
                                .child(
                                    PopupMenuButton::new(move |_app, _context| {
                                        (0..length)
                                            .map(|index| item(index, &format!("item #{index}")))
                                            .collect()
                                    })
                                    .constraints(
                                        BoxConstraints::new()
                                            .max_height(window_size.height() / 3.0),
                                    )
                                    .pop_up_animation_style(AnimationStyle::NO_ANIMATION)
                                    .initial_value(length - 1)
                                    .child(Text::new("click here")),
                                ),
                        ),
                    )
                    .into_widget()
            });
            tap_text(&mut tester, "click here");
            tester.pump(Duration::ZERO);

            // Set up finders and verify basic widget structure
            let button = only(tester.find_by_type::<PopupMenuButton<i32>>());
            assert_eq!(tester.find_onstage_text("item #49").len(), 1);
            let list_body = only(tester.find_by_type::<ListBody>());
            let list_viewport = tester.ancestor_of_type::<SingleChildScrollView>(list_body);

            // The button and the list viewport should overlap
            let button_bounds = tester.global_rect(tester.render_box_of(button));
            let list_viewport_bounds = tester.global_rect(tester.render_box_of(list_viewport));
            assert!(list_viewport_bounds.top <= window_size.height());
            assert!(list_viewport_bounds.bottom <= window_size.height());
            assert!(list_viewport_bounds.overlaps(button_bounds));
        });
    }

    #[test]
    fn popup_menu_button_honors_style() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                crate::scaffold::Scaffold::new()
                    .body(
                        PopupMenuButton::new(|_app, _context| vec![item(1, "One")]).style(
                            crate::button_style::ButtonStyle::new().icon_color(
                                <dyn WidgetStateProperty<Option<Color>>>::all(Some(
                                    crate::colors::Colors::RED.primary(),
                                )),
                            ),
                        ),
                    )
                    .into_widget()
            });
            let button = only(tester.find_by_type::<PopupMenuButton<i32>>());
            let rich_text = *tester
                .descendants_of_type::<waft_widgets::RichText>(button)
                .first()
                .expect("the icon's RichText");
            let color = tester.widget(rich_text, |rich_text: &waft_widgets::RichText| {
                rich_text
                    .text
                    .style()
                    .and_then(|style| style.color.as_deref().copied())
            });
            assert_eq!(color, Some(crate::colors::Colors::RED.primary()));
        });
    }

    #[test]
    fn popup_menu_childs_ink_well_border_radius() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let border_radius = BorderRadius::all(waft_embedder::Radius::circular(20.0));
            for radius in [None, Some(border_radius)] {
                centered_in_scaffold(&mut tester, m3, move || {
                    let mut button = PopupMenuButton::new(|_app, _context| {
                        vec![item("value".to_owned(), "Item 0")]
                    })
                    .child(
                        waft_widgets::Row::new()
                            .main_axis_size(waft_rendering::MainAxisSize::Min)
                            .children(vec![
                                Text::new("Pop up menu").into_widget(),
                                Icon::new(Some(Icons::arrow_drop_down())).into_widget(),
                            ]),
                    );
                    button.border_radius = radius;
                    button.into_widget()
                });
                tester.pump_and_settle();

                let ink_well = only(tester.find_by_type::<InkWell>());
                assert_eq!(
                    tester.widget(ink_well, |ink_well: &InkWell| ink_well
                        .ink_response
                        .border_radius),
                    radius
                );
            }
        });
    }

    #[test]
    fn popup_menu_button_respects_material_tap_target_size() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_size = 10.0;
            for (tap_target_size, expected) in [
                (MaterialTapTargetSize::Padded, Size::new(48.0, 48.0)),
                (
                    MaterialTapTargetSize::ShrinkWrap,
                    Size::new(button_size, button_size),
                ),
            ] {
                centered_in_scaffold(&mut tester, m3, move || {
                    PopupMenuButton::new(|_app, _context| vec![item("value".to_owned(), "Item 0")])
                        .style(
                            crate::button_style::ButtonStyle::new()
                                .tap_target_size(tap_target_size),
                        )
                        .child(SizedBox::new().height(button_size).width(button_size))
                        .into_widget()
                });
                tester.pump_and_settle();

                assert_eq!(rect_of::<InkWell>(&tester).size(), expected);
            }
        });
    }

    #[test]
    fn if_request_focus_is_false_the_original_focus_should_be_preserved_upon_menu_appearance() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let field_focus_node = waft_widgets::FocusNode::new(&mut tester.cell.borrow_mut());
            pump_app(&mut tester, m3, move |_app| {
                crate::scaffold::Scaffold::new()
                    .body(Column::new().children(vec![
                        crate::text_field::TextField::new()
                            .focus_node(field_focus_node)
                            .autofocus(true)
                            .into_widget(),
                        PopupMenuButton::new(|_app, _context| vec![item(1, "One")])
                            .style(crate::button_style::ButtonStyle::new().icon_color(
                                <dyn WidgetStateProperty<Option<Color>>>::all(Some(
                                    crate::colors::Colors::RED.primary(),
                                )),
                            ))
                            .request_focus(false)
                            .child(Text::new("click here"))
                            .into_widget(),
                    ]))
                    .into_widget()
            });
            tester.pump(Duration::ZERO);
            assert!(waft_widgets::FocusNodeLeaf::has_focus(
                field_focus_node,
                &mut tester.cell.borrow_mut()
            ));
            tap_text(&mut tester, "click here");
            tester.pump(Duration::ZERO);
            assert!(waft_widgets::FocusNodeLeaf::has_focus(
                field_focus_node,
                &mut tester.cell.borrow_mut()
            ));
        });
    }

    /// How many `Icon`s show `icon`: Dart's `find.byIcon(icon)` with `findsOneWidget` or
    /// `findsNothing`.
    fn icon_count(tester: &WidgetTester, icon: &waft_widgets::IconData) -> usize {
        tester
            .find_onstage_by_type::<Icon>()
            .into_iter()
            .filter(|element| {
                tester.widget(*element, |widget: &Icon| widget.icon.as_ref() == Some(icon))
            })
            .count()
    }

    #[test]
    fn popup_menu_button_is_horizontal_on_ios() {
        with_main_thread_stack(|| {
            let (mut tester, platform) = WidgetTester::sharing(
                waft_test::TestPlatform::new().on(waft_embedder::TargetPlatform::Android),
            );
            tester.install_platform_fonts();
            let build = |tester: &mut WidgetTester, target: waft_embedder::TargetPlatform| {
                platform.set_target(target);
                pump_app(tester, m3, |_app| {
                    crate::scaffold::Scaffold::new()
                        .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                            crate::app_bar::AppBar::new().actions(vec![
                                PopupMenuButton::new(|_app, _context| vec![item(1, "One")])
                                    .into_widget(),
                            ]),
                        ))
                        .into_widget()
                });
            };

            build(&mut tester, waft_embedder::TargetPlatform::Android);

            assert_eq!(icon_count(&tester, &Icons::more_vert()), 1);
            assert_eq!(icon_count(&tester, &Icons::more_horiz()), 0);

            for target in [
                waft_embedder::TargetPlatform::IOS,
                waft_embedder::TargetPlatform::MacOS,
            ] {
                build(&mut tester, target);
                tester.pump_and_settle(); // Run theme change animation.

                assert_eq!(icon_count(&tester, &Icons::more_vert()), 0);
                assert_eq!(icon_count(&tester, &Icons::more_horiz()), 1);
            }
        });
    }

    #[test]
    #[should_panic(expected = "You can only pass [child] or [icon], not both.")]
    fn popup_menu_button_fails_when_given_both_child_and_icon() {
        PopupMenuButton::new(|_app, _context| vec![item(1, "1")])
            .icon(Icon::new(Some(Icons::view_carousel())))
            .child(Text::new("heyo"))
            .into_widget();
    }

    #[test]
    fn popup_menu_button_creates_icon_button_when_given_an_icon() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                crate::scaffold::Scaffold::new()
                    .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                        crate::app_bar::AppBar::new().actions(vec![
                            PopupMenuButton::new(|_app, _context| vec![item(1, "1")])
                                .icon(Icon::new(Some(Icons::view_carousel())))
                                .into_widget(),
                        ]),
                    ))
                    .into_widget()
            });

            assert_eq!(tester.find_onstage_by_type::<IconButton>().len(), 1);
            assert_eq!(icon_count(&tester, &Icons::view_carousel()), 1);
        });
    }

    /// Dart's three 'PopupMenu positioning inside nested Overlay/Navigator' cases: the menu's
    /// top left, `find.bySemanticsLabel('Popup menu')` (the `_PopupMenu`'s box), is the
    /// button's.
    fn menu_top_left_is_the_buttons(tester: &mut WidgetTester, button_key: &GlobalKey) {
        tap_key(tester, button_key);
        tester.pump_and_settle();

        let button_top_left = rect_of_key(tester, button_key).top_left();
        assert_offset_near(
            rect_of::<PopupMenu<i32>>(tester).top_left(),
            button_top_left,
        );
    }

    fn show_menu_button(key: &GlobalKey, use_root_navigator: bool) -> WidgetRef {
        PopupMenuButton::new(|_app, _context| vec![item(1, "Item 1"), item(2, "Item 2")])
            .key(key.clone().into())
            .use_root_navigator(use_root_navigator)
            .child(Text::new("Show Menu"))
            .into_widget()
    }

    #[test]
    fn popup_menu_positioning_inside_nested_overlay() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let key = button_key.clone();
            pump_app(&mut tester, m3, move |app| {
                let body = crate::tooltip::tests::overlay(
                    app,
                    Center::new()
                        .child(show_menu_button(&key, false))
                        .into_widget(),
                );
                crate::scaffold::Scaffold::new()
                    .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                        crate::app_bar::AppBar::new().title(Text::new("Example")),
                    ))
                    .body(Padding::new(EdgeInsetsGeometry::all(8.0)).child(body))
                    .into_widget()
            });
            menu_top_left_is_the_buttons(&mut tester, &button_key);
        });
    }

    fn nested_navigator_case(use_root_navigator: bool) {
        let mut tester = tester();
        let button_key = GlobalKey::new();
        let key = button_key.clone();
        pump_app(&mut tester, m3, move |_app| {
            let key = key.clone();
            crate::scaffold::Scaffold::new()
                .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                    crate::app_bar::AppBar::new().title(Text::new("Example")),
                ))
                .body(Padding::new(EdgeInsetsGeometry::all(8.0)).child(
                    Navigator::new().on_generate_route(move |app, _settings| {
                        let key = key.clone();
                        let route = MaterialPageRoute::new(
                            app,
                            Rc::new(move |_app, _context| {
                                Padding::new(EdgeInsetsGeometry::all(8.0))
                                    .child(
                                        Center::new()
                                            .child(show_menu_button(&key, use_root_navigator)),
                                    )
                                    .into_widget()
                            }),
                        );
                        Some(Route::as_route(route))
                    }),
                ))
                .into_widget()
        });
        menu_top_left_is_the_buttons(&mut tester, &button_key);
    }

    #[test]
    fn popup_menu_positioning_inside_nested_navigator() {
        with_main_thread_stack(|| nested_navigator_case(false));
    }

    #[test]
    fn popup_menu_positioning_inside_nested_navigator_when_use_root_navigator() {
        with_main_thread_stack(|| nested_navigator_case(true));
    }

    #[test]
    fn popup_menu_button_with_route_settings() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let key = button_key.clone();
            pump_app(&mut tester, m3, move |_app| {
                crate::scaffold::Scaffold::new()
                    .body(
                        PopupMenuButton::new(|_app, _context| {
                            vec![item(1, "Item 1"), item(2, "Item 2")]
                        })
                        .key(key.into())
                        .route_settings(RouteSettingsRef::Settings(
                            RouteSettings::default().name("/popup"),
                        ))
                        .child(Text::new("Show Menu")),
                    )
                    .into_widget()
            });

            tap_key(&mut tester, &button_key);
            tester.pump_and_settle();

            // Dart's `_ClosureNavigatorObserver` records the settings of the route on top: the
            // menu's, which its items are in.
            let item = only(tester.find_onstage_text("Item 1"));
            let settings =
                waft_widgets::AnyModalRoute::settings_of(&mut tester.cell.borrow_mut(), item);
            assert_eq!(
                settings.as_ref().and_then(|settings| settings.name()),
                Some("/popup")
            );
        });
    }

    #[test]
    fn popup_menu_removes_media_query_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let popup_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
            let saved = Rc::clone(&popup_context);
            pump_app(&mut tester, m3, move |_app| {
                MediaQuery::new(
                    MediaQueryData::new().padding(EdgeInsets::all(50.0)),
                    Material::new().child(
                        PopupMenuButton::new(move |_app, context| {
                            saved.set(Some(context));
                            let saved = Rc::clone(&saved);
                            vec![PopupMenuEntryRef::new(
                                PopupMenuItem::new(Some(Builder::new(move |_app, context| {
                                    saved.set(Some(context));
                                    Text::new("AAA").into_widget()
                                })))
                                .value(1),
                            )]
                        })
                        .child(
                            SizedBox::new()
                                .height(100.0)
                                .width(100.0)
                                .child(Text::new("XXX")),
                        ),
                    ),
                )
                .into_widget()
            });

            tap_text(&mut tester, "XXX");

            tester.pump(Duration::ZERO);

            let context = popup_context.get().expect("the menu built");
            assert_eq!(
                MediaQuery::of(&mut tester.cell.borrow_mut(), context).padding,
                EdgeInsets::ZERO
            );
        });
    }

    /// Dart's `tester.widget<Container>(find.byType(Container))` of the open menu's divider.
    fn divider_container(
        divider: PopupMenuDivider,
    ) -> (Option<EdgeInsetsGeometry>, waft_painting::BoxDecoration) {
        let mut tester = tester();
        let divider = Rc::new(RefCell::new(Some(divider)));
        centered_in_scaffold(&mut tester, m3, move || {
            let divider = Rc::clone(&divider);
            PopupMenuButton::<String>::new(move |_app, _context| {
                let divider = divider.borrow_mut().take().unwrap_or_default();
                vec![PopupMenuEntryRef::new(divider)]
            })
            .on_selected(|_app, _result: String| {})
            .child(Text::new("Menu Button"))
            .into_widget()
        });

        let button = only(tester.find_by_type::<PopupMenuButton<String>>());
        let center = tester.get_rect(tester.render_box_of(button)).center();
        tester.tap(center, 1);
        tester.pump(Duration::ZERO);
        let container = only(tester.find_by_type::<Container>());
        tester.widget(container, |container: &Container| {
            (
                container.margin,
                container
                    .decoration
                    .as_ref()
                    .and_then(|decoration| {
                        decoration
                            .as_any()
                            .downcast_ref::<waft_painting::BoxDecoration>()
                    })
                    .expect("a BoxDecoration")
                    .clone(),
            )
        })
    }

    fn bottom_side(decoration: &waft_painting::BoxDecoration) -> waft_painting::BorderSide {
        decoration.border.as_ref().expect("a border").bottom()
    }

    #[test]
    fn popup_menu_divider_custom_thickness() {
        with_main_thread_stack(|| {
            let (_, decoration) = divider_container(PopupMenuDivider::new().thickness(5.0));
            assert_eq!(bottom_side(&decoration).width, 5.0);
        });
    }

    #[test]
    fn popup_menu_divider_custom_indent() {
        with_main_thread_stack(|| {
            let (margin, _) = divider_container(PopupMenuDivider::new().indent(5.0));
            match margin {
                Some(EdgeInsetsGeometry::Directional(margin)) => assert_eq!(margin.start, 5.0),
                other => panic!("an EdgeInsetsDirectional margin, not {other:?}"),
            }
        });
    }

    #[test]
    fn popup_menu_divider_custom_color() {
        with_main_thread_stack(|| {
            let deep_orange = crate::colors::Colors::DEEP_ORANGE.primary();
            let (_, decoration) = divider_container(PopupMenuDivider::new().color(deep_orange));
            assert_eq!(bottom_side(&decoration).color, deep_orange);
        });
    }

    #[test]
    fn popup_menu_divider_custom_end_indent() {
        with_main_thread_stack(|| {
            let (margin, _) = divider_container(PopupMenuDivider::new().end_indent(5.0));
            match margin {
                Some(EdgeInsetsGeometry::Directional(margin)) => assert_eq!(margin.end, 5.0),
                other => panic!("an EdgeInsetsDirectional margin, not {other:?}"),
            }
        });
    }

    #[test]
    fn popup_menu_divider_custom_radius() {
        with_main_thread_stack(|| {
            let (_, decoration) = divider_container(
                PopupMenuDivider::new()
                    .radius(BorderRadius::all(waft_embedder::Radius::circular(5.0))),
            );
            assert_eq!(
                decoration.border_radius,
                Some(BorderRadiusGeometry::from(BorderRadius::all(
                    waft_embedder::Radius::circular(5.0)
                )))
            );
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/171422.
    #[test]
    fn popup_menu_button_should_not_crash_when_being_hidden_immediately() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let show_popup_menu = Rc::new(Cell::new(true));
            let build_widget = |tester: &mut WidgetTester| {
                let show = Rc::clone(&show_popup_menu);
                pump_app(tester, m3, move |_app| {
                    let mut actions = Vec::new();
                    if show.get() {
                        let hide = Rc::clone(&show);
                        actions.push(
                            PopupMenuButton::new(|_app, _context| {
                                vec![
                                    item("add".to_owned(), "Add"),
                                    item("hide".to_owned(), "Hide"),
                                    item("delete".to_owned(), "Delete"),
                                ]
                            })
                            .on_selected(move |_app, value: String| {
                                if value == "hide" {
                                    hide.set(false);
                                }
                            })
                            .into_widget(),
                        );
                    }
                    let shown = if show.get() {
                        "PopupMenu is showing"
                    } else {
                        "PopupMenu is hidden"
                    };
                    crate::scaffold::Scaffold::new()
                        .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                            crate::app_bar::AppBar::new().actions(actions),
                        ))
                        .body(Text::new(format!("PopupMenuButton:{shown}")))
                        .into_widget()
                });
            };

            build_widget(&mut tester);

            // Find the PopupMenuButton.
            let button = only(tester.find_by_type::<PopupMenuButton<String>>());

            // Tap on PopupMenuButton.
            let center = tester.get_rect(tester.render_box_of(button)).center();
            tester.tap(center, 1);
            tester.pump_and_settle();

            // Tap on "Hide" menu item.
            tap_text(&mut tester, "Hide");

            // Rebuild the widget tree with the PopupMenuButton removed to trigger the bug.
            build_widget(&mut tester);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/43824.
    #[test]
    fn popup_menu_updates_when_popup_menu_theme_in_theme_changes() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = |elevation: f64, color: Color, shadow: Color, tint: Color, radius: f64| {
                PopupMenuThemeData::new()
                    .elevation(elevation)
                    .color(color)
                    .shadow_color(shadow)
                    .surface_tint_color(tint)
                    .shape(waft_painting::RoundedRectangleBorder::new(
                        waft_painting::BorderSide::NONE,
                        BorderRadiusGeometry::circular(radius),
                    ))
                    .menu_padding(EdgeInsetsGeometry::all(radius))
            };
            let colors = crate::colors::Colors::RED.primary();
            let popup_menu_theme1 = theme(
                10.0,
                colors,
                crate::colors::Colors::BLACK,
                crate::colors::Colors::WHITE,
                10.0,
            );
            let popup_menu_theme2 = theme(
                20.0,
                crate::colors::Colors::BLUE.primary(),
                crate::colors::Colors::WHITE,
                crate::colors::Colors::BLACK,
                20.0,
            );
            let build_app = |tester: &mut WidgetTester, popup_menu_theme: PopupMenuThemeData| {
                pump_app(
                    tester,
                    move |app| {
                        ThemeData::builder()
                            .popup_menu_theme(popup_menu_theme)
                            .build(app)
                    },
                    |_app| {
                        crate::scaffold::Scaffold::new()
                            .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                                crate::app_bar::AppBar::new().actions(vec![
                                    PopupMenuButton::new(|_app, _context| {
                                        vec![
                                            item("0".to_owned(), "Item 0"),
                                            item("1".to_owned(), "Item 1"),
                                        ]
                                    })
                                    .into_widget(),
                                ]),
                            ))
                            .into_widget()
                    },
                );
            };
            let check_popup_menu =
                |tester: &WidgetTester, popup_menu_theme: &PopupMenuThemeData| {
                    let materials = tester.find_by_type::<Material>();
                    let material = *materials.last().expect("the menu's Material");
                    tester.widget(material, |material: &Material| {
                        assert_eq!(Some(material.elevation), popup_menu_theme.elevation);
                        assert_eq!(material.color, popup_menu_theme.color);
                        assert_eq!(material.shadow_color, popup_menu_theme.shadow_color);
                        assert_eq!(
                            material.surface_tint_color,
                            popup_menu_theme.surface_tint_color
                        );
                        assert_eq!(material.shape, popup_menu_theme.shape);
                    });
                    assert_eq!(menu_padding(tester), popup_menu_theme.menu_padding);
                };

            // Show the menu with the first theme.
            build_app(&mut tester, popup_menu_theme1.clone());
            let button = only(tester.find_by_type::<PopupMenuButton<String>>());
            let center = tester.get_rect(tester.render_box_of(button)).center();
            tester.tap(center, 1);
            tester.pump_and_settle();

            check_popup_menu(&tester, &popup_menu_theme1);

            // Rebuild with the second theme.
            build_app(&mut tester, popup_menu_theme2.clone());
            tester.pump_and_settle();

            check_popup_menu(&tester, &popup_menu_theme2);
        });
    }

    #[test]
    fn checked_popup_menu_item_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            centered_in_scaffold(&mut tester, m3, || {
                SizedBox::shrink()
                    .child(CheckedPopupMenuItem::<String>::new().child(Text::new("X")))
                    .into_widget()
            });
            assert_eq!(
                rect_of::<CheckedPopupMenuItem<String>>(&tester).size(),
                Size::ZERO
            );
        });
    }

    #[test]
    fn popup_menu_item_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            centered_in_scaffold(&mut tester, m3, || {
                SizedBox::shrink()
                    .child(PopupMenuItem::<String>::new(Some(Text::new("X"))))
                    .into_widget()
            });
            assert_eq!(rect_of::<PopupMenuItem<String>>(&tester).size(), Size::ZERO);
        });
    }

    #[test]
    fn popup_menu_button_does_not_crash_at_zero_area() {
        // This test case only verifies the layout of the button itself, not the
        // overlay, because there doesn't seem to be a way to open the menu at zero
        // area. Though, this should be sufficient since the overlay has been verified
        // by similar tests for MenuAnchor and PopupMenuItem.
        with_main_thread_stack(|| {
            let mut tester = WidgetTester::with_physical_size(
                waft_test::TestPlatform::new().on(waft_embedder::TargetPlatform::Android),
                Size::ZERO,
            );
            tester.install_platform_fonts();
            pump_app(&mut tester, m3, |_app| {
                Center::new()
                    .child(
                        SizedBox::shrink().child(PopupMenuButton::new(|_app, _context| {
                            vec![item("X".to_owned(), "X")]
                        })),
                    )
                    .into_widget()
            });
            assert_eq!(
                rect_of::<PopupMenuButton<String>>(&tester).size(),
                Size::ZERO
            );
        });
    }

    #[test]
    fn popup_menu_divider_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(PopupMenuDivider::new()))
                    .into_widget()
            });
            assert_eq!(rect_of::<PopupMenuDivider>(&tester).size(), Size::ZERO);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/177003
    /// Dart's `find.bySemanticsLabel(popupMenuLabel)`: the label the menu's `Semantics` is
    /// built with, per host platform and theme platform.
    #[test]
    fn popup_menu_semantics_for_mismatched_platforms() {
        use waft_embedder::TargetPlatform;
        with_main_thread_stack(|| {
            for target in [
                TargetPlatform::Android,
                TargetPlatform::Fuchsia,
                TargetPlatform::IOS,
                TargetPlatform::Linux,
                TargetPlatform::MacOS,
                TargetPlatform::Windows,
            ] {
                let mut tester = tester_on(target);
                for theme_platform in [TargetPlatform::Android, TargetPlatform::IOS] {
                    pump_app(
                        &mut tester,
                        move |app| ThemeData::builder().platform(theme_platform).build(app),
                        |_app| {
                            crate::scaffold::Scaffold::new()
                                .body(Builder::new(|_app, context| {
                                    crate::outlined_button::OutlinedButton::new(
                                        Some(Listener::new(move |app| {
                                            drop(show_menu_at_fill::<()>(
                                                app,
                                                context,
                                                vec![PopupMenuEntryRef::new(PopupMenuItem::new(
                                                    Some(Text::new("foo")),
                                                ))],
                                                None,
                                            ));
                                        })),
                                        Some(Text::new("open")),
                                    )
                                    .into_widget()
                                }))
                                .into_widget()
                        },
                    );

                    tap_text(&mut tester, "open");
                    tester.pump_and_settle();

                    let menu = only(tester.find_by_type::<PopupMenu<()>>());
                    let label =
                        tester.widget(menu, |menu: &PopupMenu<()>| menu.semantic_label.clone());
                    match target {
                        // Apple platforms don't show label.
                        TargetPlatform::IOS | TargetPlatform::MacOS => assert_eq!(label, None),
                        // Non-Apple platforms show label.
                        _ => {
                            assert_eq!(label, Some(DefaultMaterialLocalizations.popup_menu_label()))
                        }
                    }

                    // Dismiss the popup.
                    let foo = only(tester.find_onstage_text("foo"));
                    pop(&mut tester, foo);
                    tester.pump_and_settle();
                }
            }
        });
    }

    /// Dart's collector frees the item fades `_PopupMenuState` makes once the menu closes;
    /// here the state frees them in its `dispose`.
    #[test]
    fn a_closed_menu_frees_the_item_fades_its_state_made() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let button_key = key.clone();
            pump_app(&mut tester, m3, move |_app| {
                Material::new()
                    .child(
                        PopupMenuButton::new(|_app, _context| vec![item(1, "One"), item(2, "Two")])
                            .key(button_key.into()),
                    )
                    .into_widget()
            });
            tap_key(&mut tester, &key);
            tester.pump_and_settle();
            let opacities = {
                let menu = only(tester.find_by_type::<PopupMenu<i32>>());
                let app = tester.cell.borrow();
                let state = menu
                    .state_handle::<PopupMenuState<i32>>(&app)
                    .expect("the menu is stateful");
                app.get(state).opacities.clone()
            };
            assert_eq!(opacities.len(), 2);

            tester.tap(Offset::ZERO, 1);
            tester.pump_and_settle();
            tester.cell.checkpoint();
            let app = tester.cell.borrow();
            assert!(opacities.iter().all(|opacity| !app.contains(*opacity)));
        });
    }
}
