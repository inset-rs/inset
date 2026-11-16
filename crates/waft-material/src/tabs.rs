//! Flutter counterpart: material_ui's `tabs.dart` (and `generated/tabs_defaults_m3.g.dart`).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::fmt;
use std::rc::{Rc, Weak};
use std::time::Duration;

use waft_animation::{
    Animation, AnimationRef, AnimationStatus, AnimationStatusListener, AnimationWithParent,
    AnyAnimation, ColorTween, CurvedAnimation, Curves, ReverseAnimation,
    k_always_dismissed_animation,
};
use waft_embedder::{
    Brightness, Canvas, Clip, Color, Offset, Paint, PaintStyle, Rect, SemanticsRole, Size, Stroke,
    TextBaseline, TextDirection, clamp_double, lerp_double,
};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, K_IS_WEB, Listenable, ListenableObject,
    Listener, MergingListenable, OptionRetained, Ref, Task, ValueChanged,
};
use waft_gestures::DragStartBehavior;
use waft_painting::{
    AlignmentGeometry, AnyColor, Axis, BorderRadius, BorderStyle, BoxPainter, BoxShape,
    DecorationRef, EdgeInsetsDirectional, EdgeInsetsGeometry, ImageConfiguration, TextOverflow,
    TextScaler, TextStyle, VerticalDirection, draw_line,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, ContainerBoxParentData,
    ContainerRenderObjectData, ContainerRenderObjectMixin, CrossAxisAlignment, CustomPainter,
    FlexParentData, MainAxisAlignment, MainAxisSize, PaintingContext, PipelineOwner, RenderBox,
    RenderBoxContainerDefaultsMixin, RenderBoxData, RenderFlexData, RenderFlexMixin, RenderHandle,
    RenderObject, RenderObjectData, ViewportOffset,
};
use waft_scheduler::{FrameCallback, SchedulerBinding};
use waft_services::MouseCursorRef;
use waft_widgets::{
    Align, AnimatedBuilder, AnimatedWidget, AnyScrollPosition, BoxDecoration, BuildContext, Center,
    ClampingScrollPhysics, Column, Container, CustomPaint, DefaultTextStyle, Directionality,
    Expanded, Flex, GlobalKey, IconTheme, IconThemeData, IntoWidget, KeyRef, KeyedSubtree,
    LimitedBox, MaybeWidgetStateProperty, MediaQuery, MergeSemantics, MultiChildRenderObjectWidget,
    NotificationListener, Padding, PageController, PageScrollPhysics, PageView,
    PreferredSizeWidget, RenderObjectWidget, Row, ScrollActivityDelegate, ScrollConfiguration,
    ScrollContext, ScrollControllerData, ScrollControllerLeaf, ScrollEndNotification,
    ScrollNotification, ScrollPhysics, ScrollPhysicsRef, ScrollPosition, ScrollPositionBase,
    ScrollPositionData, ScrollPositionRef, ScrollPositionWithSingleContext,
    ScrollPositionWithSingleContextData, ScrollPositionWithSingleContextLeaf,
    ScrollUpdateNotification, Semantics, SingleChildScrollView, SizedBox, Stack, State, StateData,
    StatefulWidget, StatelessWidget, Text, WidgetRef, WidgetState, WidgetStateMouseCursor,
    WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
};

use crate::colors::Colors;
use crate::constants::{
    K_DEFAULT_ICON_DARK_COLOR, K_DEFAULT_ICON_LIGHT_COLOR, K_TAB_LABEL_PADDING,
    K_TAB_SCROLL_DURATION,
};
use crate::debug::{debug_check_has_material, debug_check_has_material_localizations};
use crate::ink_well::{InkWell, InteractiveInkFeatureFactoryRef};
use crate::material::{Material, MaterialType};
use crate::material_localizations::MaterialLocalizations;
use crate::tab_bar_theme::{TabBarTheme, TabBarThemeData, TabBarThemeDataGetters};
use crate::tab_controller::{DefaultTabController, TabController};
use crate::tab_indicator::UnderlineTabIndicator;
use crate::theme::Theme;

use crate::generated::tabs_defaults_m3::{TabsPrimaryDefaultsM3, TabsSecondaryDefaultsM3};
use crate::theme_data_getters::DefaultsContext;

const K_TAB_HEIGHT: f64 = 46.0;
const K_TEXT_AND_ICON_TAB_HEIGHT: f64 = 72.0;
const K_START_OFFSET: f64 = 52.0;
const K_MIN_PRIMARY_TAB_INDICATOR_WIDTH: f64 = 24.0;

/// Defines how the bounds of the selected tab indicator are computed.
///
/// See also:
///
///  * `TabBar`, which displays a row of tabs.
///  * `TabBarView`, which displays a widget for the currently selected tab.
///  * `TabBar::indicator`, which defines the appearance of the selected tab
///    indicator relative to the tab's bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TabBarIndicatorSize {
    /// The tab indicator's bounds are as wide as the space occupied by the tab
    /// in the tab bar: from the right edge of the previous tab to the left edge
    /// of the next tab.
    Tab,

    /// The tab's bounds are only as wide as the (centered) tab widget itself.
    ///
    /// This value is used to align the tab's label, typically a `Tab`
    /// widget's text or icon, with the selected tab indicator.
    Label,
}

/// Defines how tabs are aligned horizontally in a `TabBar`.
///
/// See also:
///
///   * `TabBar`, which displays a row of tabs.
///   * `TabBarView`, which displays a widget for the currently selected tab.
///   * `TabBar::tab_alignment`, which defines the horizontal alignment of the
///     tabs within the `TabBar`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TabAlignment {
    // TODO(tahatesser): Add a link to the Material Design spec for
    // horizontal offset when it is available.
    // It's currently sourced from androidx/compose/material3/TabRow.kt.
    /// If `TabBar::is_scrollable` is true, tabs are aligned to the
    /// start of the `TabBar`. Otherwise throws an exception.
    ///
    /// It is not recommended to set [`TabAlignment::Start`] when
    /// `ThemeData::use_material3` is false.
    Start,

    /// If `TabBar::is_scrollable` is true, tabs are aligned to the
    /// start of the `TabBar` with an offset of 52.0 pixels.
    /// Otherwise throws an exception.
    ///
    /// It is not recommended to set [`TabAlignment::StartOffset`] when
    /// `ThemeData::use_material3` is false.
    StartOffset,

    /// If `TabBar::is_scrollable` is false, tabs are stretched to fill the
    /// `TabBar`. Otherwise throws an exception.
    Fill,

    /// Tabs are aligned to the center of the `TabBar`.
    Center,
}

/// Defines how the tab indicator animates when the selected tab changes.
///
/// See also:
///  * `TabBar`, which displays a row of tabs.
///  * `TabBarThemeData`, which can be used to configure the appearance of the tab
///    indicator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TabIndicatorAnimation {
    /// The tab indicator animates linearly.
    Linear,

    /// The tab indicator animates with an elastic effect.
    Elastic,
}

/// A Material Design `TabBar` tab.
///
/// If both [`icon`](Self::icon) and [`text`](Self::text) are provided, the text is displayed
/// below the icon.
///
/// See also:
///
///  * [`TabBar`], which displays a row of tabs.
///  * [`TabBarView`], which displays a widget for the currently selected tab.
///  * [`TabController`], which coordinates tab selection between a [`TabBar`] and a
///    [`TabBarView`].
///  * <https://material.io/design/components/tabs.html>
#[derive(Debug, Default)]
pub struct Tab {
    pub key: Option<KeyRef>,

    /// The text to display as the tab's label.
    ///
    /// Must not be used in combination with [`child`](Self::child).
    pub text: Option<String>,

    /// The widget to be used as the tab's label.
    ///
    /// Usually a `Text` widget, possibly wrapped in a `Semantics` widget.
    ///
    /// Must not be used in combination with [`text`](Self::text).
    pub child: Option<WidgetRef>,

    /// An icon to display as the tab's label.
    pub icon: Option<WidgetRef>,

    /// The margin added around the tab's icon.
    ///
    /// Only useful when used in combination with [`icon`](Self::icon), and either one of
    /// [`text`](Self::text) or [`child`](Self::child) is non-null.
    ///
    /// Defaults to 2 pixels of bottom margin. If `ThemeData::use_material3` is false,
    /// then defaults to 10 pixels of bottom margin.
    pub icon_margin: Option<EdgeInsetsGeometry>,

    /// The height of the [`Tab`].
    ///
    /// If null, the height will be calculated based on the content of the [`Tab`]. When `icon`
    /// is not null along with `child` or `text`, the default height is 72.0 pixels. Without an
    /// `icon`, the height is 46.0 pixels.
    ///
    /// The provided tab height cannot be lower than the default height. Use
    /// `PreferredSize` widget to adjust the overall [`TabBar`] height and match
    /// the provided tab [`height`](Self::height).
    pub height: Option<f64>,
}

impl Tab {
    /// Creates a Material Design [`TabBar`] tab; Dart's named arguments are the setters.
    ///
    /// At least one of [`text`](Self::text), [`icon`](Self::icon), and [`child`](Self::child)
    /// must be non-null. The [`text`](Self::text) and [`child`](Self::child) arguments must not
    /// be used at the same time. The [`icon_margin`](Self::icon_margin) is only useful when
    /// [`icon`](Self::icon) and either one of [`text`](Self::text) or [`child`](Self::child) is
    /// non-null.
    pub fn new() -> Tab {
        Tab::default()
    }

    /// Dart `Tab(key:)`.
    pub fn key(mut self, key: KeyRef) -> Tab {
        self.key = Some(key);
        self
    }

    /// Dart `Tab(text:)`.
    pub fn text(mut self, text: impl Into<String>) -> Tab {
        self.text = Some(text.into());
        self
    }

    /// Dart `Tab(icon:)`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> Tab {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Dart `Tab(iconMargin:)`.
    pub fn icon_margin(mut self, icon_margin: impl Into<EdgeInsetsGeometry>) -> Tab {
        self.icon_margin = Some(icon_margin.into());
        self
    }

    /// Dart `Tab(height:)`.
    pub fn height(mut self, height: f64) -> Tab {
        self.height = Some(height);
        self
    }

    /// Dart `Tab(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Tab {
        self.child = Some(child.into_widget());
        self
    }

    fn build_label_text(&self) -> WidgetRef {
        match &self.child {
            Some(child) => child.clone(),
            None => Text::new(self.text.clone().expect("a tab label has text or a child"))
                .soft_wrap(false)
                .overflow(TextOverflow::Fade)
                .into_widget(),
        }
    }
}

impl StatelessWidget for Tab {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(
            self.text.is_some() || self.child.is_some() || self.icon.is_some(),
            "Tab requires at least one of text, child, or icon to be non-null."
        );
        debug_assert!(
            self.text.is_none() || self.child.is_none(),
            "Provide either text or child, not both, when creating a Tab."
        );
    }

    fn as_preferred_size_widget(&self) -> Option<&dyn PreferredSizeWidget> {
        Some(self)
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));

        let calculated_height;
        let label;
        if self.icon.is_none() {
            calculated_height = K_TAB_HEIGHT;
            label = self.build_label_text();
        } else if self.text.is_none() && self.child.is_none() {
            calculated_height = K_TAB_HEIGHT;
            label = self.icon.clone().expect("checked above");
        } else {
            calculated_height = K_TEXT_AND_ICON_TAB_HEIGHT;
            let effective_icon_margin = self.icon_margin.unwrap_or_else(|| {
                if Theme::of(app, context).use_material3 {
                    TabsPrimaryDefaultsM3::ICON_MARGIN
                } else {
                    TabsDefaultsM2::ICON_MARGIN
                }
            });
            label = Column::new()
                .main_axis_alignment(MainAxisAlignment::Center)
                .children([
                    Padding::new(effective_icon_margin)
                        .child(self.icon.clone().expect("checked above"))
                        .into_widget(),
                    self.build_label_text(),
                ])
                .into_widget();
        }

        SizedBox::new()
            .height(self.height.unwrap_or(calculated_height))
            .child(Center::new().width_factor(1.0).child(label))
            .into_widget()
    }
}

impl PreferredSizeWidget for Tab {
    fn preferred_size(&self) -> waft_widgets::AnySize {
        if let Some(height) = self.height {
            Size::from_height(height).into()
        } else if (self.text.is_some() || self.child.is_some()) && self.icon.is_some() {
            Size::from_height(K_TEXT_AND_ICON_TAB_HEIGHT).into()
        } else {
            Size::from_height(K_TAB_HEIGHT).into()
        }
    }
}

/// Dart's `item is PreferredSizeWidget ? item.preferredSize : …` on an erased tab.
fn preferred_size_of(tab: &WidgetRef) -> Option<Size> {
    tab.as_preferred_size_widget()
        .map(|widget| widget.preferred_size().size())
}

/// Dart's `_TabStyle`.
struct TabStyle {
    animation: AnimationRef<f64>,
    is_selected: bool,
    #[allow(
        dead_code,
        reason = "Dart's `_TabStyle.isPrimary`, which Dart reads nowhere either"
    )]
    is_primary: bool,
    label_color: Option<AnyColor>,
    unselected_label_color: Option<Color>,
    label_style: Option<TextStyle>,
    unselected_label_style: Option<TextStyle>,
    defaults: Rc<dyn TabBarThemeDataGetters>,
    child: WidgetRef,
}

impl fmt::Debug for TabStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TabStyle")
            .field("animation", &self.animation)
            .field("is_selected", &self.is_selected)
            .field("label_color", &self.label_color)
            .field("unselected_label_color", &self.unselected_label_color)
            .field("label_style", &self.label_style)
            .field("unselected_label_style", &self.unselected_label_style)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl TabStyle {
    /// Dart's `_resolveWithLabelColor`: the colour of a label, between selected and not as
    /// the animation stands.
    fn resolve_with_label_color(
        &self,
        app: &mut App,
        context: BuildContext,
        icon_theme: Option<&IconThemeData>,
        states: &WidgetStates,
    ) -> Color {
        let theme_data = Theme::of(app, context);
        let tab_bar_theme = TabBarTheme::of(app, context);
        let animation = self.animation.clone();

        // labelStyle.color (and tabBarTheme.labelStyle.color) is not considered
        // as it'll be a breaking change without a possible migration plan. for
        // details: https://github.com/flutter/flutter/pull/109541#issuecomment-1294241417
        let selected_color: AnyColor = self
            .label_color
            .clone()
            .or_else(|| tab_bar_theme.label_color.clone())
            .or_else(|| {
                self.label_style
                    .as_ref()
                    .and_then(|style| style.color.clone())
            })
            .or_else(|| {
                tab_bar_theme
                    .label_style
                    .as_ref()
                    .and_then(|style| style.color.clone())
            })
            .or_else(|| self.defaults.label_color(app))
            .expect("the tab defaults have a label color");

        let unselected_color: Color;
        let selected_color: Color = if let Some(state_color) =
            selected_color.as_widget_state_property()
        {
            unselected_color = state_color.resolve(&WidgetStates::new());
            state_color.resolve(&WidgetStates::from([WidgetState::Selected]))
        } else {
            // unselectedLabelColor and tabBarTheme.unselectedLabelColor are ignored
            // when labelColor is a WidgetStateColor.
            let selected_color = selected_color.color();
            unselected_color = self
                .unselected_label_color
                .or(tab_bar_theme.unselected_label_color)
                .or_else(|| {
                    self.unselected_label_style
                        .as_ref()
                        .and_then(|style| style.color.as_ref().map(AnyColor::color))
                })
                .or_else(|| {
                    tab_bar_theme
                        .unselected_label_style
                        .as_ref()
                        .and_then(|style| style.color.as_ref().map(AnyColor::color))
                })
                .or_else(|| icon_theme.and_then(|theme| theme.color.as_ref().map(AnyColor::color)))
                .unwrap_or_else(|| {
                    if theme_data.use_material3 {
                        self.defaults
                            .unselected_label_color(app)
                            .expect("the Material 3 tab defaults have an unselected color")
                    } else {
                        selected_color.with_alpha(0xB2) // 70% alpha
                    }
                });
            selected_color
        };

        let value = animation.value(app);
        if states.contains(&WidgetState::Selected) {
            return Color::lerp(Some(selected_color), Some(unselected_color), value)
                .expect("two colors lerp to a color");
        }
        Color::lerp(Some(unselected_color), Some(selected_color), value)
            .expect("two colors lerp to a color")
    }
}

impl AnimatedWidget for TabStyle {
    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.animation.clone())
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let tab_bar_theme = TabBarTheme::of(app, context);
        let animation = self.animation.clone();

        let states = if self.is_selected {
            WidgetStates::from([WidgetState::Selected])
        } else {
            WidgetStates::new()
        };

        // To enable TextStyle.lerp(style1, style2, value), both styles must have
        // the same value of inherit. Force that to be inherit=true here.
        let selected_style = self
            .defaults
            .label_style(app)
            .expect("the tab defaults have a label style")
            .merge(
                self.label_style
                    .as_ref()
                    .or(tab_bar_theme.label_style.as_ref()),
            )
            .copy_with()
            .inherit(true);
        let unselected_style = self
            .defaults
            .unselected_label_style(app)
            .expect("the tab defaults have an unselected label style")
            .merge(
                self.unselected_label_style
                    .as_ref()
                    .or(tab_bar_theme.unselected_label_style.as_ref())
                    .or(self.label_style.as_ref()),
            )
            .copy_with()
            .inherit(true);
        let value = animation.value(app);
        let text_style = if self.is_selected {
            TextStyle::lerp(Some(&selected_style), Some(&unselected_style), value)
        } else {
            TextStyle::lerp(Some(&unselected_style), Some(&selected_style), value)
        }
        .expect("two styles lerp to a style");
        let default_icon_color = match theme.color_scheme.brightness {
            Brightness::Light => K_DEFAULT_ICON_DARK_COLOR,
            Brightness::Dark => K_DEFAULT_ICON_LIGHT_COLOR,
        };
        let icon_theme = IconTheme::of(app, context);
        let custom_icon_theme = (icon_theme.color.as_ref().map(AnyColor::color)
            != Some(default_icon_color))
        .then_some(icon_theme);
        let icon_color =
            self.resolve_with_label_color(app, context, custom_icon_theme.as_ref(), &states);
        let label_color = self.resolve_with_label_color(app, context, None, &states);

        DefaultTextStyle::new(
            text_style.copy_with().color(label_color),
            IconTheme::merge(
                None,
                IconThemeData::new()
                    .size(
                        custom_icon_theme
                            .as_ref()
                            .and_then(|theme| theme.size)
                            .unwrap_or(24.0),
                    )
                    .color(icon_color),
                self.child.clone(),
            ),
        )
        .into_widget()
    }
}

/// Dart's `_LayoutCallback`.
type LayoutCallback = Rc<dyn Fn(&mut App, Vec<f64>, TextDirection, f64)>;

/// Dart's `_TabLabelBarRenderer extends RenderFlex`.
struct TabLabelBarRenderer {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    flex: RenderFlexData,
    on_perform_layout: LayoutCallback,
}

impl TabLabelBarRenderer {
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's named arguments, in Dart's order"
    )]
    fn new(
        app: &mut App,
        direction: Axis,
        main_axis_size: MainAxisSize,
        main_axis_alignment: MainAxisAlignment,
        cross_axis_alignment: CrossAxisAlignment,
        text_direction: TextDirection,
        vertical_direction: VerticalDirection,
        on_perform_layout: LayoutCallback,
    ) -> RenderHandle<TabLabelBarRenderer> {
        let flex = RenderFlexData::new()
            .direction(direction)
            .main_axis_size(main_axis_size)
            .main_axis_alignment(main_axis_alignment)
            .cross_axis_alignment(cross_axis_alignment)
            .text_direction(Some(text_direction))
            .vertical_direction(vertical_direction);
        RenderHandle::new_box(
            app,
            TabLabelBarRenderer {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                flex,
                on_perform_layout,
            },
        )
    }
}

impl ContainerRenderObjectMixin for TabLabelBarRenderer {
    type ChildType = AnyRenderBox;
    type ParentDataType = FlexParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }
}

impl RenderBoxContainerDefaultsMixin for TabLabelBarRenderer {}

impl RenderFlexMixin for TabLabelBarRenderer {
    fn flex_data(self: RenderHandle<Self>, app: &App) -> &RenderFlexData {
        &self.get(app).flex
    }

    fn flex_data_mut(self: RenderHandle<Self>, app: &mut App) -> &mut RenderFlexData {
        &mut self.get_mut(app).flex
    }
}

impl RenderObject for TabLabelBarRenderer {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderFlexMixin::perform_layout(self, app);
        // xOffsets will contain childCount+1 values, giving the offsets of the
        // leading edge of the first tab as the first value, of the leading edge of
        // the each subsequent tab as each subsequent value, and of the trailing
        // edge of the last tab as the last value.
        let mut child = self.first_child(app);
        let mut x_offsets = Vec::new();
        while let Some(current) = child {
            let child_parent_data = current.as_object().parent_data_of::<FlexParentData>(app);
            x_offsets.push(child_parent_data.offset().dx());
            child = self.child_after(app, current);
        }
        let text_direction = RenderFlexMixin::text_direction(self, app)
            .expect("a tab label bar has a text direction");
        let width = self.size(app).width();
        match text_direction {
            TextDirection::Rtl => x_offsets.insert(0, width),
            TextDirection::Ltr => x_offsets.push(width),
        }
        let on_perform_layout = Rc::clone(&self.get(app).on_perform_layout);
        on_perform_layout(app, x_offsets, text_direction, width);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderFlexMixin::paint(self, app, context, offset)
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        RenderFlexMixin::dispose(self, app)
    }
}

impl RenderBox for TabLabelBarRenderer {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderFlexMixin::setup_parent_data(self, app, child)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderFlexMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderFlexMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderFlexMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderFlexMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderFlexMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderFlexMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderFlexMixin::compute_dry_layout(self, app, constraints)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderFlexMixin::hit_test_children(self, app, result, position)
    }
}

// This class and its renderer class only exist to report the widths of the tabs
// upon layout. The tab widths are only used at paint time (see _IndicatorPainter)
// or in response to input.
/// Dart's `_TabLabelBar extends Flex`: a [`Flex`] whose render object reports the tabs'
/// offsets after each layout.
struct TabLabelBar {
    flex: Flex,
    on_perform_layout: LayoutCallback,
}

impl TabLabelBar {
    fn new(
        children: Vec<WidgetRef>,
        on_perform_layout: LayoutCallback,
        main_axis_size: MainAxisSize,
    ) -> TabLabelBar {
        TabLabelBar {
            flex: Flex::new(Axis::Horizontal)
                .main_axis_alignment(MainAxisAlignment::Start)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .vertical_direction(VerticalDirection::Down)
                .main_axis_size(main_axis_size)
                .children(children),
            on_perform_layout,
        }
    }
}

impl fmt::Debug for TabLabelBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TabLabelBar")
            .field("flex", &self.flex)
            .finish()
    }
}

impl RenderObjectWidget for TabLabelBar {
    type RenderObject = TabLabelBarRenderer;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = self
            .flex
            .get_effective_text_direction(app, context)
            .expect("a horizontal flex has a text direction");
        TabLabelBarRenderer::new(
            app,
            self.flex.direction,
            self.flex.main_axis_size,
            self.flex.main_axis_alignment,
            self.flex.cross_axis_alignment,
            text_direction,
            self.flex.vertical_direction,
            Rc::clone(&self.on_perform_layout),
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<TabLabelBarRenderer>,
    ) {
        // Dart's `super.updateRenderObject`, `Flex.updateRenderObject`.
        render_object.set_direction(app, self.flex.direction);
        render_object.set_main_axis_alignment(app, self.flex.main_axis_alignment);
        render_object.set_main_axis_size(app, self.flex.main_axis_size);
        render_object.set_cross_axis_alignment(app, self.flex.cross_axis_alignment);
        let text_direction = self.flex.get_effective_text_direction(app, context);
        render_object.set_text_direction(app, text_direction);
        render_object.set_vertical_direction(app, self.flex.vertical_direction);
        render_object.set_text_baseline(app, self.flex.text_baseline);
        render_object.set_clip_behavior(app, self.flex.clip_behavior);
        render_object.set_spacing(app, self.flex.spacing);
        render_object.get_mut(app).on_perform_layout = Rc::clone(&self.on_perform_layout);
    }
}

impl MultiChildRenderObjectWidget for TabLabelBar {
    fn children(&self) -> &[WidgetRef] {
        &self.flex.children
    }
}

fn index_change_progress(app: &App, controller: Handle<TabController>) -> f64 {
    let controller_value = controller
        .animation(app)
        .expect("a valid controller has an animation")
        .value(app);
    let previous_index = controller.previous_index(app) as f64;
    let current_index = controller.index(app) as f64;

    // The controller's offset is changing because the user is dragging the
    // TabBarView's PageView to the left or right.
    if !controller.index_is_changing(app) {
        return clamp_double((current_index - controller_value).abs(), 0.0, 1.0);
    }

    // The TabController animation's value is changing from previousIndex to currentIndex.
    (controller_value - current_index).abs() / (current_index - previous_index).abs()
}

/// A stroke paint of `color`, `stroke_width` wide: Dart's `Paint()..color..strokeWidth`.
fn divider_paint(color: Color, stroke_width: f64) -> Paint {
    let mut paint = Paint::from_color(color.into());
    paint.style = PaintStyle::Stroke(Stroke::new(stroke_width as f32));
    paint
}

/// Dart's `_DividerPainter`.
struct DividerPainter {
    divider_color: Color,
    divider_height: f64,
}

impl CustomPainter for DividerPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        if self.divider_height <= 0.0 {
            return;
        }

        let paint = divider_paint(self.divider_color, self.divider_height);
        let stroke_width = self.divider_height;

        draw_line(
            canvas,
            Offset::new(0.0, size.height() - (stroke_width / 2.0)),
            Offset::new(size.width(), size.height() - (stroke_width / 2.0)),
            &paint,
        );
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        let Some(old_delegate) = old_delegate.as_any().downcast_ref::<DividerPainter>() else {
            return true;
        };
        old_delegate.divider_color != self.divider_color
            || old_delegate.divider_height != self.divider_height
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// A ChangeNotifier for triggering repaints when async resources load.
/// Dart's `_IndicatorPainterNotifier`.
struct IndicatorPainterNotifier {
    change_notifier: ChangeNotifierData,
}

impl IndicatorPainterNotifier {
    fn new(app: &mut App) -> Ref<IndicatorPainterNotifier> {
        app.create(IndicatorPainterNotifier {
            change_notifier: ChangeNotifierData::new(),
        })
    }

    fn notify(self: Handle<Self>, app: &mut App) {
        self.notify_listeners(app);
    }
}

impl ChangeNotifier for IndicatorPainterNotifier {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

/// Dart's `_IndicatorPainter`.
///
/// One painter is shared by the tab bar's state, which saves the tabs' offsets into it after
/// each layout and asks it for tab centres, and its `CustomPaint`; what Dart's painter
/// mutates is in cells.
struct IndicatorPainter {
    controller: Ref<TabController>,
    indicator: DecorationRef,
    indicator_size: TabBarIndicatorSize,
    minimum_indicator_width: Option<f64>,
    indicator_padding: EdgeInsetsGeometry,
    tab_keys: Rc<RefCell<Vec<GlobalKey>>>,
    label_paddings: Rc<RefCell<Vec<EdgeInsetsGeometry>>>,
    divider_color: Option<Color>,
    divider_height: Option<f64>,
    show_divider: bool,
    device_pixel_ratio: Option<f64>,
    indicator_animation: TabIndicatorAnimation,
    text_direction: TextDirection,
    repaint_notifier: Ref<IndicatorPainterNotifier>,
    repaint: Rc<dyn Listenable>,

    // _currentTabOffsets and _currentTextDirection are set each time TabBar
    // layout is completed. These values can be null when TabBar contains no
    // tabs, since there are nothing to lay out.
    current_tab_offsets: RefCell<Option<Vec<f64>>>,
    current_text_direction: Cell<Option<TextDirection>>,

    current_rect: Cell<Option<Rect>>,
    painter: RefCell<Option<Box<dyn BoxPainter>>>,
    needs_paint: Cell<bool>,
    this: Weak<IndicatorPainter>,
}

/// Dart's `_IndicatorPainter` constructor arguments.
struct IndicatorPainterArguments {
    controller: Ref<TabController>,
    indicator: DecorationRef,
    indicator_size: TabBarIndicatorSize,
    minimum_indicator_width: Option<f64>,
    tab_keys: Rc<RefCell<Vec<GlobalKey>>>,
    old: Option<Rc<IndicatorPainter>>,
    indicator_padding: EdgeInsetsGeometry,
    label_paddings: Rc<RefCell<Vec<EdgeInsetsGeometry>>>,
    divider_color: Option<Color>,
    divider_height: Option<f64>,
    show_divider: bool,
    device_pixel_ratio: Option<f64>,
    indicator_animation: TabIndicatorAnimation,
    text_direction: TextDirection,
}

impl IndicatorPainter {
    /// Dart's factory: the repaint notifier is made here so that it can be both the painter's
    /// repaint listenable (with the controller's animation) and its own field.
    fn new(app: &mut App, arguments: IndicatorPainterArguments) -> Rc<IndicatorPainter> {
        // Initializing [_IndicatorPainterNotifier] here that allows the
        // repaint notifier to be used in the super constructor call
        // (within [Listenable.merge]) while also being stored as a private field.
        //
        // The notifier is to trigger a repaint when asynchronous resources,
        // like images in the indicator [Decoration], are finished loading.
        let repaint_notifier = IndicatorPainterNotifier::new(app);
        let animation = arguments
            .controller
            .animation(app)
            .and_then(|animation| animation.upgrade(app));
        let repaint: Rc<dyn Listenable> = Rc::new(MergingListenable::new(vec![
            animation.map(|animation| Rc::new(animation) as Rc<dyn Listenable>),
            Some(Rc::new(repaint_notifier.clone()) as Rc<dyn Listenable>),
        ]));
        let painter = Rc::new_cyclic(|this| IndicatorPainter {
            controller: arguments.controller,
            indicator: arguments.indicator,
            indicator_size: arguments.indicator_size,
            minimum_indicator_width: arguments.minimum_indicator_width,
            indicator_padding: arguments.indicator_padding,
            tab_keys: arguments.tab_keys,
            label_paddings: arguments.label_paddings,
            divider_color: arguments.divider_color,
            divider_height: arguments.divider_height,
            show_divider: arguments.show_divider,
            device_pixel_ratio: arguments.device_pixel_ratio,
            indicator_animation: arguments.indicator_animation,
            text_direction: arguments.text_direction,
            repaint_notifier,
            repaint,
            current_tab_offsets: RefCell::new(None),
            current_text_direction: Cell::new(None),
            current_rect: Cell::new(None),
            painter: RefCell::new(None),
            needs_paint: Cell::new(false),
            this: this.clone(),
        });
        if let Some(old) = arguments.old {
            painter.save_tab_offsets(
                old.current_tab_offsets.borrow().clone(),
                old.current_text_direction.get(),
            );
        }
        painter
    }

    fn mark_needs_paint(&self, app: &mut App) {
        self.needs_paint.set(true);
        self.repaint_notifier.notify(app);
    }

    fn dispose(&self, app: &mut App) {
        if let Some(painter) = self.painter.borrow_mut().as_mut() {
            painter.dispose();
        }
        ChangeNotifierData::dispose(app, &self.repaint_notifier);
    }

    fn save_tab_offsets(
        &self,
        tab_offsets: Option<Vec<f64>>,
        text_direction: Option<TextDirection>,
    ) {
        *self.current_tab_offsets.borrow_mut() = tab_offsets;
        self.current_text_direction.set(text_direction);
    }

    // _currentTabOffsets[index] is the offset of the start edge of the tab at index, and
    // _currentTabOffsets[_currentTabOffsets.length] is the end edge of the last tab.
    fn max_tab_index(&self) -> usize {
        self.current_tab_offsets
            .borrow()
            .as_ref()
            .expect("the tabs were laid out")
            .len()
            - 2
    }

    fn center_of(&self, tab_index: usize) -> f64 {
        let offsets = self.current_tab_offsets.borrow();
        let offsets = offsets.as_ref().expect("the tabs were laid out");
        debug_assert!(!offsets.is_empty());
        debug_assert!(tab_index <= offsets.len() - 2);
        (offsets[tab_index] + offsets[tab_index + 1]) / 2.0
    }

    fn indicator_rect(&self, app: &mut App, tab_bar_size: Size, tab_index: usize) -> Rect {
        let current_text_direction = self
            .current_text_direction
            .get()
            .expect("the tabs were laid out");
        let (mut tab_left, mut tab_right) = {
            let offsets = self.current_tab_offsets.borrow();
            let offsets = offsets.as_ref().expect("the tabs were laid out");
            debug_assert!(!offsets.is_empty());
            debug_assert!(tab_index <= offsets.len() - 2);
            match current_text_direction {
                TextDirection::Rtl => (offsets[tab_index + 1], offsets[tab_index]),
                TextDirection::Ltr => (offsets[tab_index], offsets[tab_index + 1]),
            }
        };

        if self.indicator_size == TabBarIndicatorSize::Label {
            let tab_key = self.tab_keys.borrow()[tab_index].clone();
            let tab_width = tab_key
                .current_context(app)
                .and_then(|context| context.size(app))
                .expect("a laid-out tab has a size")
                .width();
            let label_padding = self.label_paddings.borrow()[tab_index];
            let insets = label_padding.resolve(Some(current_text_direction));
            let delta = ((tab_right - tab_left) - (tab_width + insets.horizontal())) / 2.0;
            tab_left += delta + insets.left;
            tab_right = tab_left + tab_width;
        }

        let indicator_width = tab_right - tab_left;
        if let Some(minimum_width) = self.minimum_indicator_width
            && indicator_width < minimum_width
        {
            let tab_center = (tab_left + tab_right) / 2.0;
            tab_left = tab_center - minimum_width / 2.0;
            tab_right = tab_center + minimum_width / 2.0;
        }

        let insets = self.indicator_padding.resolve(Some(current_text_direction));
        let rect = Rect::from_ltwh(tab_left, 0.0, tab_right - tab_left, tab_bar_size.height());

        if !rect
            .size()
            .ge(&EdgeInsetsGeometry::from(insets).collapsed_size())
        {
            panic!(
                "indicatorPadding insets should be less than Tab Size\n\
                 Rect Size : {:?}, Insets: {:?}",
                rect.size(),
                insets
            );
        }
        insets.deflate_rect(rect)
    }

    /// Applies the linear effect to the indicator.
    fn apply_linear_effect(&self, app: &mut App, size: Size, value: f64) -> Option<Rect> {
        let index = self.controller.index(app) as f64;
        let ltr = index > value;
        let max_tab_index = self.max_tab_index() as f64;
        let from = (if ltr { value.floor() } else { value.ceil() }).clamp(0.0, max_tab_index);
        let to = (if ltr { from + 1.0 } else { from - 1.0 }).clamp(0.0, max_tab_index);
        let from_rect = self.indicator_rect(app, size, from as usize);
        let to_rect = self.indicator_rect(app, size, to as usize);
        Rect::lerp(Some(from_rect), Some(to_rect), (value - from).abs())
    }

    // Ease out sine (decelerating).
    fn decelerate_interpolation(fraction: f64) -> f64 {
        ((fraction * PI) / 2.0).sin()
    }

    // Ease in sine (accelerating).
    fn accelerate_interpolation(fraction: f64) -> f64 {
        1.0 - ((fraction * PI) / 2.0).cos()
    }

    /// Applies the elastic effect to the indicator.
    fn apply_elastic_effect(&self, app: &mut App, size: Size, value: f64) -> Option<Rect> {
        let controller = self.controller.clone();
        let index = controller.index(app) as f64;
        let mut progress_left = (index - value).abs();
        let index_is_changing = controller.index_is_changing(app);
        let max_tab_index = self.max_tab_index() as f64;

        let to = if progress_left == 0.0 || !index_is_changing {
            match self.text_direction {
                TextDirection::Ltr => value.ceil(),
                TextDirection::Rtl => value.floor(),
            }
            .clamp(0.0, max_tab_index)
        } else {
            index
        };
        let from = if progress_left == 0.0 || !index_is_changing {
            match self.text_direction {
                TextDirection::Ltr => to - 1.0,
                TextDirection::Rtl => to + 1.0,
            }
            .clamp(0.0, max_tab_index)
        } else {
            controller.previous_index(app) as f64
        };
        let to_rect = self.indicator_rect(app, size, to as usize);
        let from_rect = self.indicator_rect(app, size, from as usize);
        let rect = Rect::lerp(Some(from_rect), Some(to_rect), (value - from).abs())
            .expect("two rects lerp to a rect");

        // If the tab animation is completed, there is no need to stretch the indicator
        // This only works for the tab change animation via tab index, not when
        // dragging a [TabBarView], but it's still ok, to avoid unnecessary calculations.
        if controller
            .animation(app)
            .expect("a valid controller has an animation")
            .is_completed(app)
        {
            return Some(rect);
        }

        let tab_change_progress = if index_is_changing {
            let tabs_delta =
                (controller.index(app) as f64 - controller.previous_index(app) as f64).abs();
            if tabs_delta != 0.0 {
                progress_left /= tabs_delta;
            }
            1.0 - clamp_double(progress_left, 0.0, 1.0)
        } else {
            (index - value).abs()
        };

        // If the animation has finished, there is no need to apply the stretch effect.
        if tab_change_progress == 1.0 {
            return Some(rect);
        }

        let is_moving_right = match self.text_direction {
            TextDirection::Ltr => {
                if index_is_changing {
                    index > value
                } else {
                    value > index
                }
            }
            TextDirection::Rtl => {
                if index_is_changing {
                    value > index
                } else {
                    index > value
                }
            }
        };
        let (left_fraction, right_fraction) = if is_moving_right {
            (
                Self::accelerate_interpolation(tab_change_progress),
                Self::decelerate_interpolation(tab_change_progress),
            )
        } else {
            (
                Self::decelerate_interpolation(tab_change_progress),
                Self::accelerate_interpolation(tab_change_progress),
            )
        };

        let lerp = |a: f64, b: f64, t: f64| {
            lerp_double(Some(a), Some(b), t).expect("two values lerp to a value")
        };
        let (lerp_rect_left, lerp_rect_right);

        // The controller.indexIsChanging is true when the Tab is pressed, instead of swipe to change tabs.
        // If the tab is pressed then only lerp between fromRect and toRect.
        if index_is_changing {
            lerp_rect_left = lerp(from_rect.left, to_rect.left, left_fraction);
            lerp_rect_right = lerp(from_rect.right, to_rect.right, right_fraction);
        } else {
            // Switch the Rect left and right lerp order based on swipe direction.
            lerp_rect_left = if is_moving_right {
                lerp(from_rect.left, to_rect.left, left_fraction)
            } else {
                lerp(to_rect.left, from_rect.left, left_fraction)
            };
            lerp_rect_right = if is_moving_right {
                lerp(from_rect.right, to_rect.right, right_fraction)
            } else {
                lerp(to_rect.right, from_rect.right, right_fraction)
            };
        }

        Some(Rect::from_ltrb(
            lerp_rect_left,
            rect.top,
            lerp_rect_right,
            rect.bottom,
        ))
    }
}

impl CustomPainter for IndicatorPainter {
    fn repaint(&self) -> Option<&Rc<dyn Listenable>> {
        Some(&self.repaint)
    }

    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        self.needs_paint.set(false);
        if self.painter.borrow().is_none() {
            let this = self.this.clone();
            let on_changed: Rc<dyn Fn(&mut App)> = Rc::new(move |app| {
                if let Some(this) = this.upgrade() {
                    this.mark_needs_paint(app);
                }
            });
            *self.painter.borrow_mut() = Some(self.indicator.create_box_painter(Some(on_changed)));
        }

        let value = self
            .controller
            .animation(app)
            .expect("a painted indicator has a valid controller")
            .value(app);

        let current_rect = match self.indicator_animation {
            TabIndicatorAnimation::Linear => self.apply_linear_effect(app, size, value),
            TabIndicatorAnimation::Elastic => self.apply_elastic_effect(app, size, value),
        };
        self.current_rect.set(current_rect);

        let current_rect = current_rect.expect("the indicator has a rect");

        let configuration = ImageConfiguration {
            size: Some(current_rect.size()),
            text_direction: self.current_text_direction.get(),
            device_pixel_ratio: self.device_pixel_ratio,
            ..ImageConfiguration::EMPTY
        };
        let divider_height = self.divider_height.unwrap_or(0.0);
        if self.show_divider && self.divider_height.expect("a shown divider has a height") > 0.0 {
            let divider_paint = divider_paint(
                self.divider_color.expect("a shown divider has a color"),
                divider_height,
            );
            let divider_p1 = Offset::new(0.0, size.height() - (divider_height / 2.0));
            let divider_p2 = Offset::new(size.width(), size.height() - (divider_height / 2.0));
            draw_line(canvas, divider_p1, divider_p2, &divider_paint);
        }
        self.painter
            .borrow_mut()
            .as_mut()
            .expect("made above")
            .paint(canvas, current_rect.top_left(), &configuration);
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        let Some(old) = old_delegate.as_any().downcast_ref::<IndicatorPainter>() else {
            return true;
        };
        self.needs_paint.get()
            || self.controller != old.controller
            || self.indicator != old.indicator
            || self.minimum_indicator_width != old.minimum_indicator_width
            || self.tab_keys.borrow().len() != old.tab_keys.borrow().len()
            || *self.current_tab_offsets.borrow() != *old.current_tab_offsets.borrow()
            || self.current_text_direction.get() != old.current_text_direction.get()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Dart's `_ChangeAnimation`: how far the controller's animation has come from the previous
/// index to the current one.
struct ChangeAnimation {
    controller: Ref<TabController>,
}

impl ChangeAnimation {
    fn new(app: &mut App, controller: Ref<TabController>) -> Ref<ChangeAnimation> {
        app.create(ChangeAnimation { controller })
    }
}

impl AnimationWithParent<f64> for ChangeAnimation {
    fn parent(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        app.get(self)
            .controller
            .animation(app)
            .expect("a change animation's controller is valid")
    }
}

impl Animation<f64> for ChangeAnimation {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        AnimationWithParent::add_listener(self, app, listener)
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        if app.get(self).controller.animation(app).is_some() {
            AnimationWithParent::remove_listener(self, app, listener)
        }
    }

    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        AnimationWithParent::add_status_listener(self, app, listener)
    }

    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        if app.get(self).controller.animation(app).is_some() {
            AnimationWithParent::remove_status_listener(self, app, listener)
        }
    }

    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        AnimationWithParent::status(self, app)
    }

    fn value(self: Handle<Self>, app: &App) -> f64 {
        index_change_progress(app, app.get(self).controller.handle())
    }
}

/// Dart's `_DragAnimation`: how far the `TabBarView` has been dragged from the tab at
/// `index`.
struct DragAnimation {
    controller: Ref<TabController>,
    index: usize,
}

impl DragAnimation {
    fn new(app: &mut App, controller: Ref<TabController>, index: usize) -> Ref<DragAnimation> {
        app.create(DragAnimation { controller, index })
    }
}

impl AnimationWithParent<f64> for DragAnimation {
    fn parent(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        app.get(self)
            .controller
            .animation(app)
            .expect("a drag animation's controller is valid")
    }
}

impl Animation<f64> for DragAnimation {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        AnimationWithParent::add_listener(self, app, listener)
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        if app.get(self).controller.animation(app).is_some() {
            AnimationWithParent::remove_listener(self, app, listener)
        }
    }

    fn add_status_listener(self: Handle<Self>, app: &mut App, listener: AnimationStatusListener) {
        AnimationWithParent::add_status_listener(self, app, listener)
    }

    fn remove_status_listener(
        self: Handle<Self>,
        app: &mut App,
        listener: &AnimationStatusListener,
    ) {
        if app.get(self).controller.animation(app).is_some() {
            AnimationWithParent::remove_status_listener(self, app, listener)
        }
    }

    fn status(self: Handle<Self>, app: &App) -> AnimationStatus {
        AnimationWithParent::status(self, app)
    }

    fn value(self: Handle<Self>, app: &App) -> f64 {
        let this = app.get(self);
        let controller = this.controller.clone();
        debug_assert!(!controller.index_is_changing(app));
        let controller_max_value = controller.length(app) as f64 - 1.0;
        let controller_value = clamp_double(
            controller
                .animation(app)
                .expect("a drag animation's controller is valid")
                .value(app),
            0.0,
            controller_max_value,
        );
        clamp_double((controller_value - this.index as f64).abs(), 0.0, 1.0)
    }
}

// This class, and TabBarScrollController, only exist to handle the case
// where a scrollable TabBar has a non-zero initialIndex. In that case we can
// only compute the scroll position's initial scroll offset (the "correct"
// pixels value) after the TabBar viewport width and scroll limits are known.
/// Dart's `_TabBarScrollPosition`.
struct TabBarScrollPosition {
    change_notifier: ChangeNotifierData,
    scroll_position: ScrollPositionData,
    single_context: ScrollPositionWithSingleContextData,
    tab_bar: Handle<TabBarState>,
    viewport_dimension_was_non_zero: bool,
    // The scroll position should be adjusted at least once.
    needs_pixels_correction: bool,
}

impl TabBarScrollPosition {
    fn new(
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
        tab_bar: Handle<TabBarState>,
    ) -> Ref<TabBarScrollPosition> {
        let scroll_position = ScrollPositionData::new(app, physics, context, true, None);
        let this = app.create(TabBarScrollPosition {
            change_notifier: ChangeNotifierData::new(),
            scroll_position,
            single_context: ScrollPositionWithSingleContextData::new(),
            tab_bar,
            viewport_dimension_was_non_zero: false,
            needs_pixels_correction: true,
        });
        ScrollPositionWithSingleContext::init(this.handle(), app, None, old_position);
        this
    }

    fn mark_needs_pixels_correction(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).needs_pixels_correction = true;
    }
}

impl ChangeNotifier for TabBarScrollPosition {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ViewportOffset for TabBarScrollPosition {
    fn pixels(self: Handle<Self>, app: &App) -> f64 {
        ScrollPositionBase::pixels(self, app)
    }

    fn has_pixels(self: Handle<Self>, app: &App) -> bool {
        ScrollPositionBase::has_pixels(self, app)
    }

    fn apply_viewport_dimension(
        self: Handle<Self>,
        app: &mut App,
        viewport_dimension: f64,
    ) -> bool {
        ScrollPositionBase::apply_viewport_dimension(self, app, viewport_dimension)
    }

    fn apply_content_dimensions(
        self: Handle<Self>,
        app: &mut App,
        min_scroll_extent: f64,
        max_scroll_extent: f64,
    ) -> bool {
        let mut result = true;
        if !app.get(self).viewport_dimension_was_non_zero {
            let was_non_zero = self.viewport_dimension(app) != 0.0;
            app.get_mut(self).viewport_dimension_was_non_zero = was_non_zero;
        }
        // If the viewport never had a non-zero dimension, we just want to jump
        // to the initial scroll position to avoid strange scrolling effects in
        // release mode: the viewport temporarily may have a dimension of zero
        // before the actual dimension is calculated. In that scenario, setting
        // the actual dimension would cause a strange scroll effect without this
        // guard because the super call below would start a ballistic scroll activity.
        let this = app.get(self);
        if !this.viewport_dimension_was_non_zero || this.needs_pixels_correction {
            app.get_mut(self).needs_pixels_correction = false;
            let tab_bar = app.get(self).tab_bar;
            let viewport_dimension = self.viewport_dimension(app);
            let initial_scroll_offset = tab_bar.initial_scroll_offset(
                app,
                viewport_dimension,
                min_scroll_extent,
                max_scroll_extent,
            );
            self.correct_pixels(app, initial_scroll_offset);
            result = false;
        }
        ScrollPositionBase::apply_content_dimensions(
            self,
            app,
            min_scroll_extent,
            max_scroll_extent,
        ) && result
    }

    fn correct_by(self: Handle<Self>, app: &mut App, correction: f64) {
        ScrollPositionBase::correct_by(self, app, correction);
    }

    fn jump_to(self: Handle<Self>, app: &mut App, value: f64) {
        ScrollPositionWithSingleContextLeaf::jump_to(self, app, value);
    }

    fn animate_to(
        self: Handle<Self>,
        app: &mut App,
        to: f64,
        duration: Duration,
        curve: Rc<dyn waft_animation::Curve>,
    ) -> waft_foundation::CompleterFuture<()> {
        ScrollPositionWithSingleContextLeaf::animate_to(self, app, to, duration, curve)
    }

    fn move_to(
        self: Handle<Self>,
        app: &mut App,
        to: f64,
        duration: Option<Duration>,
        curve: Option<Rc<dyn waft_animation::Curve>>,
        clamp: Option<bool>,
    ) -> waft_foundation::CompleterFuture<()> {
        ScrollPositionBase::move_to(self, app, to, duration, curve, clamp)
    }

    fn user_scroll_direction(self: Handle<Self>, app: &App) -> waft_rendering::ScrollDirection {
        ScrollPositionWithSingleContextLeaf::user_scroll_direction(self, app)
    }

    fn allow_implicit_scrolling(self: Handle<Self>, app: &App) -> bool {
        ScrollPositionBase::allow_implicit_scrolling(self, app)
    }
}

impl ScrollPosition for TabBarScrollPosition {
    waft_widgets::scroll_position_with_single_context_overrides!();
}

impl ScrollActivityDelegate for TabBarScrollPosition {
    waft_widgets::scroll_position_with_single_context_delegate_overrides!();
}

impl ScrollPositionWithSingleContextLeaf for TabBarScrollPosition {
    waft_widgets::scroll_position_with_single_context_accessors!();
}

/// The `ScrollController` for a [`TabBar`] widget.
pub struct TabBarScrollController {
    change_notifier: ChangeNotifierData,
    scroll_controller: ScrollControllerData,
    /// The state of the [`TabBar`] widget to which this controller is attached.
    ///
    /// Is `None` if this controller is not attached to a [`TabBar`].
    tab_bar_state: Option<Handle<TabBarState>>,
}

impl TabBarScrollController {
    /// Creates a scroll controller for a [`TabBar`], with Dart's `ScrollController` defaults.
    pub fn new(app: &mut App) -> Ref<TabBarScrollController> {
        app.create(TabBarScrollController {
            change_notifier: ChangeNotifierData::new(),
            scroll_controller: ScrollControllerData::new(0.0, true, None, None, None),
            tab_bar_state: None,
        })
    }

    /// Asserts that this controller is currently attached to a [`TabBar`]'s `State`.
    ///
    /// To invoke this function, wrap it in an assert:
    /// `debug_assert!(controller.debug_check_has_tab_bar_state(app))`.
    ///
    /// Does nothing if asserts are disabled. Always returns true.
    pub fn debug_check_has_tab_bar_state(self: Handle<Self>, app: &App) -> bool {
        debug_assert!(
            app.get(self).tab_bar_state.is_some(),
            "This TabBarScrollController is not attached to any TabBar."
        );
        true
    }
}

impl ChangeNotifier for TabBarScrollController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ScrollControllerLeaf for TabBarScrollController {
    waft_widgets::scroll_controller_accessors!();

    fn create_scroll_position(
        self: Handle<Self>,
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
    ) -> ScrollPositionRef {
        debug_assert!(self.debug_check_has_tab_bar_state(app));

        let tab_bar = app.get(self).tab_bar_state.expect("asserted above");
        TabBarScrollPosition::new(app, physics, context, old_position, tab_bar).into_erased()
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).tab_bar_state = None;
        waft_widgets::ScrollController::dispose(self.as_controller(), app);
    }
}

/// Signature for [`TabBar`] callbacks that report that an underlying value has
/// changed for a given [`Tab`] at `index`.
///
/// Used for [`TabBar::on_hover`] and [`TabBar::on_focus_change`] callbacks The provided
/// `value` being true indicates focus has been gained, or a pointer has hovered
/// over the tab, with false indicated focus has been lost or the pointer has
/// exited hovering.
pub type TabValueChanged<T> = Rc<dyn Fn(&mut App, T, usize)>;

/// A Material Design primary tab bar.
///
/// Primary tabs are placed at the top of the content pane under a top app bar.
/// They display the main content destinations.
///
/// Typically created as the `AppBar::bottom` part of an `AppBar` and in
/// conjunction with a [`TabBarView`].
///
/// If a [`TabController`] is not provided, then a [`DefaultTabController`] ancestor
/// must be provided instead. The tab controller's [`TabController::length`] must
/// equal the length of the [`tabs`](Self::tabs) list and the length of the
/// [`TabBarView::children`] list.
///
/// Requires one of its ancestors to be a [`Material`] widget.
///
/// Uses values from [`TabBarThemeData`] if it is set in the current context.
///
/// See also:
///
///  * [`TabBar::secondary`], for a secondary tab bar.
///  * [`TabBarView`], which displays page views that correspond to each tab.
///  * [`TabController`], which coordinates tab selection between a [`TabBar`] and a
///    [`TabBarView`].
///  * <https://m3.material.io/components/tabs/overview>, the Material 3
///    tab bar specification.
pub struct TabBar {
    pub key: Option<KeyRef>,

    /// Typically a list of two or more [`Tab`] widgets.
    ///
    /// The length of this list must match the [`controller`](Self::controller)'s
    /// [`TabController::length`] and the length of the [`TabBarView::children`] list.
    pub tabs: Vec<WidgetRef>,

    /// This widget's selection and animation state.
    ///
    /// If [`TabController`] is not provided, then the value of [`DefaultTabController::of`]
    /// will be used.
    pub controller: Option<Ref<TabController>>,

    /// The [`TabBarScrollController`] for this [`TabBar`].
    ///
    /// This controller can be used to manipulate the scroll position of the [`TabBar`].
    pub scroll_controller: Option<Ref<TabBarScrollController>>,

    /// Whether this tab bar can be scrolled horizontally.
    ///
    /// If [`is_scrollable`](Self::is_scrollable) is true, then each tab is as wide as needed
    /// for its label and the entire [`TabBar`] is scrollable. Otherwise each tab gets an equal
    /// share of the available space.
    pub is_scrollable: bool,

    /// The amount of space by which to inset the tab bar.
    ///
    /// When [`is_scrollable`](Self::is_scrollable) is false, this will yield the same result
    /// as if [`TabBar`] was wrapped in a `Padding` widget. When
    /// [`is_scrollable`](Self::is_scrollable) is true, the scrollable itself is inset,
    /// allowing the padding to scroll with the tab bar, rather than enclosing it.
    pub padding: Option<EdgeInsetsGeometry>,

    /// The color of the line that appears below the selected tab.
    ///
    /// If this parameter is null, then the value of the Theme's indicatorColor
    /// property is used.
    ///
    /// If [`indicator`](Self::indicator) is specified or provided from [`TabBarThemeData`],
    /// this property is ignored.
    pub indicator_color: Option<Color>,

    /// The thickness of the line that appears below the selected tab.
    ///
    /// The value of this parameter must be greater than zero.
    ///
    /// If `ThemeData::use_material3` is true and [`TabBar`] is used to create a
    /// primary tab bar, the default value is 3.0. If the provided value is less
    /// than 3.0, the default value is used.
    ///
    /// If `ThemeData::use_material3` is true and [`TabBar::secondary`] is used to
    /// create a secondary tab bar, the default value is 2.0.
    ///
    /// If `ThemeData::use_material3` is false, the default value is 2.0.
    ///
    /// If [`indicator`](Self::indicator) is specified or provided from [`TabBarThemeData`],
    /// this property is ignored. Otherwise, when drawing the default underline
    /// indicator, the value of this parameter must be greater than zero.
    pub indicator_weight: f64,

    /// The padding for the indicator.
    ///
    /// The default value of this property is `EdgeInsets::ZERO`.
    ///
    /// For [`is_scrollable`](Self::is_scrollable) tab bars, specifying `K_TAB_LABEL_PADDING`
    /// will align the indicator with the tab's text for [`Tab`] widgets and all but the
    /// shortest [`Tab::text`] values.
    pub indicator_padding: EdgeInsetsGeometry,

    /// Defines the appearance of the selected tab indicator.
    ///
    /// If [`indicator`](Self::indicator) is specified or provided from [`TabBarThemeData`],
    /// the [`indicator_color`](Self::indicator_color) and
    /// [`indicator_weight`](Self::indicator_weight) properties are ignored.
    ///
    /// The default, underline-style, selected tab indicator can be defined with
    /// [`UnderlineTabIndicator`].
    ///
    /// The indicator's size is based on the tab's bounds. If
    /// [`indicator_size`](Self::indicator_size) is [`TabBarIndicatorSize::Tab`] the tab's
    /// bounds are as wide as the space occupied by the tab in the tab bar. If
    /// [`indicator_size`](Self::indicator_size) is [`TabBarIndicatorSize::Label`], then the
    /// tab's bounds are only as wide as the tab widget itself.
    ///
    /// See also:
    ///
    ///  * [`splash_border_radius`](Self::splash_border_radius), which defines the clipping
    ///    radius of the splash and is generally used with `BoxDecoration::border_radius`.
    pub indicator: Option<DecorationRef>,

    /// Whether this tab bar should automatically adjust the
    /// [`indicator_color`](Self::indicator_color).
    ///
    /// The default value of this property is true.
    ///
    /// If [`automatic_indicator_color_adjustment`](Self::automatic_indicator_color_adjustment)
    /// is true, then the [`indicator_color`](Self::indicator_color) will be automatically
    /// adjusted to `Colors::WHITE` when the [`indicator_color`](Self::indicator_color) is same
    /// as `Material::color` of the [`Material`] parent widget.
    pub automatic_indicator_color_adjustment: bool,

    /// Defines how the selected tab indicator's size is computed.
    ///
    /// The size of the selected tab indicator is defined relative to the
    /// tab's overall bounds if [`indicator_size`](Self::indicator_size) is
    /// [`TabBarIndicatorSize::Tab`] (the default) or relative to the bounds of the tab's
    /// widget if [`indicator_size`](Self::indicator_size) is [`TabBarIndicatorSize::Label`].
    ///
    /// The selected tab's location appearance can be refined further with
    /// the [`indicator_color`](Self::indicator_color),
    /// [`indicator_weight`](Self::indicator_weight),
    /// [`indicator_padding`](Self::indicator_padding), and [`indicator`](Self::indicator)
    /// properties.
    pub indicator_size: Option<TabBarIndicatorSize>,

    /// The color of the divider.
    ///
    /// If the [`divider_color`](Self::divider_color) is `Colors::TRANSPARENT`, then the
    /// divider will not be drawn.
    ///
    /// If null and `ThemeData::use_material3` is false, [`TabBarThemeData::divider_color`]
    /// color is used. If that is null and `ThemeData::use_material3` is true,
    /// `ColorScheme::outline_variant` will be used, otherwise divider will not be drawn.
    pub divider_color: Option<Color>,

    /// The height of the divider.
    ///
    /// If the [`divider_height`](Self::divider_height) is zero or negative, then the divider
    /// will not be drawn.
    ///
    /// If null and `ThemeData::use_material3` is true, [`TabBarThemeData::divider_height`] is
    /// used. If that is also null and `ThemeData::use_material3` is true, 1.0 will be used.
    /// Otherwise divider will not be drawn.
    pub divider_height: Option<f64>,

    /// The color of selected tab labels.
    ///
    /// If null, then [`TabBarThemeData::label_color`] is used. If that is also null and
    /// `ThemeData::use_material3` is true, `ColorScheme::primary` will be used,
    /// otherwise the color of the `ThemeData::primary_text_theme`'s
    /// `TextTheme::body_large` text color is used.
    ///
    /// If [`label_color`](Self::label_color) (or, if null, [`TabBarThemeData::label_color`])
    /// is a `WidgetStateColor`, then the effective tab color will depend on the
    /// `WidgetState::Selected` state, i.e. if the [`Tab`] is selected or not,
    /// ignoring [`unselected_label_color`](Self::unselected_label_color) even if it's non-null.
    ///
    /// When this color or the [`TabBarThemeData::label_color`] is specified, it overrides
    /// the `TextStyle::color` specified for the [`label_style`](Self::label_style) or the
    /// [`TabBarThemeData::label_style`].
    ///
    /// See also:
    ///
    ///   * [`unselected_label_color`](Self::unselected_label_color), for color of unselected
    ///     tab labels.
    pub label_color: Option<AnyColor>,

    /// The text style of the selected tab labels.
    ///
    /// The color specified in [`label_style`](Self::label_style) and
    /// [`TabBarThemeData::label_style`] is used to style the label when
    /// [`label_color`](Self::label_color) or [`TabBarThemeData::label_color`] are not
    /// specified.
    ///
    /// If [`unselected_label_style`](Self::unselected_label_style) is null, then this text
    /// style will be used for both selected and unselected label styles.
    ///
    /// If this property is null, then [`TabBarThemeData::label_style`] will be used.
    ///
    /// If that is also null and `ThemeData::use_material3` is true, `TextTheme::title_small`
    /// will be used, otherwise the text style of the `ThemeData::primary_text_theme`'s
    /// `TextTheme::body_large` definition is used.
    pub label_style: Option<TextStyle>,

    /// The padding added to each of the tab labels.
    ///
    /// If there are few tabs with both icon and text and few
    /// tabs with only icon or text, this padding is vertically
    /// adjusted to provide uniform padding to all tabs.
    ///
    /// If this property is null, then `K_TAB_LABEL_PADDING` is used.
    pub label_padding: Option<EdgeInsetsGeometry>,

    /// The color of unselected tab labels.
    ///
    /// If [`label_color`](Self::label_color) (or, if null, [`TabBarThemeData::label_color`])
    /// is a `WidgetStateColor`, then the unselected tabs are rendered with
    /// that `WidgetStateColor`'s resolved color for unselected state, even if
    /// [`unselected_label_color`](Self::unselected_label_color) is non-null.
    ///
    /// If null, then [`TabBarThemeData::unselected_label_color`] is used. If that is also
    /// null and `ThemeData::use_material3` is true, `ColorScheme::on_surface_variant`
    /// will be used, otherwise unselected tab labels are rendered with
    /// [`label_color`](Self::label_color) at 70% opacity.
    ///
    /// When this color or the [`TabBarThemeData::unselected_label_color`] is specified, it
    /// overrides the `TextStyle::color` specified for the
    /// [`unselected_label_style`](Self::unselected_label_style) or the
    /// [`TabBarThemeData::unselected_label_style`].
    ///
    /// See also:
    ///
    ///  * [`label_color`](Self::label_color), for color of selected tab labels.
    pub unselected_label_color: Option<Color>,

    /// The text style of the unselected tab labels.
    ///
    /// The color specified in [`unselected_label_style`](Self::unselected_label_style) and
    /// [`TabBarThemeData::unselected_label_style`] is used to style the label when
    /// [`unselected_label_color`](Self::unselected_label_color) or
    /// [`TabBarThemeData::unselected_label_color`] are not specified.
    ///
    /// If this property is null, then [`TabBarThemeData::unselected_label_style`] will be
    /// used.
    ///
    /// If that is also null and `ThemeData::use_material3` is true, `TextTheme::title_small`
    /// will be used, otherwise then the [`label_style`](Self::label_style) value is used. If
    /// [`label_style`](Self::label_style) is null, the text style of the
    /// `ThemeData::primary_text_theme`'s `TextTheme::body_large` definition is used.
    pub unselected_label_style: Option<TextStyle>,

    /// Defines the ink response focus, hover, and splash colors.
    ///
    /// If non-null, it is resolved against one of `WidgetState::Focused`,
    /// `WidgetState::Hovered`, and `WidgetState::Pressed`.
    ///
    /// `WidgetState::Pressed` triggers a ripple (an ink splash), per
    /// the current Material Design spec.
    ///
    /// If the overlay color is null or resolves to null, then if `ThemeData::use_material3`
    /// is false, the default values for `InkResponse::focus_color`,
    /// `InkResponse::hover_color`, `InkResponse::splash_color`, and
    /// `InkResponse::highlight_color` will be used instead. If `ThemeData::use_material3`
    /// if true, the default values are:
    /// * selected:
    ///   * pressed - ThemeData.colorScheme.primary(0.1)
    ///   * hovered - ThemeData.colorScheme.primary(0.08)
    ///   * focused - ThemeData.colorScheme.primary(0.1)
    /// * pressed - ThemeData.colorScheme.primary(0.1)
    /// * hovered - ThemeData.colorScheme.onSurface(0.08)
    /// * focused - ThemeData.colorScheme.onSurface(0.1)
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// Determines the way that drag start behavior is handled.
    pub drag_start_behavior: DragStartBehavior,

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// individual tab widgets.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a `WidgetStateMouseCursor`,
    /// `WidgetStateProperty::resolve` is used for the following `WidgetState`s:
    ///
    ///  * `WidgetState::Selected`.
    ///
    /// If null, then the value of [`TabBarThemeData::mouse_cursor`] is used. If
    /// that is also null, then `WidgetStateMouseCursor::clickable` is used.
    pub mouse_cursor: Option<MouseCursorRef>,

    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a long-press
    /// will produce a short vibration, when feedback is enabled.
    ///
    /// Defaults to true.
    pub enable_feedback: Option<bool>,

    /// An optional callback that's called when the [`TabBar`] is tapped.
    ///
    /// The callback is applied to the index of the tab where the tap occurred.
    ///
    /// This callback has no effect on the default handling of taps. It's for
    /// applications that want to do a little extra work when a tab is tapped,
    /// even if the tap doesn't change the TabController's index. TabBar
    /// [`on_tap`](Self::on_tap) callbacks should not make changes to the TabController since
    /// that would interfere with the default tap handler.
    pub on_tap: Option<ValueChanged<usize>>,

    /// An optional callback that's called when a [`Tab`]'s hover state in the
    /// [`TabBar`] changes.
    ///
    /// Called when a pointer enters or exits the ink response area of the [`Tab`].
    ///
    /// The value passed to the callback is true if a pointer has entered the
    /// [`Tab`] at `index` and false if a pointer has exited.
    ///
    /// When hover is moved from one tab directly to another, this will be called
    /// twice. First to represent hover exiting the initial tab, and then second
    /// for the pointer entering hover over the next tab.
    pub on_hover: Option<TabValueChanged<bool>>,

    /// An optional callback that's called when a [`Tab`]'s focus state in the
    /// [`TabBar`] changes.
    ///
    /// Called when the node for the [`Tab`] at `index` gains or loses focus.
    ///
    /// The value passed to the callback is true if the node has gained focus for
    /// the [`Tab`] at `index` and false if focus has been lost.
    ///
    /// When focus is moved from one tab directly to another, this will be called
    /// twice. First to represent focus being lost by the initially focused tab,
    /// and then second for the next tab gaining focus.
    pub on_focus_change: Option<TabValueChanged<bool>>,

    /// How the [`TabBar`]'s scroll view should respond to user input.
    ///
    /// For example, determines how the scroll view continues to animate after the
    /// user stops dragging the scroll view.
    ///
    /// Defaults to matching platform conventions.
    pub physics: Option<ScrollPhysicsRef>,

    /// Creates the tab bar's [`InkWell`] splash factory, which defines
    /// the appearance of "ink" splashes that occur in response to taps.
    ///
    /// Use `NoSplash::splash_factory` to defeat ink splash rendering. For example
    /// to defeat both the splash and the hover/pressed overlay, but not the
    /// keyboard focused overlay:
    ///
    /// ```text
    /// TabBar::new(tabs)
    ///     .splash_factory(NoSplash::splash_factory())
    ///     .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
    ///         if states.contains(&WidgetState::Focused) { None } else { Some(Colors::TRANSPARENT) }
    ///     }))
    /// ```
    pub splash_factory: Option<InteractiveInkFeatureFactoryRef>,

    /// Defines the clipping radius of splashes that extend outside the bounds of the tab.
    ///
    /// This can be useful to match the `BoxDecoration::border_radius` provided as
    /// [`indicator`](Self::indicator).
    ///
    /// If this property is null, it is interpreted as `BorderRadius::ZERO`.
    pub splash_border_radius: Option<BorderRadius>,

    /// Specifies the horizontal alignment of the tabs within a [`TabBar`].
    ///
    /// If [`TabBar::is_scrollable`] is false, only [`TabAlignment::Fill`] and
    /// [`TabAlignment::Center`] are supported. Otherwise an exception is thrown.
    ///
    /// If [`TabBar::is_scrollable`] is true, only [`TabAlignment::Start`],
    /// [`TabAlignment::StartOffset`], and [`TabAlignment::Center`] are supported. Otherwise
    /// an exception is thrown.
    ///
    /// If this is null, then the value of [`TabBarThemeData::tab_alignment`] is used.
    ///
    /// If [`TabBarThemeData::tab_alignment`] is null and `ThemeData::use_material3` is true,
    /// then [`TabAlignment::StartOffset`] is used if [`is_scrollable`](Self::is_scrollable)
    /// is true, otherwise [`TabAlignment::Fill`] is used.
    ///
    /// If [`TabBarThemeData::tab_alignment`] is null and `ThemeData::use_material3` is false,
    /// then [`TabAlignment::Center`] is used if [`is_scrollable`](Self::is_scrollable) is
    /// true, otherwise [`TabAlignment::Fill`] is used.
    pub tab_alignment: Option<TabAlignment>,

    /// Specifies the text scaling behavior for the [`Tab`] label.
    ///
    /// If this is null, then the value of [`TabBarThemeData::text_scaler`] is used. If that is
    /// also null, then the text scaling behavior is determined by the
    /// `MediaQueryData::text_scaler` from the ambient [`MediaQuery`], or 1.0 if there is no
    /// [`MediaQuery`] in scope.
    ///
    /// See also:
    ///   * [`TextScaler`], which is used to scale text based on the device's text scale
    ///     factor.
    pub text_scaler: Option<TextScaler>,

    /// Specifies the animation behavior of the tab indicator.
    ///
    /// If this is null, then the value of [`TabBarThemeData::indicator_animation`] is used.
    /// If that is also null, then the tab indicator will animate linearly if the
    /// [`indicator_size`](Self::indicator_size) is [`TabBarIndicatorSize::Tab`], otherwise it
    /// will animate with an elastic effect if the [`indicator_size`](Self::indicator_size) is
    /// [`TabBarIndicatorSize::Label`].
    ///
    /// See also:
    ///
    ///  * [`TabIndicatorAnimation`], which specifies the animation behavior of the tab
    ///    indicator.
    pub indicator_animation: Option<TabIndicatorAnimation>,

    /// Whether this tab bar is a primary tab bar.
    ///
    /// Otherwise, it is a secondary tab bar.
    is_primary: bool,
}

impl TabBar {
    /// Creates a Material Design primary tab bar; Dart's named arguments are the setters.
    ///
    /// The length of the [`tabs`](Self::tabs) argument must match the
    /// [`controller`](Self::controller)'s [`TabController::length`].
    ///
    /// If a [`TabController`] is not provided, then there must be a
    /// [`DefaultTabController`] ancestor.
    ///
    /// The [`indicator_weight`](Self::indicator_weight) parameter defaults to 2.
    ///
    /// The [`indicator_padding`](Self::indicator_padding) parameter defaults to
    /// `EdgeInsets::ZERO`.
    ///
    /// If [`indicator`](Self::indicator) is not null or provided from [`TabBarTheme`],
    /// then [`indicator_color`](Self::indicator_color) is ignored.
    ///
    /// The [`indicator_weight`](Self::indicator_weight) does not affect the visual appearance
    /// of the indicator when a custom [`indicator`](Self::indicator) is provided. However,
    /// it may still be used to compute the TabBar's preferred size.
    pub fn new(tabs: impl IntoIterator<Item = WidgetRef>) -> TabBar {
        TabBar::with_primary(tabs, true)
    }

    /// Creates a Material Design secondary tab bar.
    ///
    /// Secondary tabs are used within a content area to further separate related
    /// content and establish hierarchy.
    ///
    /// See also:
    ///
    ///  * [`TabBar::new`], for a primary tab bar.
    ///  * [`TabBarView`], which displays page views that correspond to each tab.
    ///  * [`TabController`], which coordinates tab selection between a [`TabBar`] and a
    ///    [`TabBarView`].
    ///  * <https://m3.material.io/components/tabs/overview>, the Material 3
    ///    tab bar specification.
    pub fn secondary(tabs: impl IntoIterator<Item = WidgetRef>) -> TabBar {
        TabBar::with_primary(tabs, false)
    }

    fn with_primary(tabs: impl IntoIterator<Item = WidgetRef>, is_primary: bool) -> TabBar {
        TabBar {
            key: None,
            tabs: tabs.into_iter().collect(),
            controller: None,
            scroll_controller: None,
            is_scrollable: false,
            padding: None,
            indicator_color: None,
            automatic_indicator_color_adjustment: true,
            indicator_weight: 2.0,
            indicator_padding: EdgeInsetsGeometry::ZERO,
            indicator: None,
            indicator_size: None,
            divider_color: None,
            divider_height: None,
            label_color: None,
            label_style: None,
            label_padding: None,
            unselected_label_color: None,
            unselected_label_style: None,
            drag_start_behavior: DragStartBehavior::Start,
            overlay_color: None,
            mouse_cursor: None,
            enable_feedback: None,
            on_tap: None,
            on_hover: None,
            on_focus_change: None,
            physics: None,
            splash_factory: None,
            splash_border_radius: None,
            tab_alignment: None,
            text_scaler: None,
            indicator_animation: None,
            is_primary,
        }
    }

    /// Dart `TabBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> TabBar {
        self.key = Some(key);
        self
    }

    /// Dart `TabBar(controller:)`.
    pub fn controller(mut self, controller: Ref<TabController>) -> TabBar {
        self.controller = Some(controller);
        self
    }

    /// Dart `TabBar(scrollController:)`.
    pub fn scroll_controller(mut self, scroll_controller: Ref<TabBarScrollController>) -> TabBar {
        self.scroll_controller = Some(scroll_controller);
        self
    }

    /// Dart `TabBar(isScrollable:)`.
    pub fn is_scrollable(mut self, is_scrollable: bool) -> TabBar {
        self.is_scrollable = is_scrollable;
        self
    }

    /// Dart `TabBar(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> TabBar {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `TabBar(indicatorColor:)`.
    pub fn indicator_color(mut self, indicator_color: Color) -> TabBar {
        self.indicator_color = Some(indicator_color);
        self
    }

    /// Dart `TabBar(automaticIndicatorColorAdjustment:)`.
    pub fn automatic_indicator_color_adjustment(
        mut self,
        automatic_indicator_color_adjustment: bool,
    ) -> TabBar {
        self.automatic_indicator_color_adjustment = automatic_indicator_color_adjustment;
        self
    }

    /// Dart `TabBar(indicatorWeight:)`.
    pub fn indicator_weight(mut self, indicator_weight: f64) -> TabBar {
        self.indicator_weight = indicator_weight;
        self
    }

    /// Dart `TabBar(indicatorPadding:)`.
    pub fn indicator_padding(mut self, indicator_padding: impl Into<EdgeInsetsGeometry>) -> TabBar {
        self.indicator_padding = indicator_padding.into();
        self
    }

    /// Dart `TabBar(indicator:)`.
    pub fn indicator(mut self, indicator: impl Into<DecorationRef>) -> TabBar {
        self.indicator = Some(indicator.into());
        self
    }

    /// Dart `TabBar(indicatorSize:)`.
    pub fn indicator_size(mut self, indicator_size: TabBarIndicatorSize) -> TabBar {
        self.indicator_size = Some(indicator_size);
        self
    }

    /// Dart `TabBar(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> TabBar {
        self.divider_color = Some(divider_color);
        self
    }

    /// Dart `TabBar(dividerHeight:)`.
    pub fn divider_height(mut self, divider_height: f64) -> TabBar {
        self.divider_height = Some(divider_height);
        self
    }

    /// Dart `TabBar(labelColor:)`.
    pub fn label_color(mut self, label_color: impl Into<AnyColor>) -> TabBar {
        self.label_color = Some(label_color.into());
        self
    }

    /// Dart `TabBar(labelStyle:)`.
    pub fn label_style(mut self, label_style: TextStyle) -> TabBar {
        self.label_style = Some(label_style);
        self
    }

    /// Dart `TabBar(labelPadding:)`.
    pub fn label_padding(mut self, label_padding: impl Into<EdgeInsetsGeometry>) -> TabBar {
        self.label_padding = Some(label_padding.into());
        self
    }

    /// Dart `TabBar(unselectedLabelColor:)`.
    pub fn unselected_label_color(mut self, unselected_label_color: Color) -> TabBar {
        self.unselected_label_color = Some(unselected_label_color);
        self
    }

    /// Dart `TabBar(unselectedLabelStyle:)`.
    pub fn unselected_label_style(mut self, unselected_label_style: TextStyle) -> TabBar {
        self.unselected_label_style = Some(unselected_label_style);
        self
    }

    /// Dart `TabBar(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> TabBar {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `TabBar(overlayColor:)`.
    pub fn overlay_color(mut self, overlay_color: WidgetStatePropertyRef<Option<Color>>) -> TabBar {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `TabBar(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: impl Into<MouseCursorRef>) -> TabBar {
        self.mouse_cursor = Some(mouse_cursor.into());
        self
    }

    /// Dart `TabBar(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> TabBar {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    /// Dart `TabBar(onTap:)`.
    pub fn on_tap(mut self, on_tap: impl Fn(&mut App, usize) + 'static) -> TabBar {
        self.on_tap = Some(Rc::new(on_tap));
        self
    }

    /// Dart `TabBar(onHover:)`.
    pub fn on_hover(mut self, on_hover: impl Fn(&mut App, bool, usize) + 'static) -> TabBar {
        self.on_hover = Some(Rc::new(on_hover));
        self
    }

    /// Dart `TabBar(onFocusChange:)`.
    pub fn on_focus_change(
        mut self,
        on_focus_change: impl Fn(&mut App, bool, usize) + 'static,
    ) -> TabBar {
        self.on_focus_change = Some(Rc::new(on_focus_change));
        self
    }

    /// Dart `TabBar(physics:)`.
    pub fn physics(mut self, physics: ScrollPhysicsRef) -> TabBar {
        self.physics = Some(physics);
        self
    }

    /// Dart `TabBar(splashFactory:)`.
    pub fn splash_factory(mut self, splash_factory: InteractiveInkFeatureFactoryRef) -> TabBar {
        self.splash_factory = Some(splash_factory);
        self
    }

    /// Dart `TabBar(splashBorderRadius:)`.
    pub fn splash_border_radius(mut self, splash_border_radius: BorderRadius) -> TabBar {
        self.splash_border_radius = Some(splash_border_radius);
        self
    }

    /// Dart `TabBar(tabAlignment:)`.
    pub fn tab_alignment(mut self, tab_alignment: TabAlignment) -> TabBar {
        self.tab_alignment = Some(tab_alignment);
        self
    }

    /// Dart `TabBar(textScaler:)`.
    pub fn text_scaler(mut self, text_scaler: TextScaler) -> TabBar {
        self.text_scaler = Some(text_scaler);
        self
    }

    /// Dart `TabBar(indicatorAnimation:)`.
    pub fn indicator_animation(mut self, indicator_animation: TabIndicatorAnimation) -> TabBar {
        self.indicator_animation = Some(indicator_animation);
        self
    }

    /// Returns whether the [`TabBar`] contains a tab with both text and icon.
    ///
    /// [`TabBar`] uses this to give uniform padding to all tabs in cases where
    /// there are some tabs with both text and icon and some which contain only
    /// text or icon.
    pub fn tab_has_text_and_icon(&self) -> bool {
        self.tabs
            .iter()
            .filter_map(preferred_size_of)
            .any(|size| size.height() == K_TEXT_AND_ICON_TAB_HEIGHT)
    }
}

impl PreferredSizeWidget for TabBar {
    /// A size whose height depends on if the tabs have both icons and text.
    ///
    /// `AppBar` uses this size to compute its own preferred size.
    fn preferred_size(&self) -> waft_widgets::AnySize {
        let mut max_height = K_TAB_HEIGHT;
        for item in &self.tabs {
            if let Some(size) = preferred_size_of(item) {
                let item_height = size.height();
                max_height = item_height.max(max_height);
            }
        }
        Size::from_height(max_height + self.indicator_weight).into()
    }
}

impl fmt::Debug for TabBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TabBar")
            .field("tabs", &self.tabs.len())
            .field("is_scrollable", &self.is_scrollable)
            .field("is_primary", &self.is_primary)
            .finish()
    }
}

impl StatefulWidget for TabBar {
    type State = TabBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn as_preferred_size_widget(&self) -> Option<&dyn PreferredSizeWidget> {
        Some(self)
    }

    fn create_state(&self) -> TabBarState {
        TabBarState {
            state: StateData::new(),
            internal_scroll_controller: None,
            controller: None,
            indicator_painter: None,
            current_index: None,
            tab_strip_width: 0.0,
            tab_keys: Rc::new(RefCell::new(Vec::new())),
            label_paddings: Rc::new(RefCell::new(Vec::new())),
            debug_has_scheduled_valid_tabs_count_check: false,
        }
    }
}

/// Dart's `_TabBarState`.
pub struct TabBarState {
    state: StateData<TabBar>,
    internal_scroll_controller: Option<Ref<TabBarScrollController>>,
    controller: Option<Ref<TabController>>,
    indicator_painter: Option<Rc<IndicatorPainter>>,
    current_index: Option<usize>,
    /// Dart's `late double _tabStripWidth`, set by the first layout.
    tab_strip_width: f64,
    /// Shared with the indicator painter, as Dart hands the painter this state's list.
    tab_keys: Rc<RefCell<Vec<GlobalKey>>>,
    /// Shared with the indicator painter, as Dart hands the painter this state's list.
    label_paddings: Rc<RefCell<Vec<EdgeInsetsGeometry>>>,
    debug_has_scheduled_valid_tabs_count_check: bool,
}

impl TabBarState {
    fn defaults(self: Handle<Self>, app: &mut App) -> Rc<dyn TabBarThemeDataGetters> {
        let context = self.context(app);
        let widget = self.widget(app);
        let (is_primary, is_scrollable) = (widget.is_primary, widget.is_scrollable);
        if Theme::of(app, context).use_material3 {
            if is_primary {
                Rc::new(TabsPrimaryDefaultsM3::new(context, is_scrollable))
            } else {
                Rc::new(TabsSecondaryDefaultsM3::new(context, is_scrollable))
            }
        } else {
            Rc::new(TabsDefaultsM2::new(context, is_scrollable))
        }
    }

    fn effective_scroll_controller(
        self: Handle<Self>,
        app: &mut App,
    ) -> Ref<TabBarScrollController> {
        if let Some(scroll_controller) = self.widget(app).scroll_controller.clone() {
            if let Some(internal) = app.get_mut(self).internal_scroll_controller.take() {
                internal.dispose(app);
            }

            return scroll_controller;
        }

        match app.get(self).internal_scroll_controller.clone() {
            Some(internal) => internal,
            None => {
                let internal = TabBarScrollController::new(app);
                app.get_mut(self).internal_scroll_controller = Some(internal.clone());
                internal
            }
        }
    }

    fn get_indicator(
        self: Handle<Self>,
        app: &mut App,
        indicator_size: TabBarIndicatorSize,
    ) -> DecorationRef {
        let context = self.context(app);
        let theme = Theme::of(app, context);
        let tab_bar_theme = TabBarTheme::of(app, context);

        if let Some(indicator) = &self.widget(app).indicator {
            return indicator.clone();
        }
        if let Some(indicator) = &tab_bar_theme.indicator {
            return indicator.clone();
        }
        let widget = self.widget(app);
        if cfg!(debug_assertions) && widget.indicator_weight <= 0.0 {
            panic!(
                "Invalid indicatorWeight for TabBar.\n\
                 The indicatorWeight must be greater than zero when the TabBar draws its \
                 default underline indicator, that is when no indicator is provided by the \
                 TabBar or the TabBarTheme.\n\
                 To fix this, set indicatorWeight to a value greater than zero, or provide an \
                 indicator with the TabBar.indicator or TabBarThemeData.indicator property."
            );
        }

        let (indicator_color, automatic_adjustment, indicator_weight, is_primary) = (
            widget.indicator_color,
            widget.automatic_indicator_color_adjustment,
            widget.indicator_weight,
            widget.is_primary,
        );
        let mut color = indicator_color
            .or(tab_bar_theme.indicator_color)
            .or_else(|| self.defaults(app).indicator_color(app))
            .expect("the tab defaults have an indicator color");
        // ThemeData tries to avoid this by having indicatorColor avoid being the
        // primaryColor. However, it's possible that the tab bar is on a
        // Material that isn't the primaryColor. In that case, if the indicator
        // color ends up matching the material's color, then this overrides it.
        // When that happens, automatic transitions of the theme will likely look
        // ugly as the indicator color suddenly snaps to white at one end, but it's
        // not clear how to avoid that any further.
        //
        // The material's color might be null (if it's a transparency). In that case
        // there's no good way for us to find out what the color is so we don't.
        //
        // TODO(xu-baolin): Remove automatic adjustment to white color indicator
        // with a better long-term solution.
        // https://github.com/flutter/flutter/pull/68171#pullrequestreview-517753917
        if automatic_adjustment
            && Some(color.to_argb32())
                == Material::maybe_of(app, context)
                    .and_then(|material| material.color(app))
                    .map(|material_color| material_color.to_argb32())
        {
            color = Colors::WHITE;
        }

        let effective_indicator_weight = if theme.use_material3 {
            indicator_weight.max(if is_primary {
                TabsPrimaryDefaultsM3::indicator_weight(indicator_size)
            } else {
                TabsSecondaryDefaultsM3::INDICATOR_WEIGHT
            })
        } else {
            indicator_weight
        };
        // Only Material 3 primary TabBar with label indicatorSize should be rounded.
        let primary_with_label_indicator = match indicator_size {
            TabBarIndicatorSize::Label => is_primary,
            TabBarIndicatorSize::Tab => false,
        };
        let effective_border_radius =
            (theme.use_material3 && primary_with_label_indicator).then(|| {
                BorderRadius::only(
                    waft_embedder::Radius::circular(effective_indicator_weight),
                    waft_embedder::Radius::circular(effective_indicator_weight),
                    waft_embedder::Radius::ZERO,
                    waft_embedder::Radius::ZERO,
                )
            });
        let mut indicator = UnderlineTabIndicator::new().border_side(
            // TODO(tahatesser): Make sure this value matches Material 3 Tabs spec
            // when `preferredSize`and `indicatorWeight` are updated to support Material 3
            // https://m3.material.io/components/tabs/specs#149a189f-9039-4195-99da-15c205d20e30,
            // https://github.com/flutter/flutter/issues/116136
            waft_painting::BorderSide::new(
                color,
                effective_indicator_weight,
                BorderStyle::Solid,
                waft_painting::BorderSide::STROKE_ALIGN_INSIDE,
            ),
        );
        indicator.border_radius = effective_border_radius;
        DecorationRef::new(indicator)
    }

    // If the TabBar is rebuilt with a new tab controller, the caller should
    // dispose the old one. In that case the old controller's animation will be
    // null and should not be accessed.
    fn controller_is_valid(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .controller
            .handle()
            .is_some_and(|controller| controller.animation(app).is_some())
    }

    fn animation_tick_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::handle_tab_controller_animation_tick)
    }

    fn tick_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::handle_tab_controller_tick)
    }

    fn update_tab_controller(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let new_controller = match self.widget(app).controller.clone() {
            Some(controller) => Some(controller),
            None => DefaultTabController::maybe_of(app, context),
        };
        if cfg!(debug_assertions) && new_controller.is_none() {
            panic!(
                "No TabController for TabBar.\n\
                 When creating a TabBar, you must either provide an explicit TabController \
                 using the \"controller\" property, or you must ensure that there is a \
                 DefaultTabController above the TabBar.\n\
                 In this case, there was neither an explicit controller nor a default \
                 controller."
            );
        }

        if new_controller == app.get(self).controller {
            return;
        }

        if self.controller_is_valid(app) {
            let controller = app.get(self).controller.handle().expect("checked above");
            let animation = controller.animation(app).expect("checked above");
            animation.remove_listener(app, &self.animation_tick_listener());
            controller.remove_listener(app, &self.tick_listener());
        }
        app.get_mut(self).controller = new_controller.clone();
        if let Some(controller) = new_controller {
            let animation = controller
                .animation(app)
                .expect("a new controller is not disposed");
            animation.add_listener(app, self.animation_tick_listener());
            controller.add_listener(app, self.tick_listener());
            app.get_mut(self).current_index = Some(controller.index(app));
        }
    }

    fn update_scroll_controller(
        self: Handle<Self>,
        app: &mut App,
        old_scroll_controller: Option<Ref<TabBarScrollController>>,
    ) {
        let scroll_controller = self.widget(app).scroll_controller.clone();
        if old_scroll_controller != scroll_controller
            && let Some(old_scroll_controller) = old_scroll_controller
        {
            app.get_mut(&old_scroll_controller).tab_bar_state = None;
        }
        if let Some(scroll_controller) = scroll_controller {
            if let Some(internal) = app.get(self).internal_scroll_controller.handle() {
                app.get_mut(internal).tab_bar_state = None;
            }
            app.get_mut(&scroll_controller).tab_bar_state = Some(self);
        } else {
            let internal = match app.get(self).internal_scroll_controller.clone() {
                Some(internal) => internal,
                None => {
                    let internal = TabBarScrollController::new(app);
                    app.get_mut(self).internal_scroll_controller = Some(internal.clone());
                    internal
                }
            };
            app.get_mut(&internal).tab_bar_state = Some(self);
        }
    }

    fn init_indicator_painter(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let theme = Theme::of(app, context);
        let tab_bar_theme = TabBarTheme::of(app, context);
        let widget = self.widget(app);
        let indicator_size = match widget.indicator_size.or(tab_bar_theme.indicator_size) {
            Some(indicator_size) => indicator_size,
            None => self
                .defaults(app)
                .indicator_size(app)
                .expect("the tab defaults have an indicator size"),
        };
        let widget = self.widget(app);
        let uses_default_primary_label_indicator = theme.use_material3
            && widget.is_primary
            && indicator_size == TabBarIndicatorSize::Label
            && widget.indicator.is_none()
            && tab_bar_theme.indicator.is_none();

        let old_painter = app.get_mut(self).indicator_painter.take();

        let default_tab_indicator_animation = match indicator_size {
            TabBarIndicatorSize::Label => TabIndicatorAnimation::Elastic,
            TabBarIndicatorSize::Tab => TabIndicatorAnimation::Linear,
        };

        let indicator_painter = if !self.controller_is_valid(app) {
            None
        } else {
            let indicator = self.get_indicator(app, indicator_size);
            let defaults = self.defaults(app);
            let widget = self.widget(app);
            let (divider_color, divider_height) = (widget.divider_color, widget.divider_height);
            let divider_color = divider_color
                .or(tab_bar_theme.divider_color)
                .or_else(|| defaults.divider_color(app));
            let divider_height = divider_height
                .or(tab_bar_theme.divider_height)
                .or_else(|| defaults.divider_height(app));
            let widget = self.widget(app);
            let arguments = IndicatorPainterArguments {
                controller: app.get(self).controller.clone().expect("checked valid"),
                indicator,
                indicator_size,
                minimum_indicator_width: uses_default_primary_label_indicator
                    .then_some(K_MIN_PRIMARY_TAB_INDICATOR_WIDTH),
                indicator_padding: widget.indicator_padding,
                tab_keys: Rc::clone(&app.get(self).tab_keys),
                // Passing old painter so that the constructor can copy some values from it.
                old: old_painter.clone(),
                label_paddings: Rc::clone(&app.get(self).label_paddings),
                divider_color,
                divider_height,
                show_divider: theme.use_material3 && !widget.is_scrollable,
                device_pixel_ratio: None,
                indicator_animation: widget
                    .indicator_animation
                    .or(tab_bar_theme.indicator_animation)
                    .unwrap_or(default_tab_indicator_animation),
                text_direction: TextDirection::Ltr,
            };
            let device_pixel_ratio = MediaQuery::device_pixel_ratio_of(app, context);
            let text_direction = Directionality::of(app, context);
            Some(IndicatorPainter::new(
                app,
                IndicatorPainterArguments {
                    device_pixel_ratio: Some(device_pixel_ratio),
                    text_direction,
                    ..arguments
                },
            ))
        };
        app.get_mut(self).indicator_painter = indicator_painter;

        if let Some(old_painter) = old_painter {
            old_painter.dispose(app);
        }
    }

    fn indicator_painter(self: Handle<Self>, app: &App) -> Rc<IndicatorPainter> {
        Rc::clone(
            app.get(self)
                .indicator_painter
                .as_ref()
                .expect("a tab bar with a valid controller has an indicator painter"),
        )
    }

    fn max_tab_index(self: Handle<Self>, app: &App) -> usize {
        self.indicator_painter(app).max_tab_index()
    }

    fn tab_scroll_offset(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        viewport_width: f64,
        min_extent: f64,
        max_extent: f64,
    ) -> f64 {
        if !self.widget(app).is_scrollable {
            return 0.0;
        }
        let mut tab_center = self.indicator_painter(app).center_of(index);
        let padding_start;
        let context = self.context(app);
        let padding = self.widget(app).padding;
        match Directionality::of(app, context) {
            TextDirection::Rtl => {
                padding_start = padding
                    .map(|padding| padding.resolve(Some(TextDirection::Rtl)).right)
                    .unwrap_or(0.0);
                tab_center = app.get(self).tab_strip_width - tab_center;
            }
            TextDirection::Ltr => {
                padding_start = padding
                    .map(|padding| padding.resolve(Some(TextDirection::Ltr)).left)
                    .unwrap_or(0.0);
            }
        }

        clamp_double(
            tab_center + padding_start - viewport_width / 2.0,
            min_extent,
            max_extent,
        )
    }

    fn tab_centered_scroll_offset(self: Handle<Self>, app: &mut App, index: usize) -> f64 {
        let position = self.effective_scroll_controller(app).position(app);

        let viewport_dimension = position.viewport_dimension(app);
        let min_scroll_extent = position.min_scroll_extent(app);
        let max_scroll_extent = position.max_scroll_extent(app);
        self.tab_scroll_offset(
            app,
            index,
            viewport_dimension,
            min_scroll_extent,
            max_scroll_extent,
        )
    }

    fn initial_scroll_offset(
        self: Handle<Self>,
        app: &mut App,
        viewport_width: f64,
        min_extent: f64,
        max_extent: f64,
    ) -> f64 {
        let current_index = app.get(self).current_index.expect("a controller is set");
        self.tab_scroll_offset(app, current_index, viewport_width, min_extent, max_extent)
    }

    fn scroll_to_current_index(self: Handle<Self>, app: &mut App) {
        let current_index = app.get(self).current_index.expect("a controller is set");
        let offset = self.tab_centered_scroll_offset(app, current_index);

        let scroll_controller = self.effective_scroll_controller(app);
        drop(scroll_controller.animate_to(app, offset, K_TAB_SCROLL_DURATION, Curves::ease()));
    }

    fn scroll_to_controller_value(self: Handle<Self>, app: &mut App) {
        let current_index = app.get(self).current_index.expect("a controller is set");
        let leading_position =
            (current_index > 0).then(|| self.tab_centered_scroll_offset(app, current_index - 1));
        let middle_position = self.tab_centered_scroll_offset(app, current_index);
        let trailing_position = (current_index < self.max_tab_index(app))
            .then(|| self.tab_centered_scroll_offset(app, current_index + 1));

        let controller = app
            .get(self)
            .controller
            .handle()
            .expect("a controller is set");
        let index = controller.index(app) as f64;
        let value = controller
            .animation(app)
            .expect("a valid controller has an animation")
            .value(app);
        let lerp = |a: f64, b: f64, t: f64| {
            lerp_double(Some(a), Some(b), t).expect("two values lerp to a value")
        };
        let delta = value - index;
        let offset = if delta == -1.0 {
            leading_position.unwrap_or(middle_position)
        } else if delta == 1.0 {
            trailing_position.unwrap_or(middle_position)
        } else if delta == 0.0 {
            middle_position
        } else if delta < 0.0 {
            match leading_position {
                None => middle_position,
                Some(leading_position) => lerp(middle_position, leading_position, index - value),
            }
        } else {
            match trailing_position {
                None => middle_position,
                Some(trailing_position) => lerp(middle_position, trailing_position, value - index),
            }
        };

        let scroll_controller = self.effective_scroll_controller(app);
        scroll_controller.jump_to(app, offset);
    }

    fn handle_tab_controller_animation_tick(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.mounted(app));
        let controller = app
            .get(self)
            .controller
            .handle()
            .expect("listening to a controller");
        if !controller.index_is_changing(app) && self.widget(app).is_scrollable {
            // Sync the TabBar's scroll position with the TabBarView's PageView.
            app.get_mut(self).current_index = Some(controller.index(app));
            self.scroll_to_controller_value(app);
        }
    }

    fn handle_tab_controller_tick(self: Handle<Self>, app: &mut App) {
        let controller = app
            .get(self)
            .controller
            .handle()
            .expect("listening to a controller");
        if Some(controller.index(app)) != app.get(self).current_index {
            app.get_mut(self).current_index = Some(controller.index(app));
            if self.widget(app).is_scrollable {
                self.scroll_to_current_index(app);
            }
        }
        self.set_state(app, |_state| {
            // Rebuild the tabs after a (potentially animated) index change
            // has completed.
        });
    }

    // Called each time layout completes.
    fn save_tab_offsets(
        self: Handle<Self>,
        app: &mut App,
        tab_offsets: Vec<f64>,
        text_direction: TextDirection,
        width: f64,
    ) {
        app.get_mut(self).tab_strip_width = width;
        if let Some(indicator_painter) = &app.get(self).indicator_painter {
            indicator_painter.save_tab_offsets(Some(tab_offsets), Some(text_direction));
        }
    }

    fn handle_tap(self: Handle<Self>, app: &mut App, index: usize) {
        debug_assert!(index < self.widget(app).tabs.len());
        let controller = app
            .get(self)
            .controller
            .handle()
            .expect("a tapped tab bar has a controller");
        controller.animate_to(app, index, None, None);
        if let Some(on_tap) = self.widget(app).on_tap.clone() {
            on_tap(app, index);
        }
    }

    fn build_styled_tab(
        self: Handle<Self>,
        app: &App,
        child: WidgetRef,
        is_selected: bool,
        animation: AnimationRef<f64>,
        defaults: &Rc<dyn TabBarThemeDataGetters>,
    ) -> WidgetRef {
        let widget = self.widget(app);
        TabStyle {
            animation,
            is_selected,
            is_primary: widget.is_primary,
            label_color: widget.label_color.clone(),
            unselected_label_color: widget.unselected_label_color,
            label_style: widget.label_style.clone(),
            unselected_label_style: widget.unselected_label_style.clone(),
            defaults: Rc::clone(defaults),
            child,
        }
        .into_widget()
    }

    fn debug_schedule_check_has_valid_tabs_count(self: Handle<Self>, app: &mut App) -> bool {
        if app.get(self).debug_has_scheduled_valid_tabs_count_check {
            return true;
        }
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _duration| {
                if !app.contains(self) {
                    return;
                }
                app.get_mut(self).debug_has_scheduled_valid_tabs_count_check = false;
                if !self.mounted(app) {
                    return;
                }
                let controller = app
                    .get(self)
                    .controller
                    .handle()
                    .expect("a mounted tab bar has a controller");
                let length = controller.length(app);
                let tab_count = self.widget(app).tabs.len();
                if cfg!(debug_assertions) && length != tab_count {
                    panic!(
                        "Controller's length property ({length}) does not match the number of \
                         tabs ({tab_count}) present in TabBar's tabs property."
                    );
                }
            }),
            None,
        );
        app.get_mut(self).debug_has_scheduled_valid_tabs_count_check = true;
        true
    }

    fn debug_tab_alignment_is_valid(
        self: Handle<Self>,
        app: &App,
        tab_alignment: TabAlignment,
    ) -> bool {
        if cfg!(debug_assertions) {
            let is_scrollable = self.widget(app).is_scrollable;
            if is_scrollable && tab_alignment == TabAlignment::Fill {
                panic!("{tab_alignment:?} is only valid for non-scrollable tab bars.");
            }
            if !is_scrollable
                && (tab_alignment == TabAlignment::Start
                    || tab_alignment == TabAlignment::StartOffset)
            {
                panic!("{tab_alignment:?} is only valid for scrollable tab bars.");
            }
        }
        true
    }
}

impl State for TabBarState {
    type Widget = TabBar;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        // If indicatorSize is TabIndicatorSize.label, _tabKeys[i] is used to find
        // the width of tab widget i. See _IndicatorPainter.indicatorRect().
        let tab_count = self.widget(app).tabs.len();
        let this = app.get(self);
        *this.tab_keys.borrow_mut() = (0..tab_count).map(|_| GlobalKey::new()).collect();
        *this.label_paddings.borrow_mut() = vec![EdgeInsetsGeometry::ZERO; tab_count];
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.update_scroll_controller(app, None);
        self.update_tab_controller(app);
        self.init_indicator_painter(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &TabBar) {
        let widget = self.widget(app);
        if widget.controller != old_widget.controller
            || widget.scroll_controller != old_widget.scroll_controller
        {
            self.update_scroll_controller(app, old_widget.scroll_controller.clone());
            self.update_tab_controller(app);
            self.init_indicator_painter(app);

            // Adjust scroll position.
            let scroll_controller = self.effective_scroll_controller(app);
            if scroll_controller.has_clients(app) {
                let position = scroll_controller.position(app);
                if let Some(position) = position.downcast::<TabBarScrollPosition>(app) {
                    position.mark_needs_pixels_correction(app);
                }
            }
        } else if widget.indicator_color != old_widget.indicator_color
            || widget.indicator_weight != old_widget.indicator_weight
            || widget.indicator_size != old_widget.indicator_size
            || widget.indicator_padding != old_widget.indicator_padding
            || widget.indicator != old_widget.indicator
            || widget.divider_color != old_widget.divider_color
            || widget.divider_height != old_widget.divider_height
            || widget.indicator_animation != old_widget.indicator_animation
        {
            self.init_indicator_painter(app);
        }

        let tab_count = self.widget(app).tabs.len();
        let this = app.get(self);
        let mut tab_keys = this.tab_keys.borrow_mut();
        let mut label_paddings = this.label_paddings.borrow_mut();
        if tab_count > tab_keys.len() {
            let delta = tab_count - tab_keys.len();
            tab_keys.extend((0..delta).map(|_| GlobalKey::new()));
            label_paddings.extend(std::iter::repeat_n(EdgeInsetsGeometry::ZERO, delta));
        } else if tab_count < tab_keys.len() {
            tab_keys.truncate(tab_count);
            // Dart removes `_labelPaddings` from the new tab count up to `_tabKeys.length`,
            // which it has just shortened to that count: nothing.
            let end = tab_keys.len();
            label_paddings.drain(tab_count..end);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.indicator_painter(app).dispose(app);
        if self.controller_is_valid(app) {
            let controller = app.get(self).controller.handle().expect("checked valid");
            let animation = controller.animation(app).expect("checked valid");
            animation.remove_listener(app, &self.animation_tick_listener());
            controller.remove_listener(app, &self.tick_listener());
        }
        app.get_mut(self).controller = None;
        if let Some(internal) = app.get(self).internal_scroll_controller.handle() {
            internal.dispose(app);
        }
        if let Some(scroll_controller) = self.widget(app).scroll_controller.clone() {
            app.get_mut(&scroll_controller).tab_bar_state = None;
        }
        // We don't own the _controller Animation, so it's not disposed here.
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material_localizations(app, context));
        debug_assert!(self.debug_schedule_check_has_valid_tabs_count(app));
        let theme = Theme::of(app, context);
        let tab_bar_theme = TabBarTheme::of(app, context);
        let defaults = self.defaults(app);
        let effective_tab_alignment = self
            .widget(app)
            .tab_alignment
            .or(tab_bar_theme.tab_alignment)
            .or_else(|| defaults.tab_alignment(app))
            .expect("the tab defaults have a tab alignment");
        debug_assert!(self.debug_tab_alignment_is_valid(app, effective_tab_alignment));

        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let controller = app
            .get(self)
            .controller
            .clone()
            .expect("a built tab bar has a controller");
        let indicator_weight = self.widget(app).indicator_weight;
        if controller.length(app) == 0 {
            return LimitedBox::new()
                .max_width(0.0)
                .child(
                    SizedBox::new()
                        .width(f64::INFINITY)
                        .height(K_TAB_HEIGHT + indicator_weight),
                )
                .into_widget();
        }

        let widget = self.widget(app);
        let tab_count = widget.tabs.len();
        let mut wrapped_tabs: Vec<WidgetRef> = (0..tab_count)
            .map(|index| {
                let widget = self.widget(app);
                let mut padding = widget
                    .label_padding
                    .or(tab_bar_theme.label_padding)
                    .unwrap_or(K_TAB_LABEL_PADDING.into());
                const VERTICAL_ADJUSTMENT: f64 = (K_TEXT_AND_ICON_TAB_HEIGHT - K_TAB_HEIGHT) / 2.0;

                let tab = &widget.tabs[index];
                if preferred_size_of(tab).is_some_and(|size| size.height() == K_TAB_HEIGHT)
                    && widget.tab_has_text_and_icon()
                {
                    padding = padding.add(EdgeInsetsGeometry::symmetric(VERTICAL_ADJUSTMENT, 0.0));
                }
                let this = app.get(self);
                this.label_paddings.borrow_mut()[index] = padding;
                let tab_key = this.tab_keys.borrow()[index].clone();

                Center::new()
                    .height_factor(1.0)
                    .child(
                        Padding::new(padding).child(
                            KeyedSubtree::new(widget.tabs[index].clone()).key(tab_key.into()),
                        ),
                    )
                    .into_widget()
            })
            .collect();

        // If the controller was provided by DefaultTabController and we're part
        // of a Hero (typically the AppBar), then we will not be able to find the
        // controller during a Hero transition. See https://github.com/flutter/flutter/issues/213.
        let previous_index = controller.previous_index(app);
        let current_index = app.get(self).current_index.expect("a controller is set");

        if controller.index_is_changing(app) {
            // The user tapped on a tab, the tab controller's animation is running.
            debug_assert!(current_index != previous_index);
            let animation: AnimationRef<f64> =
                ChangeAnimation::new(app, controller.clone()).into_erased();
            wrapped_tabs[current_index] = self.build_styled_tab(
                app,
                wrapped_tabs[current_index].clone(),
                true,
                animation.clone(),
                &defaults,
            );
            wrapped_tabs[previous_index] = self.build_styled_tab(
                app,
                wrapped_tabs[previous_index].clone(),
                false,
                animation,
                &defaults,
            );
        } else {
            // The user is dragging the TabBarView's PageView left or right.
            let tab_index = current_index;
            let center_animation: AnimationRef<f64> =
                DragAnimation::new(app, controller.clone(), tab_index).into_erased();
            wrapped_tabs[tab_index] = self.build_styled_tab(
                app,
                wrapped_tabs[tab_index].clone(),
                true,
                center_animation,
                &defaults,
            );
            if current_index > 0 {
                let tab_index = current_index - 1;
                let drag_animation: AnimationRef<f64> =
                    DragAnimation::new(app, controller.clone(), tab_index).into_erased();
                let previous_animation = ReverseAnimation::new(app, drag_animation).into_erased();
                wrapped_tabs[tab_index] = self.build_styled_tab(
                    app,
                    wrapped_tabs[tab_index].clone(),
                    false,
                    previous_animation,
                    &defaults,
                );
            }
            if current_index < tab_count - 1 {
                let tab_index = current_index + 1;
                let drag_animation: AnimationRef<f64> =
                    DragAnimation::new(app, controller.clone(), tab_index).into_erased();
                let next_animation = ReverseAnimation::new(app, drag_animation).into_erased();
                wrapped_tabs[tab_index] = self.build_styled_tab(
                    app,
                    wrapped_tabs[tab_index].clone(),
                    false,
                    next_animation,
                    &defaults,
                );
            }
        }

        // Add the tap handler to each tab. If the tab bar is not scrollable,
        // then give all of the tabs equal flexibility so that they each occupy
        // the same share of the tab bar's overall width.
        #[expect(
            clippy::needless_range_loop,
            reason = "Dart's indexed loop over the tabs"
        )]
        for index in 0..tab_count {
            let selected_state: WidgetStates = if Some(index) == app.get(self).current_index {
                WidgetStates::from([WidgetState::Selected])
            } else {
                WidgetStates::new()
            };

            let widget_ref = context.widget(app).clone();
            let widget = waft_widgets::downcast_widget::<TabBar>(&*widget_ref)
                .expect("a TabBarState builds its TabBar");
            let effective_mouse_cursor: MouseCursorRef = widget
                .mouse_cursor
                .as_ref()
                .and_then(|mouse_cursor| {
                    <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(
                        &Some(mouse_cursor.clone()),
                        &selected_state,
                    )
                })
                .or_else(|| {
                    tab_bar_theme
                        .mouse_cursor
                        .as_ref()
                        .and_then(|mouse_cursor| mouse_cursor.resolve(&selected_state))
                })
                .unwrap_or_else(|| WidgetStateMouseCursor::clickable().resolve(&selected_state));

            let default_overlay_states = selected_state.clone();
            let default_overlay_color = defaults.overlay_color(app);
            let default_overlay: WidgetStatePropertyRef<Option<Color>> =
                <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                    let mut effective_states = default_overlay_states.clone();
                    effective_states.extend(states.iter().copied());
                    default_overlay_color
                        .as_ref()
                        .and_then(|overlay_color| overlay_color.resolve(&effective_states))
                });
            let mut ink_well = InkWell::new()
                .mouse_cursor(effective_mouse_cursor)
                .on_tap(Listener::new(move |app: &mut App| {
                    self.handle_tap(app, index);
                }))
                .on_hover(Rc::new(move |app: &mut App, value: bool| {
                    if let Some(on_hover) = self.widget(app).on_hover.clone() {
                        on_hover(app, value, index);
                    }
                }))
                .on_focus_change(Rc::new(move |app: &mut App, value: bool| {
                    if let Some(on_focus_change) = self.widget(app).on_focus_change.clone() {
                        on_focus_change(app, value, index);
                    }
                }))
                .enable_feedback(widget.enable_feedback.unwrap_or(true))
                .overlay_color(
                    widget
                        .overlay_color
                        .clone()
                        .or_else(|| tab_bar_theme.overlay_color.clone())
                        .unwrap_or(default_overlay),
                );
            if let Some(splash_factory) = widget
                .splash_factory
                .clone()
                .or_else(|| tab_bar_theme.splash_factory.clone())
                .or_else(|| defaults.splash_factory(app))
            {
                ink_well = ink_well.splash_factory(splash_factory);
            }
            if let Some(border_radius) = widget
                .splash_border_radius
                .or(tab_bar_theme.splash_border_radius)
                .or_else(|| defaults.splash_border_radius(app))
            {
                ink_well = ink_well.border_radius(border_radius);
            }
            let selected = Some(index) == app.get(self).current_index;
            let mut tab_semantics = Semantics::new().selected(selected);
            if !K_IS_WEB {
                tab_semantics = tab_semantics
                    .label(localizations.tab_label(index as i32 + 1, tab_count as i32));
            }
            wrapped_tabs[index] = ink_well
                .child(
                    Padding::new(EdgeInsetsGeometry::only(0.0, 0.0, 0.0, indicator_weight)).child(
                        // This has to be wrapped above the Stack to override any role set by the child.
                        Semantics::new().role(SemanticsRole::Tab).child(
                            Stack::new().children([
                                wrapped_tabs[index].clone(),
                                tab_semantics.into_widget(),
                            ]),
                        ),
                    ),
                )
                .into_widget();
            wrapped_tabs[index] = MergeSemantics::new()
                .child(wrapped_tabs[index].clone())
                .into_widget();
            if !self.widget(app).is_scrollable && effective_tab_alignment == TabAlignment::Fill {
                wrapped_tabs[index] = Expanded::new(wrapped_tabs[index].clone()).into_widget();
            }
        }

        let on_perform_layout: LayoutCallback =
            Rc::new(move |app, tab_offsets, text_direction, width| {
                self.save_tab_offsets(app, tab_offsets, text_direction, width);
            });
        let always_dismissed = k_always_dismissed_animation(app);
        let widget = self.widget(app);
        let mut indicator_paint = CustomPaint::new().child(TabStyle {
            animation: always_dismissed,
            is_selected: false,
            is_primary: widget.is_primary,
            label_color: widget.label_color.clone(),
            unselected_label_color: widget.unselected_label_color,
            label_style: widget.label_style.clone(),
            unselected_label_style: widget.unselected_label_style.clone(),
            defaults: Rc::clone(&defaults),
            child: TabLabelBar::new(
                wrapped_tabs,
                on_perform_layout,
                if effective_tab_alignment == TabAlignment::Fill {
                    MainAxisSize::Max
                } else {
                    MainAxisSize::Min
                },
            )
            .into_widget(),
        });
        indicator_paint.painter = app
            .get(self)
            .indicator_painter
            .clone()
            .map(|painter| painter as Rc<dyn CustomPainter>);
        let mut tab_bar = Semantics::new()
            .role(SemanticsRole::TabBar)
            .container(true)
            .explicit_child_nodes(true)
            .child(indicator_paint)
            .into_widget();

        let widget = self.widget(app);
        if widget.is_scrollable {
            let effective_padding = if effective_tab_alignment == TabAlignment::StartOffset {
                Some(
                    EdgeInsetsGeometry::from(EdgeInsetsDirectional::only(
                        K_START_OFFSET,
                        0.0,
                        0.0,
                        0.0,
                    ))
                    .add(widget.padding.unwrap_or(EdgeInsetsGeometry::ZERO)),
                )
            } else {
                widget.padding
            };

            let (drag_start_behavior, physics) =
                (widget.drag_start_behavior, widget.physics.clone());
            let scroll_behavior = ScrollConfiguration::of(app, context);
            let scroll_controller = self.effective_scroll_controller(app);
            let mut scroll_view = SingleChildScrollView::new()
                .drag_start_behavior(drag_start_behavior)
                .scroll_direction(Axis::Horizontal)
                .controller(scroll_controller.erase());
            if let Some(effective_padding) = effective_padding {
                scroll_view = scroll_view.padding(effective_padding);
            }
            if let Some(physics) = physics {
                scroll_view = scroll_view.physics(physics);
            }
            tab_bar = ScrollConfiguration::new(
                // The scrolling tabs should not show an overscroll indicator.
                waft_widgets::ScrollBehaviorRef::new(scroll_behavior.copy_with().overscroll(false)),
                scroll_view.child(tab_bar),
            )
            .into_widget();
            if theme.use_material3 {
                let effective_alignment = match effective_tab_alignment {
                    TabAlignment::Center => AlignmentGeometry::CENTER,
                    TabAlignment::Start | TabAlignment::StartOffset | TabAlignment::Fill => {
                        AlignmentGeometry::CENTER_START
                    }
                };

                let widget = self.widget(app);
                let (divider_color, divider_height) = (widget.divider_color, widget.divider_height);
                let divider_color = divider_color
                    .or(tab_bar_theme.divider_color)
                    .or_else(|| defaults.divider_color(app))
                    .expect("the Material 3 tab defaults have a divider color");
                let divider_height = divider_height
                    .or(tab_bar_theme.divider_height)
                    .or_else(|| defaults.divider_height(app))
                    .expect("the Material 3 tab defaults have a divider height");

                let mut align = Align::new()
                    .height_factor(1.0)
                    .alignment(effective_alignment);
                if divider_height <= 0.0 {
                    align = align.width_factor(1.0);
                }
                tab_bar = align.child(tab_bar).into_widget();

                if divider_color != Colors::TRANSPARENT && divider_height > 0.0 {
                    tab_bar = CustomPaint::new()
                        .painter(DividerPainter {
                            divider_color,
                            divider_height,
                        })
                        .child(tab_bar)
                        .into_widget();
                }
            }
        } else if let Some(padding) = widget.padding {
            tab_bar = Padding::new(padding).child(tab_bar).into_widget();
        }

        let mut media_query_data = MediaQuery::of(app, context).copy_with();
        if let Some(text_scaler) = self
            .widget(app)
            .text_scaler
            .clone()
            .or_else(|| tab_bar_theme.text_scaler.clone())
        {
            media_query_data = media_query_data.text_scaler(text_scaler);
        }
        Material::new()
            .r#type(MaterialType::Transparency)
            .child(MediaQuery::new(media_query_data, tab_bar).into_widget())
            .into_widget()
    }
}

/// A page view that displays the widget which corresponds to the currently
/// selected tab.
///
/// This widget is typically used in conjunction with a [`TabBar`].
///
/// If a [`TabController`] is not provided, then there must be a [`DefaultTabController`]
/// ancestor.
///
/// The tab controller's [`TabController::length`] must equal the length of the
/// [`children`](Self::children) list and the length of the [`TabBar::tabs`] list.
///
/// To see a sample implementation, visit the [`TabController`] documentation.
pub struct TabBarView {
    pub key: Option<KeyRef>,

    /// This widget's selection and animation state.
    ///
    /// If [`TabController`] is not provided, then the value of [`DefaultTabController::of`]
    /// will be used.
    pub controller: Option<Ref<TabController>>,

    /// One widget per tab.
    ///
    /// Its length must match the length of the [`TabBar::tabs`]
    /// list, as well as the [`controller`](Self::controller)'s [`TabController::length`].
    pub children: Vec<WidgetRef>,

    /// How the page view should respond to user input.
    ///
    /// For example, determines how the page view continues to animate after the
    /// user stops dragging the page view.
    ///
    /// The physics are modified to snap to page boundaries using
    /// [`PageScrollPhysics`] prior to being used.
    ///
    /// Defaults to matching platform conventions.
    pub physics: Option<ScrollPhysicsRef>,

    /// Determines the way that drag start behavior is handled.
    pub drag_start_behavior: DragStartBehavior,

    /// The fraction of the viewport that each page should occupy.
    ///
    /// Defaults to 1.0, which means each page fills the viewport in the scrolling
    /// direction.
    pub viewport_fraction: f64,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,
}

impl TabBarView {
    /// Creates a page view with one child per tab.
    ///
    /// The length of [`children`](Self::children) must be the same as the
    /// [`controller`](Self::controller)'s length.
    pub fn new(children: impl IntoIterator<Item = WidgetRef>) -> TabBarView {
        TabBarView {
            key: None,
            controller: None,
            children: children.into_iter().collect(),
            physics: None,
            drag_start_behavior: DragStartBehavior::Start,
            viewport_fraction: 1.0,
            clip_behavior: Clip::HardEdge,
        }
    }

    /// Dart `TabBarView(key:)`.
    pub fn key(mut self, key: KeyRef) -> TabBarView {
        self.key = Some(key);
        self
    }

    /// Dart `TabBarView(controller:)`.
    pub fn controller(mut self, controller: Ref<TabController>) -> TabBarView {
        self.controller = Some(controller);
        self
    }

    /// Dart `TabBarView(physics:)`.
    pub fn physics(mut self, physics: ScrollPhysicsRef) -> TabBarView {
        self.physics = Some(physics);
        self
    }

    /// Dart `TabBarView(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> TabBarView {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `TabBarView(viewportFraction:)`.
    pub fn viewport_fraction(mut self, viewport_fraction: f64) -> TabBarView {
        self.viewport_fraction = viewport_fraction;
        self
    }

    /// Dart `TabBarView(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> TabBarView {
        self.clip_behavior = clip_behavior;
        self
    }
}

impl fmt::Debug for TabBarView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TabBarView")
            .field("children", &self.children.len())
            .field("viewport_fraction", &self.viewport_fraction)
            .finish()
    }
}

impl StatefulWidget for TabBarView {
    type State = TabBarViewState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TabBarViewState {
        TabBarViewState {
            state: StateData::new(),
            controller: None,
            page_controller: None,
            children_with_key: Vec::new(),
            current_index: None,
            warp_underway_count: 0,
            scroll_underway_count: 0,
            debug_has_scheduled_valid_children_count_check: false,
        }
    }
}

/// Dart's `_TabBarViewState`.
pub struct TabBarViewState {
    state: StateData<TabBarView>,
    controller: Option<Ref<TabController>>,
    page_controller: Option<Ref<PageController>>,
    children_with_key: Vec<WidgetRef>,
    current_index: Option<usize>,
    warp_underway_count: i32,
    scroll_underway_count: i32,
    debug_has_scheduled_valid_children_count_check: bool,
}

impl TabBarViewState {
    // If the TabBarView is rebuilt with a new tab controller, the caller should
    // dispose the old one. In that case the old controller's animation will be
    // null and should not be accessed.
    fn controller_is_valid(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .controller
            .handle()
            .is_some_and(|controller| controller.animation(app).is_some())
    }

    fn controller(self: Handle<Self>, app: &App) -> Handle<TabController> {
        app.get(self)
            .controller
            .handle()
            .expect("a tab bar view has a controller")
    }

    fn page_controller(self: Handle<Self>, app: &App) -> Handle<PageController> {
        app.get(self)
            .page_controller
            .handle()
            .expect("made in did_change_dependencies")
    }

    fn animation_tick_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::handle_tab_controller_animation_tick)
    }

    fn update_tab_controller(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let new_controller = match self.widget(app).controller.clone() {
            Some(controller) => Some(controller),
            None => DefaultTabController::maybe_of(app, context),
        };
        if cfg!(debug_assertions) && new_controller.is_none() {
            panic!(
                "No TabController for TabBarView.\n\
                 When creating a TabBarView, you must either provide an explicit TabController \
                 using the \"controller\" property, or you must ensure that there is a \
                 DefaultTabController above the TabBarView.\n\
                 In this case, there was neither an explicit controller nor a default \
                 controller."
            );
        }

        if new_controller == app.get(self).controller {
            return;
        }

        if self.controller_is_valid(app) {
            let animation = self.controller(app).animation(app).expect("checked valid");
            animation.remove_listener(app, &self.animation_tick_listener());
        }
        app.get_mut(self).controller = new_controller.clone();
        if let Some(controller) = new_controller {
            let animation = controller
                .animation(app)
                .expect("a new controller is not disposed");
            animation.add_listener(app, self.animation_tick_listener());
        }
    }

    fn jump_to_page(self: Handle<Self>, app: &mut App, page: usize) {
        app.get_mut(self).warp_underway_count += 1;
        self.page_controller(app).jump_to_page(app, page as i32);
        app.get_mut(self).warp_underway_count -= 1;
    }

    fn animate_to_page(
        self: Handle<Self>,
        app: &mut App,
        page: usize,
        duration: Duration,
        curve: Rc<dyn waft_animation::Curve>,
    ) -> Task<()> {
        app.get_mut(self).warp_underway_count += 1;
        let animated = self
            .page_controller(app)
            .animate_to_page(app, page as i32, duration, curve);
        app.spawn(async move |cx| {
            animated.await;
            cx.update(|app| {
                if app.contains(self) {
                    app.get_mut(self).warp_underway_count -= 1;
                }
            });
        })
    }

    fn update_children(self: Handle<Self>, app: &mut App) {
        let children = self
            .widget(app)
            .children
            .iter()
            .map(|child| {
                Semantics::new()
                    .role(SemanticsRole::TabPanel)
                    .child(child.clone())
                    .into_widget()
            })
            .collect();
        app.get_mut(self).children_with_key =
            KeyedSubtree::ensure_unique_keys_for_list(children, 0);
    }

    fn handle_tab_controller_animation_tick(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        if app.get(self).scroll_underway_count > 0 || !controller.index_is_changing(app) {
            return;
        } // This widget is driving the controller's animation.

        if Some(controller.index(app)) != app.get(self).current_index {
            app.get_mut(self).current_index = Some(controller.index(app));
            drop(self.warp_to_current_index(app));
        }
    }

    fn warp_to_current_index(self: Handle<Self>, app: &mut App) -> Task<()> {
        let current_index = app.get(self).current_index.expect("a controller is set");
        if !self.mounted(app) || self.page_controller(app).page(app) == Some(current_index as f64) {
            return Task::ready(());
        }

        let controller = self.controller(app);
        let adjacent_destination =
            (current_index as i64 - controller.previous_index(app) as i64).abs() == 1;
        let animation_duration = controller.animation_duration(app);
        if adjacent_destination {
            self.warp_to_adjacent_tab(app, animation_duration)
        } else {
            self.warp_to_non_adjacent_tab(app, animation_duration)
        }
    }

    fn warp_to_adjacent_tab(self: Handle<Self>, app: &mut App, duration: Duration) -> Task<()> {
        let current_index = app.get(self).current_index.expect("a controller is set");
        let animated = if duration == Duration::ZERO {
            self.jump_to_page(app, current_index);
            Task::ready(())
        } else {
            self.animate_to_page(app, current_index, duration, Curves::ease())
        };
        app.spawn(async move |cx| {
            animated.await;
            cx.update(|app| {
                if self.mounted(app) {
                    self.update_children(app);
                    self.set_state(app, |_state| {});
                }
            });
        })
    }

    fn warp_to_non_adjacent_tab(self: Handle<Self>, app: &mut App, duration: Duration) -> Task<()> {
        let previous_index = self.controller(app).previous_index(app);
        let current_index = app.get(self).current_index.expect("a controller is set");
        debug_assert!((current_index as i64 - previous_index as i64).abs() > 1);

        // initialPage defines which page is shown when starting the animation.
        // This page is adjacent to the destination page.
        let initial_page = if current_index > previous_index {
            current_index - 1
        } else {
            current_index + 1
        };

        self.set_state(app, |state| {
            // Needed for `RenderSliverMultiBoxAdaptor.move` and kept alive children.
            // For motivation, see https://github.com/flutter/flutter/pull/29188 and
            // https://github.com/flutter/flutter/issues/27010#issuecomment-486475152.
            state.children_with_key.swap(initial_page, previous_index);
        });

        // Make a first jump to the adjacent page.
        self.jump_to_page(app, initial_page);

        // Jump or animate to the destination page.
        let animated = if duration == Duration::ZERO {
            self.jump_to_page(app, current_index);
            Task::ready(())
        } else {
            self.animate_to_page(app, current_index, duration, Curves::ease())
        };

        app.spawn(async move |cx| {
            animated.await;
            cx.update(|app| {
                if self.mounted(app) {
                    self.update_children(app);
                    self.set_state(app, |_state| {});
                }
            });
        })
    }

    fn sync_controller_offset(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        let page = self
            .page_controller(app)
            .page(app)
            .expect("a scrolled page view has a page");
        let offset = clamp_double(page - controller.index(app) as f64, -1.0, 1.0);
        controller.set_offset(app, offset);
    }

    // Called when the PageView scrolls
    fn handle_scroll_notification(
        self: Handle<Self>,
        app: &mut App,
        notification: &dyn ScrollNotification,
    ) -> bool {
        let this = app.get(self);
        if this.warp_underway_count > 0 || this.scroll_underway_count > 0 {
            return false;
        }

        if notification.depth() != 0 {
            return false;
        }

        if !self.controller_is_valid(app) {
            return false;
        }

        app.get_mut(self).scroll_underway_count += 1;
        let page = self
            .page_controller(app)
            .page(app)
            .expect("a scrolled page view has a page");
        let controller = self.controller(app);
        if notification.as_any().is::<ScrollUpdateNotification>()
            && !controller.index_is_changing(app)
        {
            let page_changed = (page - controller.index(app) as f64).abs() > 1.0;
            if page_changed {
                controller.set_index(app, page.round() as usize);
                app.get_mut(self).current_index = Some(controller.index(app));
            }
            self.sync_controller_offset(app);
        } else if notification.as_any().is::<ScrollEndNotification>() {
            controller.set_index(app, page.round() as usize);
            app.get_mut(self).current_index = Some(controller.index(app));
            if !controller.index_is_changing(app) {
                self.sync_controller_offset(app);
            }
        }
        app.get_mut(self).scroll_underway_count -= 1;

        false
    }

    fn debug_schedule_check_has_valid_children_count(self: Handle<Self>, app: &mut App) -> bool {
        if app.get(self).debug_has_scheduled_valid_children_count_check {
            return true;
        }
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _duration| {
                if !app.contains(self) {
                    return;
                }
                app.get_mut(self)
                    .debug_has_scheduled_valid_children_count_check = false;
                if !self.mounted(app) {
                    return;
                }
                let length = self.controller(app).length(app);
                let child_count = self.widget(app).children.len();
                if cfg!(debug_assertions) && length != child_count {
                    panic!(
                        "Controller's length property ({length}) does not match the number of \
                         children ({child_count}) present in TabBarView's children property."
                    );
                }
            }),
            None,
        );
        app.get_mut(self)
            .debug_has_scheduled_valid_children_count_check = true;
        true
    }
}

impl State for TabBarViewState {
    type Widget = TabBarView;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.update_children(app);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.update_tab_controller(app);
        let current_index = self.controller(app).index(app);
        app.get_mut(self).current_index = Some(current_index);
        match app.get(self).page_controller.handle() {
            None => {
                let viewport_fraction = self.widget(app).viewport_fraction;
                let page_controller = PageController::new(
                    app,
                    current_index as i32,
                    true,
                    viewport_fraction,
                    None,
                    None,
                );
                app.get_mut(self).page_controller = Some(page_controller);
            }
            Some(page_controller) => page_controller.jump_to_page(app, current_index as i32),
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &TabBarView) {
        if self.widget(app).controller != old_widget.controller {
            self.update_tab_controller(app);
            let current_index = self.controller(app).index(app);
            app.get_mut(self).current_index = Some(current_index);
            self.jump_to_page(app, current_index);
        }
        let viewport_fraction = self.widget(app).viewport_fraction;
        if viewport_fraction != old_widget.viewport_fraction {
            if let Some(page_controller) = app.get_mut(self).page_controller.take() {
                page_controller.dispose(app);
            }
            let current_index = app.get(self).current_index.expect("a controller is set");
            let page_controller = PageController::new(
                app,
                current_index as i32,
                true,
                viewport_fraction,
                None,
                None,
            );
            app.get_mut(self).page_controller = Some(page_controller);
        }
        // While a warp is under way, we stop updating the tab page contents.
        // This is tracked in https://github.com/flutter/flutter/issues/31269.
        let widget = self.widget(app);
        // Dart compares the two lists by identity; a rebuilt list of the same widgets is the
        // same list here.
        let children_changed = widget.children.len() != old_widget.children.len()
            || widget
                .children
                .iter()
                .zip(&old_widget.children)
                .any(|(child, old_child)| !Rc::ptr_eq(child, old_child));
        if children_changed && app.get(self).warp_underway_count == 0 {
            self.update_children(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if self.controller_is_valid(app) {
            let animation = self.controller(app).animation(app).expect("checked valid");
            animation.remove_listener(app, &self.animation_tick_listener());
        }
        app.get_mut(self).controller = None;
        if let Some(page_controller) = app.get(self).page_controller.handle() {
            page_controller.dispose(app);
        }
        // We don't own the _controller Animation, so it's not disposed here.
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        debug_assert!(self.debug_schedule_check_has_valid_children_count(app));

        let widget = self.widget(app);
        let physics = match widget.physics.clone() {
            None => PageScrollPhysics::new().apply_to(Some(Rc::new(ClampingScrollPhysics::new()))),
            Some(physics) => PageScrollPhysics::new().apply_to(Some(physics)),
        };
        let page_view = PageView::new()
            .drag_start_behavior(widget.drag_start_behavior)
            .clip_behavior(widget.clip_behavior)
            .controller(
                app.get(self)
                    .page_controller
                    .clone()
                    .expect("made in did_change_dependencies"),
            )
            .physics(physics)
            .children(app.get(self).children_with_key.clone());
        NotificationListener::<dyn ScrollNotification>::new(page_view)
            .on_notification(
                move |app: &mut App, notification: &dyn ScrollNotification| {
                    self.handle_scroll_notification(app, notification)
                },
            )
            .into_widget()
    }
}

/// Displays a single circle with the specified size, border style, border color
/// and background colors.
///
/// Used by [`TabPageSelector`] to indicate the selected page.
#[derive(Debug)]
pub struct TabPageSelectorIndicator {
    pub key: Option<KeyRef>,

    /// The indicator circle's background color.
    pub background_color: Color,

    /// The indicator circle's border color.
    pub border_color: Color,

    /// The indicator circle's diameter.
    pub size: f64,

    /// The indicator circle's border style.
    ///
    /// Defaults to [`BorderStyle::Solid`] if value is not specified.
    pub border_style: BorderStyle,
}

impl TabPageSelectorIndicator {
    /// Creates an indicator used by [`TabPageSelector`].
    pub fn new(
        background_color: Color,
        border_color: Color,
        size: f64,
    ) -> TabPageSelectorIndicator {
        TabPageSelectorIndicator {
            key: None,
            background_color,
            border_color,
            size,
            border_style: BorderStyle::Solid,
        }
    }

    /// Dart `TabPageSelectorIndicator(key:)`.
    pub fn key(mut self, key: KeyRef) -> TabPageSelectorIndicator {
        self.key = Some(key);
        self
    }

    /// Dart `TabPageSelectorIndicator(borderStyle:)`.
    pub fn border_style(mut self, border_style: BorderStyle) -> TabPageSelectorIndicator {
        self.border_style = border_style;
        self
    }
}

impl StatelessWidget for TabPageSelectorIndicator {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        Container::new()
            .width(self.size)
            .height(self.size)
            .margin(EdgeInsetsGeometry::all(4.0))
            .decoration(
                BoxDecoration::new()
                    .color(self.background_color)
                    .border(waft_painting::Border::all(
                        self.border_color,
                        1.0,
                        self.border_style,
                        waft_painting::BorderSide::STROKE_ALIGN_INSIDE,
                    ))
                    .shape(BoxShape::Circle),
            )
            .into_widget()
    }
}

/// Uses [`TabPageSelectorIndicator`] to display a row of small circular
/// indicators, one per tab.
///
/// The selected tab's indicator is highlighted. Often used in conjunction with
/// a [`TabBarView`].
///
/// If a [`TabController`] is not provided, then there must be a
/// [`DefaultTabController`] ancestor.
#[derive(Debug)]
pub struct TabPageSelector {
    pub key: Option<KeyRef>,

    /// This widget's selection and animation state.
    ///
    /// If [`TabController`] is not provided, then the value of
    /// [`DefaultTabController::of`] will be used.
    pub controller: Option<Ref<TabController>>,

    /// The indicator circle's diameter (the default value is 12.0).
    pub indicator_size: f64,

    /// The indicator circle's fill color for unselected pages.
    ///
    /// If this parameter is null, then the indicator is filled with `Colors::TRANSPARENT`.
    pub color: Option<Color>,

    /// The indicator circle's fill color for selected pages and border color
    /// for all indicator circles.
    ///
    /// If this parameter is null, then the indicator is filled with the theme's
    /// `ColorScheme::secondary`.
    pub selected_color: Option<Color>,

    /// The indicator circle's border style.
    ///
    /// Defaults to [`BorderStyle::Solid`] if value is not specified.
    pub border_style: Option<BorderStyle>,
}

impl Default for TabPageSelector {
    fn default() -> TabPageSelector {
        TabPageSelector {
            key: None,
            controller: None,
            indicator_size: 12.0,
            color: None,
            selected_color: None,
            border_style: None,
        }
    }
}

impl TabPageSelector {
    /// Creates a compact widget that indicates which tab has been selected; Dart's named
    /// arguments are the setters.
    pub fn new() -> TabPageSelector {
        TabPageSelector::default()
    }

    /// Dart `TabPageSelector(key:)`.
    pub fn key(mut self, key: KeyRef) -> TabPageSelector {
        self.key = Some(key);
        self
    }

    /// Dart `TabPageSelector(controller:)`.
    pub fn controller(mut self, controller: Ref<TabController>) -> TabPageSelector {
        self.controller = Some(controller);
        self
    }

    /// Dart `TabPageSelector(indicatorSize:)`.
    pub fn indicator_size(mut self, indicator_size: f64) -> TabPageSelector {
        debug_assert!(indicator_size > 0.0);
        self.indicator_size = indicator_size;
        self
    }

    /// Dart `TabPageSelector(color:)`.
    pub fn color(mut self, color: Color) -> TabPageSelector {
        self.color = Some(color);
        self
    }

    /// Dart `TabPageSelector(selectedColor:)`.
    pub fn selected_color(mut self, selected_color: Color) -> TabPageSelector {
        self.selected_color = Some(selected_color);
        self
    }

    /// Dart `TabPageSelector(borderStyle:)`.
    pub fn border_style(mut self, border_style: BorderStyle) -> TabPageSelector {
        self.border_style = Some(border_style);
        self
    }
}

impl StatefulWidget for TabPageSelector {
    type State = TabPageSelectorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TabPageSelectorState {
        TabPageSelectorState {
            state: StateData::new(),
            previous_tab_controller: None,
            animation: None,
        }
    }
}

/// Dart's `_TabPageSelectorState`.
pub struct TabPageSelectorState {
    state: StateData<TabPageSelector>,
    previous_tab_controller: Option<Ref<TabController>>,
    animation: Option<Ref<CurvedAnimation>>,
}

impl TabPageSelectorState {
    fn tab_controller(self: Handle<Self>, app: &mut App) -> Ref<TabController> {
        let context = self.context(app);
        let tab_controller = match self.widget(app).controller.clone() {
            Some(controller) => Some(controller),
            None => DefaultTabController::maybe_of(app, context),
        };
        if cfg!(debug_assertions) && tab_controller.is_none() {
            panic!(
                "No TabController for TabPageSelectorState.\n\
                 When creating a TabPageSelectorState, you must either provide an explicit \
                 TabController using the \"controller\" property, or you must ensure that there \
                 is a DefaultTabController above the TabPageSelectorState.\n\
                 In this case, there was neither an explicit controller nor a default \
                 controller."
            );
        }
        tab_controller.expect("asserted above")
    }

    fn set_animation(self: Handle<Self>, app: &mut App) {
        if let Some(animation) = app.get_mut(self).animation.take() {
            animation.dispose(app);
        }
        let parent = self
            .tab_controller(app)
            .animation(app)
            .expect("a selector's controller is valid")
            .keep(app);
        let animation = CurvedAnimation::new(app, parent, Curves::fast_out_slow_in(), None);
        app.get_mut(self).animation = Some(animation);
    }

    fn build_tab_indicator(
        self: Handle<Self>,
        app: &App,
        tab_index: usize,
        tab_controller: Handle<TabController>,
        selected_color_tween: &Ref<ColorTween>,
        previous_color_tween: &Ref<ColorTween>,
    ) -> WidgetRef {
        let background = if tab_controller.index_is_changing(app) {
            // The selection's animation is animating from previousValue to value.
            let t = 1.0 - index_change_progress(app, tab_controller);
            if tab_controller.index(app) == tab_index {
                selected_color_tween.lerp(app, t)
            } else if tab_controller.previous_index(app) == tab_index {
                previous_color_tween.lerp(app, t)
            } else {
                selected_color_tween.begin(app)
            }
        } else {
            // The selection's offset reflects how far the TabBarView has / been dragged
            // to the previous page (-1.0 to 0.0) or the next page (0.0 to 1.0).
            let offset = tab_controller.offset(app);
            let index = tab_controller.index(app);
            if index == tab_index {
                selected_color_tween.lerp(app, 1.0 - offset.abs())
            } else if index + 1 == tab_index && offset > 0.0 {
                selected_color_tween.lerp(app, offset)
            } else if index == tab_index + 1 && offset < 0.0 {
                selected_color_tween.lerp(app, -offset)
            } else {
                selected_color_tween.begin(app)
            }
        }
        .expect("two colors lerp to a color");
        let widget = self.widget(app);
        TabPageSelectorIndicator::new(
            background,
            selected_color_tween
                .end(app)
                .expect("the selected color tween has an end"),
            widget.indicator_size,
        )
        .border_style(widget.border_style.unwrap_or(BorderStyle::Solid))
        .into_widget()
    }
}

impl State for TabPageSelectorState {
    type Widget = TabPageSelector;
    waft_widgets::state_accessors!();

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &TabPageSelector) {
        let tab_controller = self.tab_controller(app);
        let previous_animation = app
            .get(self)
            .previous_tab_controller
            .handle()
            .and_then(|controller| controller.animation(app));
        if previous_animation != tab_controller.animation(app) {
            self.set_animation(app);
        }
        if app.get(self).previous_tab_controller.as_ref() != Some(&tab_controller) {
            app.get_mut(self).previous_tab_controller = Some(tab_controller);
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let tab_controller = self.tab_controller(app);
        let previous_animation = app
            .get(self)
            .previous_tab_controller
            .handle()
            .and_then(|controller| controller.animation(app));
        if app.get(self).animation.is_none() || previous_animation != tab_controller.animation(app)
        {
            self.set_animation(app);
        }
        if app.get(self).previous_tab_controller.as_ref() != Some(&tab_controller) {
            app.get_mut(self).previous_tab_controller = Some(tab_controller);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(animation) = app.get_mut(self).animation.take() {
            animation.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let (color, selected_color) = (widget.color, widget.selected_color);
        let fix_color = color.unwrap_or(Colors::TRANSPARENT);
        let fix_selected_color =
            selected_color.unwrap_or_else(|| Theme::of(app, context).color_scheme.secondary);
        let selected_color_tween = ColorTween::new(app, Some(fix_color), Some(fix_selected_color));
        let previous_color_tween = ColorTween::new(app, Some(fix_selected_color), Some(fix_color));
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let animation: AnimationRef<f64> = app
            .get(self)
            .animation
            .as_ref()
            .expect("made in did_change_dependencies")
            .erase();
        AnimatedBuilder::new(Rc::new(animation), move |app, _context, _child| {
            let tab_controller = self.tab_controller(app);
            let length = tab_controller.length(app);
            Semantics::new()
                .label(localizations.tab_label(tab_controller.index(app) as i32 + 1, length as i32))
                .child(
                    Row::new().main_axis_size(MainAxisSize::Min).children(
                        (0..length)
                            .map(|tab_index| {
                                self.build_tab_indicator(
                                    app,
                                    tab_index,
                                    tab_controller.handle(),
                                    &selected_color_tween,
                                    &previous_color_tween,
                                )
                            })
                            .collect::<Vec<_>>(),
                    ),
                )
                .into_widget()
        })
        .into_widget()
    }
}

/// Dart's `_TabsDefaultsM2`: hand coded defaults based on Material Design 2.
pub(crate) struct TabsDefaultsM2 {
    /// Dart's `super(..)` arguments.
    tab_bar_theme_data: TabBarThemeData,
    context: DefaultsContext,
    is_scrollable: bool,
}

impl TabsDefaultsM2 {
    pub(crate) fn new(context: BuildContext, is_scrollable: bool) -> TabsDefaultsM2 {
        TabsDefaultsM2 {
            tab_bar_theme_data: TabBarThemeData::new().indicator_size(TabBarIndicatorSize::Tab),
            context: DefaultsContext::new(context),
            is_scrollable,
        }
    }

    pub(crate) const ICON_MARGIN: EdgeInsetsGeometry =
        EdgeInsetsGeometry::only(0.0, 0.0, 0.0, 10.0);
}

impl TabBarThemeDataGetters for TabsDefaultsM2 {
    fn tab_bar_theme_data(&self) -> &TabBarThemeData {
        &self.tab_bar_theme_data
    }

    fn indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        let is_dark = Theme::brightness_of(app, self.context.context()) == Brightness::Dark;
        let primary_color = if is_dark {
            Colors::GREY.shade900()
        } else {
            Colors::BLUE.primary()
        };
        Some(if colors.secondary == primary_color {
            Colors::WHITE
        } else {
            colors.secondary
        })
    }

    fn label_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let body_large = theme.primary_text_theme.body_large.clone();
        Some(
            body_large
                .as_ref()
                .and_then(|style| style.color.clone())
                .expect("the primary text theme's body large has a color"),
        )
    }

    fn splash_factory(&self, app: &mut App) -> Option<InteractiveInkFeatureFactoryRef> {
        let theme = self.context.theme(app);
        Some(theme.splash_factory.clone())
    }

    fn tab_alignment(&self, _app: &mut App) -> Option<TabAlignment> {
        Some(if self.is_scrollable {
            TabAlignment::Start
        } else {
            TabAlignment::Fill
        })
    }

    fn label_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context
            .theme(app)
            .primary_text_theme
            .body_large
            .clone()
    }

    fn unselected_label_style(&self, app: &mut App) -> Option<TextStyle> {
        self.context
            .theme(app)
            .primary_text_theme
            .body_large
            .clone()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! Ported from `test/tabs_test.dart` (with `tabs_utils.dart`'s helpers). The semantics,
    //! golden and leak-tracking cases are not ported.

    use waft_embedder::{Locale, TargetPlatform};
    use waft_widgets::{AnyElement, DefaultWidgetsLocalizations, Localizations, RichText};

    use super::*;
    use crate::app_bar::AppBar;
    use crate::material_localizations::DefaultMaterialLocalizations;
    use crate::scaffold::Scaffold;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use waft_widgets::ScrollControllerLeaf;

    pub(crate) fn tester() -> WidgetTester {
        let tester = WidgetTester::on(TargetPlatform::Android);
        tester.install_platform_fonts();
        tester
    }

    /// The theme Dart's `ThemeData(useMaterial3: use_material3, tabBarTheme: tab_bar_theme)`
    /// builds.
    pub(crate) fn theme(
        tester: &WidgetTester,
        use_material3: Option<bool>,
        tab_bar_theme: Option<TabBarThemeData>,
    ) -> ThemeData {
        let app = tester.cell.borrow();
        let mut builder = ThemeData::builder();
        if let Some(use_material3) = use_material3 {
            builder = builder.use_material3(use_material3);
        }
        if let Some(tab_bar_theme) = tab_bar_theme {
            builder = builder.tab_bar_theme(tab_bar_theme);
        }
        builder.build(&app)
    }

    /// Dart's `boilerplate(child:, textDirection:, theme:)`: the theme, en-US localizations, a
    /// `Directionality` and a `Material`.
    pub(crate) fn boilerplate<K>(
        theme: ThemeData,
        text_direction: TextDirection,
        child: impl IntoWidget<K>,
    ) -> WidgetRef {
        Theme::new(
            theme,
            Localizations::new(
                Locale::new("en").country_code("US"),
                vec![
                    DefaultMaterialLocalizations::delegate(),
                    DefaultWidgetsLocalizations::delegate(),
                ],
            )
            .child(Directionality::new(
                text_direction,
                Material::new().child(child),
            )),
        )
        .into_widget()
    }

    /// Dart's `buildFrame` arguments.
    pub(crate) struct Frame {
        pub(crate) tab_bar_key: Option<GlobalKey>,
        pub(crate) secondary_tab_bar: bool,
        pub(crate) tabs: Vec<&'static str>,
        pub(crate) value: &'static str,
        pub(crate) is_scrollable: bool,
        pub(crate) indicator_color: Option<Color>,
        pub(crate) animation_duration: Option<Duration>,
        pub(crate) padding: Option<EdgeInsetsGeometry>,
        pub(crate) text_direction: TextDirection,
        pub(crate) tab_alignment: Option<TabAlignment>,
        pub(crate) tab_bar_theme: Option<TabBarThemeData>,
        pub(crate) indicator: Option<DecorationRef>,
        pub(crate) use_material3: Option<bool>,
    }

    impl Frame {
        pub(crate) fn new(tabs: Vec<&'static str>, value: &'static str) -> Frame {
            Frame {
                tab_bar_key: None,
                secondary_tab_bar: false,
                tabs,
                value,
                is_scrollable: false,
                indicator_color: None,
                animation_duration: None,
                padding: None,
                text_direction: TextDirection::Ltr,
                tab_alignment: None,
                tab_bar_theme: None,
                indicator: None,
                use_material3: None,
            }
        }

        /// Dart's `buildFrame(..)`.
        pub(crate) fn build(self, tester: &WidgetTester) -> WidgetRef {
            let theme = theme(tester, self.use_material3, self.tab_bar_theme);
            let tabs = self
                .tabs
                .iter()
                .map(|tab| Tab::new().text(*tab).into_widget())
                .collect::<Vec<_>>();
            let mut tab_bar = if self.secondary_tab_bar {
                TabBar::secondary(tabs)
            } else {
                TabBar::new(tabs)
            }
            .is_scrollable(self.is_scrollable);
            if let Some(key) = self.tab_bar_key {
                tab_bar = tab_bar.key(key.into());
            }
            if let Some(indicator_color) = self.indicator_color {
                tab_bar = tab_bar.indicator_color(indicator_color);
            }
            if let Some(padding) = self.padding {
                tab_bar = tab_bar.padding(padding);
            }
            if let Some(tab_alignment) = self.tab_alignment {
                tab_bar = tab_bar.tab_alignment(tab_alignment);
            }
            if !self.secondary_tab_bar {
                tab_bar.indicator = self.indicator;
            }
            let initial_index = self
                .tabs
                .iter()
                .position(|tab| *tab == self.value)
                .expect("the value is one of the tabs");
            let mut controller =
                DefaultTabController::new(self.tabs.len(), tab_bar).initial_index(initial_index);
            if let Some(animation_duration) = self.animation_duration {
                controller = controller.animation_duration(animation_duration);
            }
            boilerplate(theme, self.text_direction, controller)
        }
    }

    /// Dart's `buildLeftRightApp`.
    pub(crate) fn build_left_right_app(
        tester: &WidgetTester,
        tabs: Vec<&'static str>,
        value: &'static str,
        automatic_indicator_color_adjustment: bool,
        theme_data: Option<ThemeData>,
    ) -> WidgetRef {
        let theme_data = theme_data.unwrap_or_else(|| {
            let app = tester.cell.borrow();
            ThemeData::builder()
                .platform(TargetPlatform::Android)
                .build(&app)
        });
        let initial_index = tabs.iter().position(|tab| *tab == value).expect("a tab");
        let tab_widgets = tabs
            .iter()
            .map(|tab| Tab::new().text(*tab).into_widget())
            .collect::<Vec<_>>();
        material_app(
            theme_data,
            DefaultTabController::new(
                tabs.len(),
                Scaffold::new()
                    .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                        AppBar::new().title(Text::new("tabs")).bottom(
                            waft_widgets::PreferredSizeWidgetRef::new(
                                TabBar::new(tab_widgets).automatic_indicator_color_adjustment(
                                    automatic_indicator_color_adjustment,
                                ),
                            ),
                        ),
                    ))
                    .body(TabBarView::new([
                        Center::new().child(Text::new("LEFT CHILD")).into_widget(),
                        Center::new().child(Text::new("RIGHT CHILD")).into_widget(),
                    ])),
            )
            .initial_index(initial_index)
            .into_widget(),
        )
    }

    /// The `TabController` the `DefaultTabController` above `element` provides: Dart's
    /// `DefaultTabController.of(tester.element(finder))`.
    pub(crate) fn default_controller_of(
        tester: &WidgetTester,
        element: AnyElement,
    ) -> Ref<TabController> {
        let mut app = tester.cell.borrow_mut();
        DefaultTabController::of(&mut app, element)
    }

    /// The first element showing `text`: Dart's `find.text(text)`.
    pub(crate) fn text(tester: &WidgetTester, text: &str) -> AnyElement {
        *tester
            .find_text(text)
            .first()
            .unwrap_or_else(|| panic!("a Text showing {text:?}"))
    }

    /// Dart's `tester.getSize(finder)`.
    pub(crate) fn size_of(tester: &WidgetTester, element: AnyElement) -> Size {
        let render = tester.render_box_of(element);
        render.size(&tester.cell.borrow())
    }

    /// Dart's `tester.getRect(finder)`.
    pub(crate) fn rect_of(tester: &WidgetTester, element: AnyElement) -> Rect {
        tester.get_rect(tester.render_box_of(element))
    }

    /// Dart's `tester.getCenter(finder)`.
    pub(crate) fn center_of(tester: &WidgetTester, element: AnyElement) -> Offset {
        rect_of(tester, element).center()
    }

    /// Dart's `tester.tap(finder)`.
    pub(crate) fn tap(tester: &mut WidgetTester, element: AnyElement) {
        let center = center_of(tester, element);
        tester.tap(center, 1);
    }

    /// The only element whose widget is a `W`: Dart's `find.byType(W)` with `findsOneWidget`.
    pub(crate) fn only<W: 'static>(tester: &WidgetTester) -> AnyElement {
        let found = tester.find_by_type::<W>();
        assert_eq!(found.len(), 1, "one {}", std::any::type_name::<W>());
        found[0]
    }

    #[test]
    fn a_tab_with_an_icon_is_as_wide_as_the_icon_and_46_high() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = theme(&tester, None, None);
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        Material::new()
                            .child(Tab::new().icon(SizedBox::new().width(10.0).height(10.0))),
                    )
                    .into_widget(),
            ));
            assert_eq!(
                size_of(&tester, only::<Tab>(&tester)),
                Size::new(10.0, 46.0)
            );
        });
    }

    #[test]
    fn a_tab_with_a_child_is_as_wide_as_the_child_and_46_high() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = theme(&tester, None, None);
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        Material::new()
                            .child(Tab::new().child(SizedBox::new().width(10.0).height(10.0))),
                    )
                    .into_widget(),
            ));
            assert_eq!(
                size_of(&tester, only::<Tab>(&tester)),
                Size::new(10.0, 46.0)
            );
        });
    }

    #[test]
    fn a_tab_with_text_is_as_wide_as_the_text_and_46_high() {
        with_main_thread_stack(|| {
            for material3 in [true, false] {
                let mut tester = tester();
                let theme = {
                    let app = tester.cell.borrow();
                    ThemeData::builder()
                        .use_material3(material3)
                        .font_family("FlutterTest")
                        .build(&app)
                };
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(Material::new().child(Tab::new().text("x")))
                        .into_widget(),
                ));
                let paragraph = only::<RichText>(&tester);
                let render = tester.render_box_of(paragraph);
                {
                    let app = tester.cell.borrow();
                    let paragraph = render
                        .as_object()
                        .downcast::<waft_rendering::RenderParagraph>(&app)
                        .expect("a paragraph");
                    assert_eq!(
                        paragraph
                            .text(&app)
                            .style()
                            .and_then(|style| style.font_family.clone())
                            .as_deref(),
                        Some("FlutterTest")
                    );
                }
                let expected = if material3 {
                    Size::new(14.25, 46.0)
                } else {
                    Size::new(14.0, 46.0)
                };
                assert_eq!(size_of(&tester, only::<Tab>(&tester)), expected);
            }
        });
    }

    // 'TabBar tap selects tab'.
    #[test]
    fn tapping_a_tab_selects_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let tabs = vec!["A", "B", "C"];

            let frame = Frame::new(tabs.clone(), "C").build(&tester);
            tester.pump_widget(frame);
            assert!(!tester.find_text("A").is_empty());
            assert!(!tester.find_text("B").is_empty());
            assert!(!tester.find_text("C").is_empty());
            let controller = default_controller_of(&tester, text(&tester, "A"));
            {
                let app = tester.cell.borrow();
                assert_eq!(controller.index(&app), 2);
                assert_eq!(controller.previous_index(&app), 2);
            }

            let frame = Frame::new(tabs.clone(), "C").build(&tester);
            tester.pump_widget(frame);
            let b = text(&tester, "B");
            tap(&mut tester, b);
            tester.pump(Duration::ZERO);
            {
                let app = tester.cell.borrow();
                assert!(controller.index_is_changing(&app));
            }
            tester.pump(Duration::from_secs(1)); // finish the animation
            {
                let app = tester.cell.borrow();
                assert_eq!(controller.index(&app), 1);
                assert_eq!(controller.previous_index(&app), 2);
                assert!(!controller.index_is_changing(&app));
            }

            let frame = Frame::new(tabs.clone(), "C").build(&tester);
            tester.pump_widget(frame);
            let c = text(&tester, "C");
            tap(&mut tester, c);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));
            {
                let app = tester.cell.borrow();
                assert_eq!(controller.index(&app), 2);
                assert_eq!(controller.previous_index(&app), 1);
            }

            let frame = Frame::new(tabs.clone(), "C").build(&tester);
            tester.pump_widget(frame);
            let a = text(&tester, "A");
            tap(&mut tester, a);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));
            {
                let app = tester.cell.borrow();
                assert_eq!(controller.index(&app), 0);
                assert_eq!(controller.previous_index(&app), 2);
            }
        });
    }

    /// One stroked two-point path `render` or its subtree paints: Dart's `paints..line`.
    #[derive(Clone, Copy, Debug)]
    pub(crate) struct PaintedLine {
        pub(crate) color: Color,
        pub(crate) stroke_width: f64,
        pub(crate) p1: Offset,
        pub(crate) p2: Offset,
    }

    /// Every line `render` and its subtree paint, in order: a stroked path of two points,
    /// read back from its bounds, which is exact for the horizontal lines the cases draw.
    pub(crate) fn painted_lines(
        tester: &WidgetTester,
        render: AnyRenderObject,
    ) -> Vec<PaintedLine> {
        use waft_embedder::valo::{Op, PaintStyle as ValoPaintStyle};
        tester
            .painted_ops(render)
            .into_iter()
            .filter_map(|op| match op {
                Op::DrawPath { path, paint, .. } => {
                    let ValoPaintStyle::Stroke(stroke) = paint.style else {
                        return None;
                    };
                    if path.elements().count() != 2 {
                        return None;
                    }
                    let bounds = path.bounds();
                    Some(PaintedLine {
                        color: Color::new(argb32(paint.color)),
                        stroke_width: f64::from(stroke.width),
                        p1: Offset::new(f64::from(bounds.x), f64::from(bounds.y)),
                        p2: Offset::new(
                            f64::from(bounds.x + bounds.width),
                            f64::from(bounds.y + bounds.height),
                        ),
                    })
                }
                _ => None,
            })
            .collect()
    }

    fn argb32(color: waft_embedder::valo::Color) -> u32 {
        let channel = |value: f32| (value * 255.0).round() as u32;
        (channel(color.a) << 24)
            | (channel(color.r) << 16)
            | (channel(color.g) << 8)
            | channel(color.b)
    }

    /// Whether `a` and `b` are within `epsilon` of each other: Dart's `moreOrLessEquals`.
    pub(crate) fn close(a: f64, b: f64, epsilon: f64) -> bool {
        (a - b).abs() <= epsilon
    }

    /// Whether `line` is `color`, `stroke_width` wide, from exactly `p1` to `p2`, as Dart's
    /// `paints..line` asks.
    pub(crate) fn is_line(
        line: &PaintedLine,
        color: Color,
        stroke_width: Option<f64>,
        p1: Offset,
        p2: Offset,
    ) -> bool {
        line.color.to_argb32() == color.to_argb32()
            && stroke_width.is_none_or(|width| line.stroke_width == width)
            && line.p1 == p1
            && line.p2 == p2
    }

    /// The render box of the first `W`: Dart's `tester.firstRenderObject<RenderBox>(find.byType(W))`.
    pub(crate) fn first_render_box<W: 'static>(tester: &WidgetTester) -> AnyRenderBox {
        let element = *tester
            .find_by_type::<W>()
            .first()
            .unwrap_or_else(|| panic!("a {}", std::any::type_name::<W>()));
        tester.render_box_of(element)
    }

    // 'Material3 - Scrollable TabBar tap centers selected tab'.
    #[test]
    fn material3_tapping_a_tab_of_a_scrollable_tab_bar_centers_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let tabs = vec![
                "AAAAAA", "BBBBBB", "CCCCCC", "DDDDDD", "EEEEEE", "FFFFFF", "GGGGGG", "HHHHHH",
                "IIIIII", "JJJJJJ", "KKKKKK", "LLLLLL",
            ];
            let tab_bar_key = GlobalKey::new();
            let mut frame = Frame::new(tabs, "AAAAAA");
            frame.is_scrollable = true;
            frame.tab_bar_key = Some(tab_bar_key.clone());
            let frame = frame.build(&tester);
            tester.pump_widget(frame);
            let controller = default_controller_of(&tester, text(&tester, "AAAAAA"));
            assert_eq!(controller.index(&tester.cell.borrow()), 0);

            let tab_bar = tester.render_object(&tab_bar_key).as_box().expect("a box");
            assert_eq!(tab_bar.size(&tester.cell.borrow()).width(), 800.0);
            // The center of the FFFFFF item is to the right of the TabBar's center
            assert!(center_of(&tester, text(&tester, "FFFFFF")).dx() > 401.0);

            let f = text(&tester, "FFFFFF");
            tap(&mut tester, f);
            tester.pump_and_settle();
            assert_eq!(controller.index(&tester.cell.borrow()), 5);
            // The center of the FFFFFF item is now at the TabBar's center
            let center = center_of(&tester, text(&tester, "FFFFFF")).dx();
            assert!(close(center, 452.0, 1.0), "{center}");
        });
    }

    // 'TabBarView page left and right test'.
    #[test]
    fn dragging_a_tab_bar_view_past_its_width_changes_the_page() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller = {
                let mut app = tester.cell.borrow_mut();
                TabController::new(&mut app, 0, None, 2, crate::test_support::TestVSync)
            };
            let theme = theme(&tester, None, None);
            tester.pump_widget(boilerplate(
                theme,
                TextDirection::Ltr,
                TabBarView::new([
                    Text::new("First").into_widget(),
                    Text::new("Second").into_widget(),
                ])
                .controller(controller.clone()),
            ));
            let index = |tester: &WidgetTester| controller.index(&tester.cell.borrow());
            assert_eq!(index(&tester), 0);

            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, Offset::new(100.0, 100.0));
            assert_eq!(index(&tester), 0);

            // Drag to the left and right, by less than the TabBarView's width.
            // The selected index (controller.index) should not change.
            tester.move_to(&mut gesture, Offset::new(0.0, 100.0), true);
            tester.move_to(&mut gesture, Offset::new(100.0, 100.0), true);
            assert_eq!(index(&tester), 0);
            assert_eq!(tester.find_text("First").len(), 1);
            assert!(tester.find_text("Second").is_empty());

            // Drag more than the TabBarView's width to the right. This forces
            // the selected index to change to 1.
            tester.move_to(&mut gesture, Offset::new(-400.0, 100.0), true);
            tester.up(&gesture);
            tester.pump(Duration::ZERO); // start the scroll animation
            tester.pump(Duration::from_secs(1)); // finish the scroll animation
            assert_eq!(index(&tester), 1);
            assert!(tester.find_text("First").is_empty());
            assert_eq!(tester.find_text("Second").len(), 1);

            let mut gesture = WidgetTester::touch(2);
            tester.down(&mut gesture, Offset::new(100.0, 100.0));
            assert_eq!(index(&tester), 1);

            tester.move_to(&mut gesture, Offset::new(0.0, 100.0), true);
            tester.move_to(&mut gesture, Offset::new(100.0, 100.0), true);
            assert_eq!(index(&tester), 1);
            assert!(tester.find_text("First").is_empty());
            assert_eq!(tester.find_text("Second").len(), 1);

            // Drag more than the TabBarView's width to the left. This forces
            // the selected index to change back to 0.
            tester.move_to(&mut gesture, Offset::new(600.0, 100.0), true);
            tester.up(&gesture);
            tester.pump(Duration::ZERO); // start the scroll animation
            tester.pump(Duration::from_secs(1)); // finish the scroll animation
            assert_eq!(index(&tester), 0);
            assert_eq!(tester.find_text("First").len(), 1);
            assert!(tester.find_text("Second").is_empty());
        });
    }

    // 'TabBar default tab indicator (primary)'.
    #[test]
    fn the_default_primary_indicator_is_a_rounded_label_underline_or_a_tab_line() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = theme(&tester, None, None);
            let controller = {
                let mut app = tester.cell.borrow_mut();
                TabController::new(&mut app, 0, None, 4, crate::test_support::TestVSync)
            };
            const INDICATOR_WEIGHT_LABEL: f64 = 3.0;
            const INDICATOR_WEIGHT_TAB: f64 = 2.0;

            let build_tab = |indicator_size: Option<TabBarIndicatorSize>| {
                let tabs =
                    (0..4).map(|index| Tab::new().text(format!("Tab {index}")).into_widget());
                let mut tab_bar = TabBar::new(tabs).controller(controller.clone());
                if let Some(indicator_size) = indicator_size {
                    tab_bar = tab_bar.indicator_size(indicator_size);
                }
                material_app(
                    theme.clone(),
                    boilerplate(
                        theme.clone(),
                        TextDirection::Ltr,
                        waft_widgets::Container::new()
                            .alignment(AlignmentGeometry::TOP_LEFT)
                            .child(tab_bar),
                    ),
                )
            };

            // Test default tab indicator (TabBarIndicatorSize.label).
            tester.pump_widget(build_tab(None));

            let tab_bar_box = first_render_box::<TabBar>(&tester);
            assert_eq!(tab_bar_box.size(&tester.cell.borrow()).height(), 48.0);

            // Check tab indicator size and color.
            let rrect = waft_embedder::RRect::from_ltrb_and_corners(
                64.75,
                48.0 - INDICATOR_WEIGHT_LABEL,
                135.25,
                48.0,
                waft_embedder::Radius::circular(3.0),
                waft_embedder::Radius::circular(3.0),
                waft_embedder::Radius::ZERO,
                waft_embedder::Radius::ZERO,
            );
            assert!(tester.paints_rrect(
                tab_bar_box.as_object(),
                rrect,
                Some(theme.color_scheme.primary)
            ));

            // Test default tab indicator (TabBarIndicatorSize.tab).
            tester.pump_widget(build_tab(Some(TabBarIndicatorSize::Tab)));
            tester.pump_and_settle();

            let tab_bar_box = first_render_box::<TabBar>(&tester);
            assert_eq!(tab_bar_box.size(&tester.cell.borrow()).height(), 48.0);

            const INDICATOR_Y: f64 = 48.0 - (INDICATOR_WEIGHT_TAB / 2.0);
            const INDICATOR_LEFT: f64 = INDICATOR_WEIGHT_TAB / 2.0;
            const INDICATOR_RIGHT: f64 = 200.0 - (INDICATOR_WEIGHT_TAB / 2.0);

            // Check tab indicator size and color.
            let lines = painted_lines(&tester, tab_bar_box.as_object());
            // Divider.
            let divider = lines
                .iter()
                .position(|line| {
                    line.color.to_argb32() == theme.color_scheme.outline_variant().to_argb32()
                })
                .expect("a divider line");
            // Tab indicator.
            assert!(
                lines[divider + 1..].iter().any(|line| is_line(
                    line,
                    theme.color_scheme.primary,
                    Some(INDICATOR_WEIGHT_TAB),
                    Offset::new(INDICATOR_LEFT, INDICATOR_Y),
                    Offset::new(INDICATOR_RIGHT, INDICATOR_Y),
                )),
                "{lines:?}"
            );
        });
    }

    mod sizes_styles_and_flings {
        //! tabs_test.dart cases from 'indicatorPadding update test' through 'TabBar animationDuration
        //! sets indicator animation duration'.

        use super::*;

        use std::cell::RefCell;

        use waft_embedder::{RRect, Radius};
        use waft_rendering::RenderParagraph;
        use waft_widgets::{Icon, RichText};

        use crate::app_bar::AppBar;
        use crate::icons::Icons;
        use crate::material::RenderInkFeatures;
        use crate::scaffold::Scaffold;
        use crate::test_support::{Drawn, TestVSync, is_same_color_as};

        /// The style of the `Text` showing `text` under a tab's `_TabStyle`: Dart's `getTabText`'s
        /// paragraph's `text.style`.
        fn tab_text_style(tester: &WidgetTester, label: &str) -> TextStyle {
            let styles = tester.find_by_type::<waft_widgets::Animated<TabStyle>>();
            let element = tester
                .find_text(label)
                .into_iter()
                .find(|element| {
                    let ancestors = tester.ancestors(*element);
                    styles.iter().any(|style| ancestors.contains(style))
                })
                .unwrap_or_else(|| panic!("a Text showing {label:?} under a _TabStyle"));
            let render = tester.render_box_of(element);
            let app = tester.cell.borrow();
            let paragraph = render
                .as_object()
                .downcast::<RenderParagraph>(&app)
                .expect("a Text renders a paragraph");
            paragraph
                .text(&app)
                .style()
                .cloned()
                .expect("the paragraph's text has a style")
        }

        /// The `fontFamily` of the only `RichText`'s paragraph.
        fn rich_text_font_family(tester: &WidgetTester) -> Option<String> {
            let render = tester.render_box_of(only::<RichText>(tester));
            let app = tester.cell.borrow();
            let paragraph = render
                .as_object()
                .downcast::<RenderParagraph>(&app)
                .expect("a paragraph");
            paragraph
                .text(&app)
                .style()
                .and_then(|style| style.font_family.clone())
        }

        fn style_color(style: &TextStyle) -> Option<Color> {
            style.color.as_ref().map(AnyColor::color)
        }

        /// A FlutterTest theme: Dart's `ThemeData(fontFamily: 'FlutterTest')`.
        fn flutter_test_theme(tester: &WidgetTester) -> ThemeData {
            let app = tester.cell.borrow();
            ThemeData::builder().font_family("FlutterTest").build(&app)
        }

        fn tab_controller(tester: &WidgetTester, length: usize) -> Ref<TabController> {
            let mut app = tester.cell.borrow_mut();
            TabController::new(&mut app, 0, None, length, TestVSync)
        }

        /// Whether `drawn` has a rect of `color`: Dart's `paints..rect(color:)`.
        fn has_rect(drawn: &[Drawn], color: Color) -> bool {
            drawn
                .iter()
                .any(|drawn| matches!(drawn, Drawn::Rect(argb, _) if is_same_color_as(Color::new(*argb), color)))
        }

        /// Whether `drawn` has a rect, then a rect of `color`: Dart's `paints..rect()..rect(color:)`.
        fn has_rect_then_rect(drawn: &[Drawn], color: Color) -> bool {
            drawn
                .iter()
                .position(|drawn| matches!(drawn, Drawn::Rect(..)))
                .is_some_and(|first| has_rect(&drawn[first + 1..], color))
        }

        /// What the first material and its subtree paint: Dart's `paints` on
        /// `tester.allRenderObjects.firstWhere(_RenderInkFeatures)`.
        fn first_material_paints(tester: &WidgetTester) -> Vec<Drawn> {
            let render = tester.first::<RenderInkFeatures>();
            tester.painted(render.as_object())
        }

        // 'Tab sizing - icon and text'.
        #[test]
        fn a_tab_with_an_icon_and_text_is_as_wide_as_the_text_and_72_high() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = flutter_test_theme(&tester);
                let material3 = theme.use_material3;
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            Material::new().child(
                                Tab::new()
                                    .icon(SizedBox::new().width(10.0).height(10.0))
                                    .text("x"),
                            ),
                        )
                        .into_widget(),
                ));
                assert_eq!(
                    rich_text_font_family(&tester).as_deref(),
                    Some("FlutterTest")
                );
                let expected = if material3 {
                    Size::new(14.25, 72.0)
                } else {
                    Size::new(14.0, 72.0)
                };
                assert_eq!(size_of(&tester, only::<Tab>(&tester)), expected);
            });
        }

        // 'Tab sizing - icon, iconMargin and text'.
        #[test]
        fn a_tab_with_an_icon_margin_is_as_wide_as_the_icon_and_its_margin() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = flutter_test_theme(&tester);
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            Material::new().child(
                                Tab::new()
                                    .icon(SizedBox::new().width(10.0).height(10.0))
                                    .icon_margin(EdgeInsetsGeometry::symmetric(0.0, 100.0))
                                    .text("x"),
                            ),
                        )
                        .into_widget(),
                ));
                assert_eq!(
                    rich_text_font_family(&tester).as_deref(),
                    Some("FlutterTest")
                );
                assert_eq!(
                    size_of(&tester, only::<Tab>(&tester)),
                    Size::new(210.0, 72.0)
                );
            });
        }

        // 'Tab sizing - icon and child'.
        #[test]
        fn a_tab_with_an_icon_and_a_child_is_as_wide_as_the_child_and_72_high() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = flutter_test_theme(&tester);
                let material3 = theme.use_material3;
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            Material::new().child(
                                Tab::new()
                                    .icon(SizedBox::new().width(10.0).height(10.0))
                                    .child(Text::new("x")),
                            ),
                        )
                        .into_widget(),
                ));
                assert_eq!(
                    rich_text_font_family(&tester).as_deref(),
                    Some("FlutterTest")
                );
                let expected = if material3 {
                    Size::new(14.25, 72.0)
                } else {
                    Size::new(14.0, 72.0)
                };
                assert_eq!(size_of(&tester, only::<Tab>(&tester)), expected);
            });
        }

        /// Dart's `getIconMargin`: from the bottom of the house icon to the top of the label.
        fn icon_margin(tester: &WidgetTester) -> f64 {
            let icon_rect = rect_of(tester, tester.find_by_icon(&Icons::house()));
            let label_rect = rect_of(tester, text(tester, "x"));
            label_rect.top - icon_rect.bottom
        }

        // 'Material2 - Default Tab iconMargin'.
        #[test]
        fn material2_a_tab_icon_has_10_pixels_below_it_by_default() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = super::theme(&tester, Some(false), None);
                tester.pump_widget(material_app(
                    theme,
                    Material::new()
                        .child(Tab::new().icon(Icon::new(Some(Icons::house()))).text("x"))
                        .into_widget(),
                ));
                assert_eq!(icon_margin(&tester), 10.0);
            });
        }

        // 'Material3 - Default Tab iconMargin'.
        #[test]
        fn material3_a_tab_icon_has_2_pixels_below_it_by_default() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = super::theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    Material::new()
                        .child(Tab::new().icon(Icon::new(Some(Icons::house()))).text("x"))
                        .into_widget(),
                ));
                assert_eq!(icon_margin(&tester), 2.0);
            });
        }

        /// Whether the only `TabBar` paints a line of `color`: Dart's `paints..line(color:)`.
        fn tab_bar_paints_line_of(tester: &WidgetTester, color: Color) -> bool {
            let tab_bar = first_render_box::<TabBar>(tester);
            painted_lines(tester, tab_bar.as_object())
                .iter()
                .any(|line| line.color.to_argb32() == color.to_argb32())
        }

        // 'Tab color - normal'.
        #[test]
        fn a_tab_bar_on_a_material_paints_its_divider_or_its_m2_indicator() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = flutter_test_theme(&tester);
                let material3 = theme.use_material3;
                let controller = tab_controller(&tester, 1);
                tester.pump_widget(material_app(
                    theme.clone(),
                    Material::new()
                        .child(
                            TabBar::new([SizedBox::shrink().into_widget()]).controller(controller),
                        )
                        .into_widget(),
                ));
                let expected = if material3 {
                    theme.color_scheme.outline_variant()
                } else {
                    Colors::BLUE.shade500()
                };
                assert!(tab_bar_paints_line_of(&tester, expected));
            });
        }

        // 'Tab color - match'.
        #[test]
        fn an_m2_indicator_the_color_of_its_material_turns_white() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = super::theme(&tester, None, None);
                let material3 = theme.use_material3;
                let controller = tab_controller(&tester, 1);
                tester.pump_widget(material_app(
                    theme.clone(),
                    Material::new()
                        .color(Color::new(0xff2196f3))
                        .child(
                            TabBar::new([SizedBox::shrink().into_widget()]).controller(controller),
                        )
                        .into_widget(),
                ));
                let expected = if material3 {
                    theme.color_scheme.outline_variant()
                } else {
                    Colors::WHITE
                };
                assert!(tab_bar_paints_line_of(&tester, expected));
            });
        }

        // 'Tab color - transparency'.
        #[test]
        fn a_tab_bar_on_a_transparent_material_keeps_its_indicator_color() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = super::theme(&tester, None, None);
                let material3 = theme.use_material3;
                let controller = tab_controller(&tester, 1);
                tester.pump_widget(material_app(
                    theme.clone(),
                    Material::new()
                        .r#type(MaterialType::Transparency)
                        .child(
                            TabBar::new([SizedBox::shrink().into_widget()]).controller(controller),
                        )
                        .into_widget(),
                ));
                let expected = if material3 {
                    theme.color_scheme.outline_variant()
                } else {
                    Colors::BLUE.shade500()
                };
                assert!(tab_bar_paints_line_of(&tester, expected));
            });
        }

        /// Dart's label style cases: the selected and unselected labels' family, size and color.
        fn assert_label_styles(
            tester: &WidgetTester,
            selected: &str,
            selected_color: Color,
            unselected: &str,
            unselected_color: Color,
        ) {
            assert_eq!(tester.find_text("A").len(), 1);
            assert_eq!(tester.find_text("B").len(), 1);
            assert_eq!(tester.find_text("C").len(), 1);

            // Test selected label text style.
            let selected_label = tab_text_style(tester, selected);
            assert_eq!(selected_label.font_family.as_deref(), Some("Roboto"));
            assert_eq!(selected_label.font_size, Some(14.0));
            assert_eq!(style_color(&selected_label), Some(selected_color));

            // Test unselected label text style.
            let unselected_label = tab_text_style(tester, unselected);
            assert_eq!(unselected_label.font_family.as_deref(), Some("Roboto"));
            assert_eq!(unselected_label.font_size, Some(14.0));
            assert_eq!(style_color(&unselected_label), Some(unselected_color));
        }

        // 'TabBar default selected/unselected label style (primary)'.
        #[test]
        fn a_primary_tab_bar_labels_are_primary_when_selected_and_on_surface_variant_otherwise() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = super::theme(&tester, None, None);
                let mut frame = Frame::new(vec!["A", "B", "C"], "A");
                frame.use_material3 = Some(theme.use_material3);
                let frame = frame.build(&tester);
                tester.pump_widget(frame);
                assert_label_styles(
                    &tester,
                    "A",
                    theme.color_scheme.primary,
                    "C",
                    theme.color_scheme.on_surface_variant(),
                );
            });
        }

        // 'TabBar default selected/unselected label style (secondary)'.
        #[test]
        fn a_secondary_tab_bar_labels_are_on_surface_when_selected_and_on_surface_variant_otherwise()
         {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = super::theme(&tester, None, None);
                let mut frame = Frame::new(vec!["A", "B", "C"], "A");
                frame.secondary_tab_bar = true;
                frame.use_material3 = Some(theme.use_material3);
                let frame = frame.build(&tester);
                tester.pump_widget(frame);
                assert_label_styles(
                    &tester,
                    "A",
                    theme.color_scheme.on_surface,
                    "C",
                    theme.color_scheme.on_surface_variant(),
                );
            });
        }

        // 'Material 3 primary tab indicator has a minimum width of 24dp'.
        #[test]
        fn a_material3_primary_label_indicator_is_at_least_24_wide() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let mut frame = Frame::new(vec!["A", "Medium", "Longer Tab"], "A");
                frame.use_material3 = Some(true);
                let frame = frame.build(&tester);
                tester.pump_widget(frame);

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                let label_box = tester.render_box_of(text(&tester, "A"));
                let (label_center, tab_bar_height) = {
                    let app = tester.cell.borrow();
                    let label_size = label_box.size(&app);
                    assert!(label_size.width() < 24.0);
                    let global =
                        label_box.local_to_global(&app, label_size.center(Offset::ZERO), None);
                    (
                        tab_bar_box.global_to_local(&app, global, None),
                        tab_bar_box.size(&app).height(),
                    )
                };
                let tab_bar = only::<TabBar>(&tester);
                let primary = {
                    let mut app = tester.cell.borrow_mut();
                    Theme::of(&mut app, tab_bar).color_scheme.primary
                };
                let expected_indicator = Rect::from_ltrb(
                    label_center.dx() - 12.0,
                    tab_bar_height - 3.0,
                    label_center.dx() + 12.0,
                    tab_bar_height,
                );

                assert!(tester.paints_rrect(
                    tab_bar_box.as_object(),
                    RRect::from_rect_and_corners(
                        expected_indicator,
                        Radius::circular(3.0),
                        Radius::circular(3.0),
                        Radius::ZERO,
                        Radius::ZERO,
                    ),
                    Some(primary),
                ));
            });
        }

        // 'TabBar default tab indicator (secondary)'.
        #[test]
        fn the_default_secondary_indicator_is_a_tab_wide_line_over_the_divider() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = super::theme(&tester, None, None);
                let controller = tab_controller(&tester, 4);
                const INDICATOR_WEIGHT: f64 = 2.0;

                // Test default tab indicator.
                let tabs =
                    (0..4).map(|index| Tab::new().text(format!("Tab {index}")).into_widget());
                tester.pump_widget(material_app(
                    theme.clone(),
                    boilerplate(
                        theme.clone(),
                        TextDirection::Ltr,
                        waft_widgets::Container::new()
                            .alignment(AlignmentGeometry::TOP_LEFT)
                            .child(TabBar::secondary(tabs).controller(controller)),
                    ),
                ));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                assert_eq!(tab_bar_box.size(&tester.cell.borrow()).height(), 48.0);

                const INDICATOR_Y: f64 = 48.0 - (INDICATOR_WEIGHT / 2.0);
                const INDICATOR_LEFT: f64 = INDICATOR_WEIGHT / 2.0;
                const INDICATOR_RIGHT: f64 = 200.0 - (INDICATOR_WEIGHT / 2.0);

                // Check tab indicator size and color.
                let lines = painted_lines(&tester, tab_bar_box.as_object());
                // Divider.
                let divider = lines
                    .iter()
                    .position(|line| {
                        line.color.to_argb32() == theme.color_scheme.outline_variant().to_argb32()
                    })
                    .expect("a divider line");
                // Tab indicator.
                assert!(
                    lines[divider + 1..].iter().any(|line| is_line(
                        line,
                        theme.color_scheme.primary,
                        Some(INDICATOR_WEIGHT),
                        Offset::new(INDICATOR_LEFT, INDICATOR_Y),
                        Offset::new(INDICATOR_RIGHT, INDICATOR_Y),
                    )),
                    "{lines:?}"
                );
            });
        }

        /// Dart's `TabBar default overlay` cases: hover and press a selected and an unselected tab
        /// with a mouse.
        #[allow(deprecated)]
        fn check_default_overlay(
            tester: &mut WidgetTester,
            selected_hovered: Color,
            selected_pressed: Color,
            unselected_hovered: Color,
        ) {
            let mut gesture = WidgetTester::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            let selected = center_of(tester, text(tester, "A"));
            tester.move_to(&mut gesture, selected, false);
            tester.pump_and_settle();
            assert!(has_rect(&first_material_paints(tester), selected_hovered));

            tester.down(&mut gesture, selected);
            tester.pump_and_settle();
            assert!(has_rect_then_rect(
                &first_material_paints(tester),
                selected_pressed
            ));
            tester.up(&gesture);
            tester.pump_and_settle();

            let unselected = center_of(tester, text(tester, "B"));
            tester.move_to(&mut gesture, unselected, false);
            tester.pump_and_settle();
            assert!(has_rect(&first_material_paints(tester), unselected_hovered));

            tester.move_to(&mut gesture, selected, false);
            tester.pump_and_settle();
            assert!(has_rect(&first_material_paints(tester), selected_hovered));

            tester.down(&mut gesture, selected);
            tester.pump_and_settle();
            assert!(has_rect_then_rect(
                &first_material_paints(tester),
                selected_pressed
            ));
            tester.up(&gesture);
            tester.pump_and_settle();
        }

        // 'TabBar default overlay (primary)'.
        #[test]
        #[allow(deprecated)]
        fn a_primary_tab_overlay_is_primary_on_the_selected_tab_and_on_surface_on_others() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = super::theme(&tester, None, None);
                let mut frame = Frame::new(vec!["A", "B"], "A");
                frame.use_material3 = Some(theme.use_material3);
                let frame = frame.build(&tester);
                tester.pump_widget(frame);
                let colors = &theme.color_scheme;
                check_default_overlay(
                    &mut tester,
                    colors.primary.with_opacity(0.08),
                    colors.primary.with_opacity(0.1),
                    colors.on_surface.with_opacity(0.08),
                );
            });
        }

        // 'TabBar default overlay (secondary)'.
        #[test]
        #[allow(deprecated)]
        fn a_secondary_tab_overlay_is_on_surface_on_every_tab() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = super::theme(&tester, None, None);
                let mut frame = Frame::new(vec!["A", "B"], "A");
                frame.secondary_tab_bar = true;
                frame.use_material3 = Some(theme.use_material3);
                let frame = frame.build(&tester);
                tester.pump_widget(frame);
                let colors = &theme.color_scheme;
                check_default_overlay(
                    &mut tester,
                    colors.on_surface.with_opacity(0.08),
                    colors.on_surface.with_opacity(0.1),
                    colors.on_surface.with_opacity(0.08),
                );
            });
        }

        const TWELVE_TABS: [&str; 12] = [
            "AAAAAA", "BBBBBB", "CCCCCC", "DDDDDD", "EEEEEE", "FFFFFF", "GGGGGG", "HHHHHH",
            "IIIIII", "JJJJJJ", "KKKKKK", "LLLLLL",
        ];

        fn index_of(tester: &WidgetTester, controller: Handle<TabController>) -> usize {
            controller.index(&tester.cell.borrow())
        }

        // 'Scrollable TabBar tap selects tab'.
        #[test]
        fn tapping_a_tab_of_a_scrollable_tab_bar_selects_it() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let mut frame = Frame::new(vec!["A", "B", "C"], "C");
                frame.is_scrollable = true;
                let frame = frame.build(&tester);
                tester.pump_widget(frame);
                assert_eq!(tester.find_text("A").len(), 1);
                assert_eq!(tester.find_text("B").len(), 1);
                assert_eq!(tester.find_text("C").len(), 1);
                let controller = default_controller_of(&tester, text(&tester, "A"));
                assert_eq!(index_of(&tester, controller.handle()), 2);
                assert_eq!(controller.previous_index(&tester.cell.borrow()), 2);

                for (label, index) in [("C", 2), ("B", 1), ("A", 0)] {
                    let tab = text(&tester, label);
                    tap(&mut tester, tab);
                    tester.pump_and_settle();
                    assert_eq!(index_of(&tester, controller.handle()), index);
                }
            });
        }

        /// Dart's 'Scrollable TabBar ... tap centers selected tab' cases: tap FFFFFF of twelve
        /// scrollable tabs and find it centred at `expected`.
        fn check_tap_centers_tab(
            use_material3: Option<bool>,
            padding: Option<EdgeInsetsGeometry>,
            text_direction: TextDirection,
            expected: f64,
        ) {
            let mut tester = tester();
            let tab_bar_key = GlobalKey::new();
            let mut frame = Frame::new(TWELVE_TABS.to_vec(), "AAAAAA");
            frame.is_scrollable = true;
            frame.tab_bar_key = Some(tab_bar_key.clone());
            frame.padding = padding;
            frame.text_direction = text_direction;
            frame.use_material3 = use_material3;
            let frame = frame.build(&tester);
            tester.pump_widget(frame);
            let controller = default_controller_of(&tester, text(&tester, "AAAAAA"));
            assert_eq!(index_of(&tester, controller.handle()), 0);

            let tab_bar = tester.render_object(&tab_bar_key).as_box().expect("a box");
            assert_eq!(tab_bar.size(&tester.cell.borrow()).width(), 800.0);
            let center = center_of(&tester, text(&tester, "FFFFFF")).dx();
            match text_direction {
                // The center of the FFFFFF item is to the right of the TabBar's center
                TextDirection::Ltr => assert!(center > 401.0, "{center}"),
                // The center of the FFFFFF item is to the left of the TabBar's center
                TextDirection::Rtl => assert!(center < 401.0, "{center}"),
            }

            let f = text(&tester, "FFFFFF");
            tap(&mut tester, f);
            tester.pump_and_settle();
            assert_eq!(index_of(&tester, controller.handle()), 5);
            // The center of the FFFFFF item is now at the TabBar's center
            let center = center_of(&tester, text(&tester, "FFFFFF")).dx();
            assert!(close(center, expected, 1.0), "{center}");
        }

        fn padding_right_30_left_60() -> EdgeInsetsGeometry {
            EdgeInsetsGeometry::only(60.0, 0.0, 30.0, 0.0)
        }

        // 'Material2 - Scrollable TabBar tap centers selected tab'.
        #[test]
        fn material2_tapping_a_tab_of_a_scrollable_tab_bar_centers_it() {
            with_main_thread_stack(|| {
                check_tap_centers_tab(Some(false), None, TextDirection::Ltr, 400.0);
            });
        }

        // 'Material2 - Scrollable TabBar, with padding, tap centers selected tab'.
        #[test]
        fn material2_tapping_a_tab_of_a_padded_scrollable_tab_bar_centers_it() {
            with_main_thread_stack(|| {
                check_tap_centers_tab(
                    Some(false),
                    Some(padding_right_30_left_60()),
                    TextDirection::Ltr,
                    400.0,
                );
            });
        }

        // 'Material3 - Scrollable TabBar, with padding, tap centers selected tab'.
        #[test]
        fn material3_tapping_a_tab_of_a_padded_scrollable_tab_bar_centers_it() {
            with_main_thread_stack(|| {
                check_tap_centers_tab(
                    None,
                    Some(padding_right_30_left_60()),
                    TextDirection::Ltr,
                    452.0,
                );
            });
        }

        // 'Material2 - Scrollable TabBar, with padding and TextDirection.rtl, tap centers selected tab'.
        #[test]
        fn material2_tapping_a_tab_of_a_padded_right_to_left_scrollable_tab_bar_centers_it() {
            with_main_thread_stack(|| {
                check_tap_centers_tab(
                    Some(false),
                    Some(padding_right_30_left_60()),
                    TextDirection::Rtl,
                    400.0,
                );
            });
        }

        // 'Material3 - Scrollable TabBar, with padding and TextDirection.rtl, tap centers selected tab'.
        #[test]
        fn material3_tapping_a_tab_of_a_padded_right_to_left_scrollable_tab_bar_centers_it() {
            with_main_thread_stack(|| {
                check_tap_centers_tab(
                    None,
                    Some(padding_right_30_left_60()),
                    TextDirection::Rtl,
                    348.0,
                );
            });
        }

        /// Dart's 'TabBar can be scrolled independent of the selection' cases.
        fn check_scrolled_independent_of_selection(use_material3: Option<bool>, start_limit: f64) {
            let mut tester = tester();
            let tabs = vec![
                "AAAA", "BBBB", "CCCC", "DDDD", "EEEE", "FFFF", "GGGG", "HHHH", "IIII", "JJJJ",
                "KKKK", "LLLL",
            ];
            let tab_bar_key = GlobalKey::new();
            let mut frame = Frame::new(tabs, "AAAA");
            frame.is_scrollable = true;
            frame.tab_bar_key = Some(tab_bar_key.clone());
            frame.use_material3 = use_material3;
            let frame = frame.build(&tester);
            tester.pump_widget(frame);
            let controller = default_controller_of(&tester, text(&tester, "AAAA"));
            assert_eq!(index_of(&tester, controller.handle()), 0);

            // Fling-scroll the TabBar to the left
            assert!(center_of(&tester, text(&tester, "HHHH")).dx() < start_limit);
            let tab_bar_center = tester.center_of(&tab_bar_key);
            tester.fling_from(tab_bar_center, Offset::new(-200.0, 0.0), 10000.0, 1);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1)); // finish the scroll animation
            assert!(center_of(&tester, text(&tester, "HHHH")).dx() < 500.0);

            // Scrolling the TabBar doesn't change the selection
            assert_eq!(index_of(&tester, controller.handle()), 0);
        }

        // 'Material2 - TabBar can be scrolled independent of the selection'.
        #[test]
        fn material2_flinging_a_scrollable_tab_bar_keeps_the_selection() {
            with_main_thread_stack(|| check_scrolled_independent_of_selection(Some(false), 700.0));
        }

        // 'Material3 - TabBar can be scrolled independent of the selection'.
        #[test]
        fn material3_flinging_a_scrollable_tab_bar_keeps_the_selection() {
            with_main_thread_stack(|| check_scrolled_independent_of_selection(None, 720.0));
        }

        /// Dart's `TabStateMarker`: a stateful widget whose state carries a `marker`.
        #[derive(Debug)]
        struct TabStateMarker {
            child: Option<WidgetRef>,
        }

        impl StatefulWidget for TabStateMarker {
            type State = TabStateMarkerState;

            fn create_state(&self) -> TabStateMarkerState {
                TabStateMarkerState {
                    state: StateData::new(),
                    marker: None,
                }
            }
        }

        /// Dart's `TabStateMarkerState`.
        struct TabStateMarkerState {
            state: StateData<TabStateMarker>,
            marker: Option<String>,
        }

        impl State for TabStateMarkerState {
            type Widget = TabStateMarker;
            waft_widgets::state_accessors!();

            fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
                self.widget(app)
                    .child
                    .clone()
                    .unwrap_or_else(|| waft_widgets::Container::new().into_widget())
            }
        }

        // 'TabBarView maintains state'.
        #[test]
        fn a_tab_bar_view_keeps_the_state_of_the_pages_beside_the_current_one() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = ["AAAAAA", "BBBBBB", "CCCCCC", "DDDDDD", "EEEEEE"];
                let theme = super::theme(&tester, None, None);
                let builder = |value: &str| {
                    boilerplate(
                        theme.clone(),
                        TextDirection::Ltr,
                        DefaultTabController::new(
                            tabs.len(),
                            TabBarView::new(tabs.iter().map(|name| {
                                TabStateMarker {
                                    child: Some(Text::new(*name).into_widget()),
                                }
                                .into_widget()
                            })),
                        )
                        .initial_index(tabs.iter().position(|tab| *tab == value).expect("a tab")),
                    )
                };
                // Dart's `find.widgetWithText(TabStateMarker, name, skipOffstage: false)`'s state.
                let find_state_marker_state = |tester: &WidgetTester, name: &str| {
                    let marker = tester
                        .find_by_type::<TabStateMarker>()
                        .into_iter()
                        .find(|marker| {
                            tester
                                .find_text(name)
                                .iter()
                                .any(|text| tester.ancestors(*text).contains(marker))
                        })
                        .unwrap_or_else(|| panic!("a TabStateMarker with {name:?}"));
                    marker
                        .state_handle::<TabStateMarkerState>(&tester.cell.borrow())
                        .expect("a marker state")
                };

                let mut value = tabs[0];
                tester.pump_widget(builder(value));
                let controller = default_controller_of(&tester, text(&tester, "AAAAAA"));

                let mut gesture = WidgetTester::touch(1);
                let start = center_of(&tester, text(&tester, tabs[0]));
                tester.down(&mut gesture, start);
                tester.move_to(&mut gesture, start + Offset::new(-600.0, 0.0), true);
                tester.pump(Duration::ZERO);
                assert_eq!(value, tabs[0]);
                let state = find_state_marker_state(&tester, tabs[1]);
                tester.cell.borrow_mut().get_mut(state).marker = Some("marked".to_string());
                tester.up(&gesture);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_secs(1));
                value = tabs[index_of(&tester, controller.handle())];
                assert_eq!(value, tabs[1]);
                tester.pump_widget(builder(value));
                let state = find_state_marker_state(&tester, tabs[1]);
                assert_eq!(
                    tester.cell.borrow().get(state).marker.as_deref(),
                    Some("marked")
                );

                // Move to the third tab.

                let mut gesture = WidgetTester::touch(2);
                let start = center_of(&tester, text(&tester, tabs[1]));
                tester.down(&mut gesture, start);
                tester.move_to(&mut gesture, start + Offset::new(-600.0, 0.0), true);
                tester.up(&gesture);
                tester.pump(Duration::ZERO);
                let state = find_state_marker_state(&tester, tabs[1]);
                assert_eq!(
                    tester.cell.borrow().get(state).marker.as_deref(),
                    Some("marked")
                );
                tester.pump(Duration::from_secs(1));
                value = tabs[index_of(&tester, controller.handle())];
                assert_eq!(value, tabs[2]);
                tester.pump_widget(builder(value));

                // The state is now gone.

                assert!(tester.find_onstage_text(tabs[1]).is_empty());

                // Move back to the second tab.

                let mut gesture = WidgetTester::touch(3);
                let start = center_of(&tester, text(&tester, tabs[2]));
                tester.down(&mut gesture, start);
                tester.move_to(&mut gesture, start + Offset::new(600.0, 0.0), true);
                tester.pump(Duration::ZERO);
                let marker_state = find_state_marker_state(&tester, tabs[1]);
                assert_eq!(tester.cell.borrow().get(marker_state).marker, None);
                tester.cell.borrow_mut().get_mut(marker_state).marker = Some("marked".to_string());
                tester.up(&gesture);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_secs(1));
                value = tabs[index_of(&tester, controller.handle())];
                assert_eq!(value, tabs[1]);
                tester.pump_widget(builder(value));
                let state = find_state_marker_state(&tester, tabs[1]);
                assert_eq!(
                    tester.cell.borrow().get(state).marker.as_deref(),
                    Some("marked")
                );
            });
        }

        /// `buildLeftRightApp(tabs: ['LEFT', 'RIGHT'], value: 'LEFT')`, pumped, with its controller
        /// at 0 and its first page showing.
        fn pump_left_right_app(tester: &mut WidgetTester) -> Ref<TabController> {
            let app = build_left_right_app(tester, vec!["LEFT", "RIGHT"], "LEFT", true, None);
            tester.pump_widget(app);
            assert_eq!(tester.find_onstage_text("LEFT").len(), 1);
            assert_eq!(tester.find_onstage_text("RIGHT").len(), 1);
            assert_eq!(tester.find_onstage_text("LEFT CHILD").len(), 1);
            assert!(tester.find_onstage_text("RIGHT CHILD").is_empty());

            let controller = default_controller_of(tester, text(tester, "LEFT"));
            assert_eq!(index_of(tester, controller.handle()), 0);
            controller
        }

        fn assert_page(tester: &WidgetTester, left: bool) {
            assert_eq!(
                tester.find_onstage_text("LEFT CHILD").len(),
                usize::from(left)
            );
            assert_eq!(
                tester.find_onstage_text("RIGHT CHILD").len(),
                usize::from(!left)
            );
        }

        // 'TabBar left/right fling'.
        #[test]
        fn flinging_a_tab_bar_view_left_and_right_switches_its_tab() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = pump_left_right_app(&mut tester);

                // Fling to the left, switch from the 'LEFT' tab to the 'RIGHT'
                let fling_start = center_of(&tester, text(&tester, "LEFT CHILD"));
                tester.fling_from(fling_start, Offset::new(-200.0, 0.0), 10000.0, 1);
                tester.pump_and_settle();
                assert_eq!(index_of(&tester, controller.handle()), 1);
                assert_page(&tester, false);

                // Fling to the right, switch back to the 'LEFT' tab
                let fling_start = center_of(&tester, text(&tester, "RIGHT CHILD"));
                tester.fling_from(fling_start, Offset::new(200.0, 0.0), 10000.0, 2);
                tester.pump_and_settle();
                assert_eq!(index_of(&tester, controller.handle()), 0);
                assert_page(&tester, true);
            });
        }

        // 'TabBar left/right fling reverse (1)'.
        #[test]
        fn flinging_the_first_page_right_keeps_it() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = pump_left_right_app(&mut tester);

                let fling_start = center_of(&tester, text(&tester, "LEFT CHILD"));
                tester.fling_from(fling_start, Offset::new(200.0, 0.0), 10000.0, 1);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_secs(1)); // finish the scroll animation
                assert_eq!(index_of(&tester, controller.handle()), 0);
                assert_page(&tester, true);
            });
        }

        // 'TabBar left/right fling reverse (2)' (the first of the two cases Dart names so).
        #[test]
        fn flinging_the_first_page_left_settles_on_the_second_after_one_second() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = pump_left_right_app(&mut tester);

                let fling_start = center_of(&tester, text(&tester, "LEFT CHILD"));
                tester.fling_from(fling_start, Offset::new(-200.0, 0.0), 10000.0, 1);
                tester.pump(Duration::ZERO);
                // this is similar to a test above, but that one does many more pumps
                tester.pump(Duration::from_secs(1)); // finish the scroll animation
                assert_eq!(index_of(&tester, controller.handle()), 1);
                assert_page(&tester, false);
            });
        }

        // 'TabBar left/right fling reverse (2)' (the second, a regression test for
        // https://github.com/flutter/flutter/issues/5095). Dart's two move loops run while
        // `index > 50` from 0, so never: the gesture is a down and an up.
        #[test]
        fn a_fling_ended_by_reversing_settles_back_on_its_first_page() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = pump_left_right_app(&mut tester);

                let fling_start = center_of(&tester, text(&tester, "LEFT CHILD"));
                let mut gesture = WidgetTester::touch(1);
                tester.down(&mut gesture, fling_start);
                // End the fling by reversing direction. This should cause not cause
                // a change to the selected tab, everything should just settle back to
                // where it started.
                tester.up(&gesture);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_secs(1)); // finish the scroll animation
                assert_eq!(index_of(&tester, controller.handle()), 0);
                assert_page(&tester, true);
            });
        }

        /// `TabControllerFrame(length:, initialIndex:, builder:)`: a controller made for the frame,
        /// with `TestVSync` where Dart's frame state is the vsync, and the frame `builder` builds.
        fn pump_tab_controller_frame(
            tester: &mut WidgetTester,
            length: usize,
            initial_index: usize,
            builder: impl FnOnce(Ref<TabController>) -> WidgetRef,
        ) -> Ref<TabController> {
            let controller = {
                let mut app = tester.cell.borrow_mut();
                TabController::new(&mut app, initial_index, None, length, TestVSync)
            };
            tester.pump_widget(builder(controller.clone()));
            controller
        }

        /// The app Dart's `buildTabControllerFrame`s build: a scaffold with a tab bar of `tabs` and a
        /// tab bar view of `children`, on `platform`.
        fn tab_controller_frame_app(
            tester: &WidgetTester,
            platform: waft_embedder::TargetPlatform,
            tabs: &[&str],
            children: &[&str],
            controller: Ref<TabController>,
        ) -> WidgetRef {
            let theme = {
                let app = tester.cell.borrow();
                ThemeData::builder().platform(platform).build(&app)
            };
            material_app(
                theme,
                Scaffold::new()
                    .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                        AppBar::new().title(Text::new("tabs")).bottom(
                            waft_widgets::PreferredSizeWidgetRef::new(
                                TabBar::new(
                                    tabs.iter().map(|tab| Tab::new().text(*tab).into_widget()),
                                )
                                .controller(controller.clone()),
                            ),
                        ),
                    ))
                    .body(
                        TabBarView::new(
                            children
                                .iter()
                                .map(|child| Center::new().child(Text::new(*child)).into_widget()),
                        )
                        .controller(controller),
                    )
                    .into_widget(),
            )
        }

        // 'TabController notifies the index to change when left flinging'.
        #[test]
        fn flinging_a_tab_bar_view_left_notifies_the_index_of_the_page_it_lands_on() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = ["A", "B", "C"];
                let theme_tester = self::tester();
                let tab_controller =
                    pump_tab_controller_frame(&mut tester, tabs.len(), 2, |controller| {
                        tab_controller_frame_app(
                            &theme_tester,
                            waft_embedder::TargetPlatform::IOS,
                            &tabs,
                            &["CHILD A", "CHILD B", "CHILD C"],
                            controller,
                        )
                    });
                assert_eq!(index_of(&tester, tab_controller.handle()), 2);

                let notified = Rc::new(RefCell::new(Vec::new()));
                let notified_in_listener = Rc::clone(&notified);
                let listened = tab_controller.handle();
                tab_controller.add_listener(
                    &mut tester.cell.borrow_mut(),
                    Listener::new(move |app: &mut App| {
                        notified_in_listener.borrow_mut().push(listened.index(app));
                    }),
                );
                let fling_start = center_of(&tester, text(&tester, "CHILD C"));
                tester.fling_from(fling_start, Offset::new(600.0, 0.0), 10000.0, 1);
                tester.pump_and_settle();
                let index_of_b = 1;
                for index in notified.borrow().iter() {
                    assert_eq!(*index, index_of_b);
                }
            });
        }

        // 'TabBar fling velocity' (a regression test for https://github.com/flutter/flutter/issues/7133).
        #[test]
        fn a_small_slow_fling_of_a_scrollable_tab_bar_keeps_the_second_tab_visible() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = super::theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    waft_widgets::Align::new()
                        .alignment(AlignmentGeometry::TOP_LEFT)
                        .child(
                            SizedBox::new().width(300.0).height(200.0).child(
                                DefaultTabController::new(
                                    TWELVE_TABS.len(),
                                    Scaffold::new()
                                        .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                                            AppBar::new().title(Text::new("tabs")).bottom(
                                                waft_widgets::PreferredSizeWidgetRef::new(
                                                    TabBar::new(TWELVE_TABS.iter().map(|tab| {
                                                        Tab::new().text(*tab).into_widget()
                                                    }))
                                                    .is_scrollable(true),
                                                ),
                                            ),
                                        ))
                                        .body(TabBarView::new((0..TWELVE_TABS.len()).map(
                                            |index| Text::new(index.to_string()).into_widget(),
                                        ))),
                                ),
                            ),
                        )
                        .into_widget(),
                ));

                // After a small slow fling to the left, we expect the second item to still be visible.
                let start = center_of(&tester, text(&tester, "AAAAAA"));
                tester.fling_from(start, Offset::new(-25.0, 0.0), 100.0, 1);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_secs(1)); // finish the scroll animation
                let render = tester.render_box_of(text(&tester, "BBBBBB"));
                let app = tester.cell.borrow();
                assert!(render.local_to_global(&app, Offset::ZERO, None).dx() > 0.0);
            });
        }

        // 'TabController change notification'.
        #[test]
        fn a_tab_controller_notifies_its_listeners_of_taps_and_flings() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = ["LEFT", "RIGHT"];
                let app = build_left_right_app(&tester, tabs.to_vec(), "LEFT", true, None);
                tester.pump_widget(app);
                let controller = default_controller_of(&tester, text(&tester, "LEFT"));
                assert_eq!(index_of(&tester, controller.handle()), 0);

                let value = Rc::new(RefCell::new(None::<&'static str>));
                let value_in_listener = Rc::clone(&value);
                let listened = controller.handle();
                controller.add_listener(
                    &mut tester.cell.borrow_mut(),
                    Listener::new(move |app: &mut App| {
                        *value_in_listener.borrow_mut() = Some(tabs[listened.index(app)]);
                    }),
                );

                let right = text(&tester, "RIGHT");
                tap(&mut tester, right);
                tester.pump_and_settle();
                assert_eq!(*value.borrow(), Some("RIGHT"));

                let left = text(&tester, "LEFT");
                tap(&mut tester, left);
                tester.pump_and_settle();
                assert_eq!(*value.borrow(), Some("LEFT"));

                let left_fling_start = center_of(&tester, text(&tester, "LEFT CHILD"));
                tester.fling_from(left_fling_start, Offset::new(-200.0, 0.0), 10000.0, 1);
                tester.pump_and_settle();
                assert_eq!(*value.borrow(), Some("RIGHT"));

                let right_fling_start = center_of(&tester, text(&tester, "RIGHT CHILD"));
                tester.fling_from(right_fling_start, Offset::new(200.0, 0.0), 10000.0, 2);
                tester.pump_and_settle();
                assert_eq!(*value.borrow(), Some("LEFT"));
            });
        }

        // 'Explicit TabController'.
        #[test]
        fn an_explicit_tab_controller_drives_its_tab_bar_view() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = ["LEFT", "RIGHT"];
                let theme_tester = self::tester();
                let tab_controller =
                    pump_tab_controller_frame(&mut tester, tabs.len(), 1, |controller| {
                        tab_controller_frame_app(
                            &theme_tester,
                            waft_embedder::TargetPlatform::Android,
                            &tabs,
                            &["LEFT CHILD", "RIGHT CHILD"],
                            controller,
                        )
                    });

                assert_eq!(tester.find_onstage_text("LEFT").len(), 1);
                assert_eq!(tester.find_onstage_text("RIGHT").len(), 1);
                assert_page(&tester, false);
                {
                    let app = tester.cell.borrow();
                    assert_eq!(tab_controller.index(&app), 1);
                    assert_eq!(tab_controller.previous_index(&app), 1);
                    assert!(!tab_controller.index_is_changing(&app));
                    let animation = tab_controller.animation(&app).expect("an animation");
                    assert_eq!(animation.value(&app), 1.0);
                    assert_eq!(animation.status(&app), AnimationStatus::Forward);
                }

                tab_controller.set_index(&mut tester.cell.borrow_mut(), 0);
                tester.pump(Duration::from_millis(500));
                tester.pump(Duration::from_millis(500));
                assert_page(&tester, true);

                tab_controller.set_index(&mut tester.cell.borrow_mut(), 1);
                tester.pump(Duration::from_millis(500));
                tester.pump(Duration::from_millis(500));
                assert_page(&tester, false);
            });
        }

        // 'TabController listener resets index' (a regression test for the scenario in
        // https://github.com/flutter/flutter/pull/7387#pullrequestreview-15630946).
        #[test]
        fn an_animation_listener_that_resets_the_index_wins() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = ["A", "B", "C"];
                let theme_tester = self::tester();
                let tab_controller =
                    pump_tab_controller_frame(&mut tester, tabs.len(), 0, |controller| {
                        tab_controller_frame_app(
                            &theme_tester,
                            waft_embedder::TargetPlatform::Android,
                            &tabs,
                            &["CHILD A", "CHILD B", "CHILD C"],
                            controller,
                        )
                    });

                let changing_in_listener = Rc::new(RefCell::new(Vec::new()));
                let changing = Rc::clone(&changing_in_listener);
                {
                    let mut app = tester.cell.borrow_mut();
                    let animation = tab_controller.animation(&app).expect("an animation");
                    animation.add_listener(
                        &mut app,
                        Listener::new({
                            let tab_controller = tab_controller.clone();
                            move |app: &mut App| {
                                let animation =
                                    tab_controller.animation(app).expect("an animation");
                                if animation.status(app) == AnimationStatus::Forward {
                                    tab_controller.set_index(app, 2);
                                }
                                changing
                                    .borrow_mut()
                                    .push(tab_controller.index_is_changing(app));
                            }
                        }),
                    );
                }

                {
                    let app = tester.cell.borrow();
                    assert_eq!(tab_controller.index(&app), 0);
                    assert!(!tab_controller.index_is_changing(&app));
                }

                tab_controller.animate_to(
                    &mut tester.cell.borrow_mut(),
                    1,
                    Some(Duration::from_millis(200)),
                    Some(Curves::linear()),
                );
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(300));

                {
                    let app = tester.cell.borrow();
                    assert_eq!(tab_controller.index(&app), 2);
                    assert!(!tab_controller.index_is_changing(&app));
                }
                for changing in changing_in_listener.borrow().iter() {
                    assert!(*changing);
                }
            });
        }

        // 'TabBar unselectedLabelColor control test'.
        #[test]
        fn the_label_colors_give_the_selected_and_unselected_tabs_their_icon_color() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = tab_controller(&tester, 2);
                let first_color = Rc::new(RefCell::new(None::<Color>));
                let second_color = Rc::new(RefCell::new(None::<Color>));
                let first = Rc::clone(&first_color);
                let second = Rc::clone(&second_color);
                let theme = super::theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    TabBar::new([
                        waft_widgets::Builder::new(move |app, context| {
                            *first.borrow_mut() = IconTheme::of(app, context)
                                .color
                                .as_ref()
                                .map(AnyColor::color);
                            Text::new("First").into_widget()
                        })
                        .into_widget(),
                        waft_widgets::Builder::new(move |app, context| {
                            *second.borrow_mut() = IconTheme::of(app, context)
                                .color
                                .as_ref()
                                .map(AnyColor::color);
                            Text::new("Second").into_widget()
                        })
                        .into_widget(),
                    ])
                    .controller(controller)
                    .label_color(Colors::GREEN.shade500())
                    .unselected_label_color(Colors::BLUE.shade500()),
                ));

                assert_eq!(*first_color.borrow(), Some(Colors::GREEN.shade500()));
                assert_eq!(*second_color.borrow(), Some(Colors::BLUE.shade500()));
            });
        }

        // 'TabBar animationDuration sets indicator animation duration'.
        #[test]
        fn a_tab_bar_animates_its_selection_over_the_controllers_animation_duration() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let animation_duration = Duration::from_millis(100);
                let build = |tester: &WidgetTester| {
                    let mut frame = Frame::new(vec!["A", "B", "C"], "B");
                    frame.animation_duration = Some(animation_duration);
                    frame.build(tester)
                };

                let frame = build(&tester);
                tester.pump_widget(frame);
                let controller = default_controller_of(&tester, text(&tester, "A"));

                let a = text(&tester, "A");
                tap(&mut tester, a);
                tester.pump(Duration::ZERO);
                assert!(controller.index_is_changing(&tester.cell.borrow()));
                tester.pump(Duration::from_millis(50));
                tester.pump(animation_duration);
                {
                    let app = tester.cell.borrow();
                    assert_eq!(controller.index(&app), 0);
                    assert_eq!(controller.previous_index(&app), 1);
                    assert!(!controller.index_is_changing(&app));
                }

                //Test when index diff is greater than 1
                let frame = build(&tester);
                tester.pump_widget(frame);
                let c = text(&tester, "C");
                tap(&mut tester, c);
                tester.pump(Duration::ZERO);
                assert!(controller.index_is_changing(&tester.cell.borrow()));
                tester.pump(Duration::from_millis(50));
                tester.pump(animation_duration);
                {
                    let app = tester.cell.borrow();
                    assert_eq!(controller.index(&app), 2);
                    assert_eq!(controller.previous_index(&app), 0);
                    assert!(!controller.index_is_changing(&app));
                }
            });
        }

        // 'tab semantics role test' (a regression test for
        // https://github.com/flutter/flutter/issues/169175): a tab whose icon is a zero-size image
        // semantics node builds. The semantics tree it made Dart throw on is deferred.
        #[test]
        fn a_tab_whose_icon_is_a_zero_size_image_builds() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = super::theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    DefaultTabController::new(
                        1,
                        TabBar::new([Tab::new()
                            .icon(Semantics::new().image(true).child(SizedBox::shrink()))
                            .into_widget()]),
                    ),
                ));
                assert_eq!(tester.find_onstage_by_type::<Tab>().len(), 1);
            });
        }
    }

    mod tab_bar_views_and_scroll_controllers {
        //! tabs_test.dart cases, part b: `ensureVisible does not move TabViews` through
        //! `Indicator elastic animation`.

        use super::*;

        use waft_foundation::ValueKey;
        use waft_widgets::ScrollableState;

        use crate::test_support::TestVSync;

        /// Dart's `createTabController(length:, vsync: const TestVSync(), initialIndex:,
        /// animationDuration:)`.
        fn create_tab_controller(
            tester: &WidgetTester,
            length: usize,
            initial_index: usize,
            animation_duration: Option<Duration>,
        ) -> Ref<TabController> {
            let mut app = tester.cell.borrow_mut();
            TabController::new(
                &mut app,
                initial_index,
                animation_duration,
                length,
                TestVSync,
            )
        }

        /// `tabs.map((String tab) => Tab(text: tab)).toList()`.
        fn text_tabs(tabs: &[&str]) -> Vec<WidgetRef> {
            tabs.iter()
                .map(|tab| Tab::new().text(*tab).into_widget())
                .collect()
        }

        /// A string `Key(key)`.
        fn key(key: impl Into<String>) -> KeyRef {
            ValueKey::new(key.into()).into()
        }

        /// `boilerplate(child: child)` with Dart's default theme and a left-to-right direction.
        fn boilerplate_ltr<K>(tester: &WidgetTester, child: impl IntoWidget<K>) -> WidgetRef {
            let theme = theme(tester, None, None);
            boilerplate(theme, TextDirection::Ltr, child)
        }

        /// The `Column` of a `TabBar` over a 400 square `TabBarView` of three centred texts that
        /// several cases build.
        fn bar_over_view(
            tabs: &[&str],
            controller: Ref<TabController>,
            view: TabBarView,
        ) -> Column {
            Column::new().children([
                TabBar::new(text_tabs(tabs))
                    .controller(controller.clone())
                    .into_widget(),
                SizedBox::square(Some(400.0))
                    .child(view.controller(controller))
                    .into_widget(),
            ])
        }

        /// `const <Widget>[Center(child: Text('0')), ..]` for `count` pages.
        fn numbered_pages(count: usize) -> Vec<WidgetRef> {
            (0..count)
                .map(|index| {
                    Center::new()
                        .child(Text::new(index.to_string()))
                        .into_widget()
                })
                .collect()
        }

        /// The `PageView`'s controller: Dart's `tester.widget<PageView>(find.byType(PageView))
        /// .controller!`.
        fn page_controller(tester: &WidgetTester) -> Ref<PageController> {
            tester.widget(only::<PageView>(tester), |page_view: &PageView| {
                page_view
                    .controller
                    .clone()
                    .expect("a TabBarView's PageView has a controller")
            })
        }

        /// `pageController.position.pixels`.
        fn pixels(tester: &WidgetTester, controller: Handle<PageController>) -> f64 {
            let app = tester.cell.borrow();
            controller.position(&app).pixels(&app)
        }

        fn index(tester: &WidgetTester, controller: Handle<TabController>) -> usize {
            controller.index(&tester.cell.borrow())
        }

        // 'TabBarView controller sets animation duration'.
        #[test]
        fn a_tab_bar_view_animates_for_its_controller_s_duration() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let animation_duration = Duration::from_millis(100);
                let tabs = ["A", "B", "C"];
                let tab_controller = create_tab_controller(&tester, 3, 1, Some(animation_duration));
                let frame = boilerplate_ltr(
                    &tester,
                    bar_over_view(
                        &tabs,
                        tab_controller.clone(),
                        TabBarView::new(numbered_pages(3)),
                    ),
                );
                tester.pump_widget(frame);

                assert_eq!(index(&tester, tab_controller.handle()), 1);

                let page_controller = page_controller(&tester);

                // The TabBarView's page width is 400, so page 0 is at scroll offset 0.0,
                // page 1 is at 400.0, page 2 is at 800.0.
                assert_eq!(pixels(&tester, page_controller.handle()), 400.0);
                let c = text(&tester, "C");
                tap(&mut tester, c);
                tester.pump(Duration::ZERO);
                assert_eq!(pixels(&tester, page_controller.handle()), 400.0);
                tester.pump(Duration::from_millis(50));
                tester.pump(animation_duration);
                assert_eq!(pixels(&tester, page_controller.handle()), 800.0);
            });
        }

        // 'TabBarView animation can be interrupted'.
        #[test]
        fn a_tab_bar_view_animation_can_be_interrupted() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let animation_duration = Duration::from_secs(2);
                let tabs = ["A", "B", "C"];
                let tab_controller = create_tab_controller(&tester, 3, 0, Some(animation_duration));
                let frame = boilerplate_ltr(
                    &tester,
                    bar_over_view(
                        &tabs,
                        tab_controller.clone(),
                        TabBarView::new(numbered_pages(3)),
                    ),
                );
                tester.pump_widget(frame);

                assert_eq!(index(&tester, tab_controller.handle()), 0);

                let page_controller = page_controller(&tester);

                assert_eq!(pixels(&tester, page_controller.handle()), 0.0);

                let c = text(&tester, "C");
                tap(&mut tester, c);
                tester.pump(Duration::from_millis(10)); // TODO(bleroux): find why this is needed.

                // Runs the animation for half of the animation duration.
                tester.pump(Duration::from_secs(1));

                // The position should be between page 1 and page 2.
                let position = pixels(&tester, page_controller.handle());
                assert!(position > 400.0, "{position}");
                assert!(position < 800.0, "{position}");

                // Switch to another tab before the end of the animation.
                let a = text(&tester, "A");
                tap(&mut tester, a);
                tester.pump(Duration::from_millis(10)); // TODO(bleroux): find why this is needed.
                tester.pump(animation_duration);
                assert_eq!(pixels(&tester, page_controller.handle()), 0.0);

                tester.pump_and_settle(); // Finish the animation.
            });
        }

        // 'TabBarView viewportFraction sets PageView viewport fraction'.
        #[test]
        fn a_tab_bar_view_gives_its_page_view_its_viewport_fraction() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = ["A", "B", "C"];
                let tab_controller =
                    create_tab_controller(&tester, 3, 1, Some(Duration::from_millis(100)));
                let frame = boilerplate_ltr(
                    &tester,
                    bar_over_view(
                        &tabs,
                        tab_controller.clone(),
                        TabBarView::new(numbered_pages(3)).viewport_fraction(0.8),
                    ),
                );
                tester.pump_widget(frame);

                assert_eq!(index(&tester, tab_controller.handle()), 1);

                // The TabView was initialized with viewportFraction as 0.8
                // So it's expected the PageView inside would obtain the same viewportFraction
                let page_controller = page_controller(&tester);
                assert_eq!(
                    page_controller.viewport_fraction(&tester.cell.borrow()),
                    0.8
                );
            });
        }

        // 'TabBarView viewportFraction is 1 by default'.
        #[test]
        fn a_tab_bar_view_s_viewport_fraction_is_1_by_default() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = ["A", "B", "C"];
                let tab_controller =
                    create_tab_controller(&tester, 3, 1, Some(Duration::from_millis(100)));
                let frame = boilerplate_ltr(
                    &tester,
                    bar_over_view(
                        &tabs,
                        tab_controller.clone(),
                        TabBarView::new(numbered_pages(3)),
                    ),
                );
                tester.pump_widget(frame);

                assert_eq!(index(&tester, tab_controller.handle()), 1);

                // The TabView was initialized with default viewportFraction
                // So it's expected the PageView inside would obtain the value 1
                let page_controller = page_controller(&tester);
                assert_eq!(
                    page_controller.viewport_fraction(&tester.cell.borrow()),
                    1.0
                );
            });
        }

        // 'TabBarView viewportFraction can be updated'.
        #[test]
        fn a_tab_bar_view_s_viewport_fraction_can_be_updated() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = ["A", "B", "C"];
                let build_frame = |tester: &WidgetTester, viewport_fraction: f64| {
                    let controller = create_tab_controller(tester, 3, 1, None);
                    boilerplate_ltr(
                        tester,
                        bar_over_view(
                            &tabs,
                            controller.clone(),
                            TabBarView::new(numbered_pages(3)).viewport_fraction(viewport_fraction),
                        ),
                    )
                };

                let frame = build_frame(&tester, 0.8);
                tester.pump_widget(frame);
                let controller = page_controller(&tester);
                assert_eq!(controller.viewport_fraction(&tester.cell.borrow()), 0.8);

                // Rebuild with a different viewport fraction.
                let frame = build_frame(&tester, 0.5);
                tester.pump_widget(frame);
                let controller = page_controller(&tester);
                assert_eq!(controller.viewport_fraction(&tester.cell.borrow()), 0.5);
            });
        }

        // 'TabBarView has clipBehavior Clip.hardEdge by default'.
        #[test]
        fn a_tab_bar_view_clips_hard_edge_by_default() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = || {
                    vec![
                        Text::new("First").into_widget(),
                        Text::new("Second").into_widget(),
                    ]
                };
                let frame = boilerplate_ltr(
                    &tester,
                    DefaultTabController::new(2, TabBarView::new(tabs())),
                );
                tester.pump_widget(frame);
                let clip_behavior = tester
                    .widget(only::<TabBarView>(&tester), |view: &TabBarView| {
                        view.clip_behavior
                    });
                assert_eq!(clip_behavior, Clip::HardEdge);
            });
        }

        // 'TabBarView sets clipBehavior correctly'.
        #[test]
        fn a_tab_bar_view_gives_its_page_view_its_clip_behavior() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = vec![
                    Text::new("First").into_widget(),
                    Text::new("Second").into_widget(),
                ];
                let frame = boilerplate_ltr(
                    &tester,
                    DefaultTabController::new(2, TabBarView::new(tabs).clip_behavior(Clip::None)),
                );
                tester.pump_widget(frame);
                let clip_behavior = tester.widget(only::<PageView>(&tester), |view: &PageView| {
                    view.clip_behavior
                });
                assert_eq!(clip_behavior, Clip::None);
            });
        }

        /// tabs_utils' `TabIndicatorRecordingCanvas`: the rect of the last line `render` paints in
        /// `indicator_color`, inflated by half its 2.0 weight.
        fn recorded_indicator_rect(
            tester: &WidgetTester,
            render: AnyRenderBox,
            indicator_color: Color,
        ) -> Rect {
            const INDICATOR_WEIGHT: f64 = 2.0;
            let line = painted_lines(tester, render.as_object())
                .into_iter()
                .rfind(|line| line.color.to_argb32() == indicator_color.to_argb32())
                .expect("an indicator line");
            Rect::from_points(line.p1, line.p2).inflate(INDICATOR_WEIGHT / 2.0)
        }

        // 'TabBar tap skips indicator animation when disabled in controller'.
        #[test]
        fn a_tap_moves_the_indicator_at_once_when_the_controller_does_not_animate() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = vec!["A", "B"];
                let indicator_color = Color::new(0xFFFF0000);
                let mut frame = Frame::new(tabs, "A");
                frame.use_material3 = Some(false);
                frame.indicator_color = Some(indicator_color);
                frame.animation_duration = Some(Duration::ZERO);
                let frame = frame.build(&tester);
                tester.pump_widget(frame);

                let render = first_render_box::<TabBar>(&tester);
                let indicator_rect0 = recorded_indicator_rect(&tester, render, indicator_color);
                assert_eq!(indicator_rect0.left, 0.0);
                assert_eq!(indicator_rect0.width(), 400.0);
                assert_eq!(indicator_rect0.height(), 2.0);

                let b = text(&tester, "B");
                tap(&mut tester, b);
                tester.pump(Duration::ZERO);
                let indicator_rect2 = recorded_indicator_rect(&tester, render, indicator_color);
                assert_eq!(indicator_rect2.left, 400.0);
                assert_eq!(indicator_rect2.width(), 400.0);
                assert_eq!(indicator_rect2.height(), 2.0);
            });
        }

        // 'TabBar tap changes index instantly when animation is disabled in controller'.
        #[test]
        fn a_tap_changes_the_index_at_once_when_the_controller_does_not_animate() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = vec!["A", "B", "C"];
                let frame = |tester: &WidgetTester| {
                    let mut frame = Frame::new(tabs.clone(), "B");
                    frame.animation_duration = Some(Duration::ZERO);
                    frame.build(tester)
                };
                let built = frame(&tester);
                tester.pump_widget(built);
                let controller = default_controller_of(&tester, text(&tester, "A"));

                let a = text(&tester, "A");
                tap(&mut tester, a);
                tester.pump(Duration::ZERO);
                {
                    let app = tester.cell.borrow();
                    assert_eq!(controller.index(&app), 0);
                    assert_eq!(controller.previous_index(&app), 1);
                    assert!(!controller.index_is_changing(&app));
                }

                //Test when index diff is greater than 1
                let built = frame(&tester);
                tester.pump_widget(built);
                let c = text(&tester, "C");
                tap(&mut tester, c);
                tester.pump(Duration::ZERO);
                {
                    let app = tester.cell.borrow();
                    assert_eq!(controller.index(&app), 2);
                    assert_eq!(controller.previous_index(&app), 0);
                    assert!(!controller.index_is_changing(&app));
                }
            });
        }

        // 'Scrollable TabBar does not have overscroll indicator'.
        #[test]
        fn a_scrollable_tab_bar_has_no_overscroll_indicator() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let mut frame = Frame::new(vec!["A", "B", "C"], "A");
                frame.is_scrollable = true;
                let frame = frame.build(&tester);
                tester.pump_widget(frame);
                assert!(
                    tester
                        .find_onstage_by_type::<waft_widgets::GlowingOverscrollIndicator>()
                        .is_empty()
                );
            });
        }

        // 'TabBar should not throw when animation is disabled in controller'.
        #[test]
        fn tabs_added_to_a_tab_bar_whose_controller_does_not_animate_can_be_selected() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let mut tabs = vec!["A"];

                let build_with_tab_bar_view = |tester: &WidgetTester, tabs: &[&'static str]| {
                    boilerplate_ltr(
                        tester,
                        DefaultTabController::new(
                            tabs.len(),
                            Column::new().children([
                                TabBar::new(text_tabs(tabs))
                                    .is_scrollable(true)
                                    .into_widget(),
                                waft_widgets::Flexible::new(TabBarView::new(
                                    (0..tabs.len()).map(|index| {
                                        Text::new(format!("Tab {index}")).into_widget()
                                    }),
                                ))
                                .into_widget(),
                            ]),
                        )
                        .animation_duration(Duration::ZERO),
                    )
                };

                let frame = build_with_tab_bar_view(&tester, &tabs);
                tester.pump_widget(frame);
                let controller = default_controller_of(&tester, text(&tester, "A"));
                assert_eq!(index(&tester, controller.handle()), 0);

                tabs.push("B");
                let frame = build_with_tab_bar_view(&tester, &tabs);
                tester.pump_widget(frame);
                tabs.push("C");
                let frame = build_with_tab_bar_view(&tester, &tabs);
                tester.pump_widget(frame);
                let c = text(&tester, "C");
                tap(&mut tester, c);
                tester.pump_and_settle();
                let controller = default_controller_of(&tester, text(&tester, "A"));
                assert_eq!(index(&tester, controller.handle()), 2);
            });
        }

        /// The `TabBar` over a 400 square `TabBarView` of `tabs.len()` pages whose controller does
        /// not animate, that the skip-animation cases build; answers the controller.
        fn pump_unanimated_tabs(
            tester: &mut WidgetTester,
            tabs: &[&str],
            initial_index: usize,
        ) -> Ref<TabController> {
            let tab_controller =
                create_tab_controller(tester, tabs.len(), initial_index, Some(Duration::ZERO));
            let frame = boilerplate_ltr(
                tester,
                bar_over_view(
                    tabs,
                    tab_controller.clone(),
                    TabBarView::new(numbered_pages(tabs.len())),
                ),
            );
            tester.pump_widget(frame);
            tab_controller
        }

        fn tap_text(tester: &mut WidgetTester, label: &str) {
            let element = text(tester, label);
            tap(tester, element);
        }

        // 'TabBarView skips animation when disabled in controller'.
        #[test]
        fn a_tab_bar_view_jumps_when_its_controller_does_not_animate() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_controller = pump_unanimated_tabs(&mut tester, &["A", "B", "C"], 1);

                assert_eq!(index(&tester, tab_controller.handle()), 1);

                let page_controller = page_controller(&tester);

                // The TabBarView's page width is 400, so page 0 is at scroll offset 0.0,
                // page 1 is at 400.0, page 2 is at 800.0.
                assert_eq!(pixels(&tester, page_controller.handle()), 400.0);
                tap_text(&mut tester, "C");
                tester.pump(Duration::ZERO);
                assert_eq!(pixels(&tester, page_controller.handle()), 800.0);
            });
        }

        // 'TabBarView skips animation when disabled in controller - skip tabs'.
        #[test]
        fn a_tab_bar_view_jumps_over_tabs_when_its_controller_does_not_animate() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_controller = pump_unanimated_tabs(&mut tester, &["A", "B", "C"], 0);

                assert_eq!(index(&tester, tab_controller.handle()), 0);

                let page_controller = page_controller(&tester);

                // The TabBarView's page width is 400, so page 0 is at scroll offset 0.0,
                // page 1 is at 400.0, page 2 is at 800.0.
                assert_eq!(pixels(&tester, page_controller.handle()), 0.0);
                tap_text(&mut tester, "C");
                tester.pump(Duration::ZERO);
                assert_eq!(pixels(&tester, page_controller.handle()), 800.0);
            });
        }

        // 'TabBarView skips animation when disabled in controller - skip tabs twice'.
        #[test]
        fn a_tab_bar_view_jumps_over_tabs_twice_when_its_controller_does_not_animate() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_controller = pump_unanimated_tabs(&mut tester, &["A", "B", "C"], 0);

                assert_eq!(index(&tester, tab_controller.handle()), 0);

                let page_controller = page_controller(&tester);

                // The TabBarView's page width is 400, so page 0 is at scroll offset 0.0,
                // page 1 is at 400.0, page 2 is at 800.0.
                assert_eq!(pixels(&tester, page_controller.handle()), 0.0);
                tap_text(&mut tester, "C");
                tester.pump(Duration::ZERO);
                assert_eq!(pixels(&tester, page_controller.handle()), 800.0);

                tap_text(&mut tester, "A");
                tester.pump(Duration::ZERO);
                assert_eq!(pixels(&tester, page_controller.handle()), 0.0);
            });
        }

        // 'TabBarView skips animation when disabled in controller - skip tabs followed by single tab navigation'.
        #[test]
        fn a_tab_bar_view_jumps_over_tabs_then_to_neighbours_when_its_controller_does_not_animate()
        {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_controller = pump_unanimated_tabs(&mut tester, &["A", "B", "C"], 0);

                assert_eq!(index(&tester, tab_controller.handle()), 0);

                let page_controller = page_controller(&tester);

                // The TabBarView's page width is 400, so page 0 is at scroll offset 0.0,
                // page 1 is at 400.0, page 2 is at 800.0.
                assert_eq!(pixels(&tester, page_controller.handle()), 0.0);
                tap_text(&mut tester, "C");
                tester.pump(Duration::ZERO);
                assert_eq!(pixels(&tester, page_controller.handle()), 800.0);

                tap_text(&mut tester, "B");
                tester.pump(Duration::ZERO);
                assert_eq!(pixels(&tester, page_controller.handle()), 400.0);

                tap_text(&mut tester, "A");
                tester.pump(Duration::ZERO);
                assert_eq!(pixels(&tester, page_controller.handle()), 0.0);
            });
        }

        // 'TabBarView skips animation when disabled in controller - two tabs'.
        #[test]
        fn a_two_tab_bar_view_jumps_when_its_controller_does_not_animate() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_controller = pump_unanimated_tabs(&mut tester, &["A", "B"], 0);

                assert_eq!(index(&tester, tab_controller.handle()), 0);

                let page_controller = page_controller(&tester);

                // The TabBarView's page width is 400, so page 0 is at scroll offset 0.0,
                // page 1 is at 400.0, page 2 is at 800.0.
                assert_eq!(pixels(&tester, page_controller.handle()), 0.0);
                tap_text(&mut tester, "B");
                tester.pump(Duration::ZERO);
                assert_eq!(pixels(&tester, page_controller.handle()), 400.0);
            });
        }

        // 'TabBar tap animates the selection indicator'.
        #[test]
        fn a_tap_animates_the_selection_indicator() {
            with_main_thread_stack(|| {
                // This is a regression test for https://github.com/flutter/flutter/issues/7479
                let mut tester = tester();
                let indicator_color = Color::new(0xFFFF0000);
                let mut frame = Frame::new(vec!["A", "B"], "A");
                frame.use_material3 = Some(false);
                frame.indicator_color = Some(indicator_color);
                let frame = frame.build(&tester);
                tester.pump_widget(frame);

                let render = first_render_box::<TabBar>(&tester);
                let indicator_rect0 = recorded_indicator_rect(&tester, render, indicator_color);
                assert_eq!(indicator_rect0.left, 0.0);
                assert_eq!(indicator_rect0.width(), 400.0);
                assert_eq!(indicator_rect0.height(), 2.0);

                tap_text(&mut tester, "B");
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(50));
                let indicator_rect1 = recorded_indicator_rect(&tester, render, indicator_color);
                assert!(
                    indicator_rect1.left > indicator_rect0.left,
                    "{indicator_rect1:?}"
                );
                assert!(indicator_rect1.right < 800.0, "{indicator_rect1:?}");
                assert_eq!(indicator_rect1.height(), 2.0);

                tester.pump(Duration::from_millis(300));
                let indicator_rect2 = recorded_indicator_rect(&tester, render, indicator_color);
                assert_eq!(indicator_rect2.left, 400.0);
                assert_eq!(indicator_rect2.width(), 400.0);
                assert_eq!(indicator_rect2.height(), 2.0);
            });
        }

        // 'TabBarView child disposed during animation'.
        #[test]
        fn a_replaced_tab_bar_stops_listening_to_the_controller() {
            with_main_thread_stack(|| {
                // This is a regression test for this patch:
                // https://github.com/flutter/flutter/pull/9015
                let mut tester = tester();
                let controller = create_tab_controller(&tester, 2, 0, None);

                let build_frame = |tester: &WidgetTester| {
                    boilerplate_ltr(
                        tester,
                        TabBar::new([Text::new("A").into_widget(), Text::new("B").into_widget()])
                            .key(GlobalKey::new().into())
                            .controller(controller.clone()),
                    )
                };

                let frame = build_frame(&tester);
                tester.pump_widget(frame);

                // The original TabBar will be disposed. The controller should no
                // longer have any listeners from the original TabBar.
                let frame = build_frame(&tester);
                tester.pump_widget(frame);

                controller.set_index(&mut tester.cell.borrow_mut(), 1);
                tester.pump(Duration::from_millis(300));
            });
        }

        /// tabs_utils' `TabBody`: a page that logs its state's `init` and `dispose`.
        #[derive(Debug)]
        struct TabBody {
            index: usize,
            log: Rc<RefCell<Vec<String>>>,
            marker: String,
        }

        impl StatefulWidget for TabBody {
            type State = TabBodyState;

            fn create_state(&self) -> TabBodyState {
                TabBodyState {
                    state: StateData::new(),
                }
            }
        }

        struct TabBodyState {
            state: StateData<TabBody>,
        }

        impl State for TabBodyState {
            type Widget = TabBody;
            waft_widgets::state_accessors!();

            fn init_state(self: Handle<Self>, app: &mut App) {
                let widget = self.widget(app);
                widget
                    .log
                    .borrow_mut()
                    .push(format!("init: {}", widget.index));
            }

            fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &TabBody) {
                assert_eq!(old_widget.index, self.widget(app).index);
            }

            fn dispose(self: Handle<Self>, app: &mut App) {
                let widget = self.widget(app);
                widget
                    .log
                    .borrow_mut()
                    .push(format!("dispose: {}", widget.index));
            }

            fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
                let widget = self.widget(app);
                let label = if widget.marker.is_empty() {
                    widget.index.to_string()
                } else {
                    format!("{}-{}", widget.index, widget.marker)
                };
                Center::new().child(Text::new(label)).into_widget()
            }
        }

        /// The `TabBarView children updated` group's `buildFrameWithMarker`.
        fn build_frame_with_marker(
            tester: &WidgetTester,
            log: &Rc<RefCell<Vec<String>>>,
            marker: &str,
        ) -> WidgetRef {
            let theme = theme(tester, None, None);
            material_app(
                theme,
                DefaultTabController::new(
                    3,
                    Scaffold::new()
                        .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                            AppBar::new()
                                .bottom(waft_widgets::PreferredSizeWidgetRef::new(TabBar::new(
                                    text_tabs(&["A", "B", "C"]),
                                )))
                                .title(Text::new("Tabs Test")),
                        ))
                        .body(TabBarView::new((0..3).map(|index| {
                            TabBody {
                                index,
                                log: Rc::clone(log),
                                marker: marker.to_owned(),
                            }
                            .into_widget()
                        }))),
                )
                .animation_duration(Duration::from_secs(1))
                .into_widget(),
            )
        }

        // 'TabBarView children can be updated during animation to an adjacent tab'.
        #[test]
        fn tab_bar_view_children_can_be_updated_while_it_animates_to_an_adjacent_tab() {
            with_main_thread_stack(|| {
                // Regression test for https://github.com/flutter/flutter/issues/107399
                let mut tester = tester();
                let log = Rc::new(RefCell::new(Vec::new()));

                let initial_marker = "before";
                let frame = build_frame_with_marker(&tester, &log, initial_marker);
                tester.pump_widget(frame);
                assert_eq!(*log.borrow(), ["init: 0"]);
                assert_eq!(tester.find_onstage_text("0-before").len(), 1);

                // Select the second tab and wait until the transition starts
                tap_text(&mut tester, "B");
                tester.pump(Duration::from_millis(100));

                // Check that both TabBody's are instantiated while the transition is animating
                tester.pump(Duration::from_millis(400));
                assert_eq!(*log.borrow(), ["init: 0", "init: 1"]);

                // Update the TabBody's states while the transition is animating
                let updated_marker = "after";
                let frame = build_frame_with_marker(&tester, &log, updated_marker);
                tester.pump_widget(frame);

                // Wait until the transition ends
                tester.pump_and_settle();

                // The TabBody state of the second TabBar should have been updated
                assert!(tester.find_onstage_text("1-before").is_empty());
                assert_eq!(tester.find_onstage_text("1-after").len(), 1);
            });
        }

        // 'TabBarView children can be updated during animation to a non adjacent tab'.
        #[test]
        fn tab_bar_view_children_can_be_updated_while_it_animates_to_a_non_adjacent_tab() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let log = Rc::new(RefCell::new(Vec::new()));

                let initial_marker = "before";
                let frame = build_frame_with_marker(&tester, &log, initial_marker);
                tester.pump_widget(frame);
                assert_eq!(*log.borrow(), ["init: 0"]);
                assert_eq!(tester.find_onstage_text("0-before").len(), 1);

                // Select the third tab and wait until the transition starts
                tap_text(&mut tester, "C");
                tester.pump(Duration::from_millis(100));

                // Check that both TabBody's are instantiated while the transition is animating
                tester.pump(Duration::from_millis(400));
                assert_eq!(*log.borrow(), ["init: 0", "init: 2"]);

                // Update the TabBody's states while the transition is animating
                let updated_marker = "after";
                let frame = build_frame_with_marker(&tester, &log, updated_marker);
                tester.pump_widget(frame);

                // Wait until the transition ends
                tester.pump_and_settle();

                // The TabBody state of the third TabBar should have been updated
                assert!(tester.find_onstage_text("2-before").is_empty());
                assert_eq!(tester.find_onstage_text("2-after").len(), 1);
            });
        }

        /// The centred 400 square `TabBarView` of three pages, under only a `Directionality`, that the
        /// scrolls-end cases build.
        fn centered_tab_bar_view(view: TabBarView) -> WidgetRef {
            Directionality::new(
                TextDirection::Ltr,
                SizedBox::expand()
                    .child(Center::new().child(SizedBox::square(Some(400.0)).child(view))),
            )
            .into_widget()
        }

        /// The scrolls-end cases' body: jumps near the pages and checks the index the jumps settle.
        fn check_jumps_close_to_a_new_page(
            tester: &mut WidgetTester,
            tab_controller: Handle<TabController>,
        ) {
            assert_eq!(index(tester, tab_controller), 1);

            let page_controller = page_controller(tester);

            // The TabBarView's page width is 400, so page 0 is at scroll offset 0.0,
            // page 1 is at 400.0, page 2 is at 800.0.

            assert_eq!(pixels(tester, page_controller.handle()), 400.0);

            let jump_to = |tester: &mut WidgetTester, value: f64| {
                page_controller.jump_to(&mut tester.cell.borrow_mut(), value);
            };

            // Not close enough to switch to page 2
            jump_to(tester, 500.0);
            assert_eq!(index(tester, tab_controller), 1);

            // Close enough to switch to page 2
            jump_to(tester, 700.0);
            assert_eq!(index(tester, tab_controller), 2);

            // Same behavior going left: not left enough to get to page 0
            jump_to(tester, 300.0);
            assert_eq!(index(tester, tab_controller), 1);

            // Left enough to get to page 0
            jump_to(tester, 100.0);
            assert_eq!(index(tester, tab_controller), 0);
        }

        // 'TabBarView scrolls end close to a new page'.
        #[test]
        fn a_tab_bar_view_scrolled_close_to_a_page_selects_it() {
            with_main_thread_stack(|| {
                // This is a regression test for https://github.com/flutter/flutter/issues/9375
                let mut tester = tester();
                let tab_controller = create_tab_controller(&tester, 3, 1, None);
                tester.pump_widget(centered_tab_bar_view(
                    TabBarView::new(numbered_pages(3)).controller(tab_controller.clone()),
                ));
                check_jumps_close_to_a_new_page(&mut tester, tab_controller.handle());
            });
        }

        // 'On going TabBarView animation can be interrupted by a new animation'.
        #[test]
        fn an_ongoing_tab_bar_view_animation_can_be_interrupted_by_a_new_one() {
            with_main_thread_stack(|| {
                // This is a regression test for https://github.com/flutter/flutter/issues/132293.
                let mut tester = tester();
                let tabs = ["A", "B", "C"];
                let tab_controller = create_tab_controller(&tester, 3, 0, None);
                let frame = boilerplate_ltr(
                    &tester,
                    bar_over_view(
                        &tabs,
                        tab_controller.clone(),
                        TabBarView::new(numbered_pages(3)),
                    ),
                );
                tester.pump_widget(frame);

                // First page is visible.
                assert_eq!(index(&tester, tab_controller.handle()), 0);
                assert_eq!(tester.find_onstage_text("0").len(), 1);
                assert!(tester.find_onstage_text("1").is_empty());

                // Animate to the second page.
                tab_controller.animate_to(&mut tester.cell.borrow_mut(), 1, None, None);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(300));

                // Animate back to the first page before the previous animation ends.
                tab_controller.animate_to(&mut tester.cell.borrow_mut(), 0, None, None);
                tester.pump_and_settle();

                // First page should be visible.
                assert_eq!(index(&tester, tab_controller.handle()), 0);
                assert_eq!(tester.find_onstage_text("0").len(), 1);
                assert!(tester.find_onstage_text("1").is_empty());
            });
        }

        // 'Can switch to non-neighboring tab in nested TabBarView without crashing'.
        #[test]
        fn a_tab_bar_view_with_a_nested_tab_bar_view_switches_to_a_non_neighbouring_tab() {
            with_main_thread_stack(|| {
                // This is a regression test for https://github.com/flutter/flutter/issues/18756
                let mut tester = tester();
                let main_tab_controller = create_tab_controller(&tester, 4, 0, None);
                let nested_tab_controller = create_tab_controller(&tester, 2, 0, None);
                let theme = theme(&tester, None, None);
                let add_tab = |label: &str| {
                    Tab::new()
                        .icon(waft_widgets::Icon::new(Some(crate::icons::Icons::add())))
                        .text(label)
                        .into_widget()
                };
                let colored =
                    |color: Color| waft_widgets::Container::new().color(color).into_widget();

                tester.pump_widget(material_app(
                    theme,
                    Scaffold::new()
                        .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                            AppBar::new()
                                .title(Text::new("Exception for Nested Tabs"))
                                .bottom(waft_widgets::PreferredSizeWidgetRef::new(
                                    TabBar::new(["A", "B", "C", "D"].map(add_tab))
                                        .controller(main_tab_controller.clone()),
                                )),
                        ))
                        .body(
                            TabBarView::new([
                                colored(Colors::RED.primary()),
                                waft_widgets::ColoredBox::new(Colors::BLUE.primary())
                                    .child(
                                        Column::new().children([
                                            TabBar::new(text_tabs(&["Yellow", "Grey"]))
                                                .controller(nested_tab_controller.clone())
                                                .into_widget(),
                                            Expanded::new(
                                                TabBarView::new([
                                                    colored(Colors::YELLOW.primary()),
                                                    colored(Colors::GREY.primary()),
                                                ])
                                                .controller(nested_tab_controller),
                                            )
                                            .into_widget(),
                                        ]),
                                    )
                                    .into_widget(),
                                colored(Colors::GREEN.primary()),
                                colored(Colors::INDIGO.primary()),
                            ])
                            .controller(main_tab_controller.clone()),
                        )
                        .into_widget(),
                ));

                // expect first tab to be selected
                assert_eq!(index(&tester, main_tab_controller.handle()), 0);

                // tap on third tab
                tap_text(&mut tester, "C");
                tester.pump_and_settle();

                // expect third tab to be selected without exceptions
                assert_eq!(index(&tester, main_tab_controller.handle()), 2);
            });
        }

        /// tabs_utils' `TabKeepAliveInk`: a page kept alive whose text is in an `Ink`.
        #[derive(Debug)]
        struct TabKeepAliveInk {
            title: String,
        }

        impl StatefulWidget for TabKeepAliveInk {
            type State = TabKeepAliveInkState;

            fn create_state(&self) -> TabKeepAliveInkState {
                TabKeepAliveInkState {
                    state: StateData::new(),
                    keep_alive: waft_widgets::AutomaticKeepAliveClientMixinData::default(),
                }
            }
        }

        struct TabKeepAliveInkState {
            state: StateData<TabKeepAliveInk>,
            keep_alive: waft_widgets::AutomaticKeepAliveClientMixinData,
        }

        impl waft_widgets::AutomaticKeepAliveClientMixin for TabKeepAliveInkState {
            fn automatic_keep_alive_client_data(
                self: Handle<Self>,
                app: &App,
            ) -> &waft_widgets::AutomaticKeepAliveClientMixinData {
                &app.get(self).keep_alive
            }

            fn automatic_keep_alive_client_data_mut(
                self: Handle<Self>,
                app: &mut App,
            ) -> &mut waft_widgets::AutomaticKeepAliveClientMixinData {
                &mut app.get_mut(self).keep_alive
            }

            fn want_keep_alive(self: Handle<Self>, _app: &App) -> bool {
                true
            }
        }

        impl State for TabKeepAliveInkState {
            type Widget = TabKeepAliveInk;
            waft_widgets::state_accessors!();

            fn init_state(self: Handle<Self>, app: &mut App) {
                waft_widgets::AutomaticKeepAliveClientMixin::init_state(self, app);
            }

            fn deactivate(self: Handle<Self>, app: &mut App) {
                waft_widgets::AutomaticKeepAliveClientMixin::deactivate(self, app);
            }

            fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
                waft_widgets::AutomaticKeepAliveClientMixin::build(self, app);
                crate::ink_decoration::Ink::new()
                    .child(Text::new(self.widget(app).title.clone()))
                    .into_widget()
            }
        }

        // 'TabBarView can warp when child is kept alive and contains ink'.
        #[test]
        fn a_tab_bar_view_warps_past_a_kept_alive_page_with_ink() {
            with_main_thread_stack(|| {
                // Regression test for https://github.com/flutter/flutter/issues/57662.
                let mut tester = tester();
                let controller = create_tab_controller(&tester, 3, 0, None);
                let frame = boilerplate_ltr(
                    &tester,
                    TabBarView::new([
                        Text::new("Page 1").into_widget(),
                        Text::new("Page 2").into_widget(),
                        TabKeepAliveInk {
                            title: "Page 3".to_owned(),
                        }
                        .into_widget(),
                    ])
                    .controller(controller.clone()),
                );
                tester.pump_widget(frame);

                assert_eq!(index(&tester, controller.handle()), 0);
                assert_eq!(tester.find_onstage_text("Page 1").len(), 1);
                assert!(tester.find_onstage_text("Page 3").is_empty());

                controller.set_index(&mut tester.cell.borrow_mut(), 2);
                tester.pump_and_settle();
                assert!(tester.find_onstage_text("Page 1").is_empty());
                assert_eq!(tester.find_onstage_text("Page 3").len(), 1);

                controller.set_index(&mut tester.cell.borrow_mut(), 0);
                tester.pump_and_settle();
                assert_eq!(tester.find_onstage_text("Page 1").len(), 1);
                assert!(tester.find_onstage_text("Page 3").is_empty());
            });
        }

        /// tabs_utils' `TabBarTestScrollPhysics`: turns a user offset of 10 into 20, on a stiffer
        /// spring.
        #[derive(Debug, Default)]
        struct TabBarTestScrollPhysics {
            parent: Option<ScrollPhysicsRef>,
        }

        impl ScrollPhysics for TabBarTestScrollPhysics {
            fn as_any(&self) -> &dyn Any {
                self
            }

            fn parent(&self) -> Option<&ScrollPhysicsRef> {
                self.parent.as_ref()
            }

            fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
                Rc::new(TabBarTestScrollPhysics {
                    parent: waft_widgets::ScrollPhysicsBase::build_parent(self, ancestor),
                })
            }

            fn apply_physics_to_user_offset(
                &self,
                _position: &dyn waft_widgets::ScrollMetrics,
                offset: f64,
            ) -> f64 {
                if offset == 10.0 { 20.0 } else { offset }
            }

            fn spring(&self) -> waft_physics::SpringDescription {
                waft_physics::SpringDescription::with_damping_ratio_value(0.5, 500.0, 1.1)
            }
        }

        // 'TabBarView scrolls end close to a new page with custom physics'.
        #[test]
        fn a_tab_bar_view_with_custom_physics_scrolled_close_to_a_page_selects_it() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_controller = create_tab_controller(&tester, 3, 1, None);
                tester.pump_widget(centered_tab_bar_view(
                    TabBarView::new(numbered_pages(3))
                        .controller(tab_controller.clone())
                        .physics(Rc::new(TabBarTestScrollPhysics::default())),
                ));
                check_jumps_close_to_a_new_page(&mut tester, tab_controller.handle());
            });
        }

        /// `List<Tab>.generate(count, (int index) => Tab(text: 'TAB #$index'))`.
        fn numbered_tabs(count: usize) -> Vec<WidgetRef> {
            (0..count)
                .map(|index| Tab::new().text(format!("TAB #{index}")).into_widget())
                .collect()
        }

        // 'TabBar accepts custom physics'.
        #[test]
        fn a_tab_bar_keeps_the_physics_it_is_given() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = create_tab_controller(&tester, 20, 19, None);
                let frame = boilerplate_ltr(
                    &tester,
                    TabBar::new(numbered_tabs(20))
                        .is_scrollable(true)
                        .controller(controller)
                        .physics(Rc::new(TabBarTestScrollPhysics::default())),
                );
                tester.pump_widget(frame);

                let physics = tester.widget(only::<TabBar>(&tester), |tab_bar: &TabBar| {
                    tab_bar.physics.clone().expect("given physics")
                });
                // Dart's `TabMockScrollMetrics` is a fake the physics never reads.
                let metrics = waft_widgets::FixedScrollMetrics::new(
                    None,
                    None,
                    None,
                    None,
                    waft_painting::AxisDirection::Right,
                    1.0,
                );
                let position = physics.apply_physics_to_user_offset(&metrics, 10.0);

                assert_eq!(position, 20.0);
            });
        }

        /// The pixels of the only `Scrollable`'s position: Dart's
        /// `tester.state<ScrollableState>(find.byType(Scrollable)).position.pixels`.
        fn scrollable_pixels(tester: &WidgetTester) -> f64 {
            let element = only::<waft_widgets::Scrollable>(tester);
            let app = tester.cell.borrow();
            let state = element
                .state_handle::<ScrollableState>(&app)
                .expect("a scrollable state");
            state.position(&app).pixels(&app)
        }

        fn new_tab_bar_scroll_controller(tester: &WidgetTester) -> Ref<TabBarScrollController> {
            TabBarScrollController::new(&mut tester.cell.borrow_mut())
        }

        /// Whether `controller.debug_check_has_tab_bar_state()` asserts.
        fn check_has_tab_bar_state_asserts(
            tester: &WidgetTester,
            controller: Handle<TabBarScrollController>,
        ) -> bool {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                controller.debug_check_has_tab_bar_state(&tester.cell.borrow())
            }))
            .is_err()
        }

        // 'TabBar accepts external TabBarScrollController'.
        #[test]
        fn a_tab_bar_scrolls_with_an_external_tab_bar_scroll_controller() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_bar_controller = create_tab_controller(&tester, 6, 5, None);
                let tab_scroll_controller = new_tab_bar_scroll_controller(&tester);

                let frame = boilerplate_ltr(
                    &tester,
                    TabBar::new(numbered_tabs(6))
                        .is_scrollable(true)
                        .controller(tab_bar_controller)
                        .scroll_controller(tab_scroll_controller.clone()),
                );
                tester.pump_widget(frame);

                tab_scroll_controller.jump_to(&mut tester.cell.borrow_mut(), 50.0);
                tester.pump(Duration::ZERO);

                assert_eq!(scrollable_pixels(&tester), 50.0);
            });
        }

        // 'TabBar attaches state to internal TabBarScrollController in didUpdateWidget'.
        #[test]
        fn a_tab_bar_moving_to_its_internal_scroll_controller_keeps_its_pixels() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_bar_controller = create_tab_controller(&tester, 6, 5, None);
                let tab_scroll_controller = new_tab_bar_scroll_controller(&tester);

                // Attach to the external controller.
                let frame = boilerplate_ltr(
                    &tester,
                    TabBar::new(numbered_tabs(6))
                        .is_scrollable(true)
                        .controller(tab_bar_controller.clone())
                        .scroll_controller(tab_scroll_controller.clone()),
                );
                tester.pump_widget(frame);

                let old_pixels = scrollable_pixels(&tester);

                // Trigger didUpdateWidget for the change to the internal controller.
                let frame = boilerplate_ltr(
                    &tester,
                    TabBar::new(numbered_tabs(6))
                        .is_scrollable(true)
                        .controller(tab_bar_controller),
                );
                tester.pump_widget(frame);

                // Creating the new scroll position should not throw.
                // The pixels of the new position should be the same as before the update.
                assert_eq!(scrollable_pixels(&tester), old_pixels);
                assert!(check_has_tab_bar_state_asserts(
                    &tester,
                    tab_scroll_controller.handle()
                ));
            });
        }

        // 'TabBar attaches state to external TabBarScrollController in didUpdateWidget'.
        #[test]
        fn a_tab_bar_moving_to_an_external_scroll_controller_keeps_its_pixels() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_bar_controller = create_tab_controller(&tester, 6, 5, None);
                let tab_scroll_controller = new_tab_bar_scroll_controller(&tester);

                // Attach to the internal controller.
                let frame = boilerplate_ltr(
                    &tester,
                    TabBar::new(numbered_tabs(6))
                        .is_scrollable(true)
                        .controller(tab_bar_controller.clone()),
                );
                tester.pump_widget(frame);

                let old_pixels = scrollable_pixels(&tester);

                // Trigger didUpdateWidget for the change to the external controller.
                let frame = boilerplate_ltr(
                    &tester,
                    TabBar::new(numbered_tabs(6))
                        .is_scrollable(true)
                        .controller(tab_bar_controller)
                        .scroll_controller(tab_scroll_controller.clone()),
                );
                tester.pump_widget(frame);

                // Creating the new scroll position should not throw.
                // The pixels of the new position should be the same as before the update.
                assert_eq!(scrollable_pixels(&tester), old_pixels);

                // This should not throw, since the tab bar is attached to the external controller.
                assert!(tab_scroll_controller.debug_check_has_tab_bar_state(&tester.cell.borrow()));
            });
        }

        /// `<Tab>[for (int i = 0; i < count; i++) Tab(text: 'Tab $i')]`.
        fn tabs_numbered_from_zero(count: usize) -> Vec<WidgetRef> {
            (0..count)
                .map(|index| Tab::new().text(format!("Tab {index}")).into_widget())
                .collect()
        }

        // 'TabBar correctly detaches old external TabBarScrollController when switched to a new one'.
        #[test]
        fn a_tab_bar_detaches_the_external_scroll_controller_it_is_switched_from() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_controller = create_tab_controller(&tester, 10, 0, None);
                let controller_a = new_tab_bar_scroll_controller(&tester);
                let controller_b = new_tab_bar_scroll_controller(&tester);

                let frame = boilerplate_ltr(
                    &tester,
                    TabBar::new(tabs_numbered_from_zero(10))
                        .is_scrollable(true)
                        .controller(tab_controller.clone())
                        .scroll_controller(controller_a.clone()),
                );
                tester.pump_widget(frame);

                assert!(controller_a.debug_check_has_tab_bar_state(&tester.cell.borrow()));
                assert!(check_has_tab_bar_state_asserts(
                    &tester,
                    controller_b.handle()
                ));

                // Switch to controllerB
                let frame = boilerplate_ltr(
                    &tester,
                    TabBar::new(tabs_numbered_from_zero(10))
                        .is_scrollable(true)
                        .controller(tab_controller)
                        .scroll_controller(controller_b.clone()),
                );
                tester.pump_widget(frame);

                assert!(controller_b.debug_check_has_tab_bar_state(&tester.cell.borrow()));
                assert!(check_has_tab_bar_state_asserts(
                    &tester,
                    controller_a.handle()
                ));
            });
        }

        // 'TabBar correctly detaches external TabBarScrollController when disposed'.
        #[test]
        fn a_disposed_tab_bar_detaches_its_external_scroll_controller() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_controller = create_tab_controller(&tester, 10, 0, None);
                let controller = new_tab_bar_scroll_controller(&tester);

                let frame = boilerplate_ltr(
                    &tester,
                    TabBar::new(tabs_numbered_from_zero(10))
                        .is_scrollable(true)
                        .controller(tab_controller)
                        .scroll_controller(controller.clone()),
                );
                tester.pump_widget(frame);

                assert!(controller.debug_check_has_tab_bar_state(&tester.cell.borrow()));

                // Dispose the TabBar by pumping a different widget
                let frame = boilerplate_ltr(&tester, SizedBox::shrink());
                tester.pump_widget(frame);

                assert!(check_has_tab_bar_state_asserts(
                    &tester,
                    controller.handle()
                ));
            });
        }

        /// The scrollable `TabBar` of six tabs under a `RawScrollbar` on the same external scroll
        /// controller, that the scrollbar cases build.
        fn pump_tab_bar_in_raw_scrollbar(tester: &mut WidgetTester) {
            let tab_bar_controller = create_tab_controller(tester, 6, 5, None);
            let tab_scroll_controller = new_tab_bar_scroll_controller(tester);
            let frame = boilerplate_ltr(
                tester,
                waft_widgets::RawScrollbar::new(
                    TabBar::new(numbered_tabs(6))
                        .is_scrollable(true)
                        .controller(tab_bar_controller)
                        .scroll_controller(tab_scroll_controller.clone()),
                )
                .controller(tab_scroll_controller.erase()),
            );
            tester.pump_widget(frame);
        }

        // 'TabBar can be wrapped with RawScrollbar'.
        #[test]
        fn a_tab_bar_can_be_wrapped_in_a_raw_scrollbar() {
            with_main_thread_stack(|| {
                // Regression test for https://github.com/flutter/flutter/issues/124608
                let mut tester = tester();
                pump_tab_bar_in_raw_scrollbar(&mut tester);
            });
        }

        // 'TabBar can show scrollbar on hover'.
        #[test]
        fn a_tab_bar_in_a_raw_scrollbar_shows_the_scrollbar_on_hover() {
            with_main_thread_stack(|| {
                // Regression test for https://github.com/flutter/flutter/issues/124608
                let mut tester = tester();
                pump_tab_bar_in_raw_scrollbar(&mut tester);

                let tab1 = tester.find_onstage_text("TAB #1");
                assert_eq!(tab1.len(), 1);

                // Hover over the tab bar and verify that the scrollbar is shown.
                let mut gesture = WidgetTester::mouse(1, 0);
                tester.add_pointer(&mut gesture, Offset::ZERO);
                let center = center_of(&tester, tab1[0]);
                tester.move_to(&mut gesture, center, false);
                tester.pump_and_settle();

                // Dart's `paints` on the scrollbar records its whole subtree, repaint boundaries and
                // the ink of the materials in it included. Here that is the scrollbar's foreground
                // `CustomPaint`, under its repaint boundary, and the ink of each material under it.
                let scrollbar = only::<waft_widgets::RawScrollbar>(&tester);
                let custom_paint = *tester
                    .descendants_of_type::<CustomPaint>(scrollbar)
                    .first()
                    .expect("the scrollbar's CustomPaint");
                let painter = tester.render_box_of(custom_paint);
                let mut drawn = tester.painted(painter.as_object());
                let scrollbar_render = tester.render_box_of(scrollbar).as_object();
                for material in tester.all::<crate::material::RenderInkFeatures>() {
                    let under_scrollbar = {
                        let app = tester.cell.borrow();
                        let mut ancestor = material.as_object().parent(&app);
                        loop {
                            match ancestor {
                                Some(object) if object == scrollbar_render => break true,
                                Some(object) => ancestor = object.parent(&app),
                                None => break false,
                            }
                        }
                    };
                    if under_scrollbar {
                        let mut canvas = Canvas::new();
                        material.paint_ink_features(
                            &mut tester.cell.borrow_mut(),
                            &mut canvas,
                            Offset::ZERO,
                        );
                        drawn.extend(crate::test_support::Drawn::draws(&canvas.build()));
                    }
                }
                assert!(
                    drawn
                        .iter()
                        .any(|drawn| matches!(drawn, crate::test_support::Drawn::Rect(..))),
                    "the scrollbar paints a rect: {drawn:?}"
                );
            });
        }

        // 'Scrollable TabBar with a non-zero TabController initialIndex'.
        #[test]
        fn a_scrollable_tab_bar_shows_a_non_zero_initial_index_right_justified() {
            with_main_thread_stack(|| {
                // This is a regression test for https://github.com/flutter/flutter/issues/9374
                let mut tester = tester();
                let controller = create_tab_controller(&tester, 20, 19, None);
                let frame = boilerplate_ltr(
                    &tester,
                    TabBar::new(numbered_tabs(20))
                        .is_scrollable(true)
                        .controller(controller),
                );
                tester.pump_widget(frame);

                // The initialIndex tab should be visible and right justified
                let label = tester.find_onstage_text("TAB #19");
                assert_eq!(label.len(), 1);

                // Tabs have a minimum width of 72.0 and 'TAB #19' is wider than
                // that. Tabs are padded horizontally with kTabLabelPadding.
                let tab_right = 800.0 - K_TAB_LABEL_PADDING.right;

                let tab = tester.ancestor_of_type::<Tab>(label[0]);
                let right = rect_of(&tester, tab).right;
                // Dart's `moreOrLessEquals` epsilon is 1e-10; the global rect goes through valo's f32
                // `Matrix4` at a scroll offset near 1725, which rounds it to about 1e-4.
                assert!(close(right, tab_right, 1e-4), "{right}");
            });
        }

        /// tabs_utils' `tabIndicatorRRectElasticAnimation`.
        fn tab_indicator_rrect_elastic_animation(
            tab_bar_height: f64,
            from_rect: Rect,
            to_rect: Rect,
            progress: f64,
        ) -> waft_embedder::RRect {
            const INDICATOR_WEIGHT: f64 = 3.0;
            let left_fraction = 1.0 - ((progress * PI) / 2.0).cos();
            let right_fraction = ((progress * PI) / 2.0).sin();
            let lerp =
                |a: f64, b: f64, t: f64| lerp_double(Some(a), Some(b), t).expect("two values");

            waft_embedder::RRect::from_ltrb_and_corners(
                lerp(from_rect.left, to_rect.left, left_fraction),
                tab_bar_height - INDICATOR_WEIGHT,
                lerp(from_rect.right, to_rect.right, right_fraction),
                tab_bar_height,
                waft_embedder::Radius::circular(INDICATOR_WEIGHT),
                waft_embedder::Radius::circular(INDICATOR_WEIGHT),
                waft_embedder::Radius::ZERO,
                waft_embedder::Radius::ZERO,
            )
        }

        // 'Indicator elastic animation'.
        #[test]
        fn the_label_indicator_stretches_elastically_as_the_offset_changes() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_WIDTH: f64 = 50.0;
                let tabs = (0..4).map(|index| {
                    Tab::new()
                        .key(ValueKey::new(index).into())
                        .child(SizedBox::new().width(INDICATOR_WIDTH))
                        .into_widget()
                });
                let controller = create_tab_controller(&tester, 4, 0, None);
                let theme = theme(&tester, None, None);
                let inner = boilerplate_ltr(
                    &tester,
                    waft_widgets::Container::new()
                        .alignment(AlignmentGeometry::TOP_LEFT)
                        .child(TabBar::new(tabs).controller(controller.clone())),
                );
                tester.pump_widget(material_app(theme, inner));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                let height = tab_bar_box.size(&tester.cell.borrow()).height();
                assert_eq!(height, 48.0);

                let from_rect = Rect::from_ltrb(75.0, 0.0, 125.0, 48.0);
                let mut to_rect = Rect::from_ltrb(75.0, 0.0, 125.0, 48.0);
                let paints = |tester: &WidgetTester, rrect| {
                    tester.paints_rrect(tab_bar_box.as_object(), rrect, None)
                };
                assert!(paints(
                    &tester,
                    tab_indicator_rrect_elastic_animation(height, from_rect, to_rect, 0.0)
                ));

                controller.set_offset(&mut tester.cell.borrow_mut(), 0.2);
                tester.pump(Duration::ZERO);
                to_rect = Rect::from_ltrb(275.0, 0.0, 325.0, 48.0);
                assert!(paints(
                    &tester,
                    tab_indicator_rrect_elastic_animation(height, from_rect, to_rect, 0.2)
                ));

                controller.set_offset(&mut tester.cell.borrow_mut(), 0.5);
                tester.pump(Duration::ZERO);
                assert!(paints(
                    &tester,
                    tab_indicator_rrect_elastic_animation(height, from_rect, to_rect, 0.5)
                ));

                controller.set_offset(&mut tester.cell.borrow_mut(), 1.0);
                tester.pump(Duration::ZERO);
                // When the animation is completed, no stretch is applied.
                assert!(paints(
                    &tester,
                    tab_indicator_rrect_elastic_animation(height, from_rect, to_rect, 1.0)
                ));
            });
        }

        // 'ensureVisible does not move TabViews'.
        #[test]
        fn ensure_visible_does_not_move_tab_views() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = create_tab_controller(&tester, 3, 0, None);

                let pages = (0..3).map(|page_index| {
                    waft_widgets::ListView::new()
                        .key(key(format!("list_{page_index}")))
                        .children((0..100).map(|list_index| {
                            let cell = |column: usize, color: Color| {
                                waft_widgets::Container::new()
                                    .key(key(format!("{page_index}_{list_index}_{column}")))
                                    .color(color)
                                    .width(200.0)
                                    .height(10.0)
                                    .into_widget()
                            };
                            Row::new()
                                .children([
                                    cell(0, Colors::RED.primary()),
                                    cell(1, Colors::BLUE.primary()),
                                    cell(2, Colors::GREEN.primary()),
                                ])
                                .into_widget()
                        }))
                        .into_widget()
                });
                tester.pump_widget(
                    Directionality::new(
                        TextDirection::Ltr,
                        TabBarView::new(pages.collect::<Vec<_>>()).controller(controller.clone()),
                    )
                    .into_widget(),
                );

                let found = |tester: &WidgetTester, name: &str| {
                    tester.find_onstage_by_key(&key(name)).len()
                };
                let ensure_visible = |tester: &mut WidgetTester, name: &str| {
                    let element = tester.find_onstage_by_key(&key(name))[0];
                    let mut app = tester.cell.borrow_mut();
                    drop(waft_widgets::Scrollable::ensure_visible(
                        &mut app,
                        element,
                        0.0,
                        Duration::ZERO,
                        Curves::ease(),
                        waft_widgets::ScrollPositionAlignmentPolicy::Explicit,
                    ));
                };

                assert_eq!(found(&tester, "list_0"), 1);
                assert_eq!(found(&tester, "list_1"), 0);
                assert_eq!(found(&tester, "0_25_2"), 1);
                assert_eq!(found(&tester, "1_25_2"), 0);
                assert_eq!(found(&tester, "1_25_0"), 0);

                ensure_visible(&mut tester, "0_25_2");
                tester.pump_and_settle();
                assert_eq!(found(&tester, "0_25_2"), 1);
                assert_eq!(found(&tester, "1_25_2"), 0);
                assert_eq!(found(&tester, "1_25_0"), 0);

                controller.set_index(&mut tester.cell.borrow_mut(), 1);
                tester.pump_and_settle();

                assert_eq!(found(&tester, "list_0"), 0);
                assert_eq!(found(&tester, "list_1"), 1);
                ensure_visible(&mut tester, "1_25_2");
                tester.pump_and_settle();

                assert_eq!(found(&tester, "0_25_2"), 0);
                assert_eq!(found(&tester, "1_25_2"), 1);
                assert_eq!(found(&tester, "1_25_0"), 1);

                ensure_visible(&mut tester, "1_25_0");
                tester.pump_and_settle();

                assert_eq!(found(&tester, "0_25_2"), 0);
                assert_eq!(found(&tester, "1_25_2"), 1);
                assert_eq!(found(&tester, "1_25_0"), 1);
            });
        }
    }

    mod indicators_and_controller_changes {
        //! tabs_test.dart cases from 'TabBar with indicatorWeight, indicatorPadding (LTR)' through
        //! 'Number of tabs can be updated dynamically'.

        use super::*;

        use std::cell::{Cell, RefCell};
        use std::panic::{AssertUnwindSafe, catch_unwind};

        use waft_painting::EdgeInsets;

        use crate::test_support::{Drawn, TestVSync};

        /// Dart's `createTabController(length:, vsync: const TestVSync())`.
        fn new_controller(tester: &WidgetTester, length: usize) -> Ref<TabController> {
            let mut app = tester.cell.borrow_mut();
            TabController::new(&mut app, 0, None, length, TestVSync)
        }

        /// Dart's `Container(alignment: Alignment.topLeft, child: child)`.
        fn top_left<K>(child: impl IntoWidget<K>) -> WidgetRef {
            waft_widgets::Container::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .child(child)
                .into_widget()
        }

        /// `n` tabs `Tab(text: 'Tab $index')`.
        fn text_tabs(n: usize) -> Vec<WidgetRef> {
            (0..n)
                .map(|index| Tab::new().text(format!("Tab {index}")).into_widget())
                .collect()
        }

        /// Three keyed `SizedBox`es of the given sizes, Dart's `SizedBox(key: UniqueKey(), ..)` tabs.
        fn sized_tabs(sizes: &[(f64, f64)]) -> (Vec<GlobalKey>, Vec<WidgetRef>) {
            let keys: Vec<GlobalKey> = sizes.iter().map(|_| GlobalKey::new()).collect();
            let tabs = sizes
                .iter()
                .zip(&keys)
                .map(|((width, height), key)| {
                    SizedBox::new()
                        .key(key.clone().into())
                        .width(*width)
                        .height(*height)
                        .into_widget()
                })
                .collect();
            (keys, tabs)
        }

        /// Dart's `tester.getRect(find.byKey(key))`.
        fn rect_of_key(tester: &WidgetTester, key: &GlobalKey) -> Rect {
            tester.get_rect(tester.render_object(key).as_box().expect("a render box"))
        }

        /// The pictures `render` and its subtree record, painted at the origin, the ones in nested
        /// layers (a scroll view's clip) included: Dart's `paints` records what a pushed layer paints
        /// on its one canvas.
        fn deep_pictures(
            tester: &WidgetTester,
            render: AnyRenderObject,
        ) -> Vec<std::sync::Arc<waft_embedder::Picture>> {
            use waft_rendering::ContainerLayer;
            fn collect(
                app: &App,
                layer: waft_rendering::AnyContainerLayer,
                pictures: &mut Vec<std::sync::Arc<waft_embedder::Picture>>,
            ) {
                let mut child = layer.first_child(app);
                while let Some(current) = child {
                    if let Some(picture) = app
                        .handle::<waft_rendering::PictureLayer>(current.id())
                        .and_then(|picture| picture.picture(app))
                    {
                        pictures.push(std::sync::Arc::clone(picture));
                    } else if let Some(container) = current.as_container_layer() {
                        collect(app, container, pictures);
                    }
                    child = current.next_sibling(app);
                }
            }
            let mut app = tester.cell.borrow_mut();
            let layer = waft_rendering::OffsetLayer::new(&mut app, Offset::ZERO);
            let container = layer.as_container_layer();
            let bounds = render.paint_bounds(&app);
            let mut context = PaintingContext::new(container, bounds);
            context.paint_child(&mut app, render, Offset::ZERO);
            context.stop_recording_if_needed(&mut app);
            let mut pictures = Vec::new();
            collect(&app, container, &mut pictures);
            pictures
        }

        /// The lines of [`deep_pictures`], as `painted_lines` reads them.
        fn deep_lines(tester: &WidgetTester, render: AnyRenderObject) -> Vec<PaintedLine> {
            use waft_embedder::valo::{Op, PaintStyle as ValoPaintStyle};
            deep_pictures(tester, render)
                .iter()
                .flat_map(|picture| waft_embedder::flattened_ops(picture))
                .filter_map(|op| match op {
                    Op::DrawPath { path, paint, .. } => {
                        let ValoPaintStyle::Stroke(stroke) = paint.style else {
                            return None;
                        };
                        if path.elements().count() != 2 {
                            return None;
                        }
                        let bounds = path.bounds();
                        let channel = |value: f32| (value * 255.0).round() as u32;
                        let color = paint.color;
                        Some(PaintedLine {
                            color: Color::new(
                                (channel(color.a) << 24)
                                    | (channel(color.r) << 16)
                                    | (channel(color.g) << 8)
                                    | channel(color.b),
                            ),
                            stroke_width: f64::from(stroke.width),
                            p1: Offset::new(f64::from(bounds.x), f64::from(bounds.y)),
                            p2: Offset::new(
                                f64::from(bounds.x + bounds.width),
                                f64::from(bounds.y + bounds.height),
                            ),
                        })
                    }
                    _ => None,
                })
                .collect()
        }

        /// Dart's `paints..line(color:, strokeWidth:, p1:, p2:)`, `color` left out when `None`.
        fn paints_line(
            tester: &WidgetTester,
            render: AnyRenderBox,
            color: Option<Color>,
            stroke_width: f64,
            p1: Offset,
            p2: Offset,
        ) -> bool {
            let lines = deep_lines(tester, render.as_object());
            let found = lines.iter().any(|line| {
                color.is_none_or(|color| line.color.to_argb32() == color.to_argb32())
                    && line.stroke_width == stroke_width
                    && line.p1 == p1
                    && line.p2 == p2
            });
            if !found {
                eprintln!("lines painted: {lines:?}");
            }
            found
        }

        /// What [`deep_pictures`] draws, reduced.
        fn deep_painted(tester: &WidgetTester, render: AnyRenderObject) -> Vec<Drawn> {
            deep_pictures(tester, render)
                .iter()
                .flat_map(|picture| Drawn::draws(picture))
                .collect()
        }

        /// Dart's `paints..rect(rect:, color:)`.
        fn paints_rect(
            tester: &WidgetTester,
            render: AnyRenderBox,
            rect: Rect,
            color: Color,
        ) -> bool {
            let painted = deep_painted(tester, render.as_object());
            let found = painted.contains(&Drawn::Rect(color.to_argb32(), rect));
            if !found {
                eprintln!("painted: {painted:?}");
            }
            found
        }

        fn tab_bar_height(tester: &WidgetTester) -> f64 {
            first_render_box::<TabBar>(tester)
                .size(&tester.cell.borrow())
                .height()
        }

        fn tab_bar_size(tester: &WidgetTester) -> Size {
            first_render_box::<TabBar>(tester).size(&tester.cell.borrow())
        }

        fn set_index(tester: &WidgetTester, controller: Handle<TabController>, index: usize) {
            let mut app = tester.cell.borrow_mut();
            controller.set_index(&mut app, index);
        }

        /// The text of a panic: a `String` or a `&str` payload.
        fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
            panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string()))
                .unwrap_or_default()
        }

        const LABEL_LEFT: f64 = 16.0;
        const LABEL_RIGHT: f64 = 16.0;
        const LABEL_HORIZONTAL: f64 = 32.0;

        // 'TabBar with indicatorWeight, indicatorPadding (LTR)'.
        #[test]
        fn a_tab_bar_underline_takes_its_weight_and_padding_left_to_right() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_COLOR: Color = Color::new(0xFF00FF00);
                const INDICATOR_WEIGHT: f64 = 8.0;
                const PAD_LEFT: f64 = 8.0;
                const PAD_RIGHT: f64 = 4.0;
                let controller = new_controller(&tester, 4);
                let theme = theme(&tester, Some(false), None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    top_left(
                        TabBar::new(text_tabs(4))
                            .indicator_weight(INDICATOR_WEIGHT)
                            .indicator_color(INDICATOR_COLOR)
                            .indicator_padding(EdgeInsetsGeometry::only(
                                PAD_LEFT, 0.0, PAD_RIGHT, 0.0,
                            ))
                            .controller(controller.clone()),
                    ),
                ));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                assert_eq!(tab_bar_height(&tester), 54.0); // 54 = _kTabHeight(46) + indicatorWeight(8.0)

                const INDICATOR_Y: f64 = 54.0 - INDICATOR_WEIGHT / 2.0;
                let indicator_left = PAD_LEFT + INDICATOR_WEIGHT / 2.0;
                let indicator_right = 200.0 - (PAD_RIGHT + INDICATOR_WEIGHT / 2.0);
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    Some(INDICATOR_COLOR),
                    INDICATOR_WEIGHT,
                    Offset::new(indicator_left, INDICATOR_Y),
                    Offset::new(indicator_right, INDICATOR_Y),
                ));

                // Select tab 3
                set_index(&tester, controller.handle(), 3);
                tester.pump_and_settle();

                let indicator_left = 600.0 + PAD_LEFT + INDICATOR_WEIGHT / 2.0;
                let indicator_right = 800.0 - (PAD_RIGHT + INDICATOR_WEIGHT / 2.0);
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    Some(INDICATOR_COLOR),
                    INDICATOR_WEIGHT,
                    Offset::new(indicator_left, INDICATOR_Y),
                    Offset::new(indicator_right, INDICATOR_Y),
                ));
            });
        }

        // 'TabBar with indicatorWeight, indicatorPadding (RTL)'.
        #[test]
        fn a_tab_bar_underline_takes_its_weight_and_padding_right_to_left() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_COLOR: Color = Color::new(0xFF00FF00);
                const INDICATOR_WEIGHT: f64 = 8.0;
                const PAD_LEFT: f64 = 8.0;
                const PAD_RIGHT: f64 = 4.0;
                let controller = new_controller(&tester, 4);
                let theme = theme(&tester, Some(false), None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Rtl,
                    top_left(
                        TabBar::new(text_tabs(4))
                            .indicator_weight(INDICATOR_WEIGHT)
                            .indicator_color(INDICATOR_COLOR)
                            .indicator_padding(EdgeInsetsGeometry::only(
                                PAD_LEFT, 0.0, PAD_RIGHT, 0.0,
                            ))
                            .controller(controller.clone()),
                    ),
                ));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                assert_eq!(tab_bar_height(&tester), 54.0); // 54 = _kTabHeight(46) + indicatorWeight(8.0)
                assert_eq!(tab_bar_size(&tester).width(), 800.0);

                const INDICATOR_Y: f64 = 54.0 - INDICATOR_WEIGHT / 2.0;
                let indicator_left = 600.0 + PAD_LEFT + INDICATOR_WEIGHT / 2.0;
                let indicator_right = 800.0 - PAD_RIGHT - INDICATOR_WEIGHT / 2.0;
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    Some(INDICATOR_COLOR),
                    INDICATOR_WEIGHT,
                    Offset::new(indicator_left, INDICATOR_Y),
                    Offset::new(indicator_right, INDICATOR_Y),
                ));

                // Select tab 3
                set_index(&tester, controller.handle(), 3);
                tester.pump_and_settle();

                let indicator_left = PAD_LEFT + INDICATOR_WEIGHT / 2.0;
                let indicator_right = 200.0 - PAD_RIGHT - INDICATOR_WEIGHT / 2.0;
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    Some(INDICATOR_COLOR),
                    INDICATOR_WEIGHT,
                    Offset::new(indicator_left, INDICATOR_Y),
                    Offset::new(indicator_right, INDICATOR_Y),
                ));
            });
        }

        // 'TabBar changes indicator attributes'.
        #[test]
        fn a_rebuilt_tab_bar_paints_its_new_indicator_attributes() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = new_controller(&tester, 4);
                let build_frame = |tester: &WidgetTester,
                                   indicator_color: Color,
                                   indicator_weight: f64,
                                   pad_left: f64,
                                   pad_right: f64| {
                    let theme = theme(tester, Some(false), None);
                    boilerplate(
                        theme,
                        TextDirection::Ltr,
                        top_left(
                            TabBar::new(text_tabs(4))
                                .indicator_weight(indicator_weight)
                                .indicator_color(indicator_color)
                                .indicator_padding(EdgeInsetsGeometry::only(
                                    pad_left, 0.0, pad_right, 0.0,
                                ))
                                .controller(controller.clone()),
                        ),
                    )
                };

                let (indicator_color, indicator_weight, pad_left, pad_right) =
                    (Color::new(0xFF00FF00), 8.0, 8.0, 4.0);
                let frame = build_frame(
                    &tester,
                    indicator_color,
                    indicator_weight,
                    pad_left,
                    pad_right,
                );
                tester.pump_widget(frame);

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                assert_eq!(tab_bar_height(&tester), 54.0); // 54 = _kTabHeight(46) + indicatorWeight(8.0)

                let indicator_y = 54.0 - indicator_weight / 2.0;
                let indicator_left = pad_left + indicator_weight / 2.0;
                let indicator_right = 200.0 - (pad_right + indicator_weight / 2.0);
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    Some(indicator_color),
                    indicator_weight,
                    Offset::new(indicator_left, indicator_y),
                    Offset::new(indicator_right, indicator_y),
                ));

                let (indicator_color, indicator_weight, pad_left, pad_right) =
                    (Color::new(0xFF0000FF), 4.0, 4.0, 8.0);
                let frame = build_frame(
                    &tester,
                    indicator_color,
                    indicator_weight,
                    pad_left,
                    pad_right,
                );
                tester.pump_widget(frame);

                assert_eq!(tab_bar_box.size(&tester.cell.borrow()).height(), 50.0); // 54 = _kTabHeight(46) + indicatorWeight(4.0)

                let indicator_y = 50.0 - indicator_weight / 2.0;
                let indicator_left = pad_left + indicator_weight / 2.0;
                let indicator_right = 200.0 - (pad_right + indicator_weight / 2.0);
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    Some(indicator_color),
                    indicator_weight,
                    Offset::new(indicator_left, indicator_y),
                    Offset::new(indicator_right, indicator_y),
                ));
            });
        }

        /// The tab rects of a scrollable tab bar of three sized tabs at `label_left`/`label_right`
        /// padding, in the order they are laid out, and the bottom of the last.
        fn assert_three_tab_rects(
            tester: &WidgetTester,
            keys_in_order: [&GlobalKey; 3],
            sizes_in_order: [(f64, f64); 3],
            tab_bar_height: f64,
            indicator_weight: f64,
            label_left: f64,
            label_right: f64,
        ) -> (f64, f64, f64) {
            let mut tab_left = label_left;
            let mut tab_right = 0.0;
            let mut tab_bottom = 0.0;
            for (index, (key, (width, height))) in
                keys_in_order.iter().zip(sizes_in_order).enumerate()
            {
                if index > 0 {
                    tab_left = tab_right + label_right + label_left;
                }
                tab_right = tab_left + width;
                let tab_top = (tab_bar_height - indicator_weight - height) / 2.0;
                tab_bottom = tab_top + height;
                let tab_rect = Rect::from_ltrb(tab_left, tab_top, tab_right, tab_bottom);
                assert_eq!(rect_of_key(tester, key), tab_rect);
            }
            (tab_left, tab_right, tab_bottom)
        }

        // 'TabBar with directional indicatorPadding (LTR)'.
        #[test]
        fn a_directional_indicator_padding_starts_on_the_left_left_to_right() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let (keys, tabs) = sized_tabs(&[(130.0, 30.0), (140.0, 40.0), (150.0, 50.0)]);
                const INDICATOR_WEIGHT: f64 = 2.0; // the default
                let controller = new_controller(&tester, 3);
                let theme = theme(&tester, Some(false), None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    top_left(
                        TabBar::new(tabs)
                            .indicator_padding(EdgeInsetsDirectional::only(100.0, 0.0, 0.0, 0.0))
                            .is_scrollable(true)
                            .controller(controller),
                    ),
                ));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                const TAB_BAR_HEIGHT: f64 = 50.0 + INDICATOR_WEIGHT; // 50 = max tab height
                assert_eq!(tab_bar_height(&tester), TAB_BAR_HEIGHT);

                let (_, _, tab_bottom) = assert_three_tab_rects(
                    &tester,
                    [&keys[0], &keys[1], &keys[2]],
                    [(130.0, 30.0), (140.0, 40.0), (150.0, 50.0)],
                    TAB_BAR_HEIGHT,
                    INDICATOR_WEIGHT,
                    LABEL_LEFT,
                    LABEL_RIGHT,
                );

                // Tab 0 selected, indicator padding resolves to left: 100.0
                const INDICATOR_LEFT: f64 = 100.0 + INDICATOR_WEIGHT / 2.0;
                let indicator_right = 130.0 + LABEL_HORIZONTAL - INDICATOR_WEIGHT / 2.0;
                let indicator_y = tab_bottom + INDICATOR_WEIGHT / 2.0;
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    None,
                    INDICATOR_WEIGHT,
                    Offset::new(INDICATOR_LEFT, indicator_y),
                    Offset::new(indicator_right, indicator_y),
                ));
            });
        }

        // 'TabBar with directional indicatorPadding (RTL)'.
        #[test]
        fn a_directional_indicator_padding_starts_on_the_right_right_to_left() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let (keys, tabs) = sized_tabs(&[(130.0, 30.0), (140.0, 40.0), (150.0, 50.0)]);
                const INDICATOR_WEIGHT: f64 = 2.0; // the default
                let controller = new_controller(&tester, 3);
                let theme = theme(&tester, Some(false), None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Rtl,
                    top_left(
                        TabBar::new(tabs)
                            .indicator_padding(EdgeInsetsDirectional::only(100.0, 0.0, 0.0, 0.0))
                            .is_scrollable(true)
                            .controller(controller),
                    ),
                ));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                const TAB_BAR_HEIGHT: f64 = 50.0 + INDICATOR_WEIGHT; // 50 = max tab height
                assert_eq!(tab_bar_height(&tester), TAB_BAR_HEIGHT);

                let (tab_left, tab_right, _) = assert_three_tab_rects(
                    &tester,
                    [&keys[2], &keys[1], &keys[0]],
                    [(150.0, 50.0), (140.0, 40.0), (130.0, 30.0)],
                    TAB_BAR_HEIGHT,
                    INDICATOR_WEIGHT,
                    LABEL_LEFT,
                    LABEL_RIGHT,
                );

                // Tab 0 selected, indicator padding resolves to right: 100.0
                let indicator_left = tab_left - LABEL_LEFT + INDICATOR_WEIGHT / 2.0;
                let indicator_right = tab_right + LABEL_LEFT - INDICATOR_WEIGHT / 2.0 - 100.0;
                const INDICATOR_Y: f64 = 50.0 + INDICATOR_WEIGHT / 2.0;
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    None,
                    INDICATOR_WEIGHT,
                    Offset::new(indicator_left, INDICATOR_Y),
                    Offset::new(indicator_right, INDICATOR_Y),
                ));
            });
        }

        /// The custom indicator cases' tab bar of four text tabs with a `BoxDecoration` indicator.
        fn custom_indicator_frame(
            tester: &WidgetTester,
            text_direction: TextDirection,
            controller: Ref<TabController>,
            indicator_color: Color,
            indicator_padding: EdgeInsetsGeometry,
        ) -> WidgetRef {
            let theme = theme(tester, Some(false), None);
            boilerplate(
                theme,
                text_direction,
                top_left(
                    TabBar::new(text_tabs(4))
                        .indicator(BoxDecoration::new().color(indicator_color))
                        .indicator_padding(indicator_padding)
                        .controller(controller),
                ),
            )
        }

        // 'TabBar with custom indicator and indicatorPadding(LTR)'.
        #[test]
        fn a_custom_indicator_is_inset_by_its_padding_left_to_right() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_COLOR: Color = Color::new(0xFF00FF00);
                const PAD_TOP: f64 = 10.0;
                const PAD_BOTTOM: f64 = 12.0;
                const PAD_LEFT: f64 = 8.0;
                const PAD_RIGHT: f64 = 4.0;
                let controller = new_controller(&tester, 4);
                let frame = custom_indicator_frame(
                    &tester,
                    TextDirection::Ltr,
                    controller.clone(),
                    INDICATOR_COLOR,
                    EdgeInsetsGeometry::only(PAD_LEFT, PAD_TOP, PAD_RIGHT, PAD_BOTTOM),
                );
                tester.pump_widget(frame);

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                assert_eq!(tab_bar_height(&tester), 48.0);
                // 48 = _kTabHeight(46) + indicatorWeight(2.0) ~default

                const INDICATOR_BOTTOM: f64 = 48.0 - PAD_BOTTOM;
                const INDICATOR_TOP: f64 = PAD_TOP;
                assert!(paints_rect(
                    &tester,
                    tab_bar_box,
                    Rect::from_ltrb(PAD_LEFT, INDICATOR_TOP, 200.0 - PAD_RIGHT, INDICATOR_BOTTOM),
                    INDICATOR_COLOR,
                ));

                // Select tab 3
                set_index(&tester, controller.handle(), 3);
                tester.pump_and_settle();

                assert!(paints_rect(
                    &tester,
                    tab_bar_box,
                    Rect::from_ltrb(
                        600.0 + PAD_LEFT,
                        INDICATOR_TOP,
                        800.0 - PAD_RIGHT,
                        INDICATOR_BOTTOM
                    ),
                    INDICATOR_COLOR,
                ));
            });
        }

        // 'TabBar with custom indicator and indicatorPadding (RTL)'.
        #[test]
        fn a_custom_indicator_is_inset_by_its_padding_right_to_left() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_COLOR: Color = Color::new(0xFF00FF00);
                const PAD_TOP: f64 = 10.0;
                const PAD_BOTTOM: f64 = 12.0;
                const PAD_LEFT: f64 = 8.0;
                const PAD_RIGHT: f64 = 4.0;
                let controller = new_controller(&tester, 4);
                let frame = custom_indicator_frame(
                    &tester,
                    TextDirection::Rtl,
                    controller.clone(),
                    INDICATOR_COLOR,
                    EdgeInsetsGeometry::only(PAD_LEFT, PAD_TOP, PAD_RIGHT, PAD_BOTTOM),
                );
                tester.pump_widget(frame);

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                assert_eq!(tab_bar_height(&tester), 48.0);
                // 48 = _kTabHeight(46) + indicatorWeight(2.0) ~default
                assert_eq!(tab_bar_size(&tester).width(), 800.0);
                const INDICATOR_BOTTOM: f64 = 48.0 - PAD_BOTTOM;
                const INDICATOR_TOP: f64 = PAD_TOP;
                assert!(paints_rect(
                    &tester,
                    tab_bar_box,
                    Rect::from_ltrb(
                        600.0 + PAD_LEFT,
                        INDICATOR_TOP,
                        800.0 - PAD_RIGHT,
                        INDICATOR_BOTTOM
                    ),
                    INDICATOR_COLOR,
                ));

                // Select tab 3
                set_index(&tester, controller.handle(), 3);
                tester.pump_and_settle();

                assert!(paints_rect(
                    &tester,
                    tab_bar_box,
                    Rect::from_ltrb(PAD_LEFT, INDICATOR_TOP, 200.0 - PAD_RIGHT, INDICATOR_BOTTOM),
                    INDICATOR_COLOR,
                ));
            });
        }

        // 'TabBar accepts indicatorWeight: 0 when the indicator comes from TabBarThemeData'.
        #[test]
        fn a_zero_indicator_weight_is_accepted_with_a_themed_indicator() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_COLOR: Color = Color::new(0xFF00FF00);
                let build_frame = |tester: &WidgetTester, secondary: bool| {
                    let tabs = vec![
                        Tab::new().text("A").into_widget(),
                        Tab::new().text("B").into_widget(),
                    ];
                    let theme = theme(
                        tester,
                        None,
                        Some(
                            TabBarThemeData::new()
                                .indicator(BoxDecoration::new().color(INDICATOR_COLOR))
                                .indicator_size(TabBarIndicatorSize::Tab),
                        ),
                    );
                    let tab_bar = if secondary {
                        TabBar::secondary(tabs)
                    } else {
                        TabBar::new(tabs)
                    }
                    .indicator_weight(0.0);
                    boilerplate(
                        theme,
                        TextDirection::Ltr,
                        top_left(DefaultTabController::new(2, tab_bar)),
                    )
                };

                // Primary TabBar: no assertion is thrown and the themed indicator is painted.
                let frame = build_frame(&tester, false);
                tester.pump_widget(frame);

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                // 46 = _kTabHeight(46) with no weight added by the zero indicatorWeight.
                assert_eq!(tab_bar_height(&tester), 46.0);
                assert!(paints_rect(
                    &tester,
                    tab_bar_box,
                    Rect::from_ltrb(0.0, 0.0, 400.0, 46.0),
                    INDICATOR_COLOR,
                ));

                // Secondary TabBar: also accepts a zero indicatorWeight with a themed indicator.
                let frame = build_frame(&tester, true);
                tester.pump_widget(frame);
            });
        }

        // 'TabBar accepts indicatorWeight: 0 when the indicator is provided on the widget'.
        #[test]
        fn a_zero_indicator_weight_is_accepted_with_an_indicator_on_the_widget() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    DefaultTabController::new(
                        2,
                        TabBar::new([
                            Tab::new().text("A").into_widget(),
                            Tab::new().text("B").into_widget(),
                        ])
                        .indicator(BoxDecoration::new().color(Color::new(0xFF00FF00)))
                        .indicator_weight(0.0),
                    ),
                ));
            });
        }

        // 'TabBar throws indicatorWeight: 0 with the default underline indicator'.
        #[test]
        fn a_zero_indicator_weight_panics_with_the_default_underline() {
            with_main_thread_stack(|| {
                for secondary in [false, true] {
                    let mut tester = tester();
                    let theme = theme(&tester, None, None);
                    let tabs = vec![
                        Tab::new().text("A").into_widget(),
                        Tab::new().text("B").into_widget(),
                    ];
                    let tab_bar = if secondary {
                        TabBar::secondary(tabs)
                    } else {
                        TabBar::new(tabs)
                    }
                    .indicator_weight(0.0);
                    let frame = boilerplate(
                        theme,
                        TextDirection::Ltr,
                        DefaultTabController::new(2, tab_bar),
                    );
                    let result = catch_unwind(AssertUnwindSafe(|| tester.pump_widget(frame)));
                    let message = panic_message(result.expect_err("the assert fires"));
                    assert!(
                        message.contains("Invalid indicatorWeight for TabBar."),
                        "{message}"
                    );
                }
            });
        }

        /// The custom indicator cases' scrollable tab bar of three sized tabs with a directional
        /// indicator padding.
        fn custom_directional_frame(
            tester: &WidgetTester,
            text_direction: TextDirection,
            controller: Ref<TabController>,
            tabs: Vec<WidgetRef>,
            indicator_color: Color,
        ) -> WidgetRef {
            let theme = theme(tester, Some(false), None);
            boilerplate(
                theme,
                text_direction,
                top_left(
                    TabBar::new(tabs)
                        .indicator(BoxDecoration::new().color(indicator_color))
                        .indicator_padding(EdgeInsetsDirectional::only(8.0, 10.0, 4.0, 12.0))
                        .is_scrollable(true)
                        .controller(controller),
                ),
            )
        }

        // 'TabBar with custom indicator - directional indicatorPadding (LTR)'.
        #[test]
        fn a_custom_indicator_takes_a_directional_padding_left_to_right() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let (keys, tabs) = sized_tabs(&[(130.0, 30.0), (140.0, 40.0), (150.0, 50.0)]);
                const INDICATOR_COLOR: Color = Color::new(0xFF00FF00);
                const PAD_TOP: f64 = 10.0;
                const PAD_BOTTOM: f64 = 12.0;
                const PAD_START: f64 = 8.0;
                const PAD_END: f64 = 4.0;
                const INDICATOR_WEIGHT: f64 = 2.0; // the default
                let controller = new_controller(&tester, 3);
                let frame = custom_directional_frame(
                    &tester,
                    TextDirection::Ltr,
                    controller.clone(),
                    tabs,
                    INDICATOR_COLOR,
                );
                tester.pump_widget(frame);

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                const TAB_BAR_HEIGHT: f64 = 50.0 + INDICATOR_WEIGHT; // 50 = max tab height
                assert_eq!(tab_bar_height(&tester), TAB_BAR_HEIGHT);

                assert_three_tab_rects(
                    &tester,
                    [&keys[0], &keys[1], &keys[2]],
                    [(130.0, 30.0), (140.0, 40.0), (150.0, 50.0)],
                    TAB_BAR_HEIGHT,
                    INDICATOR_WEIGHT,
                    LABEL_LEFT,
                    LABEL_RIGHT,
                );

                // Tab 0 selected, indicator padding resolves to left: 8.0, right: 4.0
                let indicator_right = 130.0 + LABEL_HORIZONTAL - PAD_END;
                assert!(paints_rect(
                    &tester,
                    tab_bar_box,
                    Rect::from_ltrb(
                        PAD_START,
                        PAD_TOP,
                        indicator_right,
                        TAB_BAR_HEIGHT - PAD_BOTTOM
                    ),
                    INDICATOR_COLOR,
                ));
            });
        }

        // 'TabBar with custom indicator - directional indicatorPadding (RTL)'.
        #[test]
        fn a_custom_indicator_takes_a_directional_padding_right_to_left() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let (keys, tabs) = sized_tabs(&[(130.0, 30.0), (140.0, 40.0), (150.0, 50.0)]);
                const INDICATOR_COLOR: Color = Color::new(0xFF00FF00);
                const PAD_TOP: f64 = 10.0;
                const PAD_BOTTOM: f64 = 12.0;
                const PAD_START: f64 = 8.0;
                const PAD_END: f64 = 4.0;
                const INDICATOR_WEIGHT: f64 = 2.0; // the default
                let controller = new_controller(&tester, 3);
                let frame = custom_directional_frame(
                    &tester,
                    TextDirection::Rtl,
                    controller.clone(),
                    tabs,
                    INDICATOR_COLOR,
                );
                tester.pump_widget(frame);

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                const TAB_BAR_HEIGHT: f64 = 50.0 + INDICATOR_WEIGHT; // 50 = max tab height
                assert_eq!(tab_bar_height(&tester), TAB_BAR_HEIGHT);

                let (tab_left, tab_right, _) = assert_three_tab_rects(
                    &tester,
                    [&keys[2], &keys[1], &keys[0]],
                    [(150.0, 50.0), (140.0, 40.0), (130.0, 30.0)],
                    TAB_BAR_HEIGHT,
                    INDICATOR_WEIGHT,
                    LABEL_LEFT,
                    LABEL_RIGHT,
                );

                // Tab 0 selected, indicator padding resolves to left: 4.0, right: 8.0
                let indicator_left = tab_left - LABEL_LEFT + PAD_END;
                let indicator_right = tab_right + LABEL_LEFT - PAD_START;
                assert!(paints_rect(
                    &tester,
                    tab_bar_box,
                    Rect::from_ltrb(
                        indicator_left,
                        PAD_TOP,
                        indicator_right,
                        TAB_BAR_HEIGHT - PAD_BOTTOM
                    ),
                    INDICATOR_COLOR,
                ));
            });
        }

        // 'TabBar with padding isScrollable: false'.
        #[test]
        fn a_fixed_tab_bar_is_inset_by_its_padding() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_WEIGHT: f64 = 2.0; // default indicator weight
                let padding = EdgeInsets::only(3.0, 7.0, 5.0, 3.0);
                let (keys, tabs) = sized_tabs(&[(f64::INFINITY, 30.0), (f64::INFINITY, 40.0)]);
                let controller = new_controller(&tester, 2);
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    top_left(
                        TabBar::new(tabs)
                            .padding(padding)
                            .label_padding(EdgeInsetsGeometry::ZERO)
                            .controller(controller),
                    ),
                ));

                let tab_bar_size = tab_bar_size(&tester);
                let tab_bar_height = 40.0 + INDICATOR_WEIGHT + padding.top + padding.bottom; // 40 = max tab height
                assert_eq!(tab_bar_size.height(), tab_bar_height);

                let tab_size = (tab_bar_size.width() - padding.horizontal()) / 2.0;

                // Tab0 height = 30
                let tab_left = padding.left;
                let mut tab_right = tab_left + tab_size;
                let tab_top = (tab_bar_height - INDICATOR_WEIGHT + (padding.top - padding.bottom)
                    - 30.0)
                    / 2.0;
                let tab_rect = Rect::from_ltrb(tab_left, tab_top, tab_right, tab_top + 30.0);
                assert_eq!(rect_of_key(&tester, &keys[0]), tab_rect);

                // Tab1 height = 40
                let tab_left = tab_right;
                tab_right = tab_left + tab_size;
                let tab_top = (tab_bar_height - INDICATOR_WEIGHT + (padding.top - padding.bottom)
                    - 40.0)
                    / 2.0;
                let tab_rect = Rect::from_ltrb(tab_left, tab_top, tab_right, tab_top + 40.0);
                assert_eq!(rect_of_key(&tester, &keys[1]), tab_rect);

                tab_right += padding.right;
                assert_eq!(tab_bar_size.width(), tab_right);
            });
        }

        // 'Material3 - TabBar with padding isScrollable: true'.
        #[test]
        fn material3_a_scrollable_tab_bar_is_inset_by_its_padding_past_the_start_offset() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_WEIGHT: f64 = 2.0; // default indicator weight
                let padding = EdgeInsets::only(3.0, 7.0, 5.0, 3.0);
                const TAB_START_OFFSET: f64 = 52.0;
                let sizes = [(130.0, 30.0), (140.0, 40.0), (150.0, 50.0)];
                let (keys, tabs) = sized_tabs(&sizes);
                let controller = new_controller(&tester, 3);
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    top_left(
                        TabBar::new(tabs)
                            .padding(padding)
                            .label_padding(EdgeInsetsGeometry::ZERO)
                            .is_scrollable(true)
                            .controller(controller),
                    ),
                ));

                let tab_bar_size = tab_bar_size(&tester);
                let tab_bar_height = 50.0 + INDICATOR_WEIGHT + padding.top + padding.bottom; // 50 = max tab height
                assert_eq!(tab_bar_size.height(), tab_bar_height);

                let mut tab_left = padding.left + TAB_START_OFFSET;
                let mut tab_right = 0.0;
                for (index, (width, height)) in sizes.into_iter().enumerate() {
                    if index > 0 {
                        tab_left = tab_right;
                    }
                    tab_right = tab_left + width;
                    let tab_top = (tab_bar_height - INDICATOR_WEIGHT
                        + (padding.top - padding.bottom)
                        - height)
                        / 2.0;
                    let tab_rect = Rect::from_ltrb(tab_left, tab_top, tab_right, tab_top + height);
                    assert_eq!(rect_of_key(&tester, &keys[index]), tab_rect);
                }

                tab_right += padding.right;
                // Right tab + remaining space of the stretched tab bar.
                assert_eq!(tab_bar_size.width(), tab_right + 320.0);
            });
        }

        // 'TabBar with labelPadding'.
        #[test]
        fn a_label_padding_spaces_the_tabs_and_the_indicator_follows_it() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_WEIGHT: f64 = 2.0; // default indicator weight
                let label_padding = EdgeInsets::only(3.0, 0.0, 7.0, 0.0);
                let indicator_padding = label_padding;
                let (keys, tabs) = sized_tabs(&[(130.0, 30.0), (140.0, 40.0), (150.0, 50.0)]);
                let controller = new_controller(&tester, 3);
                let theme = theme(&tester, Some(false), None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    top_left(
                        TabBar::new(tabs)
                            .label_padding(label_padding)
                            .indicator_padding(label_padding)
                            .is_scrollable(true)
                            .controller(controller),
                    ),
                ));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                const TAB_BAR_HEIGHT: f64 = 50.0 + INDICATOR_WEIGHT; // 50 = max tab height
                assert_eq!(tab_bar_height(&tester), TAB_BAR_HEIGHT);

                let (_, _, tab_bottom) = assert_three_tab_rects(
                    &tester,
                    [&keys[0], &keys[1], &keys[2]],
                    [(130.0, 30.0), (140.0, 40.0), (150.0, 50.0)],
                    TAB_BAR_HEIGHT,
                    INDICATOR_WEIGHT,
                    label_padding.left,
                    label_padding.right,
                );

                // Tab 0 selected, indicatorPadding == labelPadding
                let indicator_left = indicator_padding.left + INDICATOR_WEIGHT / 2.0;
                let indicator_right = 130.0 + label_padding.horizontal()
                    - indicator_padding.right
                    - INDICATOR_WEIGHT / 2.0;
                let indicator_y = tab_bottom + INDICATOR_WEIGHT / 2.0;
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    None,
                    INDICATOR_WEIGHT,
                    Offset::new(indicator_left, indicator_y),
                    Offset::new(indicator_right, indicator_y),
                ));
            });
        }

        // 'TabBar with labelPadding(TabBarIndicatorSize.label)'.
        #[test]
        fn a_label_sized_indicator_sits_inside_the_label_padding() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_WEIGHT: f64 = 2.0; // default indicator weight
                let label_padding = EdgeInsets::only(7.0, 0.0, 4.0, 0.0);
                let indicator_padding = EdgeInsets::only(3.0, 0.0, 7.0, 0.0);
                let (keys, tabs) = sized_tabs(&[(130.0, 30.0), (140.0, 40.0), (150.0, 50.0)]);
                let controller = new_controller(&tester, 3);
                let theme = theme(&tester, Some(false), None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    top_left(
                        TabBar::new(tabs)
                            .label_padding(label_padding)
                            .indicator_padding(indicator_padding)
                            .is_scrollable(true)
                            .controller(controller)
                            .indicator_size(TabBarIndicatorSize::Label),
                    ),
                ));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                const TAB_BAR_HEIGHT: f64 = 50.0 + INDICATOR_WEIGHT; // 50 = max tab height
                assert_eq!(tab_bar_height(&tester), TAB_BAR_HEIGHT);

                let (_, _, tab_bottom) = assert_three_tab_rects(
                    &tester,
                    [&keys[0], &keys[1], &keys[2]],
                    [(130.0, 30.0), (140.0, 40.0), (150.0, 50.0)],
                    TAB_BAR_HEIGHT,
                    INDICATOR_WEIGHT,
                    label_padding.left,
                    label_padding.right,
                );

                // Tab 0 selected
                let indicator_left =
                    indicator_padding.left + label_padding.left + INDICATOR_WEIGHT / 2.0;
                let indicator_right =
                    label_padding.left + 130.0 - indicator_padding.right - INDICATOR_WEIGHT / 2.0;
                let indicator_y = tab_bottom + INDICATOR_WEIGHT / 2.0;
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    None,
                    INDICATOR_WEIGHT,
                    Offset::new(indicator_left, indicator_y),
                    Offset::new(indicator_right, indicator_y),
                ));
            });
        }

        // 'Overflowing RTL tab bar'.
        #[test]
        fn an_overflowing_right_to_left_tab_bar_paints_its_indicator_from_the_far_right() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                // For convenience padded width of each tab will equal 100:
                // 68 + kTabLabelPadding.horizontal(32)
                let (_, tabs) = sized_tabs(&[(68.0, 40.0); 100]);
                let controller = new_controller(&tester, 100);
                const INDICATOR_WEIGHT: f64 = 2.0; // the default
                let theme = theme(&tester, Some(false), None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Rtl,
                    top_left(
                        TabBar::new(tabs)
                            .is_scrollable(true)
                            .controller(controller.clone()),
                    ),
                ));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                const TAB_BAR_HEIGHT: f64 = 40.0 + INDICATOR_WEIGHT; // 40 = tab height
                assert_eq!(tab_bar_height(&tester), TAB_BAR_HEIGHT);

                // Tab 0 out of 100 selected
                const INDICATOR_Y: f64 = 40.0 + INDICATOR_WEIGHT / 2.0;
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    None,
                    INDICATOR_WEIGHT,
                    Offset::new(99.0 * 100.0 + INDICATOR_WEIGHT / 2.0, INDICATOR_Y),
                    Offset::new(100.0 * 100.0 - INDICATOR_WEIGHT / 2.0, INDICATOR_Y),
                ));

                {
                    let mut app = tester.cell.borrow_mut();
                    controller.animate_to(
                        &mut app,
                        99,
                        Some(Duration::from_secs(1)),
                        Some(Curves::linear()),
                    );
                }
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(500));

                // In RTL, the elastic tab animation expands the width of the tab with a negative offset
                // when jumping from the first tab to the last tab in a scrollable tab bar.
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    None,
                    INDICATOR_WEIGHT,
                    Offset::new(4951.0, INDICATOR_Y),
                    Offset::new(5049.0, INDICATOR_Y),
                ));

                tester.pump(Duration::from_millis(501));

                // Tab 99 out of 100 selected, appears on the far left because RTL.
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    None,
                    INDICATOR_WEIGHT,
                    Offset::new(INDICATOR_WEIGHT / 2.0, INDICATOR_Y),
                    Offset::new(100.0 - INDICATOR_WEIGHT / 2.0, INDICATOR_Y),
                ));
            });
        }

        // 'Tab indicator animation test'.
        #[test]
        fn the_indicator_animates_between_tabs_and_retargets_mid_animation() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_WEIGHT: f64 = 8.0;
                let controller = new_controller(&tester, 4);
                let theme = theme(&tester, Some(false), None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    top_left(
                        TabBar::new(text_tabs(4))
                            .indicator_weight(INDICATOR_WEIGHT)
                            .controller(controller.clone()),
                    ),
                ));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                let indicator_at = |tester: &WidgetTester, left: f64, right: f64| {
                    const INDICATOR_Y: f64 = 54.0 - INDICATOR_WEIGHT / 2.0;
                    paints_line(
                        tester,
                        tab_bar_box,
                        None,
                        INDICATOR_WEIGHT,
                        Offset::new(left + INDICATOR_WEIGHT / 2.0, INDICATOR_Y),
                        Offset::new(right - INDICATOR_WEIGHT / 2.0, INDICATOR_Y),
                    )
                };
                let animate_to = |tester: &WidgetTester, index: usize| {
                    let mut app = tester.cell.borrow_mut();
                    controller.animate_to(
                        &mut app,
                        index,
                        Some(Duration::from_millis(1000)),
                        Some(Curves::linear()),
                    );
                };

                // Initial indicator position.
                assert!(indicator_at(&tester, 0.0, 200.0));

                // Select tab 1.
                animate_to(&tester, 1);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(500));
                assert!(indicator_at(&tester, 100.0, 300.0));

                // Select tab 2 when animation is running.
                animate_to(&tester, 2);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(500));
                assert!(indicator_at(&tester, 250.0, 450.0));

                // Final indicator position.
                tester.pump_and_settle();
                assert!(indicator_at(&tester, 400.0, 600.0));
            });
        }

        /// Dart's `Column(children: [tabBar, Flexible(child: tabBarView)])`.
        fn bar_over_view(tab_bar: TabBar, tab_bar_view: TabBarView) -> WidgetRef {
            Column::new()
                .children([
                    tab_bar.into_widget(),
                    waft_widgets::Flexible::new(tab_bar_view).into_widget(),
                ])
                .into_widget()
        }

        // 'TabBar etc with zero tabs'.
        #[test]
        fn a_tab_bar_and_view_with_no_tabs_lay_out_and_ignore_flings() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = new_controller(&tester, 0);
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    bar_over_view(
                        TabBar::new([]).controller(controller.clone()),
                        TabBarView::new([]).controller(controller.clone()),
                    ),
                ));

                assert_eq!(controller.index(&tester.cell.borrow()), 0);
                assert_eq!(
                    size_of(&tester, only::<TabBar>(&tester)),
                    Size::new(800.0, 48.0)
                );
                assert_eq!(
                    size_of(&tester, only::<TabBarView>(&tester)),
                    Size::new(800.0, 600.0 - 48.0)
                );

                // A fling in the TabBar or TabBarView, shouldn't do anything.
                let bar = center_of(&tester, only::<TabBar>(&tester));
                tester.fling_from(bar, Offset::new(-100.0, 0.0), 5000.0, 1);
                tester.pump_and_settle();

                let view = center_of(&tester, only::<TabBarView>(&tester));
                tester.fling_from(view, Offset::new(100.0, 0.0), 5000.0, 2);
                tester.pump_and_settle();

                assert_eq!(controller.index(&tester.cell.borrow()), 0);
            });
        }

        // 'TabBar etc with one tab'.
        #[test]
        fn a_tab_bar_and_view_with_one_tab_center_it_and_ignore_flings() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = new_controller(&tester, 1);
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    bar_over_view(
                        TabBar::new([Tab::new().text("TAB").into_widget()])
                            .controller(controller.clone()),
                        TabBarView::new([Text::new("PAGE").into_widget()])
                            .controller(controller.clone()),
                    ),
                ));

                assert_eq!(controller.index(&tester.cell.borrow()), 0);
                assert_eq!(tester.find_text("TAB").len(), 1);
                assert_eq!(tester.find_text("PAGE").len(), 1);
                assert_eq!(
                    size_of(&tester, only::<TabBar>(&tester)),
                    Size::new(800.0, 48.0)
                );
                assert_eq!(
                    size_of(&tester, only::<TabBarView>(&tester)),
                    Size::new(800.0, 600.0 - 48.0)
                );

                // The one tab should be center vis the app's width (800).
                let tab_rect = |tester: &WidgetTester| {
                    let tab = tester.ancestor_of_type::<Tab>(text(tester, "TAB"));
                    rect_of(tester, tab)
                };
                let tab_left = tab_rect(&tester).left;
                let tab_right = tab_rect(&tester).right;
                // To the precision of the f32 transform `local_to_global` maps through.
                let tab_center = tab_left + (tab_right - tab_left) / 2.0;
                assert!(close(tab_center, 400.0, 1e-4), "{tab_center}");

                // A fling in the TabBar or TabBarView, shouldn't move the tab.
                let bar = center_of(&tester, only::<TabBar>(&tester));
                tester.fling_from(bar, Offset::new(-100.0, 0.0), 5000.0, 1);
                tester.pump(Duration::from_millis(50));
                assert_eq!(tab_rect(&tester).left, tab_left);
                assert_eq!(tab_rect(&tester).right, tab_right);
                tester.pump_and_settle();

                let view = center_of(&tester, only::<TabBarView>(&tester));
                tester.fling_from(view, Offset::new(100.0, 0.0), 5000.0, 2);
                tester.pump(Duration::from_millis(50));
                assert_eq!(tab_rect(&tester).left, tab_left);
                assert_eq!(tab_rect(&tester).right, tab_right);
                tester.pump_and_settle();

                assert_eq!(controller.index(&tester.cell.borrow()), 0);
                assert_eq!(tester.find_text("TAB").len(), 1);
                assert_eq!(tester.find_text("PAGE").len(), 1);
            });
        }

        // 'can tap on indicator at very bottom of TabBar to switch tabs'.
        #[test]
        fn a_tap_on_the_indicator_at_the_bottom_of_a_tab_switches_to_it() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = new_controller(&tester, 2);
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    bar_over_view(
                        TabBar::new([
                            Tab::new().text("TAB1").into_widget(),
                            Tab::new().text("TAB2").into_widget(),
                        ])
                        .controller(controller.clone())
                        .indicator_weight(30.0),
                        TabBarView::new([
                            Text::new("PAGE1").into_widget(),
                            Text::new("PAGE2").into_widget(),
                        ])
                        .controller(controller.clone()),
                    ),
                ));

                assert_eq!(controller.index(&tester.cell.borrow()), 0);

                let bottom_right = rect_of(&tester, only::<TabBar>(&tester)).bottom_right()
                    - Offset::new(1.0, 1.0);
                let mut gesture = WidgetTester::touch(1);
                tester.down(&mut gesture, bottom_right);
                tester.up(&gesture);
                tester.pump_and_settle();

                assert_eq!(controller.index(&tester.cell.borrow()), 1);
            });
        }

        // 'can be notified of TabBar onTap behavior'.
        #[test]
        fn on_tap_reports_the_tapped_tab_even_when_the_index_does_not_change() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_index = Rc::new(Cell::new(-1_i64));
                let tabs = ["A", "B", "C"];
                let controller = {
                    let mut app = tester.cell.borrow_mut();
                    TabController::new(&mut app, 2, None, tabs.len(), TestVSync)
                };
                let build_frame = |tester: &WidgetTester| {
                    let theme = theme(tester, None, None);
                    let tab_index = Rc::clone(&tab_index);
                    boilerplate(
                        theme,
                        TextDirection::Ltr,
                        TabBar::new(tabs.iter().map(|tab| Tab::new().text(*tab).into_widget()))
                            .controller(controller.clone())
                            .on_tap(move |_app, index| tab_index.set(index as i64)),
                    )
                };
                let index = |tester: &WidgetTester| controller.index(&tester.cell.borrow());
                let previous_index =
                    |tester: &WidgetTester| controller.previous_index(&tester.cell.borrow());

                let frame = build_frame(&tester);
                tester.pump_widget(frame);
                assert_eq!(tester.find_text("A").len(), 1);
                assert_eq!(tester.find_text("B").len(), 1);
                assert_eq!(tester.find_text("C").len(), 1);
                assert_eq!(index(&tester), 2);
                assert_eq!(tab_index.get(), -1); // no tap so far so tabIndex should reflect that

                // Verify whether the [onTap] notification works when the [TabBar] animates.
                let frame = build_frame(&tester);
                tester.pump_widget(frame);
                let b = text(&tester, "B");
                tap(&mut tester, b);
                tester.pump(Duration::ZERO);
                assert!(controller.index_is_changing(&tester.cell.borrow()));
                tester.pump_and_settle();
                assert_eq!(index(&tester), 1);
                assert_eq!(previous_index(&tester), 2);
                assert!(!controller.index_is_changing(&tester.cell.borrow()));
                assert_eq!(tab_index.get(), index(&tester) as i64);

                tab_index.set(-1);

                let frame = build_frame(&tester);
                tester.pump_widget(frame);
                let c = text(&tester, "C");
                tap(&mut tester, c);
                tester.pump(Duration::ZERO);
                tester.pump_and_settle();
                assert_eq!(index(&tester), 2);
                assert_eq!(previous_index(&tester), 1);
                assert_eq!(tab_index.get(), index(&tester) as i64);

                tab_index.set(-1);

                let frame = build_frame(&tester);
                tester.pump_widget(frame);
                let a = text(&tester, "A");
                tap(&mut tester, a);
                tester.pump(Duration::ZERO);
                tester.pump_and_settle();
                assert_eq!(index(&tester), 0);
                assert_eq!(previous_index(&tester), 2);
                assert_eq!(tab_index.get(), index(&tester) as i64);

                tab_index.set(-1);

                // Verify whether [onTap] is called even when the [TabController] does
                // not change.
                let current_controller_index = index(&tester);
                let frame = build_frame(&tester);
                tester.pump_widget(frame);
                let a = text(&tester, "A");
                tap(&mut tester, a);
                tester.pump(Duration::ZERO);
                tester.pump_and_settle();
                assert_eq!(index(&tester), current_controller_index); // controller has not changed
                assert_eq!(tab_index.get(), 0);
            });
        }

        /// The message of the panic `tab.into_widget()` raises.
        fn tab_constructor_panic(tab: Tab) -> String {
            let result = catch_unwind(AssertUnwindSafe(move || {
                let _ = tab.into_widget();
            }));
            panic_message(result.expect_err("the constructor assert fires"))
        }

        // 'illegal constructor combinations'.
        #[test]
        fn a_tab_without_content_or_with_both_text_and_child_panics() {
            tab_constructor_panic(Tab::new());
            tab_constructor_panic(
                Tab::new()
                    .icon(waft_widgets::Container::new())
                    .text("foo")
                    .child(waft_widgets::Container::new()),
            );
            tab_constructor_panic(Tab::new().text("foo").child(waft_widgets::Container::new()));
        }

        // 'Tab throws clear error when both text and child are set'.
        #[test]
        fn a_tab_with_both_text_and_child_says_to_give_one() {
            let message = tab_constructor_panic(Tab::new().text("Hi").child(Text::new("World")));
            assert!(
                message.contains("Provide either text or child, not both, when creating a Tab."),
                "{message}"
            );
        }

        // 'Tabs changes mouse cursor when a tab is hovered'.
        #[test]
        fn a_hovered_tab_shows_the_tab_bars_mouse_cursor_or_a_click_cursor() {
            use waft_services::{MouseCursorRef, SystemMouseCursors};
            with_main_thread_stack(|| {
                let mut tester = tester();
                let build = |tester: &WidgetTester, mouse_cursor: Option<MouseCursorRef>| {
                    let theme = theme(tester, None, None);
                    let mut tab_bar =
                        TabBar::new(["A", "B"].map(|tab| Tab::new().text(tab).into_widget()));
                    if let Some(mouse_cursor) = mouse_cursor {
                        tab_bar = tab_bar.mouse_cursor(mouse_cursor);
                    }
                    material_app(
                        theme,
                        DefaultTabController::new(
                            2,
                            Scaffold::new().body(
                                waft_widgets::MouseRegion::new()
                                    .cursor(SystemMouseCursors::FORBIDDEN.into())
                                    .child(tab_bar),
                            ),
                        )
                        .into_widget(),
                    )
                };
                let frame = build(&tester, Some(SystemMouseCursors::TEXT.into()));
                tester.pump_widget(frame);

                let mut mouse = WidgetTester::mouse(1, 0);
                let first_tab = tester.find_by_type::<Tab>()[0];
                let center = center_of(&tester, first_tab);
                tester.add_pointer(&mut mouse, center);
                tester.pump(Duration::ZERO);

                let text_cursor: MouseCursorRef = SystemMouseCursors::TEXT.into();
                assert!(*tester.active_cursor(1) == *text_cursor);

                // Test default cursor
                let frame = build(&tester, None);
                tester.pump_widget(frame);
                let click: MouseCursorRef = SystemMouseCursors::CLICK.into();
                assert!(*tester.active_cursor(1) == *click);
            });
        }

        /// The two-tab tab bar of the controller-change cases.
        fn left_right_tab_bar(tester: &WidgetTester, controller: Ref<TabController>) -> WidgetRef {
            let theme = theme(tester, Some(false), None);
            boilerplate(
                theme,
                TextDirection::Ltr,
                top_left(
                    TabBar::new([
                        Tab::new().text("LEFT").into_widget(),
                        Tab::new().text("RIGHT").into_widget(),
                    ])
                    .controller(controller),
                ),
            )
        }

        // 'TabController changes'.
        #[test]
        fn a_tab_bar_given_a_new_controller_follows_the_new_one() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller1 = new_controller(&tester, 2);
                let controller2 = new_controller(&tester, 2);

                let frame = left_right_tab_bar(&tester, controller1.clone());
                tester.pump_widget(frame);
                let frame = left_right_tab_bar(&tester, controller2.clone());
                tester.pump_widget(frame);
                assert_eq!(controller1.index(&tester.cell.borrow()), 0);
                assert_eq!(controller2.index(&tester.cell.borrow()), 0);

                const INDICATOR_WEIGHT: f64 = 2.0;
                let tab_bar_box = first_render_box::<TabBar>(&tester);
                assert_eq!(tab_bar_height(&tester), 48.0); // 48 = _kTabHeight(46) + indicatorWeight(2.0)

                const INDICATOR_Y: f64 = 48.0 - INDICATOR_WEIGHT / 2.0;
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    None,
                    INDICATOR_WEIGHT,
                    Offset::new(INDICATOR_WEIGHT / 2.0, INDICATOR_Y),
                    Offset::new(400.0 - INDICATOR_WEIGHT / 2.0, INDICATOR_Y), // 400 = screen_width / 2
                ));

                let right = text(&tester, "RIGHT");
                tap(&mut tester, right);
                tester.pump_and_settle();
                assert_eq!(controller1.index(&tester.cell.borrow()), 0);
                assert_eq!(controller2.index(&tester.cell.borrow()), 1);

                // Verify that the TabBar's _IndicatorPainter is now listening to
                // tabController2.
                assert!(paints_line(
                    &tester,
                    tab_bar_box,
                    None,
                    INDICATOR_WEIGHT,
                    Offset::new(400.0 + INDICATOR_WEIGHT / 2.0, INDICATOR_Y),
                    Offset::new(800.0 - INDICATOR_WEIGHT / 2.0, INDICATOR_Y),
                ));
            });
        }

        // 'TabController changes while flinging'.
        #[test]
        fn a_controller_replaced_while_flinging_is_followed_by_the_next_fling() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let build_frame = |tester: &WidgetTester, controller: Ref<TabController>| {
                    let theme = {
                        let app = tester.cell.borrow();
                        ThemeData::builder()
                            .platform(TargetPlatform::IOS)
                            .build(&app)
                    };
                    let length = controller.length(&tester.cell.borrow());
                    let mut tabs = vec![
                        Tab::new().text("A").into_widget(),
                        Tab::new().text("B").into_widget(),
                    ];
                    let mut children = vec![
                        Center::new().child(Text::new("CHILD A")).into_widget(),
                        Center::new().child(Text::new("CHILD B")).into_widget(),
                    ];
                    if length == 3 {
                        tabs.push(Tab::new().text("C").into_widget());
                        children.push(Center::new().child(Text::new("CHILD C")).into_widget());
                    }
                    material_app(
                        theme,
                        Scaffold::new()
                            .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                                AppBar::new().title(Text::new("tabs")).bottom(
                                    waft_widgets::PreferredSizeWidgetRef::new(
                                        TabBar::new(tabs).controller(controller.clone()),
                                    ),
                                ),
                            ))
                            .body(TabBarView::new(children).controller(controller.clone()))
                            .into_widget(),
                    )
                };

                let controller1 = new_controller(&tester, 2);
                let controller2 = new_controller(&tester, 3);
                assert_eq!(controller1.index(&tester.cell.borrow()), 0);
                assert_eq!(controller2.index(&tester.cell.borrow()), 0);

                let frame = build_frame(&tester, controller1.clone());
                tester.pump_widget(frame);
                let fling_start = center_of(&tester, text(&tester, "CHILD A"));
                tester.fling_from(fling_start, Offset::new(-200.0, 0.0), 10000.0, 1);
                tester.pump(Duration::from_millis(10)); // start the fling animation

                tester.pump(Duration::from_millis(10));

                let frame = build_frame(&tester, controller2.clone()); // replace controller
                tester.pump_widget(frame);
                tester.fling_from(fling_start, Offset::new(-200.0, 0.0), 10000.0, 2);
                tester.pump_and_settle(); // finish the fling animation

                assert_eq!(controller1.index(&tester.cell.borrow()), 0);
                assert_eq!(controller2.index(&tester.cell.borrow()), 1);
            });
        }

        // 'TabController changes with different initialIndex'.
        #[test]
        fn a_new_controller_with_the_last_tab_selected_scrolls_it_into_view() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let last_tab_key = GlobalKey::new();
                let build_frame = |tester: &WidgetTester, length: usize| {
                    let controller = {
                        let mut app = tester.cell.borrow_mut();
                        TabController::new(&mut app, length - 1, None, length, TestVSync)
                    };
                    let theme = theme(tester, None, None);
                    let tabs = (0..length).map(|index| {
                        let mut tab = Tab::new().text(format!("Tab {index}"));
                        if index == length - 1 {
                            tab = tab.key(last_tab_key.clone().into());
                        }
                        SizedBox::new().width(100.0).child(tab).into_widget()
                    });
                    let frame = boilerplate(
                        theme,
                        TextDirection::Ltr,
                        TabBar::new(tabs)
                            .label_padding(EdgeInsetsGeometry::ZERO)
                            .controller(controller.clone())
                            .is_scrollable(true),
                    );
                    (controller, frame)
                };

                let (controller, frame) = build_frame(&tester, 10);
                tester.pump_widget(frame);
                assert_eq!(controller.index(&tester.cell.borrow()), 9);
                assert_eq!(tester.center_of(&last_tab_key).dx(), 750.0);

                // Rebuild with a new controller with more tabs and last tab selected.
                // Last tab should be visible and on the right of the window.
                let (controller, frame) = build_frame(&tester, 15);
                tester.pump_widget(frame);
                assert_eq!(controller.index(&tester.cell.borrow()), 14);
                assert_eq!(tester.center_of(&last_tab_key).dx(), 750.0);
            });
        }

        /// `tabs_utils.dart`'s `TabBarTestScrollPhysics`.
        #[derive(Debug, Default)]
        struct TabBarTestScrollPhysics {
            parent: Option<ScrollPhysicsRef>,
        }

        impl ScrollPhysics for TabBarTestScrollPhysics {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn parent(&self) -> Option<&ScrollPhysicsRef> {
                self.parent.as_ref()
            }

            fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
                Rc::new(TabBarTestScrollPhysics {
                    parent: waft_widgets::ScrollPhysicsBase::build_parent(self, ancestor),
                })
            }

            fn apply_physics_to_user_offset(
                &self,
                _position: &dyn waft_widgets::ScrollMetrics,
                offset: f64,
            ) -> f64 {
                if offset == 10.0 { 20.0 } else { offset }
            }

            fn spring(&self) -> waft_physics::SpringDescription {
                waft_physics::SpringDescription::with_damping_ratio_value(0.5, 500.0, 1.1)
            }
        }

        // 'DefaultTabController changes does not recreate PageController'.
        #[test]
        fn a_default_tab_controller_with_a_new_length_keeps_the_page_controller() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let build_frame = |tester: &WidgetTester, length: usize| {
                    let theme = theme(tester, None, None);
                    boilerplate(
                        theme,
                        TextDirection::Ltr,
                        DefaultTabController::new(
                            length,
                            TabBarView::new((0..length).map(|index| {
                                Center::new()
                                    .child(Text::new(format!("Page {index}")))
                                    .into_widget()
                            }))
                            .physics(Rc::new(TabBarTestScrollPhysics::default())),
                        )
                        .initial_index(length - 1),
                    )
                };
                let page_controller = |tester: &WidgetTester| {
                    tester.widget(only::<PageView>(tester), |page_view: &PageView| {
                        page_view
                            .controller
                            .clone()
                            .expect("a page view's controller")
                    })
                };

                let frame = build_frame(&tester, 15);
                tester.pump_widget(frame);
                let page_controller1 = page_controller(&tester);
                let tab_controller = default_controller_of(&tester, text(&tester, "Page 14"));
                assert_eq!(tab_controller.index(&tester.cell.borrow()), 14);
                assert_eq!(page_controller1.page(&tester.cell.borrow()), Some(14.0));

                // Rebuild with a new default tab controller with more tabs.
                let frame = build_frame(&tester, 10);
                tester.pump_widget(frame);
                let page_controller2 = page_controller(&tester);
                let tab_controller = default_controller_of(&tester, text(&tester, "Page 9"));
                assert_eq!(tab_controller.index(&tester.cell.borrow()), 9);
                assert_eq!(page_controller2.page(&tester.cell.borrow()), Some(9.0));

                assert_eq!(page_controller1, page_controller2);
            });
        }

        // 'Do not throw when switching between a scrollable TabBar and a non-scrollable TabBar'.
        #[test]
        fn switching_between_a_scrollable_and_a_fixed_tab_bar_does_not_panic() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller1 = new_controller(&tester, 2);
                let controller2 = new_controller(&tester, 2);
                let build_frame = |tester: &WidgetTester, controller, is_scrollable| {
                    let theme = theme(tester, None, None);
                    boilerplate(
                        theme,
                        TextDirection::Ltr,
                        top_left(
                            TabBar::new([
                                Tab::new().text("LEFT").into_widget(),
                                Tab::new().text("RIGHT").into_widget(),
                            ])
                            .controller(controller)
                            .is_scrollable(is_scrollable),
                        ),
                    )
                };

                // Show both controllers once.
                let frame = build_frame(&tester, controller1.clone(), false);
                tester.pump_widget(frame);
                let frame = build_frame(&tester, controller2.clone(), true);
                tester.pump_widget(frame);

                // Switch back to the first controller.
                let frame = build_frame(&tester, controller1, false);
                tester.pump_widget(frame);

                // Switch back to the second controller.
                let frame = build_frame(&tester, controller2, true);
                tester.pump_widget(frame);
            });
        }

        // 'Default tab indicator color is white in M2 and surfaceVariant in M3'.
        #[test]
        fn the_default_indicator_line_is_white_in_material2_and_outline_variant_in_material3() {
            with_main_thread_stack(|| {
                for material3 in [true, false] {
                    let mut tester = tester();
                    let theme = {
                        let app = tester.cell.borrow();
                        ThemeData::builder()
                            .platform(TargetPlatform::Android)
                            .use_material3(material3)
                            .build(&app)
                    };
                    let frame = build_left_right_app(
                        &tester,
                        vec!["LEFT", "RIGHT"],
                        "LEFT",
                        true,
                        Some(theme.clone()),
                    );
                    tester.pump_widget(frame);
                    let tab_bar_box = first_render_box::<TabBar>(&tester);
                    let color = if material3 {
                        theme.color_scheme.outline_variant()
                    } else {
                        Colors::WHITE
                    };
                    let lines = deep_lines(&tester, tab_bar_box.as_object());
                    assert!(
                        lines
                            .iter()
                            .any(|line| line.color.to_argb32() == color.to_argb32()),
                        "{lines:?}"
                    );
                }
            });
        }

        // 'Tab indicator color should not be adjusted when disable [automaticIndicatorColorAdjustment]'.
        #[test]
        fn the_indicator_color_is_not_adjusted_without_automatic_adjustment() {
            with_main_thread_stack(|| {
                // Dart's theme is Material 3, the default, so only its Material 3 colour is
                // checked: in Material 2 the default indicator is white already
                // (`TabsDefaultsM2`), adjusted or not.
                let mut tester = tester();
                let theme = {
                    let app = tester.cell.borrow();
                    ThemeData::builder()
                        .platform(TargetPlatform::Android)
                        .build(&app)
                };
                let frame = build_left_right_app(
                    &tester,
                    vec!["LEFT", "RIGHT"],
                    "LEFT",
                    false,
                    Some(theme.clone()),
                );
                tester.pump_widget(frame);
                let tab_bar_box = first_render_box::<TabBar>(&tester);
                let color = theme.color_scheme.outline_variant();
                let lines = deep_lines(&tester, tab_bar_box.as_object());
                assert!(
                    lines
                        .iter()
                        .any(|line| line.color.to_argb32() == color.to_argb32()),
                    "{lines:?}"
                );
            });
        }

        /// Records the haptic feedback the platform is asked for.
        #[derive(Default)]
        struct FeedbackTester {
            haptics: RefCell<Vec<waft_embedder::HapticFeedbackType>>,
        }

        impl waft_embedder::Haptics for FeedbackTester {
            fn feedback(&self, kind: waft_embedder::HapticFeedbackType) {
                self.haptics.borrow_mut().push(kind);
            }
        }

        /// A tester whose platform records haptic feedback into the returned tester.
        fn feedback_tester() -> (WidgetTester, Rc<FeedbackTester>) {
            let feedback = Rc::new(FeedbackTester::default());
            let platform = waft_test::TestPlatform::new()
                .on(TargetPlatform::Android)
                .with_haptics(Rc::clone(&feedback) as Rc<dyn waft_embedder::Haptics>);
            let tester = WidgetTester::with_platform(platform);
            tester.install_platform_fonts();
            (tester, feedback)
        }

        // 'Tab feedback is enabled (default)'.
        //
        // The click system sound waits with `SystemSound`, so its count is not checked, as in
        // ink_well.rs's feedback cases.
        #[test]
        fn tab_feedback_is_enabled_by_default() {
            with_main_thread_stack(|| {
                let (mut tester, feedback) = feedback_tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    DefaultTabController::new(1, TabBar::new([Tab::new().text("A").into_widget()])),
                ));
                let ink_well = only::<InkWell>(&tester);
                tap(&mut tester, ink_well);
                tester.pump(Duration::from_secs(1));
                assert_eq!(feedback.haptics.borrow().len(), 0);

                tap(&mut tester, ink_well);
                tester.pump(Duration::from_secs(1));
                assert_eq!(feedback.haptics.borrow().len(), 0);
            });
        }

        // 'Tab feedback is disabled'.
        #[test]
        fn tab_feedback_can_be_disabled() {
            with_main_thread_stack(|| {
                let (mut tester, feedback) = feedback_tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    DefaultTabController::new(
                        1,
                        TabBar::new([Tab::new().text("A").into_widget()]).enable_feedback(false),
                    ),
                ));
                let ink_well = only::<InkWell>(&tester);
                tap(&mut tester, ink_well);
                tester.pump(Duration::from_secs(1));
                assert_eq!(feedback.haptics.borrow().len(), 0);

                let center = center_of(&tester, ink_well);
                tester.long_press(center, 1);
                tester.pump(Duration::from_secs(1));
                assert_eq!(feedback.haptics.borrow().len(), 0);
            });
        }

        /// Dart's `WidgetStateProperty.resolveWith` overlay of the overlay color cases.
        fn hover_pressed_overlay(pressed: Color) -> WidgetStatePropertyRef<Option<Color>> {
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Hovered) {
                    return Some(Color::new(0xff00ff00));
                }
                if states.contains(&WidgetState::Pressed) {
                    return Some(pressed);
                }
                Some(Color::new(0xffbadbad)) // Shouldn't happen.
            })
        }

        /// What the first material's ink features render object and its subtree paint: Dart's
        /// `paints` on `allRenderObjects.firstWhere(_RenderInkFeatures)`.
        fn ink_features_painted(tester: &WidgetTester) -> Vec<Drawn> {
            let ink_features = tester.first::<crate::material::RenderInkFeatures>();
            deep_painted(tester, ink_features.as_object())
        }

        // "Tab's ink well changes color on hover with Tab overlayColor".
        #[test]
        fn a_hovered_tab_paints_the_overlay_colors_hovered_color() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    DefaultTabController::new(
                        1,
                        TabBar::new([Tab::new().text("A").into_widget()])
                            .overlay_color(hover_pressed_overlay(Color::new(0xf00fffff))),
                    ),
                ));
                let mut mouse = WidgetTester::mouse(1, 0);
                tester.add_pointer(&mut mouse, Offset::ZERO);
                let center = center_of(&tester, only::<Tab>(&tester));
                tester.move_to(&mut mouse, center, false);
                tester.pump_and_settle();
                let painted = ink_features_painted(&tester);
                assert!(
                    painted.contains(&Drawn::Rect(
                        0xff00ff00,
                        Rect::from_ltrb(0.0, 276.0, 800.0, 324.0)
                    )),
                    "{painted:?}"
                );
            });
        }

        /// The two skipping cases' app: a 300x200 scaffold, top left, with a tab bar and a view.
        fn skipping_app(
            tester: &WidgetTester,
            controller: Ref<TabController>,
            tabs: &[&str],
            children: Vec<WidgetRef>,
        ) -> WidgetRef {
            let theme = theme(tester, None, None);
            material_app(
                theme,
                Align::new()
                    .alignment(AlignmentGeometry::TOP_LEFT)
                    .child(
                        SizedBox::new().width(300.0).height(200.0).child(
                            Scaffold::new()
                                .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                                    AppBar::new().title(Text::new("tabs")).bottom(
                                        waft_widgets::PreferredSizeWidgetRef::new(
                                            TabBar::new(
                                                tabs.iter()
                                                    .map(|tab| Tab::new().text(*tab).into_widget()),
                                            )
                                            .controller(controller.clone()),
                                        ),
                                    ),
                                ))
                                .body(TabBarView::new(children).controller(controller)),
                        ),
                    )
                    .into_widget(),
            )
        }

        // 'Skipping tabs with global key does not crash'.
        #[test]
        fn skipping_to_a_far_tab_whose_pages_have_global_keys_does_not_panic() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = ["Tab1", "Tab2", "Tab3", "Tab4"];
                let controller = new_controller(&tester, tabs.len());
                let children = ["1", "2", "3", "4"]
                    .map(|page| Text::new(page).key(GlobalKey::new().into()).into_widget())
                    .to_vec();
                let frame = skipping_app(&tester, controller.clone(), &tabs, children);
                tester.pump_widget(frame);
                assert_eq!(tester.find_text("1").len(), 1);
                assert!(tester.find_text("4").is_empty());
                let tab4 = text(&tester, "Tab4");
                tap(&mut tester, tab4);
                tester.pump_and_settle();
                assert_eq!(controller.index(&tester.cell.borrow()), 3);
                assert_eq!(tester.find_text("4").len(), 1);
                assert!(tester.find_text("1").is_empty());
            });
        }

        /// tabs_utils' `TabAlwaysKeepAliveWidget`: a page that always asks to be kept alive.
        #[derive(Debug)]
        struct TabAlwaysKeepAliveWidget {
            key: Option<KeyRef>,
        }

        impl TabAlwaysKeepAliveWidget {
            const TEXT: &str = "AlwaysKeepAlive";
        }

        impl StatefulWidget for TabAlwaysKeepAliveWidget {
            type State = TabAlwaysKeepAliveWidgetState;

            fn key(&self) -> Option<&KeyRef> {
                self.key.as_ref()
            }

            fn create_state(&self) -> TabAlwaysKeepAliveWidgetState {
                TabAlwaysKeepAliveWidgetState {
                    state: StateData::new(),
                    keep_alive: waft_widgets::AutomaticKeepAliveClientMixinData::default(),
                }
            }
        }

        struct TabAlwaysKeepAliveWidgetState {
            state: StateData<TabAlwaysKeepAliveWidget>,
            keep_alive: waft_widgets::AutomaticKeepAliveClientMixinData,
        }

        impl waft_widgets::AutomaticKeepAliveClientMixin for TabAlwaysKeepAliveWidgetState {
            fn automatic_keep_alive_client_data(
                self: Handle<Self>,
                app: &App,
            ) -> &waft_widgets::AutomaticKeepAliveClientMixinData {
                &app.get(self).keep_alive
            }

            fn automatic_keep_alive_client_data_mut(
                self: Handle<Self>,
                app: &mut App,
            ) -> &mut waft_widgets::AutomaticKeepAliveClientMixinData {
                &mut app.get_mut(self).keep_alive
            }

            fn want_keep_alive(self: Handle<Self>, _app: &App) -> bool {
                true
            }
        }

        impl State for TabAlwaysKeepAliveWidgetState {
            type Widget = TabAlwaysKeepAliveWidget;
            waft_widgets::state_accessors!();

            fn init_state(self: Handle<Self>, app: &mut App) {
                waft_widgets::AutomaticKeepAliveClientMixin::init_state(self, app);
            }

            fn deactivate(self: Handle<Self>, app: &mut App) {
                waft_widgets::AutomaticKeepAliveClientMixin::deactivate(self, app);
            }

            fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
                waft_widgets::AutomaticKeepAliveClientMixin::build(self, app);
                Text::new(TabAlwaysKeepAliveWidget::TEXT).into_widget()
            }
        }

        // 'Skipping tabs with a KeepAlive child works'.
        #[test]
        fn skipping_past_a_kept_alive_page_keeps_it() {
            with_main_thread_stack(|| {
                // Regression test for https://github.com/flutter/flutter/issues/11895
                let mut tester = tester();
                let tabs = ["Tab1", "Tab2", "Tab3", "Tab4", "Tab5"];
                let controller = new_controller(&tester, tabs.len());
                let children = vec![
                    TabAlwaysKeepAliveWidget {
                        key: Some(waft_foundation::UniqueKey::new().into()),
                    }
                    .into_widget(),
                    Text::new("2").into_widget(),
                    Text::new("3").into_widget(),
                    Text::new("4").into_widget(),
                    Text::new("5").into_widget(),
                ];
                let frame = skipping_app(&tester, controller.clone(), &tabs, children);
                tester.pump_widget(frame);
                assert_eq!(
                    tester
                        .find_onstage_text(TabAlwaysKeepAliveWidget::TEXT)
                        .len(),
                    1
                );
                assert!(tester.find_onstage_text("4").is_empty());
                let tab4 = text(&tester, "Tab4");
                tap(&mut tester, tab4);
                tester.pump_and_settle();
                tester.pump(Duration::ZERO);
                assert_eq!(controller.index(&tester.cell.borrow()), 3);
                assert_eq!(tester.find_text(TabAlwaysKeepAliveWidget::TEXT).len(), 1);
                assert_eq!(tester.find_onstage_text("4").len(), 1);
            });
        }

        // 'tabbar does not scroll when viewport dimensions initially change from zero to non-zero'.
        #[test]
        fn a_scrollable_tab_bar_whose_viewport_grows_from_zero_does_not_animate() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = ["NEW MEXICO", "GABBA", "HEY"];
                let controller = new_controller(&tester, tabs.len());
                let build_test_widget = |tester: &WidgetTester, width: f64, height: f64| {
                    let theme = theme(tester, None, None);
                    material_app(
                        theme,
                        Center::new()
                            .child(
                                SizedBox::new().height(height).width(width).child(
                                    Scaffold::new()
                                        .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                                            AppBar::new().title(Text::new("AppBarBug")).bottom(
                                                waft_widgets::PreferredSizeWidgetRef::new(
                                                    waft_widgets::PreferredSize::new(
                                                        Size::from_height(30.0),
                                                        Padding::new(
                                                            EdgeInsetsGeometry::symmetric(
                                                                0.0, 15.0,
                                                            ),
                                                        )
                                                        .child(
                                                            Align::new()
                                                                .alignment(
                                                                    AlignmentGeometry::CENTER,
                                                                )
                                                                .child(
                                                                    TabBar::new(tabs.map(|tab| {
                                                                        Tab::new()
                                                                            .text(tab)
                                                                            .into_widget()
                                                                    }))
                                                                    .controller(controller.clone())
                                                                    .is_scrollable(true),
                                                                ),
                                                        ),
                                                    ),
                                                ),
                                            ),
                                        ))
                                        .body(Center::new().child(Text::new("Hello World"))),
                                ),
                            )
                            .into_widget(),
                    )
                };

                let frame = build_test_widget(&tester, 0.0, 0.0);
                tester.pump_widget(frame);

                let frame = build_test_widget(&tester, 300.0, 400.0);
                tester.pump_widget(frame);

                let has_running_animations =
                    SchedulerBinding::transient_callback_count(&mut tester.cell.borrow_mut()) > 0;
                assert!(!has_running_animations);
                assert_eq!(tester.pump_and_settle(), 1); // no more frames are scheduled.
            });
        }

        // 'Number of tabs can be updated dynamically'.
        #[test]
        fn the_number_of_tabs_can_change_and_the_selection_follows() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let key = GlobalKey::new();
                let build_tabs = |tester: &WidgetTester, tabs: &[&str]| {
                    let theme = theme(tester, None, None);
                    boilerplate(
                        theme,
                        TextDirection::Ltr,
                        DefaultTabController::new(
                            tabs.len(),
                            TabBar::new(tabs.iter().map(|tab| Tab::new().text(*tab).into_widget())),
                        )
                        .key(key.clone().into()),
                    )
                };
                let controller_state = |tester: &WidgetTester| {
                    let controller = default_controller_of(tester, text(tester, "A"));
                    let app = tester.cell.borrow();
                    (
                        controller.previous_index(&app),
                        controller.index(&app),
                        controller.length(&app),
                    )
                };

                let frame = build_tabs(&tester, &["A", "B", "C"]);
                tester.pump_widget(frame);
                let b = text(&tester, "B");
                tap(&mut tester, b);
                tester.pump(Duration::ZERO);
                assert_eq!(controller_state(&tester), (0, 1, 3));

                let frame = build_tabs(&tester, &["A", "B"]);
                tester.pump_widget(frame);
                assert_eq!(controller_state(&tester), (0, 1, 2));

                let frame = build_tabs(&tester, &["A"]);
                tester.pump_widget(frame);
                assert_eq!(controller_state(&tester), (1, 0, 1));

                let frame = build_tabs(&tester, &["A", "B"]);
                tester.pump_widget(frame);
                assert_eq!(controller_state(&tester), (1, 0, 2));
            });
        }
    }

    mod label_colors_lengths_and_alignments {
        //! tabs_test.dart cases, part d: from 'TabBar with one tab has correct color' through
        //! 'Material3 - Scrollable TabBar without a divider does not expand to full width'.

        use std::cell::{Cell, RefCell};
        use std::panic::{AssertUnwindSafe, catch_unwind};

        use waft_widgets::{PreferredSizeWidgetRef, StatefulBuilder, WidgetStateColor};

        use super::*;
        use crate::icon_button::IconButton;
        use crate::icons::Icons;
        use crate::test_support::TestVSync;
        use crate::text_button::TextButton;

        /// Dart's `createTabController(length:, vsync: const TestVSync())`.
        fn create_tab_controller(tester: &WidgetTester, length: usize) -> Ref<TabController> {
            let mut app = tester.cell.borrow_mut();
            TabController::new(&mut app, 0, None, length, TestVSync)
        }

        /// Dart's `IconTheme.of(tester.element(find.text(text))).color`.
        fn icon_color(tester: &WidgetTester, label: &str) -> Option<Color> {
            let element = text(tester, label);
            let mut app = tester.cell.borrow_mut();
            IconTheme::of(&mut app, element)
                .color
                .as_ref()
                .map(AnyColor::color)
        }

        /// Dart's `tester.renderObject<RenderParagraph>(find.text(text)).text.style!`.
        fn paragraph_style(tester: &WidgetTester, label: &str) -> TextStyle {
            let render = tester.render_box_of(text(tester, label));
            let app = tester.cell.borrow();
            let paragraph = render
                .as_object()
                .downcast::<waft_rendering::RenderParagraph>(&app)
                .expect("a Text renders a paragraph");
            paragraph
                .text(&app)
                .style()
                .cloned()
                .expect("the paragraph's text has a style")
        }

        fn style_color(style: &TextStyle) -> Option<Color> {
            style.color.as_ref().map(AnyColor::color)
        }

        /// The panic message of `panic`.
        fn panic_message(panic: Box<dyn Any + Send>) -> String {
            panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    panic
                        .downcast_ref::<&str>()
                        .map(|message| message.to_string())
                })
                .unwrap_or_default()
        }

        /// Dart's `tester.pumpWidget(widget)` followed by `tester.takeException()`: the message of
        /// the panic the frame raised, if any.
        fn pump_and_take_exception(tester: &mut WidgetTester, widget: WidgetRef) -> Option<String> {
            catch_unwind(AssertUnwindSafe(|| tester.pump_widget(widget)))
                .err()
                .map(panic_message)
        }

        /// Dart's `find.byKey(key)` tapped at its centre.
        fn tap_key(tester: &mut WidgetTester, key: &GlobalKey) {
            let center = tester.center_of(key);
            tester.tap(center, 1);
        }

        /// Dart's `find.widgetWithIcon(Tab, icon)`.
        fn tab_with_icon(tester: &WidgetTester, icon: &waft_widgets::IconData) -> AnyElement {
            let icon = tester.find_by_icon(icon);
            tester.ancestor_of_type::<Tab>(icon)
        }

        /// Dart's `find.widgetWithText(W, text).first`: the nearest `W` above the text.
        fn nearest_with_text<W: 'static>(tester: &WidgetTester, label: &str) -> AnyElement {
            tester.ancestor_of_type::<W>(text(tester, label))
        }

        /// The `PageController` of the only `PageView`: Dart's
        /// `tester.widget<PageView>(find.byType(PageView)).controller!`.
        fn page_controller(tester: &WidgetTester) -> Ref<PageController> {
            tester.widget(only::<PageView>(tester), |page_view: &PageView| {
                page_view
                    .controller
                    .clone()
                    .expect("a tab bar view's page view has a controller")
            })
        }

        /// `tabs_utils.dart`'s `TabBody`: logs its state's `init` and `dispose`.
        #[derive(Debug)]
        struct TabBody {
            index: usize,
            log: Rc<RefCell<Vec<String>>>,
        }

        impl StatefulWidget for TabBody {
            type State = TabBodyState;

            fn create_state(&self) -> TabBodyState {
                TabBodyState {
                    state: StateData::new(),
                }
            }
        }

        struct TabBodyState {
            state: StateData<TabBody>,
        }

        impl State for TabBodyState {
            type Widget = TabBody;
            waft_widgets::state_accessors!();

            fn init_state(self: Handle<Self>, app: &mut App) {
                let widget = self.widget(app);
                widget
                    .log
                    .borrow_mut()
                    .push(format!("init: {}", widget.index));
            }

            fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &TabBody) {
                assert_eq!(old_widget.index, self.widget(app).index);
            }

            fn dispose(self: Handle<Self>, app: &mut App) {
                let widget = self.widget(app);
                widget
                    .log
                    .borrow_mut()
                    .push(format!("dispose: {}", widget.index));
            }

            fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
                Center::new()
                    .child(Text::new(format!("{}", self.widget(app).index)))
                    .into_widget()
            }
        }

        /// Dart's `Scaffold(appBar: AppBar(bottom: tab_bar, ..), body: body)`.
        fn scaffold_with_tab_bar(
            app_bar: AppBar,
            tab_bar: TabBar,
            body: Option<WidgetRef>,
        ) -> WidgetRef {
            let mut scaffold = Scaffold::new().app_bar(PreferredSizeWidgetRef::new(
                app_bar.bottom(PreferredSizeWidgetRef::new(tab_bar)),
            ));
            if let Some(body) = body {
                scaffold = scaffold.body(body);
            }
            scaffold.into_widget()
        }

        // Regression test for https://github.com/flutter/flutter/issues/15008.
        // 'TabBar with one tab has correct color'.
        #[test]
        fn a_tab_bar_with_one_tab_gives_it_the_selected_color() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const SELECTED_TAB_COLOR: Color = Color::new(0x00000001);
                const UNSELECTED_TAB_COLOR: Color = Color::new(0x00000002);

                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    DefaultTabController::new(
                        1,
                        TabBar::new([Tab::new().text("A").into_widget()])
                            .label_color(SELECTED_TAB_COLOR)
                            .unselected_label_color(UNSELECTED_TAB_COLOR),
                    ),
                ));

                assert_eq!(icon_color(&tester, "A"), Some(SELECTED_TAB_COLOR));
            });
        }

        /// Dart's `WidgetStateColor.resolveWith` answering `selected` when selected, else
        /// `unselected`.
        fn selected_state_color(selected: Color, unselected: Color) -> WidgetStateColor {
            WidgetStateColor::resolve_with(move |states| {
                if states.contains(&WidgetState::Selected) {
                    return selected;
                }
                unselected
            })
        }

        // 'TabBar.labelColor resolves material states'.
        #[test]
        fn a_widget_state_label_color_resolves_per_tab_selection() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const TAB1: &str = "Tab 1";
                const TAB2: &str = "Tab 2";
                const SELECTED_COLOR: Color = Color::new(0xff00ff00);
                const UNSELECTED_COLOR: Color = Color::new(0xffff0000);
                let label_color = selected_state_color(SELECTED_COLOR, UNSELECTED_COLOR);

                // Test labelColor correctly resolves material states.
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    DefaultTabController::new(
                        2,
                        TabBar::new([Text::new(TAB1).into_widget(), Text::new(TAB2).into_widget()])
                            .label_color(label_color),
                    ),
                ));

                assert_eq!(icon_color(&tester, TAB1), Some(SELECTED_COLOR));
                assert_eq!(icon_color(&tester, TAB2), Some(UNSELECTED_COLOR));
                assert_eq!(
                    style_color(&paragraph_style(&tester, TAB1)),
                    Some(SELECTED_COLOR)
                );
                assert_eq!(
                    style_color(&paragraph_style(&tester, TAB2)),
                    Some(UNSELECTED_COLOR)
                );
            });
        }

        // 'labelColor & unselectedLabelColor override material state labelColor'.
        #[test]
        fn plain_label_colors_replace_a_widget_state_label_color() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const TAB1: &str = "Tab 1";
                const TAB2: &str = "Tab 2";
                const SELECTED_STATE_COLOR: Color = Color::new(0xff00ff00);
                const UNSELECTED_STATE_COLOR: Color = Color::new(0xffff0000);
                const SELECTED_COLOR: Color = Color::new(0xff00ffff);
                const UNSELECTED_COLOR: Color = Color::new(0xffff12ff);

                let build_tab_bar = |tester: &WidgetTester, state_color: bool| {
                    let mut tab_bar =
                        TabBar::new([Text::new(TAB1).into_widget(), Text::new(TAB2).into_widget()]);
                    if state_color {
                        tab_bar = tab_bar.label_color(selected_state_color(
                            SELECTED_STATE_COLOR,
                            UNSELECTED_STATE_COLOR,
                        ));
                    } else {
                        tab_bar = tab_bar
                            .label_color(SELECTED_COLOR)
                            .unselected_label_color(UNSELECTED_COLOR);
                    }
                    boilerplate(
                        theme(tester, None, None),
                        TextDirection::Ltr,
                        DefaultTabController::new(2, tab_bar),
                    )
                };

                // Test material state label color.
                let frame = build_tab_bar(&tester, true);
                tester.pump_widget(frame);

                assert_eq!(icon_color(&tester, TAB1), Some(SELECTED_STATE_COLOR));
                assert_eq!(icon_color(&tester, TAB2), Some(UNSELECTED_STATE_COLOR));
                assert_eq!(
                    style_color(&paragraph_style(&tester, TAB1)),
                    Some(SELECTED_STATE_COLOR)
                );
                assert_eq!(
                    style_color(&paragraph_style(&tester, TAB2)),
                    Some(UNSELECTED_STATE_COLOR)
                );

                // Test labelColor & unselectedLabelColor override material state labelColor.
                let frame = build_tab_bar(&tester, false);
                tester.pump_widget(frame);

                assert_eq!(icon_color(&tester, TAB1), Some(SELECTED_COLOR));
                assert_eq!(icon_color(&tester, TAB2), Some(UNSELECTED_COLOR));
                assert_eq!(
                    style_color(&paragraph_style(&tester, TAB1)),
                    Some(SELECTED_COLOR)
                );
                assert_eq!(
                    style_color(&paragraph_style(&tester, TAB2)),
                    Some(UNSELECTED_COLOR)
                );
            });
        }

        // Regression test for https://github.com/flutter/flutter/issues/32428
        // 'Replacing the tabController after disposing the old one'.
        #[test]
        fn a_tab_bar_takes_a_new_controller_after_the_old_one_is_disposed() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = Rc::new(RefCell::new(create_tab_controller(&tester, 2)));

                let theme = theme(&tester, None, None);
                let shown = Rc::clone(&controller);
                tester.pump_widget(material_app(
                    theme,
                    StatefulBuilder::new(move |app, _context, set_state| {
                        let current = shown.borrow().clone();
                        let length = current.length(app);
                        let replaced = Rc::clone(&shown);
                        let change = TextButton::new(
                            Some(Listener::new(move |app| {
                                let replaced = Rc::clone(&replaced);
                                set_state(app, &mut move |app| {
                                    replaced.borrow().dispose(app);
                                    *replaced.borrow_mut() =
                                        TabController::new(app, 0, None, 3, TestVSync);
                                });
                            })),
                            Some(Text::new("Change TabController length")),
                        );
                        scaffold_with_tab_bar(
                            AppBar::new().actions(vec![change.into_widget()]),
                            TabBar::new(
                                (0..length).map(|index| {
                                    Tab::new().text(format!("Tab{index}")).into_widget()
                                }),
                            )
                            .controller(current.clone()),
                            Some(
                                TabBarView::new((0..length).map(|index| {
                                    Center::new()
                                        .child(Text::new(format!("Tab {index}")))
                                        .into_widget()
                                }))
                                .controller(current.clone())
                                .into_widget(),
                            ),
                        )
                    })
                    .into_widget(),
                ));

                {
                    let app = tester.cell.borrow();
                    assert_eq!(controller.borrow().index(&app), 0);
                    assert_eq!(controller.borrow().length(&app), 2);
                }
                assert_eq!(tester.find_text("Tab0").len(), 1);
                assert_eq!(tester.find_text("Tab1").len(), 1);
                assert!(tester.find_text("Tab2").is_empty());

                let button = text(&tester, "Change TabController length");
                tap(&mut tester, button);
                tester.pump_and_settle();
                {
                    let app = tester.cell.borrow();
                    assert_eq!(controller.borrow().index(&app), 0);
                    assert_eq!(controller.borrow().length(&app), 3);
                }
                assert_eq!(tester.find_text("Tab0").len(), 1);
                assert_eq!(tester.find_text("Tab1").len(), 1);
                assert_eq!(tester.find_text("Tab2").len(), 1);
            });
        }

        /// The app of 'DefaultTabController should allow for a length of zero' and of 'Do not crash if
        /// the controller and TabBarView are updated at different phases(build and layout) of the same
        /// frame': Dart's `BottomAppBar` is not ported, so its `Row` of buttons is the scaffold's
        /// bottom navigation bar itself; the cases read neither its size nor its colour.
        fn zero_length_app(
            tester: &WidgetTester,
            add_key: GlobalKey,
            delete_key: GlobalKey,
            body_in_layout_builder: bool,
        ) -> WidgetRef {
            let tab_text_content: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
            let theme = theme(tester, None, None);
            material_app(
                theme,
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let contents = tab_text_content.borrow().clone();
                    let mut app_bar = AppBar::new().title(Text::new("Default TabBar Preview"));
                    if !contents.is_empty() {
                        app_bar = app_bar.bottom(PreferredSizeWidgetRef::new(
                            TabBar::new(
                                contents
                                    .iter()
                                    .map(|content| Tab::new().text(content.clone()).into_widget()),
                            )
                            .is_scrollable(true),
                        ));
                    }
                    let body_contents = contents.clone();
                    let build_body = move || {
                        if body_contents.is_empty() {
                            Center::new().child(Text::new("No tabs")).into_widget()
                        } else {
                            TabBarView::new(body_contents.iter().map(|content| {
                                Tab::new().text(format!("{content}'s view")).into_widget()
                            }))
                            .into_widget()
                        }
                    };
                    let body = if body_in_layout_builder {
                        waft_widgets::LayoutBuilder::new(move |_app, _context, _constraints| {
                            build_body()
                        })
                        .into_widget()
                    } else {
                        build_body()
                    };
                    let added = Rc::clone(&tab_text_content);
                    let add_state = set_state.clone();
                    let add = IconButton::new(
                        Some(Listener::new(move |app| {
                            let added = Rc::clone(&added);
                            add_state(app, &mut move |_app| {
                                let mut contents = added.borrow().clone();
                                let next = format!("Tab {}", contents.len() + 1);
                                contents.push(next);
                                *added.borrow_mut() = contents;
                            });
                        })),
                        waft_widgets::Icon::new(Some(Icons::add())),
                    )
                    .key(add_key.clone().into());
                    let deleted = Rc::clone(&tab_text_content);
                    let delete_state = set_state.clone();
                    let delete = IconButton::new(
                        Some(Listener::new(move |app| {
                            let deleted = Rc::clone(&deleted);
                            delete_state(app, &mut move |_app| {
                                let mut contents = deleted.borrow().clone();
                                contents.pop();
                                *deleted.borrow_mut() = contents;
                            });
                        })),
                        waft_widgets::Icon::new(Some(Icons::delete())),
                    )
                    .key(delete_key.clone().into());
                    DefaultTabController::new(
                        contents.len(),
                        Scaffold::new()
                            .app_bar(PreferredSizeWidgetRef::new(app_bar))
                            .body(body)
                            .bottom_navigation_bar(
                                Row::new()
                                    .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                                    .children([add.into_widget(), delete.into_widget()]),
                            ),
                    )
                    .into_widget()
                })
                .into_widget(),
            )
        }

        // Regression test for https://github.com/flutter/flutter/issues/20292.
        // 'DefaultTabController should allow for a length of zero'.
        #[test]
        fn a_default_tab_controller_can_have_no_tabs() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let add_key = GlobalKey::new();
                let delete_key = GlobalKey::new();
                let app = zero_length_app(&tester, add_key.clone(), delete_key.clone(), false);
                tester.pump_widget(app);

                // Initializes with zero tabs properly
                assert_eq!(tester.find_text("No tabs").len(), 1);
                tap_key(&mut tester, &add_key);
                tester.pump_and_settle();
                assert_eq!(tester.find_text("Tab 1").len(), 1);
                assert_eq!(tester.find_text("Tab 1's view").len(), 1);

                // Dynamically updates to zero tabs properly
                tap_key(&mut tester, &delete_key);
                tester.pump_and_settle();
                assert_eq!(tester.find_text("No tabs").len(), 1);
            });
        }

        /// The app of the two 'add tab' / 'remove tab' cases: an `AppBar` whose bottom is a 40 high
        /// `PreferredSize` around an `Expanded` `TabBar` of `titles`, with `controller` when given
        /// and under a `DefaultTabController` otherwise.
        fn add_remove_app(
            tester: &WidgetTester,
            titles: Rc<RefCell<Vec<String>>>,
            controller: Option<Rc<RefCell<Ref<TabController>>>>,
            add_key: GlobalKey,
            remove_key: GlobalKey,
        ) -> WidgetRef {
            let theme = theme(tester, None, None);
            material_app(
                theme,
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let current_titles = titles.borrow().clone();
                    let added = Rc::clone(&titles);
                    let added_controller = controller.clone();
                    let add_state = set_state.clone();
                    let add = TextButton::new(
                        Some(Listener::new(move |app| {
                            let added = Rc::clone(&added);
                            let added_controller = added_controller.clone();
                            add_state(app, &mut move |app| {
                                let next = format!("Tab {}", added.borrow().len() + 1);
                                added.borrow_mut().push(next);
                                if let Some(controller) = &added_controller {
                                    let length = added.borrow().len();
                                    *controller.borrow_mut() =
                                        TabController::new(app, 0, None, length, TestVSync);
                                }
                            });
                        })),
                        Some(Text::new("Add tab")),
                    )
                    .key(add_key.clone().into());
                    let removed = Rc::clone(&titles);
                    let removed_controller = controller.clone();
                    let remove_state = set_state.clone();
                    let remove = TextButton::new(
                        Some(Listener::new(move |app| {
                            let removed = Rc::clone(&removed);
                            let removed_controller = removed_controller.clone();
                            remove_state(app, &mut move |app| {
                                removed.borrow_mut().pop();
                                if let Some(controller) = &removed_controller {
                                    let length = removed.borrow().len();
                                    *controller.borrow_mut() =
                                        TabController::new(app, 0, None, length, TestVSync);
                                }
                            });
                        })),
                        Some(Text::new("Remove tab")),
                    )
                    .key(remove_key.clone().into());
                    let mut tab_bar = TabBar::new(
                        current_titles
                            .iter()
                            .map(|title| Tab::new().text(title.clone()).into_widget()),
                    );
                    if let Some(controller) = &controller {
                        tab_bar = tab_bar.controller(controller.borrow().clone());
                    }
                    let scaffold = Scaffold::new().app_bar(PreferredSizeWidgetRef::new(
                        AppBar::new()
                            .actions(vec![add.into_widget(), remove.into_widget()])
                            .bottom(PreferredSizeWidgetRef::new(
                                waft_widgets::PreferredSize::new(
                                    Size::from_height(40.0),
                                    Expanded::new(tab_bar),
                                ),
                            )),
                    ));
                    match controller {
                        Some(_) => scaffold.into_widget(),
                        None => {
                            DefaultTabController::new(current_titles.len(), scaffold).into_widget()
                        }
                    }
                })
                .into_widget(),
            )
        }

        // Regression test for https://github.com/flutter/flutter/issues/94504.
        // 'DefaultTabController should allow dynamic length of tabs'.
        #[test]
        fn a_default_tab_controller_follows_a_changing_number_of_tabs() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let add_key = GlobalKey::new();
                let remove_key = GlobalKey::new();
                let titles = Rc::new(RefCell::new(Vec::new()));
                let app =
                    add_remove_app(&tester, titles, None, add_key.clone(), remove_key.clone());
                tester.pump_widget(app);

                assert!(tester.find_text("Tab 1").is_empty());
                assert!(tester.find_text("Tab 2").is_empty());

                tap_key(&mut tester, &add_key); // +1
                tester.pump_and_settle();
                assert_eq!(tester.find_text("Tab 1").len(), 1);
                assert!(tester.find_text("Tab 2").is_empty());

                tap_key(&mut tester, &add_key); // +2
                tester.pump_and_settle();
                assert_eq!(tester.find_text("Tab 1").len(), 1);
                assert_eq!(tester.find_text("Tab 2").len(), 1);

                tap_key(&mut tester, &remove_key); // -2
                tap_key(&mut tester, &remove_key); // -1
                tester.pump_and_settle();
                assert!(tester.find_text("Tab 1").is_empty());
                assert!(tester.find_text("Tab 2").is_empty());
            });
        }

        // Regression test for https://github.com/flutter/flutter/issues/68962.
        // 'TabBar - updating to and from zero tabs'.
        #[test]
        fn a_tab_bar_goes_to_and_from_no_tabs_with_new_controllers() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let add_key = GlobalKey::new();
                let remove_key = GlobalKey::new();
                let titles = Rc::new(RefCell::new(Vec::new()));
                let controller = Rc::new(RefCell::new(create_tab_controller(&tester, 0)));
                let app = add_remove_app(
                    &tester,
                    titles,
                    Some(controller),
                    add_key.clone(),
                    remove_key.clone(),
                );
                tester.pump_widget(app);

                assert!(tester.find_text("Tab 1").is_empty());
                assert_eq!(tester.find_text("Add tab").len(), 1);
                tap_key(&mut tester, &add_key);
                tester.pump_and_settle();
                assert_eq!(tester.find_text("Tab 1").len(), 1);

                tap_key(&mut tester, &remove_key);
                tester.pump_and_settle();
                assert!(tester.find_text("Tab 1").is_empty());
            });
        }

        // 'TabBar expands vertically to accommodate the Icon and child Text() pair the same amount it
        // would expand for Icon and text pair.'
        #[test]
        fn an_icon_with_a_text_child_tab_bar_is_as_tall_as_an_icon_with_text_one() {
            let tab_bar_with_text = TabBar::new([Tab::new()
                .icon(waft_widgets::Icon::new(Some(Icons::notifications())))
                .text("Test")
                .into_widget()]);
            let tab_bar_with_text_child = TabBar::new([Tab::new()
                .icon(waft_widgets::Icon::new(Some(Icons::notifications())))
                .child(Text::new("Test"))
                .into_widget()]);

            assert_eq!(
                tab_bar_with_text.preferred_size(),
                tab_bar_with_text_child.preferred_size()
            );
        }

        /// The left edge of the indicator `render` paints in `indicator_color`, as `tabs_utils.dart`'s
        /// `TabIndicatorRecordingCanvas` records it: the line's rect inflated by half the default
        /// indicator weight. Painted with `app`, so a listener that runs inside a frame can read it.
        fn indicator_left(app: &mut App, render: AnyRenderObject, indicator_color: Color) -> f64 {
            use waft_embedder::valo::{Op, PaintStyle as ValoPaintStyle};
            use waft_rendering::ContainerLayer;
            const INDICATOR_WEIGHT: f64 = 2.0;
            let layer = waft_rendering::OffsetLayer::new(app, Offset::ZERO);
            let container = layer.as_container_layer();
            let bounds = render.paint_bounds(app);
            let mut context = PaintingContext::new(container, bounds);
            context.paint_child(app, render, Offset::ZERO);
            context.stop_recording_if_needed(app);
            let mut left = None;
            let mut child = container.first_child(app);
            while let Some(current) = child {
                if let Some(picture) = app
                    .handle::<waft_rendering::PictureLayer>(current.id())
                    .and_then(|picture| picture.picture(app))
                {
                    for op in waft_embedder::flattened_ops(picture) {
                        if let Op::DrawPath { path, paint, .. } = op
                            && matches!(paint.style, ValoPaintStyle::Stroke(_))
                            && path.elements().count() == 2
                            && super::argb32(paint.color) == indicator_color.to_argb32()
                        {
                            left = Some(f64::from(path.bounds().x) - INDICATOR_WEIGHT / 2.0);
                        }
                    }
                }
                child = current.next_sibling(app);
            }
            left.expect("the indicator line was painted")
        }

        // 'Setting TabController index should make TabBar indicator immediately pop into the
        // position'. Dart's `TabControllerFrame` makes the controller with its state as the vsync;
        // here it is made with `TestVSync`, which ticks the same frames.
        #[test]
        fn setting_the_index_moves_the_indicator_at_once() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_COLOR: Color = Color::new(0xFFFF0000);
                let tab_controller = create_tab_controller(&tester, 3);
                let tabs = || ["A", "B", "C"].map(|text| Tab::new().text(text).into_widget());

                let theme = theme(&tester, Some(false), None);
                tester.pump_widget(material_app(
                    theme,
                    scaffold_with_tab_bar(
                        AppBar::new(),
                        TabBar::new(tabs())
                            .controller(tab_controller.clone())
                            .indicator_color(INDICATOR_COLOR),
                        Some(
                            TabBarView::new(
                                ["A", "B", "C"]
                                    .map(|text| Center::new().child(Text::new(text)).into_widget()),
                            )
                            .controller(tab_controller.clone())
                            .into_widget(),
                        ),
                    ),
                ));

                let tab_bar = tester.render_box_of(only::<TabBar>(&tester)).as_object();
                let expected_indicator_left = Rc::new(Cell::new(indicator_left(
                    &mut tester.cell.borrow_mut(),
                    tab_bar,
                    INDICATOR_COLOR,
                )));

                let page_controller = page_controller(&tester);
                let expected = Rc::clone(&expected_indicator_left);
                // Whenever TabBarView scrolls due to changing TabController's index,
                // check if indicator stays idle in its expectedIndicatorLeft
                let page_controller_listener = Listener::new(move |app| {
                    assert_eq!(
                        indicator_left(app, tab_bar, INDICATOR_COLOR),
                        expected.get()
                    );
                });

                // Moving from index 0 to 2 (distanced tabs)
                {
                    let mut app = tester.cell.borrow_mut();
                    tab_controller.set_index(&mut app, 2);
                    expected_indicator_left.set(indicator_left(&mut app, tab_bar, INDICATOR_COLOR));
                    Listenable::add_listener(
                        &page_controller,
                        &mut app,
                        page_controller_listener.clone(),
                    );
                }
                tester.pump_and_settle();

                // Moving from index 2 to 1 (neighboring tabs)
                {
                    let mut app = tester.cell.borrow_mut();
                    tab_controller.set_index(&mut app, 1);
                    expected_indicator_left.set(indicator_left(&mut app, tab_bar, INDICATOR_COLOR));
                }
                tester.pump_and_settle();
                let mut app = tester.cell.borrow_mut();
                Listenable::remove_listener(&page_controller, &mut app, &page_controller_listener);
            });
        }

        // Regression test for https://github.com/flutter/flutter/issues/57708
        // 'Setting BouncingScrollPhysics on TabBarView does not include ClampingScrollPhysics'.
        #[test]
        fn bouncing_physics_on_a_tab_bar_view_leave_out_clamping_physics() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    DefaultTabController::new(
                        10,
                        Scaffold::new().body(
                            TabBarView::new((0..10).map(|i| {
                                Center::new()
                                    .child(Text::new(format!("index {i}")))
                                    .into_widget()
                            }))
                            .physics(Rc::new(waft_widgets::BouncingScrollPhysics::new())),
                        ),
                    )
                    .into_widget(),
                ));

                let physics = tester.widget(only::<PageView>(&tester), |page_view: &PageView| {
                    page_view
                        .physics
                        .clone()
                        .expect("a tab bar view gives its page view physics")
                });
                let mut physics = Some(physics);
                while let Some(current) = physics {
                    assert!(
                        !current.as_any().is::<ClampingScrollPhysics>(),
                        "{current:?}"
                    );
                    physics = current.parent().cloned();
                }
            });
        }

        // 'TabController.offset changes reflect labelColor'.
        #[test]
        fn the_controller_offset_lerps_the_label_colors() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = create_tab_controller(&tester, 2);
                let first_color: Rc<Cell<Option<Color>>> = Rc::new(Cell::new(None));
                let second_color: Rc<Cell<Option<Color>>> = Rc::new(Cell::new(None));

                let build_tab_bar = |tester: &WidgetTester, state_color: bool| {
                    let first = Rc::clone(&first_color);
                    let second = Rc::clone(&second_color);
                    let mut tab_bar = TabBar::new([
                        waft_widgets::Builder::new(move |app, context| {
                            first.set(style_color(&DefaultTextStyle::of(app, context).style));
                            Text::new("First").into_widget()
                        })
                        .into_widget(),
                        waft_widgets::Builder::new(move |app, context| {
                            second.set(style_color(&DefaultTextStyle::of(app, context).style));
                            Text::new("Second").into_widget()
                        })
                        .into_widget(),
                    ])
                    .controller(controller.clone())
                    .unselected_label_color(Colors::BLACK);
                    tab_bar = if state_color {
                        // this is a third color to also test if unselectedLabelColor
                        // is ignored when labelColor is WidgetStateColor
                        tab_bar
                            .label_color(selected_state_color(Colors::WHITE, Colors::TRANSPARENT))
                    } else {
                        tab_bar.label_color(Colors::WHITE)
                    };
                    boilerplate(theme(tester, None, None), TextDirection::Ltr, tab_bar)
                };

                let test_label_color =
                    |tester: &mut WidgetTester, selected_color: Color, unselected_color: Color| {
                        let lerp = |a: Color, b: Color, t: f64| Color::lerp(Some(a), Some(b), t);
                        assert_eq!(first_color.get(), Some(selected_color));
                        assert_eq!(second_color.get(), Some(unselected_color));

                        controller.set_offset(&mut tester.cell.borrow_mut(), 0.6);
                        tester.pump(Duration::ZERO);

                        assert_eq!(
                            first_color.get(),
                            lerp(selected_color, unselected_color, 0.6)
                        );
                        assert_eq!(
                            second_color.get(),
                            lerp(unselected_color, selected_color, 0.6)
                        );

                        controller.set_index(&mut tester.cell.borrow_mut(), 1);
                        tester.pump(Duration::ZERO);

                        assert_eq!(first_color.get(), Some(unselected_color));
                        assert_eq!(second_color.get(), Some(selected_color));

                        controller.set_offset(&mut tester.cell.borrow_mut(), 0.6);
                        tester.pump(Duration::ZERO);

                        assert_eq!(first_color.get(), Some(unselected_color));
                        assert_eq!(second_color.get(), Some(selected_color));

                        controller.set_offset(&mut tester.cell.borrow_mut(), -0.6);
                        tester.pump(Duration::ZERO);

                        assert_eq!(
                            first_color.get(),
                            lerp(selected_color, unselected_color, 0.4)
                        );
                        assert_eq!(
                            second_color.get(),
                            lerp(unselected_color, selected_color, 0.4)
                        );
                    };

                let frame = build_tab_bar(&tester, false);
                tester.pump_widget(frame);
                test_label_color(&mut tester, Colors::WHITE, Colors::BLACK);

                // reset
                controller.set_index(&mut tester.cell.borrow_mut(), 0);
                tester.pump(Duration::ZERO);

                let frame = build_tab_bar(&tester, true);
                tester.pump_widget(frame);
                test_label_color(&mut tester, Colors::WHITE, Colors::TRANSPARENT);
            });
        }

        // 'No crash on dispose'.
        #[test]
        fn tapping_a_tab_while_its_view_disposes_pages_does_not_panic() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                let icon = |icon: waft_widgets::IconData| {
                    waft_widgets::Icon::new(Some(icon)).into_widget()
                };
                tester.pump_widget(material_app(
                    theme,
                    DefaultTabController::new(
                        3,
                        scaffold_with_tab_bar(
                            AppBar::new().title(Text::new("Tabs Demo")),
                            TabBar::new([
                                Tab::new().icon(icon(Icons::directions_car())).into_widget(),
                                Tab::new()
                                    .icon(icon(Icons::directions_transit()))
                                    .into_widget(),
                                Tab::new()
                                    .icon(icon(Icons::directions_bike()))
                                    .into_widget(),
                            ]),
                            Some(
                                TabBarView::new([
                                    icon(Icons::directions_car()),
                                    icon(Icons::directions_transit()),
                                    icon(Icons::directions_bike()),
                                ])
                                .into_widget(),
                            ),
                        ),
                    )
                    .into_widget(),
                ));
                let bike = tester.find_by_icon(&Icons::directions_bike());
                // No crash on dispose.
                let tapped = catch_unwind(AssertUnwindSafe(|| tap(&mut tester, bike)));
                assert!(tapped.is_ok());
            });
        }

        // "TabController's animation value should be in sync with TabBarView's scroll value when user
        // interrupts ballistic scroll".
        #[test]
        fn the_controller_animation_follows_the_page_when_a_touch_interrupts_a_ballistic_scroll() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tab_controller = create_tab_controller(&tester, 3);

                tester.pump_widget(
                    Directionality::new(
                        TextDirection::Ltr,
                        SizedBox::expand().child(
                            Center::new().child(
                                SizedBox::square(Some(400.0)).child(
                                    TabBarView::new(["0", "1", "2"].map(|text| {
                                        Center::new().child(Text::new(text)).into_widget()
                                    }))
                                    .controller(tab_controller.clone()),
                                ),
                            ),
                        ),
                    )
                    .into_widget(),
                );

                let page_controller = page_controller(&tester);
                {
                    let app = tester.cell.borrow();
                    let position = page_controller.position(&app);
                    assert_eq!(tab_controller.index(&app), 0);
                    assert_eq!(position.pixels(&app), 0.0);
                }

                page_controller.jump_to(&mut tester.cell.borrow_mut(), 300.0);
                tester.pump(Duration::ZERO);
                let animation_value = |tester: &WidgetTester| {
                    let app = tester.cell.borrow();
                    tab_controller
                        .animation(&app)
                        .expect("a live controller has an animation")
                        .value(&app)
                };
                assert_eq!(
                    Some(animation_value(&tester)),
                    page_controller.page(&tester.cell.borrow())
                );

                // Touch TabBarView while ballistic scrolling is happening and
                // check if tabController's animation value properly follows page value.
                let center = center_of(&tester, only::<PageView>(&tester));
                let mut gesture = WidgetTester::touch(1);
                tester.down(&mut gesture, center);
                tester.pump(Duration::ZERO);
                assert_eq!(
                    Some(animation_value(&tester)),
                    page_controller.page(&tester.cell.borrow())
                );

                // Finish gesture to release resources.
                tester.up(&gesture);
                tester.pump_and_settle();
            });
        }

        // Regression test for https://github.com/flutter/flutter/issues/14316.
        // 'Does not instantiate intermediate tabs during animation'.
        #[test]
        fn skipping_to_a_distant_tab_builds_no_page_in_between() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let log = Rc::new(RefCell::new(Vec::new()));
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    DefaultTabController::new(
                        5,
                        scaffold_with_tab_bar(
                            AppBar::new().title(Text::new("Tabs Test")),
                            TabBar::new(
                                ["car", "transit", "bike", "boat", "bus"]
                                    .map(|text| Tab::new().text(text).into_widget()),
                            ),
                            Some(
                                TabBarView::new((0..5).map(|index| {
                                    TabBody {
                                        index,
                                        log: Rc::clone(&log),
                                    }
                                    .into_widget()
                                }))
                                .into_widget(),
                            ),
                        ),
                    )
                    .into_widget(),
                ));

                assert_eq!(tester.find_text("0").len(), 1);
                assert!(tester.find_text("3").is_empty());
                assert_eq!(*log.borrow(), vec!["init: 0"]);

                let boat = text(&tester, "boat");
                tap(&mut tester, boat);
                tester.pump_and_settle();

                assert!(tester.find_text("0").is_empty());
                assert_eq!(tester.find_text("3").len(), 1);

                // No other tab got instantiated during the animation.
                assert_eq!(*log.borrow(), vec!["init: 0", "init: 3", "dispose: 0"]);
            });
        }

        // This is a regression test for the issue brought up here
        // https://github.com/flutter/flutter/issues/79226
        // "TabController's animation value should be updated when TabController's index >= tabs's
        // length".
        #[test]
        fn removing_the_selected_last_tab_moves_the_animation_to_the_new_last_tab() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs = Rc::new(RefCell::new(vec!["A", "B", "C"]));
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    StatefulBuilder::new(move |_app, _context, set_state| {
                        let current = tabs.borrow().clone();
                        let removed = Rc::clone(&tabs);
                        let remove = TextButton::new(
                            Some(Listener::new(move |app| {
                                let removed = Rc::clone(&removed);
                                set_state(app, &mut move |_app| {
                                    removed.borrow_mut().pop();
                                });
                            })),
                            Some(Text::new("Remove Last Tab")),
                        );
                        DefaultTabController::new(
                            current.len(),
                            scaffold_with_tab_bar(
                                AppBar::new().actions(vec![remove.into_widget()]),
                                TabBar::new(
                                    current
                                        .iter()
                                        .map(|tab| Tab::new().text(*tab).into_widget()),
                                ),
                                Some(
                                    TabBarView::new(current.iter().map(|tab| {
                                        Tab::new().text(format!("Tab child {tab}")).into_widget()
                                    }))
                                    .into_widget(),
                                ),
                            ),
                        )
                        .into_widget()
                    })
                    .into_widget(),
                ));

                let get_controller =
                    |tester: &WidgetTester| default_controller_of(tester, text(tester, "B"));
                let controller = get_controller(&tester);

                controller.animate_to(
                    &mut tester.cell.borrow_mut(),
                    2,
                    Some(Duration::from_millis(200)),
                    Some(Curves::linear()),
                );
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(300));

                let controller = get_controller(&tester);
                {
                    let app = tester.cell.borrow();
                    assert_eq!(controller.index(&app), 2);
                    assert_eq!(controller.animation(&app).expect("live").value(&app), 2.0);
                }

                let remove = text(&tester, "Remove Last Tab");
                tap(&mut tester, remove);
                tester.pump_and_settle();

                let controller = get_controller(&tester);
                let app = tester.cell.borrow();
                assert_eq!(controller.index(&app), 1);
                assert_eq!(controller.animation(&app).expect("live").value(&app), 1.0);
            });
        }

        // 'Tab preferredSize gives correct value'.
        #[test]
        fn a_tab_prefers_46_high_or_72_with_an_icon_and_text() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    Material::new()
                        .child(
                            Row::new().children([
                                Tab::new()
                                    .icon(waft_widgets::Icon::new(Some(Icons::message())))
                                    .into_widget(),
                                Tab::new().text("Two").into_widget(),
                                Tab::new()
                                    .text("Three")
                                    .icon(waft_widgets::Icon::new(Some(Icons::chat())))
                                    .into_widget(),
                            ]),
                        )
                        .into_widget(),
                ));

                let preferred = |element: AnyElement| {
                    tester.widget(element, |tab: &Tab| tab.preferred_size().size())
                };
                assert_eq!(
                    preferred(tab_with_icon(&tester, &Icons::message())),
                    Size::from_height(46.0)
                );
                assert_eq!(
                    preferred(nearest_with_text::<Tab>(&tester, "Two")),
                    Size::from_height(46.0)
                );
                assert_eq!(
                    preferred(nearest_with_text::<Tab>(&tester, "Three")),
                    Size::from_height(72.0)
                );
            });
        }

        /// The `TabBar` above the tab labelled `label`: Dart's
        /// `tester.widget<TabBar>(find.widgetWithText(TabBar, label)).preferredSize`.
        fn tab_bar_preferred_size(tester: &WidgetTester, label: &str) -> Size {
            tester.widget(
                nearest_with_text::<TabBar>(tester, label),
                |tab_bar: &TabBar| tab_bar.preferred_size().size(),
            )
        }

        // 'TabBar preferredSize gives correct value when there are both icon and text in tabs'.
        #[test]
        fn a_tab_bar_with_an_icon_and_text_tab_prefers_74_high() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    DefaultTabController::new(
                        5,
                        scaffold_with_tab_bar(
                            AppBar::new().title(Text::new("Tabs Test")),
                            TabBar::new([
                                Tab::new().text("car").into_widget(),
                                Tab::new().text("transit").into_widget(),
                                Tab::new().text("bike").into_widget(),
                                Tab::new()
                                    .text("boat")
                                    .icon(waft_widgets::Icon::new(Some(Icons::message())))
                                    .into_widget(),
                                Tab::new().text("bus").into_widget(),
                            ]),
                            None,
                        ),
                    )
                    .into_widget(),
                ));

                assert_eq!(
                    tab_bar_preferred_size(&tester, "car"),
                    Size::from_height(74.0)
                );
            });
        }

        // 'TabBar preferredSize gives correct value when there is only icon or text in tabs'.
        #[test]
        fn a_tab_bar_of_icon_or_text_tabs_prefers_48_high() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    DefaultTabController::new(
                        5,
                        scaffold_with_tab_bar(
                            AppBar::new().title(Text::new("Tabs Test")),
                            TabBar::new([
                                Tab::new().text("car").into_widget(),
                                Tab::new()
                                    .icon(waft_widgets::Icon::new(Some(Icons::message())))
                                    .into_widget(),
                                Tab::new().text("bike").into_widget(),
                                Tab::new()
                                    .icon(waft_widgets::Icon::new(Some(Icons::chat())))
                                    .into_widget(),
                                Tab::new().text("bus").into_widget(),
                            ]),
                            None,
                        ),
                    )
                    .into_widget(),
                ));

                assert_eq!(
                    tab_bar_preferred_size(&tester, "car"),
                    Size::from_height(48.0)
                );
            });
        }

        /// The three uniform padding cases: a tab bar of an icon and text tab and two text tabs in
        /// an app bar, with `label_padding` when given, under `theme`; answers the label padding of
        /// each tab, the nearest `Padding` above its text.
        fn uniform_paddings(
            tester: &mut WidgetTester,
            theme: ThemeData,
            label_padding: Option<EdgeInsetsGeometry>,
        ) -> [EdgeInsetsGeometry; 3] {
            let controller = create_tab_controller(tester, 3);
            let mut tab_bar = TabBar::new([
                Tab::new()
                    .text("Tab 1")
                    .icon(waft_widgets::Icon::new(Some(Icons::plus_one())))
                    .into_widget(),
                Tab::new().text("Tab 2").into_widget(),
                Tab::new().text("Tab 3").into_widget(),
            ])
            .controller(controller);
            if let Some(label_padding) = label_padding {
                tab_bar = tab_bar.label_padding(label_padding);
            }
            tester.pump_widget(material_app(
                theme,
                scaffold_with_tab_bar(AppBar::new(), tab_bar, None),
            ));
            ["Tab 1", "Tab 2", "Tab 3"].map(|label| {
                tester.widget(
                    nearest_with_text::<Padding>(tester, label),
                    |padding: &Padding| padding.padding,
                )
            })
        }

        // 'Tabs are given uniform padding in case of few tabs having both text and icon'.
        #[test]
        fn text_tabs_beside_an_icon_and_text_tab_get_vertical_padding() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let expected_padding_adjusted = EdgeInsetsGeometry::symmetric(13.0, 16.0);
                let expected_padding_default = EdgeInsetsGeometry::symmetric(0.0, 16.0);

                let theme = theme(&tester, None, None);
                let [tab_one, tab_two, tab_three] = uniform_paddings(&mut tester, theme, None);

                assert_eq!(tab_one, expected_padding_default);
                assert_eq!(tab_two, expected_padding_adjusted);
                assert_eq!(tab_three, expected_padding_adjusted);
            });
        }

        // 'Tabs are given uniform padding when labelPadding is given'.
        #[test]
        fn text_tabs_beside_an_icon_and_text_tab_add_vertical_padding_to_the_label_padding() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let label_padding = EdgeInsetsGeometry::symmetric(10.0, 20.0);
                let expected_padding_adjusted = EdgeInsetsGeometry::symmetric(23.0, 20.0);
                let expected_padding_default = EdgeInsetsGeometry::symmetric(10.0, 20.0);

                let theme = theme(&tester, None, None);
                let [tab_one, tab_two, tab_three] =
                    uniform_paddings(&mut tester, theme, Some(label_padding));

                assert_eq!(tab_one, expected_padding_default);
                assert_eq!(tab_two, expected_padding_adjusted);
                assert_eq!(tab_three, expected_padding_adjusted);
            });
        }

        // 'Tabs are given uniform padding TabBarTheme.labelPadding is given'.
        #[test]
        fn text_tabs_beside_an_icon_and_text_tab_add_vertical_padding_to_the_theme_label_padding() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let label_padding = EdgeInsetsGeometry::symmetric(15.0, 20.0);
                let expected_padding_adjusted = EdgeInsetsGeometry::symmetric(28.0, 20.0);
                let expected_padding_default = EdgeInsetsGeometry::symmetric(15.0, 20.0);

                let theme = theme(
                    &tester,
                    None,
                    Some(TabBarThemeData::new().label_padding(label_padding)),
                );
                let [tab_one, tab_two, tab_three] = uniform_paddings(&mut tester, theme, None);

                assert_eq!(tab_one, expected_padding_default);
                assert_eq!(tab_two, expected_padding_adjusted);
                assert_eq!(tab_three, expected_padding_adjusted);
            });
        }

        // 'Change tab bar height'.
        #[test]
        fn every_kind_of_tab_takes_the_height_it_is_given() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, Some(false), None);
                let style = TextStyle::new().font_size(25.0);
                tester.pump_widget(material_app(
                    theme,
                    DefaultTabController::new(
                        4,
                        scaffold_with_tab_bar(
                            AppBar::new(),
                            TabBar::new([
                                Tab::new()
                                    .icon(waft_widgets::Icon::new(Some(Icons::check())).size(40.0))
                                    .height(85.0)
                                    .child(Text::new("1 - OK").style(style.clone()))
                                    .into_widget(), // icon and child
                                Tab::new()
                                    .height(85.0)
                                    .child(Text::new("2 - OK").style(style.clone()))
                                    .into_widget(), // child
                                Tab::new()
                                    .icon(waft_widgets::Icon::new(Some(Icons::done())).size(40.0))
                                    .height(85.0)
                                    .into_widget(), // icon
                                Tab::new().text("4 - OK").height(85.0).into_widget(), // text
                            ]),
                            None,
                        ),
                    )
                    .into_widget(),
                ));
                let first_tab = tab_with_icon(&tester, &Icons::check());
                let sec_tab = nearest_with_text::<Tab>(&tester, "2 - OK");
                let third_tab = tab_with_icon(&tester, &Icons::done());
                let fourth_tab = nearest_with_text::<Tab>(&tester, "4 - OK");
                assert_eq!(
                    tester.widget(first_tab, |tab: &Tab| tab.preferred_size().size().height()),
                    85.0
                );
                for tab in [first_tab, sec_tab, third_tab, fourth_tab] {
                    assert_eq!(tester.widget(tab, |tab: &Tab| tab.height), Some(85.0));
                }
            });
        }

        // 'Change tab bar height 2'.
        #[test]
        fn an_icon_and_text_tab_takes_the_height_it_is_given() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    DefaultTabController::new(
                        1,
                        scaffold_with_tab_bar(
                            AppBar::new(),
                            TabBar::new([Tab::new()
                                .icon(waft_widgets::Icon::new(Some(Icons::check())).size(40.0))
                                .text("1 - OK")
                                .height(85.0)
                                .into_widget()]), // icon and text
                            None,
                        ),
                    )
                    .into_widget(),
                ));
                let first_tab = tab_with_icon(&tester, &Icons::check());
                assert_eq!(tester.widget(first_tab, |tab: &Tab| tab.height), Some(85.0));
            });
        }

        /// The two controller-swap cases' `buildFrame`: a column of a tab bar and a tab bar view of
        /// `controller`, with a third tab when `show_last`.
        fn controller_swap_frame(
            tester: &WidgetTester,
            controller: Ref<TabController>,
            show_last: bool,
        ) -> WidgetRef {
            let mut tabs = vec![
                Tab::new().text("one").into_widget(),
                Tab::new().text("two").into_widget(),
            ];
            let mut pages = vec![
                Text::new("PAGE1").into_widget(),
                Text::new("PAGE2").into_widget(),
            ];
            if show_last {
                tabs.push(Tab::new().text("three").into_widget());
                pages.push(Text::new("PAGE3").into_widget());
            }
            boilerplate(
                theme(tester, None, None),
                TextDirection::Ltr,
                Column::new().children([
                    TabBar::new(tabs)
                        .controller(controller.clone())
                        .into_widget(),
                    waft_widgets::Flexible::new(TabBarView::new(pages).controller(controller))
                        .into_widget(),
                ]),
            )
        }

        // This is a regression test for https://github.com/flutter/flutter/issues/93237
        // 'Change the TabController should make both TabBar and TabBarView return to the initial
        // index.'
        #[test]
        fn a_new_controller_returns_the_tab_bar_and_its_view_to_the_new_index() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller1 = create_tab_controller(&tester, 3);
                let controller2 = create_tab_controller(&tester, 2);
                let controller3 = create_tab_controller(&tester, 3);

                let frame = controller_swap_frame(&tester, controller1.clone(), true);
                tester.pump_widget(frame);
                let page_controller = page_controller(&tester);
                let three = text(&tester, "three");
                tap(&mut tester, three);
                tester.pump_and_settle();
                {
                    let app = tester.cell.borrow();
                    assert_eq!(controller1.index(&app), 2);
                    assert_eq!(page_controller.page(&app), Some(2.0));
                }

                // Change TabController from 3 items to 2.
                let frame = controller_swap_frame(&tester, controller2.clone(), false);
                tester.pump_widget(frame);
                tester.pump_and_settle();
                {
                    let app = tester.cell.borrow();
                    assert_eq!(controller2.index(&app), 0);
                    assert_eq!(page_controller.page(&app), Some(0.0));
                }

                // Change TabController from 2 items to 3.
                let frame = controller_swap_frame(&tester, controller3.clone(), true);
                tester.pump_widget(frame);
                tester.pump_and_settle();
                {
                    let app = tester.cell.borrow();
                    assert_eq!(controller3.index(&app), 0);
                    assert_eq!(page_controller.page(&app), Some(0.0));
                }

                let three = text(&tester, "three");
                tap(&mut tester, three);
                tester.pump_and_settle();

                let app = tester.cell.borrow();
                assert_eq!(controller3.index(&app), 2);
                assert_eq!(page_controller.page(&app), Some(2.0));
            });
        }

        // This is a regression test for https://github.com/flutter/flutter/issues/97441
        // 'Do not crash when the new TabController.index is longer than the old length.'
        #[test]
        fn going_back_to_a_controller_whose_index_the_other_lacked_does_not_panic() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller1 = create_tab_controller(&tester, 3);
                let controller2 = create_tab_controller(&tester, 2);

                let frame = controller_swap_frame(&tester, controller1.clone(), true);
                tester.pump_widget(frame);
                let three = text(&tester, "three");
                tap(&mut tester, three);
                tester.pump_and_settle();
                {
                    let page_controller = page_controller(&tester);
                    let app = tester.cell.borrow();
                    assert_eq!(controller1.index(&app), 2);
                    assert_eq!(page_controller.page(&app), Some(2.0));
                }

                // Change TabController from controller1 to controller2.
                let frame = controller_swap_frame(&tester, controller2.clone(), false);
                tester.pump_widget(frame);
                tester.pump_and_settle();
                {
                    let page_controller = page_controller(&tester);
                    let app = tester.cell.borrow();
                    assert_eq!(controller2.index(&app), 0);
                    assert_eq!(page_controller.page(&app), Some(0.0));
                }

                // Change TabController back to 'controller1' whose index is 2.
                let frame = controller_swap_frame(&tester, controller1.clone(), true);
                tester.pump_widget(frame);
                tester.pump_and_settle();
                let page_controller = page_controller(&tester);
                let app = tester.cell.borrow();
                assert_eq!(controller1.index(&app), 2);
                assert_eq!(page_controller.page(&app), Some(2.0));
            });
        }

        /// The splash factory and overlay colour of the only `InkWell`.
        fn ink_well_splash_and_overlay(
            tester: &WidgetTester,
        ) -> (
            Option<InteractiveInkFeatureFactoryRef>,
            Option<WidgetStatePropertyRef<Option<Color>>>,
        ) {
            tester.widget(only::<InkWell>(tester), |ink_well: &InkWell| {
                (
                    ink_well.ink_response.splash_factory.clone(),
                    ink_well.ink_response.overlay_color.clone(),
                )
            })
        }

        // 'TabBar InkWell splashFactory and overlayColor'.
        #[test]
        fn the_tab_ink_wells_take_the_splash_factory_and_overlay_color() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let splash_factory = crate::no_splash::NoSplash::splash_factory();
                let overlay_color: WidgetStatePropertyRef<Option<Color>> =
                    <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|_states| {
                        Some(Colors::TRANSPARENT)
                    });
                let green_tab = || {
                    waft_widgets::Container::new()
                        .width(100.0)
                        .height(100.0)
                        .color(Colors::GREEN.primary())
                        .into_widget()
                };

                // TabBarTheme splashFactory and overlayColor
                let theme_with_tab_bar = theme(
                    &tester,
                    None,
                    Some(
                        TabBarThemeData::new()
                            .splash_factory(Rc::clone(&splash_factory))
                            .overlay_color(Rc::clone(&overlay_color)),
                    ),
                );
                tester.pump_widget(material_app(
                    theme_with_tab_bar,
                    DefaultTabController::new(
                        1,
                        scaffold_with_tab_bar(AppBar::new(), TabBar::new([green_tab()]), None),
                    )
                    .into_widget(),
                ));

                let (splash, overlay) = ink_well_splash_and_overlay(&tester);
                assert!(Rc::ptr_eq(
                    &splash.expect("a splash factory"),
                    &splash_factory
                ));
                assert!(Rc::ptr_eq(
                    &overlay.expect("an overlay color"),
                    &overlay_color
                ));

                // TabBar splashFactory and overlayColor
                let default_theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    default_theme,
                    DefaultTabController::new(
                        1,
                        scaffold_with_tab_bar(
                            AppBar::new(),
                            TabBar::new([green_tab()])
                                .splash_factory(Rc::clone(&splash_factory))
                                .overlay_color(Rc::clone(&overlay_color)),
                            None,
                        ),
                    )
                    .into_widget(),
                ));
                tester.pump_and_settle(); // theme animation
                let (splash, overlay) = ink_well_splash_and_overlay(&tester);
                assert!(Rc::ptr_eq(
                    &splash.expect("a splash factory"),
                    &splash_factory
                ));
                assert!(Rc::ptr_eq(
                    &overlay.expect("an overlay color"),
                    &overlay_color
                ));
            });
        }

        // 'splashBorderRadius is passed to InkWell.borderRadius'.
        #[test]
        fn the_splash_border_radius_rounds_the_tab_hover_highlight() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const HOVER_COLOR: Color = Color::new(0xfff44336);
                const RADIUS: f64 = 20.0;
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    DefaultTabController::new(
                        1,
                        TabBar::new([Tab::new().child(Text::new("")).into_widget()])
                            .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                                |states| {
                                    if states.contains(&WidgetState::Hovered) {
                                        return Some(HOVER_COLOR);
                                    }
                                    Some(Colors::BLACK54)
                                },
                            ))
                            .splash_border_radius(BorderRadius::all(
                                waft_embedder::Radius::circular(RADIUS),
                            )),
                    ),
                ));
                tester.pump_and_settle();
                let mut gesture = WidgetTester::mouse(1, 0);
                let center = center_of(&tester, only::<Tab>(&tester));
                tester.add_pointer(&mut gesture, Offset::ZERO);
                tester.move_to(&mut gesture, center, false);
                tester.pump_and_settle();
                let object = tester.first::<crate::material::RenderInkFeatures>();
                let ink_well_rect = rect_of(&tester, only::<InkWell>(&tester));
                assert!(tester.paints_rrect(
                    object.as_object(),
                    waft_embedder::RRect::from_rect_and_radius(
                        ink_well_rect,
                        waft_embedder::Radius::circular(RADIUS),
                    ),
                    Some(HOVER_COLOR),
                ));
            });
        }

        // Regression test for https://github.com/flutter/flutter/issues/154484.
        // 'No crash if TabBar build called before didUpdateWidget with SliverAppBar'.
        #[test]
        fn a_sliver_app_bar_tab_bar_gaining_its_first_tab_does_not_panic() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let tabs: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    StatefulBuilder::new(move |_app, _context, set_state| {
                        let current = tabs.borrow().clone();
                        let added = Rc::clone(&tabs);
                        let add = TextButton::new(
                            Some(Listener::new(move |app| {
                                let added = Rc::clone(&added);
                                set_state(app, &mut move |_app| {
                                    let next = format!("Tab {}", added.borrow().len() + 1);
                                    added.borrow_mut().push(next);
                                });
                            })),
                            Some(Text::new("Add Tab")),
                        );
                        DefaultTabController::new(
                            current.len(),
                            Scaffold::new().body(
                                waft_widgets::CustomScrollView::new().slivers([
                                    crate::app_bar::SliverAppBar::new()
                                        .actions(vec![add.into_widget()])
                                        .bottom(PreferredSizeWidgetRef::new(TabBar::new(
                                            current.iter().map(|tab| {
                                                Tab::new().text(tab.clone()).into_widget()
                                            }),
                                        )))
                                        .into_widget(),
                                ]),
                            ),
                        )
                        .into_widget()
                    })
                    .into_widget(),
                ));

                // Initializes with zero tabs.
                assert!(tester.find_text("Tab 1").is_empty());
                assert!(tester.find_text("Tab 2").is_empty());

                // No crash after tabs added.
                let add = text(&tester, "Add Tab");
                let added = catch_unwind(AssertUnwindSafe(|| {
                    tap(&mut tester, add);
                    tester.pump_and_settle();
                }));
                assert!(added.is_ok());
                assert_eq!(tester.find_text("Tab 1").len(), 1);
                assert!(tester.find_text("Tab 2").is_empty());
            });
        }

        // Regression test for https://github.com/flutter/flutter/issues/104994.
        // 'Do not crash if the controller and TabBarView are updated at different phases(build and
        // layout) of the same frame'.
        #[test]
        fn a_tab_bar_view_built_in_layout_follows_a_controller_built_earlier_in_the_frame() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let add_key = GlobalKey::new();
                let delete_key = GlobalKey::new();
                let app = zero_length_app(&tester, add_key.clone(), delete_key.clone(), true);
                tester.pump_widget(app);

                // Initializes with zero tabs properly
                assert_eq!(tester.find_text("No tabs").len(), 1);
                tap_key(&mut tester, &add_key);
                tester.pump_and_settle();
                assert_eq!(tester.find_text("Tab 1").len(), 1);
                assert_eq!(tester.find_text("Tab 1's view").len(), 1);

                // Dynamically updates to zero tabs properly
                tap_key(&mut tester, &delete_key);
                tester.pump_and_settle();
                assert_eq!(tester.find_text("No tabs").len(), 1);
            });
        }

        /// A green 100 square, the tab of the two length-mismatch cases.
        fn green_square() -> WidgetRef {
            waft_widgets::Container::new()
                .width(100.0)
                .height(100.0)
                .color(Colors::GREEN.primary())
                .into_widget()
        }

        // "Throw if the controller's length mismatch the tabs count".
        #[test]
        fn a_controller_longer_than_the_tabs_panics() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                let exception = pump_and_take_exception(
                    &mut tester,
                    material_app(
                        theme,
                        DefaultTabController::new(
                            2,
                            scaffold_with_tab_bar(
                                AppBar::new(),
                                TabBar::new([green_square()]),
                                None,
                            ),
                        )
                        .into_widget(),
                    ),
                );
                let exception = exception.expect("the length check fails");
                assert!(
                    exception.contains("does not match the number of tabs"),
                    "{exception}"
                );
            });
        }

        // "Throw if the controller's length mismatch the TabBarView‘s children count".
        #[test]
        fn a_tab_bar_view_with_more_children_than_the_controller_panics() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                let icon = |icon: waft_widgets::IconData| {
                    waft_widgets::Icon::new(Some(icon)).into_widget()
                };
                let exception = pump_and_take_exception(
                    &mut tester,
                    material_app(
                        theme,
                        DefaultTabController::new(
                            1,
                            scaffold_with_tab_bar(
                                AppBar::new(),
                                TabBar::new([green_square()]),
                                Some(
                                    TabBarView::new([
                                        icon(Icons::directions_car()),
                                        icon(Icons::directions_transit()),
                                        icon(Icons::directions_bike()),
                                    ])
                                    .into_widget(),
                                ),
                            ),
                        )
                        .into_widget(),
                    ),
                );
                let exception = exception.expect("the length check fails");
                assert!(
                    exception.contains("does not match the number of children"),
                    "{exception}"
                );
            });
        }

        /// Whether `drawn` has a rect of `color`: Dart's `paints..rect(color: color)`.
        fn paints_rect_of(drawn: &[crate::test_support::Drawn], color: Color) -> bool {
            drawn.iter().any(|drawn| {
                matches!(drawn, crate::test_support::Drawn::Rect(argb, _)
                    if crate::test_support::is_same_color_as(Color::new(*argb), color))
            })
        }

        /// What the first ink-features render object and its subtree paint.
        fn ink_features_painted(tester: &WidgetTester) -> Vec<crate::test_support::Drawn> {
            let ink_features = tester.first::<crate::material::RenderInkFeatures>();
            tester.painted(ink_features.as_object())
        }

        #[allow(deprecated)]
        fn with_opacity(color: Color, opacity: f64) -> Color {
            color.with_opacity(opacity)
        }

        // 'Tab has correct selected/unselected hover color'.
        #[test]
        fn an_unselected_and_a_selected_tab_hover_in_different_colors() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                tester.set_highlight_strategy(
                    waft_widgets::FocusHighlightStrategy::AlwaysTraditional,
                );
                let theme = theme(&tester, None, None);
                let mut frame = Frame::new(vec!["A", "B", "C"], "C");
                frame.use_material3 = Some(theme.use_material3);
                let frame = frame.build(&tester);
                tester.pump_widget(frame);

                tester.pump_and_settle();
                let unselected = with_opacity(theme.color_scheme.on_surface, 0.08);
                let selected = with_opacity(theme.color_scheme.primary, 0.08);
                assert!(!paints_rect_of(&ink_features_painted(&tester), unselected));
                assert!(!paints_rect_of(&ink_features_painted(&tester), selected));

                // Start hovering unselected tab.
                let mut gesture = WidgetTester::mouse(1, 0);
                tester.add_pointer(&mut gesture, Offset::ZERO);
                let first = tester.find_by_type::<Tab>()[0];
                let first_center = center_of(&tester, first);
                tester.move_to(&mut gesture, first_center, false);
                tester.pump_and_settle();
                assert!(paints_rect_of(&ink_features_painted(&tester), unselected));

                // Start hovering selected tab.
                let last = *tester.find_by_type::<Tab>().last().expect("tabs");
                let last_center = center_of(&tester, last);
                tester.move_to(&mut gesture, last_center, false);
                tester.pump_and_settle();
                assert!(paints_rect_of(&ink_features_painted(&tester), selected));
            });
        }

        /// Whether the primary focus is a node that has it: Dart's
        /// `tester.binding.focusManager.primaryFocus?.hasPrimaryFocus`.
        fn primary_focus_has_primary_focus(tester: &WidgetTester) -> bool {
            let mut app = tester.cell.borrow_mut();
            let manager = waft_widgets::FocusManager::instance(&mut app);
            manager
                .primary_focus(&app)
                .is_some_and(|node| node.has_primary_focus(&app))
        }

        // 'Tab has correct selected/unselected focus color'.
        #[test]
        fn an_unselected_and_a_selected_tab_focus_in_different_colors() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                tester.set_highlight_strategy(
                    waft_widgets::FocusHighlightStrategy::AlwaysTraditional,
                );
                let theme = theme(&tester, None, None);
                let mut frame = Frame::new(vec!["A", "B", "C"], "B");
                frame.use_material3 = Some(theme.use_material3);
                let frame = frame.build(&tester);
                tester.pump_widget(material_app(theme.clone(), frame));

                tester.pump_and_settle();
                let unselected = with_opacity(theme.color_scheme.on_surface, 0.1);
                let selected = with_opacity(theme.color_scheme.primary, 0.1);
                assert!(!paints_rect_of(&ink_features_painted(&tester), unselected));
                assert!(!paints_rect_of(&ink_features_painted(&tester), selected));

                tester.send_key_event(
                    waft_services::PhysicalKeyboardKey::TAB,
                    waft_services::LogicalKeyboardKey::TAB,
                );
                tester.pump_and_settle();
                assert!(primary_focus_has_primary_focus(&tester));
                assert!(paints_rect_of(&ink_features_painted(&tester), unselected));

                tester.send_key_event(
                    waft_services::PhysicalKeyboardKey::TAB,
                    waft_services::LogicalKeyboardKey::TAB,
                );
                tester.pump_and_settle();
                assert!(primary_focus_has_primary_focus(&tester));
                assert!(paints_rect_of(&ink_features_painted(&tester), selected));
            });
        }

        // 'Tab has correct selected/unselected pressed color'.
        #[test]
        fn pressing_an_unselected_or_a_selected_tab_highlights_in_the_primary_color() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                let mut frame = Frame::new(vec!["A", "B", "C"], "B");
                frame.use_material3 = Some(theme.use_material3);
                let frame = frame.build(&tester);
                tester.pump_widget(material_app(theme.clone(), frame));

                tester.pump_and_settle();
                let pressed = with_opacity(theme.color_scheme.primary, 0.1);
                assert!(!paints_rect_of(&ink_features_painted(&tester), pressed));

                // Press unselected tab.
                let mut gesture = WidgetTester::touch(1);
                let a = center_of(&tester, text(&tester, "A"));
                tester.down(&mut gesture, a);
                tester.pump_and_settle(); // Let the press highlight animation finish.
                assert!(paints_rect_of(&ink_features_painted(&tester), pressed));

                // Release pressed gesture.
                tester.up(&gesture);
                tester.pump_and_settle();

                // Press selected tab.
                let mut gesture = WidgetTester::touch(2);
                let b = center_of(&tester, text(&tester, "B"));
                tester.down(&mut gesture, b);
                tester.pump_and_settle(); // Let the press highlight animation finish.
                assert!(paints_rect_of(&ink_features_painted(&tester), pressed));
            });
        }

        /// Dart's `moreOrLessEquals` epsilon, 1e-10, at the precision of the f32 transforms
        /// `local_to_global` goes through: a tab's global left comes back within 1e-6 of Dart's.
        const F32_EPSILON: f64 = 1e-4;

        /// Dart's `expect(a, equals(b))` on a global coordinate, to the precision of
        /// [`F32_EPSILON`].
        fn assert_f32_eq(a: f64, b: f64) {
            assert!(close(a, b, F32_EPSILON), "{a} != {b}");
        }

        /// The rects of the first and the last `Tab`.
        fn first_and_last_tab_rects(tester: &WidgetTester) -> (Rect, Rect) {
            let tabs = tester.find_by_type::<Tab>();
            (
                rect_of(tester, tabs[0]),
                rect_of(tester, *tabs.last().expect("tabs")),
            )
        }

        // 'Material3 - Default TabAlignment'.
        #[test]
        fn material3_tabs_fill_a_fixed_tab_bar_and_start_offset_in_a_scrollable_one() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const TAB_START_OFFSET: f64 = 52.0;
                let padding = K_TAB_LABEL_PADDING;

                // Test default TabAlignment when isScrollable is false.
                let frame = Frame::new(vec!["A", "B"], "B").build(&tester);
                tester.pump_widget(frame);

                let tab_bar = rect_of(&tester, only::<TabBar>(&tester));
                let (tab_one_rect, tab_two_rect) = first_and_last_tab_rects(&tester);

                // Tabs should fill the width of the TabBar.
                let tab_one_left = ((tab_bar.width() / 2.0) - tab_one_rect.width()) / 2.0;
                assert!(
                    close(tab_one_rect.left, tab_one_left, F32_EPSILON),
                    "{} {}",
                    tab_one_rect.left,
                    tab_one_left
                );
                let tab_two_right =
                    tab_bar.width() - ((tab_bar.width() / 2.0) - tab_two_rect.width()) / 2.0;
                assert!(
                    close(tab_two_rect.right, tab_two_right, F32_EPSILON),
                    "{} {}",
                    tab_two_rect.right,
                    tab_two_right
                );

                // Test default TabAlignment when isScrollable is true.
                let mut frame = Frame::new(vec!["A", "B"], "B");
                frame.is_scrollable = true;
                let frame = frame.build(&tester);
                tester.pump_widget(frame);

                let (tab_one_rect, tab_two_rect) = first_and_last_tab_rects(&tester);

                // Tabs should be aligned to the start of the TabBar.
                let tab_one_left = padding.left + TAB_START_OFFSET;
                assert!(
                    close(tab_one_rect.left, tab_one_left, F32_EPSILON),
                    "{} {}",
                    tab_one_rect.left,
                    tab_one_left
                );
                let tab_two_right = padding.horizontal()
                    + TAB_START_OFFSET
                    + tab_one_rect.width()
                    + padding.left
                    + tab_two_rect.width();
                assert!(
                    close(tab_two_rect.right, tab_two_right, F32_EPSILON),
                    "{} {}",
                    tab_two_rect.right,
                    tab_two_right
                );
            });
        }

        /// A `buildFrame` of tabs A and B on B with `tab_alignment`, scrollable when `is_scrollable`,
        /// inside a `MaterialApp` of the default theme; answers the panic its frame raised.
        fn alignment_exception(
            tester: &mut WidgetTester,
            tab_alignment: TabAlignment,
            is_scrollable: bool,
        ) -> Option<String> {
            let theme = theme(tester, None, None);
            let mut frame = Frame::new(vec!["A", "B"], "B");
            frame.tab_alignment = Some(tab_alignment);
            frame.is_scrollable = is_scrollable;
            let frame = frame.build(tester);
            pump_and_take_exception(tester, material_app(theme, frame))
        }

        // 'TabAlignment.fill only supports non-scrollable tab bar'.
        #[test]
        fn fill_alignment_panics_on_a_scrollable_tab_bar() {
            with_main_thread_stack(|| {
                // Test TabAlignment.fill with non-scrollable tab bar.
                let mut tester = tester();
                assert_eq!(
                    alignment_exception(&mut tester, TabAlignment::Fill, false),
                    None
                );

                // Test TabAlignment.fill with scrollable tab bar.
                let mut tester = super::tester();
                let exception = alignment_exception(&mut tester, TabAlignment::Fill, true)
                    .expect("the alignment assert fails");
                assert!(
                    exception.contains("is only valid for non-scrollable tab bars"),
                    "{exception}"
                );
            });
        }

        // 'TabAlignment.start & TabAlignment.startOffset only supports scrollable tab bar'.
        #[test]
        fn start_alignments_panic_on_a_fixed_tab_bar() {
            with_main_thread_stack(|| {
                for alignment in [TabAlignment::Start, TabAlignment::StartOffset] {
                    // Test the alignment with scrollable tab bar.
                    let mut tester = tester();
                    assert_eq!(alignment_exception(&mut tester, alignment, true), None);

                    // Test the alignment with non-scrollable tab bar.
                    let mut tester = super::tester();
                    let exception = alignment_exception(&mut tester, alignment, false)
                        .expect("the alignment assert fails");
                    assert!(
                        exception.contains("is only valid for scrollable tab bars"),
                        "{exception}"
                    );
                }
            });
        }

        /// A `buildFrame` of tabs A and B on B.
        fn alignment_frame(
            tester: &WidgetTester,
            is_scrollable: bool,
            tab_alignment: Option<TabAlignment>,
            text_direction: TextDirection,
        ) -> WidgetRef {
            let mut frame = Frame::new(vec!["A", "B"], "B");
            frame.is_scrollable = is_scrollable;
            frame.tab_alignment = tab_alignment;
            frame.text_direction = text_direction;
            frame.build(tester)
        }

        // 'Material3 - TabAlignment updates tabs alignment (non-scrollable TabBar)'.
        #[test]
        fn material3_center_alignment_packs_the_tabs_of_a_fixed_tab_bar() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let padding = K_TAB_LABEL_PADDING;

                // Test TabAlignment.fill (default) when isScrollable is false.
                let frame = alignment_frame(&tester, false, None, TextDirection::Ltr);
                tester.pump_widget(frame);

                const AVAILABLE_WIDTH: f64 = 800.0;
                let (tab_one_rect, tab_two_rect) = first_and_last_tab_rects(&tester);

                // By defaults tabs should fill the width of the TabBar.
                let tab_one_left = ((AVAILABLE_WIDTH / 2.0) - tab_one_rect.width()) / 2.0;
                assert!(close(tab_one_rect.left, tab_one_left, F32_EPSILON));
                let tab_two_right =
                    AVAILABLE_WIDTH - ((AVAILABLE_WIDTH / 2.0) - tab_two_rect.width()) / 2.0;
                assert!(close(tab_two_rect.right, tab_two_right, F32_EPSILON));

                // Test TabAlignment.center when isScrollable is false.
                let frame = alignment_frame(
                    &tester,
                    false,
                    Some(TabAlignment::Center),
                    TextDirection::Ltr,
                );
                tester.pump_widget(frame);
                tester.pump_and_settle();

                let (tab_one_rect, tab_two_rect) = first_and_last_tab_rects(&tester);

                // Tabs should not fill the width of the TabBar.
                let tab_one_left = padding.left;
                assert!(close(tab_one_rect.left, tab_one_left, F32_EPSILON));
                let tab_two_right = padding.horizontal()
                    + tab_one_rect.width()
                    + padding.left
                    + tab_two_rect.width();
                assert!(close(tab_two_rect.right, tab_two_right, F32_EPSILON));
            });
        }

        // 'Material3 - TabAlignment updates tabs alignment (scrollable TabBar)'.
        #[test]
        fn material3_alignments_place_the_tabs_of_a_scrollable_tab_bar() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const TAB_START_OFFSET: f64 = 52.0;
                let padding = K_TAB_LABEL_PADDING;

                // Test TabAlignment.startOffset (default) when isScrollable is true.
                let frame = alignment_frame(&tester, true, None, TextDirection::Ltr);
                tester.pump_widget(frame);

                let tab_bar = rect_of(&tester, only::<TabBar>(&tester));
                let (tab_one_rect, tab_two_rect) = first_and_last_tab_rects(&tester);

                // By default tabs should be aligned to the start of the TabBar with
                // an horizontal offset of 52.0 pixels.
                let tab_one_left = padding.left + TAB_START_OFFSET;
                assert_f32_eq(tab_one_rect.left, tab_one_left);
                let tab_two_right = TAB_START_OFFSET
                    + padding.horizontal()
                    + tab_one_rect.width()
                    + padding.left
                    + tab_two_rect.width();
                assert_f32_eq(tab_two_rect.right, tab_two_right);

                // Test TabAlignment.start when isScrollable is true.
                let frame =
                    alignment_frame(&tester, true, Some(TabAlignment::Start), TextDirection::Ltr);
                tester.pump_widget(frame);
                tester.pump_and_settle();

                let (tab_one_rect, tab_two_rect) = first_and_last_tab_rects(&tester);

                // Tabs should be aligned to the start of the TabBar.
                let tab_one_left = padding.left;
                assert_f32_eq(tab_one_rect.left, tab_one_left);
                let tab_two_right = padding.horizontal()
                    + tab_one_rect.width()
                    + padding.left
                    + tab_two_rect.width();
                assert_f32_eq(tab_two_rect.right, tab_two_right);

                // Test TabAlignment.center when isScrollable is true.
                let frame = alignment_frame(
                    &tester,
                    true,
                    Some(TabAlignment::Center),
                    TextDirection::Ltr,
                );
                tester.pump_widget(frame);
                tester.pump_and_settle();

                let (tab_one_rect, tab_two_rect) = first_and_last_tab_rects(&tester);

                // Tabs should be centered in the TabBar.
                let tab_one_left = (tab_bar.width() / 2.0) - tab_one_rect.width() - padding.right;
                assert_f32_eq(tab_one_rect.left, tab_one_left);
                let tab_two_right = (tab_bar.width() / 2.0) + tab_two_rect.width() + padding.left;
                assert_f32_eq(tab_two_rect.right, tab_two_right);

                // Test TabAlignment.startOffset when isScrollable is true.
                let frame = alignment_frame(
                    &tester,
                    true,
                    Some(TabAlignment::StartOffset),
                    TextDirection::Ltr,
                );
                tester.pump_widget(frame);
                tester.pump_and_settle();

                let (tab_one_rect, tab_two_rect) = first_and_last_tab_rects(&tester);

                // Tabs should be aligned to the start of the TabBar with an
                // horizontal offset of 52.0 pixels.
                let tab_one_left = padding.left + TAB_START_OFFSET;
                assert_f32_eq(tab_one_rect.left, tab_one_left);
                let tab_two_right = TAB_START_OFFSET
                    + padding.horizontal()
                    + tab_one_rect.width()
                    + padding.left
                    + tab_two_rect.width();
                assert_f32_eq(tab_two_rect.right, tab_two_right);
            });
        }

        // 'Material3 - TabAlignment.start & TabAlignment.startOffset respects TextDirection.rtl'.
        #[test]
        fn material3_start_alignments_start_from_the_right_in_rtl() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const TAB_START_OFFSET: f64 = 52.0;
                let padding = K_TAB_LABEL_PADDING;

                // Test TabAlignment.startOffset (default) when isScrollable is true.
                let frame = alignment_frame(&tester, true, None, TextDirection::Rtl);
                tester.pump_widget(frame);

                let tab_bar = rect_of(&tester, only::<TabBar>(&tester));
                let (tab_one_rect, tab_two_rect) = first_and_last_tab_rects(&tester);

                // Tabs should be aligned to the start of the TabBar with an
                // horizontal offset of 52.0 pixels.
                let tab_one_right = tab_bar.width() - padding.right - TAB_START_OFFSET;
                assert_f32_eq(tab_one_rect.right, tab_one_right);
                let tab_two_left = tab_bar.width()
                    - TAB_START_OFFSET
                    - padding.horizontal()
                    - tab_one_rect.width()
                    - padding.right
                    - tab_two_rect.width();
                assert_f32_eq(tab_two_rect.left, tab_two_left);

                // Test TabAlignment.start when isScrollable is true.
                let frame =
                    alignment_frame(&tester, true, Some(TabAlignment::Start), TextDirection::Rtl);
                tester.pump_widget(frame);
                tester.pump_and_settle();

                let (tab_one_rect, tab_two_rect) = first_and_last_tab_rects(&tester);

                // Tabs should be aligned to the start of the TabBar.
                let tab_one_right = tab_bar.width() - padding.right;
                assert_f32_eq(tab_one_rect.right, tab_one_right);
                let tab_two_left = tab_bar.width()
                    - padding.horizontal()
                    - tab_one_rect.width()
                    - padding.left
                    - tab_two_rect.width();
                assert_f32_eq(tab_two_rect.left, tab_two_left);

                // Test TabAlignment.startOffset when isScrollable is true.
                let frame = alignment_frame(
                    &tester,
                    true,
                    Some(TabAlignment::StartOffset),
                    TextDirection::Rtl,
                );
                tester.pump_widget(frame);
                tester.pump_and_settle();

                let (tab_one_rect, tab_two_rect) = first_and_last_tab_rects(&tester);

                // Tabs should be aligned to the start of the TabBar with an
                // horizontal offset of 52.0 pixels.
                let tab_one_right = tab_bar.width() - padding.right - TAB_START_OFFSET;
                assert_f32_eq(tab_one_rect.right, tab_one_right);
                let tab_two_left = tab_bar.width()
                    - TAB_START_OFFSET
                    - padding.horizontal()
                    - tab_one_rect.width()
                    - padding.right
                    - tab_two_rect.width();
                assert_f32_eq(tab_two_rect.left, tab_two_left);
            });
        }

        // 'Material3 - TabBar inherits the dividerColor of TabBarTheme'.
        #[test]
        fn material3_the_indicator_painter_takes_the_theme_divider_color() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let divider_color = Colors::YELLOW.primary();
                let theme = theme(
                    &tester,
                    None,
                    Some(TabBarThemeData::new().divider_color(divider_color)),
                );
                let controller = create_tab_controller(&tester, 3);
                tester.pump_widget(material_app(
                    theme,
                    scaffold_with_tab_bar(
                        AppBar::new(),
                        TabBar::new(
                            ["Tab 1", "Tab 2", "Tab 3"]
                                .map(|text| Tab::new().text(text).into_widget()),
                        )
                        .controller(controller),
                        None,
                    ),
                ));

                // Test painter's divider color.
                let paint = *tester
                    .find_by_type::<CustomPaint>()
                    .last()
                    .expect("a CustomPaint");
                let painter_divider_color = tester.widget(paint, |paint: &CustomPaint| {
                    paint
                        .painter
                        .as_ref()
                        .and_then(|painter| painter.as_any().downcast_ref::<IndicatorPainter>())
                        .expect("the last CustomPaint is the indicator's")
                        .divider_color
                });
                assert_eq!(painter_divider_color, Some(divider_color));
            });
        }

        // This is a regression test for https://github.com/flutter/flutter/pull/125974#discussion_r1239089151.
        // 'Divider can be constrained'.
        #[test]
        fn the_divider_of_a_constrained_scrollable_tab_bar_is_as_wide_as_the_tab_bar() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let divider_color = Colors::YELLOW.primary();
                let theme = theme(
                    &tester,
                    None,
                    Some(TabBarThemeData::new().divider_color(divider_color)),
                );
                tester.pump_widget(material_app(
                    theme,
                    Scaffold::new()
                        .body(DefaultTabController::new(
                            2,
                            Center::new().child(
                                waft_widgets::ConstrainedBox::new(
                                    BoxConstraints::new().max_width(360.0),
                                )
                                .child(
                                    waft_widgets::ColoredBox::new(Colors::GREY.shade200()).child(
                                        TabBar::secondary([
                                            Tab::new().text("Test 1").into_widget(),
                                            Tab::new().text("Test 2").into_widget(),
                                        ])
                                        .tab_alignment(TabAlignment::Start)
                                        .is_scrollable(true),
                                    ),
                                ),
                            ),
                        ))
                        .into_widget(),
                ));

                // Test tab bar width.
                assert_eq!(size_of(&tester, only::<TabBar>(&tester)).width(), 360.0);
                // Test divider width.
                let divider = tester.find_by_type::<CustomPaint>()[1];
                assert_eq!(size_of(&tester, divider).width(), 360.0);
            });
        }

        // 'TabBar labels use colors from labelStyle & unselectedLabelStyle'.
        #[test]
        fn labels_take_their_colors_from_the_label_styles() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const TAB1: &str = "Tab 1";
                const TAB2: &str = "Tab 2";
                let label_style = TextStyle::new()
                    .color(Color::new(0xff0000ff))
                    .font_style(waft_embedder::FontStyle::Italic);
                let unselected_label_style = TextStyle::new()
                    .color(Color::new(0x950000ff))
                    .font_style(waft_embedder::FontStyle::Italic);

                // Test tab bar with labelStyle & unselectedLabelStyle.
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    DefaultTabController::new(
                        2,
                        TabBar::new([
                            Tab::new().text(TAB1).into_widget(),
                            Tab::new().text(TAB2).into_widget(),
                        ])
                        .label_style(label_style.clone())
                        .unselected_label_style(unselected_label_style.clone()),
                    ),
                ));

                let selected_text_style = paragraph_style(&tester, TAB1);
                let unselected_text_style = paragraph_style(&tester, TAB2);

                // Selected tab should use the labelStyle color.
                assert_eq!(icon_color(&tester, TAB1), style_color(&label_style));
                assert_eq!(style_color(&selected_text_style), style_color(&label_style));
                assert_eq!(selected_text_style.font_style, label_style.font_style);
                // Unselected tab should use the unselectedLabelStyle color.
                assert_eq!(
                    icon_color(&tester, TAB2),
                    style_color(&unselected_label_style)
                );
                assert_eq!(
                    style_color(&unselected_text_style),
                    style_color(&unselected_label_style)
                );
                assert_eq!(
                    unselected_text_style.font_style,
                    unselected_label_style.font_style
                );
            });
        }

        // 'labelColor & unselectedLabelColor override labelStyle & unselectedLabelStyle colors'.
        #[test]
        fn label_colors_override_the_label_style_colors() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const TAB1: &str = "Tab 1";
                const TAB2: &str = "Tab 2";
                const LABEL_COLOR: Color = Color::new(0xfff00000);
                const UNSELECTED_LABEL_COLOR: Color = Color::new(0x95ff0000);
                let label_style = TextStyle::new()
                    .color(Color::new(0xff0000ff))
                    .font_style(waft_embedder::FontStyle::Italic);
                let unselected_label_style = TextStyle::new()
                    .color(Color::new(0x950000ff))
                    .font_style(waft_embedder::FontStyle::Italic);

                let build_tab_bar = |tester: &WidgetTester, colors: Option<(Color, Color)>| {
                    let mut tab_bar = TabBar::new([
                        Tab::new().text(TAB1).into_widget(),
                        Tab::new().text(TAB2).into_widget(),
                    ])
                    .label_style(label_style.clone())
                    .unselected_label_style(unselected_label_style.clone());
                    if let Some((label_color, unselected_label_color)) = colors {
                        tab_bar = tab_bar
                            .label_color(label_color)
                            .unselected_label_color(unselected_label_color);
                    }
                    boilerplate(
                        theme(tester, None, None),
                        TextDirection::Ltr,
                        DefaultTabController::new(2, tab_bar),
                    )
                };

                // Test tab bar with labelStyle & unselectedLabelStyle.
                let frame = build_tab_bar(&tester, None);
                tester.pump_widget(frame);

                let selected_text_style = paragraph_style(&tester, TAB1);
                let unselected_text_style = paragraph_style(&tester, TAB2);

                // Selected tab should use labelStyle color.
                assert_eq!(icon_color(&tester, TAB1), style_color(&label_style));
                assert_eq!(style_color(&selected_text_style), style_color(&label_style));
                assert_eq!(selected_text_style.font_style, label_style.font_style);
                // Unselected tab should use unselectedLabelStyle color.
                assert_eq!(
                    icon_color(&tester, TAB2),
                    style_color(&unselected_label_style)
                );
                assert_eq!(
                    style_color(&unselected_text_style),
                    style_color(&unselected_label_style)
                );
                assert_eq!(
                    unselected_text_style.font_style,
                    unselected_label_style.font_style
                );

                // Update tab bar with labelColor & unselectedLabelColor.
                let frame = build_tab_bar(&tester, Some((LABEL_COLOR, UNSELECTED_LABEL_COLOR)));
                tester.pump_widget(frame);
                tester.pump_and_settle();

                let selected_text_style = paragraph_style(&tester, TAB1);
                let unselected_text_style = paragraph_style(&tester, TAB2);

                // Selected tab should use the labelColor.
                assert_eq!(icon_color(&tester, TAB1), Some(LABEL_COLOR));
                assert_eq!(style_color(&selected_text_style), Some(LABEL_COLOR));
                assert_eq!(selected_text_style.font_style, label_style.font_style);
                // Unselected tab should use the unselectedLabelColor.
                assert_eq!(icon_color(&tester, TAB2), Some(UNSELECTED_LABEL_COLOR));
                assert_eq!(
                    style_color(&unselected_text_style),
                    Some(UNSELECTED_LABEL_COLOR)
                );
                assert_eq!(
                    unselected_text_style.font_style,
                    unselected_label_style.font_style
                );
            });
        }

        // This is a regression test for https://github.com/flutter/flutter/issues/140338.
        // 'Material3 - Scrollable TabBar without a divider does not expand to full width'.
        #[test]
        fn material3_a_scrollable_tab_bar_without_a_divider_shrinks_to_its_tabs() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let build_tab_bar =
                    |tester: &WidgetTester,
                     divider_height: Option<f64>,
                     tab_alignment: TabAlignment| {
                        let mut tab_bar = TabBar::new(
                            ["Tab 1", "Tab 2", "Tab 3"]
                                .map(|text| Tab::new().text(text).into_widget()),
                        )
                        .tab_alignment(tab_alignment)
                        .is_scrollable(true);
                        if let Some(divider_height) = divider_height {
                            tab_bar = tab_bar.divider_height(divider_height);
                        }
                        boilerplate(
                            theme(tester, None, None),
                            TextDirection::Ltr,
                            Center::new().child(DefaultTabController::new(3, tab_bar)),
                        )
                    };
                let mut width_with = |divider_height: Option<f64>, tab_alignment: TabAlignment| {
                    let frame = build_tab_bar(&tester, divider_height, tab_alignment);
                    tester.pump_widget(frame);
                    size_of(&tester, only::<TabBar>(&tester)).width()
                };

                // Test default tab bar width when there is a divider and tabAlignment
                // is set to startOffset.
                assert_eq!(width_with(None, TabAlignment::Start), 800.0);

                // Test default tab bar width when there is a divider and tabAlignment
                // is set to start.
                assert_eq!(width_with(None, TabAlignment::StartOffset), 800.0);

                // Test default tab bar width when there is a divider and tabAlignment
                // tabAlignment is set to center.
                assert_eq!(width_with(None, TabAlignment::Center), 800.0);

                // Test default tab bar width when the divider height is set to 0.0
                // and tabAlignment is set to startOffset.
                assert_eq!(width_with(Some(0.0), TabAlignment::StartOffset), 359.5);

                // Test default tab bar width when the divider height is set to 0.0
                // and tabAlignment is set to start.
                assert_eq!(width_with(Some(0.0), TabAlignment::Start), 307.5);

                // Test default tab bar width when the divider height is set to 0.0
                // and tabAlignment is set to center.
                assert_eq!(width_with(Some(0.0), TabAlignment::Center), 307.5);
            });
        }
    }

    mod material2_and_indicator_animations {
        //! tabs_test.dart cases, part e: the 'Material 2' group to the end of the file.

        use std::cell::{Cell, RefCell};
        use waft_painting::Decoration;

        use waft_embedder::{RRect, Radius, TargetPlatform};
        use waft_painting::EdgeInsets;
        use waft_test::TestPlatform;
        use waft_widgets::{
            AnyElement, Focus, Icon, MediaQueryData, PreferredSizeWidgetRef, UnfocusDisposition,
        };

        use super::*;
        use crate::app_bar::AppBar;
        use crate::icons::Icons;
        use crate::scaffold::Scaffold;
        use crate::test_support::{TestVSync, WidgetTester, material_app, with_main_thread_stack};
        use crate::theme_data::ThemeData;

        /// Dart's `tester.renderObject<RenderParagraph>(find.text(text)).text.style!`.
        fn paragraph_style(tester: &WidgetTester, value: &str) -> TextStyle {
            let render = tester.render_box_of(text(tester, value));
            let app = tester.cell.borrow();
            let paragraph = render
                .as_object()
                .downcast::<waft_rendering::RenderParagraph>(&app)
                .expect("a Text renders a paragraph");
            paragraph
                .text(&app)
                .style()
                .cloned()
                .expect("the paragraph's text has a style")
        }

        fn paragraph_color(tester: &WidgetTester, value: &str) -> Option<Color> {
            paragraph_style(tester, value)
                .color
                .as_ref()
                .map(AnyColor::color)
        }

        /// Dart's `createTabController(length:, vsync: const TestVSync())`.
        fn controller(tester: &WidgetTester, length: usize) -> Ref<TabController> {
            let mut app = tester.cell.borrow_mut();
            TabController::new(&mut app, 0, None, length, TestVSync)
        }

        /// Dart's `ThemeData(useMaterial3: false)`.
        fn m2(tester: &WidgetTester) -> ThemeData {
            theme(tester, Some(false), None)
        }

        /// The tabs' elements, in order: Dart's `find.byType(Tab)`.
        fn tabs_of(tester: &WidgetTester) -> Vec<AnyElement> {
            tester.find_by_type::<Tab>()
        }

        /// Dart's `RRect.fromLTRBAndCorners(left, top, right, bottom, topLeft: 3, topRight: 3)`.
        fn indicator_rrect(left: f64, top: f64, right: f64, bottom: f64) -> RRect {
            RRect::from_ltrb_and_corners(
                left,
                top,
                right,
                bottom,
                Radius::circular(3.0),
                Radius::circular(3.0),
                Radius::ZERO,
                Radius::ZERO,
            )
        }

        /// The `CustomPaint` that paints the indicator's render object.
        ///
        /// Dart's `paints` matcher on the tab bar records through the layers a scrollable tab bar's
        /// viewport pushes; `WidgetTester::pictures` reads the tab bar's own pictures only, so the
        /// cases ask this `CustomPaint`, in whose coordinates Dart's rrects are.
        fn indicator_paint(tester: &WidgetTester) -> AnyRenderObject {
            let tab_bar = only::<TabBar>(tester);
            let indicator_paint = tester
                .descendants_of_type::<CustomPaint>(tab_bar)
                .into_iter()
                .find(|element| {
                    tester.widget(*element, |paint: &CustomPaint| {
                        paint
                            .painter
                            .as_ref()
                            .is_some_and(|painter| painter.as_any().is::<IndicatorPainter>())
                    })
                })
                .expect("the indicator's CustomPaint");
            tester.render_box_of(indicator_paint).as_object()
        }

        /// Dart's `expect(find.byType(TabBar), paints..rrect(rrect: rrect))`.
        fn paints_rrect(tester: &WidgetTester, rrect: RRect) -> bool {
            tester.paints_rrect(indicator_paint(tester), rrect, None)
        }

        /// [`paints_rrect`] with the rrect's edges within `epsilon` of what is painted.
        fn paints_rrect_within(tester: &WidgetTester, rrect: RRect, epsilon: f64) -> bool {
            use waft_embedder::valo::Op;
            tester
                .painted_ops(indicator_paint(tester))
                .into_iter()
                .any(|op| match op {
                    Op::DrawPath { path, .. } => {
                        path.as_round_rect().is_some_and(|(rect, radii)| {
                            close(f64::from(rect.x), rrect.left, epsilon)
                                && close(f64::from(rect.y), rrect.top, epsilon)
                                && close(f64::from(rect.x + rect.width), rrect.right, epsilon)
                                && close(f64::from(rect.y + rect.height), rrect.bottom, epsilon)
                                && radii == [[3.0, 3.0], [3.0, 3.0], [0.0, 0.0], [0.0, 0.0]]
                        })
                    }
                    _ => false,
                })
        }

        /// Dart's `_accelerateInterpolation`.
        fn accelerate_interpolation(fraction: f64) -> f64 {
            1.0 - ((fraction * PI) / 2.0).cos()
        }

        /// Dart's `_decelerateInterpolation`.
        fn decelerate_interpolation(fraction: f64) -> f64 {
            ((fraction * PI) / 2.0).sin()
        }

        /// tabs_utils.dart's `tabIndicatorRRectElasticAnimation`.
        fn tab_indicator_rrect_elastic_animation(
            tab_bar_height: f64,
            _current_rect: Rect,
            from_rect: Rect,
            to_rect: Rect,
            progress: f64,
        ) -> RRect {
            const INDICATOR_WEIGHT: f64 = 3.0;
            let left_fraction = accelerate_interpolation(progress);
            let right_fraction = decelerate_interpolation(progress);
            let lerp = |a: f64, b: f64, t: f64| lerp_double(Some(a), Some(b), t).expect("lerps");

            RRect::from_ltrb_and_corners(
                lerp(from_rect.left, to_rect.left, left_fraction),
                tab_bar_height - INDICATOR_WEIGHT,
                lerp(from_rect.right, to_rect.right, right_fraction),
                tab_bar_height,
                Radius::circular(INDICATOR_WEIGHT),
                Radius::circular(INDICATOR_WEIGHT),
                Radius::ZERO,
                Radius::ZERO,
            )
        }

        /// Dart's `Container(alignment: Alignment.topLeft, child: child)`.
        fn top_left<K>(child: impl IntoWidget<K>) -> WidgetRef {
            waft_widgets::Container::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .child(child)
                .into_widget()
        }

        /// Dart's `Scaffold(appBar: AppBar(title:, bottom: tab_bar), body:)`.
        fn scaffold_with_tab_bar(
            title: Option<&str>,
            tab_bar: TabBar,
            body: Option<WidgetRef>,
        ) -> Scaffold {
            let mut app_bar = AppBar::new().bottom(PreferredSizeWidgetRef::new(tab_bar));
            if let Some(title) = title {
                app_bar = app_bar.title(Text::new(title));
            }
            let mut scaffold = Scaffold::new().app_bar(PreferredSizeWidgetRef::new(app_bar));
            if let Some(body) = body {
                scaffold = scaffold.body(body);
            }
            scaffold
        }

        fn text_tabs(texts: &[&str]) -> Vec<WidgetRef> {
            texts
                .iter()
                .map(|text| Tab::new().text(*text).into_widget())
                .collect()
        }

        // ---- Material 2 ----

        // 'TabBar default selected/unselected text style'.
        #[test]
        fn material2_labels_take_the_primary_text_themes_body_large_and_70_percent_unselected() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = m2(&tester);
                let mut frame = Frame::new(vec!["A", "B", "C"], "A");
                frame.use_material3 = Some(false);
                let frame = frame.build(&tester);
                tester.pump_widget(frame);
                assert_eq!(tester.find_text("A").len(), 1);
                assert_eq!(tester.find_text("B").len(), 1);
                assert_eq!(tester.find_text("C").len(), 1);

                let body_large_color = theme
                    .primary_text_theme
                    .body_large
                    .as_ref()
                    .and_then(|style| style.color.as_ref().map(AnyColor::color))
                    .expect("a color");

                // Test selected label text style.
                let selected = paragraph_style(&tester, "A");
                assert_eq!(selected.font_family.as_deref(), Some("Roboto"));
                assert_eq!(selected.font_size, Some(14.0));
                assert_eq!(paragraph_color(&tester, "A"), Some(body_large_color));

                // Test unselected label text style.
                let unselected = paragraph_style(&tester, "C");
                assert_eq!(unselected.font_family.as_deref(), Some("Roboto"));
                assert_eq!(unselected.font_size, Some(14.0));
                assert_eq!(
                    paragraph_color(&tester, "C"),
                    Some(body_large_color.with_alpha(0xB2)) // 70% alpha,
                );
            });
        }

        // 'TabBar default unselectedLabelColor inherits labelColor with 70% opacity'.
        #[test]
        fn material2_the_unselected_label_color_is_the_label_color_at_70_percent() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const LABEL_COLOR: Color = Color::new(0xff0000ff);
                let mut frame = Frame::new(vec!["A", "B", "C"], "A");
                frame.use_material3 = Some(false);
                frame.tab_bar_theme = Some(TabBarThemeData::new().label_color(LABEL_COLOR));
                let frame = frame.build(&tester);
                tester.pump_widget(frame);
                assert_eq!(tester.find_text("A").len(), 1);
                assert_eq!(tester.find_text("B").len(), 1);
                assert_eq!(tester.find_text("C").len(), 1);

                // Test selected label color.
                assert_eq!(paragraph_color(&tester, "A"), Some(LABEL_COLOR));

                // Test unselected label color.
                assert_eq!(
                    paragraph_color(&tester, "C"),
                    Some(LABEL_COLOR.with_alpha(0xB2)) // 70% alpha,
                );
            });
        }

        // 'Material2 - Default TabAlignment'.
        #[test]
        fn material2_tabs_fill_a_fixed_tab_bar_and_start_a_scrollable_one() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let mut frame = Frame::new(vec!["A", "B"], "B");
                frame.use_material3 = Some(false);
                let frame = frame.build(&tester);
                tester.pump_widget(frame);

                let tab_bar = rect_of(&tester, only::<TabBar>(&tester));
                let tabs = tabs_of(&tester);
                let tab_one_rect = rect_of(&tester, tabs[0]);
                let tab_two_rect = rect_of(&tester, tabs[1]);

                // Tabs should fill the width of the TabBar.
                let tab_one_left = ((tab_bar.width() / 2.0) - tab_one_rect.width()) / 2.0;
                assert_eq!(tab_one_rect.left, tab_one_left);
                let tab_two_right =
                    tab_bar.width() - ((tab_bar.width() / 2.0) - tab_two_rect.width()) / 2.0;
                assert_eq!(tab_two_rect.right, tab_two_right);

                // Test default TabAlignment when isScrollable is true.
                let mut frame = Frame::new(vec!["A", "B"], "B");
                frame.use_material3 = Some(false);
                frame.is_scrollable = true;
                let frame = frame.build(&tester);
                tester.pump_widget(frame);

                let tabs = tabs_of(&tester);
                let tab_one_rect = rect_of(&tester, tabs[0]);
                let tab_two_rect = rect_of(&tester, tabs[1]);

                // Tabs should be aligned to the start of the TabBar.
                let padding =
                    EdgeInsetsGeometry::from(K_TAB_LABEL_PADDING).resolve(Some(TextDirection::Ltr));
                assert_eq!(tab_one_rect.left, padding.left);
                let tab_two_right = padding.horizontal()
                    + tab_one_rect.width()
                    + padding.left
                    + tab_two_rect.width();
                assert_eq!(tab_two_rect.right, tab_two_right);
            });
        }

        fn material2_default_indicator(secondary: bool) {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = m2(&tester);
                let controller = controller(&tester, 4);
                let tabs =
                    (0..4).map(|index| Tab::new().text(format!("Tab {index}")).into_widget());
                let tab_bar = if secondary {
                    TabBar::secondary(tabs)
                } else {
                    TabBar::new(tabs)
                }
                .controller(controller);
                tester.pump_widget(material_app(
                    theme.clone(),
                    boilerplate(theme.clone(), TextDirection::Ltr, top_left(tab_bar)),
                ));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                assert_eq!(tab_bar_box.size(&tester.cell.borrow()).height(), 48.0);

                const INDICATOR_WEIGHT: f64 = 2.0;
                const INDICATOR_Y: f64 = 48.0 - (INDICATOR_WEIGHT / 2.0);
                const INDICATOR_LEFT: f64 = INDICATOR_WEIGHT / 2.0;
                const INDICATOR_RIGHT: f64 = 200.0 - (INDICATOR_WEIGHT / 2.0);

                #[allow(deprecated)]
                let indicator_color = theme.indicator_color;
                let lines = painted_lines(&tester, tab_bar_box.as_object());
                assert!(
                    lines.iter().any(|line| is_line(
                        line,
                        indicator_color,
                        Some(INDICATOR_WEIGHT),
                        Offset::new(INDICATOR_LEFT, INDICATOR_Y),
                        Offset::new(INDICATOR_RIGHT, INDICATOR_Y),
                    )),
                    "{lines:?}"
                );
            });
        }

        // 'TabBar default tab indicator (primary)', Material 2.
        #[test]
        fn material2_the_default_primary_indicator_is_a_tab_wide_line_in_the_indicator_color() {
            material2_default_indicator(false);
        }

        // 'TabBar default tab indicator (secondary)', Material 2.
        #[test]
        fn material2_the_default_secondary_indicator_is_a_tab_wide_line_in_the_indicator_color() {
            material2_default_indicator(true);
        }

        // 'Material2 - TabBar with padding isScrollable: true'.
        #[test]
        fn material2_a_scrollable_tab_bar_insets_its_tabs_by_its_padding() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_WEIGHT: f64 = 2.0; // default indicator weight
                let padding = EdgeInsets::only(3.0, 7.0, 5.0, 3.0);
                let keys = [GlobalKey::new(), GlobalKey::new(), GlobalKey::new()];
                let tabs = vec![
                    SizedBox::new()
                        .key(keys[0].clone().into())
                        .width(130.0)
                        .height(30.0)
                        .into_widget(),
                    SizedBox::new()
                        .key(keys[1].clone().into())
                        .width(140.0)
                        .height(40.0)
                        .into_widget(),
                    SizedBox::new()
                        .key(keys[2].clone().into())
                        .width(150.0)
                        .height(50.0)
                        .into_widget(),
                ];
                let controller = controller(&tester, tabs.len());
                let theme = m2(&tester);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    top_left(
                        TabBar::new(tabs)
                            .padding(padding)
                            .label_padding(EdgeInsetsGeometry::ZERO)
                            .is_scrollable(true)
                            .controller(controller),
                    ),
                ));

                let tab_bar_box = first_render_box::<TabBar>(&tester);
                let tab_bar_height = 50.0 + INDICATOR_WEIGHT + padding.top + padding.bottom; // 50 = max tab height
                assert_eq!(
                    tab_bar_box.size(&tester.cell.borrow()).height(),
                    tab_bar_height
                );

                let key_rect = |tester: &WidgetTester, key: &GlobalKey| {
                    tester.get_rect(tester.render_object(key).as_box().expect("a box"))
                };

                // Tab0 width = 130, height = 30
                let mut tab_left = padding.left;
                let mut tab_right = tab_left + 130.0;
                let mut tab_top =
                    (tab_bar_height - INDICATOR_WEIGHT + (padding.top - padding.bottom) - 30.0)
                        / 2.0;
                let mut tab_bottom = tab_top + 30.0;
                assert_eq!(
                    key_rect(&tester, &keys[0]),
                    Rect::from_ltrb(tab_left, tab_top, tab_right, tab_bottom)
                );

                // Tab1 width = 140, height = 40
                tab_left = tab_right;
                tab_right = tab_left + 140.0;
                tab_top = (tab_bar_height - INDICATOR_WEIGHT + (padding.top - padding.bottom)
                    - 40.0)
                    / 2.0;
                tab_bottom = tab_top + 40.0;
                assert_eq!(
                    key_rect(&tester, &keys[1]),
                    Rect::from_ltrb(tab_left, tab_top, tab_right, tab_bottom)
                );

                // Tab2 width = 150, height = 50
                tab_left = tab_right;
                tab_right = tab_left + 150.0;
                tab_top = (tab_bar_height - INDICATOR_WEIGHT + (padding.top - padding.bottom)
                    - 50.0)
                    / 2.0;
                tab_bottom = tab_top + 50.0;
                assert_eq!(
                    key_rect(&tester, &keys[2]),
                    Rect::from_ltrb(tab_left, tab_top, tab_right, tab_bottom)
                );

                tab_right += padding.right;
                assert_eq!(tab_bar_box.size(&tester.cell.borrow()).width(), tab_right);
            });
        }

        // 'Material2 - TabAlignment updates tabs alignment (non-scrollable TabBar)'.
        #[test]
        fn material2_tab_alignment_center_stops_fixed_tabs_filling_the_tab_bar() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = m2(&tester);

                // Test TabAlignment.fill (default) when isScrollable is false.
                let frame = Frame::new(vec!["A", "B"], "B").build(&tester);
                tester.pump_widget(material_app(theme.clone(), frame));

                let tab_bar = rect_of(&tester, only::<TabBar>(&tester));
                let tabs = tabs_of(&tester);
                let tab_one_rect = rect_of(&tester, tabs[0]);
                let tab_two_rect = rect_of(&tester, tabs[1]);

                // Within f32: global positions go through valo's f32 matrices (waft-gestures'
                // `Matrix4` entry), where Dart's are f64 and compare within 1e-10.
                const EPSILON: f64 = 1e-4;

                // By default tabs should fill the width of the TabBar.
                let tab_one_left = ((tab_bar.width() / 2.0) - tab_one_rect.width()) / 2.0;
                assert!(close(tab_one_rect.left, tab_one_left, EPSILON));
                let tab_two_right =
                    tab_bar.width() - ((tab_bar.width() / 2.0) - tab_two_rect.width()) / 2.0;
                assert!(close(tab_two_rect.right, tab_two_right, EPSILON));

                // Test TabAlignment.center when isScrollable is false.
                let mut frame = Frame::new(vec!["A", "B"], "B");
                frame.tab_alignment = Some(TabAlignment::Center);
                let frame = frame.build(&tester);
                tester.pump_widget(material_app(theme, frame));
                tester.pump_and_settle();

                let tabs = tabs_of(&tester);
                let tab_one_rect = rect_of(&tester, tabs[0]);
                let tab_two_rect = rect_of(&tester, tabs[1]);

                // Tabs should not fill the width of the TabBar.
                let padding =
                    EdgeInsetsGeometry::from(K_TAB_LABEL_PADDING).resolve(Some(TextDirection::Ltr));
                assert!(close(tab_one_rect.left, padding.left, EPSILON));
                let tab_two_right = padding.horizontal()
                    + tab_one_rect.width()
                    + padding.left
                    + tab_two_rect.width();
                assert!(close(tab_two_rect.right, tab_two_right, EPSILON));
            });
        }

        // ---- after the Material 2 group ----

        // 'does not crash if switching to a newly added tab'.
        #[test]
        fn switching_to_a_newly_added_tab_shows_its_view() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let build_tabs = |tester: &WidgetTester, tab_count: usize| {
                    let theme = theme(tester, None, None);
                    boilerplate(
                        theme,
                        TextDirection::Ltr,
                        DefaultTabController::new(
                            tab_count,
                            scaffold_with_tab_bar(
                                Some("Flutter Demo Click Counter"),
                                TabBar::new(
                                    (0..tab_count)
                                        .map(|i| Tab::new().text(format!("Tab {i}")).into_widget()),
                                )
                                .tab_alignment(TabAlignment::Start)
                                .is_scrollable(true),
                                Some(
                                    TabBarView::new(
                                        (0..tab_count)
                                            .map(|i| Text::new(format!("View {i}")).into_widget()),
                                    )
                                    .into_widget(),
                                ),
                            ),
                        ),
                    )
                };

                let widget = build_tabs(&tester, 1);
                tester.pump_widget(widget);
                assert_eq!(tabs_of(&tester).len(), 1);

                let widget = build_tabs(&tester, 2);
                tester.pump_widget(widget);
                assert_eq!(tabs_of(&tester).len(), 2);

                let widget = build_tabs(&tester, 3);
                tester.pump_widget(widget);
                assert_eq!(tabs_of(&tester).len(), 3);

                assert_eq!(tester.find_onstage_text("View 0").len(), 1);
                assert!(tester.find_onstage_text("View 2").is_empty());
                let tab_2 = text(&tester, "Tab 2");
                tap(&mut tester, tab_2);
                tester.pump_and_settle();
                assert!(tester.find_onstage_text("View 0").is_empty());
                assert_eq!(tester.find_onstage_text("View 2").len(), 1);
            });
        }

        /// tabs_utils.dart's `TestIndicatorDecoration`: each painter it creates records the last
        /// configuration it painted with.
        #[derive(Clone, Default)]
        struct TestIndicatorDecoration {
            painters: Rc<RefCell<Vec<LastConfiguration>>>,
        }

        /// What a `TestIndicatorBoxPainter` last painted with.
        type LastConfiguration = Rc<RefCell<Option<ImageConfiguration>>>;

        impl fmt::Debug for TestIndicatorDecoration {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("TestIndicatorDecoration")
            }
        }

        impl Decoration for TestIndicatorDecoration {
            fn create_box_painter(
                &self,
                _on_changed: Option<waft_foundation::VoidCallback>,
            ) -> Box<dyn BoxPainter> {
                let last_configuration = Rc::new(RefCell::new(None));
                self.painters
                    .borrow_mut()
                    .push(Rc::clone(&last_configuration));
                Box::new(TestIndicatorBoxPainter { last_configuration })
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        /// tabs_utils.dart's `TestIndicatorBoxPainter`.
        struct TestIndicatorBoxPainter {
            last_configuration: LastConfiguration,
        }

        impl BoxPainter for TestIndicatorBoxPainter {
            fn paint(
                &mut self,
                _canvas: &mut Canvas,
                _offset: Offset,
                configuration: &ImageConfiguration,
            ) {
                *self.last_configuration.borrow_mut() = Some(configuration.clone());
            }

            fn on_changed(&self) -> Option<&dyn Fn(&mut App)> {
                None
            }
        }

        // 'Tab indicator painter image configuration'.
        #[test]
        fn the_indicator_paints_with_its_size_the_text_direction_and_the_device_pixel_ratio() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let decoration = TestIndicatorDecoration::default();

                let build_tabs =
                    |tester: &WidgetTester, text_direction: TextDirection, ratio: f64| {
                        let theme = theme(tester, None, None);
                        material_app(
                            theme,
                            MediaQuery::new(
                                MediaQueryData::new().device_pixel_ratio(ratio),
                                Directionality::new(
                                    text_direction,
                                    DefaultTabController::new(
                                        2,
                                        scaffold_with_tab_bar(
                                            None,
                                            TabBar::new(text_tabs(&["A", "B"]))
                                                .indicator(decoration.clone()),
                                            None,
                                        ),
                                    ),
                                ),
                            )
                            .into_widget(),
                        )
                    };

                let widget = build_tabs(&tester, TextDirection::Ltr, 1.0);
                tester.pump_widget(widget);

                let last = |decoration: &TestIndicatorDecoration| {
                    decoration
                        .painters
                        .borrow()
                        .last()
                        .expect("a painter")
                        .borrow()
                        .clone()
                        .expect("a configuration")
                };
                let config = last(&decoration);
                let size = config.size.expect("a size");
                assert!(close(size.width(), 14.1, 0.1), "{size:?}");
                assert_eq!(size.height(), 48.0);
                assert_eq!(config.text_direction, Some(TextDirection::Ltr));
                assert_eq!(config.device_pixel_ratio, Some(1.0));

                let widget = build_tabs(&tester, TextDirection::Rtl, 2.33);
                tester.pump_widget(widget);

                let config = last(&decoration);
                let size = config.size.expect("a size");
                assert!(close(size.width(), 14.1, 0.1), "{size:?}");
                assert_eq!(size.height(), 48.0);
                assert_eq!(config.text_direction, Some(TextDirection::Rtl));
                assert_eq!(config.device_pixel_ratio, Some(2.33));
            });
        }

        // 'TabBar.textScaler overrides tab label text scale, textScaleFactor = noScaling, 1.75, 2.0'.
        #[test]
        fn the_text_scaler_of_a_tab_bar_overrides_the_ambient_one() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let build_tabs = |tester: &WidgetTester, text_scaler: TextScaler| {
                    let theme = theme(tester, None, None);
                    material_app(
                        theme,
                        MediaQuery::new(
                            MediaQueryData::new().text_scaler(TextScaler::linear(3.0)),
                            DefaultTabController::new(
                                2,
                                scaffold_with_tab_bar(
                                    None,
                                    TabBar::new(text_tabs(&["Tab 1", "Tab 2"]))
                                        .text_scaler(text_scaler),
                                    None,
                                ),
                            ),
                        )
                        .into_widget(),
                    )
                };

                let widget = build_tabs(&tester, TextScaler::NO_SCALING);
                tester.pump_widget(widget);
                assert_eq!(
                    size_of(&tester, text(&tester, "Tab 1")),
                    Size::new(70.5, 20.0)
                );

                let widget = build_tabs(&tester, TextScaler::linear(1.75));
                tester.pump_widget(widget);
                assert_eq!(
                    size_of(&tester, text(&tester, "Tab 1")),
                    Size::new(123.0, 35.0)
                );

                let widget = build_tabs(&tester, TextScaler::linear(2.0));
                tester.pump_widget(widget);
                assert_eq!(
                    size_of(&tester, text(&tester, "Tab 1")),
                    Size::new(140.5, 40.0)
                );
            });
        }

        /// The pixels of the first `SingleChildScrollView`'s controller.
        fn scroll_view_pixels(tester: &WidgetTester) -> f64 {
            let scroll_view = only::<SingleChildScrollView>(tester);
            let controller = tester.widget(scroll_view, |scroll_view: &SingleChildScrollView| {
                scroll_view
                    .controller
                    .clone()
                    .expect("a tab bar hands its scroll view a controller")
            });
            let app = tester.cell.borrow();
            controller.position(&app).pixels(&app)
        }

        /// Dart's `MaterialApp(home: DefaultTabController(length: 20, initialIndex:, child:
        /// Scaffold(appBar: AppBar(bottom: TabBar(isScrollable: true, ..)), body: TabBarView(..))))`.
        fn twenty_scrollable_tabs(tester: &WidgetTester, initial_index: usize) -> WidgetRef {
            let theme = theme(tester, None, None);
            let tabs = (0..20).map(|index| Tab::new().text(format!("Tab {index}")).into_widget());
            material_app(
                theme,
                DefaultTabController::new(
                    20,
                    scaffold_with_tab_bar(
                        None,
                        TabBar::new(tabs).is_scrollable(true),
                        Some(
                            TabBarView::new((0..20).map(|i| {
                                Center::new()
                                    .child(Text::new(format!("Page {i}")))
                                    .into_widget()
                            }))
                            .into_widget(),
                        ),
                    ),
                )
                .initial_index(initial_index)
                .into_widget(),
            )
        }

        // 'Scrollable TabBar does not jitter in the middle position'.
        #[test]
        fn a_scrollable_tab_bar_follows_a_dragged_tab_bar_view_without_jitter_in_the_middle() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let widget = twenty_scrollable_tabs(&tester, 10);
                tester.pump_widget(widget);

                assert_eq!(tester.find_onstage_text("Page 10").len(), 1);
                assert!(tester.find_onstage_text("Page 11").is_empty());
                let pixels = scroll_view_pixels(&tester);
                assert!(close(pixels, 683.2, 0.1), "{pixels}");

                // Drag the TabBarView to the left.
                let view = center_of(&tester, only::<TabBarView>(&tester));
                tester.drag_from(view, Offset::new(-800.0, 0.0), 1);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(200));

                assert!(tester.find_onstage_text("Page 10").is_empty());
                assert_eq!(tester.find_onstage_text("Page 11").len(), 1);
                let pixels = scroll_view_pixels(&tester);
                assert!(close(pixels, 799.8, 0.1), "{pixels}");
            });
        }

        // 'Scrollable TabBar does not jitter when the tab bar reaches the start'.
        #[test]
        fn a_scrollable_tab_bar_follows_a_dragged_tab_bar_view_without_jitter_at_the_start() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let widget = twenty_scrollable_tabs(&tester, 4);
                tester.pump_widget(widget);

                assert_eq!(tester.find_onstage_text("Page 4").len(), 1);
                assert!(tester.find_onstage_text("Page 3").is_empty());
                let pixels = scroll_view_pixels(&tester);
                assert!(close(pixels, 61.25, 0.1), "{pixels}");

                // Drag the TabBarView to the right.
                let start = center_of(&tester, text(&tester, "Page 4"));
                let mut gesture = WidgetTester::touch(1);
                tester.down(&mut gesture, start);
                tester.move_to(&mut gesture, start + Offset::new(600.0, 0.0), true);
                tester.up(&gesture);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(500));

                assert_eq!(tester.find_onstage_text("Page 4").len(), 1);
                assert_eq!(tester.find_onstage_text("Page 3").len(), 1);
                let pixels = scroll_view_pixels(&tester);
                assert!(close(pixels, 0.2, 0.1), "{pixels}");

                tester.pump_and_settle();
                assert!(tester.find_onstage_text("Page 4").is_empty());
                assert_eq!(tester.find_onstage_text("Page 3").len(), 1);
                assert_eq!(scroll_view_pixels(&tester), 0.0);
            });
        }

        // 'Scrollable TabBar with transparent divider expands to full width'.
        #[test]
        fn a_scrollable_tab_bar_with_a_transparent_divider_is_as_wide_as_its_parent() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                for tab_alignment in [
                    TabAlignment::StartOffset,
                    TabAlignment::Start,
                    TabAlignment::Center,
                ] {
                    let theme = theme(&tester, None, None);
                    tester.pump_widget(boilerplate(
                        theme,
                        TextDirection::Ltr,
                        Center::new().child(DefaultTabController::new(
                            3,
                            TabBar::new(text_tabs(&["Tab 1", "Tab 2", "Tab 3"]))
                                .divider_color(Colors::TRANSPARENT)
                                .tab_alignment(tab_alignment)
                                .is_scrollable(true),
                        )),
                    ));
                    assert_eq!(
                        size_of(&tester, only::<TabBar>(&tester)).width(),
                        800.0,
                        "{tab_alignment:?}"
                    );
                }
            });
        }

        // 'TabBar.indicatorAnimation can customize tab indicator animation'.
        #[test]
        fn the_indicator_animation_of_a_tab_bar_moves_the_indicator_linearly_or_elastically() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                const INDICATOR_WIDTH: f64 = 50.0;
                let controller = controller(&tester, 4);
                let build_tab =
                    |tester: &WidgetTester, indicator_animation: TabIndicatorAnimation| {
                        let theme = theme(tester, None, None);
                        let tabs = (0..4).map(|index| {
                            Tab::new()
                                .key(waft_foundation::ValueKey::new(index).into())
                                .child(SizedBox::new().width(INDICATOR_WIDTH))
                                .into_widget()
                        });
                        material_app(
                            theme.clone(),
                            boilerplate(
                                theme,
                                TextDirection::Ltr,
                                top_left(
                                    TabBar::new(tabs)
                                        .indicator_animation(indicator_animation)
                                        .controller(controller.clone()),
                                ),
                            ),
                        )
                    };

                // Test tab indicator animation with TabIndicatorAnimation.linear.
                let widget = build_tab(&tester, TabIndicatorAnimation::Linear);
                tester.pump_widget(widget);

                // Idle at tab 0.
                assert!(paints_rrect(
                    &tester,
                    indicator_rrect(75.0, 45.0, 125.0, 48.0)
                ));

                // Start moving tab indicator.
                controller.set_offset(&mut tester.cell.borrow_mut(), 0.2);
                tester.pump(Duration::ZERO);

                assert!(paints_rrect(
                    &tester,
                    indicator_rrect(115.0, 45.0, 165.0, 48.0)
                ));

                // Reset tab controller offset.
                controller.set_offset(&mut tester.cell.borrow_mut(), 0.0);

                // Test tab indicator animation with TabIndicatorAnimation.elastic.
                let widget = build_tab(&tester, TabIndicatorAnimation::Elastic);
                tester.pump_widget(widget);
                tester.pump_and_settle();

                let tab_bar_height = first_render_box::<TabBar>(&tester)
                    .size(&tester.cell.borrow())
                    .height();

                // Idle at tab 0.
                let current_rect = Rect::from_ltrb(75.0, 0.0, 125.0, 48.0);
                let from_rect = Rect::from_ltrb(75.0, 0.0, 125.0, 48.0);
                let to_rect = Rect::from_ltrb(75.0, 0.0, 125.0, 48.0);
                assert!(paints_rrect(
                    &tester,
                    tab_indicator_rrect_elastic_animation(
                        tab_bar_height,
                        current_rect,
                        from_rect,
                        to_rect,
                        0.0
                    )
                ));

                controller.set_offset(&mut tester.cell.borrow_mut(), 0.2);
                tester.pump(Duration::ZERO);
                let to_rect = Rect::from_ltrb(275.0, 0.0, 325.0, 48.0);
                assert!(paints_rrect(
                    &tester,
                    tab_indicator_rrect_elastic_animation(
                        tab_bar_height,
                        current_rect,
                        from_rect,
                        to_rect,
                        0.2
                    )
                ));
            });
        }

        // 'Tabs icon respects ambient icon theme'.
        #[test]
        fn tab_icons_take_the_ambient_icon_theme_except_the_selected_ones_color() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let icon_theme = IconThemeData::new()
                    .color(Color::new(0xffff0000))
                    .size(38.0);
                let theme = {
                    let app = tester.cell.borrow();
                    ThemeData::builder().icon_theme(icon_theme).build(&app)
                };
                let selected_icon = Icons::ac_unit();
                let unselected_icon = Icons::access_alarm();
                tester.pump_widget(boilerplate(
                    theme.clone(),
                    TextDirection::Ltr,
                    DefaultTabController::new(
                        2,
                        TabBar::new([
                            Tab::new()
                                .icon(Icon::new(Some(selected_icon.clone())))
                                .text("Tab 1")
                                .into_widget(),
                            Tab::new()
                                .icon(Icon::new(Some(unselected_icon.clone())))
                                .text("Tab 2")
                                .into_widget(),
                        ]),
                    ),
                ));

                let icon_color = |tester: &WidgetTester, icon| {
                    let element = tester.find_by_icon(icon);
                    tester
                        .glyph_style(element)
                        .color
                        .as_ref()
                        .map(AnyColor::color)
                };

                // The iconTheme color isn't applied to the selected icon.
                assert_eq!(
                    icon_color(&tester, &selected_icon),
                    Some(theme.color_scheme.primary)
                );
                // The iconTheme color is applied to the unselected icon.
                assert_eq!(
                    icon_color(&tester, &unselected_icon),
                    theme.icon_theme.color.as_ref().map(AnyColor::color)
                );

                // Both selected and unselected icons should have the iconTheme size.
                let size = theme.icon_theme.size.expect("a size");
                let selected = tester.find_by_icon(&selected_icon);
                assert_eq!(size_of(&tester, selected), Size::new(size, size));
                let unselected = tester.find_by_icon(&unselected_icon);
                assert_eq!(size_of(&tester, unselected), Size::new(size, size));
            });
        }

        /// The eight tabs of the overflow cases.
        const OVERFLOW_TABS: [&str; 8] = [
            "Short",
            "A Bit Longer Text",
            "An Extremely Long Tab Label That Overflows",
            "Tiny",
            "Moderate Length",
            "Just Right",
            "Supercalifragilisticexpialidocious",
            "Longer Than Usual",
        ];

        /// The overflow cases' `buildTabBar`, pumped, with its controller.
        fn pump_overflow_tab_bar(
            tester: &mut WidgetTester,
            text_direction: TextDirection,
            indicator_animation: TabIndicatorAnimation,
            is_scrollable: bool,
        ) -> Ref<TabController> {
            let controller = controller(tester, OVERFLOW_TABS.len());
            let theme = theme(tester, None, None);
            tester.pump_widget(boilerplate(
                theme,
                text_direction,
                top_left(
                    TabBar::new(text_tabs(&OVERFLOW_TABS))
                        .is_scrollable(is_scrollable)
                        .indicator_animation(indicator_animation)
                        .controller(controller.clone()),
                ),
            ));
            controller
        }

        /// Dart's `tester.tap(find.byType(Tab).at(index))`.
        fn tap_tab(tester: &mut WidgetTester, index: usize) {
            let tab = tabs_of(tester)[index];
            tap(tester, tab);
        }

        /// Dart's `tester.tap(find.byType(Tab).last)`.
        fn tap_last_tab(tester: &mut WidgetTester) {
            let tab = *tabs_of(tester).last().expect("a tab");
            tap(tester, tab);
        }

        fn tab_rect(tester: &WidgetTester, index: usize) -> Rect {
            rect_of(tester, tabs_of(tester)[index])
        }

        /// The cases on a fixed tab bar: the indicator at two points of the animation to tab 2, kept
        /// inside the label's trailing edge, then on the label.
        fn fixed_overflow_case(
            text_direction: TextDirection,
            indicator_animation: TabIndicatorAnimation,
            steps: [(f64, f64); 2],
        ) {
            with_main_thread_stack(|| {
                let mut tester = tester();
                pump_overflow_tab_bar(&mut tester, text_direction, indicator_animation, false);

                tap_tab(&mut tester, 2);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(100));

                let label_rect = tab_rect(&tester, 2);
                for (step, (indicator_left, indicator_right)) in steps.into_iter().enumerate() {
                    if step == 1 {
                        tester.pump(Duration::from_millis(100));
                    }
                    assert!(
                        paints_rrect(
                            &tester,
                            indicator_rrect(indicator_left, 45.0, indicator_right, 48.0)
                        ),
                        "step {step}"
                    );
                    match text_direction {
                        TextDirection::Ltr => assert!(indicator_right < label_rect.right),
                        TextDirection::Rtl => assert!(indicator_left > label_rect.left),
                    }
                }

                // Let the animation complete.
                tester.pump(Duration::from_millis(200));

                assert!(paints_rrect(
                    &tester,
                    indicator_rrect(label_rect.left, 45.0, label_rect.right, 48.0)
                ));
            });
        }

        // 'Elastic Tab animation does not overflow target tab - LTR'.
        #[test]
        fn the_elastic_indicator_stays_inside_the_target_tab_left_to_right() {
            fixed_overflow_case(
                TextDirection::Ltr,
                TabIndicatorAnimation::Elastic,
                [
                    (92.50662931979836, 241.31938023664574),
                    (192.50227846755732, 282.61484607849377),
                ],
            );
        }

        // 'Elastic Tab animation does not overflow target tab - RTL'.
        #[test]
        fn the_elastic_indicator_stays_inside_the_target_tab_right_to_left() {
            fixed_overflow_case(
                TextDirection::Rtl,
                TabIndicatorAnimation::Elastic,
                [
                    (558.6806197633543, 707.4933706802017),
                    (517.3851539215062, 607.497721532442),
                ],
            );
        }

        // 'Linear Tab animation does not overflow target tab - LTR'.
        #[test]
        fn the_linear_indicator_stays_inside_the_target_tab_left_to_right() {
            fixed_overflow_case(
                TextDirection::Ltr,
                TabIndicatorAnimation::Linear,
                [
                    (131.26358723640442, 199.26358723640442),
                    (201.00625545158982, 269.0062554515898),
                ],
            );
        }

        // 'Linear Tab animation does not overflow target tab - RTL'.
        #[test]
        fn the_linear_indicator_stays_inside_the_target_tab_right_to_left() {
            fixed_overflow_case(
                TextDirection::Rtl,
                TabIndicatorAnimation::Linear,
                [
                    (600.7364127635956, 668.7364127635956),
                    (530.9937445484102, 598.9937445484102),
                ],
            );
        }

        /// What a step of a scrollable overflow case checks against the tab's corner.
        #[derive(Clone, Copy)]
        enum Edge {
            /// `indicatorRight < bottomRight.dx`.
            RightBeforeBottomRight,
            /// `indicatorLeft < bottomRight.dx`.
            LeftBeforeBottomRight,
            /// `indicatorLeft > bottomLeft.dx`.
            LeftAfterBottomLeft,
            /// `indicatorLeft > bottomRight.dx`.
            LeftAfterBottomRight,
        }

        /// The cases on a scrollable tab bar: the indicator at three points of the animation to tab
        /// 2, each checked against the tab's corner as it then stands.
        ///
        /// The indicator's edges are compared within 1e-3: they sum the widths of long labels, which
        /// Valo measures in `f32`, a few ulps off Dart's `f64` sums at these magnitudes.
        fn scrollable_overflow_case(
            text_direction: TextDirection,
            indicator_animation: TabIndicatorAnimation,
            steps: [(f64, f64, Edge); 3],
        ) {
            with_main_thread_stack(|| {
                let mut tester = tester();
                pump_overflow_tab_bar(&mut tester, text_direction, indicator_animation, true);

                tap_tab(&mut tester, 2);
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_millis(100));

                for (step, (indicator_left, indicator_right, edge)) in steps.into_iter().enumerate()
                {
                    match step {
                        1 => tester.pump(Duration::from_millis(100)),
                        // Let the animation complete.
                        2 => tester.pump(Duration::from_millis(200)),
                        _ => {}
                    }
                    let label_rect = tab_rect(&tester, 2);
                    assert!(
                        paints_rrect_within(
                            &tester,
                            indicator_rrect(indicator_left, 45.0, indicator_right, 48.0),
                            1e-3
                        ),
                        "step {step}"
                    );
                    match edge {
                        Edge::RightBeforeBottomRight => assert!(indicator_right < label_rect.right),
                        Edge::LeftBeforeBottomRight => assert!(indicator_left < label_rect.right),
                        Edge::LeftAfterBottomLeft => assert!(indicator_left > label_rect.left),
                        Edge::LeftAfterBottomRight => assert!(indicator_left > label_rect.right),
                    }
                }
            });
        }

        // 'Elastic Tab animation does not overflow target tab in a scrollable tab bar - LTR'.
        #[test]
        fn the_elastic_indicator_stays_inside_the_target_tab_of_a_scrollable_tab_bar_left_to_right()
        {
            scrollable_overflow_case(
                TextDirection::Ltr,
                TabIndicatorAnimation::Elastic,
                [
                    (
                        159.14390228994424,
                        791.2121709715643,
                        Edge::RightBeforeBottomRight,
                    ),
                    (
                        346.2357603195887,
                        976.195212100479,
                        Edge::LeftBeforeBottomRight,
                    ),
                    (
                        390.1999969482422,
                        982.4000091552734,
                        Edge::LeftBeforeBottomRight,
                    ),
                ],
            );
        }

        // 'Elastic Tab animation does not overflow target tab in a scrollable tab bar - RTL'.
        #[test]
        fn the_elastic_indicator_stays_inside_the_target_tab_of_a_scrollable_tab_bar_right_to_left()
        {
            scrollable_overflow_case(
                TextDirection::Rtl,
                TabIndicatorAnimation::Elastic,
                [
                    (
                        1495.1878305543146,
                        2127.2560992359345,
                        Edge::LeftAfterBottomLeft,
                    ),
                    (
                        1310.2047894254,
                        1940.1642412062902,
                        Edge::LeftAfterBottomLeft,
                    ),
                    (
                        1303.9999923706055,
                        1896.2000045776367,
                        Edge::LeftAfterBottomLeft,
                    ),
                ],
            );
        }

        // 'Linear Tab animation does not overflow target tab in a scrollable tab bar - LTR'.
        #[test]
        fn the_linear_indicator_stays_inside_the_target_tab_of_a_scrollable_tab_bar_left_to_right()
        {
            scrollable_overflow_case(
                TextDirection::Ltr,
                TabIndicatorAnimation::Linear,
                [
                    (
                        159.9711660555031,
                        453.47531034110943,
                        Edge::RightBeforeBottomRight,
                    ),
                    (
                        349.4619934677845,
                        888.8090538538061,
                        Edge::LeftBeforeBottomRight,
                    ),
                    (
                        390.1999969482422,
                        982.4000091552734,
                        Edge::LeftBeforeBottomRight,
                    ),
                ],
            );
        }

        // 'Linear Tab animation does not overflow target tab in a scrollable tab bar - RTL'.
        #[test]
        fn the_linear_indicator_stays_inside_the_target_tab_of_a_scrollable_tab_bar_right_to_left()
        {
            scrollable_overflow_case(
                TextDirection::Rtl,
                TabIndicatorAnimation::Linear,
                [
                    (
                        1832.9246911847695,
                        2126.428835470376,
                        Edge::LeftAfterBottomRight,
                    ),
                    (
                        1397.590947672073,
                        1936.9380080580945,
                        Edge::LeftAfterBottomRight,
                    ),
                    (
                        1303.9999923706055,
                        1896.2000045776367,
                        Edge::LeftAfterBottomRight,
                    ),
                ],
            );
        }

        /// The skipping cases: tab 2, the last tab, then tab 1, each halfway and then settled on the
        /// label.
        fn skipping_case(indicator_animation: TabIndicatorAnimation, halfway: [(f64, f64); 3]) {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let controller = controller(&tester, 10);
                let theme = theme(&tester, None, None);
                tester.pump_widget(boilerplate(
                    theme,
                    TextDirection::Ltr,
                    top_left(
                        TabBar::new(
                            (0..10)
                                .map(|index| Tab::new().text(format!("Tab {index}")).into_widget()),
                        )
                        .indicator_animation(indicator_animation)
                        .controller(controller),
                    ),
                ));

                for (step, (indicator_left, indicator_right)) in halfway.into_iter().enumerate() {
                    let index = match step {
                        0 => {
                            tap_tab(&mut tester, 2);
                            2
                        }
                        1 => {
                            tap_last_tab(&mut tester);
                            9
                        }
                        _ => {
                            tap_tab(&mut tester, 1);
                            1
                        }
                    };
                    tester.pump(Duration::ZERO);
                    tester.pump(Duration::from_millis(200));
                    assert!(
                        paints_rrect(
                            &tester,
                            indicator_rrect(indicator_left, 45.0, indicator_right, 48.0)
                        ),
                        "step {step}"
                    );

                    tester.pump_and_settle();
                    let label_rect = tab_rect(&tester, index);
                    assert!(
                        paints_rrect(
                            &tester,
                            indicator_rrect(label_rect.left, 45.0, label_rect.right, 48.0)
                        ),
                        "step {step} settled"
                    );
                }
            });
        }

        // 'Elastic Tab animation when skipping tabs'.
        #[test]
        fn the_elastic_indicator_moves_straight_to_a_skipped_to_tab() {
            skipping_case(
                TabIndicatorAnimation::Elastic,
                [
                    (157.20182277404584, 222.89187686279502),
                    (670.2063797091604, 780.1215690197826),
                    (100.43249254881991, 219.19270890381662),
                ],
            );
        }

        // 'Linear Tab animation when skipping tabs'.
        #[test]
        fn the_linear_indicator_moves_straight_to_a_skipped_to_tab() {
            skipping_case(
                TabIndicatorAnimation::Linear,
                [
                    (164.00500436127186, 212.00500436127186),
                    (694.0175152644515, 742.0175152644515),
                    (143.97998255491257, 191.97998255491257),
                ],
            );
        }

        /// The `MaterialApp(home: DefaultTabController(length: 3, child: Scaffold(appBar:
        /// AppBar(bottom: tab_bar), body: TabBarView(..))))` of the hover and focus cases.
        fn three_tab_app(tester: &WidgetTester, tab_bar: TabBar) -> WidgetRef {
            let theme = theme(tester, None, None);
            material_app(
                theme,
                DefaultTabController::new(
                    3,
                    scaffold_with_tab_bar(
                        None,
                        tab_bar,
                        Some(
                            TabBarView::new([
                                Text::new("Tab 1 View").into_widget(),
                                Text::new("Tab 2 View").into_widget(),
                                Text::new("Tab 3 View").into_widget(),
                            ])
                            .into_widget(),
                        ),
                    ),
                )
                .into_widget(),
            )
        }

        // 'onHover is triggered when mouse pointer is over a tab'.
        #[test]
        fn on_hover_reports_a_pointer_entering_and_leaving_each_tab() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let hover_events: Rc<RefCell<Vec<(bool, usize)>>> = Rc::default();
                let tab_bar = |secondary: bool| {
                    let events = Rc::clone(&hover_events);
                    let tabs = text_tabs(&["Tab 1", "Tab 2", "Tab 3"]);
                    if secondary {
                        TabBar::secondary(tabs)
                    } else {
                        TabBar::new(tabs)
                    }
                    .on_hover(move |_app, value, index| events.borrow_mut().push((value, index)))
                };

                let widget = three_tab_app(&tester, tab_bar(false));
                tester.pump_widget(widget);
                assert!(hover_events.borrow().is_empty());

                // Hover over the first tab.
                let mut gesture = WidgetTester::mouse(1, 0);
                tester.add_pointer(&mut gesture, Offset::ZERO);
                let mut expected = Vec::new();
                for secondary in [false, true] {
                    if secondary {
                        hover_events.borrow_mut().clear();
                        expected.clear();
                        let widget = three_tab_app(&tester, tab_bar(true));
                        tester.pump_widget(widget);
                        assert!(hover_events.borrow().is_empty());
                    }
                    for (index, label) in ["Tab 1", "Tab 2", "Tab 3"].into_iter().enumerate() {
                        let at = center_of(&tester, text(&tester, label));
                        tester.move_to(&mut gesture, at, false);
                        tester.pump(Duration::ZERO);
                        if index > 0 {
                            expected.push((false, index - 1));
                        }
                        expected.push((true, index));
                        assert_eq!(*hover_events.borrow(), expected);
                    }
                    let at = center_of(&tester, only::<TabBarView>(&tester));
                    tester.move_to(&mut gesture, at, false);
                    tester.pump(Duration::ZERO);
                    expected.push((false, 2));
                    assert_eq!(*hover_events.borrow(), expected);
                }
            });
        }

        // 'onFocusChange is triggered when tabs gain and lose focus'.
        #[test]
        fn on_focus_change_reports_each_tab_gaining_and_losing_focus() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let focus_events: Rc<RefCell<Vec<(bool, usize)>>> = Rc::default();
                let tab_bar = |secondary: bool| {
                    let events = Rc::clone(&focus_events);
                    let tabs = text_tabs(&["Tab 1", "Tab 2", "Tab 3"]);
                    if secondary {
                        TabBar::secondary(tabs)
                    } else {
                        TabBar::new(tabs)
                    }
                    .on_focus_change(move |_app, value, index| {
                        events.borrow_mut().push((value, index))
                    })
                };

                for secondary in [false, true] {
                    focus_events.borrow_mut().clear();
                    let widget = three_tab_app(&tester, tab_bar(secondary));
                    tester.pump_widget(widget);
                    assert!(focus_events.borrow().is_empty());

                    let mut expected = Vec::new();
                    let mut node = None;
                    for (index, label) in ["Tab 1", "Tab 2", "Tab 3"].into_iter().enumerate() {
                        let tab_element = text(&tester, label);
                        let focus_node = {
                            let mut app = tester.cell.borrow_mut();
                            Focus::of(&mut app, tab_element, false, false)
                        };
                        focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
                        tester.pump(Duration::ZERO);
                        if index > 0 {
                            expected.push((false, index - 1));
                        }
                        expected.push((true, index));
                        assert_eq!(*focus_events.borrow(), expected);
                        node = Some(focus_node);
                    }
                    let node = node.expect("the third tab's node");
                    assert!(node.has_focus(&mut tester.cell.borrow_mut()));

                    node.unfocus(&mut tester.cell.borrow_mut(), UnfocusDisposition::Scope);
                    tester.pump(Duration::ZERO);

                    assert!(!node.has_focus(&mut tester.cell.borrow_mut()));
                    expected.push((false, 2));
                    assert_eq!(*focus_events.borrow(), expected);
                }
            });
        }

        // 'Ink features are painted on inner Material'.
        #[test]
        fn a_tab_bars_ink_features_are_painted_on_its_own_material() {
            with_main_thread_stack(|| {
                use crate::material::{AnyMaterialInkController, RenderInkFeatures};

                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    Material::new()
                        .child(
                            Center::new().child(DefaultTabController::new(
                                10,
                                TabBar::new(
                                    (1..=10)
                                        .map(|i| Tab::new().text(format!("Tab {i}")).into_widget()),
                                )
                                .is_scrollable(true),
                            )),
                        )
                        .into_widget(),
                ));

                assert_eq!(tester.find_by_type::<Material>().len(), 2);

                let tab_bar = only::<TabBar>(&tester);
                let semantics = *tester
                    .descendants_of_type::<Semantics>(tab_bar)
                    .first()
                    .expect("a Semantics under the TabBar");
                let (outer_material, inner_material) = {
                    let app = tester.cell.borrow();
                    (Material::of(&app, tab_bar), Material::of(&app, semantics))
                };
                // Material outside the TabBar, and the Material directly wrapping the TabBar.
                assert_ne!(outer_material, inner_material);

                let ink_features = |tester: &WidgetTester, material: AnyMaterialInkController| {
                    let render = tester
                        .all::<RenderInkFeatures>()
                        .into_iter()
                        .find(|render| render.as_object().id() == material.id())
                        .expect("the material's render object");
                    render
                        .debug_ink_features(&tester.cell.borrow())
                        .map(|features| features.len())
                };
                assert_eq!(ink_features(&tester, outer_material), None);
                assert_eq!(ink_features(&tester, inner_material), None);

                // Hover over the first tab to trigger the ink highlight.
                let mut gesture = WidgetTester::mouse(1, 0);
                let at = center_of(&tester, text(&tester, "Tab 1"));
                tester.add_pointer(&mut gesture, at);
                tester.pump(Duration::ZERO);

                // Only the inner Material should have ink features.
                assert_eq!(ink_features(&tester, outer_material), None);
                assert_eq!(ink_features(&tester, inner_material), Some(1));
            });
        }

        // 'Tab can have children with other semantics roles'.
        #[test]
        fn a_tab_child_can_have_its_own_semantics_role() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    Material::new()
                        .child(
                            Center::new().child(DefaultTabController::new(
                                1,
                                TabBar::new([Tab::new()
                                    .child(
                                        Semantics::new()
                                            .role(SemanticsRole::ListItem)
                                            .child(Text::new("A")),
                                    )
                                    .into_widget()])
                                .is_scrollable(true),
                            )),
                        )
                        .into_widget(),
                ));
                assert_eq!(tabs_of(&tester).len(), 1);
            });
        }

        /// A tester whose view has no area: Dart's `tester.view.physicalSize = Size.zero`.
        fn zero_area_tester() -> WidgetTester {
            let tester = WidgetTester::with_physical_size(
                TestPlatform::new().on(TargetPlatform::Android),
                Size::ZERO,
            );
            tester.install_platform_fonts();
            tester
        }

        // 'TabPageSelector does not crash at zero area'.
        #[test]
        fn a_tab_page_selector_animates_at_zero_area() {
            with_main_thread_stack(|| {
                let mut tester = zero_area_tester();
                let controller = controller(&tester, 2);
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(TabPageSelector::new().controller(controller.clone()))
                        .into_widget(),
                ));
                assert_eq!(
                    size_of(&tester, only::<TabPageSelector>(&tester)),
                    Size::ZERO
                );
                controller.animate_to(&mut tester.cell.borrow_mut(), 1, None, None);
                tester.pump(Duration::ZERO);
                tester.pump_and_settle();
            });
        }

        // 'TabBarView does not crash at zero area'.
        #[test]
        fn a_tab_bar_view_animates_at_zero_area() {
            with_main_thread_stack(|| {
                let mut tester = zero_area_tester();
                let controller = controller(&tester, 2);
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            TabBarView::new([
                                Text::new("X").into_widget(),
                                Text::new("Y").into_widget(),
                            ])
                            .controller(controller.clone()),
                        )
                        .into_widget(),
                ));
                assert_eq!(size_of(&tester, only::<TabBarView>(&tester)), Size::ZERO);
                controller.animate_to(&mut tester.cell.borrow_mut(), 1, None, None);
                tester.pump(Duration::ZERO);
                tester.pump_and_settle();
            });
        }

        // 'TabPageSelectorIndicator does not crash at zero area'.
        #[test]
        fn a_tab_page_selector_indicator_builds_at_zero_area() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(SizedBox::shrink().child(TabPageSelectorIndicator::new(
                            Colors::RED.primary(),
                            Colors::BLUE.primary(),
                            1.0,
                        )))
                        .into_widget(),
                ));
                assert_eq!(
                    size_of(&tester, only::<TabPageSelectorIndicator>(&tester)),
                    Size::ZERO
                );
            });
        }

        // 'DefaultTabController does not crash at zero area'.
        #[test]
        fn a_default_tab_controller_builds_at_zero_area() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            SizedBox::shrink().child(DefaultTabController::new(2, Scaffold::new())),
                        )
                        .into_widget(),
                ));
                assert_eq!(
                    size_of(&tester, only::<DefaultTabController>(&tester)),
                    Size::ZERO
                );
            });
        }

        // 'Tab does not crash at zero area'.
        #[test]
        fn a_tab_builds_at_zero_area() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    Scaffold::new()
                        .body(
                            Center::new()
                                .child(SizedBox::shrink().child(Tab::new().child(Text::new("X")))),
                        )
                        .into_widget(),
                ));
                assert_eq!(size_of(&tester, only::<Tab>(&tester)), Size::ZERO);
            });
        }

        // 'TabBar does not crash at zero area'.
        #[test]
        fn a_tab_bar_animates_at_zero_area() {
            with_main_thread_stack(|| {
                let mut tester = zero_area_tester();
                let controller = controller(&tester, 2);
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            TabBar::new([
                                Text::new("X").into_widget(),
                                Text::new("Y").into_widget(),
                            ])
                            .controller(controller.clone()),
                        )
                        .into_widget(),
                ));
                assert_eq!(size_of(&tester, only::<TabBar>(&tester)), Size::ZERO);
                controller.animate_to(&mut tester.cell.borrow_mut(), 1, None, None);
                tester.pump(Duration::ZERO);
                tester.pump_and_settle();
            });
        }

        /// tabs_utils.dart's `TabBarAsyncImageIndicatorDecoration`: its painters count their paints,
        /// and the first paint of each asks for a repaint later, as an image that finishes loading
        /// does.
        ///
        /// Dart's painter calls `onChanged` from `Future.delayed(Duration.zero)`; a `BoxPainter`
        /// here paints without the `App`, so it leaves the call in `pending`, and the case makes it
        /// where Dart's event queue would, between the frames.
        #[derive(Clone, Default)]
        struct TabBarAsyncImageIndicatorDecoration {
            paint_count: Rc<Cell<usize>>,
            pending: Rc<RefCell<Vec<waft_foundation::VoidCallback>>>,
        }

        impl TabBarAsyncImageIndicatorDecoration {
            fn run_pending(&self, app: &mut App) {
                let pending = std::mem::take(&mut *self.pending.borrow_mut());
                for on_changed in pending {
                    on_changed(app);
                }
            }
        }

        impl fmt::Debug for TabBarAsyncImageIndicatorDecoration {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("TabBarAsyncImageIndicatorDecoration")
            }
        }

        impl Decoration for TabBarAsyncImageIndicatorDecoration {
            fn create_box_painter(
                &self,
                on_changed: Option<waft_foundation::VoidCallback>,
            ) -> Box<dyn BoxPainter> {
                Box::new(TabBarAsyncImageIndicatorBoxPainter {
                    on_changed,
                    paint_count: Rc::clone(&self.paint_count),
                    pending: Rc::clone(&self.pending),
                    image_painted: false,
                })
            }

            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        /// tabs_utils.dart's `TabBarAsyncImageIndicatorBoxPainter`.
        struct TabBarAsyncImageIndicatorBoxPainter {
            on_changed: Option<waft_foundation::VoidCallback>,
            paint_count: Rc<Cell<usize>>,
            pending: Rc<RefCell<Vec<waft_foundation::VoidCallback>>>,
            image_painted: bool,
        }

        impl BoxPainter for TabBarAsyncImageIndicatorBoxPainter {
            fn paint(
                &mut self,
                canvas: &mut Canvas,
                offset: Offset,
                configuration: &ImageConfiguration,
            ) {
                self.paint_count.set(self.paint_count.get() + 1);

                // Simulate async image loading on first paint.
                if !self.image_painted
                    && let Some(on_changed) = &self.on_changed
                {
                    self.image_painted = true;
                    self.pending.borrow_mut().push(Rc::clone(on_changed));
                }

                // Paint a simple rectangle to indicate the indicator was painted.
                let size = configuration.size.expect("a size");
                canvas.draw_rect(
                    Rect::from_ltwh(offset.dx(), size.height() - 2.0, size.width(), 2.0),
                    &Paint::from_color(Colors::BLUE.primary().into()),
                );
            }

            fn on_changed(&self) -> Option<&dyn Fn(&mut App)> {
                self.on_changed.as_deref()
            }
        }

        // 'TabBar indicator image should be rendered at initialIndex for the first time'.
        #[test]
        fn an_indicator_whose_image_loads_later_repaints_on_the_initial_tab() {
            with_main_thread_stack(|| {
                let mut tester = tester();
                let decoration = TabBarAsyncImageIndicatorDecoration::default();
                let theme = theme(&tester, None, None);
                tester.pump_widget(material_app(
                    theme,
                    DefaultTabController::new(
                        3,
                        scaffold_with_tab_bar(
                            None,
                            TabBar::new(text_tabs(&["One", "Two", "Three"]))
                                .indicator(decoration.clone()),
                            Some(
                                TabBarView::new(["Page One", "Page Two", "Page Three"].map(
                                    |page| Center::new().child(Text::new(page)).into_widget(),
                                ))
                                .into_widget(),
                            ),
                        ),
                    )
                    .into_widget(),
                ));

                // Initial paint - indicator should be painted once.
                assert_eq!(decoration.paint_count.get(), 1);

                // Pump with duration to allow the event queue (async image load simulation) to complete.
                decoration.run_pending(&mut tester.cell.borrow_mut());
                tester.pump(Duration::from_millis(1));

                // After async image loads, the indicator should be repainted.
                // This verifies that the markNeedsPaint callback properly triggers a repaint.
                assert!(
                    decoration.paint_count.get() > 1,
                    "Indicator should be repainted after async image loads"
                );

                // Verify the indicator repaints when switching tabs.
                let initial_paint_count = decoration.paint_count.get();
                let two = text(&tester, "Two");
                tap(&mut tester, two);
                tester.pump_and_settle();

                assert!(
                    decoration.paint_count.get() > initial_paint_count,
                    "Indicator should repaint when switching tabs"
                );
            });
        }
    }
}
