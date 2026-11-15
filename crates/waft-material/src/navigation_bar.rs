//! Flutter counterpart: material_ui's `navigation_bar.dart` (and its part
//! `generated/navigation_bar_defaults_m3.g.dart`).

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, AnimationBehavior, AnimationController, AnimationRef, AnimationStatus,
    AnimationStatusListener, AnyAnimation, Curve, CurveTween, Curves, Drive, Tween,
    k_always_complete_animation, k_always_dismissed_animation,
};
use waft_embedder::{Color, Offset, Radius, SemanticsRole, Size};
use waft_foundation::{
    App, Handle, K_IS_WEB, Listenable, Listener, OptionRetained, Ref, ValueChanged, ValueKey,
};
use waft_painting::{
    AlignmentGeometry, BorderRadius, BorderSide, EdgeInsetsGeometry, RoundedRectangleBorder,
    ShapeBorderRef, ShapeDecoration, TextStyle,
};
use waft_rendering::{
    AnyRenderBox, BoxConstraints, ChildLayoutId, MultiChildLayoutChildren, MultiChildLayoutDelegate,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    AnimatedBuilder, BuildContext, CustomMultiChildLayout, Expanded, FadeTransition, GlobalKey,
    IconTheme, IconThemeData, InheritedWidget, IntoWidget, KeyRef, KeyedSubtree, LayoutId, Matrix4,
    MediaQuery, MergeSemantics, Padding, Row, SafeArea, Semantics, SingleTickerProviderStateMixin,
    SingleTickerProviderStateMixinData, SizedBox, Stack, State, StateData, StatefulWidget,
    StatelessWidget, StatusTransitionWidget, Text, Transform, TransitionBuilder, WidgetBuilder,
    WidgetRef, WidgetState, WidgetStatePropertyRef, WidgetStates,
};

use crate::colors::Colors;
use crate::elevation_overlay::ElevationOverlay;
use crate::generated::navigation_bar_defaults_m3::NavigationBarDefaultsM3;
use crate::ink_decoration::Ink;
use crate::ink_well::InkResponse;
use crate::material::{Material, RectCallback};
use crate::material_localizations::MaterialLocalizations;
#[allow(deprecated)]
use crate::material_state::MaterialStatePropertyAll;
use crate::navigation_bar_theme::{
    NavigationBarTheme, NavigationBarThemeData, NavigationBarThemeDataGetters,
};
use crate::theme::Theme;
use crate::theme_data_getters::DefaultsContext;
use crate::tooltip::Tooltip;

const K_INDICATOR_HEIGHT: f64 = 32.0;
const K_INDICATOR_WIDTH: f64 = 64.0;
const K_MAX_LABEL_TEXT_SCALE_FACTOR: f64 = 1.3;

/// Material 3 Navigation Bar component.
///
/// Learn more about [`NavigationBar`] on the [Flutter YouTube channel](https://www.youtube.com/watch?v=DVGYddFaLv0).
///
/// Navigation bars offer a persistent and convenient way to switch between
/// primary destinations in an app.
///
/// This widget does not adjust its size with the `ThemeData::visual_density`.
///
/// The `MediaQueryData::text_scaler` does not adjust the size of this widget but
/// rather the size of the [`Tooltip`]s displayed on long presses of the
/// destinations.
///
/// The style for the icons and text are not affected by parent
/// `DefaultTextStyle`s or [`IconTheme`]s but rather controlled by parameters or
/// the [`NavigationBarThemeData`].
///
/// This widget holds a collection of destinations (usually
/// [`NavigationDestination`]s).
///
/// See also:
///
///  * [`NavigationDestination`]
///  * `BottomNavigationBar`
///  * <https://api.flutter.dev/flutter/material/NavigationDestination-class.html>
///  * <https://m3.material.io/components/navigation-bar>
pub struct NavigationBar {
    pub key: Option<KeyRef>,

    /// Determines the transition time for each destination as it goes between
    /// selected and unselected.
    pub animation_duration: Option<Duration>,

    /// Determines which one of the [`destinations`](Self::destinations) is currently selected.
    ///
    /// When this is updated, the destination (from [`destinations`](Self::destinations)) at
    /// [`selected_index`](Self::selected_index) goes from unselected to selected.
    pub selected_index: usize,

    /// The list of destinations (usually [`NavigationDestination`]s) in this
    /// [`NavigationBar`].
    ///
    /// When [`selected_index`](Self::selected_index) is updated, the destination from this
    /// list at [`selected_index`](Self::selected_index) will animate from 0 (unselected) to
    /// 1.0 (selected). When the animation is increasing or completed, the destination is
    /// considered selected, when the animation is decreasing or dismissed, the destination
    /// is considered unselected.
    pub destinations: Vec<WidgetRef>,

    /// Called when one of the [`destinations`](Self::destinations) is selected.
    ///
    /// This callback usually updates the int passed to [`selected_index`](Self::selected_index).
    ///
    /// Upon updating [`selected_index`](Self::selected_index), the [`NavigationBar`] will be
    /// rebuilt.
    pub on_destination_selected: Option<ValueChanged<usize>>,

    /// The color of the [`NavigationBar`] itself.
    ///
    /// If null, `NavigationBarThemeData::background_color` is used. If that
    /// is also null, then if `ThemeData::use_material3` is true, the value is
    /// `ColorScheme::surface_container`. If that is false, the default blends
    /// `ColorScheme::surface` and `ColorScheme::on_surface` using an [`ElevationOverlay`].
    pub background_color: Option<Color>,

    /// The elevation of the [`NavigationBar`] itself.
    ///
    /// If null, `NavigationBarThemeData::elevation` is used. If that
    /// is also null, then if `ThemeData::use_material3` is true then it will
    /// be 3.0 otherwise 0.0.
    pub elevation: Option<f64>,

    /// The color used for the drop shadow to indicate elevation.
    ///
    /// If null, `NavigationBarThemeData::shadow_color` is used. If that
    /// is also null, the default value is `Colors::TRANSPARENT` which
    /// indicates that no drop shadow will be displayed.
    ///
    /// See `Material::shadow_color` for more details on drop shadows.
    pub shadow_color: Option<Color>,

    /// The color used as an overlay on [`background_color`](Self::background_color) to
    /// indicate elevation.
    ///
    /// This is not recommended for use. [Material 3 spec](https://m3.material.io/styles/color/the-color-system/color-roles)
    /// introduced a set of tone-based surfaces and surface containers in its `ColorScheme`,
    /// which provide more flexibility. The intention is to eventually remove surface tint color from
    /// the framework.
    ///
    /// If null, `NavigationBarThemeData::surface_tint_color` is used. If that
    /// is also null, the default value is `Colors::TRANSPARENT`.
    ///
    /// See `Material::surface_tint_color` for more details on how this
    /// overlay is applied.
    pub surface_tint_color: Option<Color>,

    /// The color of the [`indicator_shape`](Self::indicator_shape) when this destination is
    /// selected.
    ///
    /// If null, `NavigationBarThemeData::indicator_color` is used. If that
    /// is also null and `ThemeData::use_material3` is true, `ColorScheme::secondary_container`
    /// is used. Otherwise, `ColorScheme::secondary` with an opacity of 0.24 is used.
    pub indicator_color: Option<Color>,

    /// The shape of the selected indicator.
    ///
    /// If null, `NavigationBarThemeData::indicator_shape` is used. If that
    /// is also null and `ThemeData::use_material3` is true, `StadiumBorder` is used.
    /// Otherwise, [`RoundedRectangleBorder`] with a circular border radius of 16 is used.
    pub indicator_shape: Option<ShapeBorderRef>,

    /// The height of the [`NavigationBar`] itself.
    ///
    /// If this is used in `Scaffold::bottom_navigation_bar` and the scaffold is
    /// full-screen, the safe area padding is also added to the height
    /// automatically.
    ///
    /// The height does not adjust with `ThemeData::visual_density` or
    /// `MediaQueryData::text_scaler` as this component loses usability at
    /// larger and smaller sizes due to the truncating of labels or smaller tap
    /// targets.
    ///
    /// If null, `NavigationBarThemeData::height` is used. If that
    /// is also null, the default is 80.
    pub height: Option<f64>,

    /// Defines how the [`destinations`](Self::destinations)' labels will be laid out and when
    /// they'll be displayed.
    ///
    /// Can be used to show all labels, show only the selected label, or hide all
    /// labels.
    ///
    /// If null, `NavigationBarThemeData::label_behavior` is used. If that
    /// is also null, the default is
    /// [`NavigationDestinationLabelBehavior::AlwaysShow`].
    pub label_behavior: Option<NavigationDestinationLabelBehavior>,

    /// The highlight color that's typically used to indicate that
    /// the [`NavigationDestination`] is focused, hovered, or pressed.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The text style of the label.
    ///
    /// If null, `NavigationBarThemeData::label_text_style` is used. If that
    /// is also null, the default text style is `TextTheme::label_medium` with
    /// `ColorScheme::on_surface` when the destination is selected, and
    /// `ColorScheme::on_surface_variant` when the destination is unselected, and
    /// `ColorScheme::on_surface_variant` with an opacity of 0.38 when the
    /// destination is disabled.
    ///
    /// If `ThemeData::use_material3` is false, then the default text style is
    /// `TextTheme::label_small` with `ColorScheme::on_surface`.
    pub label_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,

    /// The padding around the `NavigationDestination::label` widget.
    ///
    /// When [`label_padding`](Self::label_padding) is null,
    /// `NavigationBarThemeData::label_padding` is used. If that is also null, the default
    /// padding is 4 pixels on the top.
    pub label_padding: Option<EdgeInsetsGeometry>,

    /// Specifies whether the underlying [`SafeArea`] should maintain the bottom
    /// `MediaQueryData::view_padding` instead of the bottom `MediaQueryData::padding`.
    ///
    /// When true, this will prevent the [`NavigationBar`] from shifting when opening a
    /// software keyboard due to the change in the padding value, especially when the
    /// app uses `SystemUiMode::EdgeToEdge`, which renders the system bars over the
    /// application instead of outside it.
    ///
    /// Defaults to false.
    ///
    /// See also:
    ///
    ///  * `SafeArea::maintain_bottom_view_padding`, which specifies whether the [`SafeArea`]
    ///    should maintain the bottom `MediaQueryData::view_padding`.
    ///  * `SystemUiMode::EdgeToEdge`, which sets a fullscreen display with status and
    ///    navigation elements rendered over the application.
    pub maintain_bottom_view_padding: bool,
}

impl NavigationBar {
    /// Creates a Material 3 Navigation Bar component.
    ///
    /// The value of [`destinations`](Self::destinations) must be a list of two or more
    /// [`NavigationDestination`] values.
    pub fn new(destinations: impl IntoIterator<Item = WidgetRef>) -> NavigationBar {
        NavigationBar {
            key: None,
            animation_duration: None,
            selected_index: 0,
            destinations: destinations.into_iter().collect(),
            on_destination_selected: None,
            background_color: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            indicator_color: None,
            indicator_shape: None,
            height: None,
            label_behavior: None,
            overlay_color: None,
            label_text_style: None,
            label_padding: None,
            maintain_bottom_view_padding: false,
        }
    }

    /// Dart `NavigationBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> NavigationBar {
        self.key = Some(key);
        self
    }

    /// Dart `NavigationBar(animationDuration:)`.
    pub fn animation_duration(mut self, animation_duration: Duration) -> NavigationBar {
        self.animation_duration = Some(animation_duration);
        self
    }

    /// Dart `NavigationBar(selectedIndex:)`.
    pub fn selected_index(mut self, selected_index: usize) -> NavigationBar {
        self.selected_index = selected_index;
        self
    }

    /// Dart `NavigationBar(onDestinationSelected:)`.
    pub fn on_destination_selected(
        mut self,
        on_destination_selected: impl Fn(&mut App, usize) + 'static,
    ) -> NavigationBar {
        self.on_destination_selected = Some(Rc::new(on_destination_selected));
        self
    }

    /// Dart `NavigationBar(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> NavigationBar {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `NavigationBar(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> NavigationBar {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `NavigationBar(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> NavigationBar {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `NavigationBar(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> NavigationBar {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `NavigationBar(indicatorColor:)`.
    pub fn indicator_color(mut self, indicator_color: Color) -> NavigationBar {
        self.indicator_color = Some(indicator_color);
        self
    }

    /// Dart `NavigationBar(indicatorShape:)`.
    pub fn indicator_shape(mut self, indicator_shape: impl Into<ShapeBorderRef>) -> NavigationBar {
        self.indicator_shape = Some(indicator_shape.into());
        self
    }

    /// Dart `NavigationBar(height:)`.
    pub fn height(mut self, height: f64) -> NavigationBar {
        self.height = Some(height);
        self
    }

    /// Dart `NavigationBar(labelBehavior:)`.
    pub fn label_behavior(
        mut self,
        label_behavior: NavigationDestinationLabelBehavior,
    ) -> NavigationBar {
        self.label_behavior = Some(label_behavior);
        self
    }

    /// Dart `NavigationBar(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> NavigationBar {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `NavigationBar(labelTextStyle:)`.
    pub fn label_text_style(
        mut self,
        label_text_style: WidgetStatePropertyRef<Option<TextStyle>>,
    ) -> NavigationBar {
        self.label_text_style = Some(label_text_style);
        self
    }

    /// Dart `NavigationBar(labelPadding:)`.
    pub fn label_padding(mut self, label_padding: impl Into<EdgeInsetsGeometry>) -> NavigationBar {
        self.label_padding = Some(label_padding.into());
        self
    }

    /// Dart `NavigationBar(maintainBottomViewPadding:)`.
    pub fn maintain_bottom_view_padding(
        mut self,
        maintain_bottom_view_padding: bool,
    ) -> NavigationBar {
        self.maintain_bottom_view_padding = maintain_bottom_view_padding;
        self
    }

    fn handle_tap(on_destination_selected: Option<&ValueChanged<usize>>, index: usize) -> Listener {
        match on_destination_selected.cloned() {
            Some(on_destination_selected) => {
                Listener::new(move |app| on_destination_selected(app, index))
            }
            None => Listener::new(|_app| {}),
        }
    }
}

impl Debug for NavigationBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NavigationBar")
            .field("animation_duration", &self.animation_duration)
            .field("selected_index", &self.selected_index)
            .field("destinations", &self.destinations)
            .field("background_color", &self.background_color)
            .field("elevation", &self.elevation)
            .field("shadow_color", &self.shadow_color)
            .field("surface_tint_color", &self.surface_tint_color)
            .field("indicator_color", &self.indicator_color)
            .field("indicator_shape", &self.indicator_shape)
            .field("height", &self.height)
            .field("label_behavior", &self.label_behavior)
            .field("label_padding", &self.label_padding)
            .field(
                "maintain_bottom_view_padding",
                &self.maintain_bottom_view_padding,
            )
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for NavigationBar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.destinations.len() >= 2);
        debug_assert!(self.selected_index < self.destinations.len());
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let defaults = defaults_for(app, context);

        let navigation_bar_theme = NavigationBarTheme::of(app, context);
        let effective_height = self
            .height
            .or(navigation_bar_theme.height)
            .or_else(|| defaults.height(app))
            .expect("the defaults give a height");
        let effective_label_behavior = self
            .label_behavior
            .or(navigation_bar_theme.label_behavior)
            .or_else(|| defaults.label_behavior(app))
            .expect("the defaults give a label behavior");

        let mut material = Material::new()
            .color(
                self.background_color
                    .or(navigation_bar_theme.background_color)
                    .or_else(|| defaults.background_color(app))
                    .expect("the defaults give a background color"),
            )
            .elevation(
                self.elevation
                    .or(navigation_bar_theme.elevation)
                    .or_else(|| defaults.elevation(app))
                    .expect("the defaults give an elevation"),
            );
        material.shadow_color = self
            .shadow_color
            .or(navigation_bar_theme.shadow_color)
            .or_else(|| defaults.shadow_color(app));
        material.surface_tint_color = self
            .surface_tint_color
            .or(navigation_bar_theme.surface_tint_color)
            .or_else(|| defaults.surface_tint_color(app));

        let destination_count = self.destinations.len();
        let destinations = (0..destination_count).map(|i| {
            let selected_index = self.selected_index;
            let indicator_color = self.indicator_color;
            let indicator_shape = self.indicator_shape.clone();
            let overlay_color = self.overlay_color.clone();
            let on_destination_selected = self.on_destination_selected.clone();
            let label_text_style = self.label_text_style.clone();
            let label_padding = self.label_padding;
            let destination = self.destinations[i].clone();
            Expanded::new(
                MergeSemantics::new().child(
                    Semantics::new()
                        .role(SemanticsRole::Tab)
                        .selected(i == selected_index)
                        .child(
                            SelectableAnimatedBuilder::new(
                                i == selected_index,
                                move |_app, _context, animation| {
                                    NavigationDestinationInfo {
                                        index: i,
                                        selected_index,
                                        total_number_of_destinations: destination_count,
                                        selected_animation: animation,
                                        label_behavior: effective_label_behavior,
                                        indicator_color,
                                        indicator_shape: indicator_shape.clone(),
                                        overlay_color: overlay_color.clone(),
                                        on_tap: NavigationBar::handle_tap(
                                            on_destination_selected.as_ref(),
                                            i,
                                        ),
                                        label_text_style: label_text_style.clone(),
                                        label_padding,
                                        child: destination.clone(),
                                    }
                                    .into_widget()
                                },
                            )
                            .duration(
                                self.animation_duration
                                    .unwrap_or(Duration::from_millis(500)),
                            ),
                        ),
                ),
            )
            .into_widget()
        });

        material
            .child(
                SafeArea::new(
                    Semantics::new()
                        .role(SemanticsRole::TabBar)
                        .explicit_child_nodes(true)
                        .container(true)
                        .child(
                            SizedBox::new()
                                .height(effective_height)
                                .child(Row::new().children(destinations.collect::<Vec<_>>())),
                        ),
                )
                .maintain_bottom_view_padding(self.maintain_bottom_view_padding),
            )
            .into_widget()
    }
}

/// Specifies when each `NavigationDestination`'s label should appear.
///
/// This is used to determine the behavior of `NavigationBar`'s destinations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavigationDestinationLabelBehavior {
    /// Always shows all of the labels under each navigation bar destination,
    /// selected and unselected.
    AlwaysShow,

    /// Never shows any of the labels under the navigation bar destinations,
    /// regardless of selected vs unselected.
    AlwaysHide,

    /// Only shows the labels of the selected navigation bar destination.
    ///
    /// When a destination is unselected, the label will be faded out, and the
    /// icon will be centered.
    ///
    /// When a destination is selected, the label will fade in and the label and
    /// icon will slide up so that they are both centered.
    OnlyShowSelected,
}

/// A Material 3 [`NavigationBar`] destination.
///
/// Displays a label below an icon. Use with [`NavigationBar::destinations`].
///
/// See also:
///
///  * [`NavigationBar`], for an interactive code sample.
#[derive(Debug)]
pub struct NavigationDestination {
    pub key: Option<KeyRef>,

    /// The [`WidgetRef`] (usually an `Icon`) that's displayed for this
    /// [`NavigationDestination`].
    ///
    /// The icon will use `NavigationBarThemeData::icon_theme`. If this is
    /// null, the default [`IconThemeData`] would use a size of 24.0 and
    /// `ColorScheme::on_surface`.
    pub icon: WidgetRef,

    /// The optional [`WidgetRef`] (usually an `Icon`) that's displayed when this
    /// [`NavigationDestination`] is selected.
    ///
    /// If [`selected_icon`](Self::selected_icon) is non-null, the destination will fade from
    /// [`icon`](Self::icon) to [`selected_icon`](Self::selected_icon) when this destination
    /// goes from unselected to selected.
    ///
    /// The icon will use `NavigationBarThemeData::icon_theme` with
    /// [`WidgetState::Selected`]. If this is null, the default [`IconThemeData`]
    /// would use a size of 24.0 and `ColorScheme::on_surface`.
    pub selected_icon: Option<WidgetRef>,

    /// The text label that appears below the icon of this
    /// [`NavigationDestination`].
    ///
    /// The accompanying [`Text`] widget will use `NavigationBarThemeData::label_text_style`.
    /// If this is null, the default text style will use `TextTheme::label_medium` with
    /// `ColorScheme::on_surface` when the destination is selected and
    /// `ColorScheme::on_surface_variant` when the destination is unselected. If
    /// `ThemeData::use_material3` is false, then the default text style will use
    /// `TextTheme::label_small` with `ColorScheme::on_surface`.
    pub label: String,

    /// The text to display in the tooltip for this [`NavigationDestination`], when
    /// the user long presses the destination.
    ///
    /// If [`tooltip`](Self::tooltip) is an empty string, no tooltip will be used.
    ///
    /// Defaults to null, in which case the [`label`](Self::label) text will be used.
    pub tooltip: Option<String>,

    /// Indicates that this destination is selectable.
    ///
    /// Defaults to true.
    pub enabled: bool,
}

impl NavigationDestination {
    /// Creates a navigation bar destination with an icon and a label, to be used
    /// in the [`NavigationBar::destinations`].
    pub fn new<K>(icon: impl IntoWidget<K>, label: impl Into<String>) -> NavigationDestination {
        NavigationDestination {
            key: None,
            icon: icon.into_widget(),
            selected_icon: None,
            label: label.into(),
            tooltip: None,
            enabled: true,
        }
    }

    /// Dart `NavigationDestination(key:)`.
    pub fn key(mut self, key: KeyRef) -> NavigationDestination {
        self.key = Some(key);
        self
    }

    /// Dart `NavigationDestination(selectedIcon:)`.
    pub fn selected_icon<K>(mut self, selected_icon: impl IntoWidget<K>) -> NavigationDestination {
        self.selected_icon = Some(selected_icon.into_widget());
        self
    }

    /// Dart `NavigationDestination(tooltip:)`.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> NavigationDestination {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Dart `NavigationDestination(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> NavigationDestination {
        self.enabled = enabled;
        self
    }
}

impl StatelessWidget for NavigationDestination {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let info = NavigationDestinationInfo::of(app, context);
        let selected_state: WidgetStates = [WidgetState::Selected].into_iter().collect();
        let unselected_state = WidgetStates::new();
        let disabled_state: WidgetStates = [WidgetState::Disabled].into_iter().collect();

        let navigation_bar_theme = NavigationBarTheme::of(app, context);
        let defaults = defaults_for(app, context);
        let animation = info.selected_animation.clone();

        let enabled = self.enabled;
        let icon = self.icon.clone();
        let selected_icon = self.selected_icon.clone();
        let build_icon = {
            let navigation_bar_theme = navigation_bar_theme.clone();
            let defaults = Rc::clone(&defaults);
            let info = info.clone();
            let animation = animation.clone();
            let (selected_state, unselected_state, disabled_state) = (
                selected_state.clone(),
                unselected_state.clone(),
                disabled_state.clone(),
            );
            move |app: &mut App, _context: BuildContext| {
                let resolve_icon_theme = |app: &mut App, states: &WidgetStates| {
                    navigation_bar_theme
                        .icon_theme
                        .as_ref()
                        .and_then(|icon_theme| icon_theme.resolve(states))
                        .unwrap_or_else(|| {
                            defaults
                                .icon_theme(app)
                                .expect("the defaults give an icon theme")
                                .resolve(states)
                                .expect("the default icon theme resolves")
                        })
                };
                let selected_icon_theme = resolve_icon_theme(app, &selected_state);
                let unselected_icon_theme = resolve_icon_theme(app, &unselected_state);
                let disabled_icon_theme = resolve_icon_theme(app, &disabled_state);

                let selected_icon_widget = IconTheme::merge(
                    None,
                    if enabled {
                        selected_icon_theme
                    } else {
                        disabled_icon_theme.clone()
                    },
                    selected_icon.clone().unwrap_or_else(|| icon.clone()),
                );
                let unselected_icon_widget = IconTheme::merge(
                    None,
                    if enabled {
                        unselected_icon_theme
                    } else {
                        disabled_icon_theme
                    },
                    icon.clone(),
                );

                Stack::new()
                    .alignment(AlignmentGeometry::CENTER)
                    .children([
                        NavigationIndicator::new(animation.clone())
                            .color(
                                info.indicator_color
                                    .or(navigation_bar_theme.indicator_color)
                                    .or_else(|| defaults.indicator_color(app))
                                    .expect("the defaults give an indicator color"),
                            )
                            .shape(
                                info.indicator_shape
                                    .clone()
                                    .or_else(|| navigation_bar_theme.indicator_shape.clone())
                                    .or_else(|| defaults.indicator_shape(app))
                                    .expect("the defaults give an indicator shape"),
                            )
                            .into_widget(),
                        StatusTransitionWidgetBuilder::new(animation.clone(), {
                            let animation = animation.clone();
                            move |app, _context, _child| {
                                if animation.is_forward_or_completed(app) {
                                    selected_icon_widget.clone()
                                } else {
                                    unselected_icon_widget.clone()
                                }
                            }
                        })
                        .into_widget(),
                    ])
                    .into_widget()
            }
        };
        let label = self.label.clone();
        let build_label = move |app: &mut App, _context: BuildContext| {
            let resolve_label_text_style = |app: &mut App, states: &WidgetStates| {
                info.label_text_style
                    .as_ref()
                    .and_then(|style| style.resolve(states))
                    .or_else(|| {
                        navigation_bar_theme
                            .label_text_style
                            .as_ref()
                            .and_then(|style| style.resolve(states))
                    })
                    .or_else(|| {
                        defaults
                            .label_text_style(app)
                            .expect("the defaults give a label text style")
                            .resolve(states)
                    })
            };
            let effective_selected_label_text_style =
                resolve_label_text_style(app, &selected_state);
            let effective_unselected_label_text_style =
                resolve_label_text_style(app, &unselected_state);
            let effective_disabled_label_text_style =
                resolve_label_text_style(app, &disabled_state);
            let label_padding = info
                .label_padding
                .or(navigation_bar_theme.label_padding)
                .or_else(|| defaults.label_padding(app))
                .expect("the defaults give a label padding");

            let text_style = if enabled {
                if animation.is_forward_or_completed(app) {
                    effective_selected_label_text_style
                } else {
                    effective_unselected_label_text_style
                }
            } else {
                effective_disabled_label_text_style
            };

            let mut text = Text::new(label.clone());
            if let Some(text_style) = text_style {
                text = text.style(text_style);
            }
            Padding::new(label_padding)
                .child(
                    // Set maximum text scale factor to K_MAX_LABEL_TEXT_SCALE_FACTOR for the
                    // label to keep the visual hierarchy the same even with larger font
                    // sizes. To opt out, wrap the `label` widget in a `MediaQuery` widget
                    // with a different `TextScaler`.
                    MediaQuery::with_clamped_text_scaling(
                        None,
                        0.0,
                        K_MAX_LABEL_TEXT_SCALE_FACTOR,
                        text,
                    ),
                )
                .into_widget()
        };

        NavigationDestinationBuilder {
            build_icon: Rc::new(build_icon),
            build_label: Rc::new(build_label),
            label: self.label.clone(),
            tooltip: self.tooltip.clone(),
            enabled: self.enabled,
        }
        .into_widget()
    }
}

/// Widget that handles the semantics and layout of a navigation bar
/// destination.
///
/// Prefer [`NavigationDestination`] over this widget, as it is a simpler
/// (although less customizable) way to get navigation bar destinations.
///
/// The icon and label of this destination are built with [`build_icon`](Self::build_icon)
/// and [`build_label`](Self::build_label). They should build the unselected and selected icon
/// and label according to [`NavigationDestinationInfo::selected_animation`], where an
/// animation value of 0 is unselected and 1 is selected.
///
/// See [`NavigationDestination`] for an example.
struct NavigationDestinationBuilder {
    /// Builds the icon for a destination in a [`NavigationBar`].
    ///
    /// To animate between unselected and selected, build the icon based on
    /// [`NavigationDestinationInfo::selected_animation`]. When the animation is 0,
    /// the destination is unselected, when the animation is 1, the destination is
    /// selected.
    ///
    /// The destination is considered selected as soon as the animation is
    /// increasing or completed, and it is considered unselected as soon as the
    /// animation is decreasing or dismissed.
    build_icon: WidgetBuilder,

    /// Builds the label for a destination in a [`NavigationBar`].
    ///
    /// To animate between unselected and selected, build the icon based on
    /// [`NavigationDestinationInfo::selected_animation`]. When the animation is
    /// 0, the destination is unselected, when the animation is 1, the destination
    /// is selected.
    ///
    /// The destination is considered selected as soon as the animation is
    /// increasing or completed, and it is considered unselected as soon as the
    /// animation is decreasing or dismissed.
    build_label: WidgetBuilder,

    /// The text value of what is in the label widget, this is required for
    /// semantics so that screen readers and tooltips can read the proper label.
    label: String,

    /// The text to display in the tooltip for this [`NavigationDestination`], when
    /// the user long presses the destination.
    ///
    /// If [`tooltip`](Self::tooltip) is an empty string, no tooltip will be used.
    ///
    /// Defaults to null, in which case the [`label`](Self::label) text will be used.
    tooltip: Option<String>,

    /// Indicates that this destination is selectable.
    ///
    /// Defaults to true.
    enabled: bool,
}

impl Debug for NavigationDestinationBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_NavigationDestinationBuilder")
            .field("label", &self.label)
            .field("tooltip", &self.tooltip)
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for NavigationDestinationBuilder {
    type State = NavigationDestinationBuilderState;

    fn create_state(&self) -> NavigationDestinationBuilderState {
        NavigationDestinationBuilderState {
            state: StateData::new(),
            icon_key: GlobalKey::new(),
        }
    }
}

/// Dart's `_NavigationDestinationBuilderState`.
struct NavigationDestinationBuilderState {
    state: StateData<NavigationDestinationBuilder>,
    icon_key: GlobalKey,
}

impl State for NavigationDestinationBuilderState {
    type Widget = NavigationDestinationBuilder;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let info = NavigationDestinationInfo::of(app, context);
        let navigation_bar_theme = NavigationBarTheme::of(app, context);
        let defaults = defaults_for(app, context);

        let icon_key = app.get(self).icon_key.clone();
        let widget = self.widget(app);
        let enabled = widget.enabled;
        let message = widget
            .tooltip
            .clone()
            .unwrap_or_else(|| widget.label.clone());
        let build_icon = Rc::clone(&widget.build_icon);
        let build_label = Rc::clone(&widget.build_label);

        let mut ink_well = IndicatorInkWell::new(icon_key.clone(), info.label_behavior);
        if let Some(custom_border) = info
            .indicator_shape
            .clone()
            .or_else(|| navigation_bar_theme.indicator_shape.clone())
            .or_else(|| defaults.indicator_shape(app))
        {
            ink_well = ink_well.custom_border(custom_border);
        }
        if let Some(overlay_color) = info
            .overlay_color
            .clone()
            .or_else(|| navigation_bar_theme.overlay_color.clone())
        {
            ink_well = ink_well.overlay_color(overlay_color);
        }
        if enabled {
            ink_well = ink_well.on_tap(info.on_tap.clone());
        }
        let icon = build_icon(app, context);
        let label = build_label(app, context);

        NavigationBarDestinationSemantics::new(
            enabled,
            NavigationBarDestinationTooltip::new(
                message,
                ink_well.child(
                    Row::new().children([Expanded::new(NavigationBarDestinationLayout {
                        icon,
                        icon_key,
                        label,
                    })
                    .into_widget()]),
                ),
            ),
        )
        .into_widget()
    }
}

/// Dart's `_IndicatorInkWell extends InkResponse`: an [`InkResponse`] whose highlight and
/// splash are clipped to the icon's rectangle.
#[derive(Debug)]
struct IndicatorInkWell {
    ink_response: InkResponse,
    #[allow(dead_code, reason = "Dart keeps the field, which nothing reads")]
    icon_key: GlobalKey,
    #[allow(dead_code, reason = "Dart keeps the field, which nothing reads")]
    label_behavior: NavigationDestinationLabelBehavior,
}

impl IndicatorInkWell {
    fn new(
        icon_key: GlobalKey,
        label_behavior: NavigationDestinationLabelBehavior,
    ) -> IndicatorInkWell {
        let rect_icon_key = icon_key.clone();
        let ink_response = InkResponse::new()
            .contained_ink_well(true)
            .highlight_color(Colors::TRANSPARENT)
            .override_get_rect_callback(move |_app, reference_box| {
                Some(IndicatorInkWell::get_rect_callback(
                    rect_icon_key.clone(),
                    reference_box,
                ))
            });
        IndicatorInkWell {
            ink_response,
            icon_key,
            label_behavior,
        }
    }

    /// Dart's override of `getRectCallback`.
    fn get_rect_callback(icon_key: GlobalKey, reference_box: AnyRenderBox) -> RectCallback {
        Rc::new(move |app| {
            let icon_box = icon_key
                .current_context(app)
                .expect("the icon is mounted")
                .find_render_object(app)
                .and_then(|render_object| render_object.as_box())
                .expect("the icon has a render box");
            let icon_rect = icon_box.local_to_global(app, Offset::ZERO, None) & icon_box.size(app);
            reference_box.global_to_local(app, icon_rect.top_left(), None) & icon_box.size(app)
        })
    }

    /// Dart `_IndicatorInkWell(overlayColor:)`.
    fn overlay_color(mut self, overlay_color: WidgetStatePropertyRef<Option<Color>>) -> Self {
        self.ink_response = self.ink_response.overlay_color(overlay_color);
        self
    }

    /// Dart `_IndicatorInkWell(customBorder:)`.
    fn custom_border(mut self, custom_border: ShapeBorderRef) -> Self {
        self.ink_response = self.ink_response.custom_border(custom_border);
        self
    }

    /// Dart `_IndicatorInkWell(onTap:)`.
    fn on_tap(mut self, on_tap: Listener) -> Self {
        self.ink_response = self.ink_response.on_tap(on_tap);
        self
    }

    /// Dart `_IndicatorInkWell(child:)`.
    fn child<K>(mut self, child: impl IntoWidget<K>) -> Self {
        self.ink_response = self.ink_response.child(child);
        self
    }
}

impl StatelessWidget for IndicatorInkWell {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        self.ink_response.clone().into_widget()
    }
}

/// Inherited widget for passing data from the [`NavigationBar`] to the
/// [`NavigationBar::destinations`] children widgets.
///
/// Useful for building navigation destinations using:
/// `NavigationDestinationInfo::of(app, context)`.
#[derive(Clone)]
struct NavigationDestinationInfo {
    /// Which destination index is this in the navigation bar.
    ///
    /// For example:
    ///
    /// ```text
    /// NavigationBar::new([
    ///     // This is destination index 0.
    ///     NavigationDestination::new(Icon::new(Some(Icons::surfing())), "Surfing").into_widget(),
    ///     // This is destination index 1.
    ///     NavigationDestination::new(Icon::new(Some(Icons::support())), "Support").into_widget(),
    ///     // This is destination index 2.
    ///     NavigationDestination::new(Icon::new(Some(Icons::local_hospital())), "Hospital")
    ///         .into_widget(),
    /// ])
    /// ```
    ///
    /// This is required for semantics, so that each destination can have a label
    /// "Tab 1 of 3", for example.
    index: usize,

    /// This is the index of the currently selected destination.
    ///
    /// This is required for `IndicatorInkWell` to apply label padding to ripple animations
    /// when label behavior is [`NavigationDestinationLabelBehavior::OnlyShowSelected`].
    #[allow(dead_code, reason = "Dart keeps the field, which nothing reads")]
    selected_index: usize,

    /// How many total destinations are in this navigation bar.
    ///
    /// This is required for semantics, so that each destination can have a label
    /// "Tab 1 of 4", for example.
    total_number_of_destinations: usize,

    /// Indicates whether or not this destination is selected, from 0 (unselected)
    /// to 1 (selected).
    selected_animation: AnimationRef<f64>,

    /// Determines the behavior for how the labels will layout.
    ///
    /// Can be used to show all labels (the default), show only the selected
    /// label, or hide all labels.
    label_behavior: NavigationDestinationLabelBehavior,

    /// The color of the selection indicator.
    ///
    /// This is used by destinations to override the indicator color.
    indicator_color: Option<Color>,

    /// The shape of the selection indicator.
    ///
    /// This is used by destinations to override the indicator shape.
    indicator_shape: Option<ShapeBorderRef>,

    /// The highlight color that's typically used to indicate that
    /// the [`NavigationDestination`] is focused, hovered, or pressed.
    ///
    /// This is used by destinations to override the overlay color.
    overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The callback that should be called when this destination is tapped.
    ///
    /// This is computed by calling [`NavigationBar::on_destination_selected`]
    /// with [`index`](Self::index) passed in.
    on_tap: Listener,

    /// The text style of the label.
    label_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,

    /// The padding around the label.
    ///
    /// Defaults to a padding of 4 pixels on the top.
    label_padding: Option<EdgeInsetsGeometry>,

    child: WidgetRef,
}

impl NavigationDestinationInfo {
    /// Returns a non null [`NavigationDestinationInfo`].
    ///
    /// This will return an error if called with no [`NavigationDestinationInfo`]
    /// ancestor.
    ///
    /// Used by widgets that are implementing a navigation destination info to
    /// get information like the selected animation and destination number.
    fn of(app: &mut App, context: BuildContext) -> NavigationDestinationInfo {
        let result = context
            .depend_on_inherited_widget_of_exact_type::<NavigationDestinationInfo>(app)
            .cloned();
        debug_assert!(
            result.is_some(),
            "Navigation destinations need a _NavigationDestinationInfo parent, which is usually \
             provided by NavigationBar."
        );
        result.expect("a _NavigationDestinationInfo above the destination")
    }
}

impl Debug for NavigationDestinationInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_NavigationDestinationInfo")
            .field("index", &self.index)
            .field("selected_index", &self.selected_index)
            .field(
                "total_number_of_destinations",
                &self.total_number_of_destinations,
            )
            .field("selected_animation", &self.selected_animation)
            .field("label_behavior", &self.label_behavior)
            .field("indicator_color", &self.indicator_color)
            .field("indicator_shape", &self.indicator_shape)
            .field("label_padding", &self.label_padding)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl InheritedWidget for NavigationDestinationInfo {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &NavigationDestinationInfo) -> bool {
        self.index != old_widget.index
            || self.total_number_of_destinations != old_widget.total_number_of_destinations
            || self.selected_animation != old_widget.selected_animation
            || self.label_behavior != old_widget.label_behavior
            || self.on_tap != old_widget.on_tap
    }
}

/// Selection Indicator for the Material 3 `NavigationBar` and `NavigationRail`
/// components.
///
/// When [`animation`](Self::animation) is 0, the indicator is not present. As
/// [`animation`](Self::animation) grows from 0 to 1, the indicator scales in on the x axis.
///
/// Used in a `Stack` widget behind the icons in the Material 3 Navigation Bar
/// to illuminate the selected destination.
pub struct NavigationIndicator {
    pub key: Option<KeyRef>,

    /// Determines the scale of the indicator.
    ///
    /// When [`animation`](Self::animation) is 0, the indicator is not present. The indicator
    /// scales in as [`animation`](Self::animation) grows from 0 to 1.
    pub animation: AnimationRef<f64>,

    /// The fill color of this indicator.
    ///
    /// If null, defaults to `ColorScheme::secondary`.
    pub color: Option<Color>,

    /// The width of this indicator.
    ///
    /// Defaults to `64`.
    pub width: f64,

    /// The height of this indicator.
    ///
    /// Defaults to `32`.
    pub height: f64,

    /// The border radius of the shape of the indicator.
    ///
    /// This is used to create a `RoundedRectangleBorder` shape for the indicator.
    /// This is ignored if [`shape`](Self::shape) is non-null.
    ///
    /// Defaults to `BorderRadius.circular(16)`.
    pub border_radius: BorderRadius,

    /// The shape of the indicator.
    ///
    /// If non-null this is used as the shape used to draw the background
    /// of the indicator. If null then a `RoundedRectangleBorder` with the
    /// [`border_radius`](Self::border_radius) is used.
    pub shape: Option<ShapeBorderRef>,
}

impl NavigationIndicator {
    /// Builds an indicator, usually used in a stack behind the icon of a
    /// navigation bar destination.
    pub fn new(animation: AnimationRef<f64>) -> NavigationIndicator {
        NavigationIndicator {
            key: None,
            animation,
            color: None,
            width: K_INDICATOR_WIDTH,
            height: K_INDICATOR_HEIGHT,
            border_radius: BorderRadius::all(Radius::circular(16.0)),
            shape: None,
        }
    }

    /// Dart `NavigationIndicator(key:)`.
    pub fn key(mut self, key: KeyRef) -> NavigationIndicator {
        self.key = Some(key);
        self
    }

    /// Dart `NavigationIndicator(color:)`.
    pub fn color(mut self, color: Color) -> NavigationIndicator {
        self.color = Some(color);
        self
    }

    /// Dart `NavigationIndicator(width:)`.
    pub fn width(mut self, width: f64) -> NavigationIndicator {
        self.width = width;
        self
    }

    /// Dart `NavigationIndicator(height:)`.
    pub fn height(mut self, height: f64) -> NavigationIndicator {
        self.height = height;
        self
    }

    /// Dart `NavigationIndicator(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> NavigationIndicator {
        self.border_radius = border_radius;
        self
    }

    /// Dart `NavigationIndicator(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> NavigationIndicator {
        self.shape = Some(shape.into());
        self
    }
}

impl Debug for NavigationIndicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NavigationIndicator")
            .field("animation", &self.animation)
            .field("color", &self.color)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("border_radius", &self.border_radius)
            .field("shape", &self.shape)
            .finish()
    }
}

impl StatelessWidget for NavigationIndicator {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let animation = self.animation.clone();
        let status_animation = animation.clone();
        let width = self.width;
        let height = self.height;
        let border_radius = self.border_radius;
        let shape = self.shape.clone();
        let color = self.color;
        AnimatedBuilder::new(Rc::new(animation.clone()), move |app, context, child| {
            // The scale should be 0 when the animation is unselected, as soon as
            // the animation starts, the scale jumps to 40%, and then animates to
            // 100% along a curve.
            let scale = if animation.is_dismissed(app) {
                0.0
            } else {
                let curved = transform_curve_tween(
                    app,
                    Curves::ease_in_out_cubic_emphasized(),
                    animation.value(app),
                );
                transform_tween(app, 0.4, 1.0, curved)
            };

            // The indicator is drawn by the Ink widget below, which paints on the
            // nearest ancestor Material rather than locally. When the animation
            // ticks without an accompanying pointer interaction (for example, the
            // selected index was changed programmatically), that Material is not
            // otherwise notified and would keep showing the indicator's previous
            // frame. See https://github.com/flutter/flutter/issues/180359.
            Material::of(app, context).mark_needs_paint(app);

            let mut transform = Transform::new(Matrix4::scale(scale as f32, 1.0))
                .alignment(AlignmentGeometry::CENTER);
            if let Some(child) = child {
                transform = transform.child(child.clone());
            }
            transform.into_widget()
        })
        // Fade should be a 100ms animation whenever the parent animation changes
        // direction.
        .child(StatusTransitionWidgetBuilder::new(
            status_animation.clone(),
            move |app, _context, _child| {
                let shape = shape.clone();
                SelectableAnimatedBuilder::new(
                    status_animation.is_forward_or_completed(app),
                    move |app, context, fade_animation| {
                        let shape = shape.clone().unwrap_or_else(|| {
                            ShapeBorderRef::new(RoundedRectangleBorder::new(
                                BorderSide::NONE,
                                border_radius.into(),
                            ))
                        });
                        let color =
                            color.unwrap_or_else(|| Theme::of(app, context).color_scheme.secondary);
                        FadeTransition::new(fade_animation)
                            .child(
                                Ink::new()
                                    .width(width)
                                    .height(height)
                                    .decoration(ShapeDecoration::new(shape).color(color)),
                            )
                            .into_widget()
                    },
                )
                .duration(Duration::from_millis(100))
                .always_do_full_animation(true)
                .into_widget()
            },
        ))
        .into_widget()
    }
}

/// Widget that handles the layout of the icon + label in a navigation bar
/// destination, based on [`NavigationDestinationInfo::label_behavior`] and
/// [`NavigationDestinationInfo::selected_animation`].
///
/// Depending on the [`NavigationDestinationInfo::label_behavior`], the labels
/// will shift and fade accordingly.
#[derive(Debug)]
struct NavigationBarDestinationLayout {
    /// The icon widget that sits on top of the label.
    ///
    /// See [`NavigationDestination::icon`].
    icon: WidgetRef,

    /// The global key for the icon of this destination.
    ///
    /// This is used to determine the position of the icon.
    icon_key: GlobalKey,

    /// The label widget that sits below the icon.
    ///
    /// This widget will sometimes be faded out, depending on
    /// [`NavigationDestinationInfo::selected_animation`].
    ///
    /// See [`NavigationDestination::label`].
    label: WidgetRef,
}

impl StatelessWidget for NavigationBarDestinationLayout {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let icon = self.icon.clone();
        let icon_key = self.icon_key.clone();
        let label = self.label.clone();
        DestinationLayoutAnimationBuilder {
            builder: Rc::new(move |_app, _context, animation| {
                CustomMultiChildLayout::new(Rc::new(NavigationDestinationLayoutDelegate::new(
                    animation.clone(),
                )))
                .children([
                    LayoutId::new(
                        NavigationDestinationLayoutDelegate::icon_id(),
                        // The key is used by the IndicatorInkWell to query the icon position.
                        KeyedSubtree::new(icon.clone()).key(KeyRef::from(icon_key.clone())),
                    )
                    .into_widget(),
                    LayoutId::new(
                        NavigationDestinationLayoutDelegate::label_id(),
                        FadeTransition::new(animation).child(label.clone()),
                    )
                    .into_widget(),
                ])
                .into_widget()
            }),
        }
        .into_widget()
    }
}

/// Determines the appropriate [`Curve`] and [`AnyAnimation`] to use for laying out the
/// [`NavigationDestination`], based on
/// [`NavigationDestinationInfo::label_behavior`].
///
/// The animation controlling the position and fade of the labels differs
/// from the selection animation, depending on the
/// [`NavigationDestinationLabelBehavior`]. This widget determines what
/// animation should be used for the position and fade of the labels.
struct DestinationLayoutAnimationBuilder {
    /// Builds the child of this widget.
    ///
    /// The animation will be the appropriate animation to use for the layout
    /// and fade of the [`NavigationDestination`], either a curve, always
    /// showing (1), or always hiding (0).
    builder: AnimationWidgetBuilder,
}

impl Debug for DestinationLayoutAnimationBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_DestinationLayoutAnimationBuilder")
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for DestinationLayoutAnimationBuilder {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let info = NavigationDestinationInfo::of(app, context);
        match info.label_behavior {
            NavigationDestinationLabelBehavior::AlwaysShow => {
                let animation = k_always_complete_animation(app);
                (self.builder)(app, context, animation)
            }
            NavigationDestinationLabelBehavior::AlwaysHide => {
                let animation = k_always_dismissed_animation(app);
                (self.builder)(app, context, animation)
            }
            NavigationDestinationLabelBehavior::OnlyShowSelected => CurvedAnimationBuilder {
                animation: info.selected_animation,
                curve: Curves::ease_in_out_cubic_emphasized(),
                reverse_curve: Rc::new(Curves::ease_in_out_cubic_emphasized().flipped()),
                builder: Rc::clone(&self.builder),
            }
            .into_widget(),
        }
    }
}

/// Semantics widget for a navigation bar destination.
///
/// Requires a [`NavigationDestinationInfo`] parent (normally provided by the
/// [`NavigationBar`] by default).
///
/// Provides localized semantic labels to the destination, for example, it will
/// read "Home, Tab 1 of 3".
///
/// Used by [`NavigationDestinationBuilder`].
#[derive(Debug)]
struct NavigationBarDestinationSemantics {
    /// Whether this widget is enabled.
    enabled: bool,

    /// The widget that should receive the destination semantics.
    child: WidgetRef,
}

impl NavigationBarDestinationSemantics {
    /// Adds the appropriate semantics for navigation bar destinations to the
    /// [`child`](Self::child).
    fn new<K>(enabled: bool, child: impl IntoWidget<K>) -> NavigationBarDestinationSemantics {
        NavigationBarDestinationSemantics {
            enabled,
            child: child.into_widget(),
        }
    }
}

impl StatelessWidget for NavigationBarDestinationSemantics {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let destination_info = NavigationDestinationInfo::of(app, context);
        let enabled = self.enabled;
        // The AnimationStatusBuilder will make sure that the semantics update to
        // "selected" when the animation status changes.
        StatusTransitionWidgetBuilder::new(
            destination_info.selected_animation,
            move |_app, _context, child| {
                let mut semantics = Semantics::new().enabled(enabled).button(true);
                if let Some(child) = child {
                    semantics = semantics.child(child.clone());
                }
                semantics.into_widget()
            },
        )
        .child(if K_IS_WEB {
            self.child.clone()
        } else {
            Stack::new()
                .alignment(AlignmentGeometry::CENTER)
                .children([
                    self.child.clone(),
                    Semantics::new()
                        .label(localizations.tab_label(
                            destination_info.index as i32 + 1,
                            destination_info.total_number_of_destinations as i32,
                        ))
                        .into_widget(),
                ])
                .into_widget()
        })
        .into_widget()
    }
}

/// Tooltip widget for use in a [`NavigationBar`].
///
/// It appears just above the navigation bar when one of the destinations is
/// long pressed.
#[derive(Debug)]
struct NavigationBarDestinationTooltip {
    /// The text that is rendered in the tooltip when it appears.
    message: String,

    /// The widget that, when pressed, will show a tooltip.
    child: WidgetRef,
}

impl NavigationBarDestinationTooltip {
    /// Adds a tooltip to the [`child`](Self::child) widget.
    fn new<K>(message: String, child: impl IntoWidget<K>) -> NavigationBarDestinationTooltip {
        NavigationBarDestinationTooltip {
            message,
            child: child.into_widget(),
        }
    }
}

impl StatelessWidget for NavigationBarDestinationTooltip {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        Tooltip::new()
            .message(self.message.clone())
            // TODO(johnsonmh): Make this value configurable/themable.
            .vertical_offset(42.0)
            .exclude_from_semantics(true)
            .prefer_below(false)
            .child(self.child.clone())
            .into_widget()
    }
}

/// Custom layout delegate for shifting navigation bar destinations.
///
/// This will lay out the icon + label according to the [`animation`](Self::animation).
///
/// When the [`animation`](Self::animation) is 0, the icon will be centered, and the label
/// will be positioned directly below it.
///
/// When the [`animation`](Self::animation) is 1, the label will still be positioned
/// directly below the icon, but the icon + label combination will be centered.
///
/// Used in a [`CustomMultiChildLayout`] widget in the
/// [`NavigationDestinationBuilder`].
struct NavigationDestinationLayoutDelegate {
    /// The selection animation that indicates whether or not this destination is
    /// selected.
    ///
    /// See [`NavigationDestinationInfo::selected_animation`].
    animation: AnimationRef<f64>,

    /// Dart's `super(relayout: animation)`.
    relayout: Rc<dyn Listenable>,
}

impl Debug for NavigationDestinationLayoutDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_NavigationDestinationLayoutDelegate")
            .field("animation", &self.animation)
            .finish_non_exhaustive()
    }
}

impl NavigationDestinationLayoutDelegate {
    fn new(animation: AnimationRef<f64>) -> NavigationDestinationLayoutDelegate {
        NavigationDestinationLayoutDelegate {
            relayout: Rc::new(animation.clone()),
            animation,
        }
    }

    /// ID for the icon widget child.
    ///
    /// This is used by the [`LayoutId`] when this delegate is used in a
    /// [`CustomMultiChildLayout`].
    ///
    /// See [`NavigationDestinationBuilder`].
    const ICON_ID: i64 = 1;

    /// ID for the label widget child.
    ///
    /// This is used by the [`LayoutId`] when this delegate is used in a
    /// [`CustomMultiChildLayout`].
    ///
    /// See [`NavigationDestinationBuilder`].
    const LABEL_ID: i64 = 2;

    /// [`ICON_ID`](Self::ICON_ID) as the [`ChildLayoutId`] of its [`LayoutId`].
    fn icon_id() -> ChildLayoutId {
        Rc::new(ValueKey::new(Self::ICON_ID))
    }

    /// [`LABEL_ID`](Self::LABEL_ID) as the [`ChildLayoutId`] of its [`LayoutId`].
    fn label_id() -> ChildLayoutId {
        Rc::new(ValueKey::new(Self::LABEL_ID))
    }
}

impl MultiChildLayoutDelegate for NavigationDestinationLayoutDelegate {
    fn relayout(&self) -> Option<&Rc<dyn Listenable>> {
        Some(&self.relayout)
    }

    fn perform_layout(&self, app: &mut App, children: &mut MultiChildLayoutChildren, size: Size) {
        let half_width = |size: Size| size.width() / 2.0;
        let half_height = |size: Size| size.height() / 2.0;

        let icon_size = children.layout_child(app, &Self::icon_id(), BoxConstraints::loose(size));
        let label_size = children.layout_child(app, &Self::label_id(), BoxConstraints::loose(size));

        let animation_value = self.animation.value(app);
        let y_position_offset = transform_tween(
            app,
            // When unselected, the icon is centered vertically.
            half_height(icon_size),
            // When selected, the icon and label are centered vertically.
            half_height(icon_size) + half_height(label_size),
            animation_value,
        );
        let icon_y_position = half_height(size) - y_position_offset;

        // Position the icon.
        children.position_child(
            app,
            &Self::icon_id(),
            Offset::new(
                // Center the icon horizontally.
                half_width(size) - half_width(icon_size),
                icon_y_position,
            ),
        );

        // Position the label.
        children.position_child(
            app,
            &Self::label_id(),
            Offset::new(
                // Center the label horizontally.
                half_width(size) - half_width(label_size),
                // Label always appears directly below the icon.
                icon_y_position + icon_size.height(),
            ),
        );
    }

    fn should_relayout(&self, old_delegate: &dyn MultiChildLayoutDelegate) -> bool {
        let old_delegate = old_delegate
            .as_any()
            .downcast_ref::<NavigationDestinationLayoutDelegate>()
            .expect("the same delegate type");
        old_delegate.animation != self.animation
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Widget that listens to an animation, and rebuilds when the animation changes
/// `AnimationStatus`.
///
/// This can be more efficient than just using an `AnimatedBuilder` when you
/// only need to rebuild when the `Animation::status` changes, since
/// `AnimatedBuilder` rebuilds every time the animation ticks.
struct StatusTransitionWidgetBuilder {
    animation: AnimationRef<f64>,

    /// Called every time the animation changes `AnimationStatus`.
    builder: TransitionBuilder,

    /// The child widget to pass to the [`builder`](Self::builder).
    ///
    /// If a [`builder`](Self::builder) callback's return value contains a subtree that does
    /// not depend on the animation, it's more efficient to build that subtree once
    /// instead of rebuilding it on every animation status change.
    ///
    /// Using this pre-built child is entirely optional, but can improve
    /// performance in some cases and is therefore a good practice.
    ///
    /// See: `AnimatedBuilder::child`
    child: Option<WidgetRef>,
}

impl StatusTransitionWidgetBuilder {
    /// Creates a widget that rebuilds when the given animation changes status.
    fn new(
        animation: AnimationRef<f64>,
        builder: impl Fn(&mut App, BuildContext, Option<&WidgetRef>) -> WidgetRef + 'static,
    ) -> StatusTransitionWidgetBuilder {
        StatusTransitionWidgetBuilder {
            animation,
            builder: Rc::new(builder),
            child: None,
        }
    }

    /// Dart `_StatusTransitionWidgetBuilder(child:)`.
    fn child<K>(mut self, child: impl IntoWidget<K>) -> StatusTransitionWidgetBuilder {
        self.child = Some(child.into_widget());
        self
    }
}

impl Debug for StatusTransitionWidgetBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_StatusTransitionWidgetBuilder")
            .field("animation", &self.animation)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatusTransitionWidget for StatusTransitionWidgetBuilder {
    fn animation(&self) -> AnyAnimation<f64> {
        self.animation.handle()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        (self.builder)(app, context, self.child.as_ref())
    }
}

/// Dart's `Widget Function(BuildContext, Animation<double>)` builders of this file.
type AnimationWidgetBuilder = Rc<dyn Fn(&mut App, BuildContext, AnimationRef<f64>) -> WidgetRef>;

// Builder widget for widgets that need to be animated from 0 (unselected) to
// 1.0 (selected).
//
// This widget creates and manages an `AnimationController` that it passes down
// to the child through the `builder` function.
//
// When `is_selected` is `true`, the animation controller will animate from
// 0 to 1 (for `duration` time).
//
// When `is_selected` is `false`, the animation controller will animate from
// 1 to 0 (for `duration` time).
//
// If `is_selected` is updated while the widget is animating, the animation will
// be reversed until it is either 0 or 1 again. If `always_do_full_animation` is
// true, the animation will reset to 0 or 1 before beginning the animation, so
// that the full animation is done.
struct SelectableAnimatedBuilder {
    /// When true, the widget will animate an animation controller from 0 to 1.
    ///
    /// The animation controller is passed to the child widget through
    /// [`builder`](Self::builder).
    is_selected: bool,

    /// How long the animation controller should animate for when
    /// [`is_selected`](Self::is_selected) is updated.
    ///
    /// If the animation is currently running and [`is_selected`](Self::is_selected) is
    /// updated, only the [`duration`](Self::duration) left to finish the animation will be
    /// run.
    duration: Duration,

    /// If true, the animation will always go all the way from 0 to 1 when
    /// [`is_selected`](Self::is_selected) is true, and from 1 to 0 when
    /// [`is_selected`](Self::is_selected) is false, even when the status changes mid
    /// animation.
    ///
    /// If this is false and the status changes mid animation, the animation will
    /// reverse direction from it's current point.
    ///
    /// Defaults to false.
    always_do_full_animation: bool,

    /// Builds the child widget based on the current animation status.
    builder: AnimationWidgetBuilder,
}

impl SelectableAnimatedBuilder {
    /// Builds and maintains an `AnimationController` that will animate from 0 to
    /// 1 and back depending on when [`is_selected`](Self::is_selected) is true.
    fn new(
        is_selected: bool,
        builder: impl Fn(&mut App, BuildContext, AnimationRef<f64>) -> WidgetRef + 'static,
    ) -> SelectableAnimatedBuilder {
        SelectableAnimatedBuilder {
            is_selected,
            duration: Duration::from_millis(200),
            always_do_full_animation: false,
            builder: Rc::new(builder),
        }
    }

    /// Dart `_SelectableAnimatedBuilder(duration:)`.
    fn duration(mut self, duration: Duration) -> SelectableAnimatedBuilder {
        self.duration = duration;
        self
    }

    /// Dart `_SelectableAnimatedBuilder(alwaysDoFullAnimation:)`.
    fn always_do_full_animation(
        mut self,
        always_do_full_animation: bool,
    ) -> SelectableAnimatedBuilder {
        self.always_do_full_animation = always_do_full_animation;
        self
    }
}

impl Debug for SelectableAnimatedBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_SelectableAnimatedBuilder")
            .field("is_selected", &self.is_selected)
            .field("duration", &self.duration)
            .field("always_do_full_animation", &self.always_do_full_animation)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SelectableAnimatedBuilder {
    type State = SelectableAnimatedBuilderState;

    fn create_state(&self) -> SelectableAnimatedBuilderState {
        SelectableAnimatedBuilderState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            controller: None,
        }
    }
}

/// Dart's `_SelectableAnimatedBuilderState`.
struct SelectableAnimatedBuilderState {
    state: StateData<SelectableAnimatedBuilder>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    controller: Option<Ref<AnimationController>>,
}

impl SelectableAnimatedBuilderState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .controller
            .handle()
            .expect("created in init_state")
    }
}

impl SingleTickerProviderStateMixin for SelectableAnimatedBuilderState {
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

impl TickerProviderObject for SelectableAnimatedBuilderState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for SelectableAnimatedBuilderState {
    type Widget = SelectableAnimatedBuilder;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = AnimationController::create(
            app,
            None,
            None,
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).controller = Some(controller.clone());
        let widget = self.widget(app);
        let duration = widget.duration;
        let value = if widget.is_selected { 1.0 } else { 0.0 };
        app.get_mut(&controller).duration = Some(duration);
        controller.set_value(app, value);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &SelectableAnimatedBuilder,
    ) {
        let controller = self.controller(app);
        let widget = self.widget(app);
        let duration = widget.duration;
        let is_selected = widget.is_selected;
        let always_do_full_animation = widget.always_do_full_animation;
        if old_widget.duration != duration {
            app.get_mut(controller).duration = Some(duration);
        }
        if old_widget.is_selected != is_selected {
            if is_selected {
                controller.forward(app, always_do_full_animation.then_some(0.0));
            } else {
                controller.reverse(app, always_do_full_animation.then_some(1.0));
            }
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let builder = Rc::clone(&self.widget(app).builder);
        let controller = app
            .get(self)
            .controller
            .as_ref()
            .expect("created in init_state")
            .erase();
        builder(app, context, controller)
    }
}

/// Watches [`animation`](Self::animation) and calls [`builder`](Self::builder) with the
/// appropriate [`Curve`] depending on the direction of the [`animation`](Self::animation)
/// status.
///
/// If `Animation::status` is forward or complete, [`curve`](Self::curve) is used. If
/// `Animation::status` is reverse or dismissed, [`reverse_curve`](Self::reverse_curve) is
/// used.
///
/// If the [`animation`](Self::animation) changes direction while it is already running, the curve
/// used will not change, this will keep the animations smooth until it
/// completes.
///
/// This is similar to `CurvedAnimation` except the animation status listeners
/// are removed when this widget is disposed.
struct CurvedAnimationBuilder {
    animation: AnimationRef<f64>,
    curve: Rc<dyn Curve>,
    reverse_curve: Rc<dyn Curve>,
    builder: AnimationWidgetBuilder,
}

impl Debug for CurvedAnimationBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_CurvedAnimationBuilder")
            .field("animation", &self.animation)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CurvedAnimationBuilder {
    type State = CurvedAnimationBuilderState;

    fn create_state(&self) -> CurvedAnimationBuilderState {
        CurvedAnimationBuilderState {
            state: StateData::new(),
            animation_direction: AnimationStatus::Dismissed,
            preserved_direction: None,
        }
    }
}

/// Dart's `_CurvedAnimationBuilderState`.
struct CurvedAnimationBuilderState {
    state: StateData<CurvedAnimationBuilder>,
    animation_direction: AnimationStatus,
    preserved_direction: Option<AnimationStatus>,
}

impl CurvedAnimationBuilderState {
    // Keeps track of the current animation status, as well as the "preserved
    // direction" when the animation changes direction mid animation.
    //
    // The preserved direction is reset when the animation finishes in either
    // direction.
    fn update_status(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if app.get(self).animation_direction != status {
            self.set_state(app, |state| {
                state.animation_direction = status;
            });
        }
        match status {
            AnimationStatus::Forward | AnimationStatus::Reverse
                if app.get(self).preserved_direction.is_some() => {}
            AnimationStatus::Forward | AnimationStatus::Reverse => {
                self.set_state(app, |state| {
                    state.preserved_direction = Some(status);
                });
            }
            AnimationStatus::Completed | AnimationStatus::Dismissed => {
                self.set_state(app, |state| {
                    state.preserved_direction = None;
                });
            }
        }
    }
}

impl State for CurvedAnimationBuilderState {
    type Widget = CurvedAnimationBuilder;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let animation = self.widget(app).animation.clone();
        let status = animation.status(app);
        app.get_mut(self).animation_direction = status;
        self.update_status(app, status);
        animation.add_status_listener(
            app,
            AnimationStatusListener::handle_method(self, Self::update_status),
        );
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let animation = self.widget(app).animation.clone();
        animation.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(self, Self::update_status),
        );
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let state = app.get(self);
        let should_use_forward_curve = state
            .preserved_direction
            .unwrap_or(state.animation_direction)
            != AnimationStatus::Reverse;

        let widget = self.widget(app);
        let curve = if should_use_forward_curve {
            Rc::clone(&widget.curve)
        } else {
            Rc::clone(&widget.reverse_curve)
        };
        let parent = widget.animation.clone();
        let builder = Rc::clone(&widget.builder);
        let curve_tween = CurveTween::new(app, curve);
        let curved_animation = parent.drive(app, curve_tween);

        builder(app, context, curved_animation)
    }
}

/// Dart's `_defaultsFor(context)`.
fn defaults_for(app: &mut App, context: BuildContext) -> Rc<dyn NavigationBarThemeDataGetters> {
    if Theme::of(app, context).use_material3 {
        Rc::new(NavigationBarDefaultsM3::new(context))
    } else {
        Rc::new(NavigationBarDefaultsM2::new(app, context))
    }
}

/// Hand coded defaults based on Material Design 2: Dart's `_NavigationBarDefaultsM2`.
struct NavigationBarDefaultsM2 {
    /// Dart's `super(..)` arguments.
    navigation_bar_theme_data: NavigationBarThemeData,
    context: DefaultsContext,
}

impl NavigationBarDefaultsM2 {
    fn new(app: &mut App, context: BuildContext) -> NavigationBarDefaultsM2 {
        NavigationBarDefaultsM2 {
            navigation_bar_theme_data: NavigationBarThemeData::new()
                .height(80.0)
                .elevation(0.0)
                .indicator_shape(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadius::all(Radius::circular(16.0)).into(),
                ))
                .label_behavior(NavigationDestinationLabelBehavior::AlwaysShow),
            context: DefaultsContext::with_theme(app, context),
        }
    }
}

#[allow(deprecated)]
impl NavigationBarThemeDataGetters for NavigationBarDefaultsM2 {
    fn navigation_bar_theme_data(&self) -> &NavigationBarThemeData {
        &self.navigation_bar_theme_data
    }

    // With Material 2, the NavigationBar uses an overlay blend for the
    // default color regardless of light/dark mode.
    fn background_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(ElevationOverlay::color_with_overlay(
            colors.surface,
            colors.on_surface,
            3.0,
        ))
    }

    fn icon_theme(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<IconThemeData>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(Rc::new(MaterialStatePropertyAll::new(Some(
            IconThemeData::new().size(24.0).color(colors.on_surface),
        ))))
    }

    fn indicator_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.secondary.with_opacity(0.24))
    }

    fn label_text_style(&self, app: &mut App) -> Option<WidgetStatePropertyRef<Option<TextStyle>>> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(Rc::new(MaterialStatePropertyAll::new(Some(
            theme
                .text_theme
                .label_small
                .clone()
                .expect("the text theme gives labelSmall")
                .copy_with()
                .color(colors.on_surface),
        ))))
    }

    fn label_padding(&self, _app: &mut App) -> Option<EdgeInsetsGeometry> {
        Some(EdgeInsetsGeometry::from(waft_painting::EdgeInsets::only(
            0.0, 4.0, 0.0, 0.0,
        )))
    }
}

/// Dart's `CurveTween(curve: curve).transform(t)`.
fn transform_curve_tween(app: &mut App, curve: Rc<dyn Curve>, t: f64) -> f64 {
    let curve_tween = CurveTween::new(app, curve);
    Animatable::transform(&curve_tween, app, t)
}

/// Dart's `Tween<double>(begin: begin, end: end).transform(t)`.
fn transform_tween(app: &mut App, begin: f64, end: f64, t: f64) -> f64 {
    let tween = Tween::new(app, Some(begin), Some(end));
    Animatable::transform(&tween, app, t)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::Cell;
    use waft_painting::ShapeBorder;

    use waft_embedder::{PointerChange, TextDirection, valo};
    use waft_painting::{EdgeInsets, StadiumBorder, TextScaler};
    use waft_rendering::RenderBox as _;
    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey};
    use waft_widgets::{
        AnyElement, Center, Directionality, FocusHighlightStrategy, Icon, IconData, MediaQueryData,
        RichText, StatefulBuilder, WidgetStateProperty,
    };

    use super::*;
    use crate::icons::Icons;
    use crate::material::RenderInkFeatures;
    use crate::scaffold::Scaffold;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3, pump_app, tester};

    pub(crate) fn icon(icon: IconData) -> WidgetRef {
        Icon::new(Some(icon)).into_widget()
    }

    pub(crate) fn destination(icon_data: IconData, label: &str) -> WidgetRef {
        NavigationDestination::new(icon(icon_data), label).into_widget()
    }

    /// `[NavigationDestination(Icons.ac_unit, 'AC'), NavigationDestination(Icons.access_alarm,
    /// 'Alarm')]`.
    pub(crate) fn ac_alarm() -> Vec<WidgetRef> {
        vec![
            destination(Icons::ac_unit(), "AC"),
            destination(Icons::access_alarm(), "Alarm"),
        ]
    }

    /// Dart's `_buildWidget(child, useMaterial3:)`.
    pub(crate) fn build_widget(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        child: impl FnOnce() -> WidgetRef,
    ) {
        pump_app(tester, theme, |_app| {
            Scaffold::new()
                .bottom_navigation_bar(Center::new().child(child()))
                .into_widget()
        });
    }

    pub(crate) fn navigation_bar(tester: &WidgetTester) -> AnyElement {
        tester.find_by_type::<NavigationBar>()[0]
    }

    pub(crate) fn bar_size(tester: &WidgetTester) -> Size {
        tester
            .render_box_of(navigation_bar(tester))
            .size(&tester.cell.borrow())
    }

    /// Dart's `_getMaterial(tester)`: the first `Material` under the bar.
    pub(crate) fn bar_material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let material = tester.descendants_of_type::<Material>(navigation_bar(tester))[0];
        tester.widget(material, read)
    }

    /// Dart's `_getIndicatorDecoration(tester)`: the color and shape of the first `Ink` under
    /// a `FadeTransition`.
    pub(crate) fn indicator_decoration(tester: &WidgetTester) -> (Option<Color>, ShapeBorderRef) {
        let fade = tester.find_by_type::<FadeTransition>();
        let ink = tester
            .find_by_type::<Ink>()
            .into_iter()
            .find(|ink| {
                let ancestors = tester.ancestors(*ink);
                fade.iter().any(|fade| ancestors.contains(fade))
            })
            .expect("an Ink under a FadeTransition");
        tester.widget(ink, |ink: &Ink| {
            let decoration = ink
                .decoration
                .as_ref()
                .and_then(|decoration| decoration.as_any().downcast_ref::<ShapeDecoration>())
                .expect("a ShapeDecoration");
            (
                decoration.color.as_deref().copied(),
                decoration.shape.clone(),
            )
        })
    }

    pub(crate) fn center_of(tester: &WidgetTester, element: AnyElement) -> Offset {
        tester.global_rect(tester.render_box_of(element)).center()
    }

    pub(crate) fn text(tester: &WidgetTester, text: &str) -> AnyElement {
        let found = tester.find_onstage_text(text);
        assert_eq!(found.len(), 1, "one Text showing {text}");
        found[0]
    }

    /// Dart's `_getLabelPadding(tester, text)`.
    pub(crate) fn label_padding(tester: &WidgetTester, label: &str) -> EdgeInsetsGeometry {
        let padding = tester.ancestor_of_type::<Padding>(text(tester, label));
        tester.widget(padding, |padding: &Padding| padding.padding)
    }

    /// Dart's `_getLabelStyle(tester, text)`.
    pub(crate) fn label_style(tester: &WidgetTester, label: &str) -> TextStyle {
        let label = text(tester, label);
        let rich_text = tester.descendants_of_type::<RichText>(label)[0];
        tester.widget(rich_text, |rich_text: &RichText| {
            rich_text
                .text
                .style()
                .cloned()
                .expect("the label has a style")
        })
    }

    fn argb(color: valo::Color) -> u32 {
        let channel = |value: f32| (value * 255.0).round() as u32;
        (channel(color.a) << 24)
            | (channel(color.r) << 16)
            | (channel(color.g) << 8)
            | channel(color.b)
    }

    /// What the ink features of the bar's own `Material` draw, in its coordinates: Dart's
    /// `paints` on the first `_RenderInkFeatures`, which records the features' calls with
    /// the arguments they are given.
    pub(crate) fn bar_ink_ops(tester: &WidgetTester) -> Vec<valo::Op> {
        let bar = tester.render_box_of(navigation_bar(tester)).as_object();
        let render = tester
            .all::<RenderInkFeatures>()
            .into_iter()
            .find(|render| {
                let app = tester.cell.borrow();
                std::iter::successors(render.as_object().parent(&app), |parent| {
                    parent.parent(&app)
                })
                .any(|ancestor| ancestor == bar)
            })
            .expect("the bar's Material");
        let mut canvas = waft_embedder::Canvas::new();
        render.paint_ink_features(&mut tester.cell.borrow_mut(), &mut canvas, Offset::ZERO);
        canvas.build().ops().to_vec()
    }

    /// Dart's `circle(x:, y:, radius:, color:)` calls: centre, radius and colour.
    pub(crate) fn circles(ops: &[valo::Op]) -> Vec<(Offset, f64, u32)> {
        ops.iter()
            .filter_map(|op| match op {
                valo::Op::DrawPath { path, paint, .. } => path.as_oval().map(|oval| {
                    (
                        Offset::new(
                            f64::from(oval.x + oval.width / 2.0),
                            f64::from(oval.y + oval.height / 2.0),
                        ),
                        f64::from(oval.width) / 2.0,
                        argb(paint.color),
                    )
                }),
                _ => None,
            })
            .collect()
    }

    /// Dart's `paints..clipPath(pathMatcher: isPathThat(includes:, excludes:))..circle(x:, y:,
    /// radius: 35.0, color: Color(0x0a000000))` for the indicator of `center`: a clip that
    /// holds the four edge midpoints of a 64x32 box and none of a 74x40 one, then the hover
    /// circle.
    pub(crate) fn paints_indicator_ripple(ops: &[valo::Op], center: Offset) -> bool {
        let midpoints = |size: Size| {
            [
                Offset::new(center.dx() - size.width() / 2.0, center.dy()),
                Offset::new(center.dx(), center.dy() - size.height() / 2.0),
                Offset::new(center.dx() + size.width() / 2.0, center.dy()),
                Offset::new(center.dx(), center.dy() + size.height() / 2.0),
            ]
        };
        // Dart's included points lie on the path's edge; the path is laid out through
        // valo's f32 `Matrix4`, so each is taken `POSITION_TOLERANCE` inside it.
        let inside = |size: Size| {
            midpoints(Size::new(
                size.width() - 2.0 * POSITION_TOLERANCE,
                size.height() - 2.0 * POSITION_TOLERANCE,
            ))
        };
        let contains = |path: &valo::Path, point: Offset| {
            path.contains(
                valo::Point::new(point.dx() as f32, point.dy() as f32),
                valo::FillRule::NonZero,
            )
        };
        let Some(clip_at) = ops.iter().position(|op| match op {
            valo::Op::ClipPath {
                path,
                is_axis_aligned_rect: false,
                ..
            } => {
                inside(Size::new(64.0, 32.0))
                    .iter()
                    .all(|point| contains(path, *point))
                    && midpoints(Size::new(74.0, 40.0))
                        .iter()
                        .all(|point| !contains(path, *point))
            }
            _ => false,
        }) else {
            return false;
        };
        circles(&ops[clip_at..])
            .iter()
            .any(|(circle_center, radius, color)| {
                (circle_center.dx() - center.dx()).abs() < 1e-3
                    && (circle_center.dy() - center.dy()).abs() < 1e-3
                    && (radius - 35.0).abs() < 1e-3
                    && *color == 0x0a000000
            })
    }

    fn mouse_over(
        tester: &mut WidgetTester,
        mouse: &mut crate::test_support::TestPointer,
        at: Offset,
    ) {
        tester.move_to(mouse, at, false);
        tester.pump_and_settle();
    }

    #[test]
    fn navigation_bar_updates_destinations_when_tapped() {
        with_main_thread_stack(|| {
            let mutated_index = Rc::new(Cell::new(None));
            let mut tester = tester();
            let seen = Rc::clone(&mutated_index);
            build_widget(&mut tester, m3, move || {
                NavigationBar::new(ac_alarm())
                    .on_destination_selected(move |_app, i| seen.set(Some(i)))
                    .into_widget()
            });

            assert_eq!(tester.find_onstage_text("AC").len(), 1);
            assert_eq!(tester.find_onstage_text("Alarm").len(), 1);

            tester.tap(center_of(&tester, text(&tester, "Alarm")), 1);
            assert_eq!(mutated_index.get(), Some(1));

            tester.tap(center_of(&tester, text(&tester, "AC")), 2);
            assert_eq!(mutated_index.get(), Some(0));
        });
    }

    #[test]
    fn navigation_bar_can_update_background_color() {
        with_main_thread_stack(|| {
            let color = Colors::YELLOW.primary();
            let mut tester = tester();
            build_widget(&mut tester, m3, move || {
                NavigationBar::new(ac_alarm())
                    .background_color(color)
                    .on_destination_selected(|_app, _i| {})
                    .into_widget()
            });

            assert_eq!(
                bar_material(&tester, |material| material.color),
                Some(color)
            );
        });
    }

    #[test]
    fn navigation_bar_can_update_elevation() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            build_widget(&mut tester, m3, || {
                NavigationBar::new(ac_alarm())
                    .elevation(42.0)
                    .on_destination_selected(|_app, _i| {})
                    .into_widget()
            });

            assert_eq!(bar_material(&tester, |material| material.elevation), 42.0);
        });
    }

    #[test]
    fn navigation_bar_adds_bottom_padding_to_height() {
        with_main_thread_stack(|| {
            let bottom_padding = 40.0;
            let mut tester = tester();
            build_widget(&mut tester, m3, || {
                NavigationBar::new(ac_alarm())
                    .on_destination_selected(|_app, _i| {})
                    .into_widget()
            });

            let default_size = bar_size(&tester).height();
            assert_eq!(default_size, 80.0);

            build_widget(&mut tester, m3, || {
                MediaQuery::new(
                    MediaQueryData::new().padding(EdgeInsets::only(0.0, 0.0, 0.0, bottom_padding)),
                    NavigationBar::new(ac_alarm()).on_destination_selected(|_app, _i| {}),
                )
                .into_widget()
            });

            let expected_height = default_size + bottom_padding;
            assert_eq!(bar_size(&tester).height(), expected_height);
        });
    }

    /// The tolerance of a position read through `local_to_global`: `Matrix4` is valo's f32
    /// matrix (waft-gestures' PORTING.md), so a position is exact only to f32 where Dart's f64
    /// one is exact.
    const POSITION_TOLERANCE: f64 = 1e-4;

    /// Dart's `expect(x, y)` and `closeTo(y, precisionErrorTolerance)` on a coordinate read
    /// through `local_to_global`.
    fn assert_coordinate_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < POSITION_TOLERANCE,
            "{actual} is not {expected}"
        );
    }

    #[test]
    fn navigation_bar_respects_the_notch_or_system_navigation_bar_in_landscape_mode() {
        with_main_thread_stack(|| {
            let safe_area_padding = 40.0;
            let center_key = || KeyRef::from(<dyn waft_foundation::Key>::new("Center"));
            let navigation_bar = move || {
                NavigationBar::new([
                    destination(Icons::ac_unit(), "AC"),
                    NavigationDestination::new(icon(Icons::center_focus_strong()), "Center")
                        .key(center_key())
                        .into_widget(),
                    destination(Icons::access_alarm(), "Alarm"),
                ])
                .on_destination_selected(|_app, _i| {})
            };
            let center_item = |tester: &WidgetTester| {
                let found = tester.find_onstage_by_key(&center_key());
                center_of(tester, found[0])
            };
            let with_padding = |padding: EdgeInsets| {
                move || {
                    MediaQuery::new(MediaQueryData::new().padding(padding), navigation_bar())
                        .into_widget()
                }
            };

            let mut tester = tester();
            build_widget(&mut tester, m3, || navigation_bar().into_widget());
            let default_width = bar_size(&tester).width();
            assert_coordinate_eq(center_item(&tester).dx(), default_width / 2.0);

            // The position of center item of navigation bar should indicate whether
            // the safe area is sufficiently respected, when safe area is on the left side.
            // e.g. Android device with system navigation bar in landscape mode.
            build_widget(
                &mut tester,
                m3,
                with_padding(EdgeInsets::only(safe_area_padding, 0.0, 0.0, 0.0)),
            );
            assert_coordinate_eq(
                center_item(&tester).dx(),
                (default_width + safe_area_padding) / 2.0,
            );

            // The position of center item of navigation bar should indicate whether
            // the safe area is sufficiently respected, when safe area is on the right side.
            // e.g. Android device with system navigation bar in landscape mode.
            build_widget(
                &mut tester,
                m3,
                with_padding(EdgeInsets::only(0.0, 0.0, safe_area_padding, 0.0)),
            );
            assert_coordinate_eq(
                center_item(&tester).dx(),
                (default_width - safe_area_padding) / 2.0,
            );

            // The position of center item of navigation bar should indicate whether
            // the safe area is sufficiently respected, when safe areas are on both sides.
            // e.g. iOS device with both sides of round corner.
            build_widget(
                &mut tester,
                m3,
                with_padding(EdgeInsets::from_ltrb(
                    safe_area_padding,
                    0.0,
                    safe_area_padding,
                    safe_area_padding,
                )),
            );
            assert_coordinate_eq(center_item(&tester).dx(), default_width / 2.0);
        });
    }

    #[test]
    fn material2_navigation_bar_uses_proper_defaults_when_no_parameters_are_given() {
        with_main_thread_stack(|| {
            // M2 settings that were hand coded.
            let mut tester = tester();
            build_widget(&mut tester, m2, || {
                NavigationBar::new(ac_alarm())
                    .on_destination_selected(|_app, _i| {})
                    .into_widget()
            });

            assert_eq!(
                bar_material(&tester, |material| material.color),
                Some(Color::new(0xffeaeaea))
            );
            assert_eq!(
                bar_material(&tester, |material| material.surface_tint_color),
                None
            );
            assert_eq!(bar_material(&tester, |material| material.elevation), 0.0);
            assert_eq!(bar_size(&tester).height(), 80.0);
            let (color, shape) = indicator_decoration(&tester);
            assert_eq!(color, Some(Color::new(0x3d2196f3)));
            assert_eq!(
                &*shape,
                &RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadius::all(Radius::circular(16.0)).into()
                ) as &dyn ShapeBorder
            );
        });
    }

    #[test]
    fn material3_navigation_bar_uses_proper_defaults_when_no_parameters_are_given() {
        with_main_thread_stack(|| {
            // M3 settings from the token database.
            let mut tester = tester();
            let theme = m3(&tester.cell.borrow());
            build_widget(&mut tester, m3, || {
                NavigationBar::new(ac_alarm())
                    .on_destination_selected(|_app, _i| {})
                    .into_widget()
            });

            assert_eq!(
                bar_material(&tester, |material| material.color),
                Some(theme.color_scheme.surface_container())
            );
            assert_eq!(
                bar_material(&tester, |material| material.surface_tint_color),
                Some(Colors::TRANSPARENT)
            );
            assert_eq!(bar_material(&tester, |material| material.elevation), 3.0);
            assert_eq!(bar_size(&tester).height(), 80.0);
            let (color, shape) = indicator_decoration(&tester);
            assert_eq!(color, Some(theme.color_scheme.secondary_container()));
            assert_eq!(
                &*shape,
                &StadiumBorder::new(BorderSide::NONE) as &dyn ShapeBorder
            );
        });
    }

    /// The app of the tooltip text-scaling cases: `MediaQuery(data: MediaQueryData(textScaler:
    /// text_scaler), child: MaterialApp(home: Scaffold(bottomNavigationBar: NavigationBar(..))))`.
    fn pump_scaled_bar(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        text_scaler: TextScaler,
        first: NavigationDestination,
    ) {
        let root = {
            let app = tester.cell.borrow();
            let theme = theme(&app);
            MediaQuery::new(
                MediaQueryData::new().text_scaler(text_scaler),
                material_app(
                    theme,
                    Scaffold::new()
                        .bottom_navigation_bar(NavigationBar::new([
                            first.into_widget(),
                            destination(Icons::battery_alert(), "B"),
                        ]))
                        .into_widget(),
                ),
            )
            .into_widget()
        };
        tester.pump_widget(root);
    }

    fn text_size(tester: &WidgetTester, element: AnyElement) -> Size {
        tester.render_box_of(element).size(&tester.cell.borrow())
    }

    #[test]
    fn material2_navigation_bar_shows_tooltips_with_text_scaling() {
        with_main_thread_stack(|| {
            let label = "A";
            let first = || NavigationDestination::new(icon(Icons::ac_unit()), label).tooltip(label);
            let mut tester = tester();

            pump_scaled_bar(&mut tester, m2, TextScaler::NO_SCALING, first());
            assert_eq!(tester.find_text(label).len(), 1);
            tester.long_press(center_of(&tester, tester.find_text(label)[0]), 1);
            assert_eq!(tester.find_text(label).len(), 2);

            // The default size of a tooltip with the text A.
            let default_tooltip_size = Size::new(14.0, 14.0);
            let found = tester.find_text(label);
            assert_eq!(
                text_size(&tester, found[found.len() - 1]),
                default_tooltip_size
            );
            // The duration is needed to ensure the tooltip disappears.
            tester.pump_and_settle_by(Duration::from_secs(2));

            pump_scaled_bar(&mut tester, m2, TextScaler::linear(4.0), first());
            assert_eq!(tester.find_text(label).len(), 1);
            tester.long_press(center_of(&tester, tester.find_text(label)[0]), 2);
            let found = tester.find_text(label);
            assert_eq!(
                text_size(&tester, found[found.len() - 1]),
                Size::new(
                    default_tooltip_size.width() * 4.0,
                    default_tooltip_size.height() * 4.0
                )
            );
        });
    }

    #[test]
    fn material3_navigation_bar_shows_tooltips_with_text_scaling() {
        with_main_thread_stack(|| {
            let label = "A";
            let first = || NavigationDestination::new(icon(Icons::ac_unit()), label).tooltip(label);
            let mut tester = tester();

            pump_scaled_bar(&mut tester, m3, TextScaler::NO_SCALING, first());
            assert_eq!(tester.find_text(label).len(), 1);
            tester.long_press(center_of(&tester, tester.find_text(label)[0]), 1);
            assert_eq!(tester.find_text(label).len(), 2);

            let found = tester.find_text(label);
            assert_eq!(
                text_size(&tester, found[found.len() - 1]),
                Size::new(14.25, 20.0)
            );
            // The duration is needed to ensure the tooltip disappears.
            tester.pump_and_settle_by(Duration::from_secs(2));

            pump_scaled_bar(&mut tester, m3, TextScaler::linear(4.0), first());
            assert_eq!(tester.find_text(label).len(), 1);
            tester.long_press(center_of(&tester, tester.find_text(label)[0]), 2);

            let found = tester.find_text(label);
            assert_eq!(
                text_size(&tester, found[found.len() - 1]),
                Size::new(56.25, 80.0)
            );
        });
    }

    /// Dart's `_sizeAlmostEqual(a, b)`, with its default `maxDiff` of 0.05.
    fn size_almost_equal(a: Size, b: Size) -> bool {
        (a.width() - b.width()).abs() <= 0.05 && (a.height() - b.height()).abs() <= 0.05
    }

    #[test]
    fn material3_navigation_bar_label_can_scale_and_has_max_scale_factor() {
        with_main_thread_stack(|| {
            let label = "A";
            let first = || NavigationDestination::new(icon(Icons::ac_unit()), label);
            let mut tester = tester();

            pump_scaled_bar(&mut tester, m3, TextScaler::NO_SCALING, first());
            assert_eq!(tester.find_text(label).len(), 1);
            let size = text_size(&tester, text(&tester, label));
            assert!(size_almost_equal(size, Size::new(12.5, 16.0)), "{size:?}");

            pump_scaled_bar(&mut tester, m3, TextScaler::linear(1.1), first());
            tester.pump_and_settle();
            let size = text_size(&tester, text(&tester, label));
            assert!(size_almost_equal(size, Size::new(13.7, 18.0)), "{size:?}");

            pump_scaled_bar(&mut tester, m3, TextScaler::linear(1.3), first());
            let size = text_size(&tester, text(&tester, label));
            assert!(size_almost_equal(size, Size::new(16.1, 21.0)), "{size:?}");

            pump_scaled_bar(&mut tester, m3, TextScaler::linear(4.0), first());
            let size = text_size(&tester, text(&tester, label));
            assert!(size_almost_equal(size, Size::new(16.1, 21.0)), "{size:?}");
        });
    }

    /// Dart's `findByTooltip(message)`: the `Tooltip`s whose message is `message`.
    fn tooltips_with_message(tester: &WidgetTester, message: &str) -> usize {
        tester
            .find_by_type::<Tooltip>()
            .into_iter()
            .filter(|tooltip| {
                tester.widget(*tooltip, |tooltip: &Tooltip| {
                    tooltip.message.as_deref() == Some(message)
                })
            })
            .count()
    }

    #[test]
    fn custom_tooltips_in_navigation_bar_destination() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Scaffold::new()
                    .bottom_navigation_bar(NavigationBar::new([
                        NavigationDestination::new(icon(Icons::ac_unit()), "A")
                            .tooltip("A tooltip")
                            .into_widget(),
                        destination(Icons::battery_alert(), "B"),
                        NavigationDestination::new(icon(Icons::cake()), "C")
                            .tooltip("")
                            .into_widget(),
                    ]))
                    .into_widget()
            });

            assert_eq!(tester.find_onstage_text("A").len(), 1);
            tester.long_press(center_of(&tester, text(&tester, "A")), 1);
            assert_eq!(tooltips_with_message(&tester, "A tooltip"), 1);

            assert_eq!(tester.find_onstage_text("B").len(), 1);
            tester.long_press(center_of(&tester, text(&tester, "B")), 2);
            assert_eq!(tooltips_with_message(&tester, "B"), 1);

            assert_eq!(tester.find_onstage_text("C").len(), 1);
            tester.long_press(center_of(&tester, text(&tester, "C")), 3);
            assert_eq!(tooltips_with_message(&tester, "C"), 0);
        });
    }

    #[test]
    fn navigation_bar_does_not_grow_with_text_scale_factor() {
        with_main_thread_stack(|| {
            let widget = |text_scaler: TextScaler| {
                move || {
                    MediaQuery::new(
                        MediaQueryData::new().text_scaler(text_scaler),
                        NavigationBar::new(ac_alarm())
                            .animation_duration(Duration::from_millis(800)),
                    )
                    .into_widget()
                }
            };
            let mut tester = tester();

            build_widget(&mut tester, m3, widget(TextScaler::NO_SCALING));
            let initial_height = bar_size(&tester).height();

            build_widget(&mut tester, m3, widget(TextScaler::linear(2.0)));
            let new_height = bar_size(&tester).height();

            assert_eq!(new_height, initial_height);
        });
    }

    /// The bar of the ripple cases, centred in the scaffold's bottom slot.
    fn pump_ripple_bar(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        selected_index: usize,
        label_behavior: Option<NavigationDestinationLabelBehavior>,
    ) {
        build_widget(tester, theme, move || {
            let mut bar = NavigationBar::new(ac_alarm())
                .selected_index(selected_index)
                .on_destination_selected(|_app, _i| {});
            if let Some(label_behavior) = label_behavior {
                bar = bar.label_behavior(label_behavior);
            }
            bar.into_widget()
        });
    }

    fn icon_center(tester: &WidgetTester, icon: &IconData) -> Offset {
        center_of(tester, tester.find_by_icon(icon))
    }

    /// Dart's `Material3/Material2 - Navigation indicator renders ripple`, with the indicator
    /// centres the label behaviours give.
    fn indicator_renders_ripple(
        theme: fn(&App) -> ThemeData,
        always_show_center: Offset,
        always_hide_center: Offset,
    ) {
        // This is a regression test for https://github.com/flutter/flutter/issues/116751.
        let mut tester = tester();
        pump_ripple_bar(&mut tester, theme, 0, None);

        let mut mouse = WidgetTester::mouse(1, 0);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        let alarm = icon_center(&tester, &Icons::access_alarm());
        mouse_over(&mut tester, &mut mouse, alarm);

        // Test ripple when NavigationBar is using `NavigationDestinationLabelBehavior.alwaysShow` (default).
        assert!(paints_indicator_ripple(
            &bar_ink_ops(&tester),
            always_show_center
        ));

        // Test ripple when NavigationBar is using `NavigationDestinationLabelBehavior.alwaysHide`.
        pump_ripple_bar(
            &mut tester,
            theme,
            0,
            Some(NavigationDestinationLabelBehavior::AlwaysHide),
        );
        let alarm = icon_center(&tester, &Icons::access_alarm());
        mouse_over(&mut tester, &mut mouse, alarm);
        assert!(paints_indicator_ripple(
            &bar_ink_ops(&tester),
            always_hide_center
        ));

        // Test ripple when NavigationBar is using `NavigationDestinationLabelBehavior.onlyShowSelected`.
        pump_ripple_bar(
            &mut tester,
            theme,
            0,
            Some(NavigationDestinationLabelBehavior::OnlyShowSelected),
        );
        let alarm = icon_center(&tester, &Icons::access_alarm());
        mouse_over(&mut tester, &mut mouse, alarm);
        assert!(paints_indicator_ripple(
            &bar_ink_ops(&tester),
            always_hide_center
        ));

        // Make sure ripple is shifted when selectedIndex changes.
        pump_ripple_bar(
            &mut tester,
            theme,
            1,
            Some(NavigationDestinationLabelBehavior::OnlyShowSelected),
        );
        tester.pump_and_settle();
        assert!(paints_indicator_ripple(
            &bar_ink_ops(&tester),
            always_show_center
        ));
    }

    #[test]
    fn material3_navigation_indicator_renders_ripple() {
        with_main_thread_stack(|| {
            indicator_renders_ripple(m3, Offset::new(600.0, 30.0), Offset::new(600.0, 40.0));
        });
    }

    #[test]
    fn material2_navigation_indicator_renders_ripple() {
        with_main_thread_stack(|| {
            indicator_renders_ripple(m2, Offset::new(600.0, 33.0), Offset::new(600.0, 40.0));
        });
    }

    /// The x scale of the last `Transform` under a `NavigationIndicator`.
    fn indicator_x_scale(tester: &WidgetTester) -> f32 {
        let transforms: Vec<AnyElement> = tester
            .find_by_type::<NavigationIndicator>()
            .into_iter()
            .flat_map(|indicator| tester.descendants_of_type::<Transform>(indicator))
            .collect();
        let transform = *transforms.last().expect("a Transform under an indicator");
        tester.widget(transform, |transform: &Transform| {
            transform.transform.to_flutter_array()[0]
        })
    }

    #[test]
    fn navigation_indicator_scale_transform() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_ripple_bar(&mut tester, m3, 0, None);
            tester.pump_and_settle();
            assert_eq!(indicator_x_scale(&tester), 0.0);

            pump_ripple_bar(&mut tester, m3, 1, None);
            tester.pump(Duration::from_millis(100));
            // The matrix is valo's f32 one: Dart's double, rounded to f32.
            assert_eq!(indicator_x_scale(&tester), 0.7805849514007568_f64 as f32);

            tester.pump(Duration::from_millis(100));
            assert_eq!(indicator_x_scale(&tester), 0.9473570239543915_f64 as f32);

            tester.pump_and_settle();
            assert_eq!(indicator_x_scale(&tester), 1.0);
        });
    }

    #[test]
    fn material3_navigation_destination_updates_indicator_color_and_shape() {
        with_main_thread_stack(|| {
            let color = Color::new(0xff0000ff);
            let shape = || RoundedRectangleBorder::new(BorderSide::NONE, BorderRadius::ZERO.into());
            let mut tester = tester();
            let theme = m3(&tester.cell.borrow());
            let pump = |tester: &mut WidgetTester, custom: bool| {
                pump_app(tester, m3, |_app| {
                    let mut bar =
                        NavigationBar::new(ac_alarm()).on_destination_selected(|_app, _i| {});
                    if custom {
                        bar = bar.indicator_color(color).indicator_shape(shape());
                    }
                    Scaffold::new()
                        .bottom_navigation_bar(waft_widgets::RepaintBoundary::new().child(bar))
                        .into_widget()
                });
            };

            pump(&mut tester, false);

            // Test default indicator color and shape.
            let (default_color, default_shape) = indicator_decoration(&tester);
            assert_eq!(
                default_color,
                Some(theme.color_scheme.secondary_container())
            );
            assert_eq!(
                &*default_shape,
                &StadiumBorder::new(BorderSide::NONE) as &dyn ShapeBorder
            );

            pump(&mut tester, true);

            // Test custom indicator color and shape.
            let (custom_color, custom_shape) = indicator_decoration(&tester);
            assert_eq!(custom_color, Some(color));
            assert_eq!(&*custom_shape, &shape() as &dyn ShapeBorder);
        });
    }

    #[test]
    fn destinations_respect_their_disabled_state() {
        with_main_thread_stack(|| {
            let selected_index = Rc::new(Cell::new(0));
            let mut tester = tester();
            let seen = Rc::clone(&selected_index);
            build_widget(&mut tester, m3, move || {
                let current = seen.get();
                NavigationBar::new([
                    destination(Icons::ac_unit(), "AC"),
                    destination(Icons::access_alarm(), "Alarm"),
                    NavigationDestination::new(icon(Icons::bookmark()), "Bookmark")
                        .enabled(false)
                        .into_widget(),
                ])
                .on_destination_selected(move |_app, i| seen.set(i))
                .selected_index(current)
                .into_widget()
            });

            tester.tap(center_of(&tester, text(&tester, "AC")), 1);
            assert_eq!(selected_index.get(), 0);

            tester.tap(center_of(&tester, text(&tester, "Alarm")), 2);
            assert_eq!(selected_index.get(), 1);

            tester.tap(center_of(&tester, text(&tester, "Bookmark")), 3);
            assert_eq!(selected_index.get(), 1);
        });
    }

    /// Dart's `overlayColor` of the overlay-colour cases: hovered, focused, pressed, else
    /// transparent.
    pub(crate) fn state_overlay_color() -> WidgetStatePropertyRef<Option<Color>> {
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
            Some(if states.contains(&WidgetState::Hovered) {
                Color::new(0xff0000ff)
            } else if states.contains(&WidgetState::Focused) {
                Color::new(0xff00ffff)
            } else if states.contains(&WidgetState::Pressed) {
                Color::new(0xffff00ff)
            } else {
                Colors::TRANSPARENT
            })
        })
    }

    /// Dart's hover, press and focus steps of the overlay-colour cases, on a bar already
    /// pumped.
    pub(crate) fn respects_overlay_color(tester: &mut WidgetTester) {
        let hover_color = 0xff0000ff;
        let focus_color = 0xff00ffff;
        let pressed_color = 0xffff00ff;

        let indicators = tester.find_by_type::<NavigationIndicator>();
        let indicator = center_of(tester, indicators[indicators.len() - 1]);
        let mut mouse = WidgetTester::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        mouse_over(tester, &mut mouse, indicator);

        // Test hovered state.
        let colors = |tester: &WidgetTester| -> Vec<u32> {
            circles(&bar_ink_ops(tester))
                .into_iter()
                .map(|(_, _, color)| color)
                .collect()
        };
        assert!(colors(tester).contains(&hover_color));

        tester.send(&mouse, PointerChange::Down);
        tester.pump_and_settle();

        // Test pressed state.
        let pressed = colors(tester);
        let at = pressed
            .iter()
            .position(|color| *color == pressed_color)
            .expect("a circle in the pressed colour");
        assert!(at >= 1, "a circle before the pressed one: {pressed:x?}");

        tester.send(&mouse, PointerChange::Up);
        tester.pump_and_settle();

        // Press tab to focus the navigation bar.
        tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
        tester.pump_and_settle();

        // Test focused state.
        let focused = colors(tester);
        let at = focused
            .iter()
            .position(|color| *color == focus_color)
            .expect("a circle in the focus colour");
        assert!(at >= 1, "a circle before the focused one: {focused:x?}");
    }

    #[test]
    fn navigation_bar_respects_overlay_color_in_active_pressed_hovered_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            pump_app(&mut tester, m3, |_app| {
                Scaffold::new()
                    .bottom_navigation_bar(
                        waft_widgets::RepaintBoundary::new().child(
                            NavigationBar::new(ac_alarm())
                                .overlay_color(state_overlay_color())
                                .on_destination_selected(|_app, _i| {}),
                        ),
                    )
                    .into_widget()
            });

            respects_overlay_color(&mut tester);
        });
    }

    #[test]
    fn navigation_bar_label_padding_overrides_navigation_destination_label_padding() {
        with_main_thread_stack(|| {
            let label_padding = EdgeInsetsGeometry::all(8.0);
            let pump = |tester: &mut WidgetTester, padding: Option<EdgeInsetsGeometry>| {
                pump_app(tester, m3, |_app| {
                    let mut bar = NavigationBar::new([
                        destination(Icons::home(), "Home"),
                        destination(Icons::settings(), "Settings"),
                    ])
                    .on_destination_selected(|_app, _i| {});
                    if let Some(padding) = padding {
                        bar = bar.label_padding(padding);
                    }
                    Scaffold::new().bottom_navigation_bar(bar).into_widget()
                });
            };
            let mut tester = tester();

            pump(&mut tester, None);
            let default_padding: EdgeInsetsGeometry = EdgeInsets::only(0.0, 4.0, 0.0, 0.0).into();
            assert_eq!(label_padding_of(&tester, "Home"), default_padding);
            assert_eq!(label_padding_of(&tester, "Settings"), default_padding);

            pump(&mut tester, Some(label_padding));
            assert_eq!(label_padding_of(&tester, "Home"), label_padding);
            assert_eq!(label_padding_of(&tester, "Settings"), label_padding);
        });
    }

    fn label_padding_of(tester: &WidgetTester, label: &str) -> EdgeInsetsGeometry {
        label_padding(tester, label)
    }

    #[test]
    fn material2_navigation_destination_updates_indicator_color_and_shape() {
        with_main_thread_stack(|| {
            let color = Color::new(0xff0000ff);
            let shape = || RoundedRectangleBorder::new(BorderSide::NONE, BorderRadius::ZERO.into());
            let mut tester = tester();
            let theme = m2(&tester.cell.borrow());
            let pump = |tester: &mut WidgetTester, custom: bool| {
                pump_app(tester, m2, |_app| {
                    let mut bar =
                        NavigationBar::new(ac_alarm()).on_destination_selected(|_app, _i| {});
                    if custom {
                        bar = bar.indicator_color(color).indicator_shape(shape());
                    }
                    Scaffold::new().bottom_navigation_bar(bar).into_widget()
                });
            };

            pump(&mut tester, false);

            // Test default indicator color and shape.
            let (default_color, default_shape) = indicator_decoration(&tester);
            #[allow(deprecated)]
            let expected_color = theme.color_scheme.secondary.with_opacity(0.24);
            assert_eq!(default_color, Some(expected_color));
            assert_eq!(
                &*default_shape,
                &RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadius::all(Radius::circular(16.0)).into()
                ) as &dyn ShapeBorder
            );

            pump(&mut tester, true);

            // Test custom indicator color and shape.
            let (custom_color, custom_shape) = indicator_decoration(&tester);
            assert_eq!(custom_color, Some(color));
            assert_eq!(&*custom_shape, &shape() as &dyn ShapeBorder);
        });
    }

    /// Dart's `IconWithRandomColor`: an `Icon` whose colour differs on every build.
    #[derive(Debug)]
    struct IconWithChangingColor {
        icon: IconData,
        builds: Rc<Cell<u32>>,
    }

    impl StatelessWidget for IconWithChangingColor {
        fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            self.builds.set(self.builds.get() + 1);
            let color = Color::new(0xff000000 | self.builds.get());
            Icon::new(Some(self.icon.clone()))
                .color(color)
                .into_widget()
        }
    }

    #[test]
    fn material2_destination_icon_does_not_rebuild_when_tapped() {
        with_main_thread_stack(|| {
            // This is a regression test for https://github.com/flutter/flutter/issues/122811.
            let builds = Rc::new(Cell::new(0));
            // Dart's `const` destinations: one instance across the builder's builds.
            let destinations = vec![
                NavigationDestination::new(
                    IconWithChangingColor {
                        icon: Icons::ac_unit(),
                        builds: Rc::clone(&builds),
                    },
                    "AC",
                )
                .into_widget(),
                NavigationDestination::new(
                    IconWithChangingColor {
                        icon: Icons::access_alarm(),
                        builds: Rc::clone(&builds),
                    },
                    "Alarm",
                )
                .into_widget(),
            ];
            let mut tester = tester();
            pump_app(&mut tester, m3, move |_app| {
                Scaffold::new()
                    .bottom_navigation_bar(StatefulBuilder::new(
                        move |_app, _context, set_state| {
                            let selected_index = 0;
                            NavigationBar::new(destinations.clone())
                                .selected_index(selected_index)
                                .on_destination_selected(move |app, _i| {
                                    set_state(app, &mut |_app| {});
                                })
                                .into_widget()
                        },
                    ))
                    .into_widget()
            });
            let last_icon_color = |tester: &WidgetTester| {
                let icons = tester.find_by_type::<Icon>();
                tester.widget(icons[icons.len() - 1], |icon: &Icon| icon.color.clone())
            };
            let initial_color = last_icon_color(&tester).expect("the icon has a color");

            // Trigger a rebuild.
            tester.tap(center_of(&tester, text(&tester, "Alarm")), 1);
            tester.pump_and_settle();

            // Icon color should be the same as before the rebuild.
            assert_eq!(last_icon_color(&tester), Some(initial_color));
        });
    }

    #[test]
    fn navigation_bar_label_text_style_overrides_navigation_destination_label_text_style() {
        with_main_thread_stack(|| {
            let selected_text = "Home";
            let unselected_text = "Settings";
            let disabled_text = "Bookmark";
            let pump = |tester: &mut WidgetTester,
                        label_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>| {
                pump_app(tester, m3, |_app| {
                    let mut bar = NavigationBar::new([
                        destination(Icons::home(), selected_text),
                        destination(Icons::settings(), unselected_text),
                        NavigationDestination::new(icon(Icons::bookmark()), disabled_text)
                            .enabled(false)
                            .into_widget(),
                    ]);
                    if let Some(label_text_style) = label_text_style {
                        bar = bar.label_text_style(label_text_style);
                    }
                    Scaffold::new().bottom_navigation_bar(bar).into_widget()
                });
            };
            let mut tester = tester();
            let theme = m3(&tester.cell.borrow());

            pump(&mut tester, None);

            // Test selected label text style.
            assert_eq!(label_style(&tester, selected_text).font_size, Some(12.0));
            assert_eq!(
                label_style(&tester, selected_text)
                    .color
                    .as_deref()
                    .copied(),
                Some(theme.color_scheme.on_surface)
            );

            // Test unselected label text style.
            assert_eq!(label_style(&tester, unselected_text).font_size, Some(12.0));
            assert_eq!(
                label_style(&tester, unselected_text)
                    .color
                    .as_deref()
                    .copied(),
                Some(theme.color_scheme.on_surface_variant())
            );

            // Test disabled label text style.
            assert_eq!(label_style(&tester, disabled_text).font_size, Some(12.0));
            #[allow(deprecated)]
            let disabled_color = theme.color_scheme.on_surface_variant().with_opacity(0.38);
            assert_eq!(
                label_style(&tester, disabled_text)
                    .color
                    .as_deref()
                    .copied(),
                Some(disabled_color)
            );

            let selected_text_style = TextStyle::new()
                .font_size(15.0)
                .color(Color::new(0xFF00FF00));
            let unselected_text_style = TextStyle::new()
                .font_size(15.0)
                .color(Color::new(0xFF0000FF));
            let disabled_text_style = TextStyle::new()
                .font_size(16.0)
                .color(Color::new(0xFFFF0000));
            pump(
                &mut tester,
                Some(<dyn WidgetStateProperty<Option<TextStyle>>>::from_map(
                    vec![
                        (
                            WidgetState::Disabled.into(),
                            Some(disabled_text_style.clone()),
                        ),
                        (
                            WidgetState::Selected.into(),
                            Some(selected_text_style.clone()),
                        ),
                        (WidgetState::any(), Some(unselected_text_style.clone())),
                    ],
                )),
            );

            // Test selected label text style.
            assert_eq!(
                label_style(&tester, selected_text).font_size,
                selected_text_style.font_size
            );
            assert_eq!(
                label_style(&tester, selected_text).color,
                selected_text_style.color
            );

            // Test unselected label text style.
            assert_eq!(
                label_style(&tester, unselected_text).font_size,
                unselected_text_style.font_size
            );
            assert_eq!(
                label_style(&tester, unselected_text).color,
                unselected_text_style.color
            );

            // Test disabled label text style.
            assert_eq!(
                label_style(&tester, disabled_text).font_size,
                disabled_text_style.font_size
            );
            assert_eq!(
                label_style(&tester, disabled_text).color,
                disabled_text_style.color
            );
        });
    }

    #[test]
    fn navigation_bar_maintain_bottom_view_padding_can_consume_bottom_media_query_padding() {
        with_main_thread_stack(|| {
            let bottom_padding = 40.0;
            let text_direction = TextDirection::Ltr;
            let mut tester = tester();
            pump_app(&mut tester, m3, move |_app| {
                Directionality::new(
                    text_direction,
                    MediaQuery::new(
                        MediaQueryData::new().padding(EdgeInsets::only(
                            0.0,
                            0.0,
                            0.0,
                            bottom_padding,
                        )),
                        Scaffold::new()
                            .bottom_navigation_bar(
                                NavigationBar::new(ac_alarm()).maintain_bottom_view_padding(true),
                            )
                            .into_widget(),
                    )
                    .into_widget(),
                )
                .into_widget()
            });

            let padding = tester.find_by_type::<Padding>()[0];
            let safe_area_bottom_padding = tester.widget(padding, |padding: &Padding| {
                padding.padding.resolve(Some(text_direction)).bottom
            });
            assert_eq!(safe_area_bottom_padding, 0.0);
        });
    }

    #[test]
    fn navigation_bar_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, m3, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(NavigationBar::new([
                        destination(Icons::add(), "X"),
                        destination(Icons::abc(), "Y"),
                    ])))
                    .into_widget()
            });
            assert_eq!(bar_size(&tester), Size::ZERO);
        });
    }
}
