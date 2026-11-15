//! Flutter counterpart: material_ui's `navigation_rail.dart` (and its part
//! `generated/navigation_rail_defaults_m3.g.dart`).

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationRef, AnyAnimation, CurveTween,
    CurvedAnimation, Curves, Drive, Interval, ReverseAnimation,
};
use waft_embedder::SemanticsRole;
use waft_embedder::{Color, Offset, Radius, Rect, lerp_double};
use waft_foundation::{App, Handle, K_IS_WEB, Listener, OptionRetained, Ref, ValueChanged};
use waft_painting::{
    Alignment, AlignmentGeometry, BorderRadius, BoxShape, EdgeInsetsGeometry, ShapeBorderRef,
    TextDirection, TextStyle,
};
use waft_rendering::{BoxConstraints, MainAxisAlignment, MainAxisSize};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    Align, BuildContext, Center, ClipRect, Column, ConstrainedBox, DefaultTextStyle,
    Directionality, FadeTransition, Flexible, IconTheme, IconThemeData, InheritedWidget,
    IntoWidget, KeyRef, MergeSemantics, Padding, Row, SafeArea, Semantics, SingleChildScrollView,
    SizedBox, Stack, State, StateData, StatefulWidget, StatelessWidget, TickerProviderStateMixin,
    TickerProviderStateMixinData, Visibility, WidgetRef,
};

use crate::generated::navigation_rail_defaults_m3::NavigationRailDefaultsM3;
use crate::ink_well::InkResponse;
use crate::material::{Material, MaterialType, RectCallback};
use crate::material_localizations::MaterialLocalizations;
use crate::navigation_bar::NavigationIndicator;
use crate::navigation_rail_theme::{
    NavigationRailTheme, NavigationRailThemeData, NavigationRailThemeDataGetters,
};
use crate::theme::{K_THEME_ANIMATION_DURATION, Theme};
use crate::theme_data_getters::DefaultsContext;

const K_CIRCULAR_INDICATOR_DIAMETER: f64 = 56.0;
const K_INDICATOR_HEIGHT: f64 = 32.0;

/// A Material Design widget that is meant to be displayed at the left or right of an
/// app to navigate between a small number of views, typically between three and
/// five.
///
/// Learn more about [`NavigationRail`] on the [Flutter YouTube channel](https://www.youtube.com/watch?v=y9xchtVTtqQ).
///
/// The navigation rail is meant for layouts with wide viewports, such as a
/// desktop web or tablet landscape layout. For smaller layouts, like mobile
/// portrait, a `BottomNavigationBar` should be used instead.
///
/// A navigation rail is usually used as the first or last element of a `Row`
/// which defines the app's `Scaffold` body.
///
/// The appearance of all of the [`NavigationRail`]s within an app can be
/// specified with [`NavigationRailTheme`]. The default values for null theme
/// properties are based on the `Theme`'s `ThemeData::text_theme`,
/// `ThemeData::icon_theme`, and `ThemeData::color_scheme`.
///
/// Adaptive layouts can build different instances of the `Scaffold` in order to
/// have a navigation rail for more horizontal layouts and a bottom navigation
/// bar for more vertical layouts. See
/// [the adaptive_scaffold.dart sample](https://github.com/flutter/demos/blob/main/web_dashboard/lib/src/widgets/third_party/adaptive_scaffold.dart)
/// for an example.
///
/// See also:
///
///  * `Scaffold`, which can display the navigation rail within a `Row` of the
///    `Scaffold::body` slot.
///  * [`NavigationRailDestination`], which is used as a model to create tappable
///    destinations in the navigation rail.
///  * `BottomNavigationBar`, which is a similar navigation widget that's laid
///    out horizontally.
///  * <https://material.io/components/navigation-rail/>
///  * <https://m3.material.io/components/navigation-rail>
pub struct NavigationRail {
    pub key: Option<KeyRef>,

    /// Sets the color of the Container that holds all of the [`NavigationRail`]'s
    /// contents.
    ///
    /// The default value is `NavigationRailThemeData::background_color`. If
    /// `NavigationRailThemeData::background_color` is null, then the default value
    /// is based on `ColorScheme::surface` of `ThemeData::color_scheme`.
    pub background_color: Option<Color>,

    /// Indicates that the [`NavigationRail`] should be in the extended state.
    ///
    /// The extended state has a wider rail container, and the labels are
    /// positioned next to the icons. [`min_extended_width`](Self::min_extended_width) can be
    /// used to set the minimum width of the rail when it is in this state.
    ///
    /// The rail will implicitly animate between the extended and normal state.
    ///
    /// If the rail is going to be in the extended state, then the
    /// [`label_type`](Self::label_type) must be set to [`NavigationRailLabelType::None`].
    ///
    /// The default value is false.
    pub extended: bool,

    /// The leading widget in the rail that is placed above the destinations.
    ///
    /// It is placed at the top of the rail, above the [`destinations`](Self::destinations).
    /// Its location is not affected by [`group_alignment`](Self::group_alignment).
    ///
    /// This is commonly a `FloatingActionButton`, but may also be a non-button,
    /// such as a logo.
    ///
    /// The default value is null.
    pub leading: Option<WidgetRef>,

    /// The trailing widget in the rail that is placed below the destinations.
    ///
    /// The trailing widget is placed below the last [`NavigationRailDestination`].
    /// It's location is affected by [`group_alignment`](Self::group_alignment).
    ///
    /// This is commonly a list of additional options or destinations that is
    /// usually only rendered when [`extended`](Self::extended) is true.
    ///
    /// The default value is null.
    pub trailing: Option<WidgetRef>,

    /// Defines the appearance of the button items that are arrayed within the
    /// navigation rail.
    ///
    /// The value must be a list of zero or more [`NavigationRailDestination`]
    /// values.
    pub destinations: Vec<NavigationRailDestination>,

    /// The index into [`destinations`](Self::destinations) for the current selected
    /// [`NavigationRailDestination`] or null if no destination is selected.
    pub selected_index: Option<usize>,

    /// Called when one of the [`destinations`](Self::destinations) is selected.
    ///
    /// The stateful widget that creates the navigation rail needs to keep
    /// track of the index of the selected [`NavigationRailDestination`] and call
    /// `set_state` to rebuild the navigation rail with the new
    /// [`selected_index`](Self::selected_index).
    pub on_destination_selected: Option<ValueChanged<usize>>,

    /// The rail's elevation or z-coordinate.
    ///
    /// If `Directionality` is `TextDirection::Ltr`, the inner side is the
    /// right side, and if `Directionality` is `TextDirection::Rtl`, it is
    /// the left side.
    ///
    /// The default value is 0.
    pub elevation: Option<f64>,

    /// The vertical alignment for the group of [`destinations`](Self::destinations) within
    /// the rail.
    ///
    /// The [`NavigationRailDestination`]s are by default grouped together with the
    /// [`trailing`](Self::trailing) widget, due to
    /// [`trailing_at_bottom`](Self::trailing_at_bottom) being `false`. The
    /// [`leading`](Self::leading) widget, can also be in the aligned group by setting
    /// [`leading_at_top`](Self::leading_at_top) to `false`. If these widgets are not
    /// included in the group, they are placed at the top and bottom, respectively, of the
    /// rail and only the space between them is considered for the alignment.
    ///
    /// The value must be between -1.0 and 1.0.
    ///
    /// If [`group_alignment`](Self::group_alignment) is -1.0, then the items are aligned to
    /// the top. If [`group_alignment`](Self::group_alignment) is 0.0, then the items are
    /// aligned to the center. If [`group_alignment`](Self::group_alignment) is 1.0, then the
    /// items are aligned to the bottom.
    ///
    /// The default is -1.0.
    ///
    /// See also:
    ///   * `Alignment::y`
    ///
    pub group_alignment: Option<f64>,

    /// Defines the layout and behavior of the labels for the default, unextended
    /// [`NavigationRail`].
    ///
    /// When a navigation rail is [`extended`](Self::extended), the labels are always shown.
    ///
    /// The default value is `NavigationRailThemeData::label_type`. If
    /// `NavigationRailThemeData::label_type` is null, then the default value is
    /// [`NavigationRailLabelType::None`].
    ///
    /// See also:
    ///
    ///   * [`NavigationRailLabelType`] for information on the meaning of different
    ///     types.
    pub label_type: Option<NavigationRailLabelType>,

    /// The `TextStyle` of a destination's label when it is unselected.
    ///
    /// When one of the [`destinations`](Self::destinations) is selected the
    /// [`selected_label_text_style`](Self::selected_label_text_style) will be used instead.
    ///
    /// The default value is based on the `Theme`'s `TextTheme::body_large`. The
    /// default color is based on the `Theme`'s `ColorScheme::on_surface`.
    ///
    /// Properties from this text style, or
    /// `NavigationRailThemeData::unselected_label_text_style` if this is null, are
    /// merged into the defaults.
    pub unselected_label_text_style: Option<TextStyle>,

    /// The `TextStyle` of a destination's label when it is selected.
    ///
    /// When a [`NavigationRailDestination`] is not selected,
    /// [`unselected_label_text_style`](Self::unselected_label_text_style) will be used.
    ///
    /// The default value is based on the `TextTheme::body_large` of
    /// `ThemeData::text_theme`. The default color is based on the `Theme`'s
    /// `ColorScheme::primary`.
    ///
    /// Properties from this text style,
    /// or `NavigationRailThemeData::selected_label_text_style` if this is null, are
    /// merged into the defaults.
    pub selected_label_text_style: Option<TextStyle>,

    /// The visual properties of the icon in the unselected destination.
    ///
    /// If this field is not provided, or provided with any null properties, then
    /// a copy of the `IconThemeData::fallback` with a custom [`NavigationRail`]
    /// specific color will be used.
    ///
    /// The default value is the `Theme`'s `ThemeData::icon_theme` with a color
    /// of the `Theme`'s `ColorScheme::on_surface` with an opacity of 0.64.
    /// Properties from this icon theme, or
    /// `NavigationRailThemeData::unselected_icon_theme` if this is null, are
    /// merged into the defaults.
    pub unselected_icon_theme: Option<IconThemeData>,

    /// The visual properties of the icon in the selected destination.
    ///
    /// When a [`NavigationRailDestination`] is not selected,
    /// [`unselected_icon_theme`](Self::unselected_icon_theme) will be used.
    ///
    /// The default value is the `Theme`'s `ThemeData::icon_theme` with a color
    /// of the `Theme`'s `ColorScheme::primary`. Properties from this icon theme,
    /// or `NavigationRailThemeData::selected_icon_theme` if this is null, are
    /// merged into the defaults.
    pub selected_icon_theme: Option<IconThemeData>,

    /// The smallest possible width for the rail regardless of the destination's
    /// icon or label size.
    ///
    /// The default is 72.
    ///
    /// This value also defines the min width and min height of the destinations.
    ///
    /// To make a compact rail, set this to 56 and use
    /// [`NavigationRailLabelType::None`].
    pub min_width: Option<f64>,

    /// The final width when the animation is complete for setting
    /// [`extended`](Self::extended) to true.
    ///
    /// This is only used when [`extended`](Self::extended) is set to true.
    ///
    /// The default value is 256.
    pub min_extended_width: Option<f64>,

    /// If `true`, adds a rounded [`NavigationIndicator`] behind the selected
    /// destination's icon.
    ///
    /// The indicator's shape will be circular if [`label_type`](Self::label_type) is
    /// [`NavigationRailLabelType::None`], or a `StadiumBorder` if
    /// [`label_type`](Self::label_type) is [`NavigationRailLabelType::All`] or
    /// [`NavigationRailLabelType::Selected`].
    ///
    /// If `null`, defaults to `NavigationRailThemeData::use_indicator`. If that is
    /// `null`, defaults to `ThemeData::use_material3`.
    pub use_indicator: Option<bool>,

    /// Overrides the default value of [`NavigationRail`]'s selection indicator color,
    /// when [`use_indicator`](Self::use_indicator) is true.
    ///
    /// If this is null, `NavigationRailThemeData::indicator_color` is used. If
    /// that is null, defaults to `ColorScheme::secondary_container`.
    pub indicator_color: Option<Color>,

    /// Overrides the default value of [`NavigationRail`]'s selection indicator shape,
    /// when [`use_indicator`](Self::use_indicator) is true.
    ///
    /// If this is null, `NavigationRailThemeData::indicator_shape` is used. If
    /// that is null, defaults to `StadiumBorder`.
    pub indicator_shape: Option<ShapeBorderRef>,

    /// Pin the [`leading`](Self::leading) widget to the top.
    ///
    /// If `true`, the [`leading`](Self::leading) widget is pinned to the top of the
    /// [`NavigationRail`]. Otherwise it precedes directly the main group of
    /// [`destinations`](Self::destinations).
    ///
    /// See also [`scrollable`](Self::scrollable) for a description of the interplay of
    /// these parameters.
    pub leading_at_top: bool,

    /// Pin the [`trailing`](Self::trailing) widget to the bottom.
    ///
    /// If `true`, the [`trailing`](Self::trailing) widget is pinned to the bottom of the
    /// [`NavigationRail`]. Otherwise it follows directly the main group of
    /// [`destinations`](Self::destinations).
    ///
    /// See also [`scrollable`](Self::scrollable) for a description of the interplay of
    /// these parameters.
    pub trailing_at_bottom: bool,

    /// Whether the main group of items should be scrollable when vertical space
    /// is insufficient to show all of [`destinations`](Self::destinations),
    /// [`leading`](Self::leading) and [`trailing`](Self::trailing).
    ///
    /// If [`leading_at_top`](Self::leading_at_top) or
    /// [`trailing_at_bottom`](Self::trailing_at_bottom) are false,
    /// [`leading`](Self::leading) or [`trailing`](Self::trailing) widgets, respectively,
    /// are part of the main group in addition to [`destinations`](Self::destinations).
    /// Otherwise these are statical at the top or bottom, respectively.
    pub scrollable: bool,

    /// How the [`destinations`](Self::destinations) should be placed along the vertical
    /// axis.
    ///
    /// When there is extra vertical space in the [`NavigationRail`], this
    /// property controls the alignment and spacing of the items. For example,
    /// setting this to `MainAxisAlignment::SpaceEvenly` will distribute the
    /// destinations equally along the available vertical space.
    ///
    /// When this property is not null, [`group_alignment`](Self::group_alignment) is
    /// ignored.
    ///
    /// If null, the layout behaves as if `MainAxisAlignment::Start` was
    /// specified.
    ///
    /// See also:
    ///
    ///  * `Column::main_axis_alignment`, which describes the different values and
    ///    their effects on the layout.
    pub main_axis_alignment: Option<MainAxisAlignment>,
}

impl NavigationRail {
    /// Creates a Material Design navigation rail.
    ///
    /// The value of [`destinations`](Self::destinations) must be a list of zero or more
    /// [`NavigationRailDestination`] values.
    ///
    /// If [`elevation`](Self::elevation) is specified, it must be non-negative.
    ///
    /// If [`min_width`](Self::min_width) is specified, it must be non-negative, and if
    /// [`min_extended_width`](Self::min_extended_width) is specified, it must be
    /// non-negative and greater than [`min_width`](Self::min_width).
    ///
    /// The [`extended`](Self::extended) argument can only be set to true when the
    /// [`label_type`](Self::label_type) is null or [`NavigationRailLabelType::None`].
    ///
    /// If [`background_color`](Self::background_color), [`elevation`](Self::elevation),
    /// [`group_alignment`](Self::group_alignment), [`label_type`](Self::label_type),
    /// [`unselected_label_text_style`](Self::unselected_label_text_style),
    /// [`selected_label_text_style`](Self::selected_label_text_style),
    /// [`unselected_icon_theme`](Self::unselected_icon_theme), or
    /// [`selected_icon_theme`](Self::selected_icon_theme) are null, then their
    /// [`NavigationRailThemeData`] values will be used. If the corresponding
    /// [`NavigationRailThemeData`] property is null, then the navigation rail
    /// defaults are used. See the individual properties for more information.
    ///
    /// Typically used within a `Row` that defines the `Scaffold::body` property.
    pub fn new(
        destinations: Vec<NavigationRailDestination>,
        selected_index: Option<usize>,
    ) -> NavigationRail {
        debug_assert!(selected_index.is_none_or(|index| index < destinations.len()));
        NavigationRail {
            key: None,
            background_color: None,
            extended: false,
            leading: None,
            trailing: None,
            destinations,
            selected_index,
            on_destination_selected: None,
            elevation: None,
            group_alignment: None,
            label_type: None,
            unselected_label_text_style: None,
            selected_label_text_style: None,
            unselected_icon_theme: None,
            selected_icon_theme: None,
            min_width: None,
            min_extended_width: None,
            use_indicator: None,
            indicator_color: None,
            indicator_shape: None,
            leading_at_top: true,
            trailing_at_bottom: false,
            scrollable: false,
            main_axis_alignment: None,
        }
    }

    /// Dart `NavigationRail(key:)`.
    pub fn key(mut self, key: KeyRef) -> NavigationRail {
        self.key = Some(key);
        self
    }

    /// Dart `NavigationRail(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> NavigationRail {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `NavigationRail(extended:)`.
    pub fn extended(mut self, extended: bool) -> NavigationRail {
        self.extended = extended;
        self
    }

    /// Dart `NavigationRail(leading:)`.
    pub fn leading<K>(mut self, leading: impl IntoWidget<K>) -> NavigationRail {
        self.leading = Some(leading.into_widget());
        self
    }

    /// Dart `NavigationRail(trailing:)`.
    pub fn trailing<K>(mut self, trailing: impl IntoWidget<K>) -> NavigationRail {
        self.trailing = Some(trailing.into_widget());
        self
    }

    /// Dart `NavigationRail(onDestinationSelected:)`.
    pub fn on_destination_selected(
        mut self,
        on_destination_selected: impl Fn(&mut App, usize) + 'static,
    ) -> NavigationRail {
        self.on_destination_selected = Some(Rc::new(on_destination_selected));
        self
    }

    /// Dart `NavigationRail(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> NavigationRail {
        debug_assert!(elevation > 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `NavigationRail(groupAlignment:)`.
    pub fn group_alignment(mut self, group_alignment: f64) -> NavigationRail {
        self.group_alignment = Some(group_alignment);
        self
    }

    /// Dart `NavigationRail(labelType:)`.
    pub fn label_type(mut self, label_type: NavigationRailLabelType) -> NavigationRail {
        self.label_type = Some(label_type);
        self
    }

    /// Dart `NavigationRail(unselectedLabelTextStyle:)`.
    pub fn unselected_label_text_style(
        mut self,
        unselected_label_text_style: TextStyle,
    ) -> NavigationRail {
        self.unselected_label_text_style = Some(unselected_label_text_style);
        self
    }

    /// Dart `NavigationRail(selectedLabelTextStyle:)`.
    pub fn selected_label_text_style(
        mut self,
        selected_label_text_style: TextStyle,
    ) -> NavigationRail {
        self.selected_label_text_style = Some(selected_label_text_style);
        self
    }

    /// Dart `NavigationRail(unselectedIconTheme:)`.
    pub fn unselected_icon_theme(mut self, unselected_icon_theme: IconThemeData) -> NavigationRail {
        self.unselected_icon_theme = Some(unselected_icon_theme);
        self
    }

    /// Dart `NavigationRail(selectedIconTheme:)`.
    pub fn selected_icon_theme(mut self, selected_icon_theme: IconThemeData) -> NavigationRail {
        self.selected_icon_theme = Some(selected_icon_theme);
        self
    }

    /// Dart `NavigationRail(minWidth:)`.
    pub fn min_width(mut self, min_width: f64) -> NavigationRail {
        debug_assert!(min_width > 0.0);
        self.min_width = Some(min_width);
        self
    }

    /// Dart `NavigationRail(minExtendedWidth:)`.
    pub fn min_extended_width(mut self, min_extended_width: f64) -> NavigationRail {
        debug_assert!(min_extended_width > 0.0);
        self.min_extended_width = Some(min_extended_width);
        self
    }

    /// Dart `NavigationRail(useIndicator:)`.
    pub fn use_indicator(mut self, use_indicator: bool) -> NavigationRail {
        self.use_indicator = Some(use_indicator);
        self
    }

    /// Dart `NavigationRail(indicatorColor:)`.
    pub fn indicator_color(mut self, indicator_color: Color) -> NavigationRail {
        self.indicator_color = Some(indicator_color);
        self
    }

    /// Dart `NavigationRail(indicatorShape:)`.
    pub fn indicator_shape(mut self, indicator_shape: impl Into<ShapeBorderRef>) -> NavigationRail {
        self.indicator_shape = Some(indicator_shape.into());
        self
    }

    /// Dart `NavigationRail(leadingAtTop:)`.
    pub fn leading_at_top(mut self, leading_at_top: bool) -> NavigationRail {
        self.leading_at_top = leading_at_top;
        self
    }

    /// Dart `NavigationRail(trailingAtBottom:)`.
    pub fn trailing_at_bottom(mut self, trailing_at_bottom: bool) -> NavigationRail {
        self.trailing_at_bottom = trailing_at_bottom;
        self
    }

    /// Dart `NavigationRail(scrollable:)`.
    pub fn scrollable(mut self, scrollable: bool) -> NavigationRail {
        self.scrollable = scrollable;
        self
    }

    /// Dart `NavigationRail(mainAxisAlignment:)`.
    pub fn main_axis_alignment(mut self, main_axis_alignment: MainAxisAlignment) -> NavigationRail {
        self.main_axis_alignment = Some(main_axis_alignment);
        self
    }

    /// Returns the animation that controls the [`NavigationRail::extended`] state.
    ///
    /// This can be used to synchronize animations in the [`leading`](Self::leading) or
    /// [`trailing`](Self::trailing) widget, such as an animated menu or a
    /// `FloatingActionButton` animation.
    pub fn extended_animation(app: &mut App, context: BuildContext) -> AnyAnimation<f64> {
        context
            .depend_on_inherited_widget_of_exact_type::<ExtendedNavigationRailAnimation>(app)
            .expect("a NavigationRail above the context")
            .animation
            .handle()
    }
}

impl Debug for NavigationRail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NavigationRail")
            .field("extended", &self.extended)
            .field("destinations", &self.destinations.len())
            .field("selected_index", &self.selected_index)
            .field("label_type", &self.label_type)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for NavigationRail {
    type State = NavigationRailState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(
            self.min_width.is_none()
                || self.min_extended_width.is_none()
                || self.min_extended_width >= self.min_width
        );
        debug_assert!(
            !self.extended
                || self.label_type.is_none()
                || self.label_type == Some(NavigationRailLabelType::None)
        );
    }

    fn create_state(&self) -> NavigationRailState {
        NavigationRailState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            destination_controllers: Vec::new(),
            destination_animations: Vec::new(),
            extended_controller: None,
            extended_animation: None,
        }
    }
}

/// Dart's `_NavigationRailState`.
pub struct NavigationRailState {
    state: StateData<NavigationRail>,
    ticker_provider: TickerProviderStateMixinData,
    destination_controllers: Vec<Ref<AnimationController>>,
    destination_animations: Vec<AnimationRef<f64>>,
    extended_controller: Option<Ref<AnimationController>>,
    extended_animation: Option<Ref<CurvedAnimation>>,
}

impl TickerProviderStateMixin for NavigationRailState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

impl TickerProviderObject for NavigationRailState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for NavigationRailState {
    type Widget = NavigationRail;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.init_controllers(app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_controllers(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &NavigationRail) {
        let widget = self.widget(app);
        let extended = widget.extended;
        let destinations_len = widget.destinations.len();
        let selected_index = widget.selected_index;
        if extended != old_widget.extended {
            let extended_controller = self.extended_controller(app);
            if extended {
                extended_controller.forward(app, None);
            } else {
                extended_controller.reverse(app, None);
            }
        }

        // No animated segue if the length of the items list changes.
        if destinations_len != old_widget.destinations.len() {
            self.reset_state(app);
            return;
        }

        if selected_index != old_widget.selected_index {
            if let Some(old_selected_index) = old_widget.selected_index {
                app.get(self).destination_controllers[old_selected_index].reverse(app, None);
            }
            if let Some(selected_index) = selected_index {
                app.get(self).destination_controllers[selected_index].forward(app, None);
            }
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let navigation_rail_theme = NavigationRailTheme::of(app, context);
        let defaults: Box<dyn NavigationRailThemeDataGetters> =
            if Theme::of(app, context).use_material3 {
                Box::new(NavigationRailDefaultsM3::new(context))
            } else {
                Box::new(NavigationRailDefaultsM2::new(app, context))
            };
        let localizations = <dyn MaterialLocalizations>::of(app, context);

        let widget_ref = context.widget(app).clone();
        let widget = waft_widgets::downcast_widget::<NavigationRail>(&*widget_ref)
            .expect("a NavigationRailState builds its NavigationRail");
        let background_color = widget
            .background_color
            .or(navigation_rail_theme.background_color)
            .or_else(|| defaults.background_color(app))
            .expect("the defaults give a background color");
        let elevation = widget
            .elevation
            .or(navigation_rail_theme.elevation)
            .or_else(|| defaults.elevation(app))
            .expect("the defaults give an elevation");
        let min_width = widget
            .min_width
            .or(navigation_rail_theme.min_width)
            .or_else(|| defaults.min_width(app))
            .expect("the defaults give a min width");
        let min_extended_width = widget
            .min_extended_width
            .or(navigation_rail_theme.min_extended_width)
            .or_else(|| defaults.min_extended_width(app))
            .expect("the defaults give a min extended width");
        let unselected_label_text_style = widget
            .unselected_label_text_style
            .clone()
            .or(navigation_rail_theme.unselected_label_text_style.clone())
            .or_else(|| defaults.unselected_label_text_style(app))
            .expect("the defaults give an unselected label text style");
        let selected_label_text_style = widget
            .selected_label_text_style
            .clone()
            .or(navigation_rail_theme.selected_label_text_style.clone())
            .or_else(|| defaults.selected_label_text_style(app))
            .expect("the defaults give a selected label text style");
        let unselected_icon_theme = widget
            .unselected_icon_theme
            .clone()
            .or(navigation_rail_theme.unselected_icon_theme.clone())
            .or_else(|| defaults.unselected_icon_theme(app))
            .expect("the defaults give an unselected icon theme");
        let selected_icon_theme = widget
            .selected_icon_theme
            .clone()
            .or(navigation_rail_theme.selected_icon_theme.clone())
            .or_else(|| defaults.selected_icon_theme(app))
            .expect("the defaults give a selected icon theme");
        let group_alignment = widget
            .group_alignment
            .or(navigation_rail_theme.group_alignment)
            .or_else(|| defaults.group_alignment(app))
            .expect("the defaults give a group alignment");
        let label_type = widget
            .label_type
            .or(navigation_rail_theme.label_type)
            .or_else(|| defaults.label_type(app))
            .expect("the defaults give a label type");
        let use_indicator = widget
            .use_indicator
            .or(navigation_rail_theme.use_indicator)
            .or_else(|| defaults.use_indicator(app))
            .expect("the defaults give use_indicator");
        let indicator_color = widget
            .indicator_color
            .or(navigation_rail_theme.indicator_color)
            .or_else(|| defaults.indicator_color(app));
        let indicator_shape = widget
            .indicator_shape
            .clone()
            .or(navigation_rail_theme.indicator_shape.clone())
            .or_else(|| defaults.indicator_shape(app));

        // For backwards compatibility, in M2 the opacity of the unselected icons needs
        // to be set to the default if it isn't in the given theme. This can be removed
        // when Material 3 is the default.
        let effective_unselected_icon_theme = if Theme::of(app, context).use_material3 {
            unselected_icon_theme
        } else {
            let opacity = unselected_icon_theme.opacity.or_else(|| {
                defaults
                    .unselected_icon_theme(app)
                    .expect("the M2 defaults give an unselected icon theme")
                    .opacity
            });
            match opacity {
                Some(opacity) => unselected_icon_theme.copy_with().opacity(opacity),
                None => unselected_icon_theme.copy_with(),
            }
        };

        let is_rtl_direction = Directionality::of(app, context) == TextDirection::Rtl;

        let state = app.get(self);
        let extended_animation: AnimationRef<f64> = state
            .extended_animation
            .as_ref()
            .expect("created in init_state")
            .erase();
        let destination_animations = state.destination_animations.clone();
        let widget = self.widget(app);
        let destinations_len = widget.destinations.len();

        let mut main_group_children = Vec::new();
        if !widget.leading_at_top
            && let Some(leading) = &widget.leading
        {
            main_group_children.push(leading.clone());
            main_group_children.push(vertical_spacer());
        }
        let rail_destinations: Vec<WidgetRef> = widget
            .destinations
            .iter()
            .enumerate()
            .map(|(i, destination)| {
                let selected = widget.selected_index == Some(i);
                let on_destination_selected = widget.on_destination_selected.clone();
                RailDestination {
                    min_width,
                    min_extended_width,
                    extended_transition_animation: extended_animation.clone(),
                    selected,
                    icon: if selected {
                        destination.selected_icon.clone()
                    } else {
                        destination.icon.clone()
                    },
                    label: destination.label.clone(),
                    destination_animation: destination_animations[i].clone(),
                    label_type,
                    icon_theme: if selected {
                        selected_icon_theme.clone()
                    } else {
                        effective_unselected_icon_theme.clone()
                    },
                    label_text_style: if selected {
                        selected_label_text_style.clone()
                    } else {
                        unselected_label_text_style.clone()
                    },
                    padding: destination.padding,
                    use_indicator,
                    indicator_color: if use_indicator { indicator_color } else { None },
                    indicator_shape: if use_indicator {
                        indicator_shape.clone()
                    } else {
                        None
                    },
                    on_tap: Listener::new(move |app| {
                        if let Some(on_destination_selected) = &on_destination_selected {
                            on_destination_selected(app, i);
                        }
                    }),
                    index_label: localizations.tab_label(i as i32 + 1, destinations_len as i32),
                    disabled: destination.disabled,
                }
                .into_widget()
            })
            .collect();
        main_group_children.push(
            Semantics::new()
                .role(SemanticsRole::TabBar)
                .explicit_child_nodes(true)
                .container(true)
                .child(
                    Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .children(rail_destinations),
                )
                .into_widget(),
        );
        if !widget.trailing_at_bottom
            && let Some(trailing) = &widget.trailing
        {
            main_group_children.push(trailing.clone());
        }
        let mut main_group = Column::new()
            .main_axis_size(if widget.main_axis_alignment.is_some() {
                MainAxisSize::Max
            } else {
                MainAxisSize::Min
            })
            .main_axis_alignment(
                widget
                    .main_axis_alignment
                    .unwrap_or(MainAxisAlignment::Start),
            )
            .children(main_group_children)
            .into_widget();

        if widget.scrollable {
            main_group = SingleChildScrollView::new().child(main_group).into_widget();
        }

        let mut column_children = vec![vertical_spacer()];
        if widget.leading_at_top
            && let Some(leading) = &widget.leading
        {
            column_children.push(leading.clone());
            column_children.push(vertical_spacer());
        }
        column_children.push(
            Flexible::new(
                Align::new()
                    .alignment(AlignmentGeometry::Alignment(Alignment::new(
                        0.0,
                        group_alignment,
                    )))
                    .child(main_group),
            )
            .into_widget(),
        );
        if widget.trailing_at_bottom
            && let Some(trailing) = &widget.trailing
        {
            column_children.push(trailing.clone());
        }

        Semantics::new()
            .container(true)
            .child(ExtendedNavigationRailAnimation {
                animation: extended_animation.clone(),
                child: Semantics::new()
                    .explicit_child_nodes(true)
                    .child(
                        Material::new()
                            .elevation(elevation)
                            .color(background_color)
                            .child(
                                SafeArea::new(Column::new().children(column_children))
                                    .right(is_rtl_direction)
                                    .left(!is_rtl_direction),
                            ),
                    )
                    .into_widget(),
            })
            .into_widget()
    }
}

impl NavigationRailState {
    fn extended_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .extended_controller
            .handle()
            .expect("created in init_state")
    }

    fn dispose_controllers(self: Handle<Self>, app: &mut App) {
        let state = app.get(self);
        let destination_controllers = state.destination_controllers.clone();
        let extended_controller = state
            .extended_controller
            .handle()
            .expect("created in init_state");
        let extended_animation = state
            .extended_animation
            .handle()
            .expect("created in init_state");
        for controller in destination_controllers {
            controller.dispose(app);
        }
        extended_controller.dispose(app);
        extended_animation.dispose(app);
    }

    fn init_controllers(self: Handle<Self>, app: &mut App) {
        let destinations_len = self.widget(app).destinations.len();
        let destination_controllers: Vec<Ref<AnimationController>> = (0..destinations_len)
            .map(|_| {
                let controller = AnimationController::create(
                    app,
                    None,
                    Some(K_THEME_ANIMATION_DURATION),
                    None,
                    0.0,
                    1.0,
                    AnimationBehavior::Normal,
                    self,
                );
                Animation::add_listener(
                    controller.handle(),
                    app,
                    Listener::handle_method(self, Self::rebuild),
                );
                controller
            })
            .collect();
        let destination_animations = destination_controllers
            .iter()
            .map(|controller| controller.erase())
            .collect();
        if let Some(selected_index) = self.widget(app).selected_index {
            destination_controllers[selected_index].set_value(app, 1.0);
        }
        let extended = self.widget(app).extended;
        let extended_controller = AnimationController::create(
            app,
            Some(if extended { 1.0 } else { 0.0 }),
            Some(K_THEME_ANIMATION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let extended_animation = CurvedAnimation::create(
            app,
            extended_controller.erase(),
            Curves::ease_in_out(),
            None,
        );
        Animation::add_listener(
            extended_controller.handle(),
            app,
            Listener::handle_method(self, Self::rebuild),
        );
        let state = app.get_mut(self);
        state.destination_controllers = destination_controllers;
        state.destination_animations = destination_animations;
        state.extended_controller = Some(extended_controller);
        state.extended_animation = Some(extended_animation);
    }

    fn reset_state(self: Handle<Self>, app: &mut App) {
        self.dispose_controllers(app);
        self.init_controllers(app);
    }

    fn rebuild(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {
            // Rebuilding when any of the controllers tick, i.e. when the items are
            // animating.
        });
    }
}

/// Dart's `_RailDestination`.
struct RailDestination {
    min_width: f64,
    min_extended_width: f64,
    icon: WidgetRef,
    label: WidgetRef,
    destination_animation: AnimationRef<f64>,
    label_type: NavigationRailLabelType,
    selected: bool,
    extended_transition_animation: AnimationRef<f64>,
    icon_theme: IconThemeData,
    label_text_style: TextStyle,
    on_tap: Listener,
    index_label: String,
    padding: Option<EdgeInsetsGeometry>,
    use_indicator: bool,
    indicator_color: Option<Color>,
    indicator_shape: Option<ShapeBorderRef>,
    disabled: bool,
}

impl Debug for RailDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_RailDestination")
            .field("label_type", &self.label_type)
            .field("selected", &self.selected)
            .field("disabled", &self.disabled)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for RailDestination {
    type State = RailDestinationState;

    fn create_state(&self) -> RailDestinationState {
        RailDestinationState {
            state: StateData::new(),
            position_animation: None,
        }
    }
}

/// Dart's `_RailDestinationState`.
struct RailDestinationState {
    state: StateData<RailDestination>,
    /// Dart's `_positionAnimation`, a `CurvedAnimation` over a `ReverseAnimation`.
    position_animation: Option<Ref<CurvedAnimation>>,
}

impl RailDestinationState {
    fn set_position_animation(self: Handle<Self>, app: &mut App) {
        let destination_animation = self.widget(app).destination_animation.clone();
        let reversed = ReverseAnimation::new(app, destination_animation);
        let curved = CurvedAnimation::create(
            app,
            reversed.erase(),
            Curves::ease_in_out(),
            Some(Rc::new(Curves::ease_in_out().flipped())),
        );
        app.get_mut(self).position_animation = Some(curved);
    }

    fn dispose_position_animation(self: Handle<Self>, app: &mut App) {
        let position_animation = app
            .get_mut(self)
            .position_animation
            .take()
            .expect("set in init_state");
        position_animation.dispose(app);
    }

    fn position_animation_value(self: Handle<Self>, app: &App) -> f64 {
        app.get(self)
            .position_animation
            .as_ref()
            .expect("set in init_state")
            .value(app)
    }

    /// Dart's `parent.drive(CurveTween(curve: Interval(begin, end)))`.
    fn label_fade_animation(
        app: &mut App,
        parent: AnimationRef<f64>,
        begin: f64,
        end: f64,
    ) -> AnimationRef<f64> {
        let curve_tween =
            CurveTween::new(app, Rc::new(Interval::new(begin, end, Curves::linear())));
        parent.drive(app, curve_tween)
    }
}

impl State for RailDestinationState {
    type Widget = RailDestination;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.set_position_animation(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &RailDestination) {
        if self.widget(app).destination_animation != old_widget.destination_animation {
            self.dispose_position_animation(app);
            self.set_position_animation(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_position_animation(app);
    }

    #[expect(deprecated, reason = "Dart calls the deprecated `Color.withOpacity`")]
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        {
            let widget = self.widget(app);
            debug_assert!(
                widget.use_indicator || widget.indicator_color.is_none(),
                "[NavigationRail.indicatorColor] does not have an effect when \
                 [NavigationRail.useIndicator] is false"
            );
        }

        let theme = Theme::of(app, context);
        let text_direction = Directionality::of(app, context);
        let material3 = theme.use_material3;
        let m2_min_width = NavigationRailDefaultsM2::new(app, context)
            .min_width(app)
            .expect("the M2 defaults give a min width");
        let position_animation_value = self.position_animation_value(app);
        let extended_transition_value = self.widget(app).extended_transition_animation.value(app);

        let widget = self.widget(app);
        let destination_padding = widget
            .padding
            .unwrap_or(EdgeInsetsGeometry::ZERO)
            .resolve(Some(text_direction));
        let indicator_offset: Offset;
        let mut apply_x_offset = false;

        let themed_icon = IconTheme::new(
            if widget.disabled {
                widget
                    .icon_theme
                    .copy_with()
                    .color(theme.color_scheme.on_surface.with_opacity(0.38))
            } else {
                widget.icon_theme.clone()
            },
            widget.icon.clone(),
        )
        .into_widget();
        let styled_label = DefaultTextStyle::new(
            if widget.disabled {
                widget
                    .label_text_style
                    .copy_with()
                    .color(theme.color_scheme.on_surface.with_opacity(0.38))
            } else {
                widget.label_text_style.clone()
            },
            widget.label.clone(),
        )
        .into_widget();

        let content: WidgetRef;

        // The indicator height is fixed and equal to _kIndicatorHeight.
        // When the icon height is larger than the indicator height the indicator
        // vertical offset is used to vertically center the indicator.
        let is_large_icon_size = widget
            .icon_theme
            .size
            .is_some_and(|size| size > K_INDICATOR_HEIGHT);
        let indicator_vertical_offset = if is_large_icon_size {
            (widget.icon_theme.size.expect("checked above") - K_INDICATOR_HEIGHT) / 2.0
        } else {
            0.0
        };

        let add_indicator = |is_circular: bool, child: WidgetRef| AddIndicator {
            add_indicator: widget.use_indicator,
            indicator_color: widget.indicator_color,
            indicator_shape: widget.indicator_shape.clone(),
            is_circular,
            indicator_animation: widget.destination_animation.clone(),
            child,
        };

        match widget.label_type {
            NavigationRailLabelType::None => {
                // Split the destination spacing across the top and bottom to keep the icon
                // centered.
                let spacing = material3.then(|| {
                    SizedBox::new()
                        .height(VERTICAL_DESTINATION_SPACING_M3 / 2.0)
                        .into_widget()
                });
                indicator_offset = Offset::new(
                    widget.min_width / 2.0 + destination_padding.left,
                    VERTICAL_DESTINATION_SPACING_M3 / 2.0
                        + destination_padding.top
                        + indicator_vertical_offset,
                );
                let mut icon_box = SizedBox::new().width(widget.min_width);
                if !material3 {
                    icon_box = icon_box.height(widget.min_width);
                }
                let icon_part = Column::new()
                    .children(
                        spacing
                            .clone()
                            .into_iter()
                            .chain([icon_box
                                .child(Center::new().child(add_indicator(!material3, themed_icon)))
                                .into_widget()])
                            .chain(spacing),
                    )
                    .into_widget();
                if extended_transition_value == 0.0 {
                    content = Padding::new(widget.padding.unwrap_or(EdgeInsetsGeometry::ZERO))
                        .child(
                            Stack::new().children([
                                icon_part,
                                // For semantics when label is not showing,
                                SizedBox::shrink()
                                    .child(
                                        Visibility::maintain(widget.label.clone()).visible(false),
                                    )
                                    .into_widget(),
                            ]),
                        )
                        .into_widget();
                } else {
                    let extended_transition_animation =
                        widget.extended_transition_animation.clone();
                    let min_width = widget.min_width;
                    let min_extended_width = widget.min_extended_width;
                    let padding = widget.padding.unwrap_or(EdgeInsetsGeometry::ZERO);
                    let label_fade_animation = RailDestinationState::label_fade_animation(
                        app,
                        extended_transition_animation,
                        0.0,
                        0.25,
                    );
                    apply_x_offset = true;
                    content = Padding::new(padding)
                        .child(
                            ConstrainedBox::new(
                                BoxConstraints::new().min_width(
                                    lerp_double(
                                        Some(min_width),
                                        Some(min_extended_width),
                                        extended_transition_value,
                                    )
                                    .expect("both ends given"),
                                ),
                            )
                            .child(
                                ClipRect::new().child(
                                    Row::new().main_axis_size(MainAxisSize::Min).children([
                                        icon_part,
                                        Flexible::new(
                                            Align::new()
                                                .height_factor(1.0)
                                                .width_factor(extended_transition_value)
                                                .alignment(AlignmentGeometry::CENTER_START)
                                                .child(
                                                    FadeTransition::new(label_fade_animation)
                                                        .child(styled_label),
                                                ),
                                        )
                                        .into_widget(),
                                        SizedBox::new()
                                            .width(
                                                HORIZONTAL_DESTINATION_PADDING
                                                    * extended_transition_value,
                                            )
                                            .into_widget(),
                                    ]),
                                ),
                            ),
                        )
                        .into_widget();
                }
            }
            NavigationRailLabelType::Selected => {
                let appearing_animation_value = 1.0 - position_animation_value;
                let vertical_padding = lerp_double(
                    Some(VERTICAL_DESTINATION_PADDING_NO_LABEL),
                    Some(VERTICAL_DESTINATION_PADDING_WITH_LABEL),
                    appearing_animation_value,
                )
                .expect("both ends given");
                let (interval_begin, interval_end) = if widget.selected {
                    (0.25, 0.75)
                } else {
                    (0.75, 1.0)
                };
                let min_height = if material3 { 0.0 } else { widget.min_width };
                let top_spacing = SizedBox::new()
                    .height(if material3 { 0.0 } else { vertical_padding })
                    .into_widget();
                let label_spacing = SizedBox::new()
                    .height(if material3 {
                        lerp_double(
                            Some(0.0),
                            Some(VERTICAL_ICON_LABEL_SPACING_M3),
                            appearing_animation_value,
                        )
                        .expect("both ends given")
                    } else {
                        0.0
                    })
                    .into_widget();
                let bottom_spacing = SizedBox::new()
                    .height(if material3 {
                        VERTICAL_DESTINATION_SPACING_M3
                    } else {
                        vertical_padding
                    })
                    .into_widget();
                let indicator_horizontal_padding =
                    (destination_padding.left / 2.0) - (destination_padding.right / 2.0);
                let indicator_vertical_padding = destination_padding.top;
                let mut offset = Offset::new(
                    widget.min_width / 2.0 + indicator_horizontal_padding,
                    indicator_vertical_padding + indicator_vertical_offset,
                );
                if widget.min_width < m2_min_width {
                    offset = Offset::new(
                        widget.min_width / 2.0 + HORIZONTAL_DESTINATION_SPACING_M3,
                        indicator_vertical_padding + indicator_vertical_offset,
                    );
                }
                indicator_offset = offset;
                let indicator = add_indicator(false, themed_icon).into_widget();
                let min_width = widget.min_width;
                let padding = widget.padding.unwrap_or(EdgeInsetsGeometry::symmetric(
                    0.0,
                    HORIZONTAL_DESTINATION_PADDING,
                ));
                let destination_animation = widget.destination_animation.clone();
                let label_fade_animation = RailDestinationState::label_fade_animation(
                    app,
                    destination_animation,
                    interval_begin,
                    interval_end,
                );
                content = ConstrainedBox::new(
                    BoxConstraints::new()
                        .min_width(min_width)
                        .min_height(min_height),
                )
                .child(
                    Padding::new(padding).child(
                        ClipRect::new().child(
                            Column::new()
                                .main_axis_size(MainAxisSize::Min)
                                .main_axis_alignment(MainAxisAlignment::Center)
                                .children([
                                    top_spacing,
                                    indicator,
                                    label_spacing,
                                    Align::new()
                                        .alignment(AlignmentGeometry::TOP_CENTER)
                                        .height_factor(appearing_animation_value)
                                        .width_factor(1.0)
                                        .child(
                                            FadeTransition::new(label_fade_animation)
                                                .child(styled_label),
                                        )
                                        .into_widget(),
                                    bottom_spacing,
                                ]),
                        ),
                    ),
                )
                .into_widget();
            }
            NavigationRailLabelType::All => {
                let min_height = if material3 { 0.0 } else { widget.min_width };
                let top_spacing = SizedBox::new()
                    .height(if material3 {
                        0.0
                    } else {
                        VERTICAL_DESTINATION_PADDING_WITH_LABEL
                    })
                    .into_widget();
                let label_spacing = SizedBox::new()
                    .height(if material3 {
                        VERTICAL_ICON_LABEL_SPACING_M3
                    } else {
                        0.0
                    })
                    .into_widget();
                let bottom_spacing = SizedBox::new()
                    .height(if material3 {
                        VERTICAL_DESTINATION_SPACING_M3
                    } else {
                        VERTICAL_DESTINATION_PADDING_WITH_LABEL
                    })
                    .into_widget();
                let indicator_horizontal_padding =
                    (destination_padding.left / 2.0) - (destination_padding.right / 2.0);
                let indicator_vertical_padding = destination_padding.top;
                let mut offset = Offset::new(
                    widget.min_width / 2.0 + indicator_horizontal_padding,
                    indicator_vertical_padding + indicator_vertical_offset,
                );
                if widget.min_width < m2_min_width {
                    offset = Offset::new(
                        widget.min_width / 2.0 + HORIZONTAL_DESTINATION_SPACING_M3,
                        indicator_vertical_padding + indicator_vertical_offset,
                    );
                }
                indicator_offset = offset;
                content = ConstrainedBox::new(
                    BoxConstraints::new()
                        .min_width(widget.min_width)
                        .min_height(min_height),
                )
                .child(
                    Padding::new(widget.padding.unwrap_or(EdgeInsetsGeometry::symmetric(
                        0.0,
                        HORIZONTAL_DESTINATION_PADDING,
                    )))
                    .child(Column::new().children([
                        top_spacing,
                        add_indicator(false, themed_icon).into_widget(),
                        label_spacing,
                        styled_label,
                        bottom_spacing,
                    ])),
                )
                .into_widget();
            }
        }

        let colors = &Theme::of(app, context).color_scheme;
        let primary_color_alpha_modified = (colors.primary.alpha() as f64) < 255.0;
        let effective_splash_color = if primary_color_alpha_modified {
            colors.primary
        } else {
            colors.primary.with_opacity(0.12)
        };
        let effective_hover_color = if primary_color_alpha_modified {
            colors.primary
        } else {
            colors.primary.with_opacity(0.04)
        };
        let widget = self.widget(app);
        let mut stack_children = vec![
            Material::new()
                .r#type(MaterialType::Transparency)
                .child(indicator_ink_well(IndicatorInkWell {
                    child: content,
                    on_tap: (!widget.disabled).then(|| widget.on_tap.clone()),
                    border_radius: BorderRadius::all(Radius::circular(widget.min_width / 2.0)),
                    custom_border: widget.indicator_shape.clone(),
                    splash_color: effective_splash_color,
                    hover_color: effective_hover_color,
                    use_material3: material3,
                    indicator_offset,
                    apply_x_offset,
                    text_direction,
                }))
                .into_widget(),
        ];
        if !K_IS_WEB {
            stack_children.push(
                Semantics::new()
                    .label(widget.index_label.clone())
                    .into_widget(),
            );
        }
        let mut semantics = Semantics::new()
            .role(SemanticsRole::Tab)
            .selected(widget.selected);
        if widget.disabled {
            semantics = semantics.enabled(false);
        }
        MergeSemantics::new()
            .child(semantics.child(Stack::new().children(stack_children)))
            .into_widget()
    }
}

/// The arguments of Dart's `_IndicatorInkWell`, an `InkResponse` subclass.
struct IndicatorInkWell {
    child: WidgetRef,
    on_tap: Option<Listener>,
    custom_border: Option<ShapeBorderRef>,
    border_radius: BorderRadius,
    splash_color: Color,
    hover_color: Color,
    use_material3: bool,
    // The offset used to position Ink highlight.
    indicator_offset: Offset,
    // Whether the horizontal offset from indicatorOffset should be used to position Ink
    // highlight. If true, Ink highlight uses the indicator horizontal offset. If false, Ink
    // highlight is centered horizontally.
    apply_x_offset: bool,
    // The text direction used to adjust the indicator horizontal offset.
    text_direction: TextDirection,
}

/// Dart's `_IndicatorInkWell`: the `InkResponse` its constructor makes, with its
/// `getRectCallback` override.
fn indicator_ink_well(ink_well: IndicatorInkWell) -> InkResponse {
    let IndicatorInkWell {
        child,
        on_tap,
        custom_border,
        border_radius,
        splash_color,
        hover_color,
        use_material3,
        indicator_offset,
        apply_x_offset,
        text_direction,
    } = ink_well;
    let mut response = InkResponse::new()
        .child(child)
        .splash_color(splash_color)
        .hover_color(hover_color)
        .contained_ink_well(true)
        .highlight_shape(BoxShape::Rectangle)
        .override_get_rect_callback(move |app, reference_box| -> Option<RectCallback> {
            if use_material3 {
                let box_width = reference_box.size(app).width();
                let mut indicator_horizontal_center = if apply_x_offset {
                    indicator_offset.dx()
                } else {
                    box_width / 2.0
                };
                if text_direction == TextDirection::Rtl {
                    indicator_horizontal_center = box_width - indicator_horizontal_center;
                }
                return Some(Rc::new(move |_app| {
                    Rect::from_ltwh(
                        indicator_horizontal_center - (K_CIRCULAR_INDICATOR_DIAMETER / 2.0),
                        indicator_offset.dy(),
                        K_CIRCULAR_INDICATOR_DIAMETER,
                        K_INDICATOR_HEIGHT,
                    )
                }));
            }
            None
        });
    if let Some(on_tap) = on_tap {
        response = response.on_tap(on_tap);
    }
    if !use_material3 {
        response = response.border_radius(border_radius);
    }
    if use_material3 && let Some(custom_border) = custom_border {
        response = response.custom_border(custom_border);
    }
    response
}

/// When `add_indicator` is `true`, puts `child` center aligned in a `Stack` with
/// a [`NavigationIndicator`] behind it, otherwise returns `child`.
///
/// When `is_circular` is true, the indicator will be a circle, otherwise the
/// indicator will be a stadium shape.
struct AddIndicator {
    add_indicator: bool,
    is_circular: bool,
    indicator_color: Option<Color>,
    indicator_shape: Option<ShapeBorderRef>,
    indicator_animation: AnimationRef<f64>,
    child: WidgetRef,
}

impl Debug for AddIndicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_AddIndicator")
            .field("add_indicator", &self.add_indicator)
            .field("is_circular", &self.is_circular)
            .field("indicator_color", &self.indicator_color)
            .field("indicator_shape", &self.indicator_shape)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for AddIndicator {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        if !self.add_indicator {
            return self.child.clone();
        }
        let indicator = if self.is_circular {
            let mut indicator = NavigationIndicator::new(self.indicator_animation.clone())
                .height(K_CIRCULAR_INDICATOR_DIAMETER)
                .width(K_CIRCULAR_INDICATOR_DIAMETER)
                .border_radius(BorderRadius::all(Radius::circular(
                    K_CIRCULAR_INDICATOR_DIAMETER / 2.0,
                )));
            if let Some(color) = self.indicator_color {
                indicator = indicator.color(color);
            }
            indicator
        } else {
            let mut indicator = NavigationIndicator::new(self.indicator_animation.clone())
                .width(K_CIRCULAR_INDICATOR_DIAMETER);
            if let Some(shape) = self.indicator_shape.clone() {
                indicator = indicator.shape(shape);
            }
            if let Some(color) = self.indicator_color {
                indicator = indicator.color(color);
            }
            indicator
        };

        Stack::new()
            .alignment(AlignmentGeometry::CENTER)
            .children([indicator.into_widget(), self.child.clone()])
            .into_widget()
    }
}

/// Defines the behavior of the labels of a [`NavigationRail`].
///
/// See also:
///
///   * [`NavigationRail`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavigationRailLabelType {
    /// Only the [`NavigationRailDestination`]s are shown.
    None,

    /// Only the selected [`NavigationRailDestination`] will show its label.
    ///
    /// The label will animate in and out as new [`NavigationRailDestination`]s are
    /// selected.
    Selected,

    /// All [`NavigationRailDestination`]s will show their label.
    All,
}

/// Defines a [`NavigationRail`] button that represents one "destination" view.
///
/// See also:
///
///  * [`NavigationRail`]
#[derive(Clone, Debug)]
pub struct NavigationRailDestination {
    /// The icon of the destination.
    ///
    /// Typically the icon is an `Icon` or an `ImageIcon` widget. If another type
    /// of widget is provided then it should configure itself to match the current
    /// `IconTheme` size and color.
    ///
    /// If [`selected_icon`](Self::selected_icon) is provided, this will only be displayed
    /// when the destination is not selected.
    ///
    /// To make the [`NavigationRail`] more accessible, consider choosing an
    /// icon with a stroked and filled version, such as `Icons::cloud` and
    /// `Icons::cloud_queue`. The [`icon`](Self::icon) should be set to the stroked version
    /// and [`selected_icon`](Self::selected_icon) to the filled version.
    pub icon: WidgetRef,

    /// An alternative icon displayed when this destination is selected.
    ///
    /// If this icon is not provided, the [`NavigationRail`] will display
    /// [`icon`](Self::icon) in either state. The size, color, and opacity of the
    /// [`NavigationRail::selected_icon_theme`] will still apply.
    ///
    /// See also:
    ///
    ///  * [`NavigationRailDestination::icon`], for a description of how to pair
    ///    icons.
    pub selected_icon: WidgetRef,

    /// The color of the [`indicator_shape`](Self::indicator_shape) when this destination is
    /// selected.
    pub indicator_color: Option<Color>,

    /// The shape of the selection indicator.
    pub indicator_shape: Option<ShapeBorderRef>,

    /// The label for the destination.
    ///
    /// The label must be provided when used with the [`NavigationRail`]. When the
    /// [`NavigationRail::label_type`] is [`NavigationRailLabelType::None`], the label is
    /// still used for semantics, and may still be used if
    /// [`NavigationRail::extended`] is true.
    pub label: WidgetRef,

    /// The amount of space to inset the destination item.
    pub padding: Option<EdgeInsetsGeometry>,

    /// Indicates that this destination is inaccessible.
    pub disabled: bool,
}

impl NavigationRailDestination {
    /// Creates a destination that is used with [`NavigationRail::destinations`].
    ///
    /// When the [`NavigationRail::label_type`] is [`NavigationRailLabelType::None`], the
    /// label is still used for semantics, and may still be used if
    /// [`NavigationRail::extended`] is true.
    pub fn new<I, L>(
        icon: impl IntoWidget<I>,
        label: impl IntoWidget<L>,
    ) -> NavigationRailDestination {
        let icon = icon.into_widget();
        NavigationRailDestination {
            selected_icon: icon.clone(),
            icon,
            indicator_color: None,
            indicator_shape: None,
            label: label.into_widget(),
            padding: None,
            disabled: false,
        }
    }

    /// Dart `NavigationRailDestination(selectedIcon:)`.
    pub fn selected_icon<K>(
        mut self,
        selected_icon: impl IntoWidget<K>,
    ) -> NavigationRailDestination {
        self.selected_icon = selected_icon.into_widget();
        self
    }

    /// Dart `NavigationRailDestination(indicatorColor:)`.
    pub fn indicator_color(mut self, indicator_color: Color) -> NavigationRailDestination {
        self.indicator_color = Some(indicator_color);
        self
    }

    /// Dart `NavigationRailDestination(indicatorShape:)`.
    pub fn indicator_shape(
        mut self,
        indicator_shape: impl Into<ShapeBorderRef>,
    ) -> NavigationRailDestination {
        self.indicator_shape = Some(indicator_shape.into());
        self
    }

    /// Dart `NavigationRailDestination(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> NavigationRailDestination {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `NavigationRailDestination(disabled:)`.
    pub fn disabled(mut self, disabled: bool) -> NavigationRailDestination {
        self.disabled = disabled;
        self
    }
}

/// Dart's `_ExtendedNavigationRailAnimation`.
#[derive(Debug)]
struct ExtendedNavigationRailAnimation {
    animation: AnimationRef<f64>,
    child: WidgetRef,
}

impl InheritedWidget for ExtendedNavigationRailAnimation {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &ExtendedNavigationRailAnimation) -> bool {
        self.animation != old.animation
    }
}

// There don't appear to be tokens for these values, but they are
// shown in the spec.
const HORIZONTAL_DESTINATION_PADDING: f64 = 8.0;
const VERTICAL_DESTINATION_PADDING_NO_LABEL: f64 = 24.0;
const VERTICAL_DESTINATION_PADDING_WITH_LABEL: f64 = 16.0;
/// Dart's `const Widget _verticalSpacer = SizedBox(height: 8.0)`.
fn vertical_spacer() -> WidgetRef {
    SizedBox::new().height(8.0).into_widget()
}
const VERTICAL_ICON_LABEL_SPACING_M3: f64 = 4.0;
const VERTICAL_DESTINATION_SPACING_M3: f64 = 12.0;
const HORIZONTAL_DESTINATION_SPACING_M3: f64 = 12.0;

/// Dart's `_NavigationRailDefaultsM2`.
struct NavigationRailDefaultsM2 {
    /// Dart's `super(..)` arguments.
    navigation_rail_theme_data: NavigationRailThemeData,
    context: DefaultsContext,
}

impl NavigationRailDefaultsM2 {
    fn new(app: &mut App, context: BuildContext) -> NavigationRailDefaultsM2 {
        NavigationRailDefaultsM2 {
            navigation_rail_theme_data: NavigationRailThemeData::new()
                .elevation(0.0)
                .group_alignment(-1.0)
                .label_type(NavigationRailLabelType::None)
                .use_indicator(false)
                .min_width(72.0)
                .min_extended_width(256.0),
            context: DefaultsContext::with_theme(app, context),
        }
    }
}

#[expect(deprecated, reason = "Dart calls the deprecated `Color.withOpacity`")]
impl NavigationRailThemeDataGetters for NavigationRailDefaultsM2 {
    fn navigation_rail_theme_data(&self) -> &NavigationRailThemeData {
        &self.navigation_rail_theme_data
    }

    fn background_color(&self, app: &mut App) -> Option<Color> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(colors.surface)
    }

    fn unselected_label_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(
            theme
                .text_theme
                .body_large
                .as_ref()
                .expect("the text theme has bodyLarge")
                .copy_with()
                .color(colors.on_surface.with_opacity(0.64)),
        )
    }

    fn selected_label_text_style(&self, app: &mut App) -> Option<TextStyle> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(
            theme
                .text_theme
                .body_large
                .as_ref()
                .expect("the text theme has bodyLarge")
                .copy_with()
                .color(colors.primary),
        )
    }

    fn unselected_icon_theme(&self, app: &mut App) -> Option<IconThemeData> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(
            IconThemeData::new()
                .size(24.0)
                .color(colors.on_surface)
                .opacity(0.64),
        )
    }

    fn selected_icon_theme(&self, app: &mut App) -> Option<IconThemeData> {
        let theme = self.context.theme(app);
        let colors = &theme.color_scheme;
        Some(
            IconThemeData::new()
                .size(24.0)
                .color(colors.primary)
                .opacity(1.0),
        )
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/navigation_rail_test.dart` and laid out in FlutterTest, whose glyphs
    //! are squares of the font size (`install_platform_fonts`).

    use std::time::Duration;
    use waft_painting::ShapeBorder;

    use waft_embedder::{FontWeight, Size};
    use waft_rendering::{AnyRenderBox, RenderParagraph};
    use waft_widgets::{
        AnyElement, Builder, Expanded, GlobalKey, Icon, IconData, MediaQuery, RichText, Text,
    };

    use waft_painting::EdgeInsets;

    use super::*;
    use crate::colors::Colors;
    use crate::floating_action_button::FloatingActionButton;
    use crate::icons::Icons;
    use crate::scaffold::Scaffold;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::tester;

    fn destination(
        icon: IconData,
        selected_icon: IconData,
        label: &str,
    ) -> NavigationRailDestination {
        NavigationRailDestination::new(Icon::new(Some(icon)), Text::new(label))
            .selected_icon(Icon::new(Some(selected_icon)))
    }

    /// Dart's `_destinations()`.
    fn destinations() -> Vec<NavigationRailDestination> {
        vec![
            destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
            destination(Icons::bookmark_border(), Icons::bookmark(), "Def"),
            destination(Icons::star_border(), Icons::star(), "Ghi"),
            destination(Icons::hotel(), Icons::home(), "Jkl"),
        ]
    }

    /// The arguments of Dart's `_pumpNavigationRail` besides the rail.
    struct Pump {
        text_scale_factor: f64,
        use_material3: bool,
        navigation_rail_theme: Option<NavigationRailThemeData>,
    }

    impl Default for Pump {
        fn default() -> Pump {
            Pump {
                text_scale_factor: 1.0,
                use_material3: true,
                navigation_rail_theme: None,
            }
        }
    }

    fn theme(app: &App, use_material3: bool) -> ThemeData {
        ThemeData::builder().use_material3(use_material3).build(app)
    }

    /// Dart's `_pumpNavigationRail`.
    fn pump_navigation_rail_with(
        tester: &mut WidgetTester,
        navigation_rail: NavigationRail,
        pump: Pump,
    ) {
        let root = {
            let app = tester.cell.borrow();
            let mut builder = ThemeData::builder().use_material3(pump.use_material3);
            if let Some(navigation_rail_theme) = pump.navigation_rail_theme {
                builder = builder.navigation_rail_theme(navigation_rail_theme);
            }
            let theme = builder.build(&app);
            let rail = navigation_rail.into_widget();
            let text_scale_factor = pump.text_scale_factor;
            material_app(
                theme,
                Builder::new(move |_app, _context| {
                    MediaQuery::with_clamped_text_scaling(
                        None,
                        text_scale_factor,
                        text_scale_factor,
                        Scaffold::new().body(Row::new().children([
                            rail.clone(),
                            Expanded::new(Text::new("body")).into_widget(),
                        ])),
                    )
                })
                .into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    fn pump_navigation_rail(tester: &mut WidgetTester, navigation_rail: NavigationRail) {
        pump_navigation_rail_with(tester, navigation_rail, Pump::default());
    }

    /// Dart's `_buildWidget`.
    fn build_widget(app: &App, child: WidgetRef, use_material3: bool, is_rtl: bool) -> WidgetRef {
        material_app(
            theme(app, use_material3),
            Directionality::new(
                if is_rtl {
                    TextDirection::Rtl
                } else {
                    TextDirection::Ltr
                },
                Scaffold::new().body(
                    Row::new().children([child, Expanded::new(Text::new("body")).into_widget()]),
                ),
            )
            .into_widget(),
        )
    }

    fn pump_built_widget<K>(
        tester: &mut WidgetTester,
        child: impl IntoWidget<K>,
        use_material3: bool,
        is_rtl: bool,
    ) {
        let child = child.into_widget();
        let root = build_widget(&tester.cell.borrow(), child, use_material3, is_rtl);
        tester.pump_widget(root);
    }

    fn rail_box(tester: &WidgetTester) -> AnyRenderBox {
        tester.render_box_of(tester.find_by_type::<NavigationRail>()[0])
    }

    fn size_of(tester: &WidgetTester, render: AnyRenderBox) -> Size {
        render.size(&tester.cell.borrow())
    }

    fn global_top_left(tester: &WidgetTester, render: AnyRenderBox) -> Offset {
        render.local_to_global(&tester.cell.borrow(), Offset::ZERO, None)
    }

    /// The `RichText` below `element`.
    fn rich_text_below(tester: &WidgetTester, element: AnyElement) -> AnyElement {
        tester.descendants_of_type::<RichText>(element)[0]
    }

    /// Dart's `_iconRenderBox`.
    fn icon_render_box(tester: &WidgetTester, icon: IconData) -> AnyRenderBox {
        let icon = tester.find_by_icon(&icon);
        tester.render_box_of(rich_text_below(tester, icon))
    }

    /// Dart's `_labelRenderBox`.
    fn label_render_box(tester: &WidgetTester, text: &str) -> AnyRenderBox {
        let text = tester.find_text(text)[0];
        tester.render_box_of(rich_text_below(tester, text))
    }

    /// Dart's `_iconStyle`.
    fn icon_style(tester: &WidgetTester, icon: IconData) -> TextStyle {
        tester.glyph_style(tester.find_by_icon(&icon))
    }

    /// Dart's `tester.renderObject<RenderParagraph>(find.text(text)).text.style!`.
    fn paragraph_style(tester: &WidgetTester, text: &str) -> TextStyle {
        let render = label_render_box(tester, text);
        let app = tester.cell.borrow();
        render
            .as_object()
            .downcast::<RenderParagraph>(&app)
            .expect("a Text renders a paragraph")
            .text(&app)
            .style()
            .cloned()
            .expect("the paragraph's text has a style")
    }

    /// Dart's `_labelOpacity`. Only valid when labelType != all.
    fn label_opacity(tester: &WidgetTester, text: &str) -> f64 {
        let text = tester.find_text(text)[0];
        let ancestors = tester.ancestors(text);
        let app = tester.cell.borrow();
        // We search for both Visibility and FadeTransition since in some
        // cases opacity is animated, in other it's not.
        if let Some(visibility) = ancestors.iter().find_map(|element| {
            waft_widgets::downcast_widget::<Visibility>(&**element.widget(&app))
        }) {
            return if visibility.visible { 1.0 } else { 0.0 };
        }
        // first because there's also a FadeTransition from the MaterialPageRoute, which is
        // up the tree
        let fade = ancestors
            .iter()
            .find_map(|element| {
                waft_widgets::downcast_widget::<FadeTransition>(&**element.widget(&app))
                    .map(|fade| fade.opacity.clone())
            })
            .expect("a FadeTransition above the label");
        fade.value(&app)
    }

    /// Dart's `_opacityAboveLabel(text)` found anything.
    fn has_opacity_above_label(tester: &WidgetTester, text: &str) -> bool {
        let text = tester.find_text(text)[0];
        let ancestors = tester.ancestors(text);
        let app = tester.cell.borrow();
        ancestors.iter().any(|element| {
            waft_widgets::downcast_widget::<waft_widgets::Opacity>(&**element.widget(&app))
                .is_some()
        })
    }

    /// Dart's `_railMaterial`: the first material is for the rail, and the rest are for the
    /// destinations.
    fn rail_material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let rail = tester.find_by_type::<NavigationRail>()[0];
        let material = tester.descendants_of_type::<Material>(rail)[0];
        tester.widget(material, read)
    }

    /// Dart's `expect(offset, equals(expected))` for a position Waft works out through
    /// valo's `f32` matrices: equal within `f32` precision.
    fn assert_offset(actual: Offset, expected: Offset) {
        let near = |a: f64, b: f64| (a - b).abs() <= 1e-4 * b.abs().max(1.0);
        assert!(
            near(actual.dx(), expected.dx()) && near(actual.dy(), expected.dy()),
            "{actual:?} (dx {}, dy {}) is not {expected:?} (dx {}, dy {})",
            actual.dx(),
            actual.dy(),
            expected.dx(),
            expected.dy()
        );
    }

    /// Asserts that the icon `render` is centred in a destination `destination_width` wide
    /// whose 32 high indicator starts at `next_destination_y`.
    fn expect_icon_at(
        tester: &WidgetTester,
        render: AnyRenderBox,
        destination_width: f64,
        destination_height: f64,
        next_destination_y: f64,
    ) {
        let size = size_of(tester, render);
        assert_offset(
            global_top_left(tester, render),
            Offset::new(
                (destination_width - size.width()) / 2.0,
                next_destination_y + (destination_height - size.height()) / 2.0,
            ),
        );
    }

    #[test]
    fn custom_selected_and_unselected_text_styles_are_honored() {
        with_main_thread_stack(|| {
            let selected_text_style = TextStyle::new()
                .font_weight(FontWeight::W300)
                .font_size(17.0);
            let unselected_text_style = TextStyle::new()
                .font_weight(FontWeight::W800)
                .font_size(11.0);
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::All)
                    .selected_label_text_style(selected_text_style.clone())
                    .unselected_label_text_style(unselected_text_style.clone()),
            );

            let actual_selected_text_style = paragraph_style(&tester, "Abc");
            let actual_unselected_text_style = paragraph_style(&tester, "Def");
            assert_eq!(
                actual_selected_text_style.font_size,
                selected_text_style.font_size
            );
            assert_eq!(
                actual_selected_text_style.font_weight,
                selected_text_style.font_weight
            );
            assert_eq!(
                actual_unselected_text_style.font_size,
                actual_unselected_text_style.font_size
            );
            assert_eq!(
                actual_unselected_text_style.font_weight,
                actual_unselected_text_style.font_weight
            );
        });
    }

    #[test]
    fn custom_selected_and_unselected_icon_themes_are_honored() {
        with_main_thread_stack(|| {
            let selected_icon_theme = IconThemeData::new()
                .size(36.0)
                .color(Color::new(0x00000001));
            let unselected_icon_theme = IconThemeData::new()
                .size(18.0)
                .color(Color::new(0x00000002));
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::All)
                    .selected_icon_theme(selected_icon_theme.clone())
                    .unselected_icon_theme(unselected_icon_theme.clone()),
            );

            let actual_selected_icon_theme = icon_style(&tester, Icons::favorite());
            let actual_unselected_icon_theme = icon_style(&tester, Icons::bookmark_border());
            assert_eq!(actual_selected_icon_theme.color, selected_icon_theme.color);
            assert_eq!(
                actual_selected_icon_theme.font_size,
                selected_icon_theme.size
            );
            assert_eq!(
                actual_unselected_icon_theme.color,
                unselected_icon_theme.color
            );
            assert_eq!(
                actual_unselected_icon_theme.font_size,
                unselected_icon_theme.size
            );
        });
    }

    #[test]
    fn no_selected_destination_when_selected_index_is_null() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(&mut tester, NavigationRail::new(destinations(), None));

            let selected = tester
                .find_by_type::<Semantics>()
                .into_iter()
                .filter(|element| {
                    tester.widget(*element, |semantics: &Semantics| {
                        semantics.properties.selected.unwrap_or(false)
                    })
                })
                .count();
            assert_eq!(selected, 0);
        });
    }

    #[test]
    fn background_color_can_be_changed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::All),
            );

            // default surface color in M3 colorScheme
            assert_eq!(
                rail_material(&tester, |material| material.color),
                Some(Color::new(0xfffef7ff))
            );

            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::All)
                    .background_color(Colors::GREEN.primary()),
            );

            assert_eq!(
                rail_material(&tester, |material| material.color),
                Some(Colors::GREEN.primary())
            );
        });
    }

    #[test]
    fn elevation_can_be_changed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::All),
            );

            assert_eq!(rail_material(&tester, |material| material.elevation), 0.0);

            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::All)
                    .elevation(7.0),
            );

            assert_eq!(rail_material(&tester, |material| material.elevation), 7.0);
        });
    }

    #[test]
    fn renders_at_the_correct_default_width_label_type_none_default() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(&mut tester, NavigationRail::new(destinations(), Some(0)));

            assert_eq!(size_of(&tester, rail_box(&tester)).width(), 80.0);
        });
    }

    #[test]
    fn renders_at_the_correct_default_width_label_type_selected() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::Selected),
            );

            assert_eq!(size_of(&tester, rail_box(&tester)).width(), 80.0);
        });
    }

    #[test]
    fn renders_at_the_correct_default_width_label_type_all() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::All),
            );

            assert_eq!(size_of(&tester, rail_box(&tester)).width(), 80.0);
        });
    }

    /// The `MaterialApp(home: Scaffold(body: Row([NavigationRail(..), Expanded(Text('body'))])))`
    /// that Dart's `StatefulBuilder` cases rebuild.
    fn pump_rail_in_app(tester: &mut WidgetTester, rail: NavigationRail) {
        pump_rail_in_app_with(tester, rail, true);
    }

    /// [`pump_rail_in_app`] under `ThemeData(useMaterial3: use_material3)`.
    fn pump_rail_in_app_with(tester: &mut WidgetTester, rail: NavigationRail, use_material3: bool) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                theme(&app, use_material3),
                Scaffold::new()
                    .body(Row::new().children([
                        rail.into_widget(),
                        Expanded::new(Text::new("body")).into_widget(),
                    ]))
                    .into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    fn render_box_of_key(tester: &WidgetTester, key: &GlobalKey) -> AnyRenderBox {
        tester.render_object(key).as_box().expect("a render box")
    }

    #[test]
    fn leading_and_trailing_spacing_is_correct_with_0_2_destinations() {
        with_main_thread_stack(|| {
            // Padding at the top of the rail.
            let top_padding = 8.0;
            // Padding at after the leading widget.
            let spacer_padding = 8.0;
            // Width of a destination.
            let destination_width = 80.0;
            // Height of a destination indicator with icon.
            let destination_height = 32.0;
            // Space between destinations.
            let destination_spacing = 12.0;
            // Height of the leading and trailing widgets.
            let fab_height = 56.0;

            let leading_widget_key = GlobalKey::new();
            let trailing_widget_key = GlobalKey::new();
            let fab = |key: &GlobalKey| {
                FloatingActionButton::new(Some(Listener::new(|_| {})))
                    .key(KeyRef::from(key.clone()))
                    .into_widget()
            };
            let one = || {
                vec![destination(
                    Icons::favorite_border(),
                    Icons::favorite(),
                    "Abc",
                )]
            };
            let two = || {
                vec![
                    destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
                    destination(Icons::bookmark_border(), Icons::bookmark(), "Longer Label"),
                ]
            };
            let match_expect = |tester: &WidgetTester, render: AnyRenderBox, y: f64| {
                expect_icon_at(tester, render, destination_width, destination_height, y)
            };
            let leading_at = |tester: &WidgetTester, key: &GlobalKey| {
                let render = render_box_of_key(tester, key);
                (global_top_left(tester, render), size_of(tester, render))
            };

            let mut tester = tester();
            pump_rail_in_app(&mut tester, NavigationRail::new(Vec::new(), None));

            // empty destinations and leading widget
            pump_rail_in_app(
                &mut tester,
                NavigationRail::new(Vec::new(), None).leading(fab(&leading_widget_key)),
            );
            tester.pump_and_settle();
            assert_eq!(
                leading_at(&tester, &leading_widget_key).0,
                Offset::new(0.0, top_padding)
            );

            // one destination and leading widget
            pump_rail_in_app(
                &mut tester,
                NavigationRail::new(one(), None).leading(fab(&leading_widget_key)),
            );
            tester.pump_and_settle();
            let mut next_destination_y = top_padding;
            let (position, size) = leading_at(&tester, &leading_widget_key);
            assert_eq!(
                position,
                Offset::new((destination_width - size.width()) / 2.0, next_destination_y)
            );

            next_destination_y += fab_height + spacer_padding + destination_spacing / 2.0;
            match_expect(
                &tester,
                icon_render_box(&tester, Icons::favorite_border()),
                next_destination_y,
            );

            // two destinations and leading widget
            pump_rail_in_app(
                &mut tester,
                NavigationRail::new(two(), None).leading(fab(&leading_widget_key)),
            );
            tester.pump_and_settle();
            next_destination_y = top_padding;
            let (position, size) = leading_at(&tester, &leading_widget_key);
            assert_eq!(
                position,
                Offset::new((destination_width - size.width()) / 2.0, next_destination_y)
            );

            next_destination_y += fab_height + spacer_padding + destination_spacing / 2.0;
            match_expect(
                &tester,
                icon_render_box(&tester, Icons::favorite_border()),
                next_destination_y,
            );

            next_destination_y += destination_height + destination_spacing;
            match_expect(
                &tester,
                icon_render_box(&tester, Icons::bookmark_border()),
                next_destination_y,
            );

            // empty destinations and trailing widget
            pump_rail_in_app(
                &mut tester,
                NavigationRail::new(Vec::new(), None).trailing(fab(&trailing_widget_key)),
            );
            tester.pump_and_settle();
            assert_eq!(
                leading_at(&tester, &trailing_widget_key).0,
                Offset::new(0.0, top_padding)
            );

            // one destination and trailing widget
            pump_rail_in_app(
                &mut tester,
                NavigationRail::new(one(), None).trailing(fab(&trailing_widget_key)),
            );
            tester.pump_and_settle();
            next_destination_y = top_padding + destination_spacing / 2.0;
            match_expect(
                &tester,
                icon_render_box(&tester, Icons::favorite_border()),
                next_destination_y,
            );

            next_destination_y += destination_height + destination_spacing / 2.0;
            let (position, size) = leading_at(&tester, &trailing_widget_key);
            assert_eq!(
                position,
                Offset::new((destination_width - size.width()) / 2.0, next_destination_y)
            );

            // two destinations and trailing widget
            pump_rail_in_app(
                &mut tester,
                NavigationRail::new(two(), None).trailing(fab(&trailing_widget_key)),
            );
            tester.pump_and_settle();
            next_destination_y = top_padding + destination_spacing / 2.0;
            match_expect(
                &tester,
                icon_render_box(&tester, Icons::favorite_border()),
                next_destination_y,
            );

            next_destination_y += destination_height + destination_spacing;
            match_expect(
                &tester,
                icon_render_box(&tester, Icons::bookmark_border()),
                next_destination_y,
            );

            next_destination_y += destination_height + destination_spacing / 2.0;
            let (position, size) = leading_at(&tester, &trailing_widget_key);
            assert_eq!(
                position,
                Offset::new((destination_width - size.width()) / 2.0, next_destination_y)
            );
        });
    }

    fn rail_widget<R>(tester: &WidgetTester, read: impl FnOnce(&NavigationRail) -> R) -> R {
        tester.widget(tester.find_by_type::<NavigationRail>()[0], read)
    }

    #[test]
    fn change_destinations_and_selected_index() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_rail_in_app(&mut tester, NavigationRail::new(Vec::new(), None));

            assert_eq!(rail_widget(&tester, |rail| rail.destinations.len()), 0);
            assert_eq!(rail_widget(&tester, |rail| rail.selected_index), None);

            let two = || {
                vec![
                    destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
                    destination(Icons::bookmark_border(), Icons::bookmark(), "Longer Label"),
                ]
            };
            pump_rail_in_app(&mut tester, NavigationRail::new(two(), None));
            tester.pump_and_settle();

            assert_eq!(rail_widget(&tester, |rail| rail.destinations.len()), 2);
            assert_eq!(rail_widget(&tester, |rail| rail.selected_index), None);

            pump_rail_in_app(&mut tester, NavigationRail::new(two(), Some(0)));
            tester.pump_and_settle();

            assert_eq!(rail_widget(&tester, |rail| rail.destinations.len()), 2);
            assert_eq!(rail_widget(&tester, |rail| rail.selected_index), Some(0));

            pump_rail_in_app(
                &mut tester,
                NavigationRail::new(
                    vec![destination(
                        Icons::favorite_border(),
                        Icons::favorite(),
                        "Abc",
                    )],
                    Some(0),
                ),
            );
            tester.pump_and_settle();

            assert_eq!(rail_widget(&tester, |rail| rail.destinations.len()), 1);
            assert_eq!(rail_widget(&tester, |rail| rail.selected_index), Some(0));
        });
    }

    #[test]
    fn renders_wider_for_a_destination_with_a_long_label_label_type_all() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(
                    vec![
                        destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
                        destination(Icons::bookmark_border(), Icons::bookmark(), "Longer Label"),
                    ],
                    Some(0),
                )
                .label_type(NavigationRailLabelType::All),
            );

            // Total padding is 16 (8 on each side).
            assert_eq!(
                size_of(&tester, rail_box(&tester)).width(),
                size_of(&tester, label_render_box(&tester, "Longer Label")).width() + 16.0
            );
        });
    }

    fn finds_one_icon(tester: &WidgetTester, icon: IconData) -> bool {
        tester
            .find_by_type::<Icon>()
            .into_iter()
            .filter(|element| {
                tester.widget(*element, |widget: &Icon| {
                    widget.icon.as_ref() == Some(&icon)
                })
            })
            .count()
            == 1
    }

    #[test]
    fn renders_only_icons_label_type_none_default() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(&mut tester, NavigationRail::new(destinations(), Some(0)));

            assert!(finds_one_icon(&tester, Icons::favorite()));
            assert!(finds_one_icon(&tester, Icons::bookmark_border()));
            assert!(finds_one_icon(&tester, Icons::star_border()));
            assert!(finds_one_icon(&tester, Icons::hotel()));

            // When there are no labels, a 0 opacity label is still shown for semantics.
            assert_eq!(label_opacity(&tester, "Abc"), 0.0);
            assert_eq!(label_opacity(&tester, "Def"), 0.0);
            assert_eq!(label_opacity(&tester, "Ghi"), 0.0);
            assert_eq!(label_opacity(&tester, "Jkl"), 0.0);
        });
    }

    #[test]
    fn renders_icons_and_labels_label_type_all() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::All),
            );

            assert!(finds_one_icon(&tester, Icons::favorite()));
            assert!(finds_one_icon(&tester, Icons::bookmark_border()));
            assert!(finds_one_icon(&tester, Icons::star_border()));
            assert!(finds_one_icon(&tester, Icons::hotel()));

            for label in ["Abc", "Def", "Ghi", "Jkl"] {
                assert_eq!(tester.find_text(label).len(), 1);
                // When displaying all labels, there is no opacity.
                assert!(!has_opacity_above_label(&tester, label));
            }
        });
    }

    #[test]
    fn renders_icons_and_selected_label_label_type_selected() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::Selected),
            );

            assert!(finds_one_icon(&tester, Icons::favorite()));
            assert!(finds_one_icon(&tester, Icons::bookmark_border()));
            assert!(finds_one_icon(&tester, Icons::star_border()));
            assert!(finds_one_icon(&tester, Icons::hotel()));

            // Only the selected label is visible.
            assert_eq!(label_opacity(&tester, "Abc"), 1.0);
            assert_eq!(label_opacity(&tester, "Def"), 0.0);
            assert_eq!(label_opacity(&tester, "Ghi"), 0.0);
            assert_eq!(label_opacity(&tester, "Jkl"), 0.0);
        });
    }

    /// The icons of [`destinations`] in order: the first selected.
    fn icons_in_order() -> [IconData; 4] {
        [
            Icons::favorite(),
            Icons::bookmark_border(),
            Icons::star_border(),
            Icons::hotel(),
        ]
    }

    #[test]
    fn destination_spacing_is_correct_label_type_none_default_text_scale_factor_1_0_default() {
        with_main_thread_stack(|| {
            // Padding at the top of the rail.
            let top_padding = 8.0;
            // Width of a destination.
            let destination_width = 80.0;
            // Height of a destination indicator with icon.
            let destination_height = 32.0;
            // Space between destinations.
            let destination_padding = 12.0;

            let mut tester = tester();
            pump_navigation_rail(&mut tester, NavigationRail::new(destinations(), Some(0)));

            assert_eq!(
                size_of(&tester, rail_box(&tester)).width(),
                destination_width
            );

            // The first destination below the rail top by some padding.
            let mut next_destination_y = top_padding + destination_padding / 2.0;
            for icon in icons_in_order() {
                expect_icon_at(
                    &tester,
                    icon_render_box(&tester, icon),
                    destination_width,
                    destination_height,
                    next_destination_y,
                );
                // The next destination is one height below this destination.
                next_destination_y += destination_height + destination_padding;
            }
        });
    }

    /// Asserts each icon of [`icons_in_order`] centred `destination_width` wide in its 32 high
    /// indicator, the first at `first_y` and each next one `steps[i]` below the one before.
    fn expect_icon_column(
        tester: &WidgetTester,
        destination_width: f64,
        first_y: f64,
        steps: [f64; 3],
    ) {
        expect_icon_column_in(tester, destination_width, 32.0, first_y, steps);
    }

    /// [`expect_icon_column`] in destinations `destination_height` high.
    fn expect_icon_column_in(
        tester: &WidgetTester,
        destination_width: f64,
        destination_height: f64,
        first_y: f64,
        steps: [f64; 3],
    ) {
        let mut next_destination_y = first_y;
        for (i, icon) in icons_in_order().into_iter().enumerate() {
            expect_icon_at(
                tester,
                icon_render_box(tester, icon),
                destination_width,
                destination_height,
                next_destination_y,
            );
            if i < 3 {
                next_destination_y += steps[i];
            }
        }
    }

    /// Asserts the label `text` centred `destination_width` wide with its top at `y`.
    fn expect_label_at(tester: &WidgetTester, text: &str, destination_width: f64, y: f64) {
        let render = label_render_box(tester, text);
        let size = size_of(tester, render);
        assert_offset(
            global_top_left(tester, render),
            Offset::new((destination_width - size.width()) / 2.0, y),
        );
    }

    /// The icon-only destination spacing cases at `text_scale_factor`, `min_width` wide.
    fn icon_only_spacing(text_scale_factor: f64, min_width: Option<f64>) {
        with_main_thread_stack(|| {
            // Since the rail is icon only, its destinations should not be affected by
            // textScaleFactor.

            // Padding at the top of the rail.
            let top_padding = 8.0;
            // Width of a destination.
            let destination_width = min_width.unwrap_or(80.0);
            // Height of a destination indicator with icon.
            let destination_height = 32.0;
            // Space between destinations.
            let destination_padding = 12.0;

            let mut tester = tester();
            let mut rail = NavigationRail::new(destinations(), Some(0));
            if let Some(min_width) = min_width {
                rail = rail.min_width(min_width);
            }
            pump_navigation_rail_with(
                &mut tester,
                rail,
                Pump {
                    text_scale_factor,
                    ..Pump::default()
                },
            );

            assert_eq!(
                size_of(&tester, rail_box(&tester)).width(),
                destination_width
            );

            // The first destination below the rail top by some padding; each next one is
            // one height below the one before.
            let step = destination_height + destination_padding;
            expect_icon_column(
                &tester,
                destination_width,
                top_padding + destination_padding / 2.0,
                [step; 3],
            );
        });
    }

    #[test]
    fn destination_spacing_is_correct_label_type_none_default_text_scale_factor_3_0() {
        icon_only_spacing(3.0, None);
    }

    #[test]
    fn destination_spacing_is_correct_label_type_none_default_text_scale_factor_0_75() {
        icon_only_spacing(0.75, None);
    }

    #[test]
    fn destination_spacing_is_correct_for_a_compact_rail_preferred_width_56_text_scale_factor_1_0_default()
     {
        icon_only_spacing(1.0, Some(56.0));
    }

    #[test]
    fn destination_spacing_is_correct_for_a_compact_rail_preferred_width_56_text_scale_factor_3_0()
    {
        icon_only_spacing(3.0, Some(56.0));
    }

    #[test]
    fn destination_spacing_is_correct_for_a_compact_rail_preferred_width_56_text_scale_factor_0_75()
    {
        icon_only_spacing(0.75, Some(56.0));
    }

    /// The labelled destination spacing cases: `label_type` at `text_scale_factor`, whose
    /// destinations are `destination_width` wide.
    fn labelled_spacing(
        label_type: NavigationRailLabelType,
        text_scale_factor: f64,
        destination_width: f64,
    ) {
        with_main_thread_stack(|| {
            // Padding at the top of the rail.
            let top_padding = 8.0;
            // Height of a destination indicator with icon.
            let destination_height = 32.0;
            // Space between the indicator and label.
            let destination_label_spacing = 4.0;
            // Height of the label.
            let label_height = 16.0 * text_scale_factor;
            // Height of a destination with both icon and label.
            let destination_height_with_label =
                destination_height + destination_label_spacing + label_height;
            // Space between destinations.
            let destination_spacing = 12.0;

            let mut tester = tester();
            pump_navigation_rail_with(
                &mut tester,
                NavigationRail::new(destinations(), Some(0)).label_type(label_type),
                Pump {
                    text_scale_factor,
                    ..Pump::default()
                },
            );

            assert_eq!(
                size_of(&tester, rail_box(&tester)).width(),
                destination_width
            );

            // The first destination topPadding below the rail top.
            let next_destination_y = top_padding;
            expect_label_at(
                &tester,
                "Abc",
                destination_width,
                next_destination_y + destination_height + destination_label_spacing,
            );

            // Each destination is below the one before with some spacing; with
            // [labelType]=selected, only the first shows its label.
            let labelled = destination_height_with_label + destination_spacing;
            let unlabelled = match label_type {
                NavigationRailLabelType::Selected => destination_height + destination_spacing,
                _ => labelled,
            };
            expect_icon_column(
                &tester,
                destination_width,
                next_destination_y,
                [labelled, unlabelled, unlabelled],
            );
        });
    }

    #[test]
    fn destination_spacing_is_correct_label_type_selected_text_scale_factor_1_0_default() {
        labelled_spacing(NavigationRailLabelType::Selected, 1.0, 80.0);
    }

    #[test]
    fn destination_spacing_is_correct_label_type_selected_text_scale_factor_3_0() {
        labelled_spacing(NavigationRailLabelType::Selected, 3.0, 125.5);
    }

    #[test]
    fn destination_spacing_is_correct_label_type_selected_text_scale_factor_0_75() {
        labelled_spacing(NavigationRailLabelType::Selected, 0.75, 80.0);
    }

    #[test]
    fn destination_spacing_is_correct_label_type_all_text_scale_factor_1_0_default() {
        labelled_spacing(NavigationRailLabelType::All, 1.0, 80.0);
    }

    #[test]
    fn destination_spacing_is_correct_label_type_all_text_scale_factor_3_0() {
        labelled_spacing(NavigationRailLabelType::All, 3.0, 125.5);
    }

    #[test]
    fn destination_spacing_is_correct_label_type_all_text_scale_factor_0_75() {
        labelled_spacing(NavigationRailLabelType::All, 0.75, 80.0);
    }

    /// The group alignment cases: the destinations `offset` below where they start at the top.
    fn group_alignment(group_alignment: Option<f64>, offset: f64) {
        with_main_thread_stack(|| {
            // Padding at the top of the rail.
            let top_padding = 8.0;
            // Width of a destination.
            let destination_width = 80.0;
            // Height of a destination indicator with icon.
            let destination_height = 32.0;
            // Space between destinations.
            let destination_padding = 12.0;

            let mut tester = tester();
            let mut rail = NavigationRail::new(destinations(), Some(0));
            if let Some(group_alignment) = group_alignment {
                rail = rail.group_alignment(group_alignment);
            }
            pump_navigation_rail(&mut tester, rail);

            assert_eq!(
                size_of(&tester, rail_box(&tester)).width(),
                destination_width
            );

            // The first destination below the rail top by some padding with an offset for the
            // alignment.
            let step = destination_height + destination_padding;
            expect_icon_column(
                &tester,
                destination_width,
                top_padding + destination_padding / 2.0 + offset,
                [step; 3],
            );
        });
    }

    #[test]
    fn group_alignment_works_group_alignment_minus_1_0_default() {
        group_alignment(None, 0.0);
    }

    #[test]
    fn group_alignment_works_group_alignment_0_0() {
        group_alignment(Some(0.0), 208.0);
    }

    #[test]
    fn group_alignment_works_group_alignment_1_0() {
        group_alignment(Some(1.0), 416.0);
    }

    fn fab() -> WidgetRef {
        FloatingActionButton::new(Some(Listener::new(|_| {}))).into_widget()
    }

    #[test]
    fn leading_and_trailing_appear_in_the_correct_places() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .leading(fab())
                    .trailing(fab()),
            );

            let fabs = tester.find_by_type::<FloatingActionButton>();
            let leading = tester.render_box_of(fabs[0]);
            let trailing = tester.render_box_of(fabs[1]);
            assert_eq!(
                global_top_left(&tester, leading),
                Offset::new((80.0 - size_of(&tester, leading).width()) / 2.0, 8.0)
            );
            assert_eq!(
                global_top_left(&tester, trailing),
                Offset::new((80.0 - size_of(&tester, trailing).width()) / 2.0, 248.0)
            );
        });
    }

    /// What Dart's `stateSetter` cases pump: `rail` in the app, then the frame
    /// `tester.pump()` runs.
    fn set_rail(tester: &mut WidgetTester, rail: NavigationRail) {
        pump_rail_in_app(tester, rail);
    }

    #[test]
    fn extended_rail_animates_the_width_and_labels_appear_text_direction_ltr() {
        with_main_thread_stack(|| {
            // Padding at the top of the rail.
            let top_padding = 8.0;
            // Width of a destination.
            let destination_width = 80.0;
            // Height of a destination indicator with icon.
            let destination_height = 32.0;
            // Space between destinations.
            let destination_padding = 12.0;

            let mut tester = tester();
            pump_rail_in_app(&mut tester, NavigationRail::new(destinations(), Some(0)));

            let rail = rail_box(&tester);
            assert_eq!(size_of(&tester, rail).width(), destination_width);

            set_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0)).extended(true),
            );
            tester.pump(Duration::from_millis(100));
            assert_eq!(size_of(&tester, rail).width(), 168.0);

            tester.pump_and_settle();
            assert_eq!(size_of(&tester, rail).width(), 256.0);

            // The first destination below the rail top by some padding.
            let mut next_destination_y = top_padding + destination_padding / 2.0;
            for (icon, label) in icons_in_order()
                .into_iter()
                .zip(["Abc", "Def", "Ghi", "Jkl"])
            {
                expect_icon_at(
                    &tester,
                    icon_render_box(&tester, icon),
                    destination_width,
                    destination_height,
                    next_destination_y,
                );
                let label = label_render_box(&tester, label);
                assert_offset(
                    global_top_left(&tester, label),
                    Offset::new(
                        destination_width,
                        next_destination_y
                            + (destination_height - size_of(&tester, label).height()) / 2.0,
                    ),
                );
                // The next destination is one height below this destination.
                next_destination_y += destination_height + destination_padding;
            }
        });
    }

    /// The `Directionality(rtl, Scaffold(body: Row(textDirection: rtl, [rail, body])))` of the
    /// RTL extended case.
    fn pump_rtl_rail(tester: &mut WidgetTester, rail: NavigationRail) {
        pump_rtl_rail_with(tester, rail, true);
    }

    /// [`pump_rtl_rail`] under `ThemeData(useMaterial3: use_material3)`.
    fn pump_rtl_rail_with(tester: &mut WidgetTester, rail: NavigationRail, use_material3: bool) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                theme(&app, use_material3),
                Directionality::new(
                    TextDirection::Rtl,
                    Scaffold::new().body(Row::new().text_direction(TextDirection::Rtl).children([
                        rail.into_widget(),
                        Expanded::new(Text::new("body")).into_widget(),
                    ])),
                )
                .into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    #[test]
    fn extended_rail_animates_the_width_and_labels_appear_text_direction_rtl() {
        with_main_thread_stack(|| {
            // Padding at the top of the rail.
            let top_padding = 8.0;
            // Width of a destination.
            let destination_width = 80.0;
            // Height of a destination indicator with icon.
            let destination_height = 32.0;
            // Space between destinations.
            let destination_padding = 12.0;

            let mut tester = tester();
            pump_rtl_rail(&mut tester, NavigationRail::new(destinations(), Some(0)));

            let rail = rail_box(&tester);
            assert_eq!(size_of(&tester, rail).width(), destination_width);
            assert_eq!(global_top_left(&tester, rail), Offset::new(720.0, 0.0));

            pump_rtl_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0)).extended(true),
            );
            tester.pump(Duration::from_millis(100));
            assert_eq!(size_of(&tester, rail).width(), 168.0);
            assert_eq!(global_top_left(&tester, rail), Offset::new(632.0, 0.0));

            tester.pump_and_settle();
            assert_eq!(size_of(&tester, rail).width(), 256.0);
            assert_eq!(global_top_left(&tester, rail), Offset::new(544.0, 0.0));

            // The first destination below the rail top by some padding.
            let mut next_destination_y = top_padding + destination_padding / 2.0;
            for (icon, label) in icons_in_order()
                .into_iter()
                .zip(["Abc", "Def", "Ghi", "Jkl"])
            {
                let icon = icon_render_box(&tester, icon);
                let icon_size = size_of(&tester, icon);
                assert_offset(
                    global_top_left(&tester, icon),
                    Offset::new(
                        800.0 - (destination_width + icon_size.width()) / 2.0,
                        next_destination_y + (destination_height - icon_size.height()) / 2.0,
                    ),
                );
                let label = label_render_box(&tester, label);
                let label_size = size_of(&tester, label);
                assert_offset(
                    global_top_left(&tester, label),
                    Offset::new(
                        800.0 - destination_width - label_size.width(),
                        next_destination_y + (destination_height - label_size.height()) / 2.0,
                    ),
                );
                // The next destination is one height below this destination.
                next_destination_y += destination_height + destination_padding;
            }
        });
    }

    fn abc_and_longer_label() -> Vec<NavigationRailDestination> {
        vec![
            destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
            destination(Icons::bookmark_border(), Icons::bookmark(), "Longer Label"),
        ]
    }

    #[test]
    fn extended_rail_gets_wider_with_longer_labels_are_larger_text_scale() {
        with_main_thread_stack(|| {
            let pump = |tester: &mut WidgetTester, extended: bool| {
                let root = {
                    let app = tester.cell.borrow();
                    material_app(
                        ThemeData::new(&app),
                        Scaffold::new()
                            .body(
                                Row::new().children([
                                    MediaQuery::with_clamped_text_scaling(
                                        None,
                                        3.0,
                                        3.0,
                                        NavigationRail::new(abc_and_longer_label(), Some(0))
                                            .extended(extended),
                                    ),
                                    Expanded::new(Text::new("body")).into_widget(),
                                ]),
                            )
                            .into_widget(),
                    )
                };
                tester.pump_widget(root);
            };
            let mut tester = tester();
            pump(&mut tester, false);

            let rail = rail_box(&tester);
            assert_eq!(size_of(&tester, rail).width(), 80.0);

            pump(&mut tester, true);
            tester.pump(Duration::from_millis(100));
            assert_eq!(size_of(&tester, rail).width(), 303.0);

            tester.pump_and_settle();
            assert_eq!(size_of(&tester, rail).width(), 526.0);
        });
    }

    #[test]
    fn extended_rail_final_width_can_be_changed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_rail_in_app(
                &mut tester,
                NavigationRail::new(destinations(), Some(0)).min_extended_width(300.0),
            );

            let rail = rail_box(&tester);
            assert_eq!(size_of(&tester, rail).width(), 80.0);

            set_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .min_extended_width(300.0)
                    .extended(true),
            );
            tester.pump_and_settle();
            assert_eq!(size_of(&tester, rail).width(), 300.0);
        });
    }

    /// Regression test for https://github.com/flutter/flutter/issues/65657
    #[test]
    fn extended_rail_transition_does_not_jump_from_the_beginning() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_rail_in_app(
                &mut tester,
                NavigationRail::new(abc_and_longer_label(), Some(0)),
            );

            // Before starting the animation, the rail has a width of 80.
            assert_eq!(size_of(&tester, rail_box(&tester)).width(), 80.0);

            set_rail(
                &mut tester,
                NavigationRail::new(abc_and_longer_label(), Some(0)).extended(true),
            );
            // Create very close to 0, but non-zero, animation value.
            tester.pump(Duration::from_millis(1));
            let width = size_of(&tester, rail_box(&tester)).width();
            // Expect that it has started to extend.
            assert!(width > 80.0, "{width}");
            // Expect that it has only extended by a small amount, or that the first
            // frame does not jump. This helps verify that it is a smooth animation.
            assert!((width - 80.0).abs() <= 1.0, "{width}");
        });
    }

    #[test]
    fn extended_rail_animation_can_be_consumed() {
        with_main_thread_stack(|| {
            let animation: Rc<std::cell::Cell<Option<AnyAnimation<f64>>>> = Rc::default();
            let rail = |extended: bool| {
                let animation = Rc::clone(&animation);
                NavigationRail::new(destinations(), Some(0))
                    .leading(Builder::new(move |app, context| {
                        animation.set(Some(NavigationRail::extended_animation(app, context)));
                        fab()
                    }))
                    .extended(extended)
            };
            let mut tester = tester();
            pump_rail_in_app(&mut tester, rail(false));

            let read = |tester: &WidgetTester| {
                animation
                    .get()
                    .expect("the leading widget was built")
                    .status(&tester.cell.borrow())
            };
            assert!(read(&tester).is_dismissed());

            set_rail(&mut tester, rail(true));
            tester.pump_and_settle();

            assert!(read(&tester).is_completed());
        });
    }

    fn center_of_element(tester: &WidgetTester, element: AnyElement) -> Offset {
        tester.global_rect(tester.render_box_of(element)).center()
    }

    fn center_of_text(tester: &WidgetTester, text: &str) -> Offset {
        center_of_element(tester, tester.find_text(text)[0])
    }

    fn center_of_icon(tester: &WidgetTester, icon: IconData) -> Offset {
        center_of_element(tester, tester.find_by_icon(&icon))
    }

    #[test]
    fn on_destination_selected_is_called() {
        with_main_thread_stack(|| {
            let selected_index = Rc::new(std::cell::Cell::new(None));
            let mut tester = tester();
            let seen = Rc::clone(&selected_index);
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .on_destination_selected(move |_app, index| seen.set(Some(index)))
                    .label_type(NavigationRailLabelType::All),
            );

            tester.tap(center_of_text(&tester, "Def"), 1);
            assert_eq!(selected_index.get(), Some(1));

            tester.tap(center_of_text(&tester, "Ghi"), 2);
            assert_eq!(selected_index.get(), Some(2));

            // Wait for any pending shader compilation.
            tester.pump_and_settle();
        });
    }

    #[test]
    fn on_destination_selected_is_not_called_if_null() {
        with_main_thread_stack(|| {
            let selected_index = 0;
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(selected_index))
                    .label_type(NavigationRailLabelType::All),
            );

            tester.tap(center_of_text(&tester, "Def"), 1);
            assert_eq!(selected_index, 0);

            // Wait for any pending shader compilation.
            tester.pump_and_settle();
        });
    }

    #[test]
    fn changing_destinations_animate_when_label_type_selected() {
        with_main_thread_stack(|| {
            let selected_index = Rc::new(std::cell::Cell::new(0_usize));
            let mut tester = tester();
            let root = {
                let app = tester.cell.borrow();
                let selected_index = Rc::clone(&selected_index);
                material_app(
                    ThemeData::new(&app),
                    waft_widgets::StatefulBuilder::new(move |_app, _context, set_state| {
                        let selected = Rc::clone(&selected_index);
                        Scaffold::new()
                            .body(
                                Row::new().children([
                                    NavigationRail::new(destinations(), Some(selected_index.get()))
                                        .label_type(NavigationRailLabelType::Selected)
                                        .on_destination_selected(move |app, index| {
                                            let selected = Rc::clone(&selected);
                                            set_state(app, &mut |_app| selected.set(index));
                                        })
                                        .into_widget(),
                                    Expanded::new(Text::new("body")).into_widget(),
                                ]),
                            )
                            .into_widget()
                    })
                    .into_widget(),
                )
            };
            tester.pump_widget(root);

            // Tap the second destination.
            tester.tap(center_of_icon(&tester, Icons::bookmark_border()), 1);
            assert_eq!(selected_index.get(), 1);

            // The second destination animates in.
            assert_eq!(label_opacity(&tester, "Def"), 0.0);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(100));
            assert_eq!(label_opacity(&tester, "Def"), 0.5);
            tester.pump_and_settle();
            assert_eq!(label_opacity(&tester, "Def"), 1.0);

            // Tap the third destination.
            tester.tap(center_of_icon(&tester, Icons::star_border()), 2);
            assert_eq!(selected_index.get(), 2);

            // The second destination animates out quickly and the third destination
            // animates in.
            assert_eq!(label_opacity(&tester, "Ghi"), 0.0);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(25));
            assert_eq!(label_opacity(&tester, "Def"), 0.5);
            assert_eq!(label_opacity(&tester, "Ghi"), 0.0);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(25));
            assert_eq!(label_opacity(&tester, "Def"), 0.0);
            assert_eq!(label_opacity(&tester, "Ghi"), 0.0);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(50));
            assert_eq!(label_opacity(&tester, "Ghi"), 0.5);
            tester.pump_and_settle();
            assert_eq!(label_opacity(&tester, "Ghi"), 1.0);
        });
    }

    fn selected_rail(selected_index: Option<usize>) -> NavigationRail {
        NavigationRail::new(destinations(), selected_index)
            .label_type(NavigationRailLabelType::Selected)
    }

    #[test]
    fn changing_destinations_animate_for_selected_index_null() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_rail_in_app(&mut tester, selected_rail(Some(0)));

            // Unset the selected index.
            set_rail(&mut tester, selected_rail(None));

            // The first destination animates out.
            assert_eq!(label_opacity(&tester, "Abc"), 1.0);
            tester.pump(Duration::from_millis(25));
            assert_eq!(label_opacity(&tester, "Abc"), 0.5);
            tester.pump_and_settle();
            assert_eq!(label_opacity(&tester, "Abc"), 0.0);

            // Set the selected index to the first destination.
            set_rail(&mut tester, selected_rail(Some(0)));

            // The first destination animates in.
            assert_eq!(label_opacity(&tester, "Abc"), 0.0);
            tester.pump(Duration::from_millis(100));
            assert_eq!(label_opacity(&tester, "Abc"), 0.5);
            tester.pump_and_settle();
            assert_eq!(label_opacity(&tester, "Abc"), 1.0);
        });
    }

    /// Dart's `StatefulBuilder` over a rail of the `selected_index` it keeps, with the
    /// `stateSetter` the case calls without pumping a frame.
    struct StatefulRail {
        selected_index: Rc<std::cell::Cell<Option<usize>>>,
        set_state: Rc<std::cell::RefCell<Option<waft_widgets::StateSetter>>>,
    }

    impl StatefulRail {
        fn pump(
            tester: &mut WidgetTester,
            selected_index: Option<usize>,
            make: fn(Option<usize>) -> NavigationRail,
        ) -> StatefulRail {
            let rail = StatefulRail {
                selected_index: Rc::new(std::cell::Cell::new(selected_index)),
                set_state: Rc::default(),
            };
            let root = {
                let app = tester.cell.borrow();
                let selected_index = Rc::clone(&rail.selected_index);
                let state_setter = Rc::clone(&rail.set_state);
                material_app(
                    ThemeData::new(&app),
                    waft_widgets::StatefulBuilder::new(move |_app, _context, set_state| {
                        *state_setter.borrow_mut() = Some(set_state);
                        Scaffold::new()
                            .body(Row::new().children([
                                make(selected_index.get()).into_widget(),
                                Expanded::new(Text::new("body")).into_widget(),
                            ]))
                            .into_widget()
                    })
                    .into_widget(),
                )
            };
            tester.pump_widget(root);
            rail
        }

        /// Dart's `stateSetter(() { selectedIndex = selected_index; })`.
        fn set_selected_index(&self, tester: &WidgetTester, selected_index: Option<usize>) {
            let set_state = self
                .set_state
                .borrow()
                .clone()
                .expect("the builder was built");
            let selected = Rc::clone(&self.selected_index);
            set_state(&mut tester.cell.borrow_mut(), &mut |_app| {
                selected.set(selected_index)
            });
        }
    }

    #[test]
    fn changing_destinations_animate_when_selected_index_null_during_transition() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let rail = StatefulRail::pump(&mut tester, Some(0), selected_rail);

            rail.set_selected_index(&tester, Some(1));

            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(175));

            // Interrupt while animating from index 0 to 1.
            rail.set_selected_index(&tester, None);

            assert_eq!(label_opacity(&tester, "Abc"), 0.0);
            assert_eq!(label_opacity(&tester, "Def"), 1.0);

            tester.pump(Duration::ZERO);
            // Create very close to 0, but non-zero, animation value.
            tester.pump(Duration::from_millis(1));
            // Ensure the opacity is animated back towards 0.
            let def = label_opacity(&tester, "Def");
            assert!(def < 0.5, "{def}");
            assert!((def - 0.5).abs() <= 0.03, "{def}");

            tester.pump_and_settle();
            assert_eq!(label_opacity(&tester, "Abc"), 0.0);
            assert_eq!(label_opacity(&tester, "Def"), 0.0);
        });
    }

    fn abc_def_ghi_with_padding() -> Vec<NavigationRailDestination> {
        vec![
            destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
            destination(Icons::bookmark_border(), Icons::bookmark(), "Def")
                .padding(EdgeInsetsGeometry::symmetric(30.0, 0.0)),
            destination(Icons::star_border(), Icons::star(), "Ghi")
                .padding(EdgeInsetsGeometry::symmetric(0.0, 10.0)),
        ]
    }

    /// Dart's `tester.widget<Padding>(find.descendant(of: find.widgetWithText(
    /// _IndicatorInkWell, text), matching: find.widgetWithText(Padding, text))).padding`: the
    /// one `Padding` between the destination's ink well and its label.
    fn destination_padding(tester: &WidgetTester, text: &str) -> EdgeInsetsGeometry {
        let text = tester.find_text(text)[0];
        let ancestors = tester.ancestors(text);
        let app = tester.cell.borrow();
        let below_ink_well: Vec<&AnyElement> = ancestors
            .iter()
            .take_while(|element| {
                waft_widgets::downcast_widget::<InkResponse>(&**element.widget(&app)).is_none()
            })
            .collect();
        assert!(
            below_ink_well.len() < ancestors.len(),
            "the label is inside an ink well"
        );
        let paddings: Vec<EdgeInsetsGeometry> = below_ink_well
            .into_iter()
            .filter_map(|element| {
                waft_widgets::downcast_widget::<Padding>(&**element.widget(&app))
                    .map(|padding| padding.padding)
            })
            .collect();
        assert_eq!(
            paddings.len(),
            1,
            "one Padding between the ink well and the label"
        );
        paddings[0]
    }

    fn padding_properly_applied(
        label_type: NavigationRailLabelType,
        default_padding: EdgeInsetsGeometry,
    ) {
        with_main_thread_stack(|| {
            let second_item_padding = EdgeInsetsGeometry::symmetric(30.0, 0.0);
            let third_item_padding = EdgeInsetsGeometry::symmetric(0.0, 10.0);

            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(abc_def_ghi_with_padding(), Some(0)).label_type(label_type),
            );

            assert_eq!(destination_padding(&tester, "Abc"), default_padding);
            assert_eq!(destination_padding(&tester, "Def"), second_item_padding);
            assert_eq!(destination_padding(&tester, "Ghi"), third_item_padding);
        });
    }

    #[test]
    fn navigation_rail_destination_padding_properly_applied_navigation_rail_label_type_all() {
        padding_properly_applied(
            NavigationRailLabelType::All,
            EdgeInsetsGeometry::symmetric(0.0, 8.0),
        );
    }

    #[test]
    fn navigation_rail_destination_padding_properly_applied_navigation_rail_label_type_selected() {
        padding_properly_applied(
            NavigationRailLabelType::Selected,
            EdgeInsetsGeometry::symmetric(0.0, 8.0),
        );
    }

    #[test]
    fn navigation_rail_destination_padding_properly_applied_navigation_rail_label_type_none() {
        padding_properly_applied(NavigationRailLabelType::None, EdgeInsetsGeometry::ZERO);
    }

    fn abc_def_ghi() -> Vec<NavigationRailDestination> {
        vec![
            destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
            destination(Icons::bookmark_border(), Icons::bookmark(), "Def"),
            destination(Icons::star_border(), Icons::star(), "Ghi"),
        ]
    }

    #[test]
    fn navigation_rail_destination_adds_indicator_by_default_when_theme_data_use_material3_is_true()
    {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(abc_def_ghi(), Some(0))
                    .label_type(NavigationRailLabelType::Selected),
            );

            assert!(!tester.find_by_type::<NavigationIndicator>().is_empty());
        });
    }

    #[test]
    fn navigation_rail_destination_adds_indicator_when_use_indicator_is_true() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(abc_def_ghi(), Some(0))
                    .use_indicator(true)
                    .label_type(NavigationRailLabelType::Selected),
            );

            assert!(!tester.find_by_type::<NavigationIndicator>().is_empty());
        });
    }

    #[test]
    fn navigation_rail_destination_does_not_add_indicator_when_use_indicator_is_false() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(abc_def_ghi(), Some(0))
                    .use_indicator(false)
                    .label_type(NavigationRailLabelType::Selected),
            );

            assert!(tester.find_by_type::<NavigationIndicator>().is_empty());
        });
    }

    fn adds_an_oval_indicator(label_type: NavigationRailLabelType) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(abc_def_ghi(), Some(0))
                    .use_indicator(true)
                    .label_type(label_type),
            );

            let indicator = tester.find_by_type::<NavigationIndicator>()[0];
            let (width, height) = tester.widget(indicator, |indicator: &NavigationIndicator| {
                (indicator.width, indicator.height)
            });
            assert_eq!(width, 56.0);
            assert_eq!(height, 32.0);
        });
    }

    #[test]
    fn navigation_rail_destination_adds_an_oval_indicator_when_no_labels_are_present() {
        adds_an_oval_indicator(NavigationRailLabelType::None);
    }

    #[test]
    fn navigation_rail_destination_adds_an_oval_indicator_when_selected_labels_are_present() {
        adds_an_oval_indicator(NavigationRailLabelType::Selected);
    }

    #[test]
    fn navigation_rail_destination_adds_an_oval_indicator_when_all_labels_are_present() {
        adds_an_oval_indicator(NavigationRailLabelType::All);
    }

    /// The destinations of "has center aligned indicator": the first icon in a `Stack` with a
    /// small text over its corner.
    fn umbrella_destinations() -> Vec<NavigationRailDestination> {
        vec![
            NavigationRailDestination::new(
                Stack::new().children([
                    Icon::new(Some(Icons::umbrella())).into_widget(),
                    waft_widgets::Positioned::new(
                        Text::new("Text").style(
                            TextStyle::new()
                                .font_size(10.0)
                                .color(Colors::RED.primary()),
                        ),
                    )
                    .top(0.0)
                    .right(0.0)
                    .into_widget(),
                ]),
                Text::new("Abc"),
            ),
            NavigationRailDestination::new(Icon::new(Some(Icons::umbrella())), Text::new("Def")),
            NavigationRailDestination::new(
                Icon::new(Some(Icons::bookmark_border())),
                Text::new("Ghi"),
            ),
        ]
    }

    fn has_center_aligned_indicator(use_material3: bool, dx: f64) {
        with_main_thread_stack(|| {
            // This is a regression test for
            // https://github.com/flutter/flutter/issues/97753
            let mut tester = tester();
            pump_navigation_rail_with(
                &mut tester,
                NavigationRail::new(umbrella_destinations(), Some(0))
                    .label_type(NavigationRailLabelType::None),
                Pump {
                    use_material3,
                    ..Pump::default()
                },
            );
            let icons = tester.find_by_type::<Icon>();
            // Indicator with Stack widget
            let first_indicator = tester.render_box_of(icons[0]);
            assert_eq!(global_top_left(&tester, first_indicator).dx(), dx);
            // Indicator without Stack widget
            let last_indicator = tester.render_box_of(*icons.last().expect("icons"));
            assert_eq!(global_top_left(&tester, last_indicator).dx(), dx);
        });
    }

    #[test]
    fn navigation_rail_destination_has_center_aligned_indicator_label_type_none() {
        has_center_aligned_indicator(true, 28.0);
    }

    fn two_destinations() -> Vec<NavigationRailDestination> {
        vec![
            destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
            destination(Icons::bookmark_border(), Icons::bookmark(), "Def"),
        ]
    }

    fn respects_the_notch(use_material3: bool, default_rail_width: f64) {
        with_main_thread_stack(|| {
            let safe_area_padding = 40.0;
            let with_padding = |padding: EdgeInsets| {
                MediaQuery::new(
                    waft_widgets::MediaQueryData::new().padding(padding),
                    NavigationRail::new(two_destinations(), Some(0)),
                )
                .into_widget()
            };

            let mut tester = tester();
            pump_built_widget(
                &mut tester,
                NavigationRail::new(two_destinations(), Some(0)),
                use_material3,
                false,
            );
            let default_width = size_of(&tester, rail_box(&tester)).width();
            assert_eq!(default_width, default_rail_width);

            pump_built_widget(
                &mut tester,
                with_padding(EdgeInsets::only(safe_area_padding, 0.0, 0.0, 0.0)),
                use_material3,
                false,
            );
            let updated_width = size_of(&tester, rail_box(&tester)).width();
            assert_eq!(updated_width, default_width + safe_area_padding);

            // test width when text direction is RTL.
            pump_built_widget(
                &mut tester,
                with_padding(EdgeInsets::only(0.0, 0.0, safe_area_padding, 0.0)),
                use_material3,
                true,
            );
            let updated_width_rtl = size_of(&tester, rail_box(&tester)).width();
            assert_eq!(updated_width_rtl, default_width + safe_area_padding);
        });
    }

    #[test]
    fn navigation_rail_respects_the_notch_system_navigation_bar_in_landscape_mode() {
        respects_the_notch(true, 80.0);
    }

    /// What the ink features of the rail's materials draw, in tree order: Dart's `paints` on
    /// the first `_RenderInkFeatures`, the scaffold's, whose paint reaches theirs.
    fn rail_ink_ops(tester: &WidgetTester) -> Vec<waft_embedder::valo::Op> {
        let rail = rail_box(tester).as_object();
        let features = tester.all::<crate::material::RenderInkFeatures>();
        let mut ops = Vec::new();
        for render in features {
            let under_rail = {
                let app = tester.cell.borrow();
                std::iter::successors(
                    Some(waft_rendering::RenderBox::as_object(render)),
                    |object| object.parent(&app),
                )
                .any(|object| object == rail)
            };
            if under_rail {
                let mut canvas = waft_embedder::Canvas::new();
                render.paint_ink_features(&mut tester.cell.borrow_mut(), &mut canvas, Offset::ZERO);
                ops.extend(canvas.build().ops().iter().cloned());
            }
        }
        ops
    }

    /// Dart's `paints..clipPath(pathMatcher: isPathThat(includes: [the edge midpoints of
    /// indicator_rect], excludes: [those of indicator_rect.inflate(10)]))..rect(rect:
    /// indicator_rect, color: Color(0x0a6750a4))..rrect(rrect: rrect, color:
    /// Color(0xffe8def8))`: the hover highlight clipped to the indicator, then the selected
    /// destination's indicator.
    fn paints_indicator_highlight(
        ops: &[waft_embedder::valo::Op],
        indicator_rect: Rect,
        rrect: waft_embedder::RRect,
    ) -> bool {
        use waft_embedder::valo::{self, Op};
        let midpoints = |rect: Rect| {
            [
                rect.center_left(),
                rect.top_center(),
                rect.center_right(),
                rect.bottom_center(),
            ]
        };
        let contains = |path: &valo::Path, point: Offset| {
            path.contains(
                valo::Point::new(point.dx() as f32, point.dy() as f32),
                valo::FillRule::NonZero,
            )
        };
        let excluded_rect = indicator_rect.inflate(10.0);
        let Some(clip_at) = ops.iter().position(|op| match op {
            Op::ClipPath { path, .. } => {
                midpoints(indicator_rect)
                    .iter()
                    .all(|point| contains(path, *point))
                    && midpoints(excluded_rect)
                        .iter()
                        .all(|point| !contains(path, *point))
            }
            _ => false,
        }) else {
            return false;
        };
        let near = |a: f32, b: f64| (f64::from(a) - b).abs() < 1e-3;
        let highlight_color: valo::Color = Color::new(0x0a6750a4).into();
        let Some(rect_at) = ops[clip_at..].iter().position(|op| match op {
            Op::DrawRect { rect, paint, .. } => {
                paint.color == highlight_color
                    && near(rect.x, indicator_rect.left)
                    && near(rect.y, indicator_rect.top)
                    && near(rect.width, indicator_rect.width())
                    && near(rect.height, indicator_rect.height())
            }
            _ => false,
        }) else {
            return false;
        };
        let mut canvas = waft_embedder::Canvas::new();
        waft_painting::draw_rrect(&mut canvas, rrect, &waft_embedder::Paint::default());
        let expected = canvas.build();
        let Some(Op::DrawPath {
            path: expected_path,
            ..
        }) = expected.ops().first()
        else {
            panic!("draw_rrect records a path");
        };
        let indicator_color: valo::Color = Color::new(0xffe8def8).into();
        ops[clip_at + rect_at..].iter().any(|op| {
            matches!(
                op,
                Op::DrawPath { path, paint, .. }
                    if path == expected_path && paint.color == indicator_color
            )
        })
    }

    /// Dart's `createGesture(kind: mouse)`, `addPointer()`, `moveTo(center of icon)` and
    /// `pumpAndSettle()`.
    fn hover_icon(tester: &mut WidgetTester, icon: IconData) {
        let at = center_of_icon(tester, icon);
        let mut mouse = WidgetTester::mouse(1, 0);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        tester.move_to(&mut mouse, at, false);
        tester.pump_and_settle();
    }

    fn rrect(left: f64, top: f64, right: f64, bottom: f64) -> waft_embedder::RRect {
        waft_embedder::RRect::from_ltrbr(left, top, right, bottom, Radius::circular(16.0))
    }

    #[test]
    fn navigation_rail_indicator_renders_ripple() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(two_destinations(), Some(1))
                    .label_type(NavigationRailLabelType::All),
            );
            hover_icon(&mut tester, Icons::favorite_border());

            let indicator_rect = Rect::from_ltrb(12.0, 0.0, 68.0, 32.0);
            assert!(paints_indicator_highlight(
                &rail_ink_ops(&tester),
                indicator_rect,
                rrect(12.0, 0.0, 68.0, 32.0)
            ));
        });
    }

    #[test]
    fn navigation_rail_indicator_renders_ripple_extended() {
        with_main_thread_stack(|| {
            // This is a regression test for https://github.com/flutter/flutter/issues/117126
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(two_destinations(), Some(1))
                    .extended(true)
                    .label_type(NavigationRailLabelType::None),
            );
            hover_icon(&mut tester, Icons::favorite_border());

            let indicator_rect = Rect::from_ltrb(12.0, 6.0, 68.0, 38.0);
            assert!(paints_indicator_highlight(
                &rail_ink_ops(&tester),
                indicator_rect,
                rrect(12.0, 6.0, 68.0, 38.0)
            ));
        });
    }

    fn padded_two_destinations() -> Vec<NavigationRailDestination> {
        vec![
            destination(Icons::favorite_border(), Icons::favorite(), "Abc")
                .padding(EdgeInsetsGeometry::all(10.0)),
            destination(Icons::bookmark_border(), Icons::bookmark(), "Def")
                .padding(EdgeInsetsGeometry::all(18.0)),
        ]
    }

    #[test]
    fn navigation_rail_indicator_renders_properly_when_padding_is_applied() {
        with_main_thread_stack(|| {
            // This is a regression test for https://github.com/flutter/flutter/issues/117126
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(padded_two_destinations(), Some(1))
                    .extended(true)
                    .label_type(NavigationRailLabelType::None),
            );
            hover_icon(&mut tester, Icons::favorite_border());

            let indicator_rect = Rect::from_ltrb(22.0, 16.0, 78.0, 48.0);
            assert!(paints_indicator_highlight(
                &rail_ink_ops(&tester),
                indicator_rect,
                rrect(30.0, 24.0, 86.0, 56.0)
            ));
        });
    }

    #[test]
    fn indicator_renders_properly_when_navigation_rai_min_width_less_than_default_min_width() {
        with_main_thread_stack(|| {
            // This is a regression test for https://github.com/flutter/flutter/issues/117126
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(two_destinations(), Some(1))
                    .min_width(50.0)
                    .extended(true)
                    .label_type(NavigationRailLabelType::None),
            );
            hover_icon(&mut tester, Icons::favorite_border());

            let indicator_rect = Rect::from_ltrb(-3.0, 6.0, 53.0, 38.0);
            assert!(paints_indicator_highlight(
                &rail_ink_ops(&tester),
                indicator_rect,
                rrect(0.0, 6.0, 50.0, 38.0)
            ));
        });
    }

    #[test]
    fn navigation_rail_indicator_renders_properly_with_custom_padding_and_min_width() {
        with_main_thread_stack(|| {
            // This is a regression test for https://github.com/flutter/flutter/issues/117126
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(padded_two_destinations(), Some(1))
                    .min_width(300.0)
                    .extended(true)
                    .label_type(NavigationRailLabelType::None),
            );
            hover_icon(&mut tester, Icons::favorite_border());

            let indicator_rect = Rect::from_ltrb(132.0, 16.0, 188.0, 48.0);
            assert!(paints_indicator_highlight(
                &rail_ink_ops(&tester),
                indicator_rect,
                rrect(140.0, 24.0, 196.0, 56.0)
            ));
        });
    }

    #[test]
    fn navigation_rail_indicator_renders_properly_with_long_labels() {
        with_main_thread_stack(|| {
            // This is a regression test for https://github.com/flutter/flutter/issues/128005.
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(
                    vec![
                        destination(
                            Icons::favorite_border(),
                            Icons::favorite(),
                            "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                        ),
                        destination(Icons::bookmark_border(), Icons::bookmark(), "ABC"),
                    ],
                    Some(1),
                )
                .label_type(NavigationRailLabelType::All),
            );
            hover_icon(&mut tester, Icons::favorite_border());

            // Default values from M3 specification.
            let rail_min_width = 80.0;
            let indicator_height = 32.0;
            let destination_width = 72.0;
            let destination_horizontal_padding = 8.0;
            let indicator_width = destination_width - 2.0 * destination_horizontal_padding; // 56.0

            // The navigation rail width is larger than default because of the first
            // destination long label.
            let rail_width = size_of(&tester, rail_box(&tester)).width();

            // Expected indicator position.
            let indicator_left = (rail_width - indicator_width) / 2.0;
            let indicator_right = (rail_width + indicator_width) / 2.0;
            let indicator_rect =
                Rect::from_ltrb(indicator_left, 0.0, indicator_right, indicator_height);
            let indicator_horizontal_padding = (rail_min_width - indicator_width) / 2.0; // 12.0

            assert!(paints_indicator_highlight(
                &rail_ink_ops(&tester),
                indicator_rect,
                rrect(
                    indicator_horizontal_padding,
                    0.0,
                    indicator_horizontal_padding + indicator_width,
                    indicator_height,
                )
            ));
        });
    }

    #[test]
    fn navigation_rail_indicator_renders_properly_with_large_icon() {
        with_main_thread_stack(|| {
            // This is a regression test for https://github.com/flutter/flutter/issues/133799.
            let icon_size = 50.0;
            let mut tester = tester();
            pump_navigation_rail_with(
                &mut tester,
                NavigationRail::new(
                    vec![
                        destination(Icons::favorite_border(), Icons::favorite(), "ABC"),
                        destination(Icons::bookmark_border(), Icons::bookmark(), "DEF"),
                    ],
                    Some(1),
                )
                .label_type(NavigationRailLabelType::All),
                Pump {
                    navigation_rail_theme: Some(
                        NavigationRailThemeData::new()
                            .selected_icon_theme(IconThemeData::new().size(icon_size))
                            .unselected_icon_theme(IconThemeData::new().size(icon_size)),
                    ),
                    ..Pump::default()
                },
            );

            // Hover the first destination.
            hover_icon(&mut tester, Icons::favorite_border());

            // Default values from M3 specification.
            let rail_min_width = 80.0;
            let indicator_height = 32.0;
            let destination_width = 72.0;
            let destination_horizontal_padding = 8.0;
            let indicator_width = destination_width - 2.0 * destination_horizontal_padding; // 56.0

            // The navigation rail width is the default one because labels are short.
            let rail_width = size_of(&tester, rail_box(&tester)).width();
            assert_eq!(rail_width, rail_min_width);

            // Expected indicator position.
            let indicator_left = (rail_width - indicator_width) / 2.0;
            let indicator_right = (rail_width + indicator_width) / 2.0;
            let indicator_top = (icon_size - indicator_height) / 2.0;
            let indicator_bottom = (icon_size + indicator_height) / 2.0;
            let indicator_rect = Rect::from_ltrb(
                indicator_left,
                indicator_top,
                indicator_right,
                indicator_bottom,
            );

            // Icon height is greater than indicator height so the indicator has a vertical
            // offset.
            let second_indicator_vertical_offset = (icon_size - indicator_height) / 2.0;

            assert!(paints_indicator_highlight(
                &rail_ink_ops(&tester),
                // Hover highlight for the hovered destination (the one with 'favorite' icon).
                indicator_rect,
                // Indicator for the selected destination (the one with 'bookmark' icon).
                rrect(
                    indicator_left,
                    second_indicator_vertical_offset,
                    indicator_right,
                    second_indicator_vertical_offset + indicator_height,
                )
            ));
        });
    }

    #[test]
    fn navigation_rail_indicator_renders_properly_when_text_direction_is_rtl() {
        with_main_thread_stack(|| {
            // This is a regression test for https://github.com/flutter/flutter/issues/134361.
            let mut tester = tester();
            pump_built_widget(
                &mut tester,
                NavigationRail::new(
                    vec![
                        destination(Icons::favorite_border(), Icons::favorite(), "ABC"),
                        destination(Icons::bookmark_border(), Icons::bookmark(), "DEF"),
                    ],
                    Some(1),
                )
                .extended(true),
                true,
                true,
            );

            // Hover the first destination.
            hover_icon(&mut tester, Icons::favorite_border());

            // Default values from M3 specification.
            let rail_min_extended_width = 256.0;
            let indicator_height = 32.0;
            let destination_width = 72.0;
            let destination_horizontal_padding = 8.0;
            let indicator_width = destination_width - 2.0 * destination_horizontal_padding; // 56.0
            let vertical_destination_spacing_m3 = 12.0;

            // The navigation rail width is the default one because labels are short.
            let rail_width = size_of(&tester, rail_box(&tester)).width();
            assert_eq!(rail_width, rail_min_extended_width);

            // Expected indicator position.
            let indicator_left =
                rail_width - (destination_width - destination_horizontal_padding / 2.0);
            let indicator_right = indicator_left + indicator_width;
            let indicator_rect = Rect::from_ltrb(
                indicator_left,
                vertical_destination_spacing_m3 / 2.0,
                indicator_right,
                vertical_destination_spacing_m3 / 2.0 + indicator_height,
            );

            // Compute the vertical position for the selected destination (the one with
            // 'bookmark' icon).
            let second_indicator_vertical_offset = vertical_destination_spacing_m3 / 2.0;

            assert!(paints_indicator_highlight(
                &rail_ink_ops(&tester),
                // Hover highlight for the hovered destination (the one with 'favorite' icon).
                indicator_rect,
                // Indicator for the selected destination (the one with 'bookmark' icon).
                rrect(
                    indicator_left,
                    second_indicator_vertical_offset,
                    indicator_right,
                    second_indicator_vertical_offset + indicator_height,
                )
            ));
        });
    }

    #[test]
    fn navigation_rail_indicator_scale_transform() {
        with_main_thread_stack(|| {
            let build = |tester: &mut WidgetTester, selected_index: usize| {
                pump_navigation_rail(
                    tester,
                    NavigationRail::new(two_destinations(), Some(selected_index))
                        .label_type(NavigationRailLabelType::All),
                );
            };
            let mut tester = tester();
            build(&mut tester, 0);
            tester.pump_and_settle();
            // Dart's `.last` Transform under the indicators: the second destination's.
            let second_scale = |tester: &WidgetTester| {
                let indicator = tester.find_by_type::<NavigationIndicator>()[1];
                let transform = *tester
                    .descendants_of_type::<waft_widgets::Transform>(indicator)
                    .last()
                    .expect("a Transform under the indicator");
                tester.widget(transform, |transform: &waft_widgets::Transform| {
                    f64::from(transform.transform.to_flutter_array()[0])
                })
            };
            assert_eq!(second_scale(&tester), 0.0);

            build(&mut tester, 1);
            tester.pump(Duration::from_millis(100));
            let scale = second_scale(&tester);
            // Dart's `closeTo(.., precisionErrorTolerance)`, within the `f32` of valo's matrix.
            assert!((scale - 0.970_502_395_629_882_8).abs() < 1e-6, "{scale}");

            tester.pump(Duration::from_millis(100));
            assert_eq!(second_scale(&tester), 1.0);
        });
    }

    /// Dart's `_getIndicatorDecoration`: the decoration of the first `Ink` under a
    /// `FadeTransition`, as a `ShapeDecoration`.
    fn indicator_decoration<R>(
        tester: &WidgetTester,
        read: impl FnOnce(&waft_painting::ShapeDecoration) -> R,
    ) -> R {
        let ink = tester
            .find_by_type::<waft_widgets::FadeTransition>()
            .into_iter()
            .find_map(|fade| {
                tester
                    .descendants_of_type::<crate::ink_decoration::Ink>(fade)
                    .first()
                    .copied()
            })
            .expect("an Ink under a FadeTransition");
        tester.widget(ink, |ink: &crate::ink_decoration::Ink| {
            read(
                ink.decoration
                    .as_deref()
                    .and_then(|decoration| decoration.as_any().downcast_ref())
                    .expect("a ShapeDecoration"),
            )
        })
    }

    #[test]
    fn navigation_destination_updates_indicator_color_and_shape() {
        with_main_thread_stack(|| {
            let color = Color::new(0xff0000ff);
            let shape = waft_painting::RoundedRectangleBorder::default();
            let build_navigation_rail = |tester: &mut WidgetTester,
                                         indicator: Option<(
                Color,
                waft_painting::RoundedRectangleBorder,
            )>| {
                let mut rail = NavigationRail::new(destinations(), Some(0)).use_indicator(true);
                if let Some((color, shape)) = indicator {
                    rail = rail.indicator_color(color).indicator_shape(shape);
                }
                pump_rail_in_app(tester, rail);
            };
            let mut tester = tester();
            let theme = ThemeData::new(&tester.cell.borrow());
            build_navigation_rail(&mut tester, None);

            // Test default indicator color and shape.
            assert_eq!(
                indicator_decoration(&tester, |decoration| decoration.color.clone()),
                Some(theme.color_scheme.secondary_container().into())
            );
            assert!(indicator_decoration(&tester, |decoration| {
                &*decoration.shape
                    == &waft_painting::StadiumBorder::new(waft_painting::BorderSide::NONE)
                        as &dyn ShapeBorder
            }));

            build_navigation_rail(&mut tester, Some((color, shape)));

            // Test custom indicator color and shape.
            assert_eq!(
                indicator_decoration(&tester, |decoration| decoration.color.clone()),
                Some(color.into())
            );
            assert!(indicator_decoration(&tester, |decoration| {
                &*decoration.shape == &shape as &dyn ShapeBorder
            }));
        });
    }

    #[test]
    fn destination_s_respect_their_disabled_state() {
        with_main_thread_stack(|| {
            let selected_index = Rc::new(std::cell::Cell::new(None));
            let seen = Rc::clone(&selected_index);
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(
                    vec![
                        destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
                        destination(Icons::star_border(), Icons::star(), "Bcd"),
                        destination(Icons::bookmark_border(), Icons::bookmark(), "Cde")
                            .disabled(true),
                    ],
                    Some(0),
                )
                .on_destination_selected(move |_app, index| seen.set(Some(index)))
                .label_type(NavigationRailLabelType::All),
            );

            tester.tap(center_of_text(&tester, "Abc"), 1);
            assert_eq!(selected_index.get(), Some(0));

            tester.tap(center_of_text(&tester, "Bcd"), 2);
            assert_eq!(selected_index.get(), Some(1));

            tester.tap(center_of_text(&tester, "Cde"), 3);
            assert_eq!(selected_index.get(), Some(1));

            // Wait for any pending shader compilation.
            tester.pump_and_settle();
        });
    }

    #[test]
    #[expect(
        deprecated,
        reason = "Dart's case reads the deprecated `Color.opacity`"
    )]
    fn destination_s_label_with_the_right_opacity_while_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(
                    vec![
                        destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
                        destination(Icons::bookmark_border(), Icons::bookmark(), "Bcd")
                            .disabled(true),
                    ],
                    Some(0),
                )
                .on_destination_selected(|_app, _index| {})
                .label_type(NavigationRailLabelType::All),
            );

            tester.pump_and_settle();

            let default_text_style_opacity = |text: &str| {
                let element =
                    tester.ancestor_of_type::<DefaultTextStyle>(tester.find_text(text)[0]);
                tester.widget(element, |style: &DefaultTextStyle| {
                    style.style.color.as_deref().map(|color| color.opacity())
                })
            };

            let abc_label_opacity = default_text_style_opacity("Abc");
            let bcd_label_opacity = default_text_style_opacity("Bcd");

            assert_eq!(abc_label_opacity, Some(1.0));
            let bcd = bcd_label_opacity.expect("a colour");
            assert!((bcd - 0.38).abs() <= 0.01, "{bcd}");
        });
    }

    #[test]
    fn navigation_rail_indicator_inkwell_can_be_transparent() {
        with_main_thread_stack(|| {
            // This is a regression test for https://github.com/flutter/flutter/issues/135866.
            let mut tester = tester();
            let root = {
                let app = tester.cell.borrow();
                let mut color_scheme = crate::color_scheme::ColorScheme::light();
                color_scheme.primary = Colors::TRANSPARENT;
                let theme = ThemeData::builder()
                    .color_scheme(color_scheme)
                    // Material 3 defaults to InkSparkle which is not testable using paints.
                    .splash_factory(crate::ink_splash::InkSplash::splash_factory())
                    .build(&app);
                material_app(
                    theme,
                    Scaffold::new()
                        .body(
                            Row::new().children([
                                NavigationRail::new(
                                    vec![
                                        destination(
                                            Icons::favorite_border(),
                                            Icons::favorite(),
                                            "ABC",
                                        ),
                                        destination(
                                            Icons::bookmark_border(),
                                            Icons::bookmark(),
                                            "DEF",
                                        ),
                                    ],
                                    Some(1),
                                )
                                .label_type(NavigationRailLabelType::All)
                                .into_widget(),
                                Expanded::new(Text::new("body")).into_widget(),
                            ]),
                        )
                        .into_widget(),
                )
            };
            tester.pump_widget(root);

            let at = center_of_icon(&tester, Icons::favorite_border());
            let mut mouse = WidgetTester::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            tester.move_to(&mut mouse, at, false);
            tester.pump_and_settle();

            // Dart's `paints..rect(color: Colors.transparent)`: Valo's canvas drops a draw whose
            // paint is transparent, so the hover highlight has no entry; the case asserts that
            // no rect is drawn instead, as a hover in the primary colour would draw one.
            let draws_rect = |ops: &[waft_embedder::valo::Op]| {
                ops.iter()
                    .any(|op| matches!(op, waft_embedder::valo::Op::DrawRect { .. }))
            };
            assert!(!draws_rect(&rail_ink_ops(&tester)));

            tester.down(&mut mouse, at);
            tester.pump(Duration::ZERO); // Start the splash and highlight animations.
            // Wait for splash and highlight to be well under way.
            tester.pump(Duration::from_millis(800));

            // Dart's `paints..circle(color: Colors.transparent)`: likewise no splash circle is
            // drawn instead.
            let ops = rail_ink_ops(&tester);
            let draws_circle = ops.iter().any(|op| {
                matches!(
                    op,
                    waft_embedder::valo::Op::DrawPath { path, .. } if path.as_oval().is_some()
                )
            });
            assert!(!draws_circle);
        });
    }

    #[test]
    fn navigation_rail_can_have_expanded_widgets_inside() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(
                    vec![destination(
                        Icons::favorite_border(),
                        Icons::favorite(),
                        "Abc",
                    )],
                    Some(0),
                )
                .trailing(Expanded::new(Icon::new(Some(Icons::search())))),
            );

            // Dart's `expect(tester.takeException(), isNull)`: the frame did not panic.
            tester.pump_and_settle();
        });
    }

    #[test]
    fn navigation_rail_labels_shall_not_overflow_if_longer_texts_provided_extended() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/110901.
            // The navigation rail has a narrow width constraint. The text should wrap.
            let normal_label = "Abc";
            let long_label = "Very long bookmark text for navigation destination";
            let mut tester = tester();
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    ThemeData::new(&app),
                    Scaffold::new()
                        .body(
                            Row::new().children([
                                SizedBox::new()
                                    .width(140.0)
                                    .child(
                                        NavigationRail::new(
                                            vec![
                                                destination(
                                                    Icons::favorite_border(),
                                                    Icons::favorite(),
                                                    normal_label,
                                                ),
                                                destination(
                                                    Icons::bookmark_border(),
                                                    Icons::bookmark(),
                                                    long_label,
                                                ),
                                            ],
                                            Some(1),
                                        )
                                        .extended(true),
                                    )
                                    .into_widget(),
                                Expanded::new(Text::new("body")).into_widget(),
                            ]),
                        )
                        .into_widget(),
                )
            };
            tester.pump_widget(root);

            assert_eq!(tester.find_by_type::<NavigationRail>().len(), 1);
            assert_eq!(tester.find_text(normal_label).len(), 1);
            assert_eq!(tester.find_text(long_label).len(), 1);

            // If the widget manages to layout without throwing an overflow exception,
            // the test passes.
        });
    }

    #[test]
    fn navigation_rail_can_scroll_in_low_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    ThemeData::new(&app),
                    Builder::new(|app, context| {
                        MediaQuery::new(
                            // Set Screen height with 300
                            MediaQuery::of(app, context)
                                .copy_with()
                                .size(Size::new(800.0, 300.0)),
                            Scaffold::new().body(
                                Row::new().children([
                                    // Set NavigationRail height with 100
                                    SizedBox::new()
                                        .height(100.0)
                                        .child(
                                            NavigationRail::new(abc_def_ghi(), Some(0))
                                                .scrollable(true),
                                        )
                                        .into_widget(),
                                    Expanded::new(Text::new("body")).into_widget(),
                                ]),
                            ),
                        )
                        .into_widget()
                    })
                    .into_widget(),
                )
            };
            tester.pump_widget(root);

            let scrollable = tester.find_by_type::<waft_widgets::Scrollable>()[0];
            let mut app = tester.cell.borrow_mut();
            let state = scrollable
                .state_handle::<waft_widgets::ScrollableState>(&app)
                .expect("a ScrollableState");
            let position = state.position(&app);
            position.jump_to(&mut app, 500.0);
            assert_eq!(position.pixels(&app), 500.0);
        });
    }

    /// The rail of the leading/trailing placement cases, 140 wide.
    fn pump_leading_trailing_rail(
        tester: &mut WidgetTester,
        configure: impl FnOnce(NavigationRail) -> NavigationRail,
    ) -> (GlobalKey, GlobalKey) {
        let leading_key = GlobalKey::new();
        let trailing_key = GlobalKey::new();
        let rail = configure(
            NavigationRail::new(
                vec![
                    NavigationRailDestination::new(
                        Icon::new(Some(Icons::favorite())),
                        Text::new("Abc"),
                    ),
                    NavigationRailDestination::new(
                        Icon::new(Some(Icons::bookmark())),
                        Text::new("Def"),
                    ),
                    NavigationRailDestination::new(
                        Icon::new(Some(Icons::star())),
                        Text::new("Ghi"),
                    ),
                ],
                Some(0),
            )
            .extended(true)
            .group_alignment(0.0)
            .leading(
                SizedBox::new()
                    .key(KeyRef::from(leading_key.clone()))
                    .height(50.0),
            )
            .trailing(
                SizedBox::new()
                    .key(KeyRef::from(trailing_key.clone()))
                    .height(50.0),
            ),
        );
        let root = {
            let app = tester.cell.borrow();
            material_app(
                ThemeData::new(&app),
                Scaffold::new()
                    .body(Row::new().children([
                        SizedBox::new().width(140.0).child(rail).into_widget(),
                        Expanded::new(Text::new("body")).into_widget(),
                    ]))
                    .into_widget(),
            )
        };
        tester.pump_widget(root);
        (leading_key, trailing_key)
    }

    /// Dart's `tester.getRect(find.ancestor(of: find.byIcon(icon), matching:
    /// find.byType(Semantics)).first)`.
    fn destination_rect(tester: &WidgetTester, icon: IconData) -> Rect {
        let semantics = tester.ancestor_of_type::<Semantics>(tester.find_by_icon(&icon));
        tester.get_rect(tester.render_box_of(semantics))
    }

    #[test]
    fn navigation_rail_leading_widget_is_at_top_and_trailing_widget_is_at_last_destination_defaults()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (leading_key, trailing_key) = pump_leading_trailing_rail(&mut tester, |rail| rail);

            let leading_rect = tester.get_rect(render_box_of_key(&tester, &leading_key));
            let trailing_rect = tester.get_rect(render_box_of_key(&tester, &trailing_key));
            let first_dest_rect = destination_rect(&tester, Icons::favorite());
            let last_dest_rect = destination_rect(&tester, Icons::star());

            assert_eq!(leading_rect.top, 8.0);
            assert!(first_dest_rect.top - leading_rect.bottom > 100.0);
            assert_eq!(trailing_rect.top - last_dest_rect.bottom, 0.0);
            assert!(trailing_rect.bottom < 500.0);
        });
    }

    #[test]
    fn navigation_rail_leading_at_top_set_to_false_and_trailing_at_bottom_to_true() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (leading_key, trailing_key) = pump_leading_trailing_rail(&mut tester, |rail| {
                rail.leading_at_top(false).trailing_at_bottom(true)
            });

            let leading_rect = tester.get_rect(render_box_of_key(&tester, &leading_key));
            let trailing_rect = tester.get_rect(render_box_of_key(&tester, &trailing_key));
            let first_dest_rect = destination_rect(&tester, Icons::favorite());
            let last_dest_rect = destination_rect(&tester, Icons::star());

            assert!(leading_rect.top > 100.0);
            assert_eq!(first_dest_rect.top - leading_rect.bottom, 8.0);
            assert!(trailing_rect.top - last_dest_rect.bottom > 100.0);
            assert_eq!(trailing_rect.bottom, 600.0);
        });
    }

    #[test]
    fn navigation_rail_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    ThemeData::new(&app),
                    Center::new()
                        .child(SizedBox::shrink().child(NavigationRail::new(
                            vec![NavigationRailDestination::new(
                                Icon::new(Some(Icons::abc())),
                                Text::new("X"),
                            )],
                            Some(0),
                        )))
                        .into_widget(),
                )
            };
            tester.pump_widget(root);
            assert_eq!(size_of(&tester, rail_box(&tester)), Size::ZERO);
        });
    }

    #[test]
    fn navigation_rail_respects_main_axis_alignment() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .main_axis_alignment(MainAxisAlignment::SpaceEvenly),
            );

            // Find the exact column by checking both alignment and size
            let columns = tester
                .find_by_type::<Column>()
                .into_iter()
                .filter(|element| {
                    tester.widget(*element, |column: &Column| {
                        column.main_axis_alignment == MainAxisAlignment::SpaceEvenly
                            && column.main_axis_size == MainAxisSize::Max
                    })
                })
                .count();
            assert_eq!(columns, 1);
        });
    }

    // These tests are only relevant for Material 2. Once Material 2
    // support is deprecated and the APIs are removed, these tests
    // can be deleted.

    fn m2() -> Pump {
        Pump {
            use_material3: false,
            ..Pump::default()
        }
    }

    fn m2_default_width(label_type: Option<NavigationRailLabelType>) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let mut rail = NavigationRail::new(destinations(), Some(0));
            if let Some(label_type) = label_type {
                rail = rail.label_type(label_type);
            }
            pump_navigation_rail_with(&mut tester, rail, m2());

            assert_eq!(size_of(&tester, rail_box(&tester)).width(), 72.0);
        });
    }

    #[test]
    fn material_2_renders_at_the_correct_default_width_label_type_none_default() {
        m2_default_width(None);
    }

    #[test]
    fn material_2_renders_at_the_correct_default_width_label_type_selected() {
        m2_default_width(Some(NavigationRailLabelType::Selected));
    }

    #[test]
    fn material_2_renders_at_the_correct_default_width_label_type_all() {
        m2_default_width(Some(NavigationRailLabelType::All));
    }

    /// The Material 2 icon-only destination spacing cases at `text_scale_factor`, `min_width`
    /// wide: Since the rail is icon only, its destinations should not be affected by
    /// textScaleFactor.
    fn m2_icon_only_spacing(text_scale_factor: f64, min_width: Option<f64>) {
        with_main_thread_stack(|| {
            let destination_width = min_width.unwrap_or(72.0);
            let mut tester = tester();
            let mut rail = NavigationRail::new(destinations(), Some(0));
            if let Some(min_width) = min_width {
                rail = rail.min_width(min_width);
            }
            pump_navigation_rail_with(
                &mut tester,
                rail,
                Pump {
                    text_scale_factor,
                    ..m2()
                },
            );

            assert_eq!(
                size_of(&tester, rail_box(&tester)).width(),
                destination_width
            );

            // The first destination is 8 from the top because of the default vertical
            // padding at the to of the rail; each next one is one width below the one before.
            expect_icon_column_in(
                &tester,
                destination_width,
                destination_width,
                8.0,
                [destination_width; 3],
            );
        });
    }

    #[test]
    fn material_2_destination_spacing_is_correct_label_type_none_default_text_scale_factor_1_0_default()
     {
        m2_icon_only_spacing(1.0, None);
    }

    #[test]
    fn material_2_destination_spacing_is_correct_label_type_none_default_text_scale_factor_3_0() {
        m2_icon_only_spacing(3.0, None);
    }

    #[test]
    fn material_2_destination_spacing_is_correct_label_type_none_default_text_scale_factor_0_75() {
        m2_icon_only_spacing(0.75, None);
    }

    /// The Material 2 selected-label spacing cases whose rail keeps its 72 width (a
    /// textScaleFactor of 1.0 and 0.75: a smaller one will not reduce the default width of the
    /// rail since there is a minWidth).
    fn m2_selected_spacing(text_scale_factor: f64) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail_with(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::Selected),
                Pump {
                    text_scale_factor,
                    ..m2()
                },
            );

            assert_eq!(size_of(&tester, rail_box(&tester)).width(), 72.0);

            // The first destination is 8 from the top because of the default vertical
            // padding at the to of the rail.
            let next_destination_y = 8.0;
            let icon = icon_render_box(&tester, Icons::favorite());
            let label = label_render_box(&tester, "Abc");
            let icon_size = size_of(&tester, icon);
            let label_size = size_of(&tester, label);
            assert_offset(
                global_top_left(&tester, icon),
                Offset::new(
                    (72.0 - icon_size.width()) / 2.0,
                    next_destination_y + (72.0 - icon_size.height() - label_size.height()) / 2.0,
                ),
            );
            assert_offset(
                global_top_left(&tester, label),
                Offset::new(
                    (72.0 - label_size.width()) / 2.0,
                    next_destination_y + (72.0 + icon_size.height() - label_size.height()) / 2.0,
                ),
            );

            // The other destinations are 72 below the one before.
            let mut next_destination_y = next_destination_y;
            for icon in &icons_in_order()[1..] {
                next_destination_y += 72.0;
                expect_icon_at(
                    &tester,
                    icon_render_box(&tester, icon.clone()),
                    72.0,
                    72.0,
                    next_destination_y,
                );
            }
        });
    }

    #[test]
    fn material_2_destination_spacing_is_correct_label_type_selected_text_scale_factor_1_0_default()
    {
        m2_selected_spacing(1.0);
    }

    #[test]
    fn material_2_destination_spacing_is_correct_label_type_selected_text_scale_factor_0_75() {
        m2_selected_spacing(0.75);
    }

    /// The Material 2 labelled spacing cases at a textScaleFactor of 3.0: the rail and
    /// destinations sizes grow to fit the larger text labels.
    fn m2_large_text_labelled_spacing(label_type: NavigationRailLabelType) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail_with(
                &mut tester,
                NavigationRail::new(destinations(), Some(0)).label_type(label_type),
                Pump {
                    text_scale_factor: 3.0,
                    ..m2()
                },
            );

            assert_eq!(size_of(&tester, rail_box(&tester)).width(), 142.0);

            // The first destination is 8 from the top because of the default vertical
            // padding at the to of the rail.
            let mut next_destination_y = 8.0;
            for (i, (icon, label)) in icons_in_order()
                .into_iter()
                .zip(["Abc", "Def", "Ghi", "Jkl"])
                .enumerate()
            {
                let icon = icon_render_box(&tester, icon);
                let icon_size = size_of(&tester, icon);
                let labelled = i == 0 || label_type == NavigationRailLabelType::All;
                if !labelled {
                    // An unselected destination is 72 high with its icon 24 below its top.
                    assert_offset(
                        global_top_left(&tester, icon),
                        Offset::new((142.0 - icon_size.width()) / 2.0, next_destination_y + 24.0),
                    );
                    next_destination_y += 72.0;
                    continue;
                }
                assert_offset(
                    global_top_left(&tester, icon),
                    Offset::new((142.0 - icon_size.width()) / 2.0, next_destination_y + 16.0),
                );
                // The label sits right below the icon.
                let label = label_render_box(&tester, label);
                let label_size = size_of(&tester, label);
                assert_offset(
                    global_top_left(&tester, label),
                    Offset::new(
                        (142.0 - label_size.width()) / 2.0,
                        next_destination_y + 16.0 + icon_size.height(),
                    ),
                );
                next_destination_y += 16.0 + icon_size.height() + label_size.height() + 16.0;
            }
        });
    }

    #[test]
    fn material_2_destination_spacing_is_correct_label_type_selected_text_scale_factor_3_0() {
        m2_large_text_labelled_spacing(NavigationRailLabelType::Selected);
    }

    #[test]
    fn material_2_destination_spacing_is_correct_label_type_all_text_scale_factor_3_0() {
        m2_large_text_labelled_spacing(NavigationRailLabelType::All);
    }

    #[test]
    fn material_2_destination_spacing_is_correct_label_type_all_text_scale_factor_1_0_default() {
        m2_all_spacing(1.0);
    }

    #[test]
    fn material_2_destination_spacing_is_correct_label_type_all_text_scale_factor_0_75() {
        m2_all_spacing(0.75);
    }

    /// The Material 2 all-labels spacing cases whose rail keeps its 72 width (a textScaleFactor
    /// of 1.0 and 0.75: a smaller one will not reduce the default size of the rail).
    fn m2_all_spacing(text_scale_factor: f64) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail_with(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .label_type(NavigationRailLabelType::All),
                Pump {
                    text_scale_factor,
                    ..m2()
                },
            );

            assert_eq!(size_of(&tester, rail_box(&tester)).width(), 72.0);

            // The first destination is 8 from the top because of the default vertical
            // padding at the to of the rail; each next one is 72 below the one before.
            let mut next_destination_y = 8.0;
            for (icon, label) in icons_in_order()
                .into_iter()
                .zip(["Abc", "Def", "Ghi", "Jkl"])
            {
                let icon = icon_render_box(&tester, icon);
                let label = label_render_box(&tester, label);
                let icon_size = size_of(&tester, icon);
                assert_offset(
                    global_top_left(&tester, icon),
                    Offset::new((72.0 - icon_size.width()) / 2.0, next_destination_y + 16.0),
                );
                assert_offset(
                    global_top_left(&tester, label),
                    Offset::new(
                        (72.0 - size_of(&tester, label).width()) / 2.0,
                        next_destination_y + 16.0 + icon_size.height(),
                    ),
                );
                next_destination_y += 72.0;
            }
        });
    }

    fn m2_group_alignment(group_alignment: Option<f64>, first_y: f64) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let mut rail = NavigationRail::new(destinations(), Some(0));
            if let Some(group_alignment) = group_alignment {
                rail = rail.group_alignment(group_alignment);
            }
            pump_navigation_rail_with(&mut tester, rail, m2());

            // Each next destination is 72 below the one before.
            expect_icon_column_in(&tester, 72.0, 72.0, first_y, [72.0; 3]);
        });
    }

    #[test]
    fn material_2_group_alignment_works_group_alignment_minus_1_0_default() {
        m2_group_alignment(None, 8.0);
    }

    #[test]
    fn material_2_group_alignment_works_group_alignment_0_0() {
        m2_group_alignment(Some(0.0), 160.0);
    }

    #[test]
    fn material_2_group_alignment_works_group_alignment_1_0() {
        m2_group_alignment(Some(1.0), 312.0);
    }

    #[test]
    fn material_2_destination_spacing_is_correct_for_a_compact_rail_preferred_width_56_text_scale_factor_1_0_default()
     {
        m2_icon_only_spacing(1.0, Some(56.0));
    }

    #[test]
    fn material_2_destination_spacing_is_correct_for_a_compact_rail_preferred_width_56_text_scale_factor_3_0()
     {
        m2_icon_only_spacing(3.0, Some(56.0));
    }

    /// Dart's case of this name pumps a textScaleFactor of 3.0, not 0.75.
    #[test]
    fn material_2_destination_spacing_is_correct_for_a_compact_rail_preferred_width_56_text_scale_factor_0_75()
     {
        m2_icon_only_spacing(3.0, Some(56.0));
    }

    #[test]
    fn material_2_extended_rail_final_width_can_be_changed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let rail = |extended: bool| {
                NavigationRail::new(destinations(), Some(0))
                    .min_extended_width(300.0)
                    .extended(extended)
            };
            pump_navigation_rail_with(&mut tester, rail(false), m2());

            let rail_render = rail_box(&tester);
            assert_eq!(size_of(&tester, rail_render).width(), 72.0);

            pump_navigation_rail_with(&mut tester, rail(true), m2());
            tester.pump_and_settle();
            assert_eq!(size_of(&tester, rail_render).width(), 300.0);
        });
    }

    #[test]
    fn material_2_leading_and_trailing_spacing_is_correct_with_0_2_destinations() {
        with_main_thread_stack(|| {
            let leading_widget_key = GlobalKey::new();
            let trailing_widget_key = GlobalKey::new();
            let fab = |key: &GlobalKey| {
                FloatingActionButton::new(Some(Listener::new(|_| {})))
                    .key(KeyRef::from(key.clone()))
                    .into_widget()
            };
            let one = || {
                vec![destination(
                    Icons::favorite_border(),
                    Icons::favorite(),
                    "Abc",
                )]
            };
            let two = || {
                vec![
                    destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
                    destination(Icons::bookmark_border(), Icons::bookmark(), "Longer Label"),
                ]
            };
            let key_at = |tester: &WidgetTester, key: &GlobalKey| {
                global_top_left(tester, render_box_of_key(tester, key))
            };
            let icon_at = |tester: &WidgetTester, icon: IconData| {
                global_top_left(tester, icon_render_box(tester, icon))
            };
            let label_at = |tester: &WidgetTester, text: &str| {
                global_top_left(tester, label_render_box(tester, text))
            };

            let mut tester = tester();
            pump_rail_in_app_with(&mut tester, NavigationRail::new(Vec::new(), None), false);

            // empty destinations and leading widget
            pump_rail_in_app_with(
                &mut tester,
                NavigationRail::new(Vec::new(), None).leading(fab(&leading_widget_key)),
                false,
            );
            tester.pump_and_settle();
            assert_eq!(key_at(&tester, &leading_widget_key), Offset::new(0.0, 8.0));

            // one destination and leading widget
            pump_rail_in_app_with(
                &mut tester,
                NavigationRail::new(one(), None).leading(fab(&leading_widget_key)),
                false,
            );
            tester.pump_and_settle();
            assert_offset(
                icon_at(&tester, Icons::favorite_border()),
                Offset::new(24.0, 96.0),
            );
            assert_offset(label_at(&tester, "Abc"), Offset::new(0.0, 72.0));
            assert_offset(key_at(&tester, &leading_widget_key), Offset::new(8.0, 8.0));

            // two destinations and leading widget
            pump_rail_in_app_with(
                &mut tester,
                NavigationRail::new(two(), None).leading(fab(&leading_widget_key)),
                false,
            );
            tester.pump_and_settle();
            assert_offset(
                icon_at(&tester, Icons::favorite_border()),
                Offset::new(24.0, 96.0),
            );
            assert_offset(label_at(&tester, "Abc"), Offset::new(0.0, 72.0));
            assert_offset(
                icon_at(&tester, Icons::bookmark_border()),
                Offset::new(24.0, 168.0),
            );
            assert_offset(label_at(&tester, "Longer Label"), Offset::new(0.0, 144.0));
            assert_offset(key_at(&tester, &leading_widget_key), Offset::new(8.0, 8.0));

            // empty destinations and trailing widget
            pump_rail_in_app_with(
                &mut tester,
                NavigationRail::new(Vec::new(), None).trailing(fab(&trailing_widget_key)),
                false,
            );
            tester.pump_and_settle();
            assert_eq!(key_at(&tester, &trailing_widget_key), Offset::new(0.0, 8.0));

            // one destination and trailing widget
            pump_rail_in_app_with(
                &mut tester,
                NavigationRail::new(one(), None).trailing(fab(&trailing_widget_key)),
                false,
            );
            tester.pump_and_settle();
            assert_offset(
                icon_at(&tester, Icons::favorite_border()),
                Offset::new(24.0, 32.0),
            );
            assert_offset(label_at(&tester, "Abc"), Offset::new(0.0, 8.0));
            assert_offset(
                key_at(&tester, &trailing_widget_key),
                Offset::new(8.0, 80.0),
            );

            // two destinations and trailing widget
            pump_rail_in_app_with(
                &mut tester,
                NavigationRail::new(two(), None).trailing(fab(&trailing_widget_key)),
                false,
            );
            tester.pump_and_settle();
            assert_offset(
                icon_at(&tester, Icons::favorite_border()),
                Offset::new(24.0, 32.0),
            );
            assert_offset(label_at(&tester, "Abc"), Offset::new(0.0, 8.0));
            assert_offset(
                icon_at(&tester, Icons::bookmark_border()),
                Offset::new(24.0, 104.0),
            );
            assert_offset(label_at(&tester, "Longer Label"), Offset::new(0.0, 80.0));
            assert_offset(
                key_at(&tester, &trailing_widget_key),
                Offset::new(8.0, 152.0),
            );
        });
    }

    #[test]
    fn material_2_change_destinations_and_selected_index() {
        with_main_thread_stack(|| {
            let two = || {
                vec![
                    destination(Icons::favorite_border(), Icons::favorite(), "Abc"),
                    destination(Icons::bookmark_border(), Icons::bookmark(), "Longer Label"),
                ]
            };
            let mut tester = tester();
            pump_rail_in_app_with(&mut tester, NavigationRail::new(Vec::new(), None), false);

            assert_eq!(rail_widget(&tester, |rail| rail.destinations.len()), 0);
            assert_eq!(rail_widget(&tester, |rail| rail.selected_index), None);

            pump_rail_in_app_with(&mut tester, NavigationRail::new(two(), None), false);
            tester.pump_and_settle();

            assert_eq!(rail_widget(&tester, |rail| rail.destinations.len()), 2);
            assert_eq!(rail_widget(&tester, |rail| rail.selected_index), None);

            pump_rail_in_app_with(&mut tester, NavigationRail::new(two(), Some(0)), false);
            tester.pump_and_settle();

            assert_eq!(rail_widget(&tester, |rail| rail.destinations.len()), 2);
            assert_eq!(rail_widget(&tester, |rail| rail.selected_index), Some(0));

            pump_rail_in_app_with(
                &mut tester,
                NavigationRail::new(
                    vec![destination(
                        Icons::favorite_border(),
                        Icons::favorite(),
                        "Abc",
                    )],
                    Some(0),
                ),
                false,
            );
            tester.pump_and_settle();

            assert_eq!(rail_widget(&tester, |rail| rail.destinations.len()), 1);
            assert_eq!(rail_widget(&tester, |rail| rail.selected_index), Some(0));
        });
    }

    #[test]
    fn material_2_leading_and_trailing_appear_in_the_correct_places() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail_with(
                &mut tester,
                NavigationRail::new(destinations(), Some(0))
                    .leading(fab())
                    .trailing(fab()),
                m2(),
            );

            let fabs = tester.find_by_type::<FloatingActionButton>();
            let leading = tester.render_box_of(fabs[0]);
            let trailing = tester.render_box_of(fabs[1]);
            assert_eq!(
                global_top_left(&tester, leading),
                Offset::new((72.0 - size_of(&tester, leading).width()) / 2.0, 8.0)
            );
            assert_eq!(
                global_top_left(&tester, trailing),
                Offset::new((72.0 - size_of(&tester, trailing).width()) / 2.0, 360.0)
            );
        });
    }

    #[test]
    fn material_2_extended_rail_animates_the_width_and_labels_appear_text_direction_rtl() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_rtl_rail_with(
                &mut tester,
                NavigationRail::new(destinations(), Some(0)),
                false,
            );

            let rail = rail_box(&tester);
            assert_eq!(size_of(&tester, rail).width(), 72.0);
            assert_eq!(global_top_left(&tester, rail), Offset::new(728.0, 0.0));

            pump_rtl_rail_with(
                &mut tester,
                NavigationRail::new(destinations(), Some(0)).extended(true),
                false,
            );
            tester.pump(Duration::from_millis(100));
            assert_eq!(size_of(&tester, rail).width(), 164.0);
            assert_eq!(global_top_left(&tester, rail), Offset::new(636.0, 0.0));

            tester.pump_and_settle();
            assert_eq!(size_of(&tester, rail).width(), 256.0);
            assert_eq!(global_top_left(&tester, rail), Offset::new(544.0, 0.0));

            // The first destination is 8 from the top because of the default vertical
            // padding at the to of the rail; each next one is 72 below the one before.
            let mut next_destination_y = 8.0;
            for (icon, label) in icons_in_order()
                .into_iter()
                .zip(["Abc", "Def", "Ghi", "Jkl"])
            {
                let icon = icon_render_box(&tester, icon);
                let icon_size = size_of(&tester, icon);
                assert_offset(
                    global_top_left(&tester, icon),
                    Offset::new(
                        800.0 - (72.0 + icon_size.width()) / 2.0,
                        next_destination_y + (72.0 - icon_size.height()) / 2.0,
                    ),
                );
                let label = label_render_box(&tester, label);
                let label_size = size_of(&tester, label);
                assert_offset(
                    global_top_left(&tester, label),
                    Offset::new(
                        800.0 - 72.0 - label_size.width(),
                        next_destination_y + (72.0 - label_size.height()) / 2.0,
                    ),
                );
                next_destination_y += 72.0;
            }
        });
    }

    #[test]
    fn material_2_extended_rail_gets_wider_with_longer_labels_are_larger_text_scale() {
        with_main_thread_stack(|| {
            let pump = |tester: &mut WidgetTester, extended: bool| {
                let root = {
                    let app = tester.cell.borrow();
                    material_app(
                        theme(&app, false),
                        Scaffold::new()
                            .body(
                                Row::new().children([
                                    MediaQuery::with_clamped_text_scaling(
                                        None,
                                        3.0,
                                        3.0,
                                        NavigationRail::new(abc_and_longer_label(), Some(0))
                                            .extended(extended),
                                    ),
                                    Expanded::new(Text::new("body")).into_widget(),
                                ]),
                            )
                            .into_widget(),
                    )
                };
                tester.pump_widget(root);
            };
            let mut tester = tester();
            pump(&mut tester, false);

            let rail = rail_box(&tester);
            assert_eq!(size_of(&tester, rail).width(), 72.0);

            pump(&mut tester, true);
            tester.pump(Duration::from_millis(100));
            assert_eq!(size_of(&tester, rail).width(), 328.0);

            tester.pump_and_settle();
            assert_eq!(size_of(&tester, rail).width(), 584.0);
        });
    }

    /// Regression test for https://github.com/flutter/flutter/issues/65657
    #[test]
    fn material_2_extended_rail_transition_does_not_jump_from_the_beginning() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_rail_in_app_with(
                &mut tester,
                NavigationRail::new(abc_and_longer_label(), Some(0)),
                false,
            );

            // Before starting the animation, the rail has a width of 72.
            assert_eq!(size_of(&tester, rail_box(&tester)).width(), 72.0);

            pump_rail_in_app_with(
                &mut tester,
                NavigationRail::new(abc_and_longer_label(), Some(0)).extended(true),
                false,
            );
            // Create very close to 0, but non-zero, animation value.
            tester.pump(Duration::from_millis(1));
            let width = size_of(&tester, rail_box(&tester)).width();
            // Expect that it has started to extend.
            assert!(width > 72.0, "{width}");
            // Expect that it has only extended by a small amount, or that the first
            // frame does not jump. This helps verify that it is a smooth animation.
            assert!((width - 72.0).abs() <= 1.0, "{width}");
        });
    }

    #[test]
    fn material_2_extended_rail_animates_the_width_and_labels_appear_text_direction_ltr() {
        with_main_thread_stack(|| {
            let pump = |tester: &mut WidgetTester, extended: bool| {
                let root = {
                    let app = tester.cell.borrow();
                    material_app(
                        theme(&app, false),
                        Scaffold::new()
                            .body(
                                Row::new().children([
                                    NavigationRail::new(destinations(), Some(0))
                                        .extended(extended)
                                        .into_widget(),
                                    Expanded::new(Text::new("body")).into_widget(),
                                ]),
                            )
                            .into_widget(),
                    )
                };
                tester.pump_widget(root);
            };
            let mut tester = tester();
            pump(&mut tester, false);

            let rail = rail_box(&tester);
            assert_eq!(size_of(&tester, rail).width(), 72.0);

            pump(&mut tester, true);
            tester.pump(Duration::from_millis(100));
            assert_eq!(size_of(&tester, rail).width(), 164.0);

            tester.pump_and_settle();
            assert_eq!(size_of(&tester, rail).width(), 256.0);

            // The first destination is 8 from the top because of the default vertical
            // padding at the to of the rail; each next one is 72 below the one before.
            let mut next_destination_y = 8.0;
            for (icon, label) in icons_in_order()
                .into_iter()
                .zip(["Abc", "Def", "Ghi", "Jkl"])
            {
                expect_icon_at(
                    &tester,
                    icon_render_box(&tester, icon),
                    72.0,
                    72.0,
                    next_destination_y,
                );
                let label = label_render_box(&tester, label);
                assert_offset(
                    global_top_left(&tester, label),
                    Offset::new(
                        72.0,
                        next_destination_y + (72.0 - size_of(&tester, label).height()) / 2.0,
                    ),
                );
                next_destination_y += 72.0;
            }
        });
    }

    #[test]
    fn material_2_navigation_rail_destination_adds_circular_indicator_when_no_labels_are_present() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_navigation_rail_with(
                &mut tester,
                NavigationRail::new(abc_def_ghi(), Some(0))
                    .use_indicator(true)
                    .label_type(NavigationRailLabelType::None),
                m2(),
            );

            let indicator = tester.find_by_type::<NavigationIndicator>()[0];
            let (width, height) = tester.widget(indicator, |indicator: &NavigationIndicator| {
                (indicator.width, indicator.height)
            });
            assert_eq!(width, 56.0);
            assert_eq!(height, 56.0);
        });
    }

    #[test]
    fn material_2_navigation_rail_destination_has_center_aligned_indicator_label_type_none() {
        has_center_aligned_indicator(false, 24.0);
    }

    #[test]
    fn material_2_navigation_rail_respects_the_notch_system_navigation_bar_in_landscape_mode() {
        respects_the_notch(false, 72.0);
    }
}
