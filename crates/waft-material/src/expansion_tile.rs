//! Flutter counterpart: material_ui's `expansion_tile.dart`.
//!
//! The state announcement `SemanticsService.sendAnnouncement` makes when the tile expands or
//! collapses (with the iOS `Timer` that delays it) waits with the semantics tree.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, AnimationStyle, AnyAnimation, ColorTween, Curve, CurveTween, Curves, Tween,
};
use waft_embedder::{Clip, Color, TargetPlatform};
use waft_foundation::{App, Handle, HandleId, ListenableObject, Listener, ValueChanged};
use waft_painting::{
    Alignment, AlignmentGeometry, Border, BorderSide, Decoration, EdgeInsetsGeometry, ShapeBorder,
    ShapeDecoration,
};
use waft_rendering::{AccessibilityFocusBlockType, CrossAxisAlignment, MainAxisSize};
use waft_widgets::{
    Align, BuildContext, Column, DecoratedBox, Expansible, ExpansibleController, Icon, IntoWidget,
    KeyRef, Padding, RotationTransition, Semantics, State, StateData, StatefulWidget, WidgetRef,
    WidgetStatesController,
};

use crate::colors::Colors;
use crate::expansion_tile_theme::{ExpansionTileTheme, ExpansionTileThemeData};
use crate::generated::expansion_tile_defaults_m3::expansion_tile_defaults_m3;
use crate::icons::Icons;
use crate::list_tile::{ListTile, ListTileControlAffinity};
use crate::list_tile_theme::ListTileTheme;
use crate::material::{Material, MaterialType, ShapeBorderTween};
use crate::material_localizations::MaterialLocalizations;
use crate::theme::Theme;
use crate::theme_data::{ThemeData, VisualDensity};

const K_EXPAND: Duration = Duration::from_millis(200);

/// Enables control over a single [`ExpansionTile`]'s expanded/collapsed state.
///
/// It can be useful to expand or collapse an [`ExpansionTile`]
/// programmatically, for example to reconfigure an existing expansion
/// tile based on a system event. To do so, create an [`ExpansionTile`]
/// with an [`ExpansionTileController`] that's owned by a stateful widget
/// or look up the tile's automatically created [`ExpansionTileController`]
/// with [`ExpansibleController::of`].
///
/// Typical usage of the [`ExpansibleController::of`] function is to call it from within the
/// `build` method of a descendant of an [`ExpansionTile`].
///
/// When the [`ExpansionTile`] is actually created in the same `build`
/// function as the callback that refers to the controller, then the
/// `context` argument to the `build` function can't be used to find
/// the [`ExpansionTileController`] (since it's "above" the widget
/// being returned in the widget tree). In cases like that you can
/// add a `Builder` widget, which provides a new scope with a
/// [`BuildContext`] that is "under" the [`ExpansionTile`].
///
/// A more efficient solution is to split your build function into
/// several widgets. This introduces a new context from which you
/// can obtain the [`ExpansionTileController`]. With this approach you
/// would have an outer widget that creates the [`ExpansionTile`]
/// populated by instances of your new inner widgets, and then in
/// these inner widgets you would use `ExpansionTileController::of`.
///
/// The  [`ExpansibleController::expand`] and [`ExpansibleController::collapse`]
/// methods cause the [`ExpansionTile`] to rebuild, so they may not be called from
/// a build method.
///
/// Remember to dispose of the [`ExpansionTileController`] when it is no longer
/// needed. This will ensure we discard any resources used by the object.
#[deprecated(
    note = "Use ExpansibleController instead. This feature was deprecated after v3.31.0-0.1.pre."
)]
pub type ExpansionTileController = ExpansibleController;

/// A single-line [`ListTile`] with an expansion arrow icon that expands or collapses
/// the tile to reveal or hide the [`children`](Self::children).
///
/// This widget is typically used with `ListView` to create an "expand /
/// collapse" list entry. When used with scrolling widgets like `ListView`, a
/// unique `PageStorageKey` must be specified as the [`key`](Self::key), to enable the
/// [`ExpansionTile`] to save and restore its expanded state when it is scrolled
/// in and out of view.
///
/// This class overrides the `ListTileThemeData::icon_color` and `ListTileThemeData::text_color`
/// theme properties for its [`ListTile`]. These colors animate between values when
/// the tile is expanded and collapsed: between [`icon_color`](Self::icon_color),
/// [`collapsed_icon_color`](Self::collapsed_icon_color) and
/// between [`text_color`](Self::text_color) and [`collapsed_text_color`](Self::collapsed_text_color).
///
/// The expansion arrow icon is shown on the right by default in left-to-right languages
/// (i.e. the trailing edge). This can be changed using [`control_affinity`](Self::control_affinity).
/// This maps to the [`leading`](Self::leading) and [`trailing`](Self::trailing) properties of
/// [`ExpansionTile`].
///
/// ## Accessibility
///
/// The accessibility behavior of [`ExpansionTile`] is platform adaptive, based on
/// the device's actual platform rather than the theme's platform setting. This
/// ensures that assistive technologies like VoiceOver on iOS and macOS receive
/// the correct platform-specific semantics hints, even when the app's theme is
/// configured to mimic a different platform's appearance.
///
/// See also:
///
///  * [`ListTile`], useful for creating expansion tile [`children`](Self::children) when the
///    expansion tile represents a sublist.
///  * The "Expand and collapse" section of
///    <https://material.io/components/lists#types>
pub struct ExpansionTile {
    pub key: Option<KeyRef>,

    /// A widget to display before the title.
    ///
    /// Typically a `CircleAvatar` widget.
    ///
    /// Depending on the value of [`control_affinity`](Self::control_affinity), the
    /// [`leading`](Self::leading) widget may replace the rotating expansion arrow icon.
    pub leading: Option<WidgetRef>,

    /// The primary content of the list item.
    ///
    /// Typically a `Text` widget.
    pub title: WidgetRef,

    /// Additional content displayed below the title.
    ///
    /// Typically a `Text` widget.
    pub subtitle: Option<WidgetRef>,

    /// Called when the tile expands or collapses.
    ///
    /// When the tile starts expanding, this function is called with the value
    /// true. When the tile starts collapsing, this function is called with
    /// the value false.
    ///
    /// Instead of providing this property, consider adding this callback as a
    /// listener to a provided [`controller`](Self::controller).
    pub on_expansion_changed: Option<ValueChanged<bool>>,

    /// The widgets that are displayed when the tile expands.
    ///
    /// Typically [`ListTile`] widgets.
    pub children: Vec<WidgetRef>,

    /// The color to display behind the sublist when expanded.
    ///
    /// If this property is `None` then [`ExpansionTileThemeData::background_color`] is used.
    /// If that is also `None` then `Colors::TRANSPARENT` is used.
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub background_color: Option<Color>,

    /// When not `None`, defines the background color of tile when the sublist is collapsed.
    ///
    /// If this property is `None` then
    /// [`ExpansionTileThemeData::collapsed_background_color`] is used. If that is also `None`
    /// then `Colors::TRANSPARENT` is used.
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub collapsed_background_color: Option<Color>,

    /// A widget to display after the title.
    ///
    /// Depending on the value of [`control_affinity`](Self::control_affinity), the
    /// [`trailing`](Self::trailing) widget may replace the rotating expansion arrow icon.
    pub trailing: Option<WidgetRef>,

    /// Specifies if the [`ExpansionTile`] should build a default trailing icon if
    /// [`trailing`](Self::trailing) is `None`.
    pub show_trailing_icon: bool,

    /// Specifies if the list tile is initially expanded (true) or collapsed (false).
    ///
    /// Alternatively, a provided [`controller`](Self::controller) can be used to initially
    /// expand the tile if [`ExpansibleController::expand`] is called before this widget is
    /// built.
    ///
    /// Defaults to false.
    pub initially_expanded: bool,

    /// Specifies whether the state of the children is maintained when the tile expands and
    /// collapses.
    ///
    /// When true, the children are kept in the tree while the tile is collapsed.
    /// When false (default), the children are removed from the tree when the tile is
    /// collapsed and recreated upon expansion.
    pub maintain_state: bool,

    /// Specifies padding for the [`ListTile`].
    ///
    /// Analogous to `ListTile::content_padding`, this property defines the insets for
    /// the [`leading`](Self::leading), [`title`](Self::title), [`subtitle`](Self::subtitle)
    /// and [`trailing`](Self::trailing) widgets. It does not inset the expanded
    /// [`children`](Self::children) widgets.
    ///
    /// If this property is `None` then [`ExpansionTileThemeData::tile_padding`] is used. If
    /// that is also `None` then the tile's padding is `EdgeInsets.symmetric(horizontal: 16.0)`.
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub tile_padding: Option<EdgeInsetsGeometry>,

    /// Specifies the alignment of [`children`](Self::children), which are arranged in a column
    /// when the tile is expanded.
    ///
    /// The internals of the expanded tile make use of a [`Column`] widget for
    /// [`children`](Self::children), and [`Align`] widget to align the column. The
    /// [`expanded_alignment`](Self::expanded_alignment) parameter is passed directly into the
    /// [`Align`].
    ///
    /// Modifying this property controls the alignment of the column within the
    /// expanded tile, not the alignment of [`children`](Self::children) widgets within the
    /// column. To align each child within [`children`](Self::children), see
    /// [`expanded_cross_axis_alignment`](Self::expanded_cross_axis_alignment).
    ///
    /// The width of the column is the width of the widest child widget in
    /// [`children`](Self::children).
    ///
    /// If this property is `None` then [`ExpansionTileThemeData::expanded_alignment`]is used.
    /// If that is also `None` then the value of [`expanded_alignment`](Self::expanded_alignment)
    /// is [`Alignment::CENTER`].
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub expanded_alignment: Option<AlignmentGeometry>,

    /// Specifies the alignment of each child within [`children`](Self::children) when the
    /// tile is expanded.
    ///
    /// The internals of the expanded tile make use of a [`Column`] widget for
    /// [`children`](Self::children), and the `cross_axis_alignment` parameter is passed
    /// directly into the [`Column`].
    ///
    /// Modifying this property controls the cross axis alignment of each child
    /// within its [`Column`]. The width of the [`Column`] that houses
    /// [`children`](Self::children) will be the same as the widest child widget in
    /// [`children`](Self::children). The width of the [`Column`] might not be equal to the
    /// width of the expanded tile.
    ///
    /// To align the [`Column`] along the expanded tile, use the
    /// [`expanded_alignment`](Self::expanded_alignment) property instead.
    ///
    /// When the value is `None`, the value of
    /// [`expanded_cross_axis_alignment`](Self::expanded_cross_axis_alignment) is
    /// [`CrossAxisAlignment::Center`].
    pub expanded_cross_axis_alignment: Option<CrossAxisAlignment>,

    /// Specifies padding for [`children`](Self::children).
    ///
    /// If this property is `None` then [`ExpansionTileThemeData::children_padding`] is used.
    /// If that is also `None` then the value of [`children_padding`](Self::children_padding)
    /// is `EdgeInsets.zero`.
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub children_padding: Option<EdgeInsetsGeometry>,

    /// The icon color of tile's expansion arrow icon when the sublist is expanded.
    ///
    /// Used to override to the `ListTileThemeData::icon_color`.
    ///
    /// If this property is `None` then [`ExpansionTileThemeData::icon_color`] is used. If that
    /// is also `None` then the value of `ColorScheme::primary` is used.
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub icon_color: Option<Color>,

    /// The icon color of tile's expansion arrow icon when the sublist is collapsed.
    ///
    /// Used to override to the `ListTileThemeData::icon_color`.
    ///
    /// If this property is `None` then [`ExpansionTileThemeData::collapsed_icon_color`] is
    /// used. If that is also `None` and `ThemeData::use_material3` is true,
    /// `ColorScheme::on_surface` is used. Otherwise, defaults to
    /// `ThemeData::unselected_widget_color` color.
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub collapsed_icon_color: Option<Color>,

    /// The color of the tile's titles when the sublist is expanded.
    ///
    /// Used to override to the `ListTileThemeData::text_color`.
    ///
    /// If this property is `None` then [`ExpansionTileThemeData::text_color`] is used. If that
    /// is also `None` then and `ThemeData::use_material3` is true, color of the
    /// `TextTheme::body_large` will be used for the [`title`](Self::title) and
    /// [`subtitle`](Self::subtitle). Otherwise, defaults to `ColorScheme::primary` color.
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub text_color: Option<Color>,

    /// The color of the tile's titles when the sublist is collapsed.
    ///
    /// Used to override to the `ListTileThemeData::text_color`.
    ///
    /// If this property is `None` then [`ExpansionTileThemeData::collapsed_text_color`] is
    /// used. If that is also `None` and `ThemeData::use_material3` is true, color of the
    /// `TextTheme::body_large` will be used for the [`title`](Self::title) and
    /// [`subtitle`](Self::subtitle). Otherwise, defaults to color of the
    /// `TextTheme::title_medium`.
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub collapsed_text_color: Option<Color>,

    /// The tile's border shape when the sublist is expanded.
    ///
    /// If this property is `None`, the [`ExpansionTileThemeData::shape`] is used. If that
    /// is also `None`, a [`Border`] with vertical sides default to `ThemeData::divider_color`
    /// is used
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub shape: Option<Box<dyn ShapeBorder>>,

    /// The tile's border shape when the sublist is collapsed.
    ///
    /// If this property is `None`, the [`ExpansionTileThemeData::collapsed_shape`] is used. If
    /// that is also `None`, a [`Border`] with vertical sides default to Color
    /// `Colors::TRANSPARENT` is used
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub collapsed_shape: Option<Box<dyn ShapeBorder>>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// If this is not `None` and a custom collapsed or expanded shape is provided,
    /// the value of [`clip_behavior`](Self::clip_behavior) will be used to clip the expansion
    /// tile.
    ///
    /// If this property is `None`, the [`ExpansionTileThemeData::clip_behavior`] is used. If
    /// that is also `None`, defaults to [`Clip::AntiAlias`].
    ///
    /// See also:
    ///
    /// * [`ExpansionTileTheme::of`], which returns the nearest [`ExpansionTileTheme`]'s
    ///   [`ExpansionTileThemeData`].
    pub clip_behavior: Option<Clip>,

    /// Typically used to force the expansion arrow icon to the tile's leading or trailing
    /// edge.
    ///
    /// By default, the value of [`control_affinity`](Self::control_affinity) is
    /// [`ListTileControlAffinity::Platform`], which means that the expansion arrow icon will
    /// appear on the tile's trailing edge.
    pub control_affinity: Option<ListTileControlAffinity>,

    /// If provided, the controller can be used to expand and collapse tiles.
    ///
    /// In cases where control over the tile's state is needed from a callback
    /// triggered by a widget within the tile, [`ExpansibleController::of`] may be
    /// more convenient than supplying a controller.
    pub controller: Option<Handle<ExpansibleController>>,

    /// Whether this list tile is part of a vertically dense list.
    ///
    /// If this property is `None` then its value is based on `ListTileThemeData::dense`.
    ///
    /// Dense list tiles default to a smaller height.
    pub dense: Option<bool>,

    /// The splash color of the ink response when the tile is tapped.
    ///
    /// This color is passed directly to the underlying [`ListTile`]'s
    /// `splash_color` property, which controls the ink ripple (splash)
    /// animation when the tile is tapped. Internally, [`ListTile`] uses
    /// an `InkWell` (which handles the actual splash effect), and so the
    /// provided color will apply to that ripple.
    ///
    /// If `None`, the splash color will default to the current theme’s
    /// `ThemeData::splash_color`.
    ///
    /// See also:
    ///
    /// * `ListTile::splash_color`, which sets the ink splash for the tile.
    /// * `InkWell::splash_color`, which determines the color of the ripple
    ///   effect in Material widgets.
    /// * `ThemeData::splash_color`, which provides a fallback color.
    pub splash_color: Option<Color>,

    /// Defines how compact the expansion tile's layout will be.
    ///
    /// Compactness for components is defined by [`VisualDensity`]; see
    /// `ThemeData::visual_density`.
    pub visual_density: Option<VisualDensity>,

    /// The minimum height allocated for the [`ListTile`] widget.
    ///
    /// If this is `None`, default tile heights are 56.0, 72.0, and 88.0 for one,
    /// two, and three lines of text respectively. If `ListTile::dense` is true,
    /// these defaults are changed to 48.0, 64.0, and 76.0.
    pub min_tile_height: Option<f64>,

    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a
    /// long-press will produce a short vibration, when feedback is enabled.
    ///
    /// When `ListTile::enable_feedback` is `None`, `ListTileThemeData::enable_feedback`
    /// is used; when that is also `None`, the default is true.
    pub enable_feedback: Option<bool>,

    /// Whether this expansion tile is interactive.
    ///
    /// If false, the internal [`ListTile`] will be disabled, changing its
    /// appearance according to the theme and disabling user interaction.
    ///
    /// Even if disabled, the expansion can still be toggled programmatically
    /// through an [`ExpansionTileController`].
    pub enabled: bool,

    /// Used to override the expansion animation curve and duration.
    ///
    /// If [`AnimationStyle::duration`] is provided, it will be used to override
    /// the expansion animation duration. If it is `None`, then [`AnimationStyle::duration`]
    /// from the [`ExpansionTileThemeData::expansion_animation_style`] will be used.
    /// Otherwise, defaults to 200ms.
    ///
    /// If [`AnimationStyle::curve`] is provided, it will be used to override
    /// the expansion animation curve. If it is `None`, then [`AnimationStyle::curve`]
    /// from the [`ExpansionTileThemeData::expansion_animation_style`] will be used.
    /// Otherwise, defaults to [`Curves::ease_in`].
    ///
    /// If [`AnimationStyle::reverse_curve`] is provided, it will be used to override
    /// the collapse animation curve. If it is `None`, then [`AnimationStyle::reverse_curve`]
    /// from the [`ExpansionTileThemeData::expansion_animation_style`] will be used.
    /// Otherwise, the same curve will be used as for expansion.
    ///
    /// To disable the theme animation, use [`AnimationStyle::NO_ANIMATION`].
    pub expansion_animation_style: Option<AnimationStyle>,

    /// Whether to add button:true to the semantics if onTap is provided.
    /// This is a temporary flag to help changing the behavior of ListTile onTap semantics.
    pub internal_add_semantic_for_on_tap: bool,

    /// The controller that notifies when the widget's `WidgetState`s change.
    ///
    /// This allows listening to and controlling states such as
    /// `WidgetState::Hovered`, `WidgetState::Focused`, `WidgetState::Pressed`,
    /// and `WidgetState::Disabled` for the tile's header.
    ///
    /// If `None`, the backing [`ListTile`] will create and manage its own
    /// `WidgetStatesController`.
    pub states_controller: Option<Handle<WidgetStatesController>>,
}

impl ExpansionTile {
    /// Creates a single-line [`ListTile`] with an expansion arrow icon that expands or
    /// collapses the tile to reveal or hide the [`children`](Self::children); Dart's optional
    /// named arguments are the setters.
    pub fn new<K>(title: impl IntoWidget<K>) -> ExpansionTile {
        ExpansionTile {
            key: None,
            leading: None,
            title: title.into_widget(),
            subtitle: None,
            on_expansion_changed: None,
            children: Vec::new(),
            background_color: None,
            collapsed_background_color: None,
            trailing: None,
            show_trailing_icon: true,
            initially_expanded: false,
            maintain_state: false,
            tile_padding: None,
            expanded_alignment: None,
            expanded_cross_axis_alignment: None,
            children_padding: None,
            icon_color: None,
            collapsed_icon_color: None,
            text_color: None,
            collapsed_text_color: None,
            shape: None,
            collapsed_shape: None,
            clip_behavior: None,
            control_affinity: None,
            controller: None,
            dense: None,
            splash_color: None,
            visual_density: None,
            min_tile_height: None,
            enable_feedback: Some(true),
            enabled: true,
            expansion_animation_style: None,
            internal_add_semantic_for_on_tap: false,
            states_controller: None,
        }
    }

    /// Dart `ExpansionTile(key:)`.
    pub fn key(mut self, key: KeyRef) -> ExpansionTile {
        self.key = Some(key);
        self
    }

    /// Dart `ExpansionTile(leading:)`.
    pub fn leading<K>(mut self, leading: impl IntoWidget<K>) -> ExpansionTile {
        self.leading = Some(leading.into_widget());
        self
    }

    /// Dart `ExpansionTile(subtitle:)`.
    pub fn subtitle<K>(mut self, subtitle: impl IntoWidget<K>) -> ExpansionTile {
        self.subtitle = Some(subtitle.into_widget());
        self
    }

    /// Dart `ExpansionTile(onExpansionChanged:)`.
    pub fn on_expansion_changed(
        mut self,
        on_expansion_changed: ValueChanged<bool>,
    ) -> ExpansionTile {
        self.on_expansion_changed = Some(on_expansion_changed);
        self
    }

    /// Dart `ExpansionTile(children:)`.
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> ExpansionTile {
        self.children = children.into_iter().collect();
        self
    }

    /// Dart `ExpansionTile(trailing:)`.
    pub fn trailing<K>(mut self, trailing: impl IntoWidget<K>) -> ExpansionTile {
        self.trailing = Some(trailing.into_widget());
        self
    }

    /// Dart `ExpansionTile(showTrailingIcon:)`.
    pub fn show_trailing_icon(mut self, show_trailing_icon: bool) -> ExpansionTile {
        self.show_trailing_icon = show_trailing_icon;
        self
    }

    /// Dart `ExpansionTile(initiallyExpanded:)`.
    pub fn initially_expanded(mut self, initially_expanded: bool) -> ExpansionTile {
        self.initially_expanded = initially_expanded;
        self
    }

    /// Dart `ExpansionTile(maintainState:)`.
    pub fn maintain_state(mut self, maintain_state: bool) -> ExpansionTile {
        self.maintain_state = maintain_state;
        self
    }

    /// Dart `ExpansionTile(tilePadding:)`.
    pub fn tile_padding(mut self, tile_padding: impl Into<EdgeInsetsGeometry>) -> ExpansionTile {
        self.tile_padding = Some(tile_padding.into());
        self
    }

    /// Dart `ExpansionTile(expandedCrossAxisAlignment:)`.
    pub fn expanded_cross_axis_alignment(
        mut self,
        expanded_cross_axis_alignment: CrossAxisAlignment,
    ) -> ExpansionTile {
        debug_assert!(
            expanded_cross_axis_alignment != CrossAxisAlignment::Baseline,
            "CrossAxisAlignment.baseline is not supported since the expanded children \
             are aligned in a column, not a row. Try to use another constant.",
        );
        self.expanded_cross_axis_alignment = Some(expanded_cross_axis_alignment);
        self
    }

    /// Dart `ExpansionTile(expandedAlignment:)`.
    pub fn expanded_alignment(
        mut self,
        expanded_alignment: impl Into<AlignmentGeometry>,
    ) -> ExpansionTile {
        self.expanded_alignment = Some(expanded_alignment.into());
        self
    }

    /// Dart `ExpansionTile(childrenPadding:)`.
    pub fn children_padding(
        mut self,
        children_padding: impl Into<EdgeInsetsGeometry>,
    ) -> ExpansionTile {
        self.children_padding = Some(children_padding.into());
        self
    }

    /// Dart `ExpansionTile(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> ExpansionTile {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `ExpansionTile(collapsedBackgroundColor:)`.
    pub fn collapsed_background_color(
        mut self,
        collapsed_background_color: Color,
    ) -> ExpansionTile {
        self.collapsed_background_color = Some(collapsed_background_color);
        self
    }

    /// Dart `ExpansionTile(textColor:)`.
    pub fn text_color(mut self, text_color: Color) -> ExpansionTile {
        self.text_color = Some(text_color);
        self
    }

    /// Dart `ExpansionTile(collapsedTextColor:)`.
    pub fn collapsed_text_color(mut self, collapsed_text_color: Color) -> ExpansionTile {
        self.collapsed_text_color = Some(collapsed_text_color);
        self
    }

    /// Dart `ExpansionTile(iconColor:)`.
    pub fn icon_color(mut self, icon_color: Color) -> ExpansionTile {
        self.icon_color = Some(icon_color);
        self
    }

    /// Dart `ExpansionTile(collapsedIconColor:)`.
    pub fn collapsed_icon_color(mut self, collapsed_icon_color: Color) -> ExpansionTile {
        self.collapsed_icon_color = Some(collapsed_icon_color);
        self
    }

    /// Dart `ExpansionTile(shape:)`.
    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> ExpansionTile {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `ExpansionTile(collapsedShape:)`.
    pub fn collapsed_shape(
        mut self,
        collapsed_shape: impl Into<Box<dyn ShapeBorder>>,
    ) -> ExpansionTile {
        self.collapsed_shape = Some(collapsed_shape.into());
        self
    }

    /// Dart `ExpansionTile(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ExpansionTile {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `ExpansionTile(controlAffinity:)`.
    pub fn control_affinity(mut self, control_affinity: ListTileControlAffinity) -> ExpansionTile {
        self.control_affinity = Some(control_affinity);
        self
    }

    /// Dart `ExpansionTile(controller:)`.
    pub fn controller(mut self, controller: Handle<ExpansibleController>) -> ExpansionTile {
        self.controller = Some(controller);
        self
    }

    /// Dart `ExpansionTile(dense:)`.
    pub fn dense(mut self, dense: bool) -> ExpansionTile {
        self.dense = Some(dense);
        self
    }

    /// Dart `ExpansionTile(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> ExpansionTile {
        self.splash_color = Some(splash_color);
        self
    }

    /// Dart `ExpansionTile(visualDensity:)`.
    pub fn visual_density(mut self, visual_density: VisualDensity) -> ExpansionTile {
        self.visual_density = Some(visual_density);
        self
    }

    /// Dart `ExpansionTile(minTileHeight:)`.
    pub fn min_tile_height(mut self, min_tile_height: f64) -> ExpansionTile {
        self.min_tile_height = Some(min_tile_height);
        self
    }

    /// Dart `ExpansionTile(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> ExpansionTile {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    /// Dart `ExpansionTile(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> ExpansionTile {
        self.enabled = enabled;
        self
    }

    /// Dart `ExpansionTile(expansionAnimationStyle:)`.
    pub fn expansion_animation_style(
        mut self,
        expansion_animation_style: AnimationStyle,
    ) -> ExpansionTile {
        self.expansion_animation_style = Some(expansion_animation_style);
        self
    }

    /// Dart `ExpansionTile(internalAddSemanticForOnTap:)`.
    pub fn internal_add_semantic_for_on_tap(
        mut self,
        internal_add_semantic_for_on_tap: bool,
    ) -> ExpansionTile {
        self.internal_add_semantic_for_on_tap = internal_add_semantic_for_on_tap;
        self
    }

    /// Dart `ExpansionTile(statesController:)`.
    pub fn states_controller(
        mut self,
        states_controller: Handle<WidgetStatesController>,
    ) -> ExpansionTile {
        self.states_controller = Some(states_controller);
        self
    }
}

impl fmt::Debug for ExpansionTile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpansionTile")
            .field("initially_expanded", &self.initially_expanded)
            .field("maintain_state", &self.maintain_state)
            .field("controller", &self.controller)
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ExpansionTile {
    type State = ExpansionTileState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ExpansionTileState {
        ExpansionTileState {
            state: StateData::new(),
            ease_in_tween: None,
            ease_out_tween: None,
            half_tween: None,
            border_tween: ShapeBorderTween::new(None, None),
            header_color_tween: None,
            icon_color_tween: None,
            background_color_tween: None,
            icon_turns: None,
            border: None,
            header_color: None,
            icon_color: None,
            background_color: None,
            expansion_tile_theme: ExpansionTileThemeData::new(),
            tile_controller: None,
            curve: Curves::ease_in(),
            reverse_curve: None,
            duration: K_EXPAND,
        }
    }
}

/// Dart's `_ExpansionTileState`.
pub struct ExpansionTileState {
    state: StateData<ExpansionTile>,
    /// Dart's `static final` `_easeInTween`, `_easeOutTween` and `_halfTween`, and the
    /// `ColorTween`s Dart's field initializers make, created in `init_state` because an arena
    /// object needs the [`App`].
    ease_in_tween: Option<Handle<CurveTween>>,
    ease_out_tween: Option<Handle<CurveTween>>,
    half_tween: Option<Handle<Tween<f64>>>,
    border_tween: ShapeBorderTween,
    header_color_tween: Option<Handle<ColorTween>>,
    icon_color_tween: Option<Handle<ColorTween>>,
    background_color_tween: Option<Handle<ColorTween>>,
    icon_turns: Option<AnyAnimation<f64>>,
    border: Option<AnyAnimation<Option<Box<dyn ShapeBorder>>>>,
    header_color: Option<AnyAnimation<Option<Color>>>,
    icon_color: Option<AnyAnimation<Option<Color>>>,
    background_color: Option<AnyAnimation<Option<Color>>>,
    expansion_tile_theme: ExpansionTileThemeData,
    tile_controller: Option<Handle<ExpansibleController>>,
    curve: Rc<dyn Curve>,
    reverse_curve: Option<Rc<dyn Curve>>,
    duration: Duration,
}

impl ExpansionTileState {
    fn tile_controller(self: Handle<Self>, app: &App) -> Handle<ExpansibleController> {
        app.get(self)
            .tile_controller
            .expect("created in init_state")
    }

    fn ease_in_tween(self: Handle<Self>, app: &App) -> Handle<CurveTween> {
        app.get(self).ease_in_tween.expect("created in init_state")
    }

    fn ease_out_tween(self: Handle<Self>, app: &App) -> Handle<CurveTween> {
        app.get(self).ease_out_tween.expect("created in init_state")
    }

    fn on_expansion_changed(self: Handle<Self>, app: &mut App) {
        let is_expanded = self.tile_controller(app).is_expanded(app);
        if let Some(on_expansion_changed) = self.widget(app).on_expansion_changed.clone() {
            on_expansion_changed(app, is_expanded);
        }
    }

    // Platform or null affinity defaults to trailing.
    fn effective_affinity(self: Handle<Self>, app: &mut App) -> ListTileControlAffinity {
        let context = self.context(app);
        let list_tile_theme = ListTileTheme::of(app, context);
        let affinity = self
            .widget(app)
            .control_affinity
            .or(list_tile_theme.control_affinity)
            .unwrap_or(ListTileControlAffinity::Trailing);
        match affinity {
            ListTileControlAffinity::Leading => ListTileControlAffinity::Leading,
            ListTileControlAffinity::Trailing | ListTileControlAffinity::Platform => {
                ListTileControlAffinity::Trailing
            }
        }
    }

    fn build_icon(
        self: Handle<Self>,
        app: &mut App,
        _context: BuildContext,
        animation: AnyAnimation<f64>,
    ) -> Option<WidgetRef> {
        let half_tween = app.get(self).half_tween.expect("created in init_state");
        let ease_in_tween = self.ease_in_tween(app);
        let icon_turns = animation.drive(app, half_tween.chain(ease_in_tween));
        let replaced = [
            app.get_mut(self)
                .icon_turns
                .replace(icon_turns)
                .map(|old| old.id()),
            None,
        ];
        destroy_replaced_animations(app, replaced);
        Some(
            RotationTransition::new(icon_turns)
                .child(Icon::new(Some(Icons::expand_more())))
                .into_widget(),
        )
    }

    fn build_leading_icon(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
    ) -> Option<WidgetRef> {
        if self.effective_affinity(app) != ListTileControlAffinity::Leading {
            return None;
        }
        self.build_icon(app, context, animation)
    }

    fn build_trailing_icon(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
    ) -> Option<WidgetRef> {
        if self.effective_affinity(app) != ListTileControlAffinity::Trailing {
            return None;
        }
        self.build_icon(app, context, animation)
    }

    fn build_header(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        let ease_in_tween = self.ease_in_tween(app);
        let icon_color_tween = app
            .get(self)
            .icon_color_tween
            .expect("created in init_state");
        let header_color_tween = app
            .get(self)
            .header_color_tween
            .expect("created in init_state");
        let icon_color = animation.drive(app, icon_color_tween.chain(ease_in_tween));
        let header_color = animation.drive(app, header_color_tween.chain(ease_in_tween));
        let state = app.get_mut(self);
        let replaced = [
            state.icon_color.replace(icon_color).map(|old| old.id()),
            state.header_color.replace(header_color).map(|old| old.id()),
        ];
        destroy_replaced_animations(app, replaced);
        let tile_controller = self.tile_controller(app);
        let is_expanded = tile_controller.is_expanded(app);
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let on_tap_hint = if is_expanded {
            localizations.expansion_tile_expanded_tap_hint()
        } else {
            localizations.expansion_tile_collapsed_tap_hint()
        };
        let semantics_hint = match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                if is_expanded {
                    format!(
                        "{}\n {}",
                        localizations.collapsed_hint(),
                        localizations.expansion_tile_expanded_hint()
                    )
                } else {
                    format!(
                        "{}\n {}",
                        localizations.expanded_hint(),
                        localizations.expansion_tile_collapsed_hint()
                    )
                }
            }
            _ => {
                if is_expanded {
                    localizations.collapsed_hint()
                } else {
                    localizations.expanded_hint()
                }
            }
        };

        let widget = self.widget(app);
        let leading = widget.leading.clone();
        let title = widget.title.clone();
        let subtitle = widget.subtitle.clone();
        let show_trailing_icon = widget.show_trailing_icon;
        let trailing = widget.trailing.clone();
        let leading = match leading {
            Some(leading) => Some(leading),
            None => self.build_leading_icon(app, context, animation),
        };
        let trailing = if show_trailing_icon {
            match trailing {
                Some(trailing) => Some(trailing),
                None => self.build_trailing_icon(app, context, animation),
            }
        } else {
            None
        };

        let widget = self.widget(app);
        let expansion_tile_theme = &app.get(self).expansion_tile_theme;
        let on_tap = if is_expanded {
            Listener::handle_method(tile_controller, ExpansibleController::collapse)
        } else {
            Listener::handle_method(tile_controller, ExpansibleController::expand)
        };
        let mut tile = ListTile::new()
            .enabled(widget.enabled)
            .on_tap(on_tap)
            .title(title);
        tile.dense = widget.dense;
        tile.splash_color = widget.splash_color;
        tile.visual_density = widget.visual_density;
        tile.enable_feedback = widget.enable_feedback;
        tile.content_padding = widget.tile_padding.or(expansion_tile_theme.tile_padding);
        tile.leading = leading;
        tile.subtitle = subtitle;
        tile.trailing = trailing;
        tile.min_tile_height = widget.min_tile_height;
        tile.internal_add_semantic_for_on_tap = widget.internal_add_semantic_for_on_tap;
        tile.states_controller = widget.states_controller;

        let theme_icon_color = expansion_tile_theme.icon_color;
        let mut child = ListTileTheme::merge(tile);
        if let Some(icon_color) = icon_color.value(app).or(theme_icon_color) {
            child = child.icon_color(icon_color);
        }
        if let Some(text_color) = header_color.value(app) {
            child = child.text_color(text_color);
        }

        let header = Semantics::new()
            .hint(semantics_hint.clone())
            .on_tap_hint(on_tap_hint)
            .child(child);
        if app.platform().target_platform() == TargetPlatform::Android {
            // Live region used to announce state changes (e.g., "expanded" or "collapsed")
            // without taking focus.
            // The container ensures the live region forms the node that blockNode
            // removes from focus traversal.
            return Semantics::new()
                .container(true)
                .label(semantics_hint)
                .live_region(true)
                .accessibility_focus_block_type(AccessibilityFocusBlockType::BlockNode)
                .child(header)
                .into_widget();
        }
        header.into_widget()
    }

    fn build_body(
        self: Handle<Self>,
        app: &mut App,
        _context: BuildContext,
        _animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        let widget = self.widget(app);
        let expansion_tile_theme = &app.get(self).expansion_tile_theme;
        Align::new()
            .alignment(
                widget
                    .expanded_alignment
                    .or(expansion_tile_theme.expanded_alignment)
                    .unwrap_or(Alignment::CENTER.into()),
            )
            .child(
                Padding::new(
                    widget
                        .children_padding
                        .or(expansion_tile_theme.children_padding)
                        .unwrap_or(EdgeInsetsGeometry::ZERO),
                )
                .child(
                    Column::new()
                        .cross_axis_alignment(
                            widget
                                .expanded_cross_axis_alignment
                                .unwrap_or(CrossAxisAlignment::Center),
                        )
                        .children(widget.children.clone()),
                ),
            )
            .into_widget()
    }

    fn build_expansible(
        self: Handle<Self>,
        app: &mut App,
        _context: BuildContext,
        header: &WidgetRef,
        body: &WidgetRef,
        animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        let ease_out_tween = self.ease_out_tween(app);
        let background_color_tween = app
            .get(self)
            .background_color_tween
            .expect("created in init_state");
        let border_tween = app.get(self).border_tween.clone();
        let background_color = animation.drive(app, background_color_tween.chain(ease_out_tween));
        let border = animation.drive(app, border_tween.chain(ease_out_tween));
        let state = app.get_mut(self);
        let replaced = [
            state
                .background_color
                .replace(background_color)
                .map(|old| old.id()),
            state.border.replace(border).map(|old| old.id()),
        ];
        destroy_replaced_animations(app, replaced);
        let expansion_tile_theme = &app.get(self).expansion_tile_theme;
        let background_color = background_color
            .value(app)
            .or(expansion_tile_theme.background_color)
            .unwrap_or(Colors::TRANSPARENT);
        let expansion_tile_border = border
            .value(app)
            .unwrap_or_else(|| Box::new(transparent_border()));
        let widget = self.widget(app);
        let clip_behavior = widget
            .clip_behavior
            .or(expansion_tile_theme.clip_behavior)
            .unwrap_or(Clip::AntiAlias);

        let decoration =
            ShapeDecoration::new(expansion_tile_border.clone()).color(background_color);

        let mut tile = Padding::new(decoration.padding())
            .child(
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .children([header.clone(), body.clone()]),
            )
            .into_widget();

        let is_shape_provided = widget.shape.is_some()
            || expansion_tile_theme.shape.is_some()
            || widget.collapsed_shape.is_some()
            || expansion_tile_theme.collapsed_shape.is_some();

        if is_shape_provided {
            return Material::new()
                .clip_behavior(clip_behavior)
                .color(background_color)
                .shape(expansion_tile_border)
                .child(tile)
                .into_widget();
        }

        // If the background color is not transparent, wrap the tile in a Material widget.
        // This is needed to ensure that the ListTile background color or ink splashes
        // are visible. A DecoratedBox with a non-transparent color will hide the
        // background color or ink splashes of the ListTile.
        if background_color.a > 0.0 {
            tile = Material::new()
                .r#type(MaterialType::Transparency)
                .child(tile)
                .into_widget();
        }

        DecoratedBox::new(decoration).child(tile).into_widget()
    }

    fn update_animation_duration(self: Handle<Self>, app: &mut App) {
        let state = app.get(self);
        let duration = self
            .widget(app)
            .expansion_animation_style
            .as_ref()
            .and_then(|style| style.duration)
            .or_else(|| {
                state
                    .expansion_tile_theme
                    .expansion_animation_style
                    .as_ref()
                    .and_then(|style| style.duration)
            })
            .unwrap_or(Duration::from_millis(200));
        app.get_mut(self).duration = duration;
    }

    fn update_shape_border(self: Handle<Self>, app: &mut App, theme: &ThemeData) {
        let state = app.get(self);
        let widget = self.widget(app);
        let begin = widget
            .collapsed_shape
            .clone()
            .or_else(|| state.expansion_tile_theme.collapsed_shape.clone())
            .unwrap_or_else(|| Box::new(transparent_border()));
        let end = widget
            .shape
            .clone()
            .or_else(|| state.expansion_tile_theme.shape.clone())
            .unwrap_or_else(|| {
                Box::new(Border {
                    top: BorderSide {
                        color: theme.divider_color,
                        ..BorderSide::default()
                    },
                    bottom: BorderSide {
                        color: theme.divider_color,
                        ..BorderSide::default()
                    },
                    ..Border::default()
                })
            });
        let border_tween = &mut app.get_mut(self).border_tween;
        border_tween.begin = Some(begin);
        border_tween.end = Some(end);
    }

    fn update_header_color(self: Handle<Self>, app: &mut App, defaults: &ExpansionTileThemeData) {
        let state = app.get(self);
        let widget = self.widget(app);
        let begin = widget
            .collapsed_text_color
            .or(state.expansion_tile_theme.collapsed_text_color)
            .or(defaults.collapsed_text_color);
        let end = widget
            .text_color
            .or(state.expansion_tile_theme.text_color)
            .or(defaults.text_color);
        let header_color_tween = state.header_color_tween.expect("created in init_state");
        header_color_tween.set_begin(app, begin);
        header_color_tween.set_end(app, end);
    }

    fn update_icon_color(self: Handle<Self>, app: &mut App, defaults: &ExpansionTileThemeData) {
        let state = app.get(self);
        let widget = self.widget(app);
        let begin = widget
            .collapsed_icon_color
            .or(state.expansion_tile_theme.collapsed_icon_color)
            .or(defaults.collapsed_icon_color);
        let end = widget
            .icon_color
            .or(state.expansion_tile_theme.icon_color)
            .or(defaults.icon_color);
        let icon_color_tween = state.icon_color_tween.expect("created in init_state");
        icon_color_tween.set_begin(app, begin);
        icon_color_tween.set_end(app, end);
    }

    fn update_background_color(self: Handle<Self>, app: &mut App) {
        let state = app.get(self);
        let widget = self.widget(app);
        let begin = widget
            .collapsed_background_color
            .or(state.expansion_tile_theme.collapsed_background_color);
        let end = widget
            .background_color
            .or(state.expansion_tile_theme.background_color);
        let background_color_tween = state.background_color_tween.expect("created in init_state");
        background_color_tween.set_begin(app, begin);
        background_color_tween.set_end(app, end);
    }

    fn update_height_factor_curve(self: Handle<Self>, app: &mut App) {
        let state = app.get(self);
        let widget_style = self.widget(app).expansion_animation_style.as_ref();
        let theme_style = state
            .expansion_tile_theme
            .expansion_animation_style
            .as_ref();
        let curve = widget_style
            .and_then(|style| style.curve.clone())
            .or_else(|| theme_style.and_then(|style| style.curve.clone()))
            .unwrap_or_else(Curves::ease_in);
        let reverse_curve = widget_style
            .and_then(|style| style.reverse_curve.clone())
            .or_else(|| theme_style.and_then(|style| style.reverse_curve.clone()));
        let state = app.get_mut(self);
        state.curve = curve;
        state.reverse_curve = reverse_curve;
    }

    /// Dart's `theme.useMaterial3 ? _ExpansionTileDefaultsM3(context) :
    /// _ExpansionTileDefaultsM2(context)`.
    fn defaults(app: &mut App, context: BuildContext, theme: &ThemeData) -> ExpansionTileThemeData {
        if theme.use_material3 {
            expansion_tile_defaults_m3(app, context)
        } else {
            expansion_tile_defaults_m2(app, context)
        }
    }
}

/// Destroys the animations a builder's new `drive` replaced, which only that builder read.
///
/// Dart's collector frees them once the field is reassigned; here the state owns each one it
/// drove until it drives the next.
fn destroy_replaced_animations(app: &mut App, replaced: [Option<HandleId>; 2]) {
    for replaced in replaced.into_iter().flatten() {
        app.destroy(replaced);
    }
}

/// The `Border(top: BorderSide(color: Colors.transparent), bottom: BorderSide(color:
/// Colors.transparent))` Dart writes as a `const` where no shape is given.
fn transparent_border() -> Border {
    Border {
        top: BorderSide {
            color: Colors::TRANSPARENT,
            ..BorderSide::default()
        },
        bottom: BorderSide {
            color: Colors::TRANSPARENT,
            ..BorderSide::default()
        },
        ..Border::default()
    }
}

impl State for ExpansionTileState {
    type Widget = ExpansionTile;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let ease_in_tween = CurveTween::new(app, Curves::ease_in());
        let ease_out_tween = CurveTween::new(app, Curves::ease_out());
        let half_tween = Tween::new(app, Some(0.0), Some(0.5));
        let header_color_tween = ColorTween::new(app, None, None);
        let icon_color_tween = ColorTween::new(app, None, None);
        let background_color_tween = ColorTween::new(app, None, None);
        let tile_controller = match self.widget(app).controller {
            Some(controller) => controller,
            None => ExpansibleController::new(app),
        };
        let state = app.get_mut(self);
        state.ease_in_tween = Some(ease_in_tween);
        state.ease_out_tween = Some(ease_out_tween);
        state.half_tween = Some(half_tween);
        state.header_color_tween = Some(header_color_tween);
        state.icon_color_tween = Some(icon_color_tween);
        state.background_color_tween = Some(background_color_tween);
        state.curve = Curves::ease_in();
        state.duration = K_EXPAND;
        state.tile_controller = Some(tile_controller);
        if self.widget(app).initially_expanded {
            tile_controller.expand(app);
        }
        tile_controller.add_listener(
            app,
            Listener::handle_method(self, Self::on_expansion_changed),
        );
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let tile_controller = self.tile_controller(app);
        tile_controller.remove_listener(
            app,
            &Listener::handle_method(self, Self::on_expansion_changed),
        );
        if self.widget(app).controller.is_none() {
            tile_controller.dispose(app);
        }
        let state = app.get_mut(self);
        state.tile_controller = None;
        let owned = [
            state.icon_turns.take().map(|animation| animation.id()),
            state.border.take().map(|animation| animation.id()),
            state.header_color.take().map(|animation| animation.id()),
            state.icon_color.take().map(|animation| animation.id()),
            state
                .background_color
                .take()
                .map(|animation| animation.id()),
            state.ease_in_tween.take().map(|tween| tween.id()),
            state.ease_out_tween.take().map(|tween| tween.id()),
            state.half_tween.take().map(|tween| tween.id()),
            state.header_color_tween.take().map(|tween| tween.id()),
            state.icon_color_tween.take().map(|tween| tween.id()),
            state.background_color_tween.take().map(|tween| tween.id()),
        ];
        for owned in owned.into_iter().flatten() {
            app.destroy(owned);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ExpansionTile) {
        let context = self.context(app);
        let theme = Theme::of(app, context);
        let expansion_tile_theme = ExpansionTileTheme::of(app, context);
        app.get_mut(self).expansion_tile_theme = expansion_tile_theme;
        let defaults = ExpansionTileState::defaults(app, context, &theme);
        let widget = self.widget(app);
        if widget.collapsed_shape != old_widget.collapsed_shape || widget.shape != old_widget.shape
        {
            self.update_shape_border(app, &theme);
        }
        let widget = self.widget(app);
        if widget.collapsed_text_color != old_widget.collapsed_text_color
            || widget.text_color != old_widget.text_color
        {
            self.update_header_color(app, &defaults);
        }
        let widget = self.widget(app);
        if widget.collapsed_icon_color != old_widget.collapsed_icon_color
            || widget.icon_color != old_widget.icon_color
        {
            self.update_icon_color(app, &defaults);
        }
        let widget = self.widget(app);
        if widget.background_color != old_widget.background_color
            || widget.collapsed_background_color != old_widget.collapsed_background_color
        {
            self.update_background_color(app);
        }
        if self.widget(app).expansion_animation_style != old_widget.expansion_animation_style {
            self.update_animation_duration(app);
            self.update_height_factor_curve(app);
        }
        let controller = self.widget(app).controller;
        if controller != old_widget.controller {
            let tile_controller = self.tile_controller(app);
            tile_controller.remove_listener(
                app,
                &Listener::handle_method(self, Self::on_expansion_changed),
            );
            if old_widget.controller.is_none() {
                tile_controller.dispose(app);
            }

            let tile_controller = match controller {
                Some(controller) => controller,
                None => ExpansibleController::new(app),
            };
            app.get_mut(self).tile_controller = Some(tile_controller);
            tile_controller.add_listener(
                app,
                Listener::handle_method(self, Self::on_expansion_changed),
            );
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let theme = Theme::of(app, context);
        let expansion_tile_theme = ExpansionTileTheme::of(app, context);
        app.get_mut(self).expansion_tile_theme = expansion_tile_theme;
        let defaults = ExpansionTileState::defaults(app, context, &theme);
        self.update_animation_duration(app);
        self.update_shape_border(app, &theme);
        self.update_header_color(app, &defaults);
        self.update_icon_color(app, &defaults);
        self.update_background_color(app);
        self.update_height_factor_curve(app);
    }

    #[allow(deprecated)]
    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let state = app.get(self);
        let mut expansible = Expansible::new(
            Rc::new(move |app, context, animation| self.build_header(app, context, animation)),
            Rc::new(move |app, context, animation| self.build_body(app, context, animation)),
            self.tile_controller(app),
        )
        .curve(state.curve.clone())
        .duration(state.duration)
        .maintain_state(self.widget(app).maintain_state)
        .expansible_builder(Rc::new(move |app, context, header, body, animation| {
            self.build_expansible(app, context, header, body, animation)
        }));
        expansible.reverse_curve = state.reverse_curve.clone();
        expansible.into_widget()
    }
}

impl fmt::Debug for ExpansionTileState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpansionTileState")
            .field("tile_controller", &self.tile_controller)
            .field("duration", &self.duration)
            .finish_non_exhaustive()
    }
}

/// Dart's `_ExpansionTileDefaultsM2(context)`: the [`ExpansionTileThemeData`] whose fields
/// are the values of that subclass's getters.
fn expansion_tile_defaults_m2(app: &mut App, context: BuildContext) -> ExpansionTileThemeData {
    let theme = Theme::of(app, context);
    let color_scheme = &theme.color_scheme;

    let mut defaults = ExpansionTileThemeData::new()
        .text_color(color_scheme.primary)
        .icon_color(color_scheme.primary)
        .collapsed_icon_color(theme.unselected_widget_color);
    defaults.collapsed_text_color = theme
        .text_theme
        .title_medium
        .as_ref()
        .expect("the theme's text theme has a title medium style")
        .color
        .as_deref()
        .copied();
    defaults
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::{Cell, RefCell};

    use waft_embedder::{Offset, Radius, Rect, TargetPlatform, TextDirection};
    use waft_painting::{
        AlignmentDirectional, BorderRadius, BorderStyle, EdgeInsets, RoundedRectangleBorder,
        StadiumBorder, TextStyle,
    };
    use waft_rendering::RenderBox as _;
    use waft_scheduler::SchedulerBinding;
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey};
    use waft_widgets::{
        Animated, AnyElement, Builder, Center, ColoredBox, DefaultTextStyle, Directionality,
        GlobalKey, IconTheme, SingleChildScrollView, SizedBox, StatefulBuilder, Text,
    };

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::filled_button::FilledButton;
    use crate::list_tile_theme::ListTileThemeData;
    use crate::material::RenderInkFeatures;
    use crate::motion::Easing;
    use crate::scaffold::Scaffold;
    use crate::test_support::{
        AhemText, Drawn, WidgetTester, is_same_color_as, material_app, with_main_thread_stack,
    };
    use crate::text_theme::TextTheme;

    const DIVIDER_COLOR: Color = Color::new(0x1f333333);
    const UNSELECTED_WIDGET_COLOR: Color = Colors::BLACK54;
    const HEADER_COLOR: Color = Colors::BLACK45;

    fn foreground_color() -> Color {
        Colors::BLUE_ACCENT.shade200()
    }

    /// A tester on `platform` with the OS fonts, which `MaterialApp`'s debug banner paints with.
    pub(crate) fn tester_on(platform: TargetPlatform) -> WidgetTester {
        let tester = WidgetTester::on(platform);
        tester.install_fonts();
        tester
    }

    pub(crate) fn tester() -> WidgetTester {
        tester_on(TargetPlatform::Android)
    }

    /// Dart's `ThemeData()`.
    pub(crate) fn m3(app: &App) -> ThemeData {
        ThemeData::builder().build(app)
    }

    /// Dart's `ThemeData(useMaterial3: false)`.
    pub(crate) fn m2(app: &App) -> ThemeData {
        ThemeData::builder().use_material3(false).build(app)
    }

    /// Dart's `tester.pumpWidget(MaterialApp(theme: theme, home: home))`.
    pub(crate) fn pump_app<K>(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        home: impl IntoWidget<K>,
    ) {
        let theme = theme(&tester.cell.borrow());
        tester.pump_widget(material_app(theme, home.into_widget()));
    }

    /// The colour a [`test_icon`] or [`test_text`] was last built with.
    pub(crate) type Probe = Rc<Cell<Option<Color>>>;

    pub(crate) fn probe() -> Probe {
        Rc::new(Cell::new(None))
    }

    /// Dart's `TestIcon`: an `Icon(Icons.expand_more)` that keeps the `IconTheme` colour it was
    /// built with.
    pub(crate) fn test_icon(color: &Probe) -> WidgetRef {
        let color = Rc::clone(color);
        Builder::new(move |app, context| {
            color.set(IconTheme::of(app, context).color.as_deref().copied());
            Icon::new(Some(Icons::expand_more())).into_widget()
        })
        .into_widget()
    }

    /// Dart's `TestText(text)`: a `Text` that keeps the `DefaultTextStyle` colour it was built
    /// with.
    pub(crate) fn test_text(text: &'static str, color: &Probe) -> WidgetRef {
        let color = Rc::clone(color);
        Builder::new(move |app, context| {
            color.set(
                DefaultTextStyle::of(app, context)
                    .style
                    .color
                    .as_deref()
                    .copied(),
            );
            Text::new(text).into_widget()
        })
        .into_widget()
    }

    pub(crate) fn rect_of(tester: &WidgetTester, element: AnyElement) -> Rect {
        tester.global_rect(tester.render_box_of(element))
    }

    /// Dart's `tester.getRect(find.text(text))`.
    pub(crate) fn text_rect(tester: &WidgetTester, text: &str) -> Rect {
        let found = tester.find_text(text);
        assert_eq!(found.len(), 1, "one {text:?}");
        rect_of(tester, found[0])
    }

    /// Dart's `tester.getRect(find.byKey(key))`.
    pub(crate) fn key_rect(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        tester.global_rect(tester.render_object(key).as_box().expect("a render box"))
    }

    /// Dart's `tester.getSize(find.byKey(key)).height`.
    pub(crate) fn height(tester: &WidgetTester, key: &GlobalKey) -> f64 {
        key_rect(tester, key).height()
    }

    /// Dart's `tester.tap(find.text(text))`.
    pub(crate) fn tap_text(tester: &mut WidgetTester, text: &str) {
        let center = text_rect(tester, text).center();
        tester.tap(center, 1);
    }

    /// Dart's `tester.getRect(find.byType(ListTile))`.
    pub(crate) fn list_tile_rect(tester: &WidgetTester) -> Rect {
        let found = tester.find_by_type::<ListTile>();
        assert_eq!(found.len(), 1, "one ListTile");
        rect_of(tester, found[0])
    }

    /// Dart's `tester.getRect(find.byIcon(Icons.expand_more))`.
    pub(crate) fn expand_more_rect(tester: &WidgetTester) -> Rect {
        rect_of(tester, tester.find_by_icon(&Icons::expand_more()))
    }

    /// Dart's `tester.getRect(find.byType(Column).last)`.
    fn last_column_rect(tester: &WidgetTester) -> Rect {
        let last = *tester.find_by_type::<Column>().last().expect("a Column");
        rect_of(tester, last)
    }

    /// Dart's `tester.getRect(find.byType(Padding).last)`.
    pub(crate) fn last_padding_rect(tester: &WidgetTester) -> Rect {
        let last = *tester.find_by_type::<Padding>().last().expect("a Padding");
        rect_of(tester, last)
    }

    fn element_of(tester: &WidgetTester, key: &GlobalKey) -> AnyElement {
        key.current_context(&mut tester.cell.borrow_mut())
            .expect("the keyed widget is mounted")
    }

    /// The colour of Dart's `tester.firstWidget<DecoratedBox>(find.descendant(of:
    /// find.byKey(key), matching: find.byType(DecoratedBox))).decoration as ShapeDecoration`,
    /// with the colours of its `Border` shape's top and bottom sides.
    fn decoration_of(tester: &WidgetTester, key: &GlobalKey) -> (Option<Color>, Color, Color) {
        let first = tester.descendants_of_type::<DecoratedBox>(element_of(tester, key))[0];
        tester.widget(first, |decorated_box: &DecoratedBox| {
            let decoration = decorated_box
                .decoration
                .as_any()
                .downcast_ref::<ShapeDecoration>()
                .expect("a ShapeDecoration");
            let border = decoration
                .shape
                .as_any()
                .downcast_ref::<Border>()
                .expect("a Border");
            (
                decoration.color.as_deref().copied(),
                border.top.color,
                border.bottom.color,
            )
        })
    }

    /// Dart's `getMaterial(tester)`: the first `Material` below the `ExpansionTile`, as its
    /// shape, colour and clip behaviour.
    pub(crate) fn tile_material(
        tester: &WidgetTester,
    ) -> (Option<Box<dyn ShapeBorder>>, Option<Color>, Clip) {
        let tile = tester.find_by_type::<ExpansionTile>()[0];
        let material = tester.descendants_of_type::<Material>(tile)[0];
        tester.widget(material, |material: &Material| {
            (
                material.shape.clone(),
                material.color,
                material.clip_behavior,
            )
        })
    }

    fn the_list_tile<R>(tester: &WidgetTester, read: impl FnOnce(&ListTile) -> R) -> R {
        let found = tester.find_by_type::<ListTile>();
        assert_eq!(found.len(), 1, "one ListTile");
        tester.widget(found[0], read)
    }

    fn is_widget<W: 'static>(widget: &Option<WidgetRef>) -> bool {
        widget
            .as_ref()
            .is_some_and(|widget| waft_widgets::downcast_widget::<W>(&**widget).is_some())
    }

    pub(crate) fn sized(width: f64, height: f64) -> WidgetRef {
        SizedBox::new().width(width).height(height).into_widget()
    }

    fn keyed_sized(key: &GlobalKey, width: f64, height: f64) -> WidgetRef {
        SizedBox::new()
            .key(key.clone().into())
            .width(width)
            .height(height)
            .into_widget()
    }

    fn list_tile(key: Option<&GlobalKey>, title: &str) -> WidgetRef {
        let tile = ListTile::new().title(Text::new(title));
        match key {
            Some(key) => tile.key(key.clone().into()),
            None => tile,
        }
        .into_widget()
    }

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{actual} is not within {tolerance} of {expected}"
        );
    }

    /// Whether `drawn` has draws matching `matchers` in that order: Dart's `paints` matcher
    /// with one call per matcher.
    fn paints_in_order(drawn: &[Drawn], matchers: &[&dyn Fn(&Drawn) -> bool]) -> bool {
        let mut rest = drawn.iter();
        matchers.iter().all(|matcher| rest.by_ref().any(matcher))
    }

    fn initial_state_case(platform: TargetPlatform) {
        let mut tester = tester_on(platform);
        let top_key = GlobalKey::new();
        let expanded_key = GlobalKey::new();
        let collapsed_key = GlobalKey::new();
        let default_key = GlobalKey::new();
        let expanded_tile_key = GlobalKey::new();
        let collapsed_tile_key = GlobalKey::new();
        let default_tile_key = GlobalKey::new();

        pump_app(
            &mut tester,
            |app| ThemeData::builder().divider_color(DIVIDER_COLOR).build(app),
            Material::new().child(
                SingleChildScrollView::new().child(
                    Column::new().children([
                        list_tile(Some(&top_key), "Top"),
                        ExpansionTile::new(Text::new("Expanded"))
                            .key(expanded_key.clone().into())
                            .initially_expanded(true)
                            .background_color(Colors::RED.primary())
                            .children([list_tile(Some(&expanded_tile_key), "0")])
                            .into_widget(),
                        ExpansionTile::new(Text::new("Collapsed"))
                            .key(collapsed_key.clone().into())
                            .children([list_tile(Some(&collapsed_tile_key), "0")])
                            .into_widget(),
                        ExpansionTile::new(Text::new("Default"))
                            .key(default_key.clone().into())
                            .children([list_tile(Some(&default_tile_key), "0")])
                            .into_widget(),
                    ]),
                ),
            ),
        );

        let tile_height = height(&tester, &expanded_tile_key);
        assert_eq!(
            height(&tester, &top_key),
            height(&tester, &expanded_key) - tile_height - 2.0
        );
        assert_eq!(
            height(&tester, &top_key),
            height(&tester, &collapsed_key) - 2.0
        );
        assert_eq!(
            height(&tester, &top_key),
            height(&tester, &default_key) - 2.0
        );

        let (color, top, bottom) = decoration_of(&tester, &expanded_key);
        assert_eq!(color, Some(Colors::RED.primary()));
        assert_eq!(top, DIVIDER_COLOR);
        assert_eq!(bottom, DIVIDER_COLOR);

        let (color, top, bottom) = decoration_of(&tester, &collapsed_key);
        assert_eq!(color, Some(Colors::TRANSPARENT));
        assert_eq!(top, Colors::TRANSPARENT);
        assert_eq!(bottom, Colors::TRANSPARENT);

        tap_text(&mut tester, "Expanded");
        tap_text(&mut tester, "Collapsed");
        tap_text(&mut tester, "Default");

        tester.pump(Duration::ZERO);

        // Pump to the middle of the animation for expansion.
        tester.pump(Duration::from_millis(100));
        let (color, top, bottom) = decoration_of(&tester, &collapsed_key);
        assert_eq!(color, Some(Colors::TRANSPARENT));
        assert!(
            is_same_color_as(top, Color::new(0x15222222)),
            "top side {top:?}"
        );
        assert!(
            is_same_color_as(bottom, Color::new(0x15222222)),
            "bottom side {bottom:?}"
        );

        // Pump all the way to the end now.
        tester.pump(Duration::from_secs(1));

        let tile_height = height(&tester, &collapsed_tile_key);
        assert_eq!(
            height(&tester, &top_key),
            height(&tester, &expanded_key) - 2.0
        );
        assert_eq!(
            height(&tester, &top_key),
            height(&tester, &collapsed_key) - tile_height - 2.0
        );
        assert_eq!(
            height(&tester, &top_key),
            height(&tester, &default_key) - tile_height - 2.0
        );

        // Expanded should be collapsed now.
        let (color, top, bottom) = decoration_of(&tester, &expanded_key);
        assert_eq!(color, Some(Colors::TRANSPARENT));
        assert_eq!(top, Colors::TRANSPARENT);
        assert_eq!(bottom, Colors::TRANSPARENT);

        // Collapsed should be expanded now.
        let (color, top, bottom) = decoration_of(&tester, &collapsed_key);
        assert_eq!(color, Some(Colors::TRANSPARENT));
        assert_eq!(top, DIVIDER_COLOR);
        assert_eq!(bottom, DIVIDER_COLOR);
    }

    /// "ExpansionTile initial state", on iOS.
    #[test]
    fn expansion_tile_initial_state_on_ios() {
        with_main_thread_stack(|| initial_state_case(TargetPlatform::IOS));
    }

    /// "ExpansionTile initial state", on macOS.
    #[test]
    fn expansion_tile_initial_state_on_macos() {
        with_main_thread_stack(|| initial_state_case(TargetPlatform::MacOS));
    }

    fn theme_dependencies_case(platform: TargetPlatform) {
        let mut tester = tester_on(platform);
        let expanded_title = probe();
        let collapsed_title = probe();
        let expanded_icon = probe();
        let collapsed_icon = probe();

        pump_app(
            &mut tester,
            |app| {
                ThemeData::builder()
                    .use_material3(false)
                    .color_scheme(
                        ColorScheme::from_swatch()
                            .build()
                            .copy_with()
                            .with_primary(foreground_color()),
                    )
                    .unselected_widget_color(UNSELECTED_WIDGET_COLOR)
                    .text_theme(TextTheme::new().title_medium(TextStyle::new().color(HEADER_COLOR)))
                    .build(app)
            },
            Material::new().child(
                SingleChildScrollView::new().child(
                    Column::new().children([
                        list_tile(None, "Top"),
                        ExpansionTile::new(test_text("Expanded", &expanded_title))
                            .initially_expanded(true)
                            .background_color(Colors::RED.primary())
                            .trailing(test_icon(&expanded_icon))
                            .children([list_tile(None, "0")])
                            .into_widget(),
                        ExpansionTile::new(test_text("Collapsed", &collapsed_title))
                            .trailing(test_icon(&collapsed_icon))
                            .children([list_tile(None, "0")])
                            .into_widget(),
                    ]),
                ),
            ),
        );

        assert_eq!(expanded_title.get(), Some(foreground_color()));
        assert_eq!(collapsed_title.get(), Some(HEADER_COLOR));
        assert_eq!(expanded_icon.get(), Some(foreground_color()));
        assert_eq!(collapsed_icon.get(), Some(UNSELECTED_WIDGET_COLOR));

        // Tap both tiles to change their state: collapse and extend respectively
        tap_text(&mut tester, "Expanded");
        tap_text(&mut tester, "Collapsed");
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_secs(1));
        tester.pump(Duration::from_secs(1));

        assert_eq!(expanded_title.get(), Some(HEADER_COLOR));
        assert_eq!(collapsed_title.get(), Some(foreground_color()));
        assert_eq!(expanded_icon.get(), Some(UNSELECTED_WIDGET_COLOR));
        assert_eq!(collapsed_icon.get(), Some(foreground_color()));
    }

    /// "ExpansionTile Theme dependencies", on iOS.
    #[test]
    fn expansion_tile_theme_dependencies_on_ios() {
        with_main_thread_stack(|| theme_dependencies_case(TargetPlatform::IOS));
    }

    /// "ExpansionTile Theme dependencies", on macOS.
    #[test]
    fn expansion_tile_theme_dependencies_on_macos() {
        with_main_thread_stack(|| theme_dependencies_case(TargetPlatform::MacOS));
    }

    /// "ExpansionTile subtitle".
    #[test]
    fn expansion_tile_subtitle() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                m3,
                Scaffold::new().body(
                    ExpansionTile::new(Text::new("Title"))
                        .subtitle(Text::new("Subtitle"))
                        .children([list_tile(None, "0")]),
                ),
            );

            assert_eq!(tester.find_onstage_text("Subtitle").len(), 1);
        });
    }

    /// "ExpansionTile maintainState".
    #[test]
    fn expansion_tile_maintain_state() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .platform(TargetPlatform::IOS)
                        .divider_color(DIVIDER_COLOR)
                        .build(app)
                },
                Material::new().child(
                    SingleChildScrollView::new().child(
                        Column::new().children([
                            ExpansionTile::new(Text::new("Tile 1"))
                                .maintain_state(true)
                                .children([Text::new("Maintaining State").into_widget()])
                                .into_widget(),
                            ExpansionTile::new(Text::new("Title 2"))
                                .children([Text::new("Discarding State").into_widget()])
                                .into_widget(),
                        ]),
                    ),
                ),
            );

            // This text should be offstage while ExpansionTile collapsed
            assert_eq!(tester.find_text("Maintaining State").len(), 1);
            assert!(tester.find_onstage_text("Maintaining State").is_empty());
            // This text shouldn't be there while ExpansionTile collapsed
            assert!(tester.find_onstage_text("Discarding State").is_empty());
        });
    }

    /// "ExpansionTile padding test".
    #[test]
    fn expansion_tile_padding_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    Center::new().child(
                        ExpansionTile::new(Text::new("Hello"))
                            .tile_padding(EdgeInsets::from_ltrb(8.0, 12.0, 4.0, 10.0)),
                    ),
                ),
            );

            let title_rect = text_rect(&tester, "Hello");
            let trailing_rect = expand_more_rect(&tester);
            let list_tile_rect = list_tile_rect(&tester);
            let taller_widget = if title_rect.height() > trailing_rect.height() {
                title_rect
            } else {
                trailing_rect
            };

            // Check the positions of title and trailing Widgets, after padding is applied.
            assert_eq!(list_tile_rect.left, title_rect.left - 8.0);
            assert_eq!(list_tile_rect.right, trailing_rect.right + 4.0);

            // Calculate the remaining height of ListTile from the default height.
            let remaining_height = 56.0 - taller_widget.height();
            assert_eq!(
                list_tile_rect.top,
                taller_widget.top - remaining_height / 2.0 - 12.0
            );
            assert_eq!(
                list_tile_rect.bottom,
                taller_widget.bottom + remaining_height / 2.0 + 10.0
            );
        });
    }

    /// `MaterialApp(home: [Directionality(textDirection:)] Material(child: Center(child: tile)))`,
    /// then a tap on "title" and a settle.
    fn pump_expanded_centered(
        tester: &mut WidgetTester,
        text_direction: Option<TextDirection>,
        tile: ExpansionTile,
    ) {
        let material = Material::new()
            .child(Center::new().child(tile))
            .into_widget();
        let home = match text_direction {
            Some(text_direction) => Directionality::new(text_direction, material).into_widget(),
            None => material,
        };
        pump_app(tester, m3, home);
        tap_text(tester, "title");
        tester.pump_and_settle();
    }

    /// "ExpansionTile expandedAlignment test".
    #[test]
    fn expansion_tile_expanded_alignment_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_expanded_centered(
                &mut tester,
                None,
                ExpansionTile::new(Text::new("title"))
                    .expanded_alignment(Alignment::CENTER_LEFT)
                    .children([sized(100.0, 100.0), sized(80.0, 100.0)]),
            );

            let column_rect = last_column_rect(&tester);

            // The expandedAlignment is used to define the alignment of the Column widget in
            // expanded tile, not the alignment of the children inside the Column.
            assert_eq!(column_rect.left, 0.0);
            // The width of the Column is the width of the largest child. The largest width
            // being 100.0, the offset of the right edge of Column from X-axis should be 100.0.
            assert_eq!(column_rect.right, 100.0);
        });
    }

    /// "ExpansionTile expandedAlignment with directional test".
    #[test]
    fn expansion_tile_expanded_alignment_with_directional_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_expanded_centered(
                &mut tester,
                Some(TextDirection::Rtl),
                ExpansionTile::new(Text::new("title"))
                    .expanded_alignment(AlignmentDirectional::TOP_END)
                    .children([sized(100.0, 100.0), sized(80.0, 100.0)]),
            );

            let column_rect = last_column_rect(&tester);

            // The expandedAlignment is used to define the alignment of the Column widget in
            // expanded tile, not the alignment of the children inside the Column.
            assert_eq!(column_rect.left, 0.0);
            // The width of the Column is the width of the largest child. The largest width
            // being 100.0, the offset of the right edge of Column from X-axis should be 100.0.
            assert_eq!(column_rect.right, 100.0);
        });
    }

    /// "ExpansionTile expandedCrossAxisAlignment test".
    #[test]
    fn expansion_tile_expanded_cross_axis_alignment_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let child0_key = GlobalKey::new();
            let child1_key = GlobalKey::new();
            pump_expanded_centered(
                &mut tester,
                None,
                ExpansionTile::new(Text::new("title"))
                    // Set the column's alignment to Alignment.centerRight to test
                    // CrossAxisAlignment of children widgets. This helps distinguish the
                    // effect of expandedAlignment and expandedCrossAxisAlignment later in the
                    // test.
                    .expanded_alignment(Alignment::CENTER_RIGHT)
                    .expanded_cross_axis_alignment(CrossAxisAlignment::Start)
                    .children([
                        keyed_sized(&child0_key, 100.0, 100.0),
                        keyed_sized(&child1_key, 80.0, 100.0),
                    ]),
            );

            let column_rect = last_column_rect(&tester);
            let child0_rect = key_rect(&tester, &child0_key);
            let child1_rect = key_rect(&tester, &child1_key);

            // Since expandedAlignment is set to Alignment.centerRight, the column of children
            // should be aligned to the center right of the expanded tile. This provides
            // confirmation that the expandedCrossAxisAlignment.start is 700.0, where
            // columnRect.left is.
            assert_eq!(column_rect.right, 800.0);
            // The width of the Column is the width of the largest child. The largest width
            // being 100.0, the offset of the left edge of Column from X-axis should be 700.0.
            assert_eq!(column_rect.left, 700.0);

            // Considering the value of expandedCrossAxisAlignment is CrossAxisAlignment.start,
            // the offset of the left edge of both the children from X-axis should be 700.0.
            assert_eq!(child0_rect.left, 700.0);
            assert_eq!(child1_rect.left, 700.0);
        });
    }

    /// "ExpansionTile expandedCrossAxisAlignment with directional expandedAlignment test".
    #[test]
    fn expansion_tile_expanded_cross_axis_alignment_with_directional_expanded_alignment_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let child0_key = GlobalKey::new();
            let child1_key = GlobalKey::new();
            pump_expanded_centered(
                &mut tester,
                Some(TextDirection::Rtl),
                ExpansionTile::new(Text::new("title"))
                    // Set the column's alignment to AlignmentDirectional.centerStart to test
                    // CrossAxisAlignment of children widgets. This helps distinguish the
                    // effect of expandedAlignment and expandedCrossAxisAlignment later in the
                    // test.
                    .expanded_alignment(AlignmentDirectional::CENTER_START)
                    .expanded_cross_axis_alignment(CrossAxisAlignment::End)
                    .children([
                        keyed_sized(&child0_key, 100.0, 100.0),
                        keyed_sized(&child1_key, 80.0, 100.0),
                    ]),
            );

            let column_rect = last_column_rect(&tester);
            let child0_rect = key_rect(&tester, &child0_key);
            let child1_rect = key_rect(&tester, &child1_key);

            // With `textDirection` set to `TextDirection.rtl`,
            // `AlignmentDirectional.centerStart` resolves to `Alignment.centerRight`. The column
            // of children should be aligned to the center right of the expanded tile.
            assert_eq!(column_rect.right, 800.0);
            // The width of the Column is the width of the largest child. The largest width
            // being 100.0, the offset of the left edge of Column from X-axis should be 700.0.
            assert_eq!(column_rect.left, 700.0);

            // With `textDirection` set to `TextDirection.rtl`, `CrossAxisAlignment.end` aligns
            // children to the left. The offset of the left edge of both children from the
            // X-axis should be 700.0.
            assert_eq!(child0_rect.left, 700.0);
            assert_eq!(child1_rect.left, 700.0);
        });
    }

    /// "CrossAxisAlignment.baseline is not allowed".
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(
        expected = "CrossAxisAlignment.baseline is not supported since the expanded children \
                    are aligned in a column, not a row. Try to use another constant."
    )]
    fn cross_axis_alignment_baseline_is_not_allowed() {
        let _ = ExpansionTile::new(Text::new("title"))
            .initially_expanded(true)
            .expanded_cross_axis_alignment(CrossAxisAlignment::Baseline);
    }

    /// "expandedCrossAxisAlignment and expandedAlignment default values".
    #[test]
    fn expanded_cross_axis_alignment_and_expanded_alignment_default_values() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let child1_key = GlobalKey::new();
            pump_expanded_centered(
                &mut tester,
                None,
                ExpansionTile::new(Text::new("title"))
                    .children([sized(100.0, 100.0), keyed_sized(&child1_key, 80.0, 100.0)]),
            );

            let column_rect = last_column_rect(&tester);
            let child1_rect = key_rect(&tester, &child1_key);

            // The default viewport size is Size(800, 600).
            // By default the value of extendedAlignment is Alignment.center, hence the offset
            // of left and right edges from x axis should be equal.
            assert_eq!(column_rect.left, 800.0 - column_rect.right);

            // By default the value of extendedCrossAxisAlignment is CrossAxisAlignment.center,
            // hence the offset of left and right edges from Column should be equal.
            assert_eq!(
                child1_rect.left - column_rect.left,
                column_rect.right - child1_rect.right
            );
        });
    }

    /// "childrenPadding default value".
    #[test]
    fn children_padding_default_value() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_expanded_centered(
                &mut tester,
                None,
                ExpansionTile::new(Text::new("title")).children([sized(100.0, 100.0)]),
            );

            let column_rect = last_column_rect(&tester);
            let padding_rect = last_padding_rect(&tester);

            // By default, the value of childrenPadding is EdgeInsets.zero, hence offset
            // of all the edges from x-axis and y-axis should be equal for Padding and Column.
            assert_eq!(column_rect.top, padding_rect.top);
            assert_eq!(column_rect.left, padding_rect.left);
            assert_eq!(column_rect.right, padding_rect.right);
            assert_eq!(column_rect.bottom, padding_rect.bottom);
        });
    }

    /// "ExpansionTile childrenPadding test".
    #[test]
    fn expansion_tile_children_padding_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_expanded_centered(
                &mut tester,
                None,
                ExpansionTile::new(Text::new("title"))
                    .children_padding(EdgeInsets::from_ltrb(10.0, 8.0, 12.0, 4.0))
                    .children([sized(100.0, 100.0)]),
            );

            let column_rect = last_column_rect(&tester);
            let padding_rect = last_padding_rect(&tester);

            // Check the offset of all the edges from x-axis and y-axis after childrenPadding
            // is applied.
            assert_eq!(column_rect.left, padding_rect.left + 10.0);
            assert_eq!(column_rect.top, padding_rect.top + 8.0);
            assert_eq!(column_rect.right, padding_rect.right - 12.0);
            assert_eq!(column_rect.bottom, padding_rect.bottom - 4.0);
        });
    }

    /// "ExpansionTile.collapsedBackgroundColor".
    #[test]
    fn expansion_tile_collapsed_background_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let expansion_tile_key = GlobalKey::new();
            let background_color = Colors::RED.primary();
            let collapsed_background_color = Colors::BROWN.primary();

            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(Text::new("Title"))
                        .key(expansion_tile_key.clone().into())
                        .background_color(background_color)
                        .collapsed_background_color(collapsed_background_color)
                        .children([sized(100.0, 100.0)]),
                ),
            );

            let (color, _, _) = decoration_of(&tester, &expansion_tile_key);
            assert_eq!(color, Some(collapsed_background_color));

            tap_text(&mut tester, "Title");
            tester.pump_and_settle();

            let (color, _, _) = decoration_of(&tester, &expansion_tile_key);
            assert_eq!(color, Some(background_color));
        });
    }

    /// "ExpansionTile default iconColor, textColor".
    #[test]
    fn expansion_tile_default_icon_color_text_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let icon_color = probe();
            let text_color = probe();

            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(test_text("title", &text_color))
                        .trailing(test_icon(&icon_color))
                        .children([sized(100.0, 100.0)]),
                ),
            );

            let theme = m3(&tester.cell.borrow());
            assert_eq!(
                icon_color.get(),
                Some(theme.color_scheme.on_surface_variant())
            );
            assert_eq!(text_color.get(), Some(theme.color_scheme.on_surface));

            tap_text(&mut tester, "title");
            tester.pump_and_settle();

            assert_eq!(icon_color.get(), Some(theme.color_scheme.primary));
            assert_eq!(text_color.get(), Some(theme.color_scheme.on_surface));
        });
    }

    /// "ExpansionTile iconColor, textColor".
    #[test]
    fn expansion_tile_icon_color_text_color() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/pull/78281
            let icon_color = Color::new(0xff00ff00);
            let collapsed_icon_color = Color::new(0xff0000ff);
            let text_color = Color::new(0xff00ffff);
            let collapsed_text_color = Color::new(0xffff00ff);
            let mut tester = tester();
            let icon = probe();
            let text = probe();

            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(test_text("title", &text))
                        .icon_color(icon_color)
                        .collapsed_icon_color(collapsed_icon_color)
                        .text_color(text_color)
                        .collapsed_text_color(collapsed_text_color)
                        .trailing(test_icon(&icon))
                        .children([sized(100.0, 100.0)]),
                ),
            );

            assert_eq!(icon.get(), Some(collapsed_icon_color));
            assert_eq!(text.get(), Some(collapsed_text_color));

            tap_text(&mut tester, "title");
            tester.pump_and_settle();

            assert_eq!(icon.get(), Some(icon_color));
            assert_eq!(text.get(), Some(text_color));
        });
    }

    /// What the first `Material` below the `ExpansionTile` and its subtree draw.
    fn tile_material_painted(tester: &WidgetTester) -> Vec<Drawn> {
        let tile = tester.find_by_type::<ExpansionTile>()[0];
        let material = tester.descendants_of_type::<Material>(tile)[0];
        tester.painted(tester.render_box_of(material).as_object())
    }

    /// "ExpansionTile shows hover color with opaque backgroundColor".
    #[test]
    fn expansion_tile_shows_hover_color_with_opaque_background_color() {
        with_main_thread_stack(|| {
            let hover_color = Colors::GREEN.primary();
            let background_color = Colors::BLUE.primary();
            let mut tester = tester();

            pump_app(
                &mut tester,
                |app| ThemeData::builder().hover_color(hover_color).build(app),
                Material::new().child(
                    ExpansionTile::new(Text::new("Title"))
                        .collapsed_background_color(background_color),
                ),
            );

            // Hover the tile.
            let mut gesture = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.pump(Duration::ZERO);
            let center = text_rect(&tester, "Title").center();
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();

            // Dart's `paints..rect(color: Colors.transparent)..rect(color: hoverColor)`: a draw
            // with a transparent paint is not recorded at all, so the tile's transparent ink
            // decoration has no entry and the hover highlight is the first draw.
            let drawn = tile_material_painted(&tester);
            assert!(
                matches!(drawn.first(), Some(Drawn::Rect(color, _)) if *color == hover_color.to_argb32()),
                "{drawn:?}"
            );
        });
    }

    /// "ExpansionTile shows focus color with opaque backgroundColor".
    #[test]
    fn expansion_tile_shows_focus_color_with_opaque_background_color() {
        with_main_thread_stack(|| {
            let focus_color = Colors::PINK.primary();
            let background_color = Colors::AMBER.primary();
            let mut tester = tester();

            pump_app(
                &mut tester,
                |app| ThemeData::builder().focus_color(focus_color).build(app),
                Material::new().child(
                    ExpansionTile::new(Text::new("Title"))
                        .collapsed_background_color(background_color),
                ),
            );

            // Focus the tile.
            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump_and_settle();

            // Dart's `paints..rect(color: Colors.transparent)..rect(color: focusColor)`: a draw
            // with a transparent paint is not recorded at all, so the tile's transparent ink
            // decoration has no entry and the focus highlight is the first draw.
            let drawn = tile_material_painted(&tester);
            assert!(
                matches!(drawn.first(), Some(Drawn::Rect(color, _)) if *color == focus_color.to_argb32()),
                "{drawn:?}"
            );
        });
    }

    fn boxed(shape: impl ShapeBorder + 'static) -> Option<Box<dyn ShapeBorder>> {
        Some(Box::new(shape))
    }

    /// "ExpansionTile Border".
    #[test]
    fn expansion_tile_border() {
        with_main_thread_stack(|| {
            let collapsed_shape = Border {
                top: BorderSide {
                    color: Colors::BLUE.primary(),
                    ..BorderSide::default()
                },
                bottom: BorderSide {
                    color: Colors::GREEN.primary(),
                    ..BorderSide::default()
                },
                ..Border::default()
            };
            let shape = Border::all(
                Colors::RED.primary(),
                1.0,
                BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_INSIDE,
            );
            let mut tester = tester();

            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(Text::new("ExpansionTile"))
                        .collapsed_shape(collapsed_shape)
                        .shape(shape)
                        .children([list_tile(None, "0")]),
                ),
            );

            // When a custom shape is provided, ExpansionTile will use the
            // Material widget to draw the shape and background color
            // instead of a Container.
            let (material_shape, _, clip_behavior) = tile_material(&tester);
            // ExpansionTile should be collapsed initially.
            assert_eq!(material_shape, boxed(collapsed_shape));
            assert_eq!(clip_behavior, Clip::AntiAlias);

            tap_text(&mut tester, "ExpansionTile");
            tester.pump_and_settle();

            // ExpansionTile should be Expanded now.
            let (material_shape, _, clip_behavior) = tile_material(&tester);
            assert_eq!(material_shape, boxed(shape));
            assert_eq!(clip_behavior, Clip::AntiAlias);
        });
    }

    fn pump_affinity(tester: &mut WidgetTester, tile: ExpansionTile) {
        pump_app(tester, m3, Material::new().child(tile));
    }

    /// "ExpansionTile platform controlAffinity test".
    #[test]
    fn expansion_tile_platform_control_affinity_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_affinity(&mut tester, ExpansionTile::new(Text::new("Title")));

            the_list_tile(&tester, |list_tile| {
                assert!(list_tile.leading.is_none());
                assert!(is_widget::<Animated<RotationTransition>>(
                    &list_tile.trailing
                ));
            });
        });
    }

    /// "ExpansionTile trailing controlAffinity test".
    #[test]
    fn expansion_tile_trailing_control_affinity_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_affinity(
                &mut tester,
                ExpansionTile::new(Text::new("Title"))
                    .control_affinity(ListTileControlAffinity::Trailing),
            );

            the_list_tile(&tester, |list_tile| {
                assert!(list_tile.leading.is_none());
                assert!(is_widget::<Animated<RotationTransition>>(
                    &list_tile.trailing
                ));
            });
        });
    }

    /// "ExpansionTile leading controlAffinity test".
    #[test]
    fn expansion_tile_leading_control_affinity_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_affinity(
                &mut tester,
                ExpansionTile::new(Text::new("Title"))
                    .control_affinity(ListTileControlAffinity::Leading),
            );

            the_list_tile(&tester, |list_tile| {
                assert!(is_widget::<Animated<RotationTransition>>(
                    &list_tile.leading
                ));
                assert!(list_tile.trailing.is_none());
            });
        });
    }

    /// "ExpansionTile override rotating icon test".
    #[test]
    fn expansion_tile_override_rotating_icon_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_affinity(
                &mut tester,
                ExpansionTile::new(Text::new("Title"))
                    .leading(Icon::new(Some(Icons::info())))
                    .control_affinity(ListTileControlAffinity::Leading),
            );

            the_list_tile(&tester, |list_tile| {
                assert!(is_widget::<Icon>(&list_tile.leading));
                assert!(list_tile.trailing.is_none());
            });
        });
    }

    /// The values a case's `StatefulBuilder` builds its tile with, and a button sets.
    struct TileValues {
        shape: Box<dyn ShapeBorder>,
        text_color: Color,
        background_color: Color,
        icon_color: Color,
    }

    fn rounded(radius: f64) -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadius::all(Radius::circular(radius)).into(),
        )
    }

    /// The `StatefulBuilder` of the "properties can be updated with setState" cases: a
    /// `Column` of the tile `tile` builds from the values, and a button that sets `update`.
    fn updatable_tile(
        values: Rc<RefCell<TileValues>>,
        update: fn() -> TileValues,
        tile: fn(&TileValues, ExpansionTile) -> ExpansionTile,
        icon: &Probe,
        text: &Probe,
    ) -> WidgetRef {
        let icon = Rc::clone(icon);
        let text = Rc::clone(text);
        StatefulBuilder::new(move |_app, _context, set_state| {
            let on_pressed = {
                let values = Rc::clone(&values);
                Listener::new(move |app| {
                    set_state(app, &mut |_app| *values.borrow_mut() = update());
                })
            };
            Column::new()
                .children([
                    tile(
                        &values.borrow(),
                        ExpansionTile::new(test_text("title", &text))
                            .trailing(test_icon(&icon))
                            .children([sized(100.0, 100.0)]),
                    )
                    .into_widget(),
                    // This button is used to update the ExpansionTile properties.
                    FilledButton::new(
                        Some(on_pressed),
                        Some(Text::new("Update collapsed properties")),
                    )
                    .into_widget(),
                ])
                .into_widget()
        })
        .into_widget()
    }

    /// "Collapsed ExpansionTile properties can be updated with setState".
    #[test]
    fn collapsed_expansion_tile_properties_can_be_updated_with_set_state() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let icon = probe();
            let text = probe();
            let values = Rc::new(RefCell::new(TileValues {
                shape: Box::new(rounded(4.0)),
                text_color: Color::new(0xffffffff),
                background_color: Color::new(0xffff0000),
                icon_color: Color::new(0xffffffff),
            }));

            pump_app(
                &mut tester,
                m3,
                Material::new().child(updatable_tile(
                    values,
                    || TileValues {
                        shape: Box::new(rounded(16.0)),
                        text_color: Color::new(0xff000000),
                        background_color: Color::new(0xffffff00),
                        icon_color: Color::new(0xff000000),
                    },
                    |values, tile| {
                        tile.collapsed_shape(values.shape.clone())
                            .collapsed_text_color(values.text_color)
                            .collapsed_background_color(values.background_color)
                            .collapsed_icon_color(values.icon_color)
                    },
                    &icon,
                    &text,
                )),
            );

            // When a custom shape is provided, ExpansionTile will use the
            // Material widget to draw the shape and background color
            // instead of a Container.
            let (shape, color, clip_behavior) = tile_material(&tester);

            // Test initial ExpansionTile properties.
            assert_eq!(shape, boxed(rounded(4.0)));
            assert_eq!(color, Some(Color::new(0xffff0000)));
            assert_eq!(clip_behavior, Clip::AntiAlias);
            assert_eq!(icon.get(), Some(Color::new(0xffffffff)));
            assert_eq!(text.get(), Some(Color::new(0xffffffff)));

            // Tap the button to update the ExpansionTile properties.
            tap_text(&mut tester, "Update collapsed properties");
            tester.pump_and_settle();

            let (shape, color, clip_behavior) = tile_material(&tester);

            // Test updated ExpansionTile properties.
            assert_eq!(shape, boxed(rounded(16.0)));
            assert_eq!(color, Some(Color::new(0xffffff00)));
            assert_eq!(clip_behavior, Clip::AntiAlias);
            assert_eq!(icon.get(), Some(Color::new(0xff000000)));
            assert_eq!(text.get(), Some(Color::new(0xff000000)));
        });
    }

    /// "Expanded ExpansionTile properties can be updated with setState".
    #[test]
    fn expanded_expansion_tile_properties_can_be_updated_with_set_state() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let icon = probe();
            let text = probe();
            let values = Rc::new(RefCell::new(TileValues {
                shape: Box::new(rounded(12.0)),
                text_color: Color::new(0xff00ffff),
                background_color: Color::new(0xff0000ff),
                icon_color: Color::new(0xff00ffff),
            }));

            pump_app(
                &mut tester,
                m3,
                Material::new().child(updatable_tile(
                    values,
                    || TileValues {
                        shape: Box::new(rounded(6.0)),
                        text_color: Color::new(0xffffffff),
                        background_color: Color::new(0xff123456),
                        icon_color: Color::new(0xffffffff),
                    },
                    |values, tile| {
                        tile.shape(values.shape.clone())
                            .text_color(values.text_color)
                            .background_color(values.background_color)
                            .icon_color(values.icon_color)
                    },
                    &icon,
                    &text,
                )),
            );

            // Tap to expand the ExpansionTile.
            tap_text(&mut tester, "title");
            tester.pump_and_settle();

            // When a custom shape is provided, ExpansionTile will use the
            // Material widget to draw the shape and background color
            // instead of a Container.
            let (shape, color, clip_behavior) = tile_material(&tester);

            // Test initial ExpansionTile properties.
            assert_eq!(shape, boxed(rounded(12.0)));
            assert_eq!(color, Some(Color::new(0xff0000ff)));
            assert_eq!(clip_behavior, Clip::AntiAlias);
            assert_eq!(icon.get(), Some(Color::new(0xff00ffff)));
            assert_eq!(text.get(), Some(Color::new(0xff00ffff)));

            // Tap the button to update the ExpansionTile properties.
            tap_text(&mut tester, "Update collapsed properties");
            tester.pump_and_settle();

            let (shape, color, clip_behavior) = tile_material(&tester);

            // Test updated ExpansionTile properties.
            assert_eq!(shape, boxed(rounded(6.0)));
            assert_eq!(color, Some(Color::new(0xff123456)));
            assert_eq!(clip_behavior, Clip::AntiAlias);
            assert_eq!(icon.get(), Some(Color::new(0xffffffff)));
            assert_eq!(text.get(), Some(Color::new(0xffffffff)));
        });
    }

    /// The steps the "Override ExpansionTile animation" cases share: `pump(style)` pumps the
    /// tile with the animation style, given where each case gives it.
    pub(crate) fn animation_style_case(
        pump: impl Fn(&mut WidgetTester, &GlobalKey, Option<AnimationStyle>),
        reverse_curve: bool,
    ) {
        let mut tester = tester();
        let expansion_tile_key = GlobalKey::new();
        let height = |tester: &WidgetTester| height(tester, &expansion_tile_key);

        pump(&mut tester, &expansion_tile_key, None);

        // Test initial ExpansionTile height.
        assert_eq!(height(&tester), 58.0);

        // Test the default expansion animation.
        tap_text(&mut tester, "title");
        tester.pump(Duration::ZERO);
        // Advance the animation by 1/4 of its duration.
        tester.pump(Duration::from_millis(50));

        assert_close(height(&tester), 67.4, 0.1);

        // Advance the animation by 2/4 of its duration.
        tester.pump(Duration::from_millis(50));

        assert_close(height(&tester), 89.6, 0.1);

        // Advance the animation to the end.
        tester.pump_and_settle();

        assert_eq!(height(&tester), 158.0);

        // Tap to collapse the ExpansionTile.
        tap_text(&mut tester, "title");
        tester.pump_and_settle();

        // Override the animation duration.
        pump(
            &mut tester,
            &expansion_tile_key,
            Some(AnimationStyle::new(
                None,
                Some(Duration::from_millis(800)),
                None,
                None,
            )),
        );
        tester.pump_and_settle();

        // Test the overridden animation duration.
        tap_text(&mut tester, "title");
        tester.pump(Duration::ZERO);
        // Advance the animation by 1/4 of its duration.
        tester.pump(Duration::from_millis(200));

        assert_close(height(&tester), 67.4, 0.1);

        // Advance the animation by 2/4 of its duration.
        tester.pump(Duration::from_millis(200));

        assert_close(height(&tester), 89.6, 0.1);

        // Advance the animation to the end.
        tester.pump_and_settle();

        assert_eq!(height(&tester), 158.0);

        // Tap to collapse the ExpansionTile.
        tap_text(&mut tester, "title");
        tester.pump_and_settle();

        // Override the animation curve.
        pump(
            &mut tester,
            &expansion_tile_key,
            Some(AnimationStyle::new(
                Some(Easing::emphasized_decelerate()),
                None,
                reverse_curve.then(Easing::emphasized_accelerate),
                None,
            )),
        );
        tester.pump_and_settle();

        // Test the overridden animation curve.
        tap_text(&mut tester, "title");
        tester.pump(Duration::ZERO);
        // Advance the animation by 1/4 of its duration.
        tester.pump(Duration::from_millis(50));

        assert_close(height(&tester), 141.2, 0.1);

        // Advance the animation by 2/4 of its duration.
        tester.pump(Duration::from_millis(50));

        assert_close(height(&tester), 153.0, 0.1);

        // Advance the animation to the end.
        tester.pump_and_settle();

        assert_eq!(height(&tester), 158.0);

        if reverse_curve {
            // Test the overridden reverse (collapse) animation curve.
            tap_text(&mut tester, "title");
            tester.pump(Duration::ZERO);
            // Advance the animation by 1/4 of its duration.
            tester.pump(Duration::from_millis(50));

            assert_close(height(&tester), 98.6, 0.1);

            // Advance the animation by 2/4 of its duration.
            tester.pump(Duration::from_millis(50));

            assert_close(height(&tester), 73.4, 0.1);

            // Advance the animation to the end.
            tester.pump_and_settle();

            assert_eq!(height(&tester), 58.0);
        } else {
            // Tap to collapse the ExpansionTile.
            tap_text(&mut tester, "title");
        }

        // Test no animation.
        pump(
            &mut tester,
            &expansion_tile_key,
            Some(AnimationStyle::NO_ANIMATION),
        );

        // Tap to expand the ExpansionTile.
        tap_text(&mut tester, "title");
        tester.pump(Duration::ZERO);

        assert_eq!(height(&tester), 158.0);
    }

    /// "Override ExpansionTile animation using AnimationStyle".
    #[test]
    fn override_expansion_tile_animation_using_animation_style() {
        with_main_thread_stack(|| {
            animation_style_case(
                |tester, key, animation_style| {
                    let mut tile = ExpansionTile::new(test_text("title", &probe()))
                        .key(key.clone().into())
                        .children([sized(100.0, 100.0)]);
                    tile.expansion_animation_style = animation_style;
                    pump_app(tester, m3, Material::new().child(Center::new().child(tile)));
                },
                true,
            );
        });
    }

    /// "Default clipBehavior when a shape is provided".
    #[test]
    fn default_clip_behavior_when_a_shape_is_provided() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                m3,
                Scaffold::new().body(
                    ExpansionTile::new(Text::new("Title"))
                        .subtitle(Text::new("Subtitle"))
                        .shape(StadiumBorder::default())
                        .children([list_tile(None, "0")]),
                ),
            );

            assert_eq!(tile_material(&tester).2, Clip::AntiAlias);
        });
    }

    /// "Can override clipBehavior when a shape is provided".
    #[test]
    fn can_override_clip_behavior_when_a_shape_is_provided() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                m3,
                Scaffold::new().body(
                    ExpansionTile::new(Text::new("Title"))
                        .subtitle(Text::new("Subtitle"))
                        .shape(StadiumBorder::default())
                        .clip_behavior(Clip::None)
                        .children([list_tile(None, "0")]),
                ),
            );

            assert_eq!(tile_material(&tester).2, Clip::None);
        });
    }

    /// Material 2: "ExpansionTile default iconColor, textColor".
    #[test]
    fn material2_expansion_tile_default_icon_color_text_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let icon_color = probe();
            let text_color = probe();

            pump_app(
                &mut tester,
                m2,
                Material::new().child(
                    ExpansionTile::new(test_text("title", &text_color))
                        .trailing(test_icon(&icon_color))
                        .children([sized(100.0, 100.0)]),
                ),
            );

            let theme = m2(&tester.cell.borrow());
            assert_eq!(icon_color.get(), Some(theme.unselected_widget_color));
            assert_eq!(
                text_color.get(),
                theme
                    .text_theme
                    .title_medium
                    .as_ref()
                    .and_then(|style| style.color.as_deref().copied())
            );

            tap_text(&mut tester, "title");
            tester.pump_and_settle();

            assert_eq!(icon_color.get(), Some(theme.color_scheme.primary));
            assert_eq!(text_color.get(), Some(theme.color_scheme.primary));
        });
    }

    /// Material 2: "Material2 - ExpansionTile draws inkwell splash on top of background color".
    #[test]
    fn material2_expansion_tile_draws_inkwell_splash_on_top_of_background_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let expansion_tile_key = GlobalKey::new();
            let collapsed_background_color = Color::new(0xff00ff00);
            let background_color = Color::new(0xffff0000);

            pump_app(
                &mut tester,
                m2,
                Material::new().child(
                    Center::new().child(
                        Padding::new(EdgeInsets::symmetric(0.0, 24.0).into()).child(
                            ExpansionTile::new(test_text("title", &probe()))
                                .key(expansion_tile_key.clone().into())
                                .shape(rounded(16.0))
                                .collapsed_background_color(collapsed_background_color)
                                .background_color(background_color)
                                .collapsed_shape(rounded(16.0))
                                .trailing(test_icon(&probe()))
                                .children([sized(100.0, 100.0)]),
                        ),
                    ),
                ),
            );

            // Tap and hold the ExpansionTile to trigger ink splash.
            let center = key_rect(&tester, &expansion_tile_key).center();
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, center);
            // Start the splash animation.
            tester.pump(Duration::ZERO);
            // Splash is underway.
            tester.pump(Duration::from_millis(100));

            let ink_features = tester.first::<RenderInkFeatures>();
            let drawn = tester.painted(ink_features.as_object());
            let splash_color = m2(&tester.cell.borrow()).splash_color;
            // Check if the ink splash is drawn on top of the background color.
            assert!(
                paints_in_order(
                    &drawn,
                    &[
                        &|draw| matches!(draw, Drawn::Path(color, _) if *color == collapsed_background_color.to_argb32()),
                        &|draw| matches!(draw, Drawn::Path(color, _) if *color == splash_color.to_argb32()),
                    ]
                ),
                "{drawn:?}"
            );

            // Finish gesture to release resources.
            tester.up(&gesture);
            tester.pump_and_settle();
        });
    }

    /// "ExpansionTileController isExpanded, expand() and collapse()".
    #[test]
    fn expansion_tile_controller_is_expanded_expand_and_collapse() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller = ExpansibleController::new(&mut tester.cell.borrow_mut());

            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(Text::new("Title"))
                        .controller(controller)
                        .children([Text::new("Child 0").into_widget()]),
                ),
            );

            let is_expanded = |tester: &WidgetTester| controller.is_expanded(&tester.cell.borrow());
            assert!(tester.find_onstage_text("Child 0").is_empty());
            assert!(!is_expanded(&tester));
            controller.expand(&mut tester.cell.borrow_mut());
            assert!(is_expanded(&tester));
            tester.pump_and_settle();
            assert_eq!(tester.find_onstage_text("Child 0").len(), 1);
            assert!(is_expanded(&tester));
            controller.collapse(&mut tester.cell.borrow_mut());
            assert!(!is_expanded(&tester));
            tester.pump_and_settle();
            assert!(tester.find_onstage_text("Child 0").is_empty());

            controller.dispose(&mut tester.cell.borrow_mut());
        });
    }

    /// Dart's `tester.hasRunningAnimations`.
    fn has_running_animations(tester: &WidgetTester) -> bool {
        SchedulerBinding::transient_callback_count(&mut tester.cell.borrow_mut()) > 0
    }

    /// "Calling ExpansionTileController.expand/collapsed has no effect if it is already
    /// expanded/collapsed".
    #[test]
    fn calling_expansion_tile_controller_expand_collapsed_has_no_effect_if_it_is_already_expanded_collapsed()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller = ExpansibleController::new(&mut tester.cell.borrow_mut());

            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(Text::new("Title"))
                        .controller(controller)
                        .initially_expanded(true)
                        .children([Text::new("Child 0").into_widget()]),
                ),
            );

            let is_expanded = |tester: &WidgetTester| controller.is_expanded(&tester.cell.borrow());
            assert_eq!(tester.find_onstage_text("Child 0").len(), 1);
            assert!(is_expanded(&tester));
            controller.expand(&mut tester.cell.borrow_mut());
            assert!(is_expanded(&tester));
            tester.pump(Duration::ZERO);
            assert!(!has_running_animations(&tester));
            assert_eq!(tester.find_onstage_text("Child 0").len(), 1);
            controller.collapse(&mut tester.cell.borrow_mut());
            assert!(!is_expanded(&tester));
            tester.pump(Duration::ZERO);
            assert!(has_running_animations(&tester));
            tester.pump_and_settle();
            assert!(!is_expanded(&tester));
            assert!(tester.find_onstage_text("Child 0").is_empty());
            controller.collapse(&mut tester.cell.borrow_mut());
            assert!(!is_expanded(&tester));
            tester.pump(Duration::ZERO);
            assert!(!has_running_animations(&tester));

            controller.dispose(&mut tester.cell.borrow_mut());
        });
    }

    /// "Call to ExpansionTileController.of()".
    #[test]
    fn call_to_expansion_tile_controller_of() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let title_key = GlobalKey::new();
            let child_key = GlobalKey::new();
            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(Text::new("Title").key(title_key.clone().into()))
                        .initially_expanded(true)
                        .children([Text::new("Child 0")
                            .key(child_key.clone().into())
                            .into_widget()]),
                ),
            );

            let child = element_of(&tester, &child_key);
            let title = element_of(&tester, &title_key);
            let app = tester.cell.borrow();
            let controller1 = ExpansibleController::of(&app, child);
            assert!(controller1.is_expanded(&app));

            let controller2 = ExpansibleController::of(&app, title);
            assert!(controller2.is_expanded(&app));

            assert_eq!(controller1, controller2);
        });
    }

    /// "Call to ExpansionTile.maybeOf()".
    #[test]
    fn call_to_expansion_tile_maybe_of() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let title_key = GlobalKey::new();
            let non_descendant_key = GlobalKey::new();
            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    Column::new().children([
                        ExpansionTile::new(Text::new("Title").key(title_key.clone().into()))
                            .children([Text::new("Child 0").into_widget()])
                            .into_widget(),
                        Text::new("Non descendant")
                            .key(non_descendant_key.clone().into())
                            .into_widget(),
                    ]),
                ),
            );

            let title = element_of(&tester, &title_key);
            let non_descendant = element_of(&tester, &non_descendant_key);
            let app = tester.cell.borrow();
            let controller1 = ExpansibleController::maybe_of(&app, title);
            assert!(controller1.is_some());
            assert_eq!(
                controller1.map(|controller| controller.is_expanded(&app)),
                Some(false)
            );

            let controller2 = ExpansibleController::maybe_of(&app, non_descendant);
            assert!(controller2.is_none());
        });
    }

    /// "Check if dense, splashColor, enableFeedback, visualDensity parameter is working".
    #[test]
    fn check_if_dense_splash_color_enable_feedback_visual_density_parameter_is_working() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dense = true;
            let splash_color = Colors::BLUE.primary();
            let enable_feedback = false;
            let visual_density = VisualDensity::COMPACT;

            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    Column::new().children([
                        ExpansionTile::new(Text::new("Title"))
                            .dense(dense)
                            .splash_color(splash_color)
                            .enable_feedback(enable_feedback)
                            .visual_density(visual_density)
                            .children([Text::new("Child 0").into_widget()])
                            .into_widget(),
                        Text::new("Non descendant").into_widget(),
                    ]),
                ),
            );

            the_list_tile(&tester, |tile| {
                assert_eq!(tile.dense, Some(dense));
                assert_eq!(tile.splash_color, Some(splash_color));
                assert_eq!(tile.enable_feedback, Some(enable_feedback));
                assert_eq!(tile.visual_density, Some(visual_density));
            });
        });
    }

    /// Dart's `tester.tap(find.widgetWithText(ExpansionTile, text))`.
    fn tap_tile(tester: &mut WidgetTester) {
        let tile = tester.find_by_type::<ExpansionTile>()[0];
        let center = rect_of(tester, tile).center();
        tester.tap(center, 1);
    }

    /// "ExpansionTileController should not toggle if disabled".
    #[test]
    fn expansion_tile_controller_should_not_toggle_if_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller = ExpansibleController::new(&mut tester.cell.borrow_mut());

            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(Text::new("Title"))
                        .enabled(false)
                        .controller(controller)
                        .children([Text::new("Child 0").into_widget()]),
                ),
            );

            let is_expanded = |tester: &WidgetTester| controller.is_expanded(&tester.cell.borrow());
            assert!(tester.find_onstage_text("Child 0").is_empty());
            assert!(!is_expanded(&tester));
            tap_tile(&mut tester);
            tester.pump_and_settle();
            assert!(tester.find_onstage_text("Child 0").is_empty());
            assert!(!is_expanded(&tester));
            controller.expand(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();
            assert_eq!(tester.find_onstage_text("Child 0").len(), 1);
            assert!(is_expanded(&tester));
            tap_tile(&mut tester);
            tester.pump_and_settle();
            assert_eq!(tester.find_onstage_text("Child 0").len(), 1);
            assert!(is_expanded(&tester));

            controller.dispose(&mut tester.cell.borrow_mut());
        });
    }

    /// Dart's `tester.getSize(find.descendant(of: find.byType(ExpansionTile), matching:
    /// find.byType(ColoredBox))).width`.
    fn title_width(tester: &WidgetTester) -> f64 {
        let tile = tester.find_by_type::<ExpansionTile>()[0];
        let colored_box = tester.descendants_of_type::<ColoredBox>(tile);
        assert_eq!(colored_box.len(), 1, "one ColoredBox");
        rect_of(tester, colored_box[0]).width()
    }

    /// "ExpansionTile does not include the default trailing icon when showTrailingIcon: false
    /// (#145268)".
    #[test]
    fn expansion_tile_does_not_include_the_default_trailing_icon_when_show_trailing_icon_false() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(
                        ColoredBox::new(Colors::RED.primary()).child(Text::new("Title")),
                    )
                    .enabled(false)
                    .tile_padding(EdgeInsetsGeometry::ZERO)
                    .show_trailing_icon(false),
                ),
            );

            assert_eq!(title_width(&tester), WidgetTester::SIZE.width());
        });
    }

    /// "ExpansionTile with smaller trailing widget allocates at least 32.0 units of space
    /// (preserves original behavior) (#145268)".
    #[test]
    fn expansion_tile_with_smaller_trailing_widget_allocates_at_least_32_units_of_space() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(
                        ColoredBox::new(Colors::RED.primary()).child(Text::new("Title")),
                    )
                    .enabled(false)
                    .tile_padding(EdgeInsetsGeometry::ZERO)
                    .trailing(SizedBox::shrink()),
                ),
            );

            assert_eq!(title_width(&tester), WidgetTester::SIZE.width() - 32.0);
        });
    }

    /// "ExpansionTile uses ListTileTheme controlAffinity".
    #[test]
    fn expansion_tile_uses_list_tile_theme_control_affinity() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_view = |tester: &mut WidgetTester, control_affinity| {
                pump_app(
                    tester,
                    m3,
                    ListTileTheme::new(
                        Material::new().child(ExpansionTile::new(AhemText::new("ExpansionTile"))),
                    )
                    .data(ListTileThemeData::new().control_affinity(control_affinity)),
                );
            };
            let top_left = |tester: &WidgetTester| {
                let found = tester.find_ahem_text("ExpansionTile");
                assert_eq!(found.len(), 1, "one ExpansionTile title");
                rect_of(tester, found[0]).top_left()
            };

            build_view(&mut tester, ListTileControlAffinity::Leading);
            assert_eq!(top_left(&tester), Offset::new(56.0, 17.0));

            build_view(&mut tester, ListTileControlAffinity::Trailing);
            assert_eq!(top_left(&tester), Offset::new(16.0, 17.0));

            build_view(&mut tester, ListTileControlAffinity::Platform);
            assert_eq!(top_left(&tester), Offset::new(16.0, 17.0));
        });
    }

    /// "ExpansionTile can accept a new controller".
    #[test]
    fn expansion_tile_can_accept_a_new_controller() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller1 = ExpansibleController::new(&mut tester.cell.borrow_mut());
            let controller2 = ExpansibleController::new(&mut tester.cell.borrow_mut());
            let tile = |controller| {
                Material::new().child(
                    ExpansionTile::new(Text::new("Title"))
                        .controller(controller)
                        .initially_expanded(true)
                        .children([Text::new("Child 0").into_widget()]),
                )
            };

            pump_app(&mut tester, m3, tile(controller1));

            assert_eq!(tester.find_onstage_text("Child 0").len(), 1);
            assert!(controller1.is_expanded(&tester.cell.borrow()));
            controller1.collapse(&mut tester.cell.borrow_mut());
            assert!(!controller1.is_expanded(&tester.cell.borrow()));
            tester.pump_and_settle();
            assert!(tester.find_onstage_text("Child 0").is_empty());

            pump_app(&mut tester, m3, tile(controller2));

            assert!(tester.find_onstage_text("Child 0").is_empty());
            controller2.expand(&mut tester.cell.borrow_mut());
            assert!(controller2.is_expanded(&tester.cell.borrow()));
            tester.pump_and_settle();
            assert_eq!(tester.find_onstage_text("Child 0").len(), 1);

            controller1.dispose(&mut tester.cell.borrow_mut());
            controller2.dispose(&mut tester.cell.borrow_mut());
        });
    }

    /// "ExpansionTile can accept a new controller with a different state".
    #[test]
    fn expansion_tile_can_accept_a_new_controller_with_a_different_state() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller1 = ExpansibleController::new(&mut tester.cell.borrow_mut());
            let controller2 = ExpansibleController::new(&mut tester.cell.borrow_mut());
            let tile = |controller| {
                Material::new().child(
                    ExpansionTile::new(Text::new("Title"))
                        .controller(controller)
                        .children([Text::new("Child 0").into_widget()]),
                )
            };

            pump_app(&mut tester, m3, tile(controller1));

            assert!(tester.find_onstage_text("Child 0").is_empty());
            assert!(!controller1.is_expanded(&tester.cell.borrow()));
            controller1.expand(&mut tester.cell.borrow_mut());
            assert!(controller1.is_expanded(&tester.cell.borrow()));
            tester.pump_and_settle();
            assert_eq!(tester.find_onstage_text("Child 0").len(), 1);

            pump_app(&mut tester, m3, tile(controller2));

            tester.pump_and_settle();

            assert!(
                tester.find_onstage_text("Child 0").is_empty(),
                "The widget should update to the state of the new controller"
            );
            controller2.expand(&mut tester.cell.borrow_mut());
            assert!(controller2.is_expanded(&tester.cell.borrow()));
            tester.pump_and_settle();
            assert_eq!(tester.find_onstage_text("Child 0").len(), 1);

            controller1.dispose(&mut tester.cell.borrow_mut());
            controller2.dispose(&mut tester.cell.borrow_mut());
        });
    }

    /// "ExpansionTile forwards statesController to ListTile".
    #[test]
    fn expansion_tile_forwards_states_controller_to_list_tile() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller = WidgetStatesController::new(&mut tester.cell.borrow_mut(), None);

            pump_app(
                &mut tester,
                m3,
                Material::new()
                    .child(ExpansionTile::new(Text::new("Tile")).states_controller(controller)),
            );

            the_list_tile(&tester, |list_tile| {
                assert_eq!(list_tile.states_controller, Some(controller));
            });
        });
    }

    /// "ExpansionTile forwards null statesController to ListTile".
    #[test]
    fn expansion_tile_forwards_null_states_controller_to_list_tile() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                m3,
                Material::new().child(ExpansionTile::new(Text::new("Tile"))),
            );

            the_list_tile(&tester, |list_tile| {
                assert_eq!(list_tile.states_controller, None);
            });
        });
    }

    /// The `onExpansionChanged` the docs describe: called with true when the tile starts
    /// expanding and false when it starts collapsing.
    #[test]
    fn on_expansion_changed_is_called_when_the_tile_starts_expanding_and_collapsing() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let changes = Rc::new(RefCell::new(Vec::new()));
            let on_expansion_changed: ValueChanged<bool> = {
                let changes = Rc::clone(&changes);
                Rc::new(move |_app, is_expanded| changes.borrow_mut().push(is_expanded))
            };

            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(Text::new("Title"))
                        .on_expansion_changed(on_expansion_changed)
                        .children([Text::new("Child 0").into_widget()]),
                ),
            );

            tap_text(&mut tester, "Title");
            assert_eq!(*changes.borrow(), [true]);
            tester.pump_and_settle();
            tap_text(&mut tester, "Title");
            assert_eq!(*changes.borrow(), [true, false]);
            tester.pump_and_settle();
        });
    }

    /// "ExpansionTile supports being embedded in a WidgetSpan".
    #[test]
    fn expansion_tile_supports_being_embedded_in_a_widget_span() {
        with_main_thread_stack(|| {
            use waft_painting::TextSpan;
            use waft_widgets::WidgetSpan;

            let mut tester = tester();
            let tile = ExpansionTile::new(Text::new("Header"))
                .children([Text::new("Details").into_widget()]);
            pump_app(
                &mut tester,
                m3,
                Scaffold::new().body(Text::rich(
                    TextSpan::new()
                        .children(vec![WidgetSpan::new(tile).into_span()])
                        .into_span(),
                )),
            );
            assert_eq!(tester.find_text("Header").len(), 1);
        });
    }

    /// Not a Dart case: a tile that made its own controller and is then given one disposes its
    /// own while the `Expansible` below still compares it with the new one, as Dart's does.
    #[test]
    fn a_tile_without_a_controller_that_is_given_one_follows_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller = ExpansibleController::new(&mut tester.cell.borrow_mut());
            let tile = || {
                ExpansionTile::new(Text::new("Title"))
                    .children([Text::new("Child 0").into_widget()])
            };

            pump_app(&mut tester, m3, Material::new().child(tile()));
            assert!(tester.find_onstage_text("Child 0").is_empty());

            pump_app(
                &mut tester,
                m3,
                Material::new().child(tile().controller(controller)),
            );
            assert!(tester.find_onstage_text("Child 0").is_empty());

            controller.toggle(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();
            assert!(controller.is_expanded(&tester.cell.borrow()));
            assert_eq!(tester.find_onstage_text("Child 0").len(), 1);

            controller.toggle(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();
            assert!(tester.find_onstage_text("Child 0").is_empty());

            pump_app(&mut tester, m3, SizedBox::new());
            controller.dispose(&mut tester.cell.borrow_mut());
        });
    }

    /// The animations a tile drove for its header and its decoration.
    fn driven_animations(tester: &WidgetTester) -> [HandleId; 5] {
        let tile = tester.find_by_type::<ExpansionTile>()[0];
        let app = tester.cell.borrow();
        let state = app.get(
            tile.state_handle::<ExpansionTileState>(&app)
                .expect("an ExpansionTile's element holds its state"),
        );
        [
            state.header_color.expect("built").id(),
            state.icon_color.expect("built").id(),
            state.background_color.expect("built").id(),
            state.border.expect("built").id(),
            state.icon_turns.expect("built").id(),
        ]
    }

    /// Not a Dart case: Dart's collector frees the animations each frame's header, icon and
    /// decoration drive; here the tile destroys each one when it drives the next, and the last
    /// ones when it is disposed.
    #[test]
    fn a_tile_keeps_only_the_animations_of_its_latest_frame() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(
                &mut tester,
                m3,
                Material::new().child(
                    ExpansionTile::new(Text::new("Title"))
                        .children([Text::new("Child 0").into_widget()]),
                ),
            );
            let first = driven_animations(&tester);

            tap_text(&mut tester, "Title");
            tester.pump(Duration::from_millis(50));
            let mid_expansion = driven_animations(&tester);
            {
                let app = tester.cell.borrow();
                assert!(first.iter().all(|&animation| !app.contains(animation)));
                assert!(
                    mid_expansion
                        .iter()
                        .all(|&animation| app.contains(animation))
                );
            }

            tester.pump_and_settle();
            let last = driven_animations(&tester);
            pump_app(&mut tester, m3, SizedBox::new());
            let app = tester.cell.borrow();
            assert!(
                mid_expansion
                    .iter()
                    .chain(&last)
                    .all(|&animation| !app.contains(animation))
            );
        });
    }
}
