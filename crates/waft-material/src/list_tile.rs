//! Flutter counterpart: material_ui's `list_tile.dart`, with its part
//! `generated/list_tile_defaults_m3.g.dart` in `crate::generated::list_tile_defaults_m3`.
#![allow(deprecated)]

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Brightness, Color, Offset, Size, TextBaseline, TextDirection};
use waft_foundation::{App, Handle, Ref, ValueChanged};
use waft_gestures::{GestureLongPressCallback, GestureTapCallback};
use waft_painting::{
    AnyColor, Border, BoxDecoration, EdgeInsets, EdgeInsetsGeometry, ShapeBorderRef,
    ShapeDecoration, TextStyle,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BaselineOffset, BoxConstraints, BoxHitTestResult, BoxParentData,
    ChildBaselineGetter, ChildLayoutHelper, ChildLayouter, DecorationPosition, PaintingContext,
    PipelineOwner, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
};
use waft_services::MouseCursorRef;
use waft_widgets::{
    AnimatedDefaultTextStyle, BuildContext, ColoredBox, DecoratedBox, Directionality, FocusNodeRef,
    IconTheme, IconThemeData, IntoWidget, KeyRef, RenderObjectWidget, SafeArea, Semantics,
    SizedBox, SlottedContainerRenderObjectData, SlottedContainerRenderObjectMixin,
    SlottedMultiChildRenderObjectWidget, StatelessWidget, WidgetRef, WidgetState, WidgetStateColor,
    WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyAll, WidgetStatePropertyRef,
    WidgetStates, downcast_widget,
};

use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::debug::debug_check_has_material;
use crate::divider::Divider;
use crate::generated::list_tile_defaults_m3::ListTileDefaultsM3;
use crate::icon_button::IconButton;
use crate::icon_button_theme::{IconButtonTheme, IconButtonThemeData};
use crate::ink_decoration::Ink;
use crate::ink_well::InkWell;
use crate::list_tile_theme::{ListTileTheme, ListTileThemeData, ListTileThemeDataGetters};
use crate::material::Material;
use crate::material_state::MaterialStatesController;
use crate::theme::Theme;
use crate::theme_data::{ThemeData, VisualDensity};
use crate::theme_data_getters::DefaultsContext;

/// Dart's `_Sizes`: what `_RenderListTile._computeSizes` works out.
struct Sizes {
    title_y: f64,
    text_constraints: BoxConstraints,
    tile_size: Size,
}

/// Dart's `_PositionChild`.
type PositionChild = fn(&mut App, AnyRenderBox, Offset);

/// Defines the title font used for [`ListTile`] descendants of a [`ListTileTheme`].
///
/// List tiles that appear in a `Drawer` use the theme's `TextTheme::body_large`
/// text style, which is a little smaller than the theme's `TextTheme::title_medium`
/// text style, which is used by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListTileStyle {
    /// Use a title font that's appropriate for a [`ListTile`] in a list.
    List,

    /// Use a title font that's appropriate for a [`ListTile`] that appears in a `Drawer`.
    Drawer,
}

/// Where to place the control in widgets that use [`ListTile`] to position a
/// control next to a label.
///
/// See also:
///
///  * `CheckboxListTile`, which combines a [`ListTile`] with a `Checkbox`.
///  * `RadioListTile`, which combines a [`ListTile`] with a `Radio` button.
///  * `SwitchListTile`, which combines a [`ListTile`] with a `Switch`.
///  * `ExpansionTile`, which combines a [`ListTile`] with a button that expands
///    or collapses the tile to reveal or hide the children.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListTileControlAffinity {
    /// Position the control on the leading edge, and the secondary widget, if
    /// any, on the trailing edge.
    Leading,

    /// Position the control on the trailing edge, and the secondary widget, if
    /// any, on the leading edge.
    Trailing,

    /// Position the control relative to the text in the fashion that is typical
    /// for the current platform, and place the secondary widget on the opposite
    /// side.
    Platform,
}

/// Defines how [`ListTile::leading`] and [`ListTile::trailing`] are
/// vertically aligned relative to the [`ListTile`]'s titles
/// ([`ListTile::title`] and [`ListTile::subtitle`]).
///
/// See also:
///
///  * [`ListTile::title_alignment`], to configure the title alignment for an
///    individual [`ListTile`].
///  * [`ListTileThemeData::title_alignment`], to configure the title alignment
///    for all of the [`ListTile`]s under a [`ListTileTheme`].
///  * `ThemeData::list_tile_theme`, to configure the [`ListTileTheme`]
///    for an entire app.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListTileTitleAlignment {
    /// The top of the [`ListTile::leading`] and [`ListTile::trailing`] widgets are
    /// placed [`ListTile::min_vertical_padding`] below the top of the [`ListTile::title`]
    /// if [`ListTile::is_three_line`] is true, otherwise they're centered relative
    /// to the [`ListTile::title`] and [`ListTile::subtitle`] widgets.
    ///
    /// This is the default when `ThemeData::use_material3` is true.
    ThreeLine,

    /// The tops of the [`ListTile::leading`] and [`ListTile::trailing`] widgets are
    /// placed 16 pixels below the top of the [`ListTile::title`] widget,
    /// if the [`ListTile`]'s overall height is greater than 72, otherwise the
    /// [`ListTile::trailing`] widget is centered relative to the [`ListTile::title`] and
    /// [`ListTile::subtitle`] widgets, and the [`ListTile::leading`] widget is 16 pixels
    /// below the top of [`ListTile::title`], or center-aligned with [`ListTile::title`],
    /// whichever makes the [`ListTile::leading`] closer to the top edge of
    /// [`ListTile::title`].
    ///
    /// This is the default when `ThemeData::use_material3` is false.
    TitleHeight,

    /// The tops of the [`ListTile::leading`] and [`ListTile::trailing`] widgets are
    /// placed [`ListTile::min_vertical_padding`] below the top of the [`ListTile::title`].
    Top,

    /// The [`ListTile::leading`] and [`ListTile::trailing`] widgets are
    /// centered relative to the [`ListTile`]'s titles.
    Center,

    /// The bottoms of the [`ListTile::leading`] and [`ListTile::trailing`] widgets are
    /// placed [`ListTile::min_vertical_padding`] above the bottom of the [`ListTile`]'s
    /// titles.
    Bottom,
}

impl ListTileTitleAlignment {
    // If isLeading is true the y offset is for the leading widget, otherwise it's
    // for the trailing child.
    fn y_offset_for(
        self,
        app: &App,
        child_height: f64,
        tile_height: f64,
        list_tile: RenderHandle<RenderListTile>,
        is_leading: bool,
    ) -> f64 {
        match self {
            ListTileTitleAlignment::ThreeLine => {
                if list_tile.is_three_line(app) {
                    ListTileTitleAlignment::Top.y_offset_for(
                        app,
                        child_height,
                        tile_height,
                        list_tile,
                        is_leading,
                    )
                } else {
                    ListTileTitleAlignment::Center.y_offset_for(
                        app,
                        child_height,
                        tile_height,
                        list_tile,
                        is_leading,
                    )
                }
            }
            // This attempts to implement the redlines for the vertical position of the
            // leading and trailing icons on the spec page:
            //   https://m2.material.io/components/lists#specs
            //
            // For large tiles (> 72dp), both leading and trailing controls should be
            // a fixed distance from top. As per guidelines this is set to 16dp.
            ListTileTitleAlignment::TitleHeight if tile_height > 72.0 => 16.0,
            // For smaller tiles, trailing should always be centered. Leading can be
            // centered or closer to the top. It should never be further than 16dp
            // to the top.
            ListTileTitleAlignment::TitleHeight => {
                if is_leading {
                    ((tile_height - child_height) / 2.0).min(16.0)
                } else {
                    (tile_height - child_height) / 2.0
                }
            }
            ListTileTitleAlignment::Top => list_tile.min_vertical_padding(app),
            ListTileTitleAlignment::Center => (tile_height - child_height) / 2.0,
            ListTileTitleAlignment::Bottom => {
                tile_height - child_height - list_tile.min_vertical_padding(app)
            }
        }
    }
}

/// A single fixed-height row that typically contains some text as well as
/// a leading or trailing icon.
///
/// A list tile contains one to three lines of text optionally flanked by icons or
/// other widgets, such as check boxes. The icons (or other widgets) for the
/// tile are defined with the [`leading`](Self::leading) and [`trailing`](Self::trailing)
/// parameters. The first line of text is not optional and is specified with
/// [`title`](Self::title). The value of [`subtitle`](Self::subtitle), which _is_ optional,
/// will occupy the space allocated for an additional line of text, or two lines if
/// [`is_three_line`](Self::is_three_line) is true. If [`dense`](Self::dense) is true then
/// the overall height of this tile and the size of the `DefaultTextStyle`s that wrap the
/// [`title`](Self::title) and [`subtitle`](Self::subtitle) widget are reduced.
///
/// It is the responsibility of the caller to ensure that [`title`](Self::title) does not
/// wrap, and to ensure that [`subtitle`](Self::subtitle) doesn't wrap (if
/// [`is_three_line`](Self::is_three_line) is false) or wraps to two lines (if it is true).
///
/// The heights of the [`leading`](Self::leading) and [`trailing`](Self::trailing) widgets
/// are constrained according to the
/// [Material spec](https://material.io/design/components/lists.html).
/// An exception is made for one-line ListTiles for accessibility. Please
/// see the example below to see how to adhere to both Material spec and
/// accessibility requirements.
///
/// The [`leading`](Self::leading) and [`trailing`](Self::trailing) widgets can expand as far
/// as they wish horizontally, so ensure that they are properly constrained.
///
/// List tiles are typically used in `ListView`s, or arranged in `Column`s in
/// `Drawer`s and `Card`s.
///
/// This widget requires a [`Material`] widget ancestor in the tree to paint
/// itself on, which is typically provided by the app's `Scaffold`.
/// The [`tile_color`](Self::tile_color), [`selected_tile_color`](Self::selected_tile_color),
/// [`focus_color`](Self::focus_color), and [`hover_color`](Self::hover_color)
/// are not painted by the [`ListTile`] itself but by the [`Material`] widget
/// ancestor. In this case, one can wrap a [`Material`] widget around the
/// [`ListTile`].
///
/// ## Performance considerations when wrapping [`ListTile`] with [`Material`]
///
/// Wrapping a large number of [`ListTile`]s individually with [`Material`]s
/// is expensive. Consider only wrapping the [`ListTile`]s that require it
/// or include a common [`Material`] ancestor where possible.
///
/// [`ListTile`] must be wrapped in a [`Material`] widget to animate
/// [`tile_color`](Self::tile_color), [`selected_tile_color`](Self::selected_tile_color),
/// [`focus_color`](Self::focus_color), and [`hover_color`](Self::hover_color) as these
/// colors are not drawn by the list tile itself but by the material widget ancestor.
///
/// To use a [`ListTile`] within a `Row`, it needs to be wrapped in an
/// `Expanded` widget. [`ListTile`] requires fixed width constraints,
/// whereas a `Row` does not constrain its children.
///
/// To be accessible, tappable [`leading`](Self::leading) and [`trailing`](Self::trailing)
/// widgets have to be at least 48x48 in size. However, to adhere to the Material spec,
/// [`trailing`](Self::trailing) and [`leading`](Self::leading) widgets in one-line ListTiles
/// should visually be at most 32 ([`dense`](Self::dense): true) or 40
/// ([`dense`](Self::dense): false) in height, which may conflict with the accessibility
/// requirement.
///
/// For this reason, a one-line ListTile allows the height of [`leading`](Self::leading)
/// and [`trailing`](Self::trailing) widgets to be constrained by the height of the ListTile.
/// This allows for the creation of tappable [`leading`](Self::leading) and
/// [`trailing`](Self::trailing) widgets that are large enough, but it is up to the
/// developer to ensure that their widgets follow the Material spec.
///
/// ## The ListTile layout isn't exactly what I want
///
/// If the way ListTile pads and positions its elements isn't quite what
/// you're looking for, it's easy to create custom list items with a
/// combination of other widgets, such as `Row`s and `Column`s.
///
/// See also:
///
///  * [`ListTileTheme`], which defines visual properties for [`ListTile`]s.
///  * `ListView`, which can display an arbitrary number of [`ListTile`]s
///    in a scrolling list.
///  * `CircleAvatar`, which shows an icon representing a person and is often
///    used as the [`leading`](Self::leading) element of a ListTile.
///  * `Card`, which can be used with `Column` to show a few [`ListTile`]s.
///  * [`Divider`], which can be used to separate [`ListTile`]s.
///  * [`ListTile::divide_tiles`], a utility for inserting [`Divider`]s in between
///    [`ListTile`]s.
///  * `CheckboxListTile`, `RadioListTile`, and `SwitchListTile`, widgets
///    that combine [`ListTile`] with other controls.
///  * Material 3 [`ListTile`] specifications are referenced from
///    <https://m3.material.io/components/lists/specs>
///    and Material 2 [`ListTile`] specifications are referenced from
///    <https://material.io/design/components/lists.html>
///  * Cookbook: [Use lists](https://docs.flutter.dev/cookbook/lists/basic-list)
///  * Cookbook: [Implement swipe to dismiss](https://docs.flutter.dev/cookbook/gestures/dismissible)
#[derive(Clone)]
pub struct ListTile {
    pub key: Option<KeyRef>,

    /// A widget to display before the title.
    ///
    /// Typically an `Icon` or a `CircleAvatar` widget.
    pub leading: Option<WidgetRef>,

    /// The primary content of the list tile.
    ///
    /// Typically a `Text` widget.
    ///
    /// This should not wrap. To enforce the single line limit, use
    /// `Text::max_lines`.
    pub title: Option<WidgetRef>,

    /// Additional content displayed below the title.
    ///
    /// Typically a `Text` widget.
    ///
    /// If [`is_three_line`](Self::is_three_line) is false, this should not wrap.
    ///
    /// If [`is_three_line`](Self::is_three_line) is true, this should be configured to take
    /// a maximum of two lines. For example, you can use `Text::max_lines` to enforce the
    /// number of lines.
    ///
    /// The subtitle's default [`TextStyle`] depends on `TextTheme::body_medium` except
    /// `TextStyle::color`. The `TextStyle::color` depends on the value of
    /// [`enabled`](Self::enabled) and [`selected`](Self::selected).
    ///
    /// When [`enabled`](Self::enabled) is false, the text color is set to
    /// `ThemeData::disabled_color`.
    ///
    /// When [`selected`](Self::selected) is false, the text color is set to
    /// `ListTileTheme::text_color` if it's not null and to `TextTheme::body_small`'s color
    /// if `ListTileTheme::text_color` is null.
    pub subtitle: Option<WidgetRef>,

    /// A widget to display after the title.
    ///
    /// Typically an `Icon` widget.
    ///
    /// To show right-aligned metadata (assuming left-to-right reading order;
    /// left-aligned for right-to-left reading order), consider using a `Row` with
    /// `CrossAxisAlignment::Baseline` alignment whose first item is `Expanded` and
    /// whose second child is the metadata text, instead of using the
    /// [`trailing`](Self::trailing) property.
    pub trailing: Option<WidgetRef>,

    /// Whether this list tile is intended to display three lines of text.
    ///
    /// If true, then [`subtitle`](Self::subtitle) must be non-null (since it is expected to
    /// give the second and third lines of text).
    ///
    /// If false, the list tile is treated as having one line if the subtitle is
    /// null and treated as having two lines if the subtitle is non-null.
    ///
    /// When using a `Text` widget for [`title`](Self::title) and
    /// [`subtitle`](Self::subtitle), you can enforce line limits using `Text::max_lines`.
    ///
    /// See also:
    ///
    /// * [`ListTileTheme::of`], which returns the nearest [`ListTileTheme`]'s
    ///   [`ListTileThemeData`].
    pub is_three_line: Option<bool>,

    /// Whether this list tile is part of a vertically dense list.
    ///
    /// If this property is null then its value is based on `ListTileTheme::dense`.
    ///
    /// Dense list tiles default to a smaller height.
    ///
    /// It is not recommended to set [`dense`](Self::dense) to true when
    /// `ThemeData::use_material3` is true.
    pub dense: Option<bool>,

    /// Defines how compact the list tile's layout will be.
    ///
    /// See also:
    ///
    ///  * `ThemeData::visual_density`, which specifies the
    ///    [`visual_density`](Self::visual_density) for all widgets within a `Theme`.
    pub visual_density: Option<VisualDensity>,

    /// Defines the tile's `InkWell::custom_border` and `Ink::decoration` shape.
    ///
    /// If this property is null then [`ListTileThemeData::shape`] is used. If that
    /// is also null then a rectangular [`Border`] will be used.
    ///
    /// See also:
    ///
    /// * [`ListTileTheme::of`], which returns the nearest [`ListTileTheme`]'s
    ///   [`ListTileThemeData`].
    pub shape: Option<ShapeBorderRef>,

    /// Defines the color used for icons and text when the list tile is selected.
    ///
    /// If this property is null then [`ListTileThemeData::selected_color`]
    /// is used. If that is also null then `ColorScheme::primary` is used.
    ///
    /// See also:
    ///
    /// * [`ListTileTheme::of`], which returns the nearest [`ListTileTheme`]'s
    ///   [`ListTileThemeData`].
    pub selected_color: Option<Color>,

    /// Defines the default color for [`leading`](Self::leading) and
    /// [`trailing`](Self::trailing) icons.
    ///
    /// If this property is null and [`selected`](Self::selected) is false then
    /// [`ListTileThemeData::icon_color`] is used. If that is also null and
    /// `ThemeData::use_material3` is true, `ColorScheme::on_surface_variant` is used,
    /// otherwise if `ThemeData::brightness` is [`Brightness::Light`], `Colors::BLACK54` is
    /// used, and if `ThemeData::brightness` is [`Brightness::Dark`], the value is null.
    ///
    /// If this property is null and [`selected`](Self::selected) is true then
    /// [`ListTileThemeData::selected_color`] is used. If that is also null then
    /// `ColorScheme::primary` is used.
    ///
    /// If this color is a [`WidgetStateColor`] it will be resolved against
    /// [`WidgetState::Selected`] and [`WidgetState::Disabled`] states.
    ///
    /// See also:
    ///
    /// * [`ListTileTheme::of`], which returns the nearest [`ListTileTheme`]'s
    ///   [`ListTileThemeData`].
    pub icon_color: Option<AnyColor>,

    /// Defines the text color for the [`title`](Self::title), [`subtitle`](Self::subtitle),
    /// [`leading`](Self::leading), and [`trailing`](Self::trailing).
    ///
    /// If this property is null and [`selected`](Self::selected) is false then
    /// [`ListTileThemeData::text_color`] is used. If that is also null then default text
    /// color is used for the [`title`](Self::title), [`subtitle`](Self::subtitle)
    /// [`leading`](Self::leading), and [`trailing`](Self::trailing). Except for
    /// [`subtitle`](Self::subtitle), if `ThemeData::use_material3` is false,
    /// `TextTheme::body_small` is used.
    ///
    /// If this property is null and [`selected`](Self::selected) is true then
    /// [`ListTileThemeData::selected_color`] is used. If that is also null then
    /// `ColorScheme::primary` is used.
    ///
    /// If this color is a [`WidgetStateColor`] it will be resolved against
    /// [`WidgetState::Selected`] and [`WidgetState::Disabled`] states.
    ///
    /// See also:
    ///
    /// * [`ListTileTheme::of`], which returns the nearest [`ListTileTheme`]'s
    ///   [`ListTileThemeData`].
    pub text_color: Option<AnyColor>,

    /// The text style for ListTile's [`title`](Self::title).
    ///
    /// If this property is null, then [`ListTileThemeData::title_text_style`] is used.
    /// If that is also null and `ThemeData::use_material3` is true, `TextTheme::body_large`
    /// with `ColorScheme::on_surface` will be used. Otherwise, If ListTile style is
    /// [`ListTileStyle::List`], `TextTheme::title_medium` will be used and if ListTile style
    /// is [`ListTileStyle::Drawer`], `TextTheme::body_large` will be used.
    pub title_text_style: Option<TextStyle>,

    /// The text style for ListTile's [`subtitle`](Self::subtitle).
    ///
    /// If this property is null, then [`ListTileThemeData::subtitle_text_style`] is used.
    /// If that is also null and `ThemeData::use_material3` is true, `TextTheme::body_medium`
    /// with `ColorScheme::on_surface_variant` will be used, otherwise
    /// `TextTheme::body_medium` with `TextTheme::body_small` color will be used.
    pub subtitle_text_style: Option<TextStyle>,

    /// The text style for ListTile's [`leading`](Self::leading) and
    /// [`trailing`](Self::trailing).
    ///
    /// If this property is null, then
    /// [`ListTileThemeData::leading_and_trailing_text_style`] is used. If that is also null
    /// and `ThemeData::use_material3` is true, `TextTheme::label_small` with
    /// `ColorScheme::on_surface_variant` will be used, otherwise `TextTheme::body_medium`
    /// will be used.
    pub leading_and_trailing_text_style: Option<TextStyle>,

    /// Defines the font used for the [`title`](Self::title).
    ///
    /// If this property is null then [`ListTileThemeData::style`] is used. If that
    /// is also null then [`ListTileStyle::List`] is used.
    ///
    /// See also:
    ///
    /// * [`ListTileTheme::of`], which returns the nearest [`ListTileTheme`]'s
    ///   [`ListTileThemeData`].
    pub style: Option<ListTileStyle>,

    /// The tile's internal padding.
    ///
    /// Insets a [`ListTile`]'s contents: its [`leading`](Self::leading),
    /// [`title`](Self::title), [`subtitle`](Self::subtitle), and
    /// [`trailing`](Self::trailing) widgets.
    ///
    /// If this property is null, then [`ListTileThemeData::content_padding`] is used. If
    /// that is also null and `ThemeData::use_material3` is true, then a default value of
    /// `EdgeInsetsDirectional::only(start: 16.0, end: 24.0)` will be used. Otherwise, a
    /// default value of `EdgeInsets::symmetric(horizontal: 16.0)` will be used.
    pub content_padding: Option<EdgeInsetsGeometry>,

    /// Whether this list tile is interactive.
    ///
    /// If false, this list tile is styled with the disabled color from the
    /// current `Theme` and the [`on_tap`](Self::on_tap) and
    /// [`on_long_press`](Self::on_long_press) callbacks are inoperative.
    pub enabled: bool,

    /// Called when the user taps this list tile.
    ///
    /// Inoperative if [`enabled`](Self::enabled) is false.
    pub on_tap: Option<GestureTapCallback>,

    /// Called when the user long-presses on this list tile.
    ///
    /// Inoperative if [`enabled`](Self::enabled) is false.
    pub on_long_press: Option<GestureLongPressCallback>,

    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's node gains focus, and false if it loses
    /// focus.
    pub on_focus_change: Option<ValueChanged<bool>>,

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a [`WidgetStateMouseCursor`],
    /// [`WidgetStateProperty::resolve`] is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of [`ListTileThemeData::mouse_cursor`] is used. If
    /// that is also null, then [`WidgetStateMouseCursor::clickable`] is used.
    pub mouse_cursor: Option<MouseCursorRef>,

    /// If this tile is also [`enabled`](Self::enabled) then icons and text are rendered with
    /// the same color.
    ///
    /// By default the selected color is the theme's primary color. The selected color
    /// can be overridden with a [`ListTileTheme`].
    pub selected: bool,

    /// The color for the tile's [`Material`] when it has the input focus.
    pub focus_color: Option<Color>,

    /// The color for the tile's [`Material`] when a pointer is hovering over it.
    pub hover_color: Option<Color>,

    /// The color of splash for the tile's [`Material`].
    pub splash_color: Option<Color>,

    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,

    /// True if this widget will be selected as the initial focus when no other
    /// node in its scope is currently focused.
    pub autofocus: bool,

    /// Defines the background color of `ListTile` when [`selected`](Self::selected) is
    /// false.
    ///
    /// If this property is null and [`selected`](Self::selected) is false then
    /// [`ListTileThemeData::tile_color`] is used. If that is also null and
    /// [`selected`](Self::selected) is true, [`selected_tile_color`](Self::selected_tile_color)
    /// is used. When that is also null, the `ListTileTheme::selected_tile_color` is used,
    /// otherwise `Colors::TRANSPARENT` is used.
    pub tile_color: Option<Color>,

    /// Defines the background color of `ListTile` when [`selected`](Self::selected) is true.
    ///
    /// When the value if null, the [`selected_tile_color`](Self::selected_tile_color) is set
    /// to `ListTileTheme::selected_tile_color` if it's not null and to `Colors::TRANSPARENT`
    /// if it's null.
    pub selected_tile_color: Option<Color>,

    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a
    /// long-press will produce a short vibration, when feedback is enabled.
    ///
    /// When null, the default value is true.
    ///
    /// See also:
    ///
    ///  * `Feedback` for providing platform-specific feedback to certain actions.
    pub enable_feedback: Option<bool>,

    /// The horizontal gap between the titles and the leading/trailing widgets.
    ///
    /// If null, then the value of `ListTileTheme::horizontal_title_gap` is used. If
    /// that is also null, then a default value of 16 is used.
    pub horizontal_title_gap: Option<f64>,

    /// The minimum padding on the top and bottom of the title and subtitle widgets.
    ///
    /// If null, then the value of `ListTileTheme::min_vertical_padding` is used. If
    /// that is also null, then a default value of 4 is used.
    pub min_vertical_padding: Option<f64>,

    /// The minimum width allocated for the [`ListTile::leading`] widget.
    ///
    /// If null, then the value of `ListTileTheme::min_leading_width` is used. If
    /// that is also null, then a default value of 40 is used.
    pub min_leading_width: Option<f64>,

    /// The minimum height allocated for the [`ListTile`] widget.
    ///
    /// If this is null, default tile heights are 56.0, 72.0, and 88.0 for one,
    /// two, and three lines of text respectively. If `isDense` is true, these
    /// defaults are changed to 48.0, 64.0, and 76.0. A visual density value or
    /// a large title will also adjust the default tile heights.
    pub min_tile_height: Option<f64>,

    /// Defines how [`ListTile::leading`] and [`ListTile::trailing`] are
    /// vertically aligned relative to the [`ListTile`]'s titles
    /// ([`ListTile::title`] and [`ListTile::subtitle`]).
    ///
    /// If this property is null then [`ListTileThemeData::title_alignment`]
    /// is used. If that is also null then [`ListTileTitleAlignment::ThreeLine`]
    /// is used.
    ///
    /// See also:
    ///
    /// * [`ListTileTheme::of`], which returns the nearest [`ListTileTheme`]'s
    ///   [`ListTileThemeData`].
    pub title_alignment: Option<ListTileTitleAlignment>,

    /// Whether to add button:true to the semantics if onTap is provided.
    /// This is a temporary flag to help changing the behavior of ListTile onTap semantics.
    ///
    /// Defaults to true.
    pub internal_add_semantic_for_on_tap: bool,

    /// Represents the interactive "state" of this widget in terms of a set of
    /// [`WidgetState`]s, including [`WidgetState::Pressed`], [`WidgetState::Hovered`] and
    /// [`WidgetState::Focused`].
    pub states_controller: Option<Ref<MaterialStatesController>>,
}

impl Default for ListTile {
    fn default() -> ListTile {
        ListTile {
            key: None,
            leading: None,
            title: None,
            subtitle: None,
            trailing: None,
            is_three_line: None,
            dense: None,
            visual_density: None,
            shape: None,
            selected_color: None,
            icon_color: None,
            text_color: None,
            title_text_style: None,
            subtitle_text_style: None,
            leading_and_trailing_text_style: None,
            style: None,
            content_padding: None,
            enabled: true,
            on_tap: None,
            on_long_press: None,
            on_focus_change: None,
            mouse_cursor: None,
            selected: false,
            focus_color: None,
            hover_color: None,
            splash_color: None,
            focus_node: None,
            autofocus: false,
            tile_color: None,
            selected_tile_color: None,
            enable_feedback: None,
            horizontal_title_gap: None,
            min_vertical_padding: None,
            min_leading_width: None,
            min_tile_height: None,
            title_alignment: None,
            internal_add_semantic_for_on_tap: true,
            states_controller: None,
        }
    }
}

/// The setters of `ListTile`'s optional arguments that hold a value.
macro_rules! list_tile_setters {
    ($($field:ident: $ty:ty => $name:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `ListTile(", $name, ":)`.")]
            pub fn $field(mut self, $field: impl Into<$ty>) -> ListTile {
                self.$field = Some($field.into());
                self
            }
        )*
    };
}

/// The setters of `ListTile`'s optional arguments that are widgets.
macro_rules! list_tile_widget_setters {
    ($($field:ident => $name:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `ListTile(", $name, ":)`.")]
            pub fn $field<K>(mut self, $field: impl IntoWidget<K>) -> ListTile {
                self.$field = Some($field.into_widget());
                self
            }
        )*
    };
}

impl ListTile {
    /// Creates a list tile; Dart's named arguments are the setters.
    ///
    /// If [`is_three_line`](Self::is_three_line) is true, then [`subtitle`](Self::subtitle)
    /// must not be null.
    ///
    /// Requires one of its ancestors to be a [`Material`] widget.
    pub fn new() -> ListTile {
        ListTile::default()
    }

    /// Dart `ListTile(key:)`.
    pub fn key(mut self, key: KeyRef) -> ListTile {
        self.key = Some(key);
        self
    }

    list_tile_widget_setters!(
        leading => "leading",
        title => "title",
        subtitle => "subtitle",
        trailing => "trailing",
    );

    /// Dart `ListTile(isThreeLine:)`.
    pub fn is_three_line(mut self, is_three_line: bool) -> ListTile {
        self.is_three_line = Some(is_three_line);
        self
    }

    list_tile_setters!(
        dense: bool => "dense",
        visual_density: VisualDensity => "visualDensity",
        shape: ShapeBorderRef => "shape",
        style: ListTileStyle => "style",
        selected_color: Color => "selectedColor",
        icon_color: AnyColor => "iconColor",
        text_color: AnyColor => "textColor",
        title_text_style: TextStyle => "titleTextStyle",
        subtitle_text_style: TextStyle => "subtitleTextStyle",
        leading_and_trailing_text_style: TextStyle => "leadingAndTrailingTextStyle",
        content_padding: EdgeInsetsGeometry => "contentPadding",
        on_tap: GestureTapCallback => "onTap",
        on_long_press: GestureLongPressCallback => "onLongPress",
        on_focus_change: ValueChanged<bool> => "onFocusChange",
        mouse_cursor: MouseCursorRef => "mouseCursor",
        focus_color: Color => "focusColor",
        hover_color: Color => "hoverColor",
        splash_color: Color => "splashColor",
        focus_node: FocusNodeRef => "focusNode",
        tile_color: Color => "tileColor",
        selected_tile_color: Color => "selectedTileColor",
        enable_feedback: bool => "enableFeedback",
        horizontal_title_gap: f64 => "horizontalTitleGap",
        min_vertical_padding: f64 => "minVerticalPadding",
        min_leading_width: f64 => "minLeadingWidth",
        min_tile_height: f64 => "minTileHeight",
        title_alignment: ListTileTitleAlignment => "titleAlignment",
        states_controller: Ref<MaterialStatesController> => "statesController",
    );

    /// Dart `ListTile(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> ListTile {
        self.enabled = enabled;
        self
    }

    /// Dart `ListTile(selected:)`.
    pub fn selected(mut self, selected: bool) -> ListTile {
        self.selected = selected;
        self
    }

    /// Dart `ListTile(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> ListTile {
        self.autofocus = autofocus;
        self
    }

    /// Dart `ListTile(internalAddSemanticForOnTap:)`.
    pub fn internal_add_semantic_for_on_tap(
        mut self,
        internal_add_semantic_for_on_tap: bool,
    ) -> ListTile {
        self.internal_add_semantic_for_on_tap = internal_add_semantic_for_on_tap;
        self
    }

    /// Add a one pixel border in between each tile. If color isn't specified the
    /// `ThemeData::divider_color` of the context's `Theme` is used.
    ///
    /// See also:
    ///
    ///  * [`Divider`], which you can use to obtain this effect manually.
    pub fn divide_tiles(
        app: &mut App,
        context: Option<BuildContext>,
        tiles: impl IntoIterator<Item = WidgetRef>,
        color: Option<Color>,
    ) -> Vec<WidgetRef> {
        debug_assert!(color.is_some() || context.is_some());
        let tiles: Vec<WidgetRef> = tiles.into_iter().collect();

        if tiles.is_empty() || tiles.len() == 1 {
            return tiles;
        }

        let mut wrap_tile = |tile: WidgetRef| -> WidgetRef {
            DecoratedBox::new(BoxDecoration::new().border(Border {
                bottom: Divider::create_border_side(app, context, color, None),
                ..Border::default()
            }))
            .position(DecorationPosition::Foreground)
            .child(tile)
            .into_widget()
        };

        let last = tiles.len() - 1;
        let mut divided: Vec<WidgetRef> =
            tiles[..last].iter().cloned().map(&mut wrap_tile).collect();
        divided.push(tiles[last].clone());
        divided
    }

    fn is_dense_layout(&self, theme: &ThemeData, tile_theme: &ListTileThemeData) -> bool {
        self.dense
            .or(tile_theme.dense)
            .or(theme.list_tile_theme.dense)
            .unwrap_or(false)
    }

    fn debug_check_background_is_hidden(&self, app: &App, context: BuildContext) -> bool {
        if cfg!(debug_assertions)
            && let Some((intermediate_widget, intermediate_debug)) =
                ListTile::find_intermediate_widget(app, context)
        {
            panic!(
                "ListTile background color or ink splashes may be invisible.\nThe ListTile is \
                 wrapped in a {intermediate_widget} that has a background color. Because \
                 ListTile paints its background and ink splashes on the nearest Material \
                 ancestor, this {intermediate_widget} will hide those effects.\nTo fix this, \
                 wrap the ListTile in its own Material widget, or remove the background color \
                 from the intermediate {intermediate_widget}.\nListTile: {self:?}\n\
                 {intermediate_widget}: {intermediate_debug}"
            );
        }
        true
    }

    /// The nearest ancestor below the closest [`Material`] that paints a visible colour:
    /// its type's name and its debug form.
    fn find_intermediate_widget(
        app: &App,
        context: BuildContext,
    ) -> Option<(&'static str, String)> {
        let mut intermediate_widget = None;
        context.visit_ancestor_elements(app, &mut |ancestor| {
            let widget = ancestor.widget(app);
            if downcast_widget::<Material>(&**widget).is_some() {
                return false;
            }
            let found = if let Some(colored_box) = downcast_widget::<ColoredBox>(&**widget) {
                Some(("ColoredBox", Some(colored_box.color.color())))
            } else if let Some(decorated_box) = downcast_widget::<DecoratedBox>(&**widget) {
                let decoration = decorated_box.decoration.as_any();
                let color = if let Some(decoration) = decoration.downcast_ref::<BoxDecoration>() {
                    decoration.color.as_ref().map(AnyColor::color)
                } else if let Some(decoration) = decoration.downcast_ref::<ShapeDecoration>() {
                    decoration.color.as_ref().map(AnyColor::color)
                } else {
                    None
                };
                Some(("DecoratedBox", color))
            } else {
                None
            };
            if let Some((name, Some(color))) = found
                && color.a > 0.0
            {
                intermediate_widget = Some((name, format!("{widget:?}")));
                return false;
            }
            true
        });
        intermediate_widget
    }
}

impl StatelessWidget for ListTile {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.is_three_line != Some(true) || self.subtitle.is_some());
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        let theme = Theme::of(app, context);
        let icon_button_theme = IconButtonTheme::of(app, context);
        let tile_theme = ListTileTheme::of(app, context);
        let list_tile_style = self
            .style
            .or(tile_theme.style)
            .or(theme.list_tile_theme.style)
            .unwrap_or(ListTileStyle::List);
        let defaults: Box<dyn ListTileThemeDataGetters> = if theme.use_material3 {
            Box::new(ListTileDefaultsM3::new(context))
        } else {
            Box::new(LisTileDefaultsM2::new(context, list_tile_style))
        };

        let background_color = self
            .tile_color
            .or(tile_theme.tile_color)
            .or(theme.list_tile_theme.tile_color)
            .unwrap_or_else(|| {
                defaults
                    .tile_color(app)
                    .expect("the defaults have a tile color")
            });
        let selected_background_color = self
            .selected_tile_color
            .or(tile_theme.selected_tile_color)
            .or(theme.list_tile_theme.selected_tile_color)
            .unwrap_or_else(|| {
                defaults
                    .tile_color(app)
                    .expect("the defaults have a tile color")
            });
        let effective_tile_color = if self.selected {
            selected_background_color
        } else {
            background_color
        };
        let has_opaque_background =
            background_color.alpha() > 0 || selected_background_color.alpha() > 0;
        if self.on_tap.is_some() || self.on_long_press.is_some() || has_opaque_background {
            debug_assert!(self.debug_check_background_is_hidden(app, context));
        }
        let mut states = WidgetStates::new();
        if !self.enabled {
            states.insert(WidgetState::Disabled);
        }
        if self.selected {
            states.insert(WidgetState::Selected);
        }

        let resolve_color = |explicit_color: Option<&AnyColor>,
                             selected_color: Option<Color>,
                             enabled_color: Option<&AnyColor>,
                             disabled_color: Option<Color>|
         -> Option<Color> {
            IndividualOverrides {
                explicit_color: explicit_color.cloned(),
                enabled_color: enabled_color.map(AnyColor::color),
                selected_color,
                disabled_color,
            }
            .resolve(&states)
        };

        let mut effective_icon_color = resolve_color(
            self.icon_color.as_ref(),
            self.selected_color,
            self.icon_color.as_ref(),
            None,
        )
        .or_else(|| {
            resolve_color(
                tile_theme.icon_color.as_ref(),
                tile_theme.selected_color,
                tile_theme.icon_color.as_ref(),
                None,
            )
        })
        .or_else(|| {
            resolve_color(
                theme.list_tile_theme.icon_color.as_ref(),
                theme.list_tile_theme.selected_color,
                theme.list_tile_theme.icon_color.as_ref(),
                None,
            )
        });

        let default_icon_color = defaults.icon_color(app);
        let default_effective_icon_color = resolve_color(
            default_icon_color.as_ref(),
            defaults.selected_color(app),
            default_icon_color.as_ref(),
            Some(theme.disabled_color),
        );

        let effective_icon_button_color = effective_icon_color
            .or_else(|| {
                icon_button_theme
                    .style
                    .as_ref()
                    .and_then(|style| style.foreground_color.as_ref())
                    .and_then(|foreground_color| foreground_color.resolve(&states))
            })
            .or(default_effective_icon_color);

        effective_icon_color = effective_icon_color.or(default_effective_icon_color);

        let effective_color = resolve_color(
            self.text_color.as_ref(),
            self.selected_color,
            self.text_color.as_ref(),
            None,
        )
        .or_else(|| {
            resolve_color(
                tile_theme.text_color.as_ref(),
                tile_theme.selected_color,
                tile_theme.text_color.as_ref(),
                None,
            )
        })
        .or_else(|| {
            resolve_color(
                theme.list_tile_theme.text_color.as_ref(),
                theme.list_tile_theme.selected_color,
                theme.list_tile_theme.text_color.as_ref(),
                None,
            )
        })
        .or_else(|| {
            let default_text_color = defaults.text_color(app);
            resolve_color(
                default_text_color.as_ref(),
                defaults.selected_color(app),
                default_text_color.as_ref(),
                Some(theme.disabled_color),
            )
        });
        let mut icon_theme_data = IconThemeData::new();
        if let Some(effective_icon_color) = effective_icon_color {
            icon_theme_data = icon_theme_data.color(effective_icon_color);
        }
        let icon_button_theme_data =
            IconButtonThemeData::new().style(match IconButtonTheme::of(app, context).style {
                Some(style) => {
                    style
                        .copy_with()
                        .foreground_color(Rc::new(WidgetStatePropertyAll::new(
                            effective_icon_button_color,
                        ))
                            as WidgetStatePropertyRef<Option<Color>>)
                }
                None => {
                    let mut style_from = IconButton::style_from();
                    if let Some(effective_icon_button_color) = effective_icon_button_color {
                        style_from = style_from.foreground_color(effective_icon_button_color);
                    }
                    style_from.build()
                }
            });

        let mut leading_and_trailing_style: Option<TextStyle> = None;
        if self.leading.is_some() || self.trailing.is_some() {
            let style = self
                .leading_and_trailing_text_style
                .clone()
                .or(tile_theme.leading_and_trailing_text_style.clone())
                .unwrap_or_else(|| {
                    defaults
                        .leading_and_trailing_text_style(app)
                        .expect("the defaults have a leading and trailing text style")
                });
            let leading_and_trailing_text_color = effective_color;
            leading_and_trailing_style = Some(with_color(&style, leading_and_trailing_text_color));
        }

        let mut leading_icon: Option<WidgetRef> = None;
        if let Some(leading) = &self.leading {
            leading_icon = Some(
                AnimatedDefaultTextStyle::new(
                    leading.clone(),
                    leading_and_trailing_style
                        .clone()
                        .expect("a leading widget has a style"),
                    K_THEME_CHANGE_DURATION,
                )
                .into_widget(),
            );
        }

        let title_style = self
            .title_text_style
            .clone()
            .or(tile_theme.title_text_style.clone())
            .unwrap_or_else(|| {
                defaults
                    .title_text_style(app)
                    .expect("the defaults have a title text style")
            });
        let title_color = effective_color;
        let mut title_style = with_color(&title_style, title_color);
        if self.is_dense_layout(&theme, &tile_theme) {
            title_style = title_style.font_size(13.0);
        }
        let title_text = AnimatedDefaultTextStyle::new(
            self.title
                .clone()
                .unwrap_or_else(|| SizedBox::new().into_widget()),
            title_style.clone(),
            K_THEME_CHANGE_DURATION,
        )
        .into_widget();

        let mut subtitle_text: Option<WidgetRef> = None;
        let mut subtitle_style: Option<TextStyle> = None;
        if let Some(subtitle) = &self.subtitle {
            let style = self
                .subtitle_text_style
                .clone()
                .or(tile_theme.subtitle_text_style.clone())
                .unwrap_or_else(|| {
                    defaults
                        .subtitle_text_style(app)
                        .expect("the defaults have a subtitle text style")
                });
            let subtitle_color = effective_color;
            let mut style = with_color(&style, subtitle_color);
            if self.is_dense_layout(&theme, &tile_theme) {
                style = style.font_size(12.0);
            }
            subtitle_text = Some(
                AnimatedDefaultTextStyle::new(
                    subtitle.clone(),
                    style.clone(),
                    K_THEME_CHANGE_DURATION,
                )
                .into_widget(),
            );
            subtitle_style = Some(style);
        }

        let mut trailing_icon: Option<WidgetRef> = None;
        if let Some(trailing) = &self.trailing {
            trailing_icon = Some(
                AnimatedDefaultTextStyle::new(
                    trailing.clone(),
                    leading_and_trailing_style
                        .clone()
                        .expect("a trailing widget has a style"),
                    K_THEME_CHANGE_DURATION,
                )
                .into_widget(),
            );
        }

        let text_direction = Directionality::of(app, context);
        let resolved_content_padding: EdgeInsets = self
            .content_padding
            .map(|padding| padding.resolve(Some(text_direction)))
            .or_else(|| {
                tile_theme
                    .content_padding
                    .map(|padding| padding.resolve(Some(text_direction)))
            })
            .unwrap_or_else(|| {
                defaults
                    .content_padding(app)
                    .expect("the defaults have a content padding")
                    .resolve(Some(text_direction))
            });

        // Show basic cursor when ListTile isn't enabled or gesture callbacks are null.
        let mut mouse_states = WidgetStates::new();
        if !self.enabled || (self.on_tap.is_none() && self.on_long_press.is_none()) {
            mouse_states.insert(WidgetState::Disabled);
        }
        let effective_mouse_cursor: MouseCursorRef =
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(
                &self.mouse_cursor,
                &mouse_states,
            )
            .or_else(|| {
                tile_theme
                    .mouse_cursor
                    .as_ref()
                    .and_then(|mouse_cursor| mouse_cursor.resolve(&mouse_states))
            })
            .unwrap_or_else(|| WidgetStateMouseCursor::clickable().resolve(&mouse_states));

        let effective_title_alignment = self
            .title_alignment
            .or(tile_theme.title_alignment)
            .unwrap_or(if theme.use_material3 {
                ListTileTitleAlignment::ThreeLine
            } else {
                ListTileTitleAlignment::TitleHeight
            });

        let is_dense = self.is_dense_layout(&theme, &tile_theme);
        let raw_list_tile = RawListTile {
            leading: leading_icon,
            title: title_text,
            subtitle: subtitle_text,
            trailing: trailing_icon,
            is_dense,
            visual_density: self
                .visual_density
                .or(tile_theme.visual_density)
                .unwrap_or(theme.visual_density),
            is_three_line: self
                .is_three_line
                .or(tile_theme.is_three_line)
                .or(theme.list_tile_theme.is_three_line)
                .unwrap_or(false),
            text_direction,
            title_baseline_type: title_style.text_baseline.unwrap_or_else(|| {
                defaults
                    .title_text_style(app)
                    .as_ref()
                    .and_then(|style| style.text_baseline)
                    .expect("the default title text style has a text baseline")
            }),
            subtitle_baseline_type: Some(
                subtitle_style
                    .as_ref()
                    .and_then(|style| style.text_baseline)
                    .unwrap_or_else(|| {
                        defaults
                            .subtitle_text_style(app)
                            .as_ref()
                            .and_then(|style| style.text_baseline)
                            .expect("the default subtitle text style has a text baseline")
                    }),
            ),
            horizontal_title_gap: self
                .horizontal_title_gap
                .or(tile_theme.horizontal_title_gap)
                .unwrap_or(16.0),
            min_vertical_padding: self
                .min_vertical_padding
                .or(tile_theme.min_vertical_padding)
                .unwrap_or_else(|| {
                    defaults
                        .min_vertical_padding(app)
                        .expect("the defaults have a min vertical padding")
                }),
            min_leading_width: self
                .min_leading_width
                .or(tile_theme.min_leading_width)
                .unwrap_or_else(|| {
                    defaults
                        .min_leading_width(app)
                        .expect("the defaults have a min leading width")
                }),
            min_tile_height: self.min_tile_height.or(tile_theme.min_tile_height),
            title_alignment: effective_title_alignment,
        };

        let mut ink_well = InkWell::new()
            .mouse_cursor(effective_mouse_cursor)
            .can_request_focus(self.enabled)
            .autofocus(self.autofocus)
            .enable_feedback(
                self.enable_feedback
                    .or(tile_theme.enable_feedback)
                    .unwrap_or(true),
            )
            .child(
                Semantics::new()
                    .button(
                        self.internal_add_semantic_for_on_tap
                            && (self.on_tap.is_some() || self.on_long_press.is_some()),
                    )
                    .selected(self.selected)
                    .enabled(self.enabled)
                    .child(
                        Ink::new()
                            .decoration(
                                ShapeDecoration::new(
                                    self.shape
                                        .clone()
                                        .or(tile_theme.shape.clone())
                                        .unwrap_or_else(|| ShapeBorderRef::new(Border::default())),
                                )
                                .color(effective_tile_color),
                            )
                            .child(
                                SafeArea::new(IconTheme::merge(
                                    None,
                                    icon_theme_data,
                                    IconButtonTheme::new(icon_button_theme_data, raw_list_tile),
                                ))
                                .top(false)
                                .bottom(false)
                                .minimum(resolved_content_padding),
                            ),
                    ),
            );
        ink_well.ink_response.custom_border = self.shape.clone().or(tile_theme.shape.clone());
        ink_well.ink_response.on_tap = if self.enabled {
            self.on_tap.clone()
        } else {
            None
        };
        ink_well.ink_response.on_long_press = if self.enabled {
            self.on_long_press.clone()
        } else {
            None
        };
        ink_well.ink_response.on_focus_change = self.on_focus_change.clone();
        ink_well.ink_response.focus_node = self.focus_node.clone();
        ink_well.ink_response.focus_color = self.focus_color;
        ink_well.ink_response.hover_color = self.hover_color;
        ink_well.ink_response.splash_color = self.splash_color;
        ink_well.ink_response.states_controller = self.states_controller.clone();
        ink_well.into_widget()
    }
}

/// Dart's `style.copyWith(color: color)`: the color is kept when `color` is null.
fn with_color(style: &TextStyle, color: Option<Color>) -> TextStyle {
    match color {
        Some(color) => style.copy_with().color(color),
        None => style.copy_with(),
    }
}

impl Debug for ListTile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("ListTile");
        properties.field(
            "is_three_line",
            &match self.is_three_line {
                Some(true) => "THREE_LINE",
                Some(false) => "TWO_LINE",
                None => "null",
            },
        );
        properties.field("dense", &self.dense);
        if let Some(visual_density) = &self.visual_density {
            properties.field("visual_density", visual_density);
        }
        if let Some(shape) = &self.shape {
            properties.field("shape", shape);
        }
        if let Some(style) = &self.style {
            properties.field("style", style);
        }
        if let Some(selected_color) = &self.selected_color {
            properties.field("selected_color", selected_color);
        }
        if let Some(icon_color) = &self.icon_color {
            properties.field("icon_color", icon_color);
        }
        if let Some(text_color) = &self.text_color {
            properties.field("text_color", text_color);
        }
        if let Some(title_text_style) = &self.title_text_style {
            properties.field("title_text_style", title_text_style);
        }
        if let Some(subtitle_text_style) = &self.subtitle_text_style {
            properties.field("subtitle_text_style", subtitle_text_style);
        }
        if let Some(style) = &self.leading_and_trailing_text_style {
            properties.field("leading_and_trailing_text_style", style);
        }
        if let Some(content_padding) = &self.content_padding {
            properties.field("content_padding", content_padding);
        }
        if !self.enabled {
            properties.field("enabled", &false);
        }
        if let Some(on_tap) = &self.on_tap {
            properties.field("on_tap", on_tap);
        }
        if let Some(on_long_press) = &self.on_long_press {
            properties.field("on_long_press", on_long_press);
        }
        if let Some(mouse_cursor) = &self.mouse_cursor {
            properties.field("mouse_cursor", mouse_cursor);
        }
        if self.selected {
            properties.field("selected", &true);
        }
        if let Some(focus_color) = &self.focus_color {
            properties.field("focus_color", focus_color);
        }
        if let Some(hover_color) = &self.hover_color {
            properties.field("hover_color", hover_color);
        }
        if let Some(focus_node) = &self.focus_node {
            properties.field("focus_node", focus_node);
        }
        if self.autofocus {
            properties.field("autofocus", &true);
        }
        if let Some(tile_color) = &self.tile_color {
            properties.field("tile_color", tile_color);
        }
        if let Some(selected_tile_color) = &self.selected_tile_color {
            properties.field("selected_tile_color", selected_tile_color);
        }
        properties.field("enable_feedback", &self.enable_feedback);
        if let Some(horizontal_title_gap) = &self.horizontal_title_gap {
            properties.field("horizontal_title_gap", horizontal_title_gap);
        }
        if let Some(min_vertical_padding) = &self.min_vertical_padding {
            properties.field("min_vertical_padding", min_vertical_padding);
        }
        if let Some(min_leading_width) = &self.min_leading_width {
            properties.field("min_leading_width", min_leading_width);
        }
        if let Some(title_alignment) = &self.title_alignment {
            properties.field("title_alignment", title_alignment);
        }
        properties.finish()
    }
}

/// Dart's `_IndividualOverrides`.
#[derive(Debug)]
struct IndividualOverrides {
    explicit_color: Option<AnyColor>,
    enabled_color: Option<Color>,
    selected_color: Option<Color>,
    disabled_color: Option<Color>,
}

impl WidgetStateProperty<Option<Color>> for IndividualOverrides {
    fn resolve(&self, states: &WidgetStates) -> Option<Color> {
        if let Some(explicit_color) = &self.explicit_color
            && explicit_color.extension::<WidgetStateColor>().is_some()
        {
            return <dyn WidgetStateProperty<Color>>::resolve_as(&self.explicit_color, states)
                .map(|color| color.color());
        }
        if states.contains(&WidgetState::Disabled) {
            return self.disabled_color;
        }
        if states.contains(&WidgetState::Selected) {
            return self.selected_color;
        }
        self.enabled_color
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Identifies the children of a _ListTileElement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ListTileSlot {
    Leading,
    Title,
    Subtitle,
    Trailing,
}

/// Dart's `_ListTile`.
#[derive(Debug)]
struct RawListTile {
    leading: Option<WidgetRef>,
    title: WidgetRef,
    subtitle: Option<WidgetRef>,
    trailing: Option<WidgetRef>,
    is_three_line: bool,
    is_dense: bool,
    visual_density: VisualDensity,
    text_direction: TextDirection,
    title_baseline_type: TextBaseline,
    subtitle_baseline_type: Option<TextBaseline>,
    horizontal_title_gap: f64,
    min_vertical_padding: f64,
    min_leading_width: f64,
    min_tile_height: Option<f64>,
    title_alignment: ListTileTitleAlignment,
}

impl SlottedMultiChildRenderObjectWidget for RawListTile {
    type Slot = ListTileSlot;

    fn slots(&self) -> Vec<ListTileSlot> {
        vec![
            ListTileSlot::Leading,
            ListTileSlot::Title,
            ListTileSlot::Subtitle,
            ListTileSlot::Trailing,
        ]
    }

    fn child_for_slot(&self, slot: &ListTileSlot) -> Option<WidgetRef> {
        match slot {
            ListTileSlot::Leading => self.leading.clone(),
            ListTileSlot::Title => Some(self.title.clone()),
            ListTileSlot::Subtitle => self.subtitle.clone(),
            ListTileSlot::Trailing => self.trailing.clone(),
        }
    }
}

impl RenderObjectWidget for RawListTile {
    type RenderObject = RenderListTile;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderListTile::new(
            app,
            self.is_dense,
            self.visual_density,
            self.is_three_line,
            self.text_direction,
            self.title_baseline_type,
            self.subtitle_baseline_type,
            self.horizontal_title_gap,
            self.min_vertical_padding,
            self.min_leading_width,
            self.min_tile_height,
            self.title_alignment,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderListTile>,
    ) {
        render_object.set_is_three_line(app, self.is_three_line);
        render_object.set_is_dense(app, self.is_dense);
        render_object.set_visual_density(app, self.visual_density);
        render_object.set_text_direction(app, self.text_direction);
        render_object.set_title_baseline_type(app, self.title_baseline_type);
        render_object.set_subtitle_baseline_type(app, self.subtitle_baseline_type);
        render_object.set_horizontal_title_gap(app, self.horizontal_title_gap);
        render_object.set_min_leading_width(app, self.min_leading_width);
        render_object.set_min_tile_height(app, self.min_tile_height);
        render_object.set_min_vertical_padding(app, self.min_vertical_padding);
        render_object.set_title_alignment(app, self.title_alignment);
    }
}

/// Dart's `_RenderListTile`.
struct RenderListTile {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    slotted: SlottedContainerRenderObjectData<ListTileSlot, AnyRenderBox>,
    is_dense: bool,
    visual_density: VisualDensity,
    is_three_line: bool,
    text_direction: TextDirection,
    title_baseline_type: TextBaseline,
    subtitle_baseline_type: Option<TextBaseline>,
    horizontal_title_gap: f64,
    min_vertical_padding: f64,
    min_leading_width: f64,
    min_tile_height: Option<f64>,
    title_alignment: ListTileTitleAlignment,
}

/// The getter and the setter of one of `_RenderListTile`'s properties, the setter marking
/// the tile as needing layout when the value changes.
macro_rules! render_list_tile_property {
    ($($field:ident, $setter:ident: $ty:ty;)*) => {
        $(
            fn $field(self: RenderHandle<Self>, app: &App) -> $ty {
                self.get(app).$field
            }

            fn $setter(self: RenderHandle<Self>, app: &mut App, value: $ty) {
                if self.get(app).$field == value {
                    return;
                }
                self.get_mut(app).$field = value;
                self.mark_needs_layout(app);
            }
        )*
    };
}

impl RenderListTile {
    #[allow(clippy::too_many_arguments)]
    fn new(
        app: &mut App,
        is_dense: bool,
        visual_density: VisualDensity,
        is_three_line: bool,
        text_direction: TextDirection,
        title_baseline_type: TextBaseline,
        subtitle_baseline_type: Option<TextBaseline>,
        horizontal_title_gap: f64,
        min_vertical_padding: f64,
        min_leading_width: f64,
        min_tile_height: Option<f64>,
        title_alignment: ListTileTitleAlignment,
    ) -> RenderHandle<RenderListTile> {
        RenderHandle::new_box(
            app,
            RenderListTile {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                slotted: SlottedContainerRenderObjectData::new(),
                is_dense,
                visual_density,
                is_three_line,
                text_direction,
                title_baseline_type,
                subtitle_baseline_type,
                horizontal_title_gap,
                min_vertical_padding,
                min_leading_width,
                min_tile_height,
                title_alignment,
            },
        )
    }

    fn leading(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &ListTileSlot::Leading)
    }

    fn title(self: RenderHandle<Self>, app: &App) -> AnyRenderBox {
        self.child_for_slot(app, &ListTileSlot::Title)
            .expect("a list tile always has a title")
    }

    fn subtitle(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &ListTileSlot::Subtitle)
    }

    fn trailing(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &ListTileSlot::Trailing)
    }

    render_list_tile_property! {
        is_dense, set_is_dense: bool;
        visual_density, set_visual_density: VisualDensity;
        is_three_line, set_is_three_line: bool;
        text_direction, set_text_direction: TextDirection;
        title_baseline_type, set_title_baseline_type: TextBaseline;
        subtitle_baseline_type, set_subtitle_baseline_type: Option<TextBaseline>;
        horizontal_title_gap, set_horizontal_title_gap: f64;
        min_vertical_padding, set_min_vertical_padding: f64;
        min_leading_width, set_min_leading_width: f64;
        min_tile_height, set_min_tile_height: Option<f64>;
        title_alignment, set_title_alignment: ListTileTitleAlignment;
    }

    fn effective_horizontal_title_gap(self: RenderHandle<Self>, app: &App) -> f64 {
        self.horizontal_title_gap(app) + self.visual_density(app).horizontal * 2.0
    }

    fn min_width(app: &mut App, r#box: Option<AnyRenderBox>, height: f64) -> f64 {
        match r#box {
            None => 0.0,
            Some(r#box) => r#box.get_min_intrinsic_width(app, height),
        }
    }

    fn max_width(app: &mut App, r#box: Option<AnyRenderBox>, height: f64) -> f64 {
        match r#box {
            None => 0.0,
            Some(r#box) => r#box.get_max_intrinsic_width(app, height),
        }
    }

    // The target tile height to use if _minTileHeight is not specified.
    fn default_tile_height(self: RenderHandle<Self>, app: &App) -> f64 {
        let base_density = self.visual_density(app).base_size_adjustment();
        let is_dense = self.is_dense(app);
        base_density.dy()
            + match (self.is_three_line(app), self.subtitle(app).is_some()) {
                (true, _) => {
                    if is_dense {
                        76.0
                    } else {
                        88.0
                    }
                } // 3 lines,
                (false, true) => {
                    if is_dense {
                        64.0
                    } else {
                        72.0
                    }
                } // 2 lines
                (false, false) => {
                    if is_dense {
                        48.0
                    } else {
                        56.0
                    }
                } // 1 line,
            }
    }

    fn target_tile_height(self: RenderHandle<Self>, app: &App) -> f64 {
        self.min_tile_height(app)
            .unwrap_or_else(|| self.default_tile_height(app))
    }

    fn max_icon_height_constraint(self: RenderHandle<Self>, app: &App) -> BoxConstraints {
        BoxConstraints {
            // One-line trailing and leading widget heights do not follow
            // Material specifications, but this sizing is required to adhere
            // to accessibility requirements for smallest tappable widget.
            // Two- and three-line trailing widget heights are constrained
            // properly according to the Material spec.
            max_height: (if self.is_dense(app) { 48.0 } else { 56.0 })
                + self.visual_density(app).base_size_adjustment().dy(),
            ..BoxConstraints::new()
        }
    }

    fn position_box(app: &mut App, r#box: AnyRenderBox, offset: Offset) {
        r#box.parent_data_of_mut::<BoxParentData>(app).offset = offset;
    }

    // Implements _RenderListTile's layout algorithm. If `positionChild` is not null,
    // it will be called on each child with that child's layout offset.
    //
    // All of the dimensions below were taken from the Material Design spec:
    // https://material.io/design/components/lists.html#specs
    fn compute_sizes(
        self: RenderHandle<Self>,
        app: &mut App,
        get_baseline: ChildBaselineGetter,
        get_size: ChildLayouter,
        constraints: BoxConstraints,
        position_child: Option<PositionChild>,
    ) -> Sizes {
        let loose_constraints = constraints.loosen();
        let tile_width = loose_constraints.max_width;
        let icon_constraints = loose_constraints.enforce(self.max_icon_height_constraint(app));
        let leading = self.leading(app);
        let trailing = self.trailing(app);

        let leading_size = leading.map(|leading| get_size(app, leading, icon_constraints));
        let trailing_size = trailing.map(|trailing| get_size(app, trailing, icon_constraints));

        if cfg!(debug_assertions) && tile_width != 0.0 {
            let overflowed_widget = if Some(tile_width) == leading_size.map(|size| size.width()) {
                Some("Leading")
            } else if Some(tile_width) == trailing_size.map(|size| size.width()) {
                Some("Trailing")
            } else {
                None
            };

            if let Some(overflowed_widget) = overflowed_widget {
                panic!(
                    "{overflowed_widget} widget consumes the entire tile width (including \
                     ListTile.contentPadding).\nEither resize the tile width so that the {} \
                     widget plus any content padding do not exceed the tile width, or use a \
                     sized widget, or consider replacing ListTile with a custom widget.\nSee \
                     also: https://api.flutter.dev/flutter/material/ListTile-class.html#material.ListTile.4",
                    overflowed_widget.to_lowercase()
                );
            }
        }

        let title_start = match leading_size {
            None => 0.0,
            Some(leading_size) => {
                self.min_leading_width(app).max(leading_size.width())
                    + self.effective_horizontal_title_gap(app)
            }
        };

        let adjusted_trailing_width = match trailing_size {
            None => 0.0,
            Some(trailing_size) => {
                (trailing_size.width() + self.effective_horizontal_title_gap(app)).max(32.0)
            }
        };

        let text_constraints = loose_constraints.tighten(
            Some(tile_width - title_start - adjusted_trailing_width),
            None,
        );

        let subtitle = self.subtitle(app);
        let title = self.title(app);
        let title_height = get_size(app, title, text_constraints).height();

        let is_ltr = match self.text_direction(app) {
            TextDirection::Ltr => true,
            TextDirection::Rtl => false,
        };

        let title_y;
        let tile_height;
        let min_vertical_padding = self.min_vertical_padding(app);
        match subtitle {
            None => {
                tile_height = self
                    .target_tile_height(app)
                    .max(title_height + 2.0 * min_vertical_padding);
                title_y = (tile_height - title_height) / 2.0;
            }
            Some(subtitle) => {
                let subtitle_height = get_size(app, subtitle, text_constraints).height();
                let title_baseline_type = self.title_baseline_type(app);
                let title_baseline =
                    get_baseline(app, title, text_constraints, title_baseline_type)
                        .unwrap_or(title_height);
                let subtitle_baseline_type = self
                    .subtitle_baseline_type(app)
                    .expect("a list tile with a subtitle has a subtitle baseline type");
                let subtitle_baseline =
                    get_baseline(app, subtitle, text_constraints, subtitle_baseline_type)
                        .unwrap_or(subtitle_height);

                let is_three_line = self.is_three_line(app);
                let is_dense = self.is_dense(app);
                let target_title_y = (if is_three_line {
                    if is_dense { 22.0 } else { 28.0 }
                } else if is_dense {
                    28.0
                } else {
                    32.0
                }) - title_baseline;
                let target_subtitle_y = (if is_three_line {
                    if is_dense { 42.0 } else { 48.0 }
                } else if is_dense {
                    48.0
                } else {
                    52.0
                }) + self.visual_density(app).vertical * 2.0
                    - subtitle_baseline;
                // Prevent the title and the subtitle from overlapping by moving them away from
                // each other by the same distance.
                let half_overlap =
                    (target_title_y + title_height - target_subtitle_y).max(0.0) / 2.0;
                let ideal_title_y = target_title_y - half_overlap;
                let ideal_subtitle_y = target_subtitle_y + half_overlap;
                // However if either component can't maintain the minimal padding from the top/bottom edges, the ListTile enters "compat mode".
                let target_tile_height = self.target_tile_height(app);
                let compact = ideal_title_y < min_vertical_padding
                    || ideal_subtitle_y + subtitle_height + min_vertical_padding
                        > target_tile_height;

                // Position subtitle.
                if let Some(position_child) = position_child {
                    position_child(
                        app,
                        subtitle,
                        Offset::new(
                            if is_ltr {
                                title_start
                            } else {
                                adjusted_trailing_width
                            },
                            if compact {
                                min_vertical_padding + title_height
                            } else {
                                ideal_subtitle_y
                            },
                        ),
                    );
                }
                tile_height = if compact {
                    2.0 * min_vertical_padding + title_height + subtitle_height
                } else {
                    target_tile_height
                };
                title_y = if compact {
                    min_vertical_padding
                } else {
                    ideal_title_y
                };
            }
        }

        if let Some(position_child) = position_child {
            position_child(
                app,
                title,
                Offset::new(
                    if is_ltr {
                        title_start
                    } else {
                        adjusted_trailing_width
                    },
                    title_y,
                ),
            );

            let title_alignment = self.title_alignment(app);
            if let (Some(leading), Some(leading_size)) = (leading, leading_size) {
                let dy = title_alignment.y_offset_for(
                    app,
                    leading_size.height(),
                    tile_height,
                    self,
                    true,
                );
                position_child(
                    app,
                    leading,
                    Offset::new(
                        if is_ltr {
                            0.0
                        } else {
                            tile_width - leading_size.width()
                        },
                        dy,
                    ),
                );
            }

            if let (Some(trailing), Some(trailing_size)) = (trailing, trailing_size) {
                let dy = title_alignment.y_offset_for(
                    app,
                    trailing_size.height(),
                    tile_height,
                    self,
                    false,
                );
                position_child(
                    app,
                    trailing,
                    Offset::new(
                        if is_ltr {
                            tile_width - trailing_size.width()
                        } else {
                            0.0
                        },
                        dy,
                    ),
                );
            }
        }

        Sizes {
            title_y,
            text_constraints,
            tile_size: Size::new(tile_width, tile_height),
        }
    }
}

impl SlottedContainerRenderObjectMixin<ListTileSlot> for RenderListTile {
    type ChildType = AnyRenderBox;

    fn slotted_container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &SlottedContainerRenderObjectData<ListTileSlot, AnyRenderBox> {
        &self.get(app).slotted
    }

    fn slotted_container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut SlottedContainerRenderObjectData<ListTileSlot, AnyRenderBox> {
        &mut self.get_mut(app).slotted
    }

    // The returned list is ordered for hit testing.
    fn children(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        let title = self.child_for_slot(app, &ListTileSlot::Title);
        [
            self.leading(app),
            title,
            self.subtitle(app),
            self.trailing(app),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

impl RenderObject for RenderListTile {
    waft_rendering::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        SlottedContainerRenderObjectMixin::did_attach(self, app, owner);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        SlottedContainerRenderObjectMixin::did_detach(self, app);
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        SlottedContainerRenderObjectMixin::redepth_children(self, app);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        SlottedContainerRenderObjectMixin::visit_children(self, app, visitor);
    }

    fn sized_by_parent(self: RenderHandle<Self>, _app: &App) -> bool {
        false
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let tile_size = self
            .compute_sizes(
                app,
                ChildLayoutHelper::get_baseline,
                ChildLayoutHelper::layout_child,
                constraints,
                Some(RenderListTile::position_box),
            )
            .tile_size;

        let size = constraints.constrain(tile_size);
        self.set_size(app, size);
        debug_assert!(size.width() == constraints.constrain_width(tile_size.width()));
        debug_assert!(size.height() == constraints.constrain_height(tile_size.height()));
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let mut do_paint = |app: &mut App, child: Option<AnyRenderBox>| {
            if let Some(child) = child {
                let child_offset = child.box_parent_data(app).offset;
                context.paint_child(app, child.as_object(), child_offset + offset);
            }
        };

        do_paint(app, self.leading(app));
        do_paint(app, Some(self.title(app)));
        do_paint(app, self.subtitle(app));
        do_paint(app, self.trailing(app));
    }
}

impl RenderBox for RenderListTile {
    waft_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let leading_width = match self.leading(app) {
            Some(leading) => {
                leading
                    .get_min_intrinsic_width(app, height)
                    .max(self.min_leading_width(app))
                    + self.effective_horizontal_title_gap(app)
            }
            None => 0.0,
        };
        let title = Some(self.title(app));
        let subtitle = self.subtitle(app);
        let trailing = self.trailing(app);
        leading_width
            + RenderListTile::min_width(app, title, height)
                .max(RenderListTile::min_width(app, subtitle, height))
            + RenderListTile::max_width(app, trailing, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let leading_width = match self.leading(app) {
            Some(leading) => {
                leading
                    .get_max_intrinsic_width(app, height)
                    .max(self.min_leading_width(app))
                    + self.effective_horizontal_title_gap(app)
            }
            None => 0.0,
        };
        let title = Some(self.title(app));
        let subtitle = self.subtitle(app);
        let trailing = self.trailing(app);
        leading_width
            + RenderListTile::max_width(app, title, height)
                .max(RenderListTile::max_width(app, subtitle, height))
            + RenderListTile::max_width(app, trailing, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let title_min_height = self.title(app).get_min_intrinsic_height(app, width);
        let subtitle_min_height = self
            .subtitle(app)
            .map(|subtitle| subtitle.get_min_intrinsic_height(app, width));

        const TOP_AND_BOTTOM_PADDING_MULTIPLIER: f64 = 2.0;
        let content_height = title_min_height
            + subtitle_min_height.unwrap_or(0.0)
            + TOP_AND_BOTTOM_PADDING_MULTIPLIER * self.min_vertical_padding(app);

        self.target_tile_height(app).max(content_height)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.as_box().get_min_intrinsic_height(app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let title = self.title(app);
        let offset_dy = title.box_parent_data(app).offset.dy();
        let offset =
            BaselineOffset(title.get_distance_to_actual_baseline(app, baseline)) + offset_dy;
        offset.offset()
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let sizes = self.compute_sizes(
            app,
            ChildLayoutHelper::get_dry_baseline,
            ChildLayoutHelper::dry_layout_child,
            constraints,
            None,
        );
        let title_baseline = BaselineOffset(self.title(app).get_dry_baseline(
            app,
            sizes.text_constraints,
            baseline,
        )) + sizes.title_y;
        title_baseline.offset()
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.constrain(
            self.compute_sizes(
                app,
                ChildLayoutHelper::get_dry_baseline,
                ChildLayoutHelper::dry_layout_child,
                constraints,
                None,
            )
            .tile_size,
        )
    }

    fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
        true
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        for child in SlottedContainerRenderObjectMixin::children(self, app) {
            let offset = child.box_parent_data(app).offset;
            let is_hit =
                result.add_with_paint_offset(Some(offset), position, |result, transformed| {
                    debug_assert!(transformed == position - offset);
                    child.hit_test(app, result, transformed)
                });
            if is_hit {
                return true;
            }
        }
        false
    }
}

/// Dart's `_LisTileDefaultsM2`.
struct LisTileDefaultsM2 {
    /// Dart's `super(..)` arguments.
    list_tile_theme_data: ListTileThemeData,
    context: DefaultsContext,
}

impl LisTileDefaultsM2 {
    fn new(context: BuildContext, style: ListTileStyle) -> LisTileDefaultsM2 {
        LisTileDefaultsM2 {
            list_tile_theme_data: ListTileThemeData::new()
                .content_padding(EdgeInsets::symmetric(0.0, 16.0))
                .min_leading_width(40.0)
                .min_vertical_padding(4.0)
                .shape(Border::default())
                .style(style),
            context: DefaultsContext::new(context),
        }
    }
}

impl ListTileThemeDataGetters for LisTileDefaultsM2 {
    fn list_tile_theme_data(&self) -> &ListTileThemeData {
        &self.list_tile_theme_data
    }

    fn tile_color(&self, _app: &mut App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn title_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let text_theme = self.context.text_theme(app);
        match self
            .list_tile_theme_data
            .style
            .expect("the constructor sets a style")
        {
            ListTileStyle::Drawer => text_theme.body_large.clone(),
            ListTileStyle::List => text_theme.title_medium.clone(),
        }
    }

    fn subtitle_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let text_theme = self.context.text_theme(app);
        let body_medium = text_theme
            .body_medium
            .as_ref()
            .expect("the text theme has a body medium style");
        let body_small = text_theme
            .body_small
            .as_ref()
            .expect("the text theme has a body small style");
        Some(match &body_small.color {
            Some(color) => body_medium.copy_with().color(color.clone()),
            None => body_medium.copy_with(),
        })
    }

    fn leading_and_trailing_text_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context.text_theme(app).body_medium.clone()
    }

    fn selected_color(&self, app: &mut App) -> Option<Color> {
        Some(self.context.colors(app).primary)
    }

    fn icon_color(&self, app: &mut App) -> Option<AnyColor> {
        match self.context.theme(app).brightness() {
            // For the sake of backwards compatibility, the default for unselected
            // tiles is Colors.black45 rather than colorScheme.onSurface.withAlpha(0x73).
            Brightness::Light => Some(AnyColor::new(Colors::BLACK45)),
            // null -> use current icon theme color
            Brightness::Dark => None,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;
    use waft_painting::OutlinedBorderRef;

    use waft_embedder::{RRect, Rect};
    use waft_foundation::{Listenable, Listener};
    use waft_painting::{
        AlignmentGeometry, EdgeInsetsDirectional, RoundedRectangleBorder, StadiumBorder, TextScaler,
    };
    use waft_services::SystemMouseCursors;
    use waft_widgets::{
        Builder, Center, Container, FocusHighlightStrategy, FocusNode, GlobalKey, ListView,
        MediaQuery, MediaQueryData, MouseRegion, StatefulBuilder, Text,
    };

    use waft_widgets::testing::{
        TestPointer, WidgetTester, expect, find, finds_one_widget, test_widgets,
    };

    use super::*;
    use crate::colors::Colors;
    use crate::scaffold::Scaffold;
    use crate::test_support::{PaintInspection, material_app};
    use waft_rendering::testing::{PaintCall, assert_paints, draws, paints_exactly};
    use waft_widgets::AnyFocusNode;

    /// Dart's `ThemeData()`.
    pub(crate) fn m3(app: &App) -> ThemeData {
        ThemeData::builder().build(app)
    }

    /// Dart's `ThemeData(useMaterial3: false)`.
    pub(crate) fn m2(app: &App) -> ThemeData {
        ThemeData::builder().use_material3(false).build(app)
    }

    /// Dart's `tester.pumpWidget(MaterialApp(theme: theme, home: home))`.
    pub(crate) fn pump_app(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        home: impl FnOnce(&mut App) -> WidgetRef,
    ) {
        let home = home(&mut tester.cell.borrow_mut());
        let theme = theme(&tester.cell.borrow());
        tester.pump_widget(material_app(theme, home));
    }

    /// Dart's `tester.pumpWidget(MaterialApp(key: UniqueKey(), theme: theme, home: home))`: a
    /// new app each frame, so the theme does not animate from the last frame's.
    pub(crate) fn pump_unique_app(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        home: impl FnOnce(&mut App) -> WidgetRef,
    ) {
        let home = home(&mut tester.cell.borrow_mut());
        let theme = theme(&tester.cell.borrow());
        tester.pump_widget(
            crate::app::MaterialApp::new()
                .key(waft_foundation::UniqueKey::new().into())
                .theme(theme)
                .home(home),
        );
    }

    /// Dart's `tester.getRect(find.byType(ListTile).at(index))`.
    pub(crate) fn tile_rect(tester: &WidgetTester, index: usize) -> Rect {
        tester.get_rect(tester.element(find::by_type::<ListTile>().at(index)))
    }

    /// Dart's `tester.getRect(find.byKey(key))`.
    pub(crate) fn key_rect(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        tester.get_rect(key)
    }

    pub(crate) fn sized(key: &GlobalKey, width: f64, height: f64) -> WidgetRef {
        SizedBox::new()
            .key(key.clone().into())
            .width(width)
            .height(height)
            .into_widget()
    }

    pub(crate) fn noop() -> Listener {
        Listener::new(|_app| {})
    }

    /// One frame of "ListTile geometry (LTR)".
    #[derive(Clone, Copy)]
    struct GeometryFrame {
        dense: bool,
        is_two_line: bool,
        is_three_line: bool,
        text_scale: f64,
        subtitle_scale: Option<f64>,
    }

    impl GeometryFrame {
        const PLAIN: GeometryFrame = GeometryFrame {
            dense: false,
            is_two_line: false,
            is_three_line: false,
            text_scale: 1.0,
            subtitle_scale: None,
        };
    }

    const LEFT_PADDING: f64 = 10.0;
    const RIGHT_PADDING: f64 = 20.0;

    /// Pumps one frame of "ListTile geometry (LTR)" on `tester`, under a bare theme so that
    /// its text styles animate from the last frame's as Dart's do, and checks it, `right_content_padding`
    /// being the theme's default end padding and `title_offset` how far the title starts from
    /// the start padding.
    fn check_ltr_geometry(
        tester: &mut WidgetTester,
        [leading_key, trailing_key]: &[GlobalKey; 2],
        theme: fn(&App) -> ThemeData,
        frame: GeometryFrame,
        expected_height: f64,
        right_content_padding: f64,
        title_offset: f64,
    ) {
        let has_subtitle = frame.is_two_line || frame.is_three_line;
        let subtitle_scale = frame.subtitle_scale.unwrap_or(frame.text_scale);
        let mut tile = ListTile::new()
            .leading(sized(leading_key, 24.0, 24.0))
            .title(Text::new("title"))
            .trailing(sized(trailing_key, 24.0, 24.0))
            .dense(frame.dense);
        if has_subtitle {
            tile = tile
                .subtitle(Text::new("subtitle").text_scaler(TextScaler::linear(subtitle_scale)));
        }
        tile = tile.is_three_line(frame.is_three_line);
        let tree = bare(
            theme(&tester.cell.borrow()),
            MediaQuery::new(
                MediaQueryData::new()
                    .padding(EdgeInsets::only(LEFT_PADDING, 0.0, RIGHT_PADDING, 0.0))
                    .text_scaler(TextScaler::linear(frame.text_scale)),
                Material::new().child(Center::new().child(tile)),
            )
            .into_widget(),
        );
        tester.pump_widget(tree);
        let tester = &*tester;

        // ListTiles are contained by a SafeArea defined like this:
        // SafeArea(top: false, bottom: false, minimum: contentPadding)
        // The default contentPadding is 16.0 on the left and 24.0 on the right (16.0 in
        // Material 2).
        let leading = key_rect(tester, leading_key);
        let trailing = key_rect(tester, trailing_key);
        let title = tester.get_rect(find::text("title"));
        assert_eq!(leading.left, 16.0_f64.max(LEFT_PADDING));
        assert_eq!(title.left, title_offset + 16.0_f64.max(LEFT_PADDING));
        if has_subtitle {
            assert_eq!(
                tester.get_rect(find::text("subtitle")).left,
                title_offset + 16.0_f64.max(LEFT_PADDING)
            );
        }
        assert_eq!(title.left, leading.right + title_offset - 24.0);
        assert_eq!(
            trailing.right,
            800.0 - right_content_padding.max(RIGHT_PADDING)
        );
        assert_eq!(trailing.width(), 24.0);

        let tile = tile_rect(tester, 0);
        assert_eq!(tile.size(), Size::new(800.0, expected_height));
        assert!(title.top >= tile.top);
        if has_subtitle {
            let subtitle = tester.get_rect(find::text("subtitle"));
            assert!(subtitle.top >= title.bottom);
            assert!(subtitle.bottom < tile.bottom);
        } else {
            assert_eq!(title.top, tile.top + (tile.height() - title.height()) / 2.0);
        }
        assert_eq!(trailing.height(), 24.0);
    }

    /// "ListTile geometry (LTR)", Material 3. The FlutterTest font gives Flutter's
    /// line heights and baselines, so the tile heights are Flutter's.
    #[test]
    fn list_tile_geometry_ltr() {
        test_widgets(|tester| {
            let frames = [
                (GeometryFrame::PLAIN, 56.0),
                (
                    GeometryFrame {
                        is_two_line: true,
                        ..GeometryFrame::PLAIN
                    },
                    72.0,
                ),
                (
                    GeometryFrame {
                        is_three_line: true,
                        ..GeometryFrame::PLAIN
                    },
                    88.0,
                ),
                (
                    GeometryFrame {
                        text_scale: 4.0,
                        ..GeometryFrame::PLAIN
                    },
                    112.0,
                ),
                (
                    GeometryFrame {
                        is_two_line: true,
                        text_scale: 4.0,
                        ..GeometryFrame::PLAIN
                    },
                    192.0,
                ),
                // Make sure that the height of a large subtitle is taken into account.
                (
                    GeometryFrame {
                        is_two_line: true,
                        text_scale: 0.5,
                        subtitle_scale: Some(4.0),
                        ..GeometryFrame::PLAIN
                    },
                    108.0,
                ),
                (
                    GeometryFrame {
                        is_three_line: true,
                        text_scale: 4.0,
                        ..GeometryFrame::PLAIN
                    },
                    192.0,
                ),
            ];
            let keys = [GlobalKey::new(), GlobalKey::new()];
            for (frame, expected_height) in frames {
                check_ltr_geometry(tester, &keys, m3, frame, expected_height, 24.0, 40.0);
            }
        });
    }

    /// "ListTile geometry (RTL)".
    #[test]
    fn list_tile_geometry_rtl() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                MediaQuery::new(
                    MediaQueryData::new().padding(EdgeInsets::only(
                        LEFT_PADDING,
                        0.0,
                        RIGHT_PADDING,
                        0.0,
                    )),
                    Directionality::new(
                        TextDirection::Rtl,
                        Material::new().child(
                            Center::new().child(
                                ListTile::new()
                                    .leading(Text::new("L"))
                                    .title(Text::new("title"))
                                    .trailing(Text::new("T")),
                            ),
                        ),
                    ),
                )
                .into_widget()
            });
            assert_eq!(
                tester.get_rect(find::text("L")).right,
                800.0 - 16.0_f64.max(RIGHT_PADDING)
            );
            assert_eq!(
                tester.get_rect(find::text("title")).right,
                800.0 - 40.0 - 16.0_f64.max(RIGHT_PADDING)
            );
            assert_eq!(
                tester.get_rect(find::text("T")).left,
                24.0_f64.max(LEFT_PADDING)
            );
        });
    }

    /// "ListTile.divideTiles".
    #[test]
    fn list_tile_divide_tiles() {
        test_widgets(|tester| {
            let titles = ["first", "second", "third"];
            pump_app(tester, m3, |_app| {
                Material::new()
                    .child(Builder::new(move |app, context| {
                        let tiles = titles
                            .map(|title| ListTile::new().title(Text::new(title)).into_widget());
                        ListView::new()
                            .children(ListTile::divide_tiles(app, Some(context), tiles, None))
                            .into_widget()
                    }))
                    .into_widget()
            });
            expect(tester, &find::text("first"), finds_one_widget());
            expect(tester, &find::text("second"), finds_one_widget());
            expect(tester, &find::text("third"), finds_one_widget());
        });
    }

    /// "ListTile.divideTiles with empty list".
    #[test]
    fn list_tile_divide_tiles_with_empty_list() {
        test_widgets(|tester| {
            let output = ListTile::divide_tiles(
                &mut tester.cell.borrow_mut(),
                None,
                Vec::new(),
                Some(Colors::GREY.primary()),
            );
            assert!(output.is_empty());
        });
    }

    /// "ListTile.divideTiles with single item list".
    #[test]
    fn list_tile_divide_tiles_with_single_item_list() {
        test_widgets(|tester| {
            let output = ListTile::divide_tiles(
                &mut tester.cell.borrow_mut(),
                None,
                vec![SizedBox::new().into_widget()],
                Some(Colors::GREY.primary()),
            );
            assert_eq!(output.len(), 1);
            assert!(downcast_widget::<SizedBox>(&*output[0]).is_some());
        });
    }

    /// "ListTile.divideTiles only runs the generator once". The generator is a lazy iterator
    /// whose body starts on its first `next`, as a Dart `sync*` body starts on each walk; Rust
    /// consumes an `IntoIterator` when it walks it, so a second walk cannot compile, and the
    /// case fails if `divide_tiles` does not walk it.
    #[test]
    fn list_tile_divide_tiles_only_runs_the_generator_once() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/pull/78879
            let call_count = Rc::new(Cell::new(0));
            let generator = || {
                let call_count = Rc::clone(&call_count);
                let mut yielded = 0;
                std::iter::from_fn(move || {
                    if yielded == 0 {
                        call_count.set(call_count.get() + 1);
                    }
                    (yielded < 2).then(|| {
                        yielded += 1;
                        Text::new("").into_widget()
                    })
                })
            };
            let output = ListTile::divide_tiles(
                &mut tester.cell.borrow_mut(),
                None,
                generator(),
                Some(Colors::GREY.primary()),
            );
            assert_eq!(output.len(), 2);
            assert_eq!(call_count.get(), 1);
        });
    }

    /// "ListTile contentPadding".
    #[test]
    fn list_tile_content_padding() {
        test_widgets(|tester| {
            let check = |tester: &mut WidgetTester, text_direction: TextDirection| {
                tester.pump_widget(
                    MediaQuery::new(
                        MediaQueryData::new(),
                        Directionality::new(
                            text_direction,
                            Material::new().child(
                                Container::new()
                                    .alignment(AlignmentGeometry::TOP_LEFT)
                                    .child(
                                        ListTile::new()
                                            .content_padding(EdgeInsetsDirectional::only(
                                                10.0, 30.0, 20.0, 40.0,
                                            ))
                                            .leading(Text::new("L"))
                                            .title(Text::new("title"))
                                            .trailing(Text::new("T")),
                                    ),
                            ),
                        ),
                    )
                    .into_widget(),
                );
            };

            check(tester, TextDirection::Ltr);
            // 126 = 56 + 30 + 40
            assert_eq!(tile_rect(tester, 0).size(), Size::new(800.0, 126.0));
            assert_eq!(tester.get_rect(find::text("L")).left, 10.0); // contentPadding.start = 10
            assert_eq!(tester.get_rect(find::text("T")).right, 780.0); // 800 - contentPadding.end

            check(tester, TextDirection::Rtl);
            // 126 = 56 + 30 + 40
            assert_eq!(tile_rect(tester, 0).size(), Size::new(800.0, 126.0));
            assert_eq!(tester.get_rect(find::text("T")).left, 20.0); // contentPadding.end = 20
            assert_eq!(tester.get_rect(find::text("L")).right, 790.0); // 800 - contentPadding.start
        });
    }

    /// One frame of "ListTile wide leading Widget".
    fn wide_leading(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        leading_width: f64,
        text_direction: TextDirection,
    ) -> GlobalKey {
        let leading_key = GlobalKey::new();
        pump_app(tester, theme, |_app| {
            Directionality::new(
                text_direction,
                Material::new().child(
                    Container::new()
                        .alignment(AlignmentGeometry::TOP_LEFT)
                        .child(
                            ListTile::new()
                                .content_padding(EdgeInsets::ZERO)
                                .leading(sized(&leading_key, leading_width, 32.0))
                                .title(Text::new("title"))
                                .subtitle(Text::new("subtitle")),
                        ),
                ),
            )
            .into_widget()
        });
        leading_key
    }

    fn check_wide_leading(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        leading_top: f64,
        narrow_title: f64,
    ) {
        // Two-line tile's height = 72, leading 24x32 widget is positioned in the center.
        let key = wide_leading(tester, theme, 24.0, TextDirection::Ltr);
        assert_eq!(tile_rect(tester, 0).size(), Size::new(800.0, 72.0));
        assert_eq!(
            key_rect(tester, &key).top_left(),
            Offset::new(0.0, leading_top)
        );
        assert_eq!(
            key_rect(tester, &key).bottom_right(),
            Offset::new(24.0, leading_top + 32.0)
        );

        // Leading widget's width is 20, so default layout: the left edges of the
        // title and subtitle are at 40dps, leading widget width is 24dp and 16dp
        // is horizontalTitleGap (contentPadding is zero).
        assert_eq!(tester.get_rect(find::text("title")).left, narrow_title);
        assert_eq!(tester.get_rect(find::text("subtitle")).left, narrow_title);

        // If the leading widget is wider than 40 it is separated from the
        // title and subtitle by 16.
        let key = wide_leading(tester, theme, 56.0, TextDirection::Ltr);
        assert_eq!(tile_rect(tester, 0).size(), Size::new(800.0, 72.0));
        assert_eq!(
            key_rect(tester, &key).top_left(),
            Offset::new(0.0, leading_top)
        );
        assert_eq!(
            key_rect(tester, &key).bottom_right(),
            Offset::new(56.0, leading_top + 32.0)
        );
        assert_eq!(tester.get_rect(find::text("title")).left, 72.0);
        assert_eq!(tester.get_rect(find::text("subtitle")).left, 72.0);

        // Same tests, textDirection = RTL

        let key = wide_leading(tester, theme, 24.0, TextDirection::Rtl);
        assert_eq!(tile_rect(tester, 0).size(), Size::new(800.0, 72.0));
        assert_eq!(
            key_rect(tester, &key).top_right(),
            Offset::new(800.0, leading_top)
        );
        assert_eq!(
            key_rect(tester, &key).bottom_left(),
            Offset::new(800.0 - 24.0, leading_top + 32.0)
        );
        assert_eq!(
            tester.get_rect(find::text("title")).right,
            800.0 - narrow_title
        );
        assert_eq!(
            tester.get_rect(find::text("subtitle")).right,
            800.0 - narrow_title
        );

        let key = wide_leading(tester, theme, 56.0, TextDirection::Rtl);
        assert_eq!(tile_rect(tester, 0).size(), Size::new(800.0, 72.0));
        assert_eq!(
            key_rect(tester, &key).top_right(),
            Offset::new(800.0, leading_top)
        );
        assert_eq!(
            key_rect(tester, &key).bottom_left(),
            Offset::new(800.0 - 56.0, leading_top + 32.0)
        );
        assert_eq!(tester.get_rect(find::text("title")).right, 800.0 - 72.0);
        assert_eq!(tester.get_rect(find::text("subtitle")).right, 800.0 - 72.0);
    }

    /// "ListTile wide leading Widget".
    #[test]
    fn list_tile_wide_leading_widget() {
        test_widgets(|tester| check_wide_leading(tester, m3, 20.0, 40.0));
    }

    /// Dart's tree without a `MaterialApp`: `theme` above `home`, with the directionality and
    /// media query `MaterialApp` would give. A tester can pump it again with a new `home`.
    pub(crate) fn bare<K>(theme: ThemeData, home: impl IntoWidget<K>) -> WidgetRef {
        MediaQuery::new(
            MediaQueryData::new(),
            Directionality::new(TextDirection::Ltr, crate::theme::Theme::new(theme, home)),
        )
        .into_widget()
    }

    /// One case of "ListTile leading and trailing positions": two tiles in a list.
    fn two_tiles(tester: &mut WidgetTester, theme: fn(&App) -> ThemeData, tiles: [ListTile; 2]) {
        pump_app(tester, theme, |_app| {
            Material::new()
                .child(ListView::new().children(tiles.map(IntoWidget::into_widget)))
                .into_widget()
        })
    }

    /// The `CircleAvatar` of the case: Waft has none, and a default one is a 40x40 box.
    pub(crate) fn avatar(key: &GlobalKey) -> WidgetRef {
        sized(key, 40.0, 40.0)
    }

    /// `SizedBox(height:, width:, child: Placeholder())`: Waft has no `Placeholder`, which
    /// fills the box, so the box's rect is the placeholder's.
    pub(crate) fn placeholder(key: &GlobalKey, width: f64, height: f64) -> WidgetRef {
        sized(key, width, height)
    }

    pub(crate) const THIRTEEN_LINES: &str = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM";

    /// "ListTile leading and trailing positions", Material 3.
    #[test]
    fn list_tile_leading_and_trailing_positions() {
        test_widgets(|tester| {
            let keys: [GlobalKey; 4] = std::array::from_fn(|_| GlobalKey::new());
            let [avatar0, avatar1, placeholder0, placeholder1] = &keys;

            // "ONE"-LINE
            two_tiles(
                tester,
                m3,
                [
                    ListTile::new()
                        .leading(avatar(avatar0))
                        .trailing(placeholder(placeholder0, 24.0, 24.0))
                        .title(Text::new(THIRTEEN_LINES)),
                    ListTile::new()
                        .leading(avatar(avatar1))
                        .trailing(placeholder(placeholder1, 24.0, 24.0))
                        .title(Text::new("A")),
                ],
            );
            tester.pump_by(Duration::from_secs(2)); // the text styles are animated when we change dense
            assert_eq!(
                tile_rect(tester, 0),
                Rect::from_ltwh(0.0, 0.0, 800.0, 328.0)
            );
            assert_eq!(
                key_rect(tester, avatar0),
                Rect::from_ltwh(16.0, 144.0, 40.0, 40.0)
            );
            assert_eq!(
                key_rect(tester, placeholder0),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, 152.0, 24.0, 24.0)
            );
            assert_eq!(
                tile_rect(tester, 1),
                Rect::from_ltwh(0.0, 328.0, 800.0, 56.0)
            );
            assert_eq!(
                key_rect(tester, avatar1),
                Rect::from_ltwh(16.0, 328.0 + 8.0, 40.0, 40.0)
            );
            assert_eq!(
                key_rect(tester, placeholder1),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, 328.0 + 16.0, 24.0, 24.0)
            );

            // "TWO"-LINE
            two_tiles(
                tester,
                m3,
                [
                    ListTile::new()
                        .leading(avatar(avatar0))
                        .trailing(placeholder(placeholder0, 24.0, 24.0))
                        .title(Text::new("A"))
                        .subtitle(Text::new(THIRTEEN_LINES)),
                    ListTile::new()
                        .leading(avatar(avatar1))
                        .trailing(placeholder(placeholder1, 24.0, 24.0))
                        .title(Text::new("A"))
                        .subtitle(Text::new("A")),
                ],
            );
            const HEIGHT: f64 = 300.0;
            const AVATAR_TOP: f64 = 130.0;
            const PLACEHOLDER_TOP: f64 = 138.0;
            assert_eq!(
                tile_rect(tester, 0),
                Rect::from_ltwh(0.0, 0.0, 800.0, HEIGHT)
            );
            assert_eq!(
                key_rect(tester, avatar0),
                Rect::from_ltwh(16.0, AVATAR_TOP, 40.0, 40.0)
            );
            assert_eq!(
                key_rect(tester, placeholder0),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, PLACEHOLDER_TOP, 24.0, 24.0)
            );
            assert_eq!(
                tile_rect(tester, 1),
                Rect::from_ltwh(0.0, HEIGHT, 800.0, 72.0)
            );
            assert_eq!(
                key_rect(tester, avatar1),
                Rect::from_ltwh(16.0, HEIGHT + 16.0, 40.0, 40.0)
            );
            assert_eq!(
                key_rect(tester, placeholder1),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, HEIGHT + 24.0, 24.0, 24.0)
            );

            // THREE-LINE
            two_tiles(
                tester,
                m3,
                [
                    ListTile::new()
                        .leading(avatar(avatar0))
                        .trailing(placeholder(placeholder0, 24.0, 24.0))
                        .title(Text::new("A"))
                        .subtitle(Text::new(THIRTEEN_LINES))
                        .is_three_line(true),
                    ListTile::new()
                        .leading(avatar(avatar1))
                        .trailing(placeholder(placeholder1, 24.0, 24.0))
                        .title(Text::new("A"))
                        .subtitle(Text::new("A"))
                        .is_three_line(true),
                ],
            );
            assert_eq!(
                tile_rect(tester, 0),
                Rect::from_ltwh(0.0, 0.0, 800.0, HEIGHT)
            );
            assert_eq!(
                key_rect(tester, avatar0),
                Rect::from_ltwh(16.0, 8.0, 40.0, 40.0)
            );
            assert_eq!(
                key_rect(tester, placeholder0),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, 8.0, 24.0, 24.0)
            );
            assert_eq!(
                tile_rect(tester, 1),
                Rect::from_ltwh(0.0, HEIGHT, 800.0, 88.0)
            );
            assert_eq!(
                key_rect(tester, avatar1),
                Rect::from_ltwh(16.0, HEIGHT + 8.0, 40.0, 40.0)
            );
            assert_eq!(
                key_rect(tester, placeholder1),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, HEIGHT + 8.0, 24.0, 24.0)
            );

            // "ONE-LINE" with Small Leading Widget
            let [leading0, trailing0, leading1, trailing1] = &keys;
            two_tiles(
                tester,
                m3,
                [
                    ListTile::new()
                        .leading(placeholder(leading0, 24.0, 12.0))
                        .trailing(placeholder(trailing0, 24.0, 24.0))
                        .title(Text::new(THIRTEEN_LINES)),
                    ListTile::new()
                        .leading(placeholder(leading1, 24.0, 12.0))
                        .trailing(placeholder(trailing1, 24.0, 24.0))
                        .title(Text::new("A")),
                ],
            );
            tester.pump_by(Duration::from_secs(2)); // the text styles are animated when we change dense
            assert_eq!(
                tile_rect(tester, 0),
                Rect::from_ltwh(0.0, 0.0, 800.0, 328.0)
            );
            assert_eq!(
                key_rect(tester, leading0),
                Rect::from_ltwh(16.0, 158.0, 24.0, 12.0)
            );
            assert_eq!(
                key_rect(tester, trailing0),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, 152.0, 24.0, 24.0)
            );
            assert_eq!(
                tile_rect(tester, 1),
                Rect::from_ltwh(0.0, 328.0, 800.0, 56.0)
            );
            assert_eq!(
                key_rect(tester, leading1),
                Rect::from_ltwh(16.0, 328.0 + 22.0, 24.0, 12.0)
            );
            assert_eq!(
                key_rect(tester, trailing1),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, 328.0 + 16.0, 24.0, 24.0)
            );
        });
    }

    /// The two rects of "ListTile {leading, trailing} icon height does not exceed ListTile
    /// height" for one count of lines.
    fn oversized_rects(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        leading: bool,
        subtitle: bool,
        is_three_line: bool,
    ) -> [Rect; 2] {
        oversized_rects_dense(tester, theme, leading, subtitle, is_three_line, false)
    }

    /// [`oversized_rects`] with tiles that are `dense`.
    fn oversized_rects_dense(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        leading: bool,
        subtitle: bool,
        is_three_line: bool,
        dense: bool,
    ) -> [Rect; 2] {
        // regression test for https://github.com/flutter/flutter/issues/28765
        let keys = [GlobalKey::new(), GlobalKey::new()];
        let tiles = [("A", &keys[0]), ("B", &keys[1])].map(|(text, key)| {
            let oversized = placeholder(key, 24.0, 80.0);
            let mut tile = ListTile::new().title(Text::new(text)).dense(dense);
            tile = if leading {
                tile.leading(oversized)
            } else {
                tile.trailing(oversized)
            };
            if subtitle {
                tile = tile.subtitle(Text::new(text));
            }
            tile.is_three_line(is_three_line)
        });
        two_tiles(tester, theme, tiles);
        [key_rect(tester, &keys[0]), key_rect(tester, &keys[1])]
    }

    /// "ListTile leading icon height does not exceed ListTile height".
    #[test]
    fn list_tile_leading_icon_height_does_not_exceed_list_tile_height() {
        test_widgets(|tester| {
            // One line
            assert_eq!(
                oversized_rects(tester, m3, true, false, false),
                [
                    Rect::from_ltwh(16.0, 0.0, 24.0, 56.0),
                    Rect::from_ltwh(16.0, 56.0, 24.0, 56.0)
                ]
            );
            // Two line
            assert_eq!(
                oversized_rects(tester, m3, true, true, false),
                [
                    Rect::from_ltwh(16.0, 8.0, 24.0, 56.0),
                    Rect::from_ltwh(16.0, 72.0 + 8.0, 24.0, 56.0)
                ]
            );
            // Three line
            assert_eq!(
                oversized_rects(tester, m3, true, true, true),
                [
                    Rect::from_ltwh(16.0, 8.0, 24.0, 56.0),
                    Rect::from_ltwh(16.0, 88.0 + 8.0, 24.0, 56.0)
                ]
            );
        });
    }

    /// "ListTile trailing icon height does not exceed ListTile height".
    #[test]
    fn list_tile_trailing_icon_height_does_not_exceed_list_tile_height() {
        test_widgets(|tester| {
            // One line
            assert_eq!(
                oversized_rects(tester, m3, false, false, false),
                [
                    Rect::from_ltwh(800.0 - 24.0 - 24.0, 0.0, 24.0, 56.0),
                    Rect::from_ltwh(800.0 - 24.0 - 24.0, 56.0, 24.0, 56.0)
                ]
            );
            // Two line
            assert_eq!(
                oversized_rects(tester, m3, false, true, false),
                [
                    Rect::from_ltwh(800.0 - 24.0 - 24.0, 8.0, 24.0, 56.0),
                    Rect::from_ltwh(800.0 - 24.0 - 24.0, 72.0 + 8.0, 24.0, 56.0)
                ]
            );
            // Three line
            assert_eq!(
                oversized_rects(tester, m3, false, true, true),
                [
                    Rect::from_ltwh(800.0 - 24.0 - 24.0, 8.0, 24.0, 56.0),
                    Rect::from_ltwh(800.0 - 24.0 - 24.0, 88.0 + 8.0, 24.0, 56.0)
                ]
            );
        });
    }

    /// The focus node of the `Focus` above `key`'s widget: Dart's `Focus.of(key.currentContext!)`.
    fn focus_of(tester: &WidgetTester, key: &GlobalKey) -> AnyFocusNode {
        let mut app = tester.cell.borrow_mut();
        let context = key.current_context(&app).expect("mounted");
        waft_widgets::Focus::of(&mut app, context, false, true)
    }

    /// "ListTile only accepts focus when enabled".
    #[test]
    fn list_tile_only_accepts_focus_when_enabled() {
        test_widgets(|tester| {
            let child_key = GlobalKey::new();
            let frame = |tester: &WidgetTester, enabled: bool| {
                bare(
                    m3(&tester.cell.borrow()),
                    Material::new().child(
                        ListView::new().children([ListTile::new()
                            .title(Text::new("A").key(child_key.clone().into()))
                            .dense(true)
                            .enabled(enabled)
                            .on_tap(noop())
                            .into_widget()]),
                    ),
                )
            };
            let first = frame(tester, true);
            tester.pump_widget(first);
            tester.pump(); // Let the focus take effect.

            let tile_node = focus_of(tester, &child_key);
            tile_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump(); // Let the focus take effect.
            assert!(focus_of(tester, &child_key).has_primary_focus(&tester.cell.borrow()));

            assert!(tile_node.has_primary_focus(&tester.cell.borrow()));
            let second = frame(tester, false);
            tester.pump_widget(second);

            assert_ne!(
                waft_widgets::primary_focus(&mut tester.cell.borrow_mut()),
                Some(tile_node)
            );
            assert!(!focus_of(tester, &child_key).has_primary_focus(&tester.cell.borrow()));
        });
    }

    /// "ListTile can autofocus unless disabled.".
    #[test]
    fn list_tile_can_autofocus_unless_disabled() {
        test_widgets(|tester| {
            let child_key = GlobalKey::new();
            let frame = |tester: &WidgetTester, enabled: bool| {
                bare(
                    m3(&tester.cell.borrow()),
                    Material::new().child(
                        ListView::new().children([ListTile::new()
                            .title(Text::new("A").key(child_key.clone().into()))
                            .dense(true)
                            .enabled(enabled)
                            .autofocus(true)
                            .on_tap(noop())
                            .into_widget()]),
                    ),
                )
            };
            let first = frame(tester, true);
            tester.pump_widget(first);
            tester.pump();
            assert!(focus_of(tester, &child_key).has_primary_focus(&tester.cell.borrow()));

            let second = frame(tester, false);
            tester.pump_widget(second);
            tester.pump();
            assert!(!focus_of(tester, &child_key).has_primary_focus(&tester.cell.borrow()));
        });
    }

    /// The 100x100 white material of the focus and hover cases.
    fn white_square(tile: ListTile) -> WidgetRef {
        Center::new()
            .child(
                SizedBox::new()
                    .width(100.0)
                    .height(100.0)
                    .child(Material::new().color(Colors::WHITE).child(tile)),
            )
            .into_widget()
    }

    const SQUARE: Rect = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);

    /// "ListTile is focusable and has correct focus color". The ink list holds what the ink
    /// features paint, which leaves out the material's own white rounded rectangle; valo
    /// records no transparent draw, so the tile's transparent `Ink` rect is checked on its
    /// decoration instead.
    #[test]
    fn list_tile_is_focusable_and_has_correct_focus_color() {
        test_widgets(|tester| {
            let focus_node =
                FocusNode::new(&mut tester.cell.borrow_mut()).into_erased::<AnyFocusNode>();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let frame = |tester: &WidgetTester, enabled: bool| {
                let mut tile = ListTile::new()
                    .focus_color(Colors::ORANGE.shade500())
                    .autofocus(true)
                    .focus_node(focus_node.clone());
                if enabled {
                    tile = tile.on_tap(noop());
                }
                bare(m3(&tester.cell.borrow()), white_square(tile))
            };
            let first = frame(tester, true);
            tester.pump_widget(first);

            tester.pump_and_settle();
            assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_eq!(ink_tile_color(tester), Some(Color::new(0x00000000)));
            paints_exactly(
                &draws(tester.ink_list().ops()),
                &[PaintCall::rect_at(SQUARE, Some(Colors::ORANGE.shade500()))],
            )
            .unwrap();

            // Check when the list tile is disabled.
            let second = frame(tester, false);
            tester.pump_widget(second);
            tester.pump_and_settle();
            assert!(!focus_node.has_primary_focus(&tester.cell.borrow()));
            assert_eq!(ink_tile_color(tester), Some(Color::new(0x00000000)));
            paints_exactly(&draws(tester.ink_list().ops()), &[]).unwrap();
        });
    }

    /// "ListTile can be hovered and has correct hover color". valo records no transparent
    /// draw: the tile's transparent `Ink` rect is checked on its decoration, and the hover
    /// highlight faded to alpha 0 draws nothing.
    #[test]
    fn list_tile_can_be_hovered_and_has_correct_hover_color() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let frame = |tester: &WidgetTester, enabled: bool| {
                let mut tile = ListTile::new()
                    .hover_color(Colors::ORANGE.shade500())
                    .autofocus(true);
                if enabled {
                    tile = tile.on_tap(noop());
                }
                bare(m3(&tester.cell.borrow()), white_square(tile))
            };
            let first = frame(tester, true);
            tester.pump_widget(first);

            tester.pump();
            tester.pump_and_settle();
            assert_eq!(ink_tile_color(tester), Some(Color::new(0x00000000)));
            paints_exactly(
                &draws(tester.ink_list().ops()),
                &[PaintCall::rect_at(SQUARE, Some(Color::new(0x1f000000)))],
            )
            .unwrap();

            // Start hovering
            let mut gesture = TestPointer::mouse(1, 0);
            let center = tile_rect(tester, 0).center();
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, center, false);

            let again = frame(tester, true);
            tester.pump_widget(again);
            tester.pump();
            tester.pump_and_settle();
            paints_exactly(
                &draws(tester.ink_list().ops()),
                &[
                    PaintCall::rect_at(SQUARE, Some(Color::new(0x1f000000))),
                    PaintCall::rect_at(SQUARE, Some(Colors::ORANGE.shade500())),
                ],
            )
            .unwrap();

            let disabled = frame(tester, false);
            tester.pump_widget(disabled);
            tester.pump();
            tester.pump_and_settle();
            assert_eq!(ink_tile_color(tester), Some(Color::new(0x00000000)));
            paints_exactly(&draws(tester.ink_list().ops()), &[]).unwrap();
        });
    }

    /// "ListTile can be splashed and has correct splash color".
    #[test]
    fn list_tile_can_be_splashed_and_has_correct_splash_color() {
        test_widgets(|tester| {
            pump_app(tester, m2, |_app| {
                Material::new()
                    .child(
                        Center::new().child(
                            SizedBox::square(Some(100.0)).child(
                                ListTile::new()
                                    .on_tap(noop())
                                    .splash_color(Color::new(0xff88ff88)),
                            ),
                        ),
                    )
                    .into_widget()
            });
            tester.pump_and_settle();
            let center = tile_rect(tester, 0).center();
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, center);
            tester.pump_by(Duration::from_millis(200));
            // valo records no circle of radius 0, which the splash has until its first frame
            // after this one; Flutter's `paints` sees the zero circle.
            tester.pump_by(Duration::from_millis(16));
            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::Circle {
                    x: Some(50.0),
                    y: Some(50.0),
                    radius: None,
                    color: Some(Color::new(0xff88ff88)),
                    style: None,
                }],
            );
            tester.up(&gesture);
        });
    }

    /// "ListTile responds to density changes.".
    #[test]
    fn list_tile_responds_to_density_changes() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let build_test = |tester: &mut WidgetTester, visual_density: VisualDensity| {
                let theme = m3(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Material::new()
                        .child(
                            Center::new().child(
                                ListTile::new()
                                    .key(key.clone().into())
                                    .on_tap(noop())
                                    .autofocus(true)
                                    .visual_density(visual_density),
                            ),
                        )
                        .into_widget(),
                ));
            };

            build_test(tester, VisualDensity::STANDARD);
            let render = tester.render_object(&key).as_box().expect("a render box");
            let size = |tester: &WidgetTester| render.size(&tester.cell.borrow());
            tester.pump_and_settle();
            assert_eq!(size(tester), Size::new(800.0, 56.0));

            build_test(tester, VisualDensity::new(3.0, 3.0));
            tester.pump_and_settle();
            assert_eq!(size(tester), Size::new(800.0, 68.0));

            build_test(tester, VisualDensity::new(-3.0, -3.0));
            tester.pump_and_settle();
            assert_eq!(size(tester), Size::new(800.0, 44.0));

            build_test(tester, VisualDensity::new(3.0, -3.0));
            tester.pump_and_settle();
            assert_eq!(size(tester), Size::new(800.0, 44.0));
        });
    }

    /// "ListTile shape is painted correctly".
    #[test]
    fn list_tile_shape_is_painted_correctly() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/63877
            let tile_color = Colors::RED.shade500();
            let pump_shape = |tester: &mut WidgetTester, shape: ShapeBorderRef| {
                pump_app(tester, m3, |_app| {
                    Material::new()
                        .child(
                            Center::new()
                                .child(ListTile::new().shape(shape).tile_color(tile_color)),
                        )
                        .into_widget()
                })
            };

            // Test rectangle shape
            pump_shape(
                tester,
                ShapeBorderRef::new(RoundedRectangleBorder::default()),
            );
            let rect = tile_rect(tester, 0);
            // Check if a rounded rectangle was painted with the correct color and shape
            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::rect_at(rect, Some(tile_color))],
            );

            // Test stadium shape
            pump_shape(tester, ShapeBorderRef::new(StadiumBorder::default()));
            let rect = tile_rect(tester, 0);
            // Check if a rounded rectangle was painted with the correct color and shape
            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::rrect_at(
                    RRect::from_rect_and_radius(
                        rect,
                        waft_embedder::Radius::circular(rect.width().min(rect.height()) / 2.0),
                    ),
                    Some(tile_color),
                )],
            );
        });
    }

    /// One frame of "ListTile changes mouse cursor when hovered", with the mouse over the tile.
    ///
    /// Dart creates its gesture after the first frame and adds the mouse there: later frames
    /// only pump the new tile under the mouse that is already there.
    fn cursor_over(
        tester: &mut WidgetTester,
        gesture: &mut Option<TestPointer>,
        tile: ListTile,
    ) -> MouseCursorRef {
        pump_app(tester, m3, |_app| {
            Material::new()
                .child(
                    Center::new().child(
                        MouseRegion::new()
                            .cursor(SystemMouseCursors::FORBIDDEN.into())
                            .child(tile),
                    ),
                )
                .into_widget()
        });
        if gesture.is_none() {
            let center = tile_rect(tester, 0).center();
            let mut mouse = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut mouse, center);
            tester.pump();
            *gesture = Some(mouse);
        }
        tester.active_cursor(1)
    }

    /// "ListTile changes mouse cursor when hovered".
    #[test]
    fn list_tile_changes_mouse_cursor_when_hovered() {
        test_widgets(|tester| {
            let text: MouseCursorRef = SystemMouseCursors::TEXT.into();
            let click: MouseCursorRef = SystemMouseCursors::CLICK.into();
            let basic: MouseCursorRef = SystemMouseCursors::BASIC.into();

            let mut gesture = None;

            // Test ListTile() constructor
            let cursor = cursor_over(
                tester,
                &mut gesture,
                ListTile::new()
                    .on_tap(noop())
                    .mouse_cursor(SystemMouseCursors::TEXT),
            );
            assert!(*cursor == *text);

            // Test default cursor
            let cursor = cursor_over(tester, &mut gesture, ListTile::new().on_tap(noop()));
            assert!(*cursor == *click);

            // Test default cursor when disabled
            let cursor = cursor_over(tester, &mut gesture, ListTile::new().enabled(false));
            assert!(*cursor == *basic);

            // Test default cursor when onTap or onLongPress is null
            let cursor = cursor_over(tester, &mut gesture, ListTile::new());
            assert!(*cursor == *basic);
        });
    }

    /// "ListTile onFocusChange callback".
    #[test]
    fn list_tile_on_focus_change_callback() {
        test_widgets(|tester| {
            let node = FocusNode::new(&mut tester.cell.borrow_mut()).into_erased::<AnyFocusNode>();
            let got_focus = Rc::new(Cell::new(false));
            let on_focus_change: ValueChanged<bool> = {
                let got_focus = Rc::clone(&got_focus);
                Rc::new(move |_app, focused| got_focus.set(focused))
            };
            let tree = bare(
                m3(&tester.cell.borrow()),
                Material::new().child(
                    ListTile::new()
                        .focus_node(node.clone())
                        .on_focus_change(on_focus_change)
                        .on_tap(noop()),
                ),
            );
            tester.pump_widget(tree);

            node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump();
            assert!(got_focus.get());
            assert!(node.has_focus(&mut tester.cell.borrow_mut()));

            node.unfocus(
                &mut tester.cell.borrow_mut(),
                waft_widgets::UnfocusDisposition::Scope,
            );
            tester.pump();
            assert!(!got_focus.get());
            assert!(!node.has_focus(&mut tester.cell.borrow_mut()));
        });
    }

    /// The colour of the tile's `Ink` decoration: what Flutter's `paints..rect(color:)` sees
    /// for a transparent tile, which valo does not record.
    pub(crate) fn ink_tile_color(tester: &WidgetTester) -> Option<Color> {
        let ink = tester.element(find::by_type::<Ink>().at(0));
        tester.widget(ink, |ink: &Ink| {
            ink.effective_decoration().and_then(|decoration| {
                decoration
                    .as_any()
                    .downcast_ref::<ShapeDecoration>()
                    .and_then(|decoration| decoration.color.as_ref().map(AnyColor::color))
            })
        })
    }

    /// A tile in a `StatefulBuilder` whose tap toggles `selected`: the tile-colour cases.
    pub(crate) fn toggling_tile(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        tile: impl Fn() -> ListTile + 'static,
    ) {
        let is_selected = Rc::new(Cell::new(false));
        pump_app(tester, theme, move |_app| {
            Material::new()
                .child(Center::new().child(StatefulBuilder::new(
                    move |_app, _context, set_state| {
                        let toggled = Rc::clone(&is_selected);
                        tile()
                            .selected(is_selected.get())
                            .on_tap(Listener::new(move |app| {
                                let toggled = Rc::clone(&toggled);
                                set_state(app, &mut |_app| toggled.set(!toggled.get()));
                            }))
                            .title(Text::new("Title"))
                            .into_widget()
                    },
                )))
                .into_widget()
        })
    }

    /// "ListTile respects tileColor & selectedTileColor".
    #[test]
    fn list_tile_respects_tile_color_and_selected_tile_color() {
        test_widgets(|tester| {
            let tile_color = Colors::GREEN.shade500();
            let selected_tile_color = Colors::RED.shade500();
            toggling_tile(tester, m3, move || {
                ListTile::new()
                    .selected_tile_color(selected_tile_color)
                    .tile_color(tile_color)
            });

            // Initially, when isSelected is false, the ListTile should respect tileColor.
            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::rect(Some(tile_color))],
            );

            // Tap on tile to change isSelected.
            let center = tile_rect(tester, 0).center();
            tester.tap_at(center, 1);
            tester.pump_and_settle();

            // When isSelected is true, the ListTile should respect selectedTileColor.
            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::rect(Some(selected_tile_color))],
            );
        });
    }

    /// "ListTile shows Material ripple effects on top of tileColor".
    #[test]
    fn list_tile_shows_material_ripple_effects_on_top_of_tile_color() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/73616
            let tile_color = Colors::RED.shade500();
            pump_app(tester, m2, |_app| {
                Material::new()
                    .child(
                        Center::new().child(
                            ListTile::new()
                                .tile_color(tile_color)
                                .on_tap(noop())
                                .title(Text::new("Title")),
                        ),
                    )
                    .into_widget()
            });

            // Before ListTile is tapped, it should be tileColor
            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::rect(Some(tile_color))],
            );

            // Tap on tile to trigger ink effect and wait for it to be underway.
            let center = tile_rect(tester, 0).center();
            tester.tap_at(center, 1);
            tester.pump_by(Duration::from_millis(200));
            // valo records no circle of radius 0, which the ripple has until its first frame
            // after this one; Flutter's `paints` sees the zero circle.
            tester.pump_by(Duration::from_millis(16));

            // After tap, the tile could be drawn in tileColor, with the ripple (circle) on top
            assert_paints(
                tester.ink_list().ops(),
                &[PaintCall::rect(Some(tile_color)), PaintCall::circle(None)],
            );
        });
    }

    /// "ListTile default tile color": valo records no transparent draw, so the colour is
    /// read from the tile's `Ink` decoration.
    #[test]
    fn list_tile_default_tile_color() {
        test_widgets(|tester| {
            toggling_tile(tester, m3, ListTile::new);
            assert_eq!(ink_tile_color(tester), Some(Colors::TRANSPARENT));

            // Tap on tile to change isSelected.
            let center = tile_rect(tester, 0).center();
            tester.tap_at(center, 1);
            tester.pump_and_settle();

            assert_eq!(ink_tile_color(tester), Some(Colors::TRANSPARENT));
        });
    }

    /// "ListTile layout at zero size".
    #[test]
    fn list_tile_layout_at_zero_size() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/66636
            let key = GlobalKey::new();
            pump_app(tester, m3, |_app| {
                Scaffold::new()
                    .body(
                        SizedBox::shrink().child(
                            ListTile::new()
                                .key(key.clone().into())
                                .tile_color(Colors::GREEN.primary()),
                        ),
                    )
                    .into_widget()
            });
            let size = key_rect(tester, &key).size();
            assert_eq!(size.width(), 0.0);
            assert_eq!(size.height(), 0.0);
        });
    }

    /// "ListTile.mouseCursor overrides ListTileTheme.mouseCursor".
    #[test]
    fn list_tile_mouse_cursor_overrides_list_tile_theme_mouse_cursor() {
        test_widgets(|tester| {
            let tile_key = GlobalKey::new();
            pump_app(tester, m3, |_app| {
                let clickable: WidgetStatePropertyRef<Option<MouseCursorRef>> =
                    <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(|states| {
                        Some(WidgetStateMouseCursor::clickable().resolve(states))
                    });
                Material::new()
                    .child(
                        ListTileTheme::new(
                            ListTile::new()
                                .key(tile_key.clone().into())
                                .mouse_cursor(WidgetStateMouseCursor::textable())
                                .title(Text::new("Title"))
                                .on_tap(noop()),
                        )
                        .data(ListTileThemeData::new().mouse_cursor(clickable)),
                    )
                    .into_widget()
            });
            let list_tile = key_rect(tester, &tile_key).center();
            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, list_tile, false);
            tester.pump_and_settle();
            let text: MouseCursorRef = SystemMouseCursors::TEXT.into();
            assert!(*tester.active_cursor(1) == *text);
        });
    }

    /// A `Container(alignment: Alignment.topLeft, child: tile)` in a material under a
    /// `ListTileTheme(data: theme_data)`, in `text_direction`: the horizontal and vertical
    /// spacing cases.
    fn spacing_frame(
        tester: &mut WidgetTester,
        theme_data: ListTileThemeData,
        text_direction: TextDirection,
        tile: ListTile,
    ) {
        spacing_frame_in(tester, m3, theme_data, text_direction, tile)
    }

    /// [`spacing_frame`] under `theme`.
    fn spacing_frame_in(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        theme_data: ListTileThemeData,
        text_direction: TextDirection,
        tile: ListTile,
    ) {
        let theme = theme(&tester.cell.borrow());
        tester.pump_widget(
            MediaQuery::new(
                MediaQueryData::new(),
                Directionality::new(
                    text_direction,
                    crate::theme::Theme::new(
                        theme,
                        Material::new().child(
                            ListTileTheme::new(
                                Container::new()
                                    .alignment(AlignmentGeometry::TOP_LEFT)
                                    .child(tile),
                            )
                            .data(theme_data),
                        ),
                    ),
                ),
            )
            .into_widget(),
        );
    }

    /// `ListTile(leading: Text('L'), title: Text('title'), trailing: Text('T'))`.
    fn l_title_t() -> ListTile {
        ListTile::new()
            .leading(Text::new("L"))
            .title(Text::new("title"))
            .trailing(Text::new("T"))
    }

    /// Where the title starts (LTR) or ends (RTL), and the tile's size.
    fn title_edge(tester: &WidgetTester, text_direction: TextDirection) -> (f64, Size) {
        let title = tester.get_rect(find::text("title"));
        let edge = match text_direction {
            TextDirection::Ltr => title.left,
            TextDirection::Rtl => title.right,
        };
        (edge, tile_rect(tester, 0).size())
    }

    /// "ListTile horizontalTitleGap = 0.0".
    #[test]
    fn list_tile_horizontal_title_gap_0() {
        test_widgets(|tester| {
            for (text_direction, edge) in [(TextDirection::Ltr, 40.0), (TextDirection::Rtl, 760.0)]
            {
                let frames = [
                    (None, Some(0.0)),
                    (Some(0.0), None),
                    (Some(10.0), Some(0.0)),
                ];
                for (theme_gap, widget_gap) in frames {
                    let mut theme_data = ListTileThemeData::new();
                    theme_data.horizontal_title_gap = theme_gap;
                    let mut tile = l_title_t();
                    tile.horizontal_title_gap = widget_gap;
                    spacing_frame(tester, theme_data, text_direction, tile);
                    assert_eq!(
                        title_edge(tester, text_direction),
                        (edge, Size::new(800.0, 56.0))
                    );
                }
            }
        });
    }

    /// "ListTile horizontalTitleGap = (default) && ListTile minLeadingWidth = (default)".
    #[test]
    fn list_tile_default_horizontal_title_gap_and_min_leading_width() {
        test_widgets(|tester| {
            spacing_frame(
                tester,
                ListTileThemeData::new(),
                TextDirection::Ltr,
                l_title_t(),
            );
            // horizontalTitleGap: ListTileDefaultValue.horizontalTitleGap (16.0)
            assert_eq!(
                title_edge(tester, TextDirection::Ltr),
                (56.0, Size::new(800.0, 56.0))
            );

            spacing_frame(
                tester,
                ListTileThemeData::new(),
                TextDirection::Rtl,
                l_title_t(),
            );
            // horizontalTitleGap: ListTileDefaultValue.horizontalTitleGap (16.0)
            assert_eq!(
                title_edge(tester, TextDirection::Rtl),
                (744.0, Size::new(800.0, 56.0))
            );
        });
    }

    /// "ListTile horizontalTitleGap with visualDensity".
    #[test]
    fn list_tile_horizontal_title_gap_with_visual_density() {
        test_widgets(|tester| {
            let frame = || {
                l_title_t()
                    .visual_density(VisualDensity::new(VisualDensity::MINIMUM_DENSITY, 0.0))
                    .horizontal_title_gap(10.0)
            };
            spacing_frame(
                tester,
                ListTileThemeData::new(),
                TextDirection::Ltr,
                frame(),
            );
            assert_eq!(
                title_edge(tester, TextDirection::Ltr),
                (42.0, Size::new(800.0, 56.0))
            );

            // Pump another frame of the same widget to ensure the underlying render
            // object did not cache the original horizontalTitleGap calculation based on the
            // visualDensity
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(
                MediaQuery::new(
                    MediaQueryData::new(),
                    Directionality::new(
                        TextDirection::Ltr,
                        crate::theme::Theme::new(
                            theme,
                            Material::new().child(
                                ListTileTheme::new(
                                    Container::new()
                                        .alignment(AlignmentGeometry::TOP_LEFT)
                                        .child(frame()),
                                )
                                .data(ListTileThemeData::new()),
                            ),
                        ),
                    ),
                )
                .into_widget(),
            );
            assert_eq!(
                title_edge(tester, TextDirection::Ltr),
                (42.0, Size::new(800.0, 56.0))
            );
        });
    }

    /// "ListTile minVerticalPadding = 80.0".
    #[test]
    fn list_tile_min_vertical_padding_80() {
        test_widgets(|tester| {
            for text_direction in [TextDirection::Ltr, TextDirection::Rtl] {
                let frames = [
                    (None, Some(80.0)),
                    (Some(80.0), None),
                    (Some(0.0), Some(80.0)),
                ];
                for (theme_padding, widget_padding) in frames {
                    let mut theme_data = ListTileThemeData::new();
                    theme_data.min_vertical_padding = theme_padding;
                    let mut tile = l_title_t();
                    tile.min_vertical_padding = widget_padding;
                    spacing_frame(tester, theme_data, text_direction, tile);
                    // 80 + 80 + 24(Title) = 184
                    assert_eq!(tile_rect(tester, 0).size(), Size::new(800.0, 184.0));
                }
            }
        });
    }

    /// "ListTile minLeadingWidth = 60.0".
    #[test]
    fn list_tile_min_leading_width_60() {
        test_widgets(|tester| {
            // 92.0 = 16.0(Default contentPadding) + 16.0(Default horizontalTitleGap) + 60.0
            // 708.0 = 800.0 - (16.0(Default contentPadding) + 16.0(Default horizontalTitleGap) + 60.0)
            for (text_direction, edge) in [(TextDirection::Ltr, 92.0), (TextDirection::Rtl, 708.0)]
            {
                let frames = [
                    (None, Some(60.0)),
                    (Some(60.0), None),
                    (Some(0.0), Some(60.0)),
                ];
                for (theme_width, widget_width) in frames {
                    let mut theme_data = ListTileThemeData::new();
                    theme_data.min_leading_width = theme_width;
                    let mut tile = l_title_t();
                    tile.min_leading_width = widget_width;
                    spacing_frame(tester, theme_data, text_direction, tile);
                    assert_eq!(
                        title_edge(tester, text_direction),
                        (edge, Size::new(800.0, 56.0))
                    );
                }
            }
        });
    }

    /// "ListTile minTileHeight".
    #[test]
    fn list_tile_min_tile_height() {
        test_widgets(|tester| {
            let height = |tester: &mut WidgetTester, min_tile_height: Option<f64>| {
                let mut tile = ListTile::new();
                tile.min_tile_height = min_tile_height;
                spacing_frame(tester, ListTileThemeData::new(), TextDirection::Ltr, tile);
                tile_rect(tester, 0).size()
            };
            // Default list tile with height = 56.0
            assert_eq!(height(tester, None), Size::new(800.0, 56.0));
            // Set list tile height = 30.0
            assert_eq!(height(tester, Some(30.0)), Size::new(800.0, 30.0));
            // Set list tile height = 60.0
            assert_eq!(height(tester, Some(60.0)), Size::new(800.0, 60.0));
        });
    }

    /// "ListTile computeMinIntrinsicHeight respects minTileHeight and padding": the frame
    /// lays out without a panic.
    #[test]
    fn list_tile_compute_min_intrinsic_height_respects_min_tile_height_and_padding() {
        test_widgets(|tester| {
            let column = || {
                waft_widgets::Expanded::new(
                    waft_widgets::Column::new()
                        .children([ListTile::new().title(Text::new("item.label")).into_widget()]),
                )
                .into_widget()
            };
            pump_app(
                tester,
                |app| {
                    ThemeData::builder()
                        .list_tile_theme(ListTileThemeData::new().min_tile_height(30.0))
                        .build(app)
                },
                |_app| {
                    Material::new()
                        .r#type(crate::material::MaterialType::Transparency)
                        .child(
                            waft_widgets::Column::new().children([
                                waft_widgets::IntrinsicHeight::new()
                                    .child(waft_widgets::Row::new().children([column(), column()]))
                                    .into_widget(),
                            ]),
                        )
                        .into_widget()
                },
            );
        });
    }

    /// Dart's `TestText(text)`: a `Text` whose build keeps the ambient `DefaultTextStyle`,
    /// laid out in the FlutterTest font.
    pub(crate) fn test_text(text: &str, seen: &Rc<RefCell<Option<TextStyle>>>) -> WidgetRef {
        let text = text.to_owned();
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            *seen.borrow_mut() = Some(waft_widgets::DefaultTextStyle::of(app, context).style);
            Text::new(&text).into_widget()
        })
        .into_widget()
    }

    /// Dart's `TestIcon()`: an `Icon` whose build keeps the ambient icon colour.
    pub(crate) fn test_icon(seen: &Rc<Cell<Option<Color>>>) -> WidgetRef {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            let icon_theme = IconTheme::of(app, context);
            seen.set(icon_theme.color.as_ref().map(AnyColor::color));
            waft_widgets::Icon::new(Some(crate::icons::Icons::add())).into_widget()
        })
        .into_widget()
    }

    pub(crate) fn style_color(seen: &Rc<RefCell<Option<TextStyle>>>) -> Option<Color> {
        seen.borrow()
            .as_ref()
            .and_then(|style| style.color.as_ref().map(AnyColor::color))
    }

    /// "colors are applied to leading and trailing text widgets".
    #[test]
    fn colors_are_applied_to_leading_and_trailing_text_widgets() {
        test_widgets(|tester| {
            let check = |tester: &mut WidgetTester, enabled: bool, selected: bool| {
                let leading = Rc::new(RefCell::new(None));
                let trailing = Rc::new(RefCell::new(None));
                let title = Rc::new(RefCell::new(None));
                let theme = Rc::new(RefCell::new(None));
                pump_app(tester, m2, |_app| {
                    let (leading, trailing, title, theme) = (
                        Rc::clone(&leading),
                        Rc::clone(&trailing),
                        Rc::clone(&title),
                        Rc::clone(&theme),
                    );
                    Material::new()
                        .child(Center::new().child(Builder::new(move |app, context| {
                            *theme.borrow_mut() = Some(Theme::of(app, context));
                            ListTile::new()
                                .enabled(enabled)
                                .selected(selected)
                                .leading(test_text("leading", &leading))
                                .title(test_text("title", &title))
                                .trailing(test_text("trailing", &trailing))
                                .into_widget()
                        })))
                        .into_widget()
                });
                // Wait for text color to animate.
                tester.pump_and_settle();
                let theme = theme.borrow_mut().take().expect("built");
                (style_color(&leading), style_color(&trailing), theme)
            };

            let (leading, trailing, theme) = check(tester, true, false);
            // Enabled color should be default bodyMedium color.
            let body_medium = theme
                .text_theme
                .body_medium
                .as_ref()
                .and_then(|style| style.color.as_ref().map(AnyColor::color));
            assert_eq!(leading, body_medium);
            assert_eq!(trailing, body_medium);

            let (leading, trailing, theme) = check(tester, true, true);
            // Selected color should be ThemeData.primaryColor by default.
            assert_eq!(leading, Some(theme.primary_color));
            assert_eq!(trailing, Some(theme.primary_color));

            let (leading, trailing, theme) = check(tester, false, false);
            // Disabled color should be ThemeData.disabledColor by default.
            assert_eq!(leading, Some(theme.disabled_color));
            assert_eq!(trailing, Some(theme.disabled_color));
        });
    }

    /// The icon colours each slot's `TestIcon` saw.
    fn slot_icon_colors(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        selected: bool,
    ) -> [Option<Color>; 4] {
        let seen: [Rc<Cell<Option<Color>>>; 4] = std::array::from_fn(|_| Rc::new(Cell::new(None)));
        pump_app(tester, theme, |_app| {
            Material::new()
                .child(
                    Center::new().child(
                        ListTile::new()
                            .selected(selected)
                            .leading(test_icon(&seen[0]))
                            .title(test_icon(&seen[1]))
                            .subtitle(test_icon(&seen[2]))
                            .trailing(test_icon(&seen[3])),
                    ),
                )
                .into_widget()
        });
        seen.map(|seen| seen.get())
    }

    /// "selected, enabled ListTile default icon color".
    #[test]
    fn selected_enabled_list_tile_default_icon_color() {
        test_widgets(|tester| {
            let color_scheme = m3(&tester.cell.borrow()).color_scheme;
            assert_eq!(
                slot_icon_colors(tester, m3, true),
                [Some(color_scheme.primary); 4]
            );
            assert_eq!(
                slot_icon_colors(tester, m3, false),
                [Some(color_scheme.on_surface_variant()); 4]
            );
        });
    }

    /// The default text styles each slot's `TestText` saw, in Dart's order: leading, title,
    /// subtitle, trailing.
    fn slot_text_styles(tester: &mut WidgetTester, theme: fn(&App) -> ThemeData) -> [TextStyle; 4] {
        let seen: [Rc<RefCell<Option<TextStyle>>>; 4] =
            std::array::from_fn(|_| Rc::new(RefCell::new(None)));
        pump_app(tester, theme, |_app| {
            Material::new()
                .child(
                    Center::new().child(
                        ListTile::new()
                            .leading(test_text("leading", &seen[0]))
                            .title(test_text("title", &seen[1]))
                            .subtitle(test_text("subtitle", &seen[2]))
                            .trailing(test_text("trailing", &seen[3])),
                    ),
                )
                .into_widget()
        });
        seen.map(|seen| seen.borrow_mut().take().expect("built"))
    }

    /// "ListTile font size": the sizes of the `DefaultTextStyle` each `Text` lays out with,
    /// where Dart reads its paragraph's style.
    #[test]
    fn list_tile_font_size() {
        test_widgets(|tester| {
            let [leading, title, subtitle, trailing] = slot_text_styles(tester, m3);
            // ListTile default text sizes.
            assert_eq!(leading.font_size, Some(11.0));
            assert_eq!(title.font_size, Some(16.0));
            assert_eq!(subtitle.font_size, Some(14.0));
            assert_eq!(trailing.font_size, Some(11.0));
        });
    }

    /// "ListTile text color".
    #[test]
    fn list_tile_text_color() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let colors =
                slot_text_styles(tester, m3).map(|style| style.color.as_ref().map(AnyColor::color));
            // ListTile default text colors.
            assert_eq!(
                colors,
                [
                    Some(theme.color_scheme.on_surface_variant()),
                    Some(theme.color_scheme.on_surface),
                    Some(theme.color_scheme.on_surface_variant()),
                    Some(theme.color_scheme.on_surface_variant()),
                ]
            );
        });
    }

    /// The colour by state of the `WidgetStateColor` cases.
    fn state_color() -> WidgetStateColor {
        WidgetStateColor::resolve_with(|states| {
            if states.contains(&WidgetState::Disabled) {
                return Colors::RED.primary();
            }
            if states.contains(&WidgetState::Selected) {
                return Colors::GREEN.primary();
            }
            Colors::BLUE.primary()
        })
    }

    /// "ListTile.textColor respects WidgetStateColor".
    #[test]
    fn list_tile_text_color_respects_widget_state_color() {
        test_widgets(|tester| {
            let title_color = |tester: &mut WidgetTester, enabled: bool, selected: bool| {
                pump_app(tester, m3, |_app| {
                    Material::new()
                        .child(
                            Center::new().child(
                                ListTile::new()
                                    .enabled(enabled)
                                    .selected(selected)
                                    .text_color(state_color())
                                    .title(Text::new("title"))
                                    .subtitle(Text::new("subtitle")),
                            ),
                        )
                        .into_widget()
                });
                tester.pump_and_settle();
                tester.text_color("title")
            };
            // Test disabled state.
            assert_eq!(title_color(tester, false, false), Colors::RED.primary());
            // Test enabled state.
            assert_eq!(title_color(tester, true, false), Colors::BLUE.primary());
            // Test selected state.
            assert_eq!(title_color(tester, true, true), Colors::GREEN.primary());
        });
    }

    /// "ListTile.iconColor respects WidgetStateColor".
    #[test]
    fn list_tile_icon_color_respects_widget_state_color() {
        test_widgets(|tester| {
            let icon_color = |tester: &mut WidgetTester, enabled: bool, selected: bool| {
                let seen = Rc::new(Cell::new(None));
                pump_app(tester, m3, |_app| {
                    Material::new()
                        .child(
                            Center::new().child(
                                ListTile::new()
                                    .enabled(enabled)
                                    .selected(selected)
                                    .icon_color(state_color())
                                    .leading(test_icon(&seen)),
                            ),
                        )
                        .into_widget()
                });
                tester.pump_and_settle();
                seen.get()
            };
            // Test disabled state.
            assert_eq!(
                icon_color(tester, false, false),
                Some(Colors::RED.primary())
            );
            // Test enabled state.
            assert_eq!(
                icon_color(tester, true, false),
                Some(Colors::BLUE.primary())
            );
            // Test selected state.
            assert_eq!(
                icon_color(tester, true, true),
                Some(Colors::GREEN.primary())
            );
        });
    }

    /// The colour of the glyph of the `Icon` showing `icon`: Dart's
    /// `tester.widget<RichText>(find.descendant(of: find.byIcon(icon), matching:
    /// find.byType(RichText))).text.style?.color`.
    fn glyph_color(tester: &WidgetTester, icon: &waft_widgets::IconData) -> Option<Color> {
        let icon_element = find::by_type::<waft_widgets::Icon>()
            .evaluate(tester)
            .into_iter()
            .find(|element| {
                tester.widget(*element, |widget: &waft_widgets::Icon| {
                    widget.icon.as_ref() == Some(icon)
                })
            })
            .expect("the icon");
        let rich_text = find::by_type::<waft_widgets::RichText>()
            .evaluate(tester)
            .into_iter()
            .find(|rich_text| tester.ancestors(*rich_text).contains(&icon_element))
            .expect("the icon's RichText");
        tester.widget(rich_text, |rich_text: &waft_widgets::RichText| {
            rich_text
                .text
                .style()
                .and_then(|style| style.color.as_ref().map(AnyColor::color))
        })
    }

    /// The tile of the `IconButtonTheme` cases: an icon button in each of leading and
    /// trailing, under a theme whose icon button theme is `icon_button_theme`.
    fn icon_buttons_tile(
        tester: &mut WidgetTester,
        icon_button_theme: IconButtonThemeData,
        icon_color: Option<Color>,
        home: fn(ListTile) -> WidgetRef,
    ) {
        pump_app(
            tester,
            move |app| {
                ThemeData::builder()
                    .icon_button_theme(icon_button_theme)
                    .build(app)
            },
            move |_app| {
                let mut tile = ListTile::new()
                    .leading(IconButton::new(
                        Some(noop()),
                        waft_widgets::Icon::new(Some(crate::icons::Icons::favorite())),
                    ))
                    .trailing(IconButton::new(
                        Some(noop()),
                        waft_widgets::Icon::new(Some(crate::icons::Icons::close())),
                    ));
                if let Some(icon_color) = icon_color {
                    tile = tile.icon_color(icon_color);
                }
                home(tile)
            },
        )
    }

    /// Dart's `Material(child: Center(child: tile))`.
    fn centered_in_material(tile: ListTile) -> WidgetRef {
        Material::new()
            .child(Center::new().child(tile))
            .into_widget()
    }

    /// Dart's `Scaffold(body: Center(child: tile))`.
    fn centered_in_scaffold(tile: ListTile) -> WidgetRef {
        Scaffold::new()
            .body(Center::new().child(tile))
            .into_widget()
    }

    /// "ListTile.iconColor respects iconColor property with icon buttons Material 3 in
    /// presence of IconButtonTheme override".
    #[test]
    fn list_tile_icon_color_respects_icon_color_with_icon_buttons_over_an_icon_button_theme() {
        test_widgets(|tester| {
            icon_buttons_tile(
                tester,
                IconButtonThemeData::new().style(
                    IconButton::style_from()
                        .foreground_color(Colors::BLUE.primary())
                        .build(),
                ),
                Some(Colors::GREEN.primary()),
                centered_in_material,
            );
            assert_eq!(
                glyph_color(tester, &crate::icons::Icons::favorite()),
                Some(Colors::GREEN.primary())
            );
            assert_eq!(
                glyph_color(tester, &crate::icons::Icons::close()),
                Some(Colors::GREEN.primary())
            );
        });
    }

    /// "IconButtonTheme.style.foregroundColor is preserved in ListTile in non-overriding
    /// scenario".
    #[test]
    fn icon_button_theme_foreground_color_is_preserved_in_list_tile_when_not_overridden() {
        test_widgets(|tester| {
            icon_buttons_tile(
                tester,
                IconButtonThemeData::new().style(
                    IconButton::style_from()
                        .foreground_color(Colors::BLUE.primary())
                        .build(),
                ),
                None,
                centered_in_material,
            );
            assert_eq!(
                glyph_color(tester, &crate::icons::Icons::favorite()),
                Some(Colors::BLUE.primary())
            );
            assert_eq!(
                glyph_color(tester, &crate::icons::Icons::close()),
                Some(Colors::BLUE.primary())
            );
        });
    }

    /// "ListTile respects and combines parent IconButtonThemeData style"; the properties are
    /// compared by identity, as Dart's constant ones are.
    #[test]
    fn list_tile_respects_and_combines_parent_icon_button_theme_data_style() {
        test_widgets(|tester| {
            let custom_shape: WidgetStatePropertyRef<Option<OutlinedBorderRef>> = Rc::new(
                WidgetStatePropertyAll::new(Some(OutlinedBorderRef::new(RoundedRectangleBorder {
                    side: waft_painting::BorderSide::default(),
                    ..RoundedRectangleBorder::default()
                }))),
            );
            // Specially treated in ButtonStyle.merge, thus check these, too.
            let overlay_color: WidgetStatePropertyRef<Option<Color>> =
                Rc::new(WidgetStatePropertyAll::new(Some(Colors::PURPLE.primary())));
            let mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>> =
                Rc::new(WidgetStatePropertyAll::new(Some(
                    SystemMouseCursors::RESIZE_UP_RIGHT_DOWN_LEFT.into(),
                )));
            icon_buttons_tile(
                tester,
                IconButtonThemeData::new().style(
                    crate::button_style::ButtonStyle::new()
                        .overlay_color(Rc::clone(&overlay_color))
                        .mouse_cursor(Rc::clone(&mouse_cursor))
                        .shape(Rc::clone(&custom_shape)),
                ),
                Some(Colors::GREEN.primary()),
                centered_in_scaffold,
            );

            // Verify that the merged theme retains the parent shape and other critical fields.
            for icon_button in find::by_type::<IconButton>().evaluate(tester) {
                let merged_theme = IconButtonTheme::of(&mut tester.cell.borrow_mut(), icon_button);
                let style = merged_theme.style.expect("a style");
                assert!(Rc::ptr_eq(
                    style.shape.as_ref().expect("a shape"),
                    &custom_shape
                ));
                assert!(Rc::ptr_eq(
                    style.overlay_color.as_ref().expect("an overlay color"),
                    &overlay_color
                ));
                assert!(Rc::ptr_eq(
                    style.mouse_cursor.as_ref().expect("a mouse cursor"),
                    &mouse_cursor
                ));
            }

            // Verify that ListTile's iconColor overrides the inherited color.
            assert_eq!(
                glyph_color(tester, &crate::icons::Icons::favorite()),
                Some(Colors::GREEN.primary())
            );
            assert_eq!(
                glyph_color(tester, &crate::icons::Icons::close()),
                Some(Colors::GREEN.primary())
            );
        });
    }

    /// "ListTile.dense does not throw assertion".
    #[test]
    fn list_tile_dense_does_not_throw_assertion() {
        test_widgets(|tester| {
            // This is a regression test for https://github.com/flutter/flutter/pull/116908
            for theme in [m2, m3] {
                let theme = theme(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Material::new()
                        .child(Center::new().child(StatefulBuilder::new(
                            |_app, _context, _set_state| {
                                ListTile::new()
                                    .dense(true)
                                    .title(Text::new("Title"))
                                    .into_widget()
                            },
                        )))
                        .into_widget(),
                ));
            }
        });
    }

    /// A title-alignment case: one tester, whose frames Dart's `buildFrame` pumps in turn.
    struct AlignmentCase<'a> {
        tester: &'a mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        leading_key: GlobalKey,
        trailing_key: GlobalKey,
    }

    impl<'a> AlignmentCase<'a> {
        fn new(tester: &'a mut WidgetTester, theme: fn(&App) -> ThemeData) -> AlignmentCase<'a> {
            AlignmentCase {
                tester,
                theme,
                leading_key: GlobalKey::new(),
                trailing_key: GlobalKey::new(),
            }
        }

        /// Pumps the frame and answers the leading and trailing tops, relative to the
        /// tile's.
        fn offsets(
            &mut self,
            title_alignment: Option<ListTileTitleAlignment>,
            with_subtitle: bool,
            is_three_line: bool,
        ) -> (f64, f64) {
            let mut tile = ListTile::new()
                .min_vertical_padding(10.0)
                .leading(sized(&self.leading_key, 24.0, 24.0))
                .title(SizedBox::new().width(20.0).height(50.0))
                .trailing(sized(&self.trailing_key, 24.0, 24.0));
            tile.title_alignment = title_alignment;
            if with_subtitle {
                tile = tile
                    .subtitle(SizedBox::new().width(20.0).height(50.0))
                    .is_three_line(is_three_line);
            }
            let theme = (self.theme)(&self.tester.cell.borrow());
            self.tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(Center::new().child(tile))
                    .into_widget(),
            ));
            let tile = tile_rect(self.tester, 0).top;
            (
                key_rect(self.tester, &self.leading_key).top - tile,
                key_rect(self.tester, &self.trailing_key).top - tile,
            )
        }
    }

    const LEADING_HEIGHT: f64 = 24.0;
    const TITLE_HEIGHT: f64 = 50.0;
    const MIN_VERTICAL_PADDING: f64 = 10.0;

    /// "titleAlignment position with title widget".
    #[test]
    fn title_alignment_position_with_title_widget() {
        test_widgets(|tester| {
            let mut case = AlignmentCase::new(tester, m3);
            const TILE_HEIGHT: f64 = MIN_VERTICAL_PADDING * 2.0 + TITLE_HEIGHT;
            const CENTER_POSITION: f64 = (TILE_HEIGHT / 2.0) - (LEADING_HEIGHT / 2.0);
            // If [ThemeData.useMaterial3] is true, the default title alignment is
            // [ListTileTitleAlignment.threeLine], which positions the leading and
            // trailing widgets center vertically in the tile if the [ListTile.isThreeLine]
            // property is false.
            assert_eq!(
                case.offsets(None, false, false),
                (CENTER_POSITION, CENTER_POSITION)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::ThreeLine), false, false),
                (CENTER_POSITION, CENTER_POSITION)
            );
            // If the tile height is less than 72.0 pixels, the leading widget is placed
            // 16.0 pixels below the top of the title widget, and the trailing is centered
            // vertically in the tile.
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::TitleHeight), false, false),
                (16.0, CENTER_POSITION)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Top), false, false),
                (MIN_VERTICAL_PADDING, MIN_VERTICAL_PADDING)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Center), false, false),
                (CENTER_POSITION, CENTER_POSITION)
            );
            const BOTTOM_POSITION: f64 = TILE_HEIGHT - MIN_VERTICAL_PADDING - LEADING_HEIGHT;
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Bottom), false, false),
                (BOTTOM_POSITION, BOTTOM_POSITION)
            );
        });
    }

    /// "titleAlignment position with title and subtitle widgets".
    #[test]
    fn title_alignment_position_with_title_and_subtitle_widgets() {
        test_widgets(|tester| {
            let mut case = AlignmentCase::new(tester, m3);
            const TILE_HEIGHT: f64 = MIN_VERTICAL_PADDING * 2.0 + TITLE_HEIGHT + 50.0;
            const CENTER_POSITION: f64 = (TILE_HEIGHT / 2.0) - (LEADING_HEIGHT / 2.0);
            assert_eq!(
                case.offsets(None, true, false),
                (CENTER_POSITION, CENTER_POSITION)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::ThreeLine), true, false),
                (CENTER_POSITION, CENTER_POSITION)
            );
            // Leading and trailing widgets are positioned 16.0 pixels below the
            // top of the title widget.
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::TitleHeight), true, false),
                (16.0, 16.0)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Top), true, false),
                (MIN_VERTICAL_PADDING, MIN_VERTICAL_PADDING)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Center), true, false),
                (CENTER_POSITION, CENTER_POSITION)
            );
            const BOTTOM_POSITION: f64 = TILE_HEIGHT - MIN_VERTICAL_PADDING - LEADING_HEIGHT;
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Bottom), true, false),
                (BOTTOM_POSITION, BOTTOM_POSITION)
            );
        });
    }

    /// "ListTile.isThreeLine updates ListTileTitleAlignment.threeLine's alignment".
    #[test]
    fn list_tile_is_three_line_updates_the_three_line_alignment() {
        test_widgets(|tester| {
            let mut case = AlignmentCase::new(tester, m3);
            const TILE_HEIGHT: f64 = MIN_VERTICAL_PADDING * 2.0 + TITLE_HEIGHT + 50.0;
            const CENTER_POSITION: f64 = (TILE_HEIGHT / 2.0) - (LEADING_HEIGHT / 2.0);
            // By default, leading and trailing widgets are centered vertically
            // in the tile.
            assert_eq!(
                case.offsets(None, true, false),
                (CENTER_POSITION, CENTER_POSITION)
            );
            // The leading and trailing widgets are placed minVerticalPadding
            // to the top of the tile widget.
            assert_eq!(
                case.offsets(None, true, true),
                (MIN_VERTICAL_PADDING, MIN_VERTICAL_PADDING)
            );
        });
    }

    /// The frame of "Leading/Trailing exceeding list tile width throws exception".
    fn exceeding_width(leading: bool) {
        test_widgets(|tester| {
            // 16 (content padding) + 61 (leading width) + 24 (content padding) = 101.
            // List tile width is 100 as a result, an exception should be thrown.
            pump_app(tester, m3, |_app| {
                let wide = SizedBox::new().width(61.0);
                let tile = if leading {
                    ListTile::new().leading(wide)
                } else {
                    ListTile::new().trailing(wide)
                };
                Material::new()
                    .child(Center::new().child(SizedBox::new().width(100.0).child(tile)))
                    .into_widget()
            });
        });
    }

    /// "Leading/Trailing exceeding list tile width throws exception": "leading".
    #[test]
    #[should_panic(
        expected = "Leading widget consumes the entire tile width (including ListTile.contentPadding).\nEither resize the tile width so that the leading widget plus any content padding do not exceed the tile width, or use a sized widget, or consider replacing ListTile with a custom widget.\nSee also: https://api.flutter.dev/flutter/material/ListTile-class.html#material.ListTile.4"
    )]
    fn leading_exceeding_list_tile_width_panics() {
        exceeding_width(true);
    }

    /// "Leading/Trailing exceeding list tile width throws exception": "trailing".
    #[test]
    #[should_panic(
        expected = "Trailing widget consumes the entire tile width (including ListTile.contentPadding).\nEither resize the tile width so that the trailing widget plus any content padding do not exceed the tile width, or use a sized widget, or consider replacing ListTile with a custom widget.\nSee also: https://api.flutter.dev/flutter/material/ListTile-class.html#material.ListTile.4"
    )]
    fn trailing_exceeding_list_tile_width_panics() {
        exceeding_width(false);
    }

    /// The rects of "ListTile isThreeLine": the two tiles, their avatars and placeholders.
    fn is_three_line_rects(
        tester: &mut WidgetTester,
        theme_data_is_three_line: Option<bool>,
        theme_is_three_line: Option<bool>,
        is_three_line: Option<bool>,
    ) -> [Rect; 6] {
        let keys: [GlobalKey; 4] = std::array::from_fn(|_| GlobalKey::new());
        pump_unique_app(
            tester,
            |app| {
                let mut theme = ThemeData::builder();
                if let Some(is_three_line) = theme_data_is_three_line {
                    theme = theme
                        .list_tile_theme(ListTileThemeData::new().is_three_line(is_three_line));
                }
                theme.build(app)
            },
            |_app| {
                let tile = |avatar_key: &GlobalKey, placeholder_key: &GlobalKey, subtitle: &str| {
                    let mut tile = ListTile::new()
                        .leading(avatar(avatar_key))
                        .trailing(placeholder(placeholder_key, 24.0, 24.0))
                        .title(Text::new("A"))
                        .subtitle(Text::new(subtitle));
                    tile.is_three_line = is_three_line;
                    tile.into_widget()
                };
                let list = ListView::new().children([
                    tile(&keys[0], &keys[1], THIRTEEN_LINES),
                    tile(&keys[2], &keys[3], "A"),
                ]);
                let mut theme = ListTileTheme::new(list);
                if let Some(is_three_line) = theme_is_three_line {
                    theme = theme.data(ListTileThemeData::new().is_three_line(is_three_line));
                }
                Material::new().child(theme).into_widget()
            },
        );
        [
            tile_rect(tester, 0),
            key_rect(tester, &keys[0]),
            key_rect(tester, &keys[1]),
            tile_rect(tester, 1),
            key_rect(tester, &keys[2]),
            key_rect(tester, &keys[3]),
        ]
    }

    /// "ListTile isThreeLine".
    #[test]
    fn list_tile_is_three_line() {
        test_widgets(|tester| {
            const HEIGHT: f64 = 300.0;
            const AVATAR_TOP: f64 = 130.0;
            const PLACEHOLDER_TOP: f64 = 138.0;
            let two_line = [
                Rect::from_ltwh(0.0, 0.0, 800.0, HEIGHT),
                Rect::from_ltwh(16.0, AVATAR_TOP, 40.0, 40.0),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, PLACEHOLDER_TOP, 24.0, 24.0),
                Rect::from_ltwh(0.0, HEIGHT, 800.0, 72.0),
                Rect::from_ltwh(16.0, HEIGHT + 16.0, 40.0, 40.0),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, HEIGHT + 24.0, 24.0, 24.0),
            ];
            let three_line = [
                Rect::from_ltwh(0.0, 0.0, 800.0, HEIGHT),
                Rect::from_ltwh(16.0, 8.0, 40.0, 40.0),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, 8.0, 24.0, 24.0),
                Rect::from_ltwh(0.0, HEIGHT, 800.0, 88.0),
                Rect::from_ltwh(16.0, HEIGHT + 8.0, 40.0, 40.0),
                Rect::from_ltwh(800.0 - 24.0 - 24.0, HEIGHT + 8.0, 24.0, 24.0),
            ];
            let frames = [
                ((None, None, None), two_line),
                ((Some(true), None, None), three_line),
                ((Some(false), Some(true), None), three_line),
                ((Some(true), Some(false), None), two_line),
                ((None, None, Some(true)), three_line),
                ((None, Some(true), Some(false)), two_line),
                ((Some(true), None, Some(false)), two_line),
                ((Some(true), Some(true), Some(false)), two_line),
                ((None, Some(false), Some(true)), three_line),
                ((Some(false), None, Some(true)), three_line),
                ((Some(false), Some(false), Some(true)), three_line),
            ];
            for ((theme_data, theme, tile), expected) in frames {
                assert_eq!(
                    is_three_line_rects(tester, theme_data, theme, tile),
                    expected
                );
            }
        });
    }

    /// Dart's `assert(isThreeLine != true || subtitle != null)` sees both arguments at once, so
    /// a chain may give the subtitle after `is_three_line`.
    #[test]
    fn a_three_line_tile_may_be_given_its_subtitle_after_is_three_line() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Material::new()
                    .child(
                        ListTile::new()
                            .is_three_line(true)
                            .title(Text::new("title"))
                            .subtitle(Text::new("subtitle")),
                    )
                    .into_widget()
            });
            expect(tester, &find::by_type::<ListTile>(), finds_one_widget());
        });
    }

    /// Dart's `assert(isThreeLine != true || subtitle != null)`.
    #[test]
    #[should_panic(expected = "self.is_three_line != Some(true) || self.subtitle.is_some()")]
    fn a_three_line_tile_without_a_subtitle_panics_when_made_a_widget() {
        ListTile::new()
            .is_three_line(true)
            .title(Text::new("title"))
            .into_widget();
    }

    /// "ListTile statesController".
    #[test]
    fn list_tile_states_controller() {
        test_widgets(|tester| {
            let count = Rc::new(Cell::new(0));
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            controller.add_listener(&mut tester.cell.borrow_mut(), {
                let count = Rc::clone(&count);
                Listener::new(move |_app| count.set(count.get() + 1))
            });
            let frame = |tester: &WidgetTester, enabled: bool| {
                let mut tile = ListTile::new()
                    .states_controller(controller.clone())
                    .title(Text::new("title"));
                if enabled {
                    tile = tile.on_tap(noop());
                }
                material_app(m3(&tester.cell.borrow()), centered_in_scaffold(tile))
            };
            let value = |tester: &WidgetTester| -> WidgetStates {
                tester.cell.borrow().get(&controller).value().clone()
            };
            let states =
                |states: &[WidgetState]| -> WidgetStates { states.iter().copied().collect() };

            let first = frame(tester, true);
            tester.pump_widget(first);

            assert_eq!(value(tester), states(&[]));
            assert_eq!(count.get(), 0);

            let center = tester
                .get_rect(find::text("title").evaluate(tester)[0])
                .center();
            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();

            assert_eq!(value(tester), states(&[WidgetState::Hovered]));
            assert_eq!(count.get(), 1);

            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump_and_settle();

            assert_eq!(value(tester), states(&[]));
            assert_eq!(count.get(), 2);

            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();

            assert_eq!(value(tester), states(&[WidgetState::Hovered]));
            assert_eq!(count.get(), 3);

            tester.send(&gesture, waft_embedder::PointerChange::Down);
            tester.pump_and_settle();

            assert_eq!(
                value(tester),
                states(&[WidgetState::Hovered, WidgetState::Pressed])
            );
            assert_eq!(count.get(), 4);

            tester.up(&gesture);
            tester.pump_and_settle();

            assert_eq!(value(tester), states(&[WidgetState::Hovered]));
            assert_eq!(count.get(), 5);

            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump_and_settle();

            assert_eq!(value(tester), states(&[]));
            assert_eq!(count.get(), 6);

            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            assert_eq!(
                value(tester),
                states(&[WidgetState::Hovered, WidgetState::Pressed])
            );
            assert_eq!(count.get(), 8); // adds hovered and pressed - two changes

            // If the button is rebuilt disabled, then the pressed state is
            // removed.
            let disabled = frame(tester, false);
            tester.pump_widget(disabled);
            tester.pump_and_settle();
            assert_eq!(
                value(tester),
                states(&[WidgetState::Hovered, WidgetState::Disabled])
            );
            assert_eq!(count.get(), 10); // removes pressed and adds disabled - two changes
            tester.move_to(&mut gesture, Offset::ZERO, true);
            tester.pump_and_settle();
            assert_eq!(value(tester), states(&[WidgetState::Disabled]));
            assert_eq!(count.get(), 11);
        });
    }

    /// "ListTile does not crash at zero area".
    #[test]
    fn list_tile_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Scaffold::new()
                    .body(
                        Center::new().child(
                            SizedBox::shrink().child(
                                ListTile::new()
                                    .title(Text::new("title"))
                                    .leading(waft_widgets::Icon::new(Some(
                                        crate::icons::Icons::add(),
                                    )))
                                    .trailing(waft_widgets::Icon::new(Some(
                                        crate::icons::Icons::remove(),
                                    )))
                                    .subtitle(Text::new("subTitle")),
                            ),
                        ),
                    )
                    .into_widget()
            });
            assert_eq!(tile_rect(tester, 0).size(), Size::ZERO);
        });
    }

    /// The panic message of building `around(tile)` in a material, or `None`.
    ///
    /// Dart reports the error and builds on; Waft's debug build panics with it, in the middle
    /// of a frame, which the tester then does not tear down (`testing/PORTING.md`).
    fn background_check_message(around: fn(ListTile) -> WidgetRef) -> Option<String> {
        let mut tester = WidgetTester::new();
        let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pump_app(&mut tester, m3, |_app| {
                Material::new()
                    .child(
                        Center::new().child(around(
                            ListTile::new()
                                .tile_color(Colors::RED.primary())
                                .title(Text::new("ListTile")),
                        )),
                    )
                    .into_widget()
            });
        }));
        match built {
            Ok(()) => None,
            Err(panic) => Some(
                panic
                    .downcast_ref::<String>()
                    .cloned()
                    .unwrap_or_else(|| "a panic without a message".to_owned()),
            ),
        }
    }

    /// "ListTile shows warning when a Container with color wraps it": Dart reports the error
    /// and builds on, where Waft's debug build panics with it.
    #[test]
    fn list_tile_shows_warning_when_a_container_with_color_wraps_it() {
        test_widgets(|_| {
            let message = background_check_message(|tile| {
                Container::new()
                    .color(Colors::AMBER.primary())
                    .height(200.0)
                    .child(tile)
                    .into_widget()
            })
            .expect("a reported error");
            assert!(message.contains("ListTile background color or ink splashes may be invisible"));
            assert!(message.contains("The ListTile is wrapped in a ColoredBox"));
            assert!(message.contains("ListTile:"));
            assert!(message.contains("tile_color:"));
            assert!(message.contains("ColoredBox:"));
            assert!(message.contains("color:"));
        });
    }

    /// "ListTile throw exception when wrapped in ColoredBox with non-transparent color".
    #[test]
    fn list_tile_panics_when_wrapped_in_a_colored_box_with_a_non_transparent_color() {
        test_widgets(|_| {
            let message = background_check_message(|tile| {
                ColoredBox::new(Colors::AMBER.primary())
                    .child(tile)
                    .into_widget()
            })
            .expect("a reported error");
            assert!(message.contains("ListTile background color or ink splashes may be invisible"));
            assert!(message.contains("The ListTile is wrapped in a ColoredBox"));
            assert!(message.contains("ListTile:"));
            assert!(message.contains("tile_color:"));
            assert!(message.contains("ColoredBox:"));
            assert!(message.contains("color:"));
        });
    }

    /// "ListTile throw exception when wrapped in DecoratedBox with non-transparent color".
    #[test]
    fn list_tile_panics_when_wrapped_in_a_decorated_box_with_a_non_transparent_color() {
        test_widgets(|_| {
            let message = background_check_message(|tile| {
                DecoratedBox::new(BoxDecoration::new().color(Colors::AMBER.primary()))
                    .child(tile)
                    .into_widget()
            })
            .expect("a reported error");
            assert!(message.contains("ListTile background color or ink splashes may be invisible"));
            assert!(message.contains("The ListTile is wrapped in a DecoratedBox"));
            assert!(message.contains("ListTile:"));
            assert!(message.contains("tile_color:"));
            assert!(message.contains("DecoratedBox:"));
            assert!(message.contains("color"));
        });
    }

    /// "ListTile does not throw exception when parent has no/transparent color".
    #[test]
    fn list_tile_does_not_panic_when_the_parent_has_no_or_a_transparent_color() {
        test_widgets(|_| {
            let message = background_check_message(|tile| {
                ColoredBox::new(Colors::TRANSPARENT)
                    .child(tile)
                    .into_widget()
            });
            assert_eq!(message, None);
        });
    }

    // These tests are only relevant for Material 2. Once Material 2
    // support is deprecated and the APIs are removed, these tests
    // can be deleted.

    /// "Material 2: ListTile geometry (LTR)".
    #[test]
    fn material2_list_tile_geometry_ltr() {
        test_widgets(|tester| {
            let plain = GeometryFrame::PLAIN;
            let frames = [
                (plain, 56.0),
                (
                    GeometryFrame {
                        dense: true,
                        ..plain
                    },
                    48.0,
                ),
                (
                    GeometryFrame {
                        is_two_line: true,
                        ..plain
                    },
                    72.0,
                ),
                (
                    GeometryFrame {
                        is_two_line: true,
                        dense: true,
                        ..plain
                    },
                    64.0,
                ),
                (
                    GeometryFrame {
                        is_three_line: true,
                        ..plain
                    },
                    88.0,
                ),
                (
                    GeometryFrame {
                        is_three_line: true,
                        dense: true,
                        ..plain
                    },
                    76.0,
                ),
                (
                    GeometryFrame {
                        text_scale: 4.0,
                        ..plain
                    },
                    72.0,
                ),
                (
                    GeometryFrame {
                        dense: true,
                        text_scale: 4.0,
                        ..plain
                    },
                    72.0,
                ),
                (
                    GeometryFrame {
                        is_two_line: true,
                        text_scale: 4.0,
                        ..plain
                    },
                    128.0,
                ),
                // Make sure that the height of a large subtitle is taken into account.
                (
                    GeometryFrame {
                        is_two_line: true,
                        text_scale: 0.5,
                        subtitle_scale: Some(4.0),
                        ..plain
                    },
                    72.0,
                ),
                (
                    GeometryFrame {
                        is_two_line: true,
                        dense: true,
                        text_scale: 4.0,
                        ..plain
                    },
                    128.0,
                ),
                (
                    GeometryFrame {
                        is_three_line: true,
                        text_scale: 4.0,
                        ..plain
                    },
                    128.0,
                ),
                (
                    GeometryFrame {
                        is_three_line: true,
                        dense: true,
                        text_scale: 4.0,
                        ..plain
                    },
                    128.0,
                ),
            ];
            let keys = [GlobalKey::new(), GlobalKey::new()];
            for (frame, expected_height) in frames {
                // The default contentPadding is 16.0 on the left and right.
                check_ltr_geometry(tester, &keys, m2, frame, expected_height, 16.0, 56.0);
            }
        });
    }

    /// "Material 2: ListTile geometry (RTL)".
    #[test]
    fn material2_list_tile_geometry_rtl() {
        test_widgets(|tester| {
            pump_app(tester, m2, |_app| {
                MediaQuery::new(
                    MediaQueryData::new().padding(EdgeInsets::only(
                        LEFT_PADDING,
                        0.0,
                        RIGHT_PADDING,
                        0.0,
                    )),
                    Directionality::new(
                        TextDirection::Rtl,
                        Material::new().child(
                            Center::new().child(
                                ListTile::new()
                                    .leading(Text::new("L"))
                                    .title(Text::new("title"))
                                    .trailing(Text::new("T")),
                            ),
                        ),
                    ),
                )
                .into_widget()
            });
            assert_eq!(
                tester.get_rect(find::text("L")).right,
                800.0 - 16.0_f64.max(RIGHT_PADDING)
            );
            assert_eq!(
                tester.get_rect(find::text("title")).right,
                800.0 - 56.0 - 16.0_f64.max(RIGHT_PADDING)
            );
            assert_eq!(
                tester.get_rect(find::text("T")).left,
                16.0_f64.max(LEFT_PADDING)
            );
        });
    }

    /// "Material 2: ListTile leading and trailing positions", on one tester under a bare
    /// theme, so that each frame's text styles animate from the last frame's as Dart's do.
    #[test]
    fn material2_list_tile_leading_and_trailing_positions() {
        test_widgets(|tester| {
            // This test is based on the redlines at
            // https://material.io/design/components/lists.html#specs
            let keys: [GlobalKey; 4] = std::array::from_fn(|_| GlobalKey::new());
            let [
                first_leading,
                first_trailing,
                second_leading,
                second_trailing,
            ] = &keys;
            let frame = |tester: &mut WidgetTester,
                         small_leading: bool,
                         title: &str,
                         subtitle: Option<&str>,
                         dense: bool,
                         is_three_line: bool| {
                let tile = |leading_key: &GlobalKey, trailing_key: &GlobalKey, lines: &str| {
                    let leading = if small_leading {
                        placeholder(leading_key, 24.0, 12.0)
                    } else {
                        avatar(leading_key)
                    };
                    let (title, subtitle) = match subtitle {
                        None => (lines, None),
                        Some(_) => (title, Some(lines)),
                    };
                    let mut tile = ListTile::new()
                        .dense(dense)
                        .leading(leading)
                        .trailing(placeholder(trailing_key, 24.0, 24.0))
                        .title(Text::new(title));
                    if let Some(subtitle) = subtitle {
                        tile = tile.subtitle(Text::new(subtitle));
                    }
                    tile.is_three_line(is_three_line).into_widget()
                };
                let second_lines = subtitle.unwrap_or(title);
                let tree = bare(
                    m2(&tester.cell.borrow()),
                    Material::new().child(ListView::new().children([
                        tile(first_leading, first_trailing, THIRTEEN_LINES),
                        tile(second_leading, second_trailing, second_lines),
                    ])),
                );
                tester.pump_widget(tree);
            };
            let check = |tester: &WidgetTester, expected: [Rect; 6]| {
                assert_eq!(
                    [
                        tile_rect(tester, 0),
                        key_rect(tester, first_leading),
                        key_rect(tester, first_trailing),
                        tile_rect(tester, 1),
                        key_rect(tester, second_leading),
                        key_rect(tester, second_trailing),
                    ],
                    expected
                );
            };
            let trailing_left = 800.0 - 24.0 - 16.0;

            // DENSE "ONE"-LINE
            frame(tester, false, "A", None, true, false);
            check(
                tester,
                [
                    Rect::from_ltwh(0.0, 0.0, 800.0, 177.0),
                    Rect::from_ltwh(16.0, 16.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 16.0, 24.0, 24.0),
                    Rect::from_ltwh(0.0, 177.0, 800.0, 48.0),
                    Rect::from_ltwh(16.0, 177.0 + 4.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 177.0 + 12.0, 24.0, 24.0),
                ],
            );

            // NON-DENSE "ONE"-LINE
            frame(tester, false, "A", None, false, false);
            tester.pump_by(Duration::from_secs(2)); // the text styles are animated when we change dense
            check(
                tester,
                [
                    Rect::from_ltwh(0.0, 0.0, 800.0, 216.0),
                    Rect::from_ltwh(16.0, 16.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 16.0, 24.0, 24.0),
                    Rect::from_ltwh(0.0, 216.0, 800.0, 56.0),
                    Rect::from_ltwh(16.0, 216.0 + 8.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 216.0 + 16.0, 24.0, 24.0),
                ],
            );

            // DENSE "TWO"-LINE
            frame(tester, false, "A", Some("A"), true, false);
            check(
                tester,
                [
                    Rect::from_ltwh(0.0, 0.0, 800.0, 180.0),
                    Rect::from_ltwh(16.0, 16.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 16.0, 24.0, 24.0),
                    Rect::from_ltwh(0.0, 180.0, 800.0, 64.0),
                    Rect::from_ltwh(16.0, 180.0 + 12.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 180.0 + 20.0, 24.0, 24.0),
                ],
            );

            // NON-DENSE "TWO"-LINE
            frame(tester, false, "A", Some("A"), false, false);
            check(
                tester,
                [
                    Rect::from_ltwh(0.0, 0.0, 800.0, 180.0),
                    Rect::from_ltwh(16.0, 16.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 16.0, 24.0, 24.0),
                    Rect::from_ltwh(0.0, 180.0, 800.0, 72.0),
                    Rect::from_ltwh(16.0, 180.0 + 16.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 180.0 + 24.0, 24.0, 24.0),
                ],
            );

            // DENSE "THREE"-LINE
            frame(tester, false, "A", Some("A"), true, true);
            check(
                tester,
                [
                    Rect::from_ltwh(0.0, 0.0, 800.0, 180.0),
                    Rect::from_ltwh(16.0, 16.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 16.0, 24.0, 24.0),
                    Rect::from_ltwh(0.0, 180.0, 800.0, 76.0),
                    Rect::from_ltwh(16.0, 180.0 + 16.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 180.0 + 16.0, 24.0, 24.0),
                ],
            );

            // NON-DENSE THREE-LINE
            frame(tester, false, "A", Some("A"), false, true);
            check(
                tester,
                [
                    Rect::from_ltwh(0.0, 0.0, 800.0, 180.0),
                    Rect::from_ltwh(16.0, 16.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 16.0, 24.0, 24.0),
                    Rect::from_ltwh(0.0, 180.0, 800.0, 88.0),
                    Rect::from_ltwh(16.0, 180.0 + 16.0, 40.0, 40.0),
                    Rect::from_ltwh(trailing_left, 180.0 + 16.0, 24.0, 24.0),
                ],
            );

            // "ONE-LINE" with Small Leading Widget
            frame(tester, true, "A", None, false, false);
            tester.pump_by(Duration::from_secs(2)); // the text styles are animated when we change dense
            check(
                tester,
                [
                    Rect::from_ltwh(0.0, 0.0, 800.0, 216.0),
                    Rect::from_ltwh(16.0, 16.0, 24.0, 12.0),
                    Rect::from_ltwh(trailing_left, 16.0, 24.0, 24.0),
                    Rect::from_ltwh(0.0, 216.0, 800.0, 56.0),
                    Rect::from_ltwh(16.0, 216.0 + 16.0, 24.0, 12.0),
                    Rect::from_ltwh(trailing_left, 216.0 + 16.0, 24.0, 24.0),
                ],
            );
        });
    }

    /// "Material 2: ListTile leading icon height does not exceed ListTile height".
    #[test]
    fn material2_list_tile_leading_icon_height_does_not_exceed_list_tile_height() {
        test_widgets(|tester| {
            let cases = [
                // Dense One line
                ((false, false, true), [(0.0, 48.0), (48.0, 48.0)]),
                // Non-dense One line
                ((false, false, false), [(0.0, 56.0), (56.0, 56.0)]),
                // Dense Two line
                ((true, false, true), [(8.0, 48.0), (64.0 + 8.0, 48.0)]),
                // Non-dense Two line
                ((true, false, false), [(8.0, 56.0), (72.0 + 8.0, 56.0)]),
                // Dense Three line
                ((true, true, true), [(16.0, 48.0), (76.0 + 16.0, 48.0)]),
                // Non-dense Three line
                ((true, true, false), [(16.0, 56.0), (88.0 + 16.0, 56.0)]),
            ];
            for ((subtitle, is_three_line, dense), expected) in cases {
                assert_eq!(
                    oversized_rects_dense(tester, m2, true, subtitle, is_three_line, dense),
                    expected.map(|(top, height)| Rect::from_ltwh(16.0, top, 24.0, height))
                );
            }
        });
    }

    /// "Material 2: ListTile trailing icon height does not exceed ListTile height".
    #[test]
    fn material2_list_tile_trailing_icon_height_does_not_exceed_list_tile_height() {
        test_widgets(|tester| {
            let cases = [
                // Dense One line
                ((false, false, true), [(0.0, 48.0), (48.0, 48.0)]),
                // Non-dense One line
                ((false, false, false), [(0.0, 56.0), (56.0, 56.0)]),
                // Dense Two line
                ((true, false, true), [(8.0, 48.0), (64.0 + 8.0, 48.0)]),
                // Non-dense Two line
                ((true, false, false), [(8.0, 56.0), (72.0 + 8.0, 56.0)]),
                // Dense Three line
                ((true, true, true), [(16.0, 48.0), (76.0 + 16.0, 48.0)]),
                // Non-dense Three line
                ((true, true, false), [(16.0, 56.0), (88.0 + 16.0, 56.0)]),
            ];
            for ((subtitle, is_three_line, dense), expected) in cases {
                assert_eq!(
                    oversized_rects_dense(tester, m2, false, subtitle, is_three_line, dense),
                    expected.map(|(top, height)| Rect::from_ltwh(
                        800.0 - 16.0 - 24.0,
                        top,
                        24.0,
                        height
                    ))
                );
            }
        });
    }

    /// "Material 2: ListTile wide leading Widget".
    #[test]
    fn material2_list_tile_wide_leading_widget() {
        test_widgets(|tester| check_wide_leading(tester, m2, 16.0, 56.0));
    }

    /// "Material 2: ListTile horizontalTitleGap = 0.0".
    #[test]
    fn material2_list_tile_horizontal_title_gap_0() {
        test_widgets(|tester| {
            for (text_direction, edge) in [(TextDirection::Ltr, 56.0), (TextDirection::Rtl, 744.0)]
            {
                let frames = [
                    (None, Some(0.0)),
                    (Some(0.0), None),
                    (Some(10.0), Some(0.0)),
                ];
                for (theme_gap, widget_gap) in frames {
                    let mut theme_data = ListTileThemeData::new();
                    theme_data.horizontal_title_gap = theme_gap;
                    let mut tile = l_title_t();
                    tile.horizontal_title_gap = widget_gap;
                    spacing_frame_in(tester, m2, theme_data, text_direction, tile);
                    assert_eq!(
                        title_edge(tester, text_direction),
                        (edge, Size::new(800.0, 56.0))
                    );
                }
            }
        });
    }

    /// "Material 2: ListTile horizontalTitleGap = (default) && ListTile minLeadingWidth =
    /// (default)".
    #[test]
    fn material2_list_tile_default_horizontal_title_gap_and_min_leading_width() {
        test_widgets(|tester| {
            spacing_frame_in(
                tester,
                m2,
                ListTileThemeData::new(),
                TextDirection::Ltr,
                l_title_t(),
            );
            // horizontalTitleGap: ListTileDefaultValue.horizontalTitleGap (16.0)
            assert_eq!(
                title_edge(tester, TextDirection::Ltr),
                (72.0, Size::new(800.0, 56.0))
            );

            spacing_frame_in(
                tester,
                m2,
                ListTileThemeData::new(),
                TextDirection::Rtl,
                l_title_t(),
            );
            // horizontalTitleGap: ListTileDefaultValue.horizontalTitleGap (16.0)
            assert_eq!(
                title_edge(tester, TextDirection::Rtl),
                (728.0, Size::new(800.0, 56.0))
            );
        });
    }

    /// "Material 2: ListTile horizontalTitleGap with visualDensity".
    #[test]
    fn material2_list_tile_horizontal_title_gap_with_visual_density() {
        test_widgets(|tester| {
            spacing_frame_in(
                tester,
                m2,
                ListTileThemeData::new(),
                TextDirection::Ltr,
                l_title_t()
                    .visual_density(VisualDensity::new(VisualDensity::MINIMUM_DENSITY, 0.0))
                    .horizontal_title_gap(10.0),
            );
            assert_eq!(
                title_edge(tester, TextDirection::Ltr),
                (58.0, Size::new(800.0, 56.0))
            );
        });
    }

    /// "Material 2: ListTile minVerticalPadding = 80.0".
    #[test]
    fn material2_list_tile_min_vertical_padding_80() {
        test_widgets(|tester| {
            for text_direction in [TextDirection::Ltr, TextDirection::Rtl] {
                let frames = [
                    (None, Some(80.0)),
                    (Some(80.0), None),
                    (Some(0.0), Some(80.0)),
                ];
                for (theme_padding, widget_padding) in frames {
                    let mut theme_data = ListTileThemeData::new();
                    theme_data.min_vertical_padding = theme_padding;
                    let mut tile = l_title_t();
                    tile.min_vertical_padding = widget_padding;
                    spacing_frame_in(tester, m2, theme_data, text_direction, tile);
                    // 80 + 80 + 16(Title) = 176
                    assert_eq!(tile_rect(tester, 0).size(), Size::new(800.0, 176.0));
                }
            }
        });
    }

    /// The default text styles each slot's `TestText` saw under `theme`, for a tile that is
    /// `dense` and of `style`.
    fn slot_text_styles_of(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        dense: bool,
        style: Option<ListTileStyle>,
        settle: bool,
    ) -> [TextStyle; 4] {
        let seen: [Rc<RefCell<Option<TextStyle>>>; 4] =
            std::array::from_fn(|_| Rc::new(RefCell::new(None)));
        pump_app(tester, theme, |_app| {
            let mut tile = ListTile::new()
                .dense(dense)
                .leading(test_text("leading", &seen[0]))
                .title(test_text("title", &seen[1]))
                .subtitle(test_text("subtitle", &seen[2]))
                .trailing(test_text("trailing", &seen[3]));
            tile.style = style;
            Material::new()
                .child(Center::new().child(tile))
                .into_widget()
        });
        if settle {
            tester.pump_and_settle();
        }
        seen.map(|seen| seen.borrow_mut().take().expect("built"))
    }

    /// "Material 2: ListTile font size".
    #[test]
    fn material2_list_tile_font_size() {
        test_widgets(|tester| {
            // Dart pumps and settles every frame but the first.
            let sizes = |tester: &mut WidgetTester,
                         dense: bool,
                         style: Option<ListTileStyle>,
                         settle: bool| {
                slot_text_styles_of(tester, m2, dense, style, settle).map(|style| style.font_size)
            };
            // ListTile default text sizes.
            assert_eq!(
                sizes(tester, false, None, false),
                [Some(14.0), Some(16.0), Some(14.0), Some(14.0)]
            );
            // ListTile dense text sizes.
            assert_eq!(
                sizes(tester, true, None, true),
                [Some(14.0), Some(13.0), Some(12.0), Some(14.0)]
            );
            // ListTile default text sizes with ListTileStyle.drawer.
            assert_eq!(
                sizes(tester, false, Some(ListTileStyle::Drawer), true),
                [Some(14.0), Some(14.0), Some(14.0), Some(14.0)]
            );
            // ListTile dense text sizes with ListTileStyle.drawer.
            assert_eq!(
                sizes(tester, true, Some(ListTileStyle::Drawer), true),
                [Some(14.0), Some(13.0), Some(12.0), Some(14.0)]
            );
        });
    }

    /// "Material 2: ListTile text color".
    #[test]
    fn material2_list_tile_text_color() {
        test_widgets(|tester| {
            let theme = m2(&tester.cell.borrow());
            let color_of = |style: &Option<TextStyle>| {
                style
                    .as_ref()
                    .and_then(|style| style.color.as_ref().map(AnyColor::color))
            };
            let expected = [
                color_of(&theme.text_theme.body_medium),
                color_of(&theme.text_theme.title_medium),
                color_of(&theme.text_theme.body_small),
                color_of(&theme.text_theme.body_medium),
            ];
            for style in [None, Some(ListTileStyle::Drawer)] {
                let colors = slot_text_styles_of(tester, m2, false, style, false)
                    .map(|style| style.color.as_ref().map(AnyColor::color));
                assert_eq!(colors, expected);
            }
        });
    }

    /// "Material 2: selected, enabled ListTile default icon color, light and dark themes".
    #[test]
    fn material2_selected_enabled_list_tile_default_icon_color_light_and_dark_themes() {
        test_widgets(|_| {
            // Regression test for https://github.com/flutter/flutter/pull/77004
            fn light(app: &App) -> ThemeData {
                ThemeData::from(crate::color_scheme::ColorScheme::light())
                    .use_material3(false)
                    .build(app)
            }
            fn dark(app: &App) -> ThemeData {
                ThemeData::from(crate::color_scheme::ColorScheme::dark())
                    .use_material3(false)
                    .build(app)
            }
            // Dart pumps every frame on one tester, where the dark frame shows the new theme at
            // once; here the theme animates from the last frame's, so each frame has a tester.
            let slot_icon_colors = |theme: fn(&App) -> ThemeData, selected: bool| {
                slot_icon_colors(&mut WidgetTester::new(), theme, selected)
            };
            let light_color_scheme = crate::color_scheme::ColorScheme::light();
            let dark_color_scheme = crate::color_scheme::ColorScheme::dark();

            assert_eq!(
                slot_icon_colors(light, true),
                [Some(light_color_scheme.primary); 4]
            );
            assert_eq!(slot_icon_colors(light, false), [Some(Colors::BLACK45); 4]);
            assert_eq!(
                slot_icon_colors(dark, true),
                [Some(dark_color_scheme.primary); 4]
            );
            // For this configuration, ListTile defers to the default IconTheme.
            // The default dark theme's IconTheme has color:white
            assert_eq!(slot_icon_colors(dark, false), [Some(Colors::WHITE); 4]);
        });
    }

    /// "Material 2: ListTile default tile color": valo records no transparent draw, so the
    /// colour is read from the tile's `Ink` decoration.
    #[test]
    fn material2_list_tile_default_tile_color() {
        test_widgets(|tester| {
            toggling_tile(tester, m2, ListTile::new);
            assert_eq!(ink_tile_color(tester), Some(Colors::TRANSPARENT));

            // Tap on tile to change isSelected.
            let center = tile_rect(tester, 0).center();
            tester.tap_at(center, 1);
            tester.pump_and_settle();

            assert_eq!(ink_tile_color(tester), Some(Colors::TRANSPARENT));
        });
    }

    /// "Material 2: titleAlignment position with title widget".
    #[test]
    fn material2_title_alignment_position_with_title_widget() {
        test_widgets(|tester| {
            let mut case = AlignmentCase::new(tester, m2);
            const TILE_HEIGHT: f64 = MIN_VERTICAL_PADDING * 2.0 + TITLE_HEIGHT;
            const CENTER_POSITION: f64 = (TILE_HEIGHT / 2.0) - (LEADING_HEIGHT / 2.0);
            const TITLE_POSITION: f64 = 16.0;
            // If [ThemeData.useMaterial3] is false, the default title alignment is
            // [ListTileTitleAlignment.titleHeight]: if the tile height is less than 72.0
            // pixels, the leading widget is placed 16.0 pixels below the top of the title
            // widget, and the trailing is centered vertically in the tile.
            assert_eq!(
                case.offsets(None, false, false),
                (TITLE_POSITION, CENTER_POSITION)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::ThreeLine), false, false),
                (CENTER_POSITION, CENTER_POSITION)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::TitleHeight), false, false),
                (TITLE_POSITION, CENTER_POSITION)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Top), false, false),
                (MIN_VERTICAL_PADDING, MIN_VERTICAL_PADDING)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Center), false, false),
                (CENTER_POSITION, CENTER_POSITION)
            );
            const BOTTOM_POSITION: f64 = TILE_HEIGHT - MIN_VERTICAL_PADDING - LEADING_HEIGHT;
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Bottom), false, false),
                (BOTTOM_POSITION, BOTTOM_POSITION)
            );
        });
    }

    /// "Material 2: titleAlignment position with title and subtitle widgets".
    #[test]
    fn material2_title_alignment_position_with_title_and_subtitle_widgets() {
        test_widgets(|tester| {
            let mut case = AlignmentCase::new(tester, m2);
            const TILE_HEIGHT: f64 = MIN_VERTICAL_PADDING * 2.0 + TITLE_HEIGHT + 50.0;
            const CENTER_POSITION: f64 = (TILE_HEIGHT / 2.0) - (LEADING_HEIGHT / 2.0);
            const TITLE_POSITION: f64 = 16.0;
            assert_eq!(
                case.offsets(None, true, false),
                (TITLE_POSITION, TITLE_POSITION)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::ThreeLine), true, false),
                (CENTER_POSITION, CENTER_POSITION)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::TitleHeight), true, false),
                (TITLE_POSITION, TITLE_POSITION)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Top), true, false),
                (MIN_VERTICAL_PADDING, MIN_VERTICAL_PADDING)
            );
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Center), true, false),
                (CENTER_POSITION, CENTER_POSITION)
            );
            const BOTTOM_POSITION: f64 = TILE_HEIGHT - MIN_VERTICAL_PADDING - LEADING_HEIGHT;
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::Bottom), true, false),
                (BOTTOM_POSITION, BOTTOM_POSITION)
            );
        });
    }

    /// "Material 2: ListTile.isThreeLine updates ListTileTitleAlignment.threeLine's
    /// alignment".
    #[test]
    fn material2_list_tile_is_three_line_updates_the_three_line_alignment() {
        test_widgets(|tester| {
            let mut case = AlignmentCase::new(tester, m2);
            const TILE_HEIGHT: f64 = MIN_VERTICAL_PADDING * 2.0 + TITLE_HEIGHT + 50.0;
            const LEADING_TRAILING_POSITION: f64 = (TILE_HEIGHT / 2.0) - (LEADING_HEIGHT / 2.0);
            // If title alignment is threeLine and [ListTile.isThreeLine] is false,
            // leading and trailing widgets are centered vertically in the tile.
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::ThreeLine), true, false),
                (LEADING_TRAILING_POSITION, LEADING_TRAILING_POSITION)
            );
            // Set [ListTile.isThreeLine] to true to update the alignment.
            assert_eq!(
                case.offsets(Some(ListTileTitleAlignment::ThreeLine), true, true),
                (MIN_VERTICAL_PADDING, MIN_VERTICAL_PADDING)
            );
        });
    }

    /// "ListTile can be triggered by keyboard shortcuts".
    #[test]
    fn list_tile_can_be_triggered_by_keyboard_shortcuts() {
        test_widgets(|tester| {
            let tapped = Rc::new(Cell::new(false));
            pump_app(tester, m3, |_app| {
                let tapped = Rc::clone(&tapped);
                Material::new()
                    .child(Center::new().child(StatefulBuilder::new(
                        move |_app, _context, set_state| {
                            let tapped = Rc::clone(&tapped);
                            SizedBox::new()
                                .width(200.0)
                                .height(100.0)
                                .child(
                                    ListTile::new()
                                        .on_tap(Listener::new(move |app| {
                                            let tapped = Rc::clone(&tapped);
                                            set_state(app, &mut |_app| tapped.set(true));
                                        }))
                                        .hover_color(Colors::ORANGE.shade500())
                                        .autofocus(true),
                                )
                                .into_widget()
                        },
                    )))
                    .into_widget()
            });
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            tester.pump_and_settle();

            tester.send_key_event(
                waft_services::PhysicalKeyboardKey::SPACE,
                waft_services::LogicalKeyboardKey::SPACE,
            );
            tester.pump_and_settle();

            assert!(tapped.get());
        });
    }

    /// Whether the tile's `InkWell` asks for feedback: what the `feedback` group's click
    /// sound count stands for, as the click system sound waits with `SystemSound`.
    fn ink_well_enable_feedback(
        tester: &mut WidgetTester,
        tile: ListTile,
        theme_feedback: Option<bool>,
    ) -> bool {
        pump_app(tester, m3, |_app| {
            let tile = tile.title(Text::new("Title")).on_tap(noop());
            let child = match theme_feedback {
                Some(enable_feedback) => ListTileTheme::new(tile)
                    .data(ListTileThemeData::new().enable_feedback(enable_feedback))
                    .into_widget(),
                None => tile.into_widget(),
            };
            Material::new().child(child).into_widget()
        });
        let ink_well = tester.element(find::by_type::<InkWell>().at(0));
        tester.widget(ink_well, |ink_well: &InkWell| {
            ink_well.ink_response.enable_feedback
        })
    }

    /// "feedback: ListTile with disabled feedback".
    #[test]
    fn list_tile_with_disabled_feedback() {
        test_widgets(|tester| {
            assert!(!ink_well_enable_feedback(
                tester,
                ListTile::new().enable_feedback(false),
                None
            ));
        });
    }

    /// "feedback: ListTile with enabled feedback".
    #[test]
    fn list_tile_with_enabled_feedback() {
        test_widgets(|tester| {
            assert!(ink_well_enable_feedback(
                tester,
                ListTile::new().enable_feedback(true),
                None
            ));
        });
    }

    /// "feedback: ListTile with enabled feedback by default".
    #[test]
    fn list_tile_with_enabled_feedback_by_default() {
        test_widgets(|tester| {
            assert!(ink_well_enable_feedback(tester, ListTile::new(), None));
        });
    }

    /// "feedback: ListTile with disabled feedback using ListTileTheme".
    #[test]
    fn list_tile_with_disabled_feedback_using_list_tile_theme() {
        test_widgets(|tester| {
            assert!(!ink_well_enable_feedback(
                tester,
                ListTile::new(),
                Some(false)
            ));
        });
    }

    /// "feedback: ListTile.enableFeedback overrides ListTileTheme.enableFeedback".
    #[test]
    fn list_tile_enable_feedback_overrides_list_tile_theme_enable_feedback() {
        test_widgets(|tester| {
            assert!(ink_well_enable_feedback(
                tester,
                ListTile::new().enable_feedback(true),
                Some(false)
            ));
        });
    }
}
