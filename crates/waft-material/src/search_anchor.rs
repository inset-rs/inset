//! Flutter counterpart: material_ui's `search_anchor.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animation, AnimationRef, Curve, CurvedAnimation, Curves, Interval, RectTween,
};
use waft_embedder::{
    Brightness, Clip, Color, Offset, Rect, SemanticsInputType, Size, SmartDashesType,
    SmartQuotesType, TextCapitalization, TextDirection, TextEditingValue, TextInputAction,
    TextInputType, clamp_double, lerp_double,
};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, ListenableObject, Listener, OptionRetained,
    Ref, Task, Timer, ValueChanged,
};
use waft_painting::{
    AlignmentGeometry, AnyBorderSide, BorderSide, EdgeInsets, EdgeInsetsGeometry,
    OutlinedBorderRef, TextStyle,
};
use waft_rendering::{BoxConstraints, CrossAxisAlignment, MainAxisSize, OverflowBoxFit};
use waft_scheduler::{FrameCallback, SchedulerBinding, TickerFuture};
use waft_widgets::{
    Align, AnimatedBuilder, AnimatedOpacity, AnyRoute, BuildContext, CapturedThemes, Column,
    ConstrainedBox, Directionality, EditableTextContextMenuBuilder, EditableTextState, Expanded,
    FadeTransition, FlexFit, Flexible, FocusNode, FocusNodeLeaf, FocusNodeRef, FocusScope,
    GestureDetector, GlobalKey, Icon, IconTheme, IconThemeData, IgnorePointer, InheritedTheme,
    IntoWidget, KeyRef, LayoutBuilder, ListView, LocalHistoryRoute, LocalHistoryRouteData,
    MediaQuery, ModalRoute, ModalRouteData, Navigator, Opacity, OverflowBox, OverlayRoute,
    OverlayRouteData, Padding, PopupRoute, PredictiveBackRoute, Route, RouteData, RouteResult, Row,
    SafeArea, Semantics, State, StateData, StatefulWidget, SystemContextMenu, TapRegionCallback,
    TextEditingControllerData, TextEditingControllerLeaf, TextEditingControllerRef, Transform,
    TransitionRoute, TransitionRouteData, UnfocusDisposition, WidgetRef, WidgetStatePropertyAll,
    WidgetStatePropertyRef, WidgetStatesController,
};

use crate::action_buttons::BackButton;
use crate::adaptive_text_selection_toolbar::AdaptiveTextSelectionToolbar;
use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::constants::{K_DEFAULT_ICON_DARK_COLOR, K_DEFAULT_ICON_LIGHT_COLOR};
use crate::divider::Divider;
use crate::divider_theme::DividerTheme;
use crate::generated::search_bar_defaults_m3::SearchBarDefaultsM3;
use crate::generated::search_view_defaults_m3::SearchViewDefaultsM3;
use crate::icon_button::IconButton;
use crate::icons::Icons;
use crate::ink_well::InkWell;
use crate::input_border::InputBorderRef;
use crate::input_decorator::{InputDecoration, InputDecorationThemeData};
use crate::material::Material;
use crate::material_localizations::MaterialLocalizations;
use crate::search_bar_theme::{SearchBarTheme, SearchBarThemeDataGetters};
use crate::search_view_theme::{SearchViewTheme, SearchViewThemeData, SearchViewThemeDataGetters};
use crate::text_field::TextField;
use crate::theme::Theme;
use crate::theme_data::MaterialTapTargetSize;

const K_OPEN_VIEW_MILLISECONDS: u64 = 600;
const K_OPEN_VIEW_DURATION: Duration = Duration::from_millis(K_OPEN_VIEW_MILLISECONDS);
const K_ANCHOR_FADE_DURATION: Duration = Duration::from_millis(150);
const K_DISABLE_SEARCH_BAR_OPACITY: f64 = 0.38;

fn k_view_fade_on_interval() -> Rc<dyn Curve> {
    Rc::new(Interval::new(0.0, 1.0 / 2.0, Curves::linear()))
}

fn k_view_icons_fade_on_interval() -> Rc<dyn Curve> {
    Rc::new(Interval::new(1.0 / 6.0, 2.0 / 6.0, Curves::linear()))
}

fn k_view_divider_fade_on_interval() -> Rc<dyn Curve> {
    Rc::new(Interval::new(0.0, 1.0 / 6.0, Curves::linear()))
}

fn k_view_list_fade_on_interval() -> Rc<dyn Curve> {
    Rc::new(Interval::new(
        133.0 / K_OPEN_VIEW_MILLISECONDS as f64,
        233.0 / K_OPEN_VIEW_MILLISECONDS as f64,
        Curves::linear(),
    ))
}

/// Signature for a function that creates a [`WidgetRef`] to build the child of a
/// [`SearchAnchor`].
///
/// Used by [`SearchAnchor::builder`].
pub type SearchAnchorChildBuilder =
    Rc<dyn Fn(&mut App, BuildContext, Handle<SearchController>) -> WidgetRef>;

/// Signature for a function that creates a list of suggestion widgets for the search view
/// of a [`SearchAnchor`].
///
/// Dart's `FutureOr<Iterable<Widget>>` is a [`Task`]: a builder with its suggestions at hand
/// answers [`Task::ready`], which reaches the view at the next checkpoint, as Dart's `await` of
/// a plain value resumes from the microtask queue.
///
/// Used by [`SearchAnchor::suggestions_builder`].
pub type SuggestionsBuilder =
    Rc<dyn Fn(&mut App, BuildContext, Handle<SearchController>) -> Task<Vec<WidgetRef>>>;

/// Signature for a function that creates a [`WidgetRef`] to build the suggestion list
/// of a [`SearchAnchor`] in its search view.
///
/// Used by [`SearchAnchor::view_builder`].
pub type ViewBuilder = Rc<dyn Fn(&mut App, Vec<WidgetRef>) -> WidgetRef>;

/// Creates a search bar that when focused opens a search view to input a search query.
///
/// This widget is the anchor of a search view: the search view is shown over the anchor, and
/// closes back to it. The [`builder`](Self::builder) creates the anchor and the
/// [`suggestions_builder`](Self::suggestions_builder) creates the suggestions in the search
/// view. A [`SearchController`] opens and closes the view and holds what is typed in it.
///
/// See also:
///
///  * [`SearchBar`], a widget that defines a search bar.
///  * [`SearchBarTheme`], a widget that overrides the default configuration of a search bar.
///  * [`SearchViewTheme`], a widget that overrides the default configuration of a search
///    view.
#[derive(Clone)]
pub struct SearchAnchor {
    pub key: Option<KeyRef>,

    /// Whether the search view grows to fill the entire screen when the [`SearchBar`] is
    /// tapped.
    ///
    /// By default, the search view is full-screen on mobile devices. On other platforms, the
    /// search view only grows to a specific size that is determined by the anchor and the
    /// default size.
    pub is_full_screen: Option<bool>,

    /// An optional controller that allows opening and closing of the search view from other
    /// widgets.
    ///
    /// If this is null, one internal search controller is created automatically. It can
    /// be accessed through [`SearchAnchorChildBuilder`] or [`SuggestionsBuilder`].
    pub search_controller: Option<Ref<SearchController>>,

    /// Optional callback to obtain a widget to lay out the suggestion list of the search
    /// view.
    ///
    /// Default view uses a [`ListView`] with a vertical scroll direction.
    pub view_builder: Option<ViewBuilder>,

    /// An optional widget to display before the text input field when the search view is
    /// open.
    ///
    /// Typically the [`view_leading`](Self::view_leading) widget is an [`Icon`] or an
    /// [`IconButton`].
    ///
    /// Defaults to a back button which pops the view.
    pub view_leading: Option<WidgetRef>,

    /// An optional widget list to display after the text input field when the search view is
    /// open.
    ///
    /// Typically the [`view_trailing`](Self::view_trailing) widget list is a list of
    /// [`Icon`]s or [`IconButton`]s.
    ///
    /// Defaults to an icon button which clears the text in the input field.
    pub view_trailing: Option<Vec<WidgetRef>>,

    /// Text that is displayed when the search bar's input field is empty.
    pub view_hint_text: Option<String>,

    /// The search view's background fill color.
    ///
    /// If null, the value of [`SearchViewThemeData::background_color`] will be used. If this
    /// is also null, then the default value is `ColorScheme::surface_container_high`.
    pub view_background_color: Option<Color>,

    /// The elevation of the search view's [`Material`].
    ///
    /// If null, the value of [`SearchViewThemeData::elevation`] will be used. If this is also
    /// null, then default value is 6.0.
    pub view_elevation: Option<f64>,

    /// The surface tint color of the search view's [`Material`].
    ///
    /// If null, the value of [`SearchViewThemeData::surface_tint_color`] will be used. If this
    /// is also null, then the default value is `Colors::TRANSPARENT`.
    pub view_surface_tint_color: Option<Color>,

    /// The color and weight of the search view's outline.
    ///
    /// This value is combined with [`view_shape`](Self::view_shape) to create a shape
    /// decorated with an outline.
    ///
    /// If null, the value of [`SearchViewThemeData::side`] will be used. If this is also null,
    /// the search view doesn't have a side by default.
    pub view_side: Option<AnyBorderSide>,

    /// The shape of the search view's underlying [`Material`].
    ///
    /// This shape is combined with [`view_side`](Self::view_side) to create a shape
    /// decorated with an outline.
    ///
    /// If null, the value of [`SearchViewThemeData::shape`] will be used. If this is also
    /// null, then the default value is a rectangle shape for full-screen mode and a
    /// [`RoundedRectangleBorder`](waft_painting::RoundedRectangleBorder) shape with a 28.0
    /// radius otherwise.
    pub view_shape: Option<OutlinedBorderRef>,

    /// The padding to use for the search view's search bar.
    ///
    /// If null, the value of [`SearchViewThemeData::bar_padding`] will be used. If this is
    /// also null, then the default value is `EdgeInsets::symmetric(0.0, 8.0)`.
    pub view_bar_padding: Option<EdgeInsetsGeometry>,

    /// The height of the search field on the search view.
    ///
    /// If null, the value of [`SearchViewThemeData::header_height`] will be used. If this is
    /// also null, the default value is 56.0.
    pub header_height: Option<f64>,

    /// The style to use for the text being edited on the search view.
    ///
    /// If null, defaults to the `bodyLarge` text style from the current [`Theme`]. The default
    /// text color is `ColorScheme::on_surface`.
    pub header_text_style: Option<TextStyle>,

    /// The style to use for the [`view_hint_text`](Self::view_hint_text) on the search view.
    ///
    /// If null, the value of [`SearchViewThemeData::header_hint_style`] will be used. If this
    /// is also null, the value of `header_text_style` will be used, and then `bodyLarge`.
    pub header_hint_style: Option<TextStyle>,

    /// The color of the divider on the search view.
    ///
    /// If this property is null, then [`SearchViewThemeData::divider_color`] is used. If that
    /// is also null, the default value is `ColorScheme::outline`.
    pub divider_color: Option<Color>,

    /// Optional size constraints for the search view.
    ///
    /// By default, the search view has the same width as the anchor and is 2/3 the height of
    /// the screen. If the width and height of the view are within the range of constraints,
    /// the view will keep the default size.
    ///
    /// If null, the value of [`SearchViewThemeData::constraints`] will be used. If that is
    /// also null, the default constraints are `BoxConstraints { min_width: 360.0, min_height:
    /// 240.0, .. }`.
    pub view_constraints: Option<BoxConstraints>,

    /// The padding to use for the search view.
    ///
    /// Has no effect if the search view is full-screen.
    pub view_padding: Option<EdgeInsetsGeometry>,

    /// Whether the search view should shrink-wrap its contents.
    ///
    /// Has no effect if the search view is full-screen.
    pub shrink_wrap: Option<bool>,

    /// Configures how the platform keyboard will select an uppercase or lowercase keyboard.
    ///
    /// Only supports text keyboards, other keyboard types will ignore this configuration.
    /// Capitalization is locale-aware.
    ///
    /// Defaults to [`TextCapitalization::None`].
    pub text_capitalization: Option<TextCapitalization>,

    /// Called each time the user modifies the search view's text field.
    pub view_on_changed: Option<ValueChanged<String>>,

    /// Called when the user indicates that they are done editing the text in the field.
    pub view_on_submitted: Option<ValueChanged<String>>,

    /// Called when the search view is closed.
    pub view_on_close: Option<Listener>,

    /// Called when the search view is opened.
    pub view_on_open: Option<Listener>,

    /// Called to create a widget which can open a search view route when it is tapped.
    ///
    /// The widget returned by this builder is faded out when it is tapped. At the same time a
    /// search view route is faded in.
    pub builder: SearchAnchorChildBuilder,

    /// Called to get the suggestion list for the search view.
    ///
    /// By default, the list returned by this builder is laid out in a [`ListView`]. To get a
    /// different layout, use [`view_builder`](Self::view_builder) to override.
    pub suggestions_builder: SuggestionsBuilder,

    /// The type of action button to use for the keyboard.
    ///
    /// Defaults to the default value specified in [`TextField`].
    pub text_input_action: Option<TextInputAction>,

    /// The type of action button to use for the keyboard.
    ///
    /// Defaults to the default value specified in [`TextField`].
    pub keyboard_type: Option<TextInputType>,

    /// Whether or not this widget is currently interactive.
    ///
    /// When false, the widget will ignore taps and appear dimmed.
    pub enabled: bool,

    /// Whether to allow the platform to automatically format dashes.
    ///
    /// This flag is only meaningful on an iOS device.
    pub smart_dashes_type: Option<SmartDashesType>,

    /// Whether to allow the platform to automatically format quotes.
    ///
    /// This flag is only meaningful on an iOS device.
    pub smart_quotes_type: Option<SmartQuotesType>,

    /// Whether the anchor opens the search view when it is tapped.
    ///
    /// [`SearchAnchor::bar`] sets this to false because its search bar opens the view itself.
    pub enable_tap_handling: bool,
}

impl SearchAnchor {
    /// Creates a const [`SearchAnchor`].
    ///
    /// The [`builder`](Self::builder) and [`suggestions_builder`](Self::suggestions_builder)
    /// arguments are required.
    pub fn new(
        builder: impl Fn(&mut App, BuildContext, Handle<SearchController>) -> WidgetRef + 'static,
        suggestions_builder: impl Fn(
            &mut App,
            BuildContext,
            Handle<SearchController>,
        ) -> Task<Vec<WidgetRef>>
        + 'static,
    ) -> SearchAnchor {
        SearchAnchor {
            key: None,
            is_full_screen: None,
            search_controller: None,
            view_builder: None,
            view_leading: None,
            view_trailing: None,
            view_hint_text: None,
            view_background_color: None,
            view_elevation: None,
            view_surface_tint_color: None,
            view_side: None,
            view_shape: None,
            view_bar_padding: None,
            header_height: None,
            header_text_style: None,
            header_hint_style: None,
            divider_color: None,
            view_constraints: None,
            view_padding: None,
            shrink_wrap: None,
            text_capitalization: None,
            view_on_changed: None,
            view_on_submitted: None,
            view_on_close: None,
            view_on_open: None,
            builder: Rc::new(builder),
            suggestions_builder: Rc::new(suggestions_builder),
            text_input_action: None,
            keyboard_type: None,
            enabled: true,
            smart_dashes_type: None,
            smart_quotes_type: None,
            enable_tap_handling: true,
        }
    }

    /// Creates a search anchor that is a [`SearchBar`] when the search view is closed.
    ///
    /// Dart's `SearchAnchor.bar` factory, which has about forty named arguments; here it is
    /// [`SearchAnchorWithSearchBar`], whose setters are those arguments and whose
    /// [`into_widget`](IntoWidget::into_widget) builds the [`SearchAnchor`].
    pub fn bar(
        suggestions_builder: impl Fn(
            &mut App,
            BuildContext,
            Handle<SearchController>,
        ) -> Task<Vec<WidgetRef>>
        + 'static,
    ) -> SearchAnchorWithSearchBar {
        SearchAnchorWithSearchBar::new(suggestions_builder)
    }

    /// Dart `SearchAnchor(key:)`.
    pub fn key(mut self, key: KeyRef) -> SearchAnchor {
        self.key = Some(key);
        self
    }

    /// Dart `SearchAnchor(isFullScreen:)`.
    pub fn is_full_screen(mut self, is_full_screen: bool) -> SearchAnchor {
        self.is_full_screen = Some(is_full_screen);
        self
    }

    /// Dart `SearchAnchor(searchController:)`.
    pub fn search_controller(mut self, search_controller: Ref<SearchController>) -> SearchAnchor {
        self.search_controller = Some(search_controller);
        self
    }

    /// Dart `SearchAnchor(viewBuilder:)`.
    pub fn view_builder(
        mut self,
        view_builder: impl Fn(&mut App, Vec<WidgetRef>) -> WidgetRef + 'static,
    ) -> SearchAnchor {
        self.view_builder = Some(Rc::new(view_builder));
        self
    }

    /// Dart `SearchAnchor(viewLeading:)`.
    pub fn view_leading<K>(mut self, view_leading: impl IntoWidget<K>) -> SearchAnchor {
        self.view_leading = Some(view_leading.into_widget());
        self
    }

    /// Dart `SearchAnchor(viewTrailing:)`.
    pub fn view_trailing(mut self, view_trailing: Vec<WidgetRef>) -> SearchAnchor {
        self.view_trailing = Some(view_trailing);
        self
    }

    /// Dart `SearchAnchor(viewHintText:)`.
    pub fn view_hint_text(mut self, view_hint_text: impl Into<String>) -> SearchAnchor {
        self.view_hint_text = Some(view_hint_text.into());
        self
    }

    /// Dart `SearchAnchor(viewBackgroundColor:)`.
    pub fn view_background_color(mut self, view_background_color: Color) -> SearchAnchor {
        self.view_background_color = Some(view_background_color);
        self
    }

    /// Dart `SearchAnchor(viewElevation:)`.
    pub fn view_elevation(mut self, view_elevation: f64) -> SearchAnchor {
        self.view_elevation = Some(view_elevation);
        self
    }

    /// Dart `SearchAnchor(viewSurfaceTintColor:)`.
    pub fn view_surface_tint_color(mut self, view_surface_tint_color: Color) -> SearchAnchor {
        self.view_surface_tint_color = Some(view_surface_tint_color);
        self
    }

    /// Dart `SearchAnchor(viewSide:)`.
    pub fn view_side(mut self, view_side: impl Into<AnyBorderSide>) -> SearchAnchor {
        self.view_side = Some(view_side.into());
        self
    }

    /// Dart `SearchAnchor(viewShape:)`.
    pub fn view_shape(mut self, view_shape: impl Into<OutlinedBorderRef>) -> SearchAnchor {
        self.view_shape = Some(view_shape.into());
        self
    }

    /// Dart `SearchAnchor(viewBarPadding:)`.
    pub fn view_bar_padding(
        mut self,
        view_bar_padding: impl Into<EdgeInsetsGeometry>,
    ) -> SearchAnchor {
        self.view_bar_padding = Some(view_bar_padding.into());
        self
    }

    /// Dart `SearchAnchor(headerHeight:)`.
    pub fn header_height(mut self, header_height: f64) -> SearchAnchor {
        self.header_height = Some(header_height);
        self
    }

    /// Dart `SearchAnchor(headerTextStyle:)`.
    pub fn header_text_style(mut self, header_text_style: TextStyle) -> SearchAnchor {
        self.header_text_style = Some(header_text_style);
        self
    }

    /// Dart `SearchAnchor(headerHintStyle:)`.
    pub fn header_hint_style(mut self, header_hint_style: TextStyle) -> SearchAnchor {
        self.header_hint_style = Some(header_hint_style);
        self
    }

    /// Dart `SearchAnchor(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> SearchAnchor {
        self.divider_color = Some(divider_color);
        self
    }

    /// Dart `SearchAnchor(viewConstraints:)`.
    pub fn view_constraints(mut self, view_constraints: BoxConstraints) -> SearchAnchor {
        self.view_constraints = Some(view_constraints);
        self
    }

    /// Dart `SearchAnchor(viewPadding:)`.
    pub fn view_padding(mut self, view_padding: impl Into<EdgeInsetsGeometry>) -> SearchAnchor {
        self.view_padding = Some(view_padding.into());
        self
    }

    /// Dart `SearchAnchor(shrinkWrap:)`.
    pub fn shrink_wrap(mut self, shrink_wrap: bool) -> SearchAnchor {
        self.shrink_wrap = Some(shrink_wrap);
        self
    }

    /// Dart `SearchAnchor(textCapitalization:)`.
    pub fn text_capitalization(mut self, text_capitalization: TextCapitalization) -> SearchAnchor {
        self.text_capitalization = Some(text_capitalization);
        self
    }

    /// Dart `SearchAnchor(viewOnChanged:)`.
    pub fn view_on_changed(
        mut self,
        view_on_changed: impl Fn(&mut App, String) + 'static,
    ) -> SearchAnchor {
        self.view_on_changed = Some(Rc::new(view_on_changed));
        self
    }

    /// Dart `SearchAnchor(viewOnSubmitted:)`.
    pub fn view_on_submitted(
        mut self,
        view_on_submitted: impl Fn(&mut App, String) + 'static,
    ) -> SearchAnchor {
        self.view_on_submitted = Some(Rc::new(view_on_submitted));
        self
    }

    /// Dart `SearchAnchor(viewOnClose:)`.
    pub fn view_on_close(mut self, view_on_close: Listener) -> SearchAnchor {
        self.view_on_close = Some(view_on_close);
        self
    }

    /// Dart `SearchAnchor(viewOnOpen:)`.
    pub fn view_on_open(mut self, view_on_open: Listener) -> SearchAnchor {
        self.view_on_open = Some(view_on_open);
        self
    }

    /// Dart `SearchAnchor(textInputAction:)`.
    pub fn text_input_action(mut self, text_input_action: TextInputAction) -> SearchAnchor {
        self.text_input_action = Some(text_input_action);
        self
    }

    /// Dart `SearchAnchor(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> SearchAnchor {
        self.keyboard_type = Some(keyboard_type);
        self
    }

    /// Dart `SearchAnchor(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> SearchAnchor {
        self.enabled = enabled;
        self
    }

    /// Dart `SearchAnchor(smartDashesType:)`.
    pub fn smart_dashes_type(mut self, smart_dashes_type: SmartDashesType) -> SearchAnchor {
        self.smart_dashes_type = Some(smart_dashes_type);
        self
    }

    /// Dart `SearchAnchor(smartQuotesType:)`.
    pub fn smart_quotes_type(mut self, smart_quotes_type: SmartQuotesType) -> SearchAnchor {
        self.smart_quotes_type = Some(smart_quotes_type);
        self
    }

    /// Dart `SearchAnchor(enableTapHandling:)`.
    pub fn enable_tap_handling(mut self, enable_tap_handling: bool) -> SearchAnchor {
        self.enable_tap_handling = enable_tap_handling;
        self
    }
}

impl Debug for SearchAnchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SearchAnchor")
            .field("is_full_screen", &self.is_full_screen)
            .field("view_hint_text", &self.view_hint_text)
            .field("enabled", &self.enabled)
            .field("enable_tap_handling", &self.enable_tap_handling)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SearchAnchor {
    type State = SearchAnchorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SearchAnchorState {
        SearchAnchorState {
            state: StateData::new(),
            screen_size: None,
            anchor_is_visible: true,
            anchor_key: GlobalKey::new(),
            internal_search_controller: None,
            route: None,
        }
    }
}

/// Dart's `_SearchAnchorState`.
pub struct SearchAnchorState {
    state: StateData<SearchAnchor>,
    screen_size: Option<Size>,
    anchor_is_visible: bool,
    anchor_key: GlobalKey,
    internal_search_controller: Option<Ref<SearchController>>,
    route: Option<Ref<SearchViewRoute>>,
}

impl SearchAnchorState {
    /// Dart's `_viewIsOpen`.
    fn view_is_open(self: Handle<Self>, app: &App) -> bool {
        !app.get(self).anchor_is_visible
    }

    /// Dart's `_searchController`.
    fn search_controller(self: Handle<Self>, app: &mut App) -> Handle<SearchController> {
        if let Some(search_controller) = &self.widget(app).search_controller {
            return search_controller.handle();
        }
        if app.get(self).internal_search_controller.is_none() {
            let internal = SearchController::new(app);
            app.get_mut(self).internal_search_controller = Some(internal);
        }
        app.get(self)
            .internal_search_controller
            .as_ref()
            .expect("created above")
            .handle()
    }

    fn open_view(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let navigator = Navigator::of(app, context, false);
        let widget = self.widget(app).clone();
        let search_controller = self.search_controller(app);
        let show_full_screen_view = self.get_show_full_screen_view(app);
        let text_direction = Directionality::of(app, context);
        let captured_themes =
            <dyn InheritedTheme>::capture(app, context, Some(navigator.context(app)));
        let anchor_key = app.get(self).anchor_key.clone();
        let route = SearchViewRoute::new(
            app,
            SearchViewRouteArguments {
                view_on_changed: widget.view_on_changed.clone(),
                view_on_submitted: widget.view_on_submitted.clone(),
                view_on_close: widget.view_on_close.clone(),
                view_on_open: widget.view_on_open.clone(),
                view_leading: widget.view_leading.clone(),
                view_trailing: widget.view_trailing.clone(),
                view_hint_text: widget.view_hint_text.clone(),
                view_background_color: widget.view_background_color,
                view_elevation: widget.view_elevation,
                view_surface_tint_color: widget.view_surface_tint_color,
                view_side: widget.view_side.clone(),
                view_shape: widget.view_shape.clone(),
                view_bar_padding: widget.view_bar_padding,
                view_header_height: widget.header_height,
                view_header_text_style: widget.header_text_style.clone(),
                view_header_hint_style: widget.header_hint_style.clone(),
                divider_color: widget.divider_color,
                view_constraints: widget.view_constraints,
                view_padding: widget.view_padding,
                shrink_wrap: widget.shrink_wrap,
                show_full_screen_view,
                toggle_visibility: Some(Rc::new(move |app: &mut App| self.toggle_visibility(app))),
                text_direction: Some(text_direction),
                view_builder: widget.view_builder.clone(),
                anchor_key,
                search_controller: search_controller.keep(app),
                suggestions_builder: Rc::clone(&widget.suggestions_builder),
                text_capitalization: widget.text_capitalization,
                captured_themes: Rc::new(captured_themes),
                text_input_action: widget.text_input_action,
                keyboard_type: widget.keyboard_type,
                smart_dashes_type: widget.smart_dashes_type,
                smart_quotes_type: widget.smart_quotes_type,
            },
        );
        app.get_mut(self).route = Some(route.clone());
        navigator.push(app, route.into_erased());
    }

    fn close_view(self: Handle<Self>, app: &mut App, selected_text: Option<String>) {
        if let Some(selected_text) = selected_text {
            let search_controller = self.search_controller(app);
            search_controller.set_value(app, TextEditingValue::new().text(selected_text));
        }
        let context = self.context(app);
        Navigator::pop(app, context, None);
    }

    fn toggle_visibility(self: Handle<Self>, app: &mut App) -> bool {
        self.set_state(app, |state| {
            state.anchor_is_visible = !state.anchor_is_visible;
        });
        app.get(self).anchor_is_visible
    }

    fn get_show_full_screen_view(self: Handle<Self>, app: &mut App) -> bool {
        use waft_embedder::TargetPlatform;
        if let Some(is_full_screen) = self.widget(app).is_full_screen {
            return is_full_screen;
        }
        let context = self.context(app);
        match Theme::of(app, context).platform {
            TargetPlatform::IOS | TargetPlatform::Android | TargetPlatform::Fuchsia => true,
            TargetPlatform::MacOS | TargetPlatform::Linux | TargetPlatform::Windows => false,
        }
    }

    fn get_opacity(self: Handle<Self>, app: &App) -> f64 {
        if self.widget(app).enabled {
            return if app.get(self).anchor_is_visible {
                1.0
            } else {
                0.0
            };
        }
        K_DISABLE_SEARCH_BAR_OPACITY
    }
}

impl State for SearchAnchorState {
    type Widget = SearchAnchor;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let search_controller = self.search_controller(app);
        search_controller.attach(app, self);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let updated_screen_size = MediaQuery::of(app, context).size;
        if let Some(screen_size) = app.get(self).screen_size
            && screen_size != updated_screen_size
        {
            let search_controller = self.search_controller(app);
            if search_controller.is_open(app) && !self.get_show_full_screen_view(app) {
                self.close_view(app, None);
            }
        }
        app.get_mut(self).screen_size = Some(updated_screen_size);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SearchAnchor) {
        let widget = self.widget(app);
        let (old, new) = (
            old_widget.search_controller.as_ref().map(Ref::handle),
            widget.search_controller.as_ref().map(Ref::handle),
        );
        if old != new {
            if let Some(old) = old {
                old.detach(app, self);
            }
            let search_controller = self.search_controller(app);
            search_controller.attach(app, self);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(search_controller) = self.widget(app).search_controller.clone() {
            search_controller.handle().detach(app, self);
        }
        if let Some(internal) = app.get(self).internal_search_controller.clone() {
            internal.handle().detach(app, self);
        }
        let using_external_controller = self.widget(app).search_controller.is_some();
        let route = app.get(self).route.clone();
        let route_is_in_navigator = route
            .as_ref()
            .is_some_and(|route| route.handle().navigator(app).is_some());
        if route_is_in_navigator {
            if let Some(route) = route {
                route.handle().dismiss(app, !using_external_controller);
            }
            if using_external_controller
                && let Some(internal) = app.get(self).internal_search_controller.clone()
            {
                ChangeNotifierData::dispose(app, internal.handle());
            }
        } else if let Some(internal) = app.get(self).internal_search_controller.clone() {
            ChangeNotifierData::dispose(app, internal.handle());
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let search_controller = self.search_controller(app);
        let mut child = (widget.builder)(app, context, search_controller);
        if widget.enable_tap_handling {
            child = GestureDetector::new()
                .on_tap(Listener::handle_method(self, SearchAnchorState::open_view))
                .child(child)
                .into_widget();
        }
        let anchor_key = app.get(self).anchor_key.clone();
        AnimatedOpacity::new(self.get_opacity(app), K_ANCHOR_FADE_DURATION)
            .key(anchor_key.into())
            .child(IgnorePointer::new().ignoring(!widget.enabled).child(child))
            .into_widget()
    }
}

/// Dart's `ValueGetter<bool>` the anchor hands its route: toggles the anchor's visibility and
/// answers the new one.
type ToggleVisibility = Rc<dyn Fn(&mut App) -> bool>;

/// The arguments of a [`SearchViewRoute`], Dart's `_SearchViewRoute(..)` named arguments.
pub(crate) struct SearchViewRouteArguments {
    view_on_changed: Option<ValueChanged<String>>,
    view_on_submitted: Option<ValueChanged<String>>,
    view_on_close: Option<Listener>,
    view_on_open: Option<Listener>,
    view_leading: Option<WidgetRef>,
    view_trailing: Option<Vec<WidgetRef>>,
    view_hint_text: Option<String>,
    view_background_color: Option<Color>,
    view_elevation: Option<f64>,
    view_surface_tint_color: Option<Color>,
    view_side: Option<AnyBorderSide>,
    view_shape: Option<OutlinedBorderRef>,
    view_bar_padding: Option<EdgeInsetsGeometry>,
    view_header_height: Option<f64>,
    view_header_text_style: Option<TextStyle>,
    view_header_hint_style: Option<TextStyle>,
    divider_color: Option<Color>,
    view_constraints: Option<BoxConstraints>,
    view_padding: Option<EdgeInsetsGeometry>,
    shrink_wrap: Option<bool>,
    show_full_screen_view: bool,
    toggle_visibility: Option<ToggleVisibility>,
    text_direction: Option<TextDirection>,
    view_builder: Option<ViewBuilder>,
    anchor_key: GlobalKey,
    search_controller: Ref<SearchController>,
    suggestions_builder: SuggestionsBuilder,
    text_capitalization: Option<TextCapitalization>,
    captured_themes: Rc<CapturedThemes>,
    text_input_action: Option<TextInputAction>,
    keyboard_type: Option<TextInputType>,
    smart_dashes_type: Option<SmartDashesType>,
    smart_quotes_type: Option<SmartQuotesType>,
}

/// Dart's `_SearchViewRoute`.
pub(crate) struct SearchViewRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    arguments: SearchViewRouteArguments,
    curved_animation: Option<Ref<CurvedAnimation>>,
    view_fade_on_interval_curve: Option<Ref<CurvedAnimation>>,
    will_dispose_search_controller: bool,
    view_defaults: Option<SearchViewDefaultsM3>,
    view_theme: Option<SearchViewThemeData>,
    rect_tween: Ref<RectTween>,
}

impl SearchViewRoute {
    fn new(app: &mut App, arguments: SearchViewRouteArguments) -> Ref<SearchViewRoute> {
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let rect_tween = RectTween::new(app, None, None);
        app.create(SearchViewRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            arguments,
            curved_animation: None,
            view_fade_on_interval_curve: None,
            will_dispose_search_controller: false,
            view_defaults: None,
            view_theme: None,
            rect_tween,
        })
    }

    fn get_rect(self: Handle<Self>, app: &mut App) -> Option<Rect> {
        let anchor_key = app.get(self).arguments.anchor_key.clone();
        let context = anchor_key.current_context(app)?;
        let search_bar_box = context
            .find_render_object(app)
            .expect("a mounted anchor")
            .as_box()
            .expect("a box");
        let box_size = search_bar_box.size(app);
        let navigator = Navigator::of(app, context, false);
        let ancestor = navigator.context(app).find_render_object(app);
        let box_location = search_bar_box.local_to_global(app, Offset::ZERO, ancestor);
        Some(Rect::from_ltwh(
            box_location.dx(),
            box_location.dy(),
            box_size.width(),
            box_size.height(),
        ))
    }

    fn dismiss(self: Handle<Self>, app: &mut App, dispose_controller: bool) {
        app.get_mut(self).will_dispose_search_controller = dispose_controller;
        if self.as_route().is_active(app) {
            let navigator = self.as_route().navigator(app);
            if let Some(navigator) = navigator {
                navigator.remove_route(app, self.as_route(), None);
            }
        }
    }

    fn update_view_config(self: Handle<Self>, app: &mut App, context: BuildContext) {
        let show_full_screen_view = app.get(self).arguments.show_full_screen_view;
        let view_theme = SearchViewTheme::of(app, context);
        let route = app.get_mut(self);
        route.view_defaults = Some(SearchViewDefaultsM3::new(context, show_full_screen_view));
        route.view_theme = Some(view_theme);
    }

    fn update_tweens(self: Handle<Self>, app: &mut App, context: BuildContext) {
        let navigator_box = Navigator::of(app, context, false)
            .context(app)
            .find_render_object(app)
            .expect("a navigator render object")
            .as_box()
            .expect("a box");
        let screen_size = navigator_box.size(app);
        let anchor_rect = self.get_rect(app).unwrap_or(Rect::ZERO);

        let view_constraints = app.get(self).arguments.view_constraints;
        let theme_constraints = app
            .get(self)
            .view_theme
            .as_ref()
            .and_then(|view_theme| view_theme.constraints);
        let effective_constraints = view_constraints
            .or(theme_constraints)
            .or_else(|| {
                // `viewDefaults.constraints!`.
                let defaults = app
                    .get_mut(self)
                    .view_defaults
                    .take()
                    .expect("set by update_view_config");
                let constraints = defaults.constraints(app);
                app.get_mut(self).view_defaults = Some(defaults);
                constraints
            })
            .expect("viewDefaults.constraints!");
        app.get_mut(&app.get(self).rect_tween.clone()).begin = Some(anchor_rect);

        let view_width = clamp_double(
            anchor_rect.width(),
            effective_constraints.min_width,
            effective_constraints.max_width,
        );
        let view_height = clamp_double(
            screen_size.height() * 2.0 / 3.0,
            effective_constraints.min_height,
            effective_constraints.max_height,
        );

        let show_full_screen_view = app.get(self).arguments.show_full_screen_view;
        let text_direction = app
            .get(self)
            .arguments
            .text_direction
            .unwrap_or(TextDirection::Ltr);
        let end = match text_direction {
            TextDirection::Ltr => {
                let view_left_to_screen_right = screen_size.width() - anchor_rect.left;
                let view_top_to_screen_bottom = screen_size.height() - anchor_rect.top;

                // Make sure the search view doesn't go off the screen. If the search view
                // doesn't fit, move the top-left corner of the view to fit the window.
                // If the window is smaller than the view, then we resize the view to fit the
                // window.
                let mut top_left = anchor_rect.top_left();
                if view_left_to_screen_right < view_width {
                    top_left = Offset::new(
                        screen_size.width() - view_width.min(screen_size.width()),
                        top_left.dy(),
                    );
                }
                if view_top_to_screen_bottom < view_height {
                    top_left = Offset::new(
                        top_left.dx(),
                        screen_size.height() - view_height.min(screen_size.height()),
                    );
                }
                let end_size = Size::new(view_width, view_height);
                if show_full_screen_view {
                    Rect::from_ltwh(0.0, 0.0, screen_size.width(), screen_size.height())
                } else {
                    Rect::from_ltwh(
                        top_left.dx(),
                        top_left.dy(),
                        end_size.width(),
                        end_size.height(),
                    )
                }
            }
            TextDirection::Rtl => {
                let view_right_to_screen_left = anchor_rect.right;
                let view_top_to_screen_bottom = screen_size.height() - anchor_rect.top;

                // Make sure the search view doesn't go off the screen.
                let mut top_left =
                    Offset::new((anchor_rect.right - view_width).max(0.0), anchor_rect.top);
                if view_right_to_screen_left < view_width {
                    top_left = Offset::new(0.0, top_left.dy());
                }
                if view_top_to_screen_bottom < view_height {
                    top_left = Offset::new(
                        top_left.dx(),
                        screen_size.height() - view_height.min(screen_size.height()),
                    );
                }
                let end_size = Size::new(view_width, view_height);
                if show_full_screen_view {
                    Rect::from_ltwh(0.0, 0.0, screen_size.width(), screen_size.height())
                } else {
                    Rect::from_ltwh(
                        top_left.dx(),
                        top_left.dy(),
                        end_size.width(),
                        end_size.height(),
                    )
                }
            }
        };
        let rect_tween = app.get(self).rect_tween.clone();
        app.get_mut(&rect_tween).end = Some(end);
    }
}

impl Route for SearchViewRoute {
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

    fn did_push(self: Handle<Self>, app: &mut App) -> TickerFuture {
        let anchor_key = app.get(self).arguments.anchor_key.clone();
        let context = anchor_key
            .current_context(app)
            .expect("the anchor is mounted");
        self.update_view_config(app, context);
        self.update_tweens(app, context);
        if let Some(toggle_visibility) = app.get(self).arguments.toggle_visibility.clone() {
            toggle_visibility(app);
        }
        if let Some(view_on_open) = app.get(self).arguments.view_on_open.clone() {
            view_on_open.call(app);
        }
        ModalRoute::did_push(self, app)
    }

    fn did_add(self: Handle<Self>, app: &mut App) {
        ModalRoute::did_add(self, app)
    }

    fn did_replace(self: Handle<Self>, app: &mut App, old_route: Option<AnyRoute>) {
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
        let anchor_key = app.get(self).arguments.anchor_key.clone();
        let context = anchor_key
            .current_context(app)
            .expect("the anchor is mounted");
        self.update_tweens(app, context);
        if let Some(toggle_visibility) = app.get(self).arguments.toggle_visibility.clone() {
            toggle_visibility(app);
        }
        if let Some(view_on_close) = app.get(self).arguments.view_on_close.clone() {
            view_on_close.call(app);
        }
        // Unfocus the anchor to prevent the Enter key from triggering unwanted
        // actions (like route pops) when the view closes and focus returns to
        // the anchor's search bar.
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if let Some(context) = anchor_key.current_context(app) {
                    FocusScope::of(app, context, false).unfocus(app, UnfocusDisposition::Scope);
                }
            }),
            None,
        );
        LocalHistoryRoute::did_pop(self, app, result)
    }

    fn did_pop_next(self: Handle<Self>, app: &mut App, next_route: AnyRoute) {
        ModalRoute::did_pop_next(self, app, next_route)
    }

    fn did_change_next(self: Handle<Self>, app: &mut App, next_route: Option<AnyRoute>) {
        ModalRoute::did_change_next(self, app, next_route)
    }

    fn did_change_previous(self: Handle<Self>, app: &mut App, previous_route: Option<AnyRoute>) {
        ModalRoute::did_change_previous(self, app, previous_route)
    }

    fn changed_internal_state(self: Handle<Self>, app: &mut App) {
        ModalRoute::changed_internal_state(self, app)
    }

    fn changed_external_state(self: Handle<Self>, app: &mut App) {
        ModalRoute::changed_external_state(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(curved_animation) = app.get(self).curved_animation.handle() {
            curved_animation.dispose(app);
        }
        if let Some(curve) = app.get(self).view_fade_on_interval_curve.handle() {
            curve.dispose(app);
        }
        if app.get(self).will_dispose_search_controller {
            let search_controller = app.get(self).arguments.search_controller.handle();
            ChangeNotifierData::dispose(app, search_controller);
        }
        TransitionRoute::dispose(self, app)
    }
}

impl OverlayRoute for SearchViewRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for SearchViewRoute {
    waft_widgets::modal_predictive_back_route_overrides!(ModalRoute::pop_gesture_enabled);
}

impl TransitionRoute for SearchViewRoute {
    waft_widgets::modal_transition_route_overrides!();

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::opaque(self, app)
    }

    fn transition_duration(self: Handle<Self>, _app: &mut App) -> Duration {
        K_OPEN_VIEW_DURATION
    }
}

impl LocalHistoryRoute for SearchViewRoute {
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for SearchViewRoute {
    waft_widgets::modal_route_accessors!();

    fn barrier_color(self: Handle<Self>, _app: &App) -> Option<Color> {
        Some(Colors::TRANSPARENT)
    }

    fn barrier_dismissible(self: Handle<Self>, _app: &App) -> bool {
        true
    }

    fn barrier_label(self: Handle<Self>, _app: &App) -> Option<String> {
        Some("Dismiss".to_string())
    }

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::maintain_state(self, app)
    }

    fn build_page(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let _ = (context, secondary_animation);
        let text_direction = app
            .get(self)
            .arguments
            .text_direction
            .unwrap_or(TextDirection::Ltr);
        let listenable = animation.clone();
        Directionality::new(
            text_direction,
            AnimatedBuilder::new(Rc::new(listenable), move |app, context, _child| {
                if app.get(self).curved_animation.is_none() {
                    let curve = Curves::ease_in_out_cubic_emphasized();
                    let curved = CurvedAnimation::new(
                        app,
                        animation.clone(),
                        Rc::clone(&curve),
                        Some(Rc::new(curve.flipped())),
                    );
                    app.get_mut(self).curved_animation = Some(curved);
                }
                let curved_animation = app
                    .get(self)
                    .curved_animation
                    .as_ref()
                    .expect("created above")
                    .clone();
                let rect_tween = app.get(self).rect_tween.clone();
                let view_rect = rect_tween
                    .handle()
                    .lerp(app, curved_animation.handle().value(app))
                    .expect("both ends of the tween are set");
                let show_full_screen_view = app.get(self).arguments.show_full_screen_view;
                let top_padding = if show_full_screen_view {
                    lerp_double(
                        Some(0.0),
                        Some(MediaQuery::padding_of(app, context).top),
                        curved_animation.handle().value(app),
                    )
                    .expect("both ends are given")
                } else {
                    0.0
                };

                if app.get(self).view_fade_on_interval_curve.is_none() {
                    let curve = k_view_fade_on_interval();
                    let curved = CurvedAnimation::new(
                        app,
                        animation.clone(),
                        Rc::clone(&curve),
                        Some(Rc::new(curve.flipped())),
                    );
                    app.get_mut(self).view_fade_on_interval_curve = Some(curved);
                }
                let fade = app
                    .get(self)
                    .view_fade_on_interval_curve
                    .as_ref()
                    .expect("created above")
                    .erase();
                let view_max_width = app
                    .get(&rect_tween)
                    .end
                    .expect("both ends of the tween are set")
                    .width();
                let arguments = &app.get(self).arguments;
                let content = ViewContent {
                    key: None,
                    view_on_changed: arguments.view_on_changed.clone(),
                    view_on_submitted: arguments.view_on_submitted.clone(),
                    view_builder: arguments.view_builder.clone(),
                    view_leading: arguments.view_leading.clone(),
                    view_trailing: arguments.view_trailing.clone(),
                    view_hint_text: arguments.view_hint_text.clone(),
                    view_background_color: arguments.view_background_color,
                    view_elevation: arguments.view_elevation,
                    view_surface_tint_color: arguments.view_surface_tint_color,
                    view_side: arguments.view_side.clone(),
                    view_shape: arguments.view_shape.clone(),
                    view_bar_padding: arguments.view_bar_padding,
                    view_header_height: arguments.view_header_height,
                    view_header_text_style: arguments.view_header_text_style.clone(),
                    view_header_hint_style: arguments.view_header_hint_style.clone(),
                    divider_color: arguments.divider_color,
                    view_constraints: arguments.view_constraints,
                    view_padding: arguments.view_padding,
                    shrink_wrap: arguments.shrink_wrap,
                    text_capitalization: arguments.text_capitalization,
                    show_full_screen_view: arguments.show_full_screen_view,
                    top_padding,
                    animation: curved_animation.erase(),
                    view_max_width,
                    view_rect,
                    search_controller: arguments.search_controller.clone(),
                    suggestions_builder: Rc::clone(&arguments.suggestions_builder),
                    text_input_action: arguments.text_input_action,
                    keyboard_type: arguments.keyboard_type,
                    smart_dashes_type: arguments.smart_dashes_type,
                    smart_quotes_type: arguments.smart_quotes_type,
                };
                let captured_themes = arguments.captured_themes.clone();
                FadeTransition::new(fade)
                    .child(captured_themes.wrap(content.into_widget()))
                    .into_widget()
            }),
        )
        .into_widget()
    }
}

impl PopupRoute for SearchViewRoute {}

/// Dart's `_ViewContent`.
#[derive(Clone)]
pub(crate) struct ViewContent {
    key: Option<KeyRef>,
    view_on_changed: Option<ValueChanged<String>>,
    view_on_submitted: Option<ValueChanged<String>>,
    view_builder: Option<ViewBuilder>,
    view_leading: Option<WidgetRef>,
    view_trailing: Option<Vec<WidgetRef>>,
    view_hint_text: Option<String>,
    view_background_color: Option<Color>,
    view_elevation: Option<f64>,
    view_surface_tint_color: Option<Color>,
    view_side: Option<AnyBorderSide>,
    view_shape: Option<OutlinedBorderRef>,
    view_bar_padding: Option<EdgeInsetsGeometry>,
    view_header_height: Option<f64>,
    view_header_text_style: Option<TextStyle>,
    view_header_hint_style: Option<TextStyle>,
    divider_color: Option<Color>,
    view_constraints: Option<BoxConstraints>,
    view_padding: Option<EdgeInsetsGeometry>,
    shrink_wrap: Option<bool>,
    text_capitalization: Option<TextCapitalization>,
    show_full_screen_view: bool,
    top_padding: f64,
    animation: AnimationRef<f64>,
    view_max_width: f64,
    view_rect: Rect,
    search_controller: Ref<SearchController>,
    suggestions_builder: SuggestionsBuilder,
    text_input_action: Option<TextInputAction>,
    keyboard_type: Option<TextInputType>,
    smart_dashes_type: Option<SmartDashesType>,
    smart_quotes_type: Option<SmartQuotesType>,
}

impl Debug for ViewContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_ViewContent")
            .field("show_full_screen_view", &self.show_full_screen_view)
            .field("view_rect", &self.view_rect)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ViewContent {
    type State = ViewContentState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ViewContentState {
        ViewContentState {
            state: StateData::new(),
            screen_size: None,
            view_rect: self.view_rect,
            view_icons_fade_curve: None,
            view_divider_fade_curve: None,
            view_list_fade_on_interval_curve: None,
            result: Vec::new(),
            search_value: None,
            timer: None,
            suggestions_call_id: 0,
        }
    }
}

/// Dart's `_ViewContentState`.
pub(crate) struct ViewContentState {
    state: StateData<ViewContent>,
    screen_size: Option<Size>,
    view_rect: Rect,
    /// Dart's `late CurvedAnimation`s, created in `init_state` because arena objects need the
    /// [`App`].
    view_icons_fade_curve: Option<Ref<CurvedAnimation>>,
    view_divider_fade_curve: Option<Ref<CurvedAnimation>>,
    view_list_fade_on_interval_curve: Option<Ref<CurvedAnimation>>,
    result: Vec<WidgetRef>,
    search_value: Option<String>,
    timer: Option<Timer>,
    /// Identifies the latest call so that older async results cannot replace newer ones.
    suggestions_call_id: i32,
}

impl ViewContentState {
    fn setup_animations(self: Handle<Self>, app: &mut App) {
        let animation = self.widget(app).animation.clone();
        let icons = CurvedAnimation::new(
            app,
            animation.clone(),
            k_view_icons_fade_on_interval(),
            Some(Rc::new(k_view_icons_fade_on_interval().flipped())),
        );
        let divider = CurvedAnimation::new(
            app,
            animation.clone(),
            k_view_divider_fade_on_interval(),
            Some(Rc::new(k_view_fade_on_interval().flipped())),
        );
        let list = CurvedAnimation::new(
            app,
            animation,
            k_view_list_fade_on_interval(),
            Some(Rc::new(k_view_list_fade_on_interval().flipped())),
        );
        let state = app.get_mut(self);
        state.view_icons_fade_curve = Some(icons);
        state.view_divider_fade_curve = Some(divider);
        state.view_list_fade_on_interval_curve = Some(list);
    }

    fn dispose_animations(self: Handle<Self>, app: &mut App) {
        for curve in [
            app.get(self).view_icons_fade_curve.clone(),
            app.get(self).view_divider_fade_curve.clone(),
            app.get(self).view_list_fade_on_interval_curve.clone(),
        ]
        .into_iter()
        .flatten()
        {
            curve.handle().dispose(app);
        }
    }

    fn controller(self: Handle<Self>, app: &App) -> Handle<SearchController> {
        self.widget(app).search_controller.handle()
    }

    /// Dart's `updateSuggestions`.
    fn update_suggestions(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        let text = controller.text_value(app).to_string();
        if app.get(self).search_value.as_deref() != Some(text.as_str()) {
            app.get_mut(self).search_value = Some(text);
            drop(self.build_suggestions(app));
        }
    }

    /// Dart's `_buildSuggestions`.
    fn build_suggestions(self: Handle<Self>, app: &mut App) -> Task<()> {
        let call_id = {
            let state = app.get_mut(self);
            state.suggestions_call_id += 1;
            state.suggestions_call_id
        };
        let context = self.context(app);
        let controller = self.controller(app);
        let suggestions_builder = Rc::clone(&self.widget(app).suggestions_builder);
        let suggestions = suggestions_builder(app, context, controller);
        app.spawn(async move |cx| {
            let suggestions = suggestions.await;
            cx.update(|app| {
                if !self.mounted(app) || call_id != app.get(self).suggestions_call_id {
                    return;
                }
                self.set_state(app, |state| {
                    state.result = suggestions;
                });
            });
        })
    }

    /// The body of the timer `did_change_dependencies` starts: Dart's closure, which clears the
    /// timer once the suggestions are built.
    fn run_suggestions_timer(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        app.get_mut(self).search_value = Some(controller.text_value(app).to_string());
        let built = self.build_suggestions(app);
        drop(app.spawn(async move |cx| {
            built.await;
            cx.update(|app| {
                // `dispose` has cancelled and cleared the timer of a state no longer mounted.
                if !self.mounted(app) {
                    return;
                }
                if let Some(timer) = app.get_mut(self).timer.take() {
                    timer.cancel(app);
                }
            });
        }));
    }
}

impl State for ViewContentState {
    type Widget = ViewContent;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let view_rect = self.widget(app).view_rect;
        app.get_mut(self).view_rect = view_rect;
        let controller = self.controller(app);
        controller.add_listener(
            app,
            Listener::handle_method(self, ViewContentState::update_suggestions),
        );
        self.setup_animations(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ViewContent) {
        let view_rect = self.widget(app).view_rect;
        if view_rect != old_widget.view_rect {
            self.set_state(app, |state| {
                state.view_rect = view_rect;
            });
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let updated_screen_size = MediaQuery::of(app, context).size;

        if app.get(self).screen_size != Some(updated_screen_size) {
            app.get_mut(self).screen_size = Some(updated_screen_size);
            if self.widget(app).show_full_screen_view {
                app.get_mut(self).view_rect = Rect::from_ltwh(
                    0.0,
                    0.0,
                    updated_screen_size.width(),
                    updated_screen_size.height(),
                );
            }
        }

        let controller = self.controller(app);
        if app.get(self).search_value.as_deref() != Some(controller.text_value(app)) {
            if let Some(timer) = app.get_mut(self).timer.take() {
                timer.cancel(app);
            }
            let timer = Timer::new(
                app,
                Duration::ZERO,
                Listener::handle_method(self, ViewContentState::run_suggestions_timer),
            );
            app.get_mut(self).timer = Some(timer);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.remove_listener(
            app,
            &Listener::handle_method(self, ViewContentState::update_suggestions),
        );
        self.dispose_animations(app);
        if let Some(timer) = app.get_mut(self).timer.take() {
            timer.cancel(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let controller = self.controller(app);
        let default_leading = BackButton::new()
            .style(ButtonStyle::new().tap_target_size(MaterialTapTargetSize::ShrinkWrap))
            .on_pressed(Listener::new(move |app: &mut App| {
                Navigator::pop(app, context, None);
            }))
            .into_widget();

        let mut default_trailing = Vec::new();
        if !controller.text_value(app).is_empty() {
            default_trailing.push(
                IconButton::new(
                    Some(Listener::new(move |app: &mut App| {
                        controller.clear(app);
                    })),
                    Icon::new(Some(Icons::close())),
                )
                .tooltip(<dyn MaterialLocalizations>::of(app, context).clear_button_tooltip())
                .into_widget(),
            );
        }

        let view_defaults = SearchViewDefaultsM3::new(context, widget.show_full_screen_view);
        let view_theme = SearchViewTheme::of(app, context);
        let divider_theme = DividerTheme::of(app, context);

        let effective_background_color = widget
            .view_background_color
            .or(view_theme.background_color)
            .or_else(|| view_defaults.background_color(app))
            .expect("viewDefaults.backgroundColor!");
        let effective_surface_tint = widget
            .view_surface_tint_color
            .or(view_theme.surface_tint_color)
            .or_else(|| view_defaults.surface_tint_color(app))
            .expect("viewDefaults.surfaceTintColor!");
        let effective_elevation = widget
            .view_elevation
            .or(view_theme.elevation)
            .or_else(|| view_defaults.elevation(app))
            .expect("viewDefaults.elevation!");
        let effective_side = widget
            .view_side
            .clone()
            .or_else(|| view_theme.side.clone())
            .or_else(|| view_defaults.side(app));
        let mut effective_shape = widget
            .view_shape
            .clone()
            .or_else(|| view_theme.shape.clone())
            .or_else(|| view_defaults.shape(app))
            .expect("viewDefaults.shape!");
        if let Some(effective_side) = &effective_side {
            effective_shape = effective_shape.copy_with(Some(**effective_side));
        }
        let effective_divider_color = widget
            .divider_color
            .or(view_theme.divider_color)
            .or(divider_theme.color)
            .or_else(|| view_defaults.divider_color(app))
            .expect("viewDefaults.dividerColor!");
        let effective_header_height = widget.view_header_height.or(view_theme.header_height);
        let header_constraints =
            effective_header_height.map(|height| BoxConstraints::tight_for(None, Some(height)));
        let effective_text_style = widget
            .view_header_text_style
            .clone()
            .or_else(|| view_theme.header_text_style.clone())
            .or_else(|| view_defaults.header_text_style(app));
        let effective_hint_style = widget
            .view_header_hint_style
            .clone()
            .or_else(|| view_theme.header_hint_style.clone())
            .or_else(|| widget.view_header_text_style.clone())
            .or_else(|| view_theme.header_text_style.clone())
            .or_else(|| view_defaults.header_hint_style(app));
        let effective_padding = widget
            .view_padding
            .or(view_theme.padding)
            .or_else(|| view_defaults.padding(app));
        let effective_bar_padding = widget
            .view_bar_padding
            .or(view_theme.bar_padding)
            .or_else(|| view_defaults.bar_padding(app));

        let effective_constraints = widget
            .view_constraints
            .or(view_theme.constraints)
            .or_else(|| view_defaults.constraints(app))
            .expect("viewDefaults.constraints!");
        let view_rect = app.get(self).view_rect;
        let min_width = effective_constraints.min_width.min(view_rect.width());
        let min_height = effective_constraints.min_height.min(view_rect.height());

        let effective_shrink_wrap = widget
            .shrink_wrap
            .or(view_theme.shrink_wrap)
            .or_else(|| view_defaults.shrink_wrap(app))
            .expect("viewDefaults.shrinkWrap!");

        let view_divider = DividerTheme::new(
            divider_theme.copy_with().color(effective_divider_color),
            Divider::new().height(1.0),
        )
        .into_widget();

        let view_icons_fade_curve: AnimationRef<f64> = app
            .get(self)
            .view_icons_fade_curve
            .as_ref()
            .expect("created in init_state")
            .erase();
        let view_divider_fade_curve: AnimationRef<f64> = app
            .get(self)
            .view_divider_fade_curve
            .as_ref()
            .expect("created in init_state")
            .erase();
        let view_list_fade_on_interval_curve: AnimationRef<f64> = app
            .get(self)
            .view_list_fade_on_interval_curve
            .as_ref()
            .expect("created in init_state")
            .erase();
        let result = app.get(self).result.clone();
        let show_full_screen_view = widget.show_full_screen_view;

        LayoutBuilder::new(move |app, context, constraints| {
            let view_icons_fade_curve = view_icons_fade_curve.clone();
            let mut search_bar = SearchBar::new()
                .auto_focus(true)
                .padding(Rc::new(WidgetStatePropertyAll::new(effective_bar_padding)))
                .leading(
                    widget
                        .view_leading
                        .clone()
                        .unwrap_or_else(|| default_leading.clone()),
                )
                .trailing(
                    widget
                        .view_trailing
                        .clone()
                        .unwrap_or_else(|| default_trailing.clone()),
                )
                .background_color(Rc::new(WidgetStatePropertyAll::new(Some(
                    Colors::TRANSPARENT,
                ))))
                .overlay_color(Rc::new(WidgetStatePropertyAll::new(Some(
                    Colors::TRANSPARENT,
                ))))
                .elevation(Rc::new(WidgetStatePropertyAll::new(Some(0.0))))
                .text_style(Rc::new(WidgetStatePropertyAll::new(
                    effective_text_style.clone(),
                )))
                .hint_style(Rc::new(WidgetStatePropertyAll::new(
                    effective_hint_style.clone(),
                )))
                .controller(controller.as_controller().keep(app))
                .on_changed({
                    let view_on_changed = widget.view_on_changed.clone();
                    move |app, value| {
                        if let Some(view_on_changed) = &view_on_changed {
                            view_on_changed(app, value);
                        }
                        self.update_suggestions(app);
                    }
                });
            search_bar.constraints = header_constraints.or(if show_full_screen_view {
                Some(BoxConstraints {
                    min_height: SearchViewDefaultsM3::FULL_SCREEN_BAR_HEIGHT,
                    ..BoxConstraints::new()
                })
            } else {
                None
            });
            search_bar.hint_text = widget.view_hint_text.clone();
            search_bar.on_submitted = widget.view_on_submitted.clone();
            search_bar.text_capitalization = widget.text_capitalization;
            search_bar.text_input_action = widget.text_input_action;
            search_bar.keyboard_type = widget.keyboard_type;
            search_bar.smart_dashes_type = widget.smart_dashes_type;
            search_bar.smart_quotes_type = widget.smart_quotes_type;
            let header_bar = SafeArea::new(search_bar).top(false).bottom(false);

            let mut children = vec![
                Padding::new(EdgeInsetsGeometry::from(EdgeInsets::only(
                    0.0,
                    widget.top_padding,
                    0.0,
                    0.0,
                )))
                .child(header_bar)
                .into_widget(),
            ];
            if !effective_shrink_wrap
                || min_height > 0.0
                || show_full_screen_view
                || !result.is_empty()
            {
                children.push(
                    FadeTransition::new(view_divider_fade_curve.clone())
                        .child(view_divider.clone())
                        .into_widget(),
                );
                let view_insets_bottom = MediaQuery::view_insets_of(app, context).bottom;
                let list = match &widget.view_builder {
                    None => MediaQuery::remove_padding(
                        app,
                        context,
                        false,
                        true,
                        false,
                        false,
                        ListView::new()
                            .padding(EdgeInsetsGeometry::from(EdgeInsets::only(
                                0.0,
                                0.0,
                                0.0,
                                view_insets_bottom,
                            )))
                            .shrink_wrap(effective_shrink_wrap)
                            .children(result.clone()),
                    )
                    .into_widget(),
                    Some(view_builder) => view_builder(app, result.clone()),
                };
                children.push(
                    Flexible::new(
                        FadeTransition::new(view_list_fade_on_interval_curve.clone()).child(list),
                    )
                    .fit(if effective_shrink_wrap && !show_full_screen_view {
                        FlexFit::Loose
                    } else {
                        FlexFit::Tight
                    })
                    .into_widget(),
                );
            }

            Align::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .child(
                    Transform::translate(view_rect.top_left()).child(
                        ConstrainedBox::new(BoxConstraints {
                            min_width,
                            max_width: if show_full_screen_view {
                                constraints.max_width.max(min_width)
                            } else {
                                view_rect.width()
                            },
                            min_height,
                            max_height: if show_full_screen_view {
                                constraints.max_height.max(min_height)
                            } else {
                                view_rect.height()
                            },
                        })
                        .child(
                            Padding::new(if show_full_screen_view {
                                EdgeInsetsGeometry::from(EdgeInsets::ZERO)
                            } else {
                                effective_padding
                                    .unwrap_or(EdgeInsetsGeometry::from(EdgeInsets::ZERO))
                            })
                            .child(
                                Material::new()
                                    .clip_behavior(Clip::AntiAlias)
                                    .shape(effective_shape.clone())
                                    .color(effective_background_color)
                                    .surface_tint_color(effective_surface_tint)
                                    .elevation(effective_elevation)
                                    .child(
                                        OverflowBox::new()
                                            .alignment(AlignmentGeometry::TOP_LEFT)
                                            .max_width(if show_full_screen_view {
                                                constraints.max_width
                                            } else {
                                                widget.view_max_width.min(constraints.max_width)
                                            })
                                            .min_width(0.0)
                                            .fit(OverflowBoxFit::DeferToChild)
                                            .child(
                                                FadeTransition::new(view_icons_fade_curve).child(
                                                    Column::new()
                                                        .main_axis_size(MainAxisSize::Min)
                                                        .cross_axis_alignment(
                                                            CrossAxisAlignment::Stretch,
                                                        )
                                                        .children(children),
                                                ),
                                            ),
                                    ),
                            ),
                        ),
                    ),
                )
                .into_widget()
        })
        .into_widget()
    }
}

/// Dart's `_SearchAnchorWithSearchBar`, the object `SearchAnchor.bar(..)` makes: a
/// [`SearchAnchor`] whose anchor is a [`SearchBar`] that opens the search view itself.
///
/// The setters are the named arguments of `SearchAnchor.bar`. [`into_widget`](IntoWidget)
/// builds the [`SearchAnchor`] (with `enable_tap_handling` false) the factory's `super(..)`
/// call makes, whose `builder` makes the [`SearchBar`].
pub struct SearchAnchorWithSearchBar {
    pub bar_leading: Option<WidgetRef>,
    pub bar_trailing: Option<Vec<WidgetRef>>,
    pub bar_hint_text: Option<String>,
    pub on_tap: Option<Listener>,
    pub bar_elevation: Option<WidgetStatePropertyRef<Option<f64>>>,
    pub bar_background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    pub bar_overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    pub bar_side: Option<WidgetStatePropertyRef<Option<BorderSide>>>,
    pub bar_shape: Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>>,
    pub bar_padding: Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>>,
    pub bar_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,
    pub bar_hint_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,
    pub constraints: Option<BoxConstraints>,
    pub scroll_padding: EdgeInsets,
    pub context_menu_builder: Option<EditableTextContextMenuBuilder>,
    pub bar_focus_node: Option<FocusNodeRef>,
    pub on_changed: Option<ValueChanged<String>>,
    pub on_submitted: Option<ValueChanged<String>>,
    pub on_close: Option<Listener>,
    pub on_open: Option<Listener>,
    pub view_hint_text: Option<String>,
    pub view_header_height: Option<f64>,
    pub view_header_text_style: Option<TextStyle>,
    pub view_header_hint_style: Option<TextStyle>,
    pub anchor: SearchAnchor,
}

impl SearchAnchorWithSearchBar {
    fn new(
        suggestions_builder: impl Fn(
            &mut App,
            BuildContext,
            Handle<SearchController>,
        ) -> Task<Vec<WidgetRef>>
        + 'static,
    ) -> SearchAnchorWithSearchBar {
        SearchAnchorWithSearchBar {
            bar_leading: None,
            bar_trailing: None,
            bar_hint_text: None,
            on_tap: None,
            bar_elevation: None,
            bar_background_color: None,
            bar_overlay_color: None,
            bar_side: None,
            bar_shape: None,
            bar_padding: None,
            bar_text_style: None,
            bar_hint_style: None,
            constraints: None,
            scroll_padding: EdgeInsets::all(20.0),
            context_menu_builder: Some(Rc::new(SearchBar::default_context_menu_builder)),
            bar_focus_node: None,
            on_changed: None,
            on_submitted: None,
            on_close: None,
            on_open: None,
            view_hint_text: None,
            view_header_height: None,
            view_header_text_style: None,
            view_header_hint_style: None,
            anchor: SearchAnchor::new(
                |_app, _context, _controller| unreachable!("replaced in into_widget"),
                suggestions_builder,
            ),
        }
    }

    /// Dart `SearchAnchor.bar(key:)`.
    pub fn key(mut self, key: KeyRef) -> SearchAnchorWithSearchBar {
        self.anchor.key = Some(key);
        self
    }

    /// Dart `SearchAnchor.bar(barLeading:)`.
    pub fn bar_leading<K>(mut self, bar_leading: impl IntoWidget<K>) -> SearchAnchorWithSearchBar {
        self.bar_leading = Some(bar_leading.into_widget());
        self
    }

    /// Dart `SearchAnchor.bar(barTrailing:)`.
    pub fn bar_trailing(mut self, bar_trailing: Vec<WidgetRef>) -> SearchAnchorWithSearchBar {
        self.bar_trailing = Some(bar_trailing);
        self
    }

    /// Dart `SearchAnchor.bar(barHintText:)`.
    pub fn bar_hint_text(mut self, bar_hint_text: impl Into<String>) -> SearchAnchorWithSearchBar {
        self.bar_hint_text = Some(bar_hint_text.into());
        self
    }

    /// Dart `SearchAnchor.bar(onTap:)`.
    pub fn on_tap(mut self, on_tap: Listener) -> SearchAnchorWithSearchBar {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `SearchAnchor.bar(onSubmitted:)`.
    pub fn on_submitted(
        mut self,
        on_submitted: impl Fn(&mut App, String) + 'static,
    ) -> SearchAnchorWithSearchBar {
        self.on_submitted = Some(Rc::new(on_submitted));
        self
    }

    /// Dart `SearchAnchor.bar(onChanged:)`.
    pub fn on_changed(
        mut self,
        on_changed: impl Fn(&mut App, String) + 'static,
    ) -> SearchAnchorWithSearchBar {
        self.on_changed = Some(Rc::new(on_changed));
        self
    }

    /// Dart `SearchAnchor.bar(onClose:)`.
    pub fn on_close(mut self, on_close: Listener) -> SearchAnchorWithSearchBar {
        self.on_close = Some(on_close);
        self
    }

    /// Dart `SearchAnchor.bar(onOpen:)`.
    pub fn on_open(mut self, on_open: Listener) -> SearchAnchorWithSearchBar {
        self.on_open = Some(on_open);
        self
    }

    /// Dart `SearchAnchor.bar(barElevation:)`.
    pub fn bar_elevation(
        mut self,
        bar_elevation: WidgetStatePropertyRef<Option<f64>>,
    ) -> SearchAnchorWithSearchBar {
        self.bar_elevation = Some(bar_elevation);
        self
    }

    /// Dart `SearchAnchor.bar(barBackgroundColor:)`.
    pub fn bar_background_color(
        mut self,
        bar_background_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SearchAnchorWithSearchBar {
        self.bar_background_color = Some(bar_background_color);
        self
    }

    /// Dart `SearchAnchor.bar(barOverlayColor:)`.
    pub fn bar_overlay_color(
        mut self,
        bar_overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SearchAnchorWithSearchBar {
        self.bar_overlay_color = Some(bar_overlay_color);
        self
    }

    /// Dart `SearchAnchor.bar(barSide:)`.
    pub fn bar_side(
        mut self,
        bar_side: WidgetStatePropertyRef<Option<BorderSide>>,
    ) -> SearchAnchorWithSearchBar {
        self.bar_side = Some(bar_side);
        self
    }

    /// Dart `SearchAnchor.bar(barShape:)`.
    pub fn bar_shape(
        mut self,
        bar_shape: WidgetStatePropertyRef<Option<OutlinedBorderRef>>,
    ) -> SearchAnchorWithSearchBar {
        self.bar_shape = Some(bar_shape);
        self
    }

    /// Dart `SearchAnchor.bar(barPadding:)`.
    pub fn bar_padding(
        mut self,
        bar_padding: WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>,
    ) -> SearchAnchorWithSearchBar {
        self.bar_padding = Some(bar_padding);
        self
    }

    /// Dart `SearchAnchor.bar(viewBarPadding:)`.
    pub fn view_bar_padding(
        mut self,
        view_bar_padding: impl Into<EdgeInsetsGeometry>,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.view_bar_padding = Some(view_bar_padding.into());
        self
    }

    /// Dart `SearchAnchor.bar(barTextStyle:)`.
    pub fn bar_text_style(
        mut self,
        bar_text_style: WidgetStatePropertyRef<Option<TextStyle>>,
    ) -> SearchAnchorWithSearchBar {
        self.bar_text_style = Some(bar_text_style);
        self
    }

    /// Dart `SearchAnchor.bar(barHintStyle:)`.
    pub fn bar_hint_style(
        mut self,
        bar_hint_style: WidgetStatePropertyRef<Option<TextStyle>>,
    ) -> SearchAnchorWithSearchBar {
        self.bar_hint_style = Some(bar_hint_style);
        self
    }

    /// Dart `SearchAnchor.bar(viewBuilder:)`.
    pub fn view_builder(
        mut self,
        view_builder: impl Fn(&mut App, Vec<WidgetRef>) -> WidgetRef + 'static,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.view_builder = Some(Rc::new(view_builder));
        self
    }

    /// Dart `SearchAnchor.bar(viewLeading:)`.
    pub fn view_leading<K>(
        mut self,
        view_leading: impl IntoWidget<K>,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.view_leading = Some(view_leading.into_widget());
        self
    }

    /// Dart `SearchAnchor.bar(viewTrailing:)`.
    pub fn view_trailing(mut self, view_trailing: Vec<WidgetRef>) -> SearchAnchorWithSearchBar {
        self.anchor.view_trailing = Some(view_trailing);
        self
    }

    /// Dart `SearchAnchor.bar(viewHintText:)`.
    pub fn view_hint_text(
        mut self,
        view_hint_text: impl Into<String>,
    ) -> SearchAnchorWithSearchBar {
        self.view_hint_text = Some(view_hint_text.into());
        self
    }

    /// Dart `SearchAnchor.bar(viewBackgroundColor:)`.
    pub fn view_background_color(
        mut self,
        view_background_color: Color,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.view_background_color = Some(view_background_color);
        self
    }

    /// Dart `SearchAnchor.bar(viewElevation:)`.
    pub fn view_elevation(mut self, view_elevation: f64) -> SearchAnchorWithSearchBar {
        self.anchor.view_elevation = Some(view_elevation);
        self
    }

    /// Dart `SearchAnchor.bar(viewSide:)`.
    pub fn view_side(mut self, view_side: impl Into<AnyBorderSide>) -> SearchAnchorWithSearchBar {
        self.anchor.view_side = Some(view_side.into());
        self
    }

    /// Dart `SearchAnchor.bar(viewShape:)`.
    pub fn view_shape(
        mut self,
        view_shape: impl Into<OutlinedBorderRef>,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.view_shape = Some(view_shape.into());
        self
    }

    /// Dart `SearchAnchor.bar(viewHeaderHeight:)`.
    pub fn view_header_height(mut self, view_header_height: f64) -> SearchAnchorWithSearchBar {
        self.view_header_height = Some(view_header_height);
        self
    }

    /// Dart `SearchAnchor.bar(viewHeaderTextStyle:)`.
    pub fn view_header_text_style(
        mut self,
        view_header_text_style: TextStyle,
    ) -> SearchAnchorWithSearchBar {
        self.view_header_text_style = Some(view_header_text_style);
        self
    }

    /// Dart `SearchAnchor.bar(viewHeaderHintStyle:)`.
    pub fn view_header_hint_style(
        mut self,
        view_header_hint_style: TextStyle,
    ) -> SearchAnchorWithSearchBar {
        self.view_header_hint_style = Some(view_header_hint_style);
        self
    }

    /// Dart `SearchAnchor.bar(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> SearchAnchorWithSearchBar {
        self.anchor.divider_color = Some(divider_color);
        self
    }

    /// Dart `SearchAnchor.bar(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> SearchAnchorWithSearchBar {
        self.constraints = Some(constraints);
        self
    }

    /// Dart `SearchAnchor.bar(viewConstraints:)`.
    pub fn view_constraints(
        mut self,
        view_constraints: BoxConstraints,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.view_constraints = Some(view_constraints);
        self
    }

    /// Dart `SearchAnchor.bar(viewPadding:)`.
    pub fn view_padding(
        mut self,
        view_padding: impl Into<EdgeInsetsGeometry>,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.view_padding = Some(view_padding.into());
        self
    }

    /// Dart `SearchAnchor.bar(shrinkWrap:)`.
    pub fn shrink_wrap(mut self, shrink_wrap: bool) -> SearchAnchorWithSearchBar {
        self.anchor.shrink_wrap = Some(shrink_wrap);
        self
    }

    /// Dart `SearchAnchor.bar(isFullScreen:)`.
    pub fn is_full_screen(mut self, is_full_screen: bool) -> SearchAnchorWithSearchBar {
        self.anchor.is_full_screen = Some(is_full_screen);
        self
    }

    /// Dart `SearchAnchor.bar(searchController:)`.
    pub fn search_controller(
        mut self,
        search_controller: Ref<SearchController>,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.search_controller = Some(search_controller);
        self
    }

    /// Dart `SearchAnchor.bar(textCapitalization:)`.
    pub fn text_capitalization(
        mut self,
        text_capitalization: TextCapitalization,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.text_capitalization = Some(text_capitalization);
        self
    }

    /// Dart `SearchAnchor.bar(textInputAction:)`.
    pub fn text_input_action(
        mut self,
        text_input_action: TextInputAction,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.text_input_action = Some(text_input_action);
        self
    }

    /// Dart `SearchAnchor.bar(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> SearchAnchorWithSearchBar {
        self.anchor.keyboard_type = Some(keyboard_type);
        self
    }

    /// Dart `SearchAnchor.bar(scrollPadding:)`.
    pub fn scroll_padding(mut self, scroll_padding: EdgeInsets) -> SearchAnchorWithSearchBar {
        self.scroll_padding = scroll_padding;
        self
    }

    /// Dart `SearchAnchor.bar(contextMenuBuilder:)`.
    pub fn context_menu_builder(
        mut self,
        context_menu_builder: EditableTextContextMenuBuilder,
    ) -> SearchAnchorWithSearchBar {
        self.context_menu_builder = Some(context_menu_builder);
        self
    }

    /// Dart `SearchAnchor.bar(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> SearchAnchorWithSearchBar {
        self.anchor.enabled = enabled;
        self
    }

    /// Dart `SearchAnchor.bar(smartDashesType:)`.
    pub fn smart_dashes_type(
        mut self,
        smart_dashes_type: SmartDashesType,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.smart_dashes_type = Some(smart_dashes_type);
        self
    }

    /// Dart `SearchAnchor.bar(smartQuotesType:)`.
    pub fn smart_quotes_type(
        mut self,
        smart_quotes_type: SmartQuotesType,
    ) -> SearchAnchorWithSearchBar {
        self.anchor.smart_quotes_type = Some(smart_quotes_type);
        self
    }

    /// Dart `SearchAnchor.bar(barFocusNode:)`.
    pub fn bar_focus_node(mut self, bar_focus_node: FocusNodeRef) -> SearchAnchorWithSearchBar {
        self.bar_focus_node = Some(bar_focus_node);
        self
    }

    /// The [`SearchAnchor`] the factory's `super(..)` call makes.
    pub fn into_search_anchor(self) -> SearchAnchor {
        let SearchAnchorWithSearchBar {
            bar_leading,
            bar_trailing,
            bar_hint_text,
            on_tap,
            bar_elevation,
            bar_background_color,
            bar_overlay_color,
            bar_side,
            bar_shape,
            bar_padding,
            bar_text_style,
            bar_hint_style,
            constraints,
            scroll_padding,
            context_menu_builder,
            bar_focus_node,
            on_changed,
            on_submitted,
            on_close,
            on_open,
            view_hint_text,
            view_header_height,
            view_header_text_style,
            view_header_hint_style,
            mut anchor,
        } = self;
        anchor.enable_tap_handling = false;
        anchor.view_hint_text = view_hint_text.or_else(|| bar_hint_text.clone());
        anchor.header_height = view_header_height;
        anchor.header_text_style = view_header_text_style;
        anchor.header_hint_style = view_header_hint_style;
        anchor.view_on_submitted = on_submitted.clone();
        anchor.view_on_changed = on_changed;
        anchor.view_on_close = on_close;
        anchor.view_on_open = on_open;
        let text_capitalization = anchor.text_capitalization;
        let text_input_action = anchor.text_input_action;
        let keyboard_type = anchor.keyboard_type;
        let smart_dashes_type = anchor.smart_dashes_type;
        let smart_quotes_type = anchor.smart_quotes_type;
        anchor.builder = Rc::new(move |app, context, controller| {
            let mut search_bar = SearchBar::new()
                .controller(controller.as_controller().keep(app))
                .on_tap(Listener::new({
                    let on_tap = on_tap.clone();
                    move |app: &mut App| {
                        controller.open_view(app);
                        if let Some(on_tap) = &on_tap {
                            on_tap.call(app);
                        }
                    }
                }))
                .on_changed(move |app, _value| {
                    controller.open_view(app);
                })
                .padding(
                    bar_padding
                        .clone()
                        .or_else(|| SearchBarTheme::of(app, context).padding)
                        .unwrap_or_else(|| {
                            Rc::new(WidgetStatePropertyAll::new(Some(EdgeInsetsGeometry::from(
                                EdgeInsets::symmetric(0.0, 16.0),
                            ))))
                        }),
                )
                .leading(
                    bar_leading
                        .clone()
                        .unwrap_or_else(|| Icon::new(Some(Icons::search())).into_widget()),
                )
                .scroll_padding(scroll_padding);
            search_bar.constraints = constraints;
            search_bar.focus_node = bar_focus_node.clone();
            search_bar.on_submitted = on_submitted.clone();
            search_bar.hint_text = bar_hint_text.clone();
            search_bar.hint_style = bar_hint_style.clone();
            search_bar.text_style = bar_text_style.clone();
            search_bar.elevation = bar_elevation.clone();
            search_bar.background_color = bar_background_color.clone();
            search_bar.overlay_color = bar_overlay_color.clone();
            search_bar.side = bar_side.clone();
            search_bar.shape = bar_shape.clone();
            search_bar.trailing = bar_trailing.clone();
            search_bar.text_capitalization = text_capitalization;
            search_bar.text_input_action = text_input_action;
            search_bar.keyboard_type = keyboard_type;
            search_bar.smart_dashes_type = smart_dashes_type;
            search_bar.smart_quotes_type = smart_quotes_type;
            search_bar.context_menu_builder = context_menu_builder.clone();
            search_bar.into_widget()
        });
        anchor
    }
}

impl<K> IntoWidget<K> for SearchAnchorWithSearchBar {
    fn into_widget(self) -> WidgetRef {
        self.into_search_anchor().into_widget()
    }
}

/// A controller to manage a search view created by [`SearchAnchor`].
///
/// A [`SearchController`] is used to control a menu after it has been created, with methods
/// such as [`open_view`](Self::open_view) and [`close_view`](Self::close_view). It can also
/// be used to read the text the user has typed, and to set that text, as any other
/// [`TextEditingControllerLeaf`].
pub struct SearchController {
    change_notifier: ChangeNotifierData,
    text_editing_controller: TextEditingControllerData,
    /// The anchor that this controller controls.
    ///
    /// This is set automatically when a [`SearchController`] is given to the anchor it
    /// controls.
    anchor: Option<Handle<SearchAnchorState>>,
}

impl SearchController {
    /// Creates a controller for a [`SearchAnchor`].
    pub fn new(app: &mut App) -> Ref<SearchController> {
        app.create(SearchController {
            change_notifier: ChangeNotifierData::new(),
            text_editing_controller: TextEditingControllerData::new(TextEditingValue::new()),
            anchor: None,
        })
    }

    fn attach(self: Handle<Self>, app: &mut App, anchor: Handle<SearchAnchorState>) {
        app.get_mut(self).anchor = Some(anchor);
    }

    fn detach(self: Handle<Self>, app: &mut App, anchor: Handle<SearchAnchorState>) {
        if app.get(self).anchor == Some(anchor) {
            app.get_mut(self).anchor = None;
        }
    }

    /// Whether this controller has been attached to a [`SearchAnchor`].
    pub fn is_attached(self: Handle<Self>, app: &App) -> bool {
        app.get(self).anchor.is_some()
    }

    /// Whether this controller's search view is open.
    ///
    /// The controller must be attached to a [`SearchAnchor`].
    pub fn is_open(self: Handle<Self>, app: &App) -> bool {
        let anchor = app.get(self).anchor;
        assert!(
            anchor.is_some(),
            "the controller is not attached to an anchor"
        );
        anchor.expect("attached").view_is_open(app)
    }

    /// Opens the search view.
    ///
    /// The controller must be attached to a [`SearchAnchor`].
    pub fn open_view(self: Handle<Self>, app: &mut App) {
        let anchor = app.get(self).anchor;
        assert!(
            anchor.is_some(),
            "the controller is not attached to an anchor"
        );
        anchor.expect("attached").open_view(app);
    }

    /// Close the search view that was opened by this controller.
    ///
    /// If `selected_text` is given, it is the new text of the search controller, which a
    /// search view writes when the user selects a suggestion.
    pub fn close_view(self: Handle<Self>, app: &mut App, selected_text: Option<String>) {
        let anchor = app.get(self).anchor;
        assert!(
            anchor.is_some(),
            "the controller is not attached to an anchor"
        );
        anchor.expect("attached").close_view(app, selected_text);
    }
}

impl Debug for SearchController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SearchController")
            .field("anchor", &self.anchor)
            .finish_non_exhaustive()
    }
}

impl ChangeNotifier for SearchController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl TextEditingControllerLeaf for SearchController {
    waft_widgets::text_editing_controller_accessors!();
}

/// A Material Design search bar.
///
/// A [`SearchBar`] looks like a [`TextField`]. Pressing the [`SearchBar`] typically shows a
/// search view route. Users can type the search query in the search view.
///
/// See also:
///
///  * [`SearchAnchor`], a widget that typically shows a [`SearchBar`] and, when tapped,
///    displays a search view.
///  * [`SearchBarTheme`], a widget that overrides the default configuration of a search bar.
///  * [`SearchViewTheme`], a widget that overrides the default configuration of a search
///    view.
#[derive(Clone)]
pub struct SearchBar {
    pub key: Option<KeyRef>,

    /// Controls the text being edited in the search bar's text field.
    pub controller: Option<TextEditingControllerRef>,

    /// Defines the keyboard focus for this widget.
    pub focus_node: Option<FocusNodeRef>,

    /// Text that suggests what sort of input the field accepts.
    ///
    /// Displayed at the same location on the screen where text may be entered when the input
    /// is empty.
    pub hint_text: Option<String>,

    /// A widget to display before the text input field.
    ///
    /// Typically the [`leading`](Self::leading) widget is an [`Icon`] or an [`IconButton`].
    pub leading: Option<WidgetRef>,

    /// A list of widgets to display in a row after the text field.
    ///
    /// Typically these actions can represent additional modes of searching (like voice
    /// search), an avatar, or an overflow menu. There should not be more than two trailing
    /// actions.
    pub trailing: Option<Vec<WidgetRef>>,

    /// Called when the user taps this search bar.
    pub on_tap: Option<Listener>,

    /// Called when the user taps outside the search bar's text field.
    pub on_tap_outside: Option<TapRegionCallback>,

    /// Invoked upon user input.
    pub on_changed: Option<ValueChanged<String>>,

    /// Called when the user indicates that they are done editing the text in the field.
    pub on_submitted: Option<ValueChanged<String>>,

    /// Optional size constraints for the search bar.
    ///
    /// If null, the value of [`SearchBarThemeData::constraints`] will be used. If that is
    /// also null, then the constraints defaults to:
    /// `BoxConstraints { min_width: 360.0, max_width: 800.0, min_height: 56.0, .. }`.
    pub constraints: Option<BoxConstraints>,

    /// The elevation of the search bar's [`Material`].
    ///
    /// If null, the value of [`SearchBarThemeData::elevation`] will be used. If that is also
    /// null, then default value is 6.0.
    pub elevation: Option<WidgetStatePropertyRef<Option<f64>>>,

    /// The search bar's background fill color.
    ///
    /// If null, the value of [`SearchBarThemeData::background_color`] will be used. If that
    /// is also null, then the default value is `ColorScheme::surface_container_high`.
    pub background_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The shadow color of the search bar's [`Material`].
    ///
    /// If null, the value of [`SearchBarThemeData::shadow_color`] will be used. If that is
    /// also null, then the default value is `ColorScheme::shadow`.
    pub shadow_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The surface tint color of the search bar's [`Material`].
    ///
    /// If null, the value of [`SearchBarThemeData::surface_tint_color`] will be used. If that
    /// is also null, then the default value is `Colors::TRANSPARENT`.
    pub surface_tint_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The highlight color that's typically used to indicate that the search bar is focused,
    /// hovered, or pressed.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The color and weight of the search bar's outline.
    ///
    /// This value is combined with [`shape`](Self::shape) to create a shape decorated with an
    /// outline.
    pub side: Option<WidgetStatePropertyRef<Option<BorderSide>>>,

    /// The shape of the search bar's underlying [`Material`].
    ///
    /// This shape is combined with [`side`](Self::side) to create a shape decorated with an
    /// outline.
    pub shape: Option<WidgetStatePropertyRef<Option<OutlinedBorderRef>>>,

    /// The padding between the search bar's boundary and its contents.
    ///
    /// If null, the value of [`SearchBarThemeData::padding`] will be used. If that is also
    /// null, then the default value is 8.0 horizontally.
    pub padding: Option<WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>>,

    /// The style to use for the text being edited.
    ///
    /// If null, defaults to the `bodyLarge` text style from the current [`Theme`]. The
    /// default text color is `ColorScheme::on_surface`.
    pub text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,

    /// The style to use for the [`hint_text`](Self::hint_text).
    ///
    /// If null, the value of [`SearchBarThemeData::hint_style`] will be used. If that is also
    /// null, the value of [`text_style`](Self::text_style) will be used, and then
    /// `bodyLarge`.
    pub hint_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,

    /// Configures how the platform keyboard will select an uppercase or lowercase keyboard.
    ///
    /// Defaults to [`TextCapitalization::None`].
    pub text_capitalization: Option<TextCapitalization>,

    /// If false the text field is "disabled": it ignores taps and its decoration is rendered
    /// in grey.
    pub enabled: bool,

    /// Whether this text field should focus itself if nothing else is already focused.
    pub auto_focus: bool,

    /// The type of action button to use for the keyboard.
    pub text_input_action: Option<TextInputAction>,

    /// The type of action button to use for the keyboard.
    pub keyboard_type: Option<TextInputType>,

    /// Configures padding to edges surrounding a [`Scrollable`](waft_widgets::Scrollable) when
    /// the text field scrolls into view.
    pub scroll_padding: EdgeInsets,

    /// Builds the text selection toolbar when requested by the user.
    ///
    /// See also:
    ///  * [`SearchBar::default_context_menu_builder`], which is the default context menu
    ///    builder.
    pub context_menu_builder: Option<EditableTextContextMenuBuilder>,

    /// Whether this text field is read only.
    pub read_only: bool,

    /// Whether to allow the platform to automatically format dashes.
    pub smart_dashes_type: Option<SmartDashesType>,

    /// Whether to allow the platform to automatically format quotes.
    pub smart_quotes_type: Option<SmartQuotesType>,
}

impl SearchBar {
    /// Creates a Material Design search bar.
    pub fn new() -> SearchBar {
        SearchBar {
            key: None,
            controller: None,
            focus_node: None,
            hint_text: None,
            leading: None,
            trailing: None,
            on_tap: None,
            on_tap_outside: None,
            on_changed: None,
            on_submitted: None,
            constraints: None,
            elevation: None,
            background_color: None,
            shadow_color: None,
            surface_tint_color: None,
            overlay_color: None,
            side: None,
            shape: None,
            padding: None,
            text_style: None,
            hint_style: None,
            text_capitalization: None,
            enabled: true,
            auto_focus: false,
            text_input_action: None,
            keyboard_type: None,
            scroll_padding: EdgeInsets::all(20.0),
            context_menu_builder: Some(Rc::new(SearchBar::default_context_menu_builder)),
            read_only: false,
            smart_dashes_type: None,
            smart_quotes_type: None,
        }
    }

    /// Dart `SearchBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> SearchBar {
        self.key = Some(key);
        self
    }

    /// Dart `SearchBar(controller:)`.
    pub fn controller(mut self, controller: TextEditingControllerRef) -> SearchBar {
        self.controller = Some(controller);
        self
    }

    /// Dart `SearchBar(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> SearchBar {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `SearchBar(hintText:)`.
    pub fn hint_text(mut self, hint_text: impl Into<String>) -> SearchBar {
        self.hint_text = Some(hint_text.into());
        self
    }

    /// Dart `SearchBar(leading:)`.
    pub fn leading<K>(mut self, leading: impl IntoWidget<K>) -> SearchBar {
        self.leading = Some(leading.into_widget());
        self
    }

    /// Dart `SearchBar(trailing:)`.
    pub fn trailing(mut self, trailing: Vec<WidgetRef>) -> SearchBar {
        self.trailing = Some(trailing);
        self
    }

    /// Dart `SearchBar(onTap:)`.
    pub fn on_tap(mut self, on_tap: Listener) -> SearchBar {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `SearchBar(onTapOutside:)`.
    pub fn on_tap_outside(mut self, on_tap_outside: TapRegionCallback) -> SearchBar {
        self.on_tap_outside = Some(on_tap_outside);
        self
    }

    /// Dart `SearchBar(onChanged:)`.
    pub fn on_changed(mut self, on_changed: impl Fn(&mut App, String) + 'static) -> SearchBar {
        self.on_changed = Some(Rc::new(on_changed));
        self
    }

    /// Dart `SearchBar(onSubmitted:)`.
    pub fn on_submitted(mut self, on_submitted: impl Fn(&mut App, String) + 'static) -> SearchBar {
        self.on_submitted = Some(Rc::new(on_submitted));
        self
    }

    /// Dart `SearchBar(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> SearchBar {
        self.constraints = Some(constraints);
        self
    }

    /// Dart `SearchBar(elevation:)`.
    pub fn elevation(mut self, elevation: WidgetStatePropertyRef<Option<f64>>) -> SearchBar {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `SearchBar(backgroundColor:)`.
    pub fn background_color(
        mut self,
        background_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SearchBar {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `SearchBar(shadowColor:)`.
    pub fn shadow_color(
        mut self,
        shadow_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SearchBar {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `SearchBar(surfaceTintColor:)`.
    pub fn surface_tint_color(
        mut self,
        surface_tint_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SearchBar {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `SearchBar(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SearchBar {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `SearchBar(side:)`.
    pub fn side(mut self, side: WidgetStatePropertyRef<Option<BorderSide>>) -> SearchBar {
        self.side = Some(side);
        self
    }

    /// Dart `SearchBar(shape:)`.
    pub fn shape(mut self, shape: WidgetStatePropertyRef<Option<OutlinedBorderRef>>) -> SearchBar {
        self.shape = Some(shape);
        self
    }

    /// Dart `SearchBar(padding:)`.
    pub fn padding(
        mut self,
        padding: WidgetStatePropertyRef<Option<EdgeInsetsGeometry>>,
    ) -> SearchBar {
        self.padding = Some(padding);
        self
    }

    /// Dart `SearchBar(textStyle:)`.
    pub fn text_style(
        mut self,
        text_style: WidgetStatePropertyRef<Option<TextStyle>>,
    ) -> SearchBar {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `SearchBar(hintStyle:)`.
    pub fn hint_style(
        mut self,
        hint_style: WidgetStatePropertyRef<Option<TextStyle>>,
    ) -> SearchBar {
        self.hint_style = Some(hint_style);
        self
    }

    /// Dart `SearchBar(textCapitalization:)`.
    pub fn text_capitalization(mut self, text_capitalization: TextCapitalization) -> SearchBar {
        self.text_capitalization = Some(text_capitalization);
        self
    }

    /// Dart `SearchBar(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> SearchBar {
        self.enabled = enabled;
        self
    }

    /// Dart `SearchBar(autoFocus:)`.
    pub fn auto_focus(mut self, auto_focus: bool) -> SearchBar {
        self.auto_focus = auto_focus;
        self
    }

    /// Dart `SearchBar(textInputAction:)`.
    pub fn text_input_action(mut self, text_input_action: TextInputAction) -> SearchBar {
        self.text_input_action = Some(text_input_action);
        self
    }

    /// Dart `SearchBar(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> SearchBar {
        self.keyboard_type = Some(keyboard_type);
        self
    }

    /// Dart `SearchBar(scrollPadding:)`.
    pub fn scroll_padding(mut self, scroll_padding: EdgeInsets) -> SearchBar {
        self.scroll_padding = scroll_padding;
        self
    }

    /// Dart `SearchBar(contextMenuBuilder:)`.
    pub fn context_menu_builder(
        mut self,
        context_menu_builder: Option<EditableTextContextMenuBuilder>,
    ) -> SearchBar {
        self.context_menu_builder = context_menu_builder;
        self
    }

    /// Dart `SearchBar(readOnly:)`.
    pub fn read_only(mut self, read_only: bool) -> SearchBar {
        self.read_only = read_only;
        self
    }

    /// Dart `SearchBar(smartDashesType:)`.
    pub fn smart_dashes_type(mut self, smart_dashes_type: SmartDashesType) -> SearchBar {
        self.smart_dashes_type = Some(smart_dashes_type);
        self
    }

    /// Dart `SearchBar(smartQuotesType:)`.
    pub fn smart_quotes_type(mut self, smart_quotes_type: SmartQuotesType) -> SearchBar {
        self.smart_quotes_type = Some(smart_quotes_type);
        self
    }

    /// Dart's `_defaultContextMenuBuilder`.
    pub fn default_context_menu_builder(
        app: &mut App,
        _context: BuildContext,
        editable_text_state: Handle<EditableTextState>,
    ) -> WidgetRef {
        if SystemContextMenu::is_supported_by_field(app, editable_text_state) {
            return SystemContextMenu::editable_text(app, editable_text_state).into_widget();
        }
        AdaptiveTextSelectionToolbar::editable_text(app, editable_text_state).into_widget()
    }
}

impl Default for SearchBar {
    fn default() -> SearchBar {
        SearchBar::new()
    }
}

impl Debug for SearchBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SearchBar")
            .field("hint_text", &self.hint_text)
            .field("enabled", &self.enabled)
            .field("auto_focus", &self.auto_focus)
            .field("read_only", &self.read_only)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SearchBar {
    type State = SearchBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SearchBarState {
        SearchBarState {
            state: StateData::new(),
            internal_states_controller: None,
            internal_focus_node: None,
        }
    }
}

/// Dart's `_SearchBarState`.
pub struct SearchBarState {
    state: StateData<SearchBar>,
    /// Dart's `late final MaterialStatesController _internalStatesController`, created in
    /// `init_state` because an arena object needs the [`App`].
    internal_states_controller: Option<Ref<WidgetStatesController>>,
    internal_focus_node: Option<Ref<FocusNode>>,
}

impl SearchBarState {
    fn states_controller(self: Handle<Self>, app: &App) -> Handle<WidgetStatesController> {
        app.get(self)
            .internal_states_controller
            .as_ref()
            .expect("created in init_state")
            .handle()
    }

    /// Dart's `_focusNode`.
    fn focus_node(self: Handle<Self>, app: &mut App) -> FocusNodeRef {
        if let Some(focus_node) = &self.widget(app).focus_node {
            return focus_node.clone();
        }
        if app.get(self).internal_focus_node.is_none() {
            let focus_node = FocusNode::new(app);
            app.get_mut(self).internal_focus_node = Some(focus_node);
        }
        app.get(self)
            .internal_focus_node
            .as_ref()
            .expect("created above")
            .erase()
    }

    fn handle_states_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
    }

    fn handle_tap(self: Handle<Self>, app: &mut App) {
        if let Some(on_tap) = self.widget(app).on_tap.clone() {
            on_tap.call(app);
        }
        let focus_node = self.focus_node(app);
        if !focus_node.has_focus(app) {
            focus_node.request_focus(app, None);
        }
    }
}

/// Dart's local `resolve<T>(widgetValue, themeValue, defaultValue)`.
fn resolve_search_bar_property<T: Clone + 'static>(
    states: &waft_widgets::WidgetStates,
    widget_value: &Option<WidgetStatePropertyRef<Option<T>>>,
    theme_value: &Option<WidgetStatePropertyRef<Option<T>>>,
    default_value: &Option<WidgetStatePropertyRef<Option<T>>>,
) -> Option<T> {
    widget_value
        .as_ref()
        .and_then(|value| value.resolve(states))
        .or_else(|| theme_value.as_ref().and_then(|value| value.resolve(states)))
        .or_else(|| {
            default_value
                .as_ref()
                .and_then(|value| value.resolve(states))
        })
}

impl State for SearchBarState {
    type Widget = SearchBar;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = WidgetStatesController::new(app, None);
        controller.add_listener(
            app,
            Listener::handle_method(self, SearchBarState::handle_states_changed),
        );
        app.get_mut(self).internal_states_controller = Some(controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.states_controller(app);
        ChangeNotifierData::dispose(app, controller);
        if let Some(focus_node) = app.get(self).internal_focus_node.clone() {
            ChangeNotifierData::dispose(app, focus_node.handle());
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let text_direction = Directionality::of(app, context);
        let color_scheme = Theme::of(app, context).color_scheme.clone();
        let search_bar_theme = SearchBarTheme::of(app, context);
        let defaults = SearchBarDefaultsM3::new(context);
        let states = app.get(self.states_controller(app)).value().clone();

        let default_text_style = defaults.text_style(app);
        let default_elevation = defaults.elevation(app);
        let default_shadow_color = defaults.shadow_color(app);
        let default_background_color = defaults.background_color(app);
        let default_surface_tint_color = defaults.surface_tint_color(app);
        let default_shape = defaults.shape(app);
        let default_side = defaults.side(app);
        let default_padding = defaults.padding(app);

        let effective_text_style = resolve_search_bar_property(
            &states,
            &widget.text_style,
            &search_bar_theme.text_style,
            &default_text_style,
        );
        let effective_elevation = resolve_search_bar_property(
            &states,
            &widget.elevation,
            &search_bar_theme.elevation,
            &default_elevation,
        );
        let effective_shadow_color = resolve_search_bar_property(
            &states,
            &widget.shadow_color,
            &search_bar_theme.shadow_color,
            &default_shadow_color,
        );
        let effective_background_color = resolve_search_bar_property(
            &states,
            &widget.background_color,
            &search_bar_theme.background_color,
            &default_background_color,
        );
        let effective_surface_tint_color = resolve_search_bar_property(
            &states,
            &widget.surface_tint_color,
            &search_bar_theme.surface_tint_color,
            &default_surface_tint_color,
        );
        let effective_shape = resolve_search_bar_property(
            &states,
            &widget.shape,
            &search_bar_theme.shape,
            &default_shape,
        );
        let effective_side = resolve_search_bar_property(
            &states,
            &widget.side,
            &search_bar_theme.side,
            &default_side,
        );
        let effective_padding = resolve_search_bar_property(
            &states,
            &widget.padding,
            &search_bar_theme.padding,
            &default_padding,
        );
        let effective_overlay_color = widget
            .overlay_color
            .clone()
            .or_else(|| search_bar_theme.overlay_color.clone())
            .or_else(|| defaults.overlay_color(app));
        let effective_text_capitalization = widget
            .text_capitalization
            .or(search_bar_theme.text_capitalization)
            .or_else(|| defaults.text_capitalization(app))
            .expect("defaults.textCapitalization!");

        let effective_hint_style = widget
            .hint_style
            .as_ref()
            .and_then(|hint_style| hint_style.resolve(&states))
            .or_else(|| {
                search_bar_theme
                    .hint_style
                    .as_ref()
                    .and_then(|hint_style| hint_style.resolve(&states))
            })
            .or_else(|| {
                widget
                    .text_style
                    .as_ref()
                    .and_then(|text_style| text_style.resolve(&states))
            })
            .or_else(|| {
                search_bar_theme
                    .text_style
                    .as_ref()
                    .and_then(|text_style| text_style.resolve(&states))
            })
            .or_else(|| {
                defaults
                    .hint_style(app)
                    .and_then(|hint_style| hint_style.resolve(&states))
            });

        let default_color = match color_scheme.brightness {
            Brightness::Light => K_DEFAULT_ICON_DARK_COLOR,
            Brightness::Dark => K_DEFAULT_ICON_LIGHT_COLOR,
        };
        let icon_theme = IconTheme::of(app, context);
        let custom_theme =
            if icon_theme.color.as_ref().map(|color| color.color()) != Some(default_color) {
                Some(icon_theme)
            } else {
                None
            };

        let leading = widget.leading.clone().map(|leading| {
            IconTheme::merge(
                None,
                custom_theme
                    .clone()
                    .unwrap_or_else(|| IconThemeData::new().color(color_scheme.on_surface)),
                leading,
            )
            .into_widget()
        });

        let trailing: Option<Vec<WidgetRef>> = widget.trailing.as_ref().map(|trailing| {
            trailing
                .iter()
                .map(|trailing| {
                    IconTheme::merge(
                        None,
                        custom_theme.clone().unwrap_or_else(|| {
                            IconThemeData::new().color(color_scheme.on_surface_variant())
                        }),
                        trailing.clone(),
                    )
                    .into_widget()
                })
                .collect()
        });

        let effective_shape_with_side = effective_shape
            .as_ref()
            .map(|shape| shape.copy_with(effective_side));
        let effective_padding = effective_padding.expect("effectivePadding!");
        let focus_node = self.focus_node(app);
        let states_controller = app
            .get(self)
            .internal_states_controller
            .clone()
            .expect("created in init_state");
        let bar_constraints = widget
            .constraints
            .or(search_bar_theme.constraints)
            .or_else(|| defaults.constraints(app))
            .expect("defaults.constraints!");

        let mut input_decoration_theme = InputDecorationThemeData::new();
        if let Some(effective_hint_style) = effective_hint_style {
            input_decoration_theme = input_decoration_theme.hint_style(effective_hint_style);
        }
        let input_decoration_theme = input_decoration_theme
            // The configuration below is to make sure that the text field
            // in `SearchBar` will not be overridden by the overall
            // `InputDecorationThemeData`
            .enabled_border(InputBorderRef::none())
            .border(InputBorderRef::none())
            .focused_border(InputBorderRef::none())
            .content_padding(EdgeInsetsGeometry::from(EdgeInsets::ZERO))
            // Setting `isDense` to true to allow the text field height to be
            // smaller than 48.0
            .is_dense(true);
        let mut text_field = TextField::new()
            .read_only(widget.read_only)
            .autofocus(widget.auto_focus)
            .on_tap_always_called(true)
            .focus_node(focus_node)
            .enabled(widget.enabled)
            .decoration(Some(
                hint_decoration(widget.hint_text.clone()).apply_defaults(input_decoration_theme),
            ))
            .text_capitalization(effective_text_capitalization)
            .scroll_padding(widget.scroll_padding)
            .context_menu_builder(widget.context_menu_builder.clone());
        if let Some(on_tap) = widget.on_tap.clone() {
            text_field = text_field.on_tap(on_tap);
        }
        if let Some(on_tap_outside) = widget.on_tap_outside.clone() {
            text_field = text_field.on_tap_outside(move |app, event| on_tap_outside(app, event));
        }
        if let Some(on_changed) = widget.on_changed.clone() {
            text_field = text_field.on_changed(move |app, value| on_changed(app, value));
        }
        if let Some(on_submitted) = widget.on_submitted.clone() {
            text_field = text_field.on_submitted(move |app, value| on_submitted(app, value));
        }
        if let Some(controller) = widget.controller.clone() {
            text_field = text_field.controller(controller);
        }
        if let Some(style) = effective_text_style {
            text_field = text_field.style(style);
        }
        if let Some(text_input_action) = widget.text_input_action {
            text_field = text_field.text_input_action(text_input_action);
        }
        if let Some(keyboard_type) = widget.keyboard_type {
            text_field = text_field.keyboard_type(keyboard_type);
        }
        if let Some(smart_dashes_type) = widget.smart_dashes_type {
            text_field = text_field.smart_dashes_type(smart_dashes_type);
        }
        if let Some(smart_quotes_type) = widget.smart_quotes_type {
            text_field = text_field.smart_quotes_type(smart_quotes_type);
        }

        let mut row_children = Vec::new();
        row_children.extend(leading);
        row_children.push(
            Expanded::new(
                Padding::new(effective_padding).child(
                    Semantics::new()
                        .input_type(SemanticsInputType::Search)
                        .child(text_field),
                ),
            )
            .into_widget(),
        );
        row_children.extend(trailing.into_iter().flatten());

        let mut material =
            Material::new().elevation(effective_elevation.expect("effectiveElevation!"));
        if let Some(shadow_color) = effective_shadow_color {
            material = material.shadow_color(shadow_color);
        }
        if let Some(color) = effective_background_color {
            material = material.color(color);
        }
        if let Some(surface_tint_color) = effective_surface_tint_color {
            material = material.surface_tint_color(surface_tint_color);
        }
        if let Some(shape) = effective_shape_with_side.clone() {
            material = material.shape(shape);
        }
        let mut ink_well = InkWell::new()
            // Avoid creating a redundant focus stop and focusable/tappable
            // semantics node on the InkWell, since the inner TextField
            // already handles focus and semantics for the SearchBar.
            .can_request_focus(false)
            .exclude_from_semantics(true)
            .on_tap(Listener::handle_method(self, SearchBarState::handle_tap))
            .states_controller(states_controller)
            .child(
                Padding::new(effective_padding).child(
                    Row::new()
                        .text_direction(text_direction)
                        .children(row_children),
                ),
            );
        if let Some(overlay_color) = effective_overlay_color {
            ink_well = ink_well.overlay_color(overlay_color);
        }
        if let Some(shape) = effective_shape_with_side {
            ink_well = ink_well.custom_border(shape);
        }

        ConstrainedBox::new(bar_constraints)
            .child(
                Opacity::new(if widget.enabled {
                    1.0
                } else {
                    K_DISABLE_SEARCH_BAR_OPACITY
                })
                .child(
                    material.child(
                        IgnorePointer::new()
                            .ignoring(!widget.enabled)
                            .child(ink_well),
                    ),
                ),
            )
            .into_widget()
    }
}

/// Dart's `InputDecoration(hintText: hintText)`.
fn hint_decoration(hint_text: Option<String>) -> InputDecoration {
    match hint_text {
        Some(hint_text) => InputDecoration::new().hint_text(hint_text),
        None => InputDecoration::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::rc::Rc;

    use waft_embedder::{FontWeight, TargetPlatform, TextAffinity, TextSelection};
    use waft_foundation::Completer;
    use waft_painting::{Border, BoxDecoration, StadiumBorder};
    use waft_widgets::testing::{
        WidgetTester, expect, find, finds_nothing, finds_one_widget, test_widgets,
    };
    use waft_widgets::{Center, Container, EditableText, FocusNode, Text, TextEditingController};

    use super::*;
    use crate::icon_button::IconButton;
    use crate::icons::Icons;
    use crate::list_tile::ListTile;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    fn m3(tester: &WidgetTester) -> ThemeData {
        ThemeData::new(&tester.cell.borrow())
    }

    fn pump_material_home(tester: &mut WidgetTester, theme: Option<ThemeData>, home: WidgetRef) {
        let theme = theme.unwrap_or_else(|| m3(tester));
        tester.pump_widget(material_app(theme, home));
    }

    fn search_bar_material(tester: &WidgetTester) -> Material {
        let finder = find::descendant(find::by_type::<SearchBar>(), find::by_type::<Material>());
        tester.widget(finder, |material: &Material| material.clone())
    }

    /// Dart's `checkSearchBarDefaults`.
    fn check_search_bar_defaults(
        tester: &mut WidgetTester,
        color_scheme: &ColorScheme,
        material: &Material,
    ) {
        assert_eq!(material.animation_duration, Duration::from_millis(200));
        assert!(material.border_on_foreground);
        assert_eq!(material.border_radius, None);
        assert_eq!(material.clip_behavior, Clip::None);
        assert_eq!(material.color, Some(color_scheme.surface_container_high()));
        assert_eq!(material.elevation, 6.0);
        assert_eq!(material.shadow_color, Some(color_scheme.shadow()));
        assert_eq!(material.surface_tint_color, Some(Colors::TRANSPARENT));
        assert_eq!(
            material.shape,
            Some(StadiumBorder::new(BorderSide::NONE).into())
        );

        let hint_style = tester.widget(find::text("hint text"), |text: &Text| text.style.clone());
        let hint_style = hint_style.expect("a hint style");
        assert_eq!(
            hint_style.color.as_ref().map(|color| color.color()),
            Some(color_scheme.on_surface_variant())
        );
        assert_eq!(hint_style.font_size, Some(16.0));
        assert_eq!(hint_style.font_family.as_deref(), Some("Roboto"));
        assert_eq!(hint_style.font_weight, Some(FontWeight::W400));

        let input = "entered text";
        tester.enter_text(find::by_type::<SearchBar>(), input);
        let input_style =
            tester.widget(find::text(input), |text: &EditableText| text.style.clone());
        assert_eq!(
            input_style.color.as_ref().map(|color| color.color()),
            Some(color_scheme.on_surface)
        );
        assert_eq!(input_style.font_size, Some(16.0));
        assert_eq!(input_style.font_weight, Some(FontWeight::W400));
    }

    // search_anchor_test.dart 'SearchBar defaults'.
    #[test]
    fn search_bar_defaults() {
        test_widgets(|tester| {
            let theme = m3(tester);
            let color_scheme = theme.color_scheme.clone();
            pump_material_home(
                tester,
                Some(theme),
                Material::new()
                    .child(SearchBar::new().hint_text("hint text"))
                    .into_widget(),
            );

            let material = search_bar_material(tester);
            check_search_bar_defaults(tester, &color_scheme, &material);
        });
    }

    // search_anchor_test.dart 'SearchBar respects controller property'.
    #[test]
    fn search_bar_respects_controller_property() {
        test_widgets(|tester| {
            let default_text = "default text";
            let controller = {
                let mut app = tester.cell.borrow_mut();
                TextEditingController::new(&mut app).text(&mut app, default_text)
            };
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(SearchBar::new().controller(controller.erase()))
                    .into_widget(),
            );

            assert_eq!(
                controller.handle().text_value(&tester.cell.borrow()),
                default_text
            );
            expect(tester, &find::text(default_text), finds_one_widget());

            let updated_text = "updated text";
            tester.enter_text(find::by_type::<SearchBar>(), updated_text);
            assert_eq!(
                controller.handle().text_value(&tester.cell.borrow()),
                updated_text
            );
            expect(tester, &find::text(default_text), finds_nothing());
            expect(tester, &find::text(updated_text), finds_one_widget());
        });
    }

    // search_anchor_test.dart 'SearchBar respects focusNode property'.
    #[test]
    fn search_bar_respects_focus_node_property() {
        test_widgets(|tester| {
            let node = FocusNode::new(&mut tester.cell.borrow_mut());
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(SearchBar::new().focus_node(node.erase()))
                    .into_widget(),
            );

            assert!(!node.handle().has_focus(&mut tester.cell.borrow_mut()));

            node.handle()
                .request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump();
            assert!(node.handle().has_focus(&mut tester.cell.borrow_mut()));

            node.handle()
                .unfocus(&mut tester.cell.borrow_mut(), UnfocusDisposition::Scope);
            tester.pump();
            assert!(!node.handle().has_focus(&mut tester.cell.borrow_mut()));
        });
    }

    // search_anchor_test.dart 'SearchBar has correct default layout and padding LTR'.
    #[test]
    fn search_bar_has_correct_default_layout_and_padding_ltr() {
        test_widgets(|tester| {
            pump_material_home(tester, None, layout_bar());

            let bar_rect = tester.get_rect(find::by_type::<SearchBar>());
            assert_eq!(bar_rect.size(), Size::new(800.0, 56.0));
            assert_eq!(bar_rect, Rect::from_ltrb(0.0, 272.0, 800.0, 328.0));

            let leading_icon =
                tester.get_rect(find::widget_with_icon::<IconButton>(&Icons::search()));
            // Default left padding is 8.0, and icon button has 8.0 padding, so in total the
            // padding between the edge of the bar and the icon of the button is 16.0, which
            // matches the spec.
            assert_eq!(leading_icon.left, bar_rect.left + 8.0);

            let text_field = tester.get_rect(find::by_type::<TextField>());
            assert_eq!(text_field.left, leading_icon.right + 8.0);

            let trailing_icon =
                tester.get_rect(find::widget_with_icon::<IconButton>(&Icons::menu()));
            assert_eq!(trailing_icon.left, text_field.right + 8.0);
            assert_eq!(trailing_icon.right, bar_rect.right - 8.0);
        });
    }

    fn layout_bar() -> WidgetRef {
        Center::new()
            .child(
                SearchBar::new()
                    .leading(IconButton::new(
                        Some(Listener::new(|_| {})),
                        Icon::new(Some(Icons::search())),
                    ))
                    .trailing(vec![
                        IconButton::new(
                            Some(Listener::new(|_| {})),
                            Icon::new(Some(Icons::menu())),
                        )
                        .into_widget(),
                    ]),
            )
            .into_widget()
    }

    // search_anchor_test.dart 'SearchBar has correct default layout and padding - RTL'.
    #[test]
    fn search_bar_has_correct_default_layout_and_padding_rtl() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                Directionality::new(TextDirection::Rtl, layout_bar()).into_widget(),
            );

            let bar_rect = tester.get_rect(find::by_type::<SearchBar>());
            assert_eq!(bar_rect.size(), Size::new(800.0, 56.0));
            assert_eq!(bar_rect, Rect::from_ltrb(0.0, 272.0, 800.0, 328.0));

            // The default padding is set to 8.0 so the distance between the icon of the
            // button and the edge of the bar is 16.0, which matches the spec.
            let leading_icon =
                tester.get_rect(find::widget_with_icon::<IconButton>(&Icons::search()));
            assert_eq!(leading_icon.right, bar_rect.right - 8.0);

            let text_field = tester.get_rect(find::by_type::<TextField>());
            assert_eq!(text_field.right, leading_icon.left - 8.0);

            let trailing_icon =
                tester.get_rect(find::widget_with_icon::<IconButton>(&Icons::menu()));
            assert_eq!(trailing_icon.right, text_field.left - 8.0);
            assert_eq!(trailing_icon.left, bar_rect.left + 8.0);
        });
    }

    // search_anchor_test.dart 'SearchBar respects hintText property'.
    #[test]
    fn search_bar_respects_hint_text_property() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(SearchBar::new().hint_text("hint text"))
                    .into_widget(),
            );
            expect(tester, &find::text("hint text"), finds_one_widget());
        });
    }

    // search_anchor_test.dart 'SearchBar respects onTap property'.
    #[test]
    fn search_bar_respects_on_tap_property() {
        test_widgets(|tester| {
            let tap_count = Rc::new(Cell::new(0));
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(SearchBar::new().on_tap(Listener::new({
                        let tap_count = Rc::clone(&tap_count);
                        move |_| tap_count.set(tap_count.get() + 1)
                    })))
                    .into_widget(),
            );
            assert_eq!(tap_count.get(), 0);
            tester.tap(find::by_type::<SearchBar>());
            assert_eq!(tap_count.get(), 1);
            tester.tap(find::by_type::<SearchBar>());
            assert_eq!(tap_count.get(), 2);
        });
    }

    // search_anchor_test.dart 'SearchBar respects onChanged property'.
    #[test]
    fn search_bar_respects_on_changed_property() {
        test_widgets(|tester| {
            let change_count = Rc::new(Cell::new(0));
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(SearchBar::new().on_changed({
                        let change_count = Rc::clone(&change_count);
                        move |_, _| change_count.set(change_count.get() + 1)
                    }))
                    .into_widget(),
            );
            assert_eq!(change_count.get(), 0);
            tester.enter_text(find::by_type::<SearchBar>(), "a");
            assert_eq!(change_count.get(), 1);
            tester.enter_text(find::by_type::<SearchBar>(), "b");
            assert_eq!(change_count.get(), 2);
        });
    }

    // search_anchor_test.dart 'SearchBar respects onSubmitted property'.
    #[test]
    fn search_bar_respects_on_submitted_property() {
        test_widgets(|tester| {
            let submitted_query = Rc::new(RefCell::new(String::new()));
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(SearchBar::new().on_submitted({
                        let submitted_query = Rc::clone(&submitted_query);
                        move |_, text| *submitted_query.borrow_mut() = text
                    }))
                    .into_widget(),
            );
            tester.enter_text(find::by_type::<SearchBar>(), "query");
            tester
                .test_text_input()
                .receive_action(TextInputAction::Done);
            assert_eq!(*submitted_query.borrow(), "query");
        });
    }

    fn view_content() -> waft_widgets::testing::Finder {
        find::by_type::<ViewContent>()
    }

    fn search_view_material(tester: &WidgetTester) -> Material {
        let finder = find::descendant(view_content(), find::by_type::<Material>()).first();
        tester.widget(finder, |material: &Material| material.clone())
    }

    fn search_view_size(tester: &WidgetTester) -> Size {
        tester.get_size(find::descendant(view_content(), find::by_type::<ConstrainedBox>()).first())
    }

    fn search_icon_anchor() -> SearchAnchor {
        SearchAnchor::new(
            |_app, _context, _controller| Icon::new(Some(Icons::search())).into_widget(),
            |_app, _context, _controller| Task::ready(Vec::new()),
        )
    }

    fn button_anchor() -> SearchAnchor {
        SearchAnchor::new(
            |_app, _context, _controller| {
                IconButton::new(
                    Some(Listener::new(|_| {})),
                    Icon::new(Some(Icons::search())),
                )
                .into_widget()
            },
            |_app, _context, _controller| Task::ready(Vec::new()),
        )
    }

    // search_anchor_test.dart 'The search view defaults'.
    #[test]
    fn the_search_view_defaults() {
        test_widgets(|tester| {
            let theme = m3(tester);
            let color_scheme = theme.color_scheme.clone();
            pump_material_home(
                tester,
                Some(theme),
                Scaffold::new()
                    .body(
                        Material::new().child(
                            Align::new()
                                .alignment(AlignmentGeometry::TOP_LEFT)
                                .child(search_icon_anchor().view_hint_text("hint text")),
                        ),
                    )
                    .into_widget(),
            );

            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            let material = search_view_material(tester);
            assert_eq!(material.elevation, 6.0);
            assert_eq!(material.color, Some(color_scheme.surface_container_high()));
            assert_eq!(material.surface_tint_color, Some(Colors::TRANSPARENT));
            assert_eq!(material.clip_behavior, Clip::AntiAlias);

            // Default search view has a leading back button on the start of the header.
            expect(tester, &find::by_type::<BackButton>(), finds_one_widget());

            let divider_color = tester.widget(
                find::descendant(find::by_type::<Divider>(), find::by_type::<Container>()).first(),
                |container: &Container| {
                    container
                        .decoration
                        .as_ref()
                        .expect("the container has a decoration")
                        .as_any()
                        .downcast_ref::<BoxDecoration>()
                        .expect("a BoxDecoration")
                        .border
                        .as_ref()
                        .expect("the decoration has a border")
                        .as_any()
                        .downcast_ref::<Border>()
                        .expect("a Border")
                        .bottom
                        .color
                },
            );
            assert_eq!(divider_color, color_scheme.outline());

            let hint_style = tester
                .widget(find::text("hint text"), |text: &Text| text.style.clone())
                .expect("a hint style");
            assert_eq!(
                hint_style.color.as_ref().map(|color| color.color()),
                Some(color_scheme.on_surface_variant())
            );
            assert_eq!(hint_style.font_size, Some(16.0));
            assert_eq!(hint_style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(hint_style.font_weight, Some(FontWeight::W400));

            let input = "entered text";
            tester.enter_text(find::by_type::<SearchBar>(), input);
            let input_style =
                tester.widget(find::text(input), |text: &EditableText| text.style.clone());
            assert_eq!(
                input_style.color.as_ref().map(|color| color.color()),
                Some(color_scheme.on_surface)
            );
            assert_eq!(input_style.font_size, Some(16.0));
            assert_eq!(input_style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(input_style.font_weight, Some(FontWeight::W400));
        });
    }

    fn pump_platform_anchor(
        tester: &mut WidgetTester,
        platform: TargetPlatform,
        full_screen: Option<bool>,
    ) {
        let theme = ThemeData::builder()
            .platform(platform)
            .build(&tester.cell.borrow());
        let mut anchor = search_icon_anchor();
        anchor.is_full_screen = full_screen;
        pump_material_home(
            tester,
            Some(theme),
            Scaffold::new()
                .body(SafeArea::new(
                    Material::new().child(
                        Align::new()
                            .alignment(AlignmentGeometry::TOP_LEFT)
                            .child(anchor),
                    ),
                ))
                .into_widget(),
        );
    }

    // search_anchor_test.dart 'The search view default size on different platforms'.
    #[test]
    fn the_search_view_default_size_on_different_platforms() {
        test_widgets(|tester| {
            // The search view should be is full-screen on mobile platforms, and have a size
            // of (360, 2/3 screen height) on other platforms.
            for platform in [
                TargetPlatform::IOS,
                TargetPlatform::Android,
                TargetPlatform::Fuchsia,
            ] {
                tester.pump_widget(Container::new().into_widget());
                pump_platform_anchor(tester, platform, None);
                tester.tap(find::by_icon(&Icons::search()));
                tester.pump_and_settle();
                let size = search_view_size(tester);
                assert_eq!(size.width(), 800.0);
                assert_eq!(size.height(), 600.0);
            }

            for platform in [TargetPlatform::Linux, TargetPlatform::Windows] {
                tester.pump_widget(Container::new().into_widget());
                pump_platform_anchor(tester, platform, None);
                tester.tap(find::by_icon(&Icons::search()));
                tester.pump_and_settle();
                let size = search_view_size(tester);
                assert_eq!(size.width(), 360.0);
                assert_eq!(size.height(), 400.0);
            }
        });
    }

    // search_anchor_test.dart 'SearchAnchor respects isFullScreen property'.
    #[test]
    fn search_anchor_respects_is_full_screen_property() {
        test_widgets(|tester| {
            for platform in [TargetPlatform::Linux, TargetPlatform::Windows] {
                tester.pump_widget(Container::new().into_widget());
                pump_platform_anchor(tester, platform, Some(true));
                tester.tap(find::by_icon(&Icons::search()));
                tester.pump_and_settle();
                let size = search_view_size(tester);
                assert_eq!(size.width(), 800.0);
                assert_eq!(size.height(), 600.0);
            }
        });
    }

    // search_anchor_test.dart 'SearchAnchor respects controller property'.
    #[test]
    fn search_anchor_respects_controller_property() {
        test_widgets(|tester| {
            let default_text = "initial text";
            let controller = SearchController::new(&mut tester.cell.borrow_mut());
            controller
                .handle()
                .set_text(&mut tester.cell.borrow_mut(), default_text);

            let anchor = SearchAnchor::new(
                |_app, _context, controller| {
                    IconButton::new(
                        Some(Listener::new(move |app: &mut App| {
                            controller.open_view(app)
                        })),
                        Icon::new(Some(Icons::search())),
                    )
                    .into_widget()
                },
                |_app, _context, _controller| Task::ready(Vec::new()),
            )
            .search_controller(controller.clone());
            pump_material_home(tester, None, Material::new().child(anchor).into_widget());

            tester.tap(find::widget_with_icon::<IconButton>(&Icons::search()));
            tester.pump_and_settle();
            assert_eq!(
                controller.handle().text_value(&tester.cell.borrow()),
                default_text
            );
            expect(tester, &find::text(default_text), finds_one_widget());

            let updated_text = "updated text";
            tester.enter_text(find::by_type::<SearchBar>(), updated_text);
            assert_eq!(
                controller.handle().text_value(&tester.cell.borrow()),
                updated_text
            );
            expect(tester, &find::text(default_text), finds_nothing());
            expect(tester, &find::text(updated_text), finds_one_widget());
        });
    }

    // search_anchor_test.dart 'SearchAnchor attaches and detaches controllers property'.
    #[test]
    fn search_anchor_attaches_and_detaches_controllers_property() {
        test_widgets(|tester| {
            let anchor = |controller: Option<&Ref<SearchController>>| {
                let mut anchor = SearchAnchor::new(
                    |_app, _context, _controller| Icon::new(Some(Icons::search())).into_widget(),
                    |_app, _context, _controller| Task::ready(Vec::new()),
                );
                anchor.search_controller = controller.cloned();
                Material::new().child(anchor).into_widget()
            };
            let is_attached = |tester: &WidgetTester, controller: &Ref<SearchController>| {
                controller.handle().is_attached(&tester.cell.borrow())
            };

            let controller1 = SearchController::new(&mut tester.cell.borrow_mut());
            assert!(!is_attached(tester, &controller1));

            pump_material_home(tester, None, anchor(Some(&controller1)));
            assert!(is_attached(tester, &controller1));

            pump_material_home(tester, None, anchor(None));
            assert!(!is_attached(tester, &controller1));

            let controller2 = SearchController::new(&mut tester.cell.borrow_mut());
            assert!(!is_attached(tester, &controller2));

            pump_material_home(tester, None, anchor(Some(&controller2)));
            assert!(!is_attached(tester, &controller1));
            assert!(is_attached(tester, &controller2));

            pump_material_home(tester, None, anchor(None));
            assert!(!is_attached(tester, &controller1));
            assert!(!is_attached(tester, &controller2));
        });
    }

    // search_anchor_test.dart 'SearchAnchor respects suggestionsBuilder property'.
    #[test]
    fn search_anchor_respects_suggestions_builder_property() {
        test_widgets(|tester| {
            let controller = SearchController::new(&mut tester.cell.borrow_mut());
            let suggestion = "suggestion text";
            let anchor = SearchAnchor::new(
                |_app, _context, _controller| Icon::new(Some(Icons::search())).into_widget(),
                move |_app, _context, controller| {
                    Task::ready(vec![
                        ListTile::new()
                            .title(Text::new(suggestion))
                            .on_tap(Listener::new(move |app: &mut App| {
                                controller.close_view(app, Some(suggestion.to_string()))
                            }))
                            .into_widget(),
                    ])
                },
            )
            .search_controller(controller.clone());
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(
                        Align::new()
                            .alignment(AlignmentGeometry::TOP_CENTER)
                            .child(anchor),
                    )
                    .into_widget(),
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();

            let list_tile = find::widget_with_text::<ListTile>(suggestion);
            expect(tester, &list_tile, finds_one_widget());
            tester.tap(list_tile);
            tester.pump_and_settle();

            assert!(!controller.handle().is_open(&tester.cell.borrow()));
            assert_eq!(
                controller.handle().text_value(&tester.cell.borrow()),
                suggestion
            );
        });
    }

    // search_anchor_test.dart 'SearchController can open/close view'.
    #[test]
    fn search_controller_can_open_and_close_view() {
        test_widgets(|tester| {
            let controller = SearchController::new(&mut tester.cell.borrow_mut());
            let anchor = SearchAnchor::bar(|_app, _context, controller| {
                Task::ready(vec![
                    ListTile::new()
                        .title(Text::new("item 0"))
                        .on_tap(Listener::new(move |app: &mut App| {
                            controller.close_view(app, Some("item 0".to_string()))
                        }))
                        .into_widget(),
                ])
            })
            .search_controller(controller.clone())
            .is_full_screen(false);
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(anchor.into_search_anchor())
                    .into_widget(),
            );

            let is_open =
                |tester: &WidgetTester| controller.handle().is_open(&tester.cell.borrow());
            assert!(!is_open(tester));
            tester.tap(find::by_type::<SearchBar>());
            tester.pump_and_settle();

            assert!(is_open(tester));
            tester.tap(find::widget_with_text::<ListTile>("item 0"));
            tester.pump_and_settle();
            assert!(!is_open(tester));
            controller.handle().open_view(&mut tester.cell.borrow_mut());
            assert!(is_open(tester));
        });
    }

    // search_anchor_test.dart 'Search view does not go off the screen - LTR'.
    #[test]
    fn search_view_does_not_go_off_the_screen_ltr() {
        test_widgets(|tester| {
            let mut anchor = button_anchor();
            anchor.is_full_screen = Some(false);
            anchor.builder = Rc::new(|_app, _context, controller| {
                IconButton::new(
                    Some(Listener::new(move |app: &mut App| {
                        controller.open_view(app)
                    })),
                    Icon::new(Some(Icons::search())),
                )
                .into_widget()
            });
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(
                        // Put the search anchor on the bottom-right corner of the screen to test
                        // if the search view goes off the window.
                        Align::new()
                            .alignment(AlignmentGeometry::BOTTOM_RIGHT)
                            .child(anchor),
                    )
                    .into_widget(),
            );

            let icon_button =
                tester.get_rect(find::widget_with_icon::<IconButton>(&Icons::search()));
            // Icon button has a size of (48.0, 48.0) and the screen size is (800.0, 600.0).
            assert_eq!(icon_button, Rect::from_ltrb(752.0, 552.0, 800.0, 600.0));

            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();

            let search_view_rect = tester.get_rect(
                find::descendant(view_content(), find::by_type::<ConstrainedBox>()).first(),
            );
            assert_eq!(
                search_view_rect,
                Rect::from_ltrb(440.0, 200.0, 800.0, 600.0)
            );
        });
    }

    // search_anchor_test.dart 'SearchAnchor should update suggestions on changes to search
    // controller'.
    #[test]
    fn search_anchor_updates_suggestions_on_changes_to_search_controller() {
        test_widgets(|tester| {
            let controller = SearchController::new(&mut tester.cell.borrow_mut());
            let suggestions = ["foo", "far", "bim"];
            let anchor = SearchAnchor::new(
                |_app, _context, _controller| Icon::new(Some(Icons::search())).into_widget(),
                move |app, _context, controller| {
                    let search_text = controller.text_value(app).to_lowercase();
                    if search_text.is_empty() {
                        return Task::ready(vec![
                            Center::new().child(Text::new("No Search")).into_widget(),
                        ]);
                    }
                    Task::ready(
                        suggestions
                            .iter()
                            .filter(|suggestion| suggestion.to_lowercase().contains(&search_text))
                            .map(|suggestion| {
                                let suggestion = *suggestion;
                                ListTile::new()
                                    .title(Text::new(suggestion))
                                    .trailing(IconButton::new(
                                        Some(Listener::new(move |app: &mut App| {
                                            controller.set_text(app, suggestion)
                                        })),
                                        Icon::new(Some(Icons::call_missed())),
                                    ))
                                    .on_tap(Listener::new(move |app: &mut App| {
                                        controller.close_view(app, Some(suggestion.to_string()))
                                    }))
                                    .into_widget()
                            })
                            .collect(),
                    )
                },
            )
            .search_controller(controller.clone());
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(
                        Align::new()
                            .alignment(AlignmentGeometry::TOP_CENTER)
                            .child(anchor),
                    )
                    .into_widget(),
            );

            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();

            let list_tile1 = || find::widget_with_text::<ListTile>("foo");
            let list_tile2 = || find::widget_with_text::<ListTile>("far");
            let list_tile3 = || find::widget_with_text::<ListTile>("bim");
            let text_widget = || find::widget_with_text::<Center>("No Search");
            let icon_in_list_tile1 =
                find::descendant(list_tile1(), find::by_icon(&Icons::call_missed()));

            expect(tester, &text_widget(), finds_one_widget());
            expect(tester, &list_tile1(), finds_nothing());
            expect(tester, &list_tile2(), finds_nothing());
            expect(tester, &list_tile3(), finds_nothing());

            tester.enter_text(find::by_type::<SearchBar>(), "f");
            tester.pump_and_settle();
            expect(tester, &text_widget(), finds_nothing());
            expect(tester, &list_tile1(), finds_one_widget());
            expect(tester, &list_tile2(), finds_one_widget());
            expect(tester, &list_tile3(), finds_nothing());

            tester.tap(icon_in_list_tile1);
            tester.pump_and_settle();
            assert_eq!(controller.handle().text_value(&tester.cell.borrow()), "foo");
            expect(tester, &text_widget(), finds_nothing());
            expect(tester, &list_tile1(), finds_one_widget());
            expect(tester, &list_tile2(), finds_nothing());
            expect(tester, &list_tile3(), finds_nothing());

            tester.tap(list_tile1());
            tester.pump_and_settle();
            assert!(!controller.handle().is_open(&tester.cell.borrow()));
            assert_eq!(controller.handle().text_value(&tester.cell.borrow()), "foo");
            expect(tester, &text_widget(), finds_nothing());
            expect(tester, &list_tile1(), finds_nothing());
            expect(tester, &list_tile2(), finds_nothing());
            expect(tester, &list_tile3(), finds_nothing());
        });
    }

    // search_anchor_test.dart 'SearchAnchor suggestionsBuilder property could be async'.
    #[test]
    fn search_anchor_suggestions_builder_could_be_async() {
        test_widgets(|tester| {
            let controller = SearchController::new(&mut tester.cell.borrow_mut());
            let suggestion = "suggestion text";
            let anchor = SearchAnchor::new(
                |_app, _context, _controller| Icon::new(Some(Icons::search())).into_widget(),
                move |app, _context, controller| {
                    let completer = Completer::new();
                    let future = completer.future();
                    let list_tile = ListTile::new()
                        .title(Text::new(suggestion))
                        .on_tap(Listener::new(move |app: &mut App| {
                            controller.close_view(app, Some(suggestion.to_string()))
                        }))
                        .into_widget();
                    completer.complete(app, vec![list_tile]);
                    app.spawn(async move |_cx| future.await)
                },
            )
            .search_controller(controller.clone());
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(
                        Align::new()
                            .alignment(AlignmentGeometry::TOP_CENTER)
                            .child(anchor),
                    )
                    .into_widget(),
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();

            expect(tester, &find::text(suggestion), finds_one_widget());
            tester.tap(find::text(suggestion));
            tester.pump_and_settle();

            assert!(!controller.handle().is_open(&tester.cell.borrow()));
            assert_eq!(
                controller.handle().text_value(&tester.cell.borrow()),
                suggestion
            );
        });
    }

    // search_anchor_test.dart 'SearchAnchor ignores out-of-order async suggestions'.
    #[test]
    fn search_anchor_ignores_out_of_order_async_suggestions() {
        test_widgets(|tester| {
            type Request = Completer<Vec<WidgetRef>>;
            let requests: Rc<RefCell<HashMap<String, Request>>> = Rc::default();
            let anchor = SearchAnchor::new(
                |_app, _context, _controller| Icon::new(Some(Icons::search())).into_widget(),
                {
                    let requests = Rc::clone(&requests);
                    move |app, _context, controller| {
                        let query = controller.text_value(app).to_string();
                        if query.is_empty() {
                            return Task::ready(Vec::new());
                        }
                        let request = Completer::new();
                        let future = request.future();
                        requests.borrow_mut().insert(query, request);
                        app.spawn(async move |_cx| future.await)
                    }
                },
            );
            pump_material_home(tester, None, anchor.into_widget());

            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();

            let text_field =
                || find::descendant(find::by_type::<SearchBar>(), find::by_type::<TextField>());
            tester.enter_text(text_field(), "a");
            tester.pump();
            tester.enter_text(text_field(), "ab");
            tester.pump();

            let complete = |tester: &mut WidgetTester, query: &str, text: &str| {
                let request = requests.borrow_mut().remove(query).expect("a request");
                request.complete(
                    &mut tester.cell.borrow_mut(),
                    vec![Text::new(text).into_widget()],
                );
                tester.cell.checkpoint();
                tester.pump_and_settle();
            };
            complete(tester, "ab", "ab-result");
            expect(tester, &find::text("ab-result"), finds_one_widget());

            complete(tester, "a", "a-result");
            expect(tester, &find::text("ab-result"), finds_one_widget());
            expect(tester, &find::text("a-result"), finds_nothing());
        });
    }

    // search_anchor_test.dart 'SearchAnchor.bar has a default search bar as the anchor'.
    #[test]
    fn search_anchor_bar_has_a_default_search_bar_as_the_anchor() {
        test_widgets(|tester| {
            let anchor = SearchAnchor::bar(|_app, _context, _controller| Task::ready(Vec::new()))
                .is_full_screen(false)
                .into_search_anchor();
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(
                        Align::new()
                            .alignment(AlignmentGeometry::TOP_LEFT)
                            .child(anchor),
                    )
                    .into_widget(),
            );

            expect(tester, &find::by_type::<SearchBar>(), finds_one_widget());
            let anchor_rect = tester.get_rect(find::by_type::<SearchBar>());
            assert_eq!(anchor_rect.size(), Size::new(800.0, 56.0));
            assert_eq!(anchor_rect, Rect::from_ltrb(0.0, 0.0, 800.0, 56.0));

            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();

            let search_view_rect = tester.get_rect(
                find::descendant(view_content(), find::by_type::<ConstrainedBox>()).first(),
            );
            assert_eq!(search_view_rect, Rect::from_ltrb(0.0, 0.0, 800.0, 400.0));

            // Search view has same width with the default anchor(search bar).
            assert_eq!(search_view_rect.width(), anchor_rect.width());
        });
    }

    // search_anchor_test.dart 'SearchAnchor viewOnClose function test'.
    #[test]
    fn search_anchor_view_on_close_function_test() {
        test_widgets(|tester| {
            let name = Rc::new(RefCell::new("silva".to_string()));
            let mut anchor = button_anchor_opening_view();
            anchor.view_on_close = Some(Listener::new({
                let name = Rc::clone(&name);
                move |_| *name.borrow_mut() = "Pedro".to_string()
            }));
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(Center::new().child(anchor))
                    .into_widget(),
            );

            // Open search view
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            assert_eq!(*name.borrow(), "silva");

            // Pop search view route
            tester.tap(find::by_type::<BackButton>());
            tester.pump_and_settle();
            assert_eq!(*name.borrow(), "Pedro");
        });
    }

    // search_anchor_test.dart 'SearchAnchor viewOnOpen is called when the search view is
    // opened'.
    #[test]
    fn search_anchor_view_on_open_is_called_when_the_search_view_is_opened() {
        test_widgets(|tester| {
            let opened = Rc::new(Cell::new(0));
            let mut anchor = button_anchor_opening_view();
            anchor.view_on_open = Some(Listener::new({
                let opened = Rc::clone(&opened);
                move |_| opened.set(opened.get() + 1)
            }));
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(Center::new().child(anchor))
                    .into_widget(),
            );

            assert_eq!(opened.get(), 0);
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            assert_eq!(opened.get(), 1);
        });
    }

    fn button_anchor_opening_view() -> SearchAnchor {
        SearchAnchor::new(
            |_app, _context, controller| {
                IconButton::new(
                    Some(Listener::new(move |app: &mut App| {
                        controller.open_view(app)
                    })),
                    Icon::new(Some(Icons::search())),
                )
                .into_widget()
            },
            |_app, _context, _controller| {
                Task::ready(
                    (0..5)
                        .map(|index| {
                            ListTile::new()
                                .leading(Icon::new(Some(Icons::history())))
                                .title(Text::new(format!("item {index}")))
                                .trailing(Icon::new(Some(Icons::chevron_right())))
                                .on_tap(Listener::new(|_| {}))
                                .into_widget()
                        })
                        .collect(),
                )
            },
        )
    }

    // search_anchor_test.dart 'Check SearchBar opacity when disabled'.
    #[test]
    fn search_bar_is_dimmed_when_disabled() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(Center::new().child(SearchBar::new().enabled(false)))
                    .into_widget(),
            );

            expect(tester, &find::by_type::<SearchBar>(), finds_one_widget());
            let opacity_finder =
                find::descendant(find::by_type::<SearchBar>(), find::by_type::<Opacity>());
            expect(tester, &opacity_finder, finds_one_widget());
            let opacity = tester.widget(opacity_finder, |opacity: &Opacity| opacity.opacity);
            assert_eq!(opacity, 0.38);
        });
    }

    // search_anchor_test.dart 'Check SearchAnchor opacity when disabled'.
    #[test]
    fn search_anchor_is_dimmed_when_disabled() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                Center::new()
                    .child(Material::new().child(search_icon_anchor().enabled(false)))
                    .into_widget(),
            );

            expect(tester, &find::by_type::<SearchAnchor>(), finds_one_widget());
            let opacity_finder = find::descendant(
                find::by_type::<SearchAnchor>(),
                find::by_type::<AnimatedOpacity>(),
            );
            expect(tester, &opacity_finder, finds_one_widget());
            let opacity =
                tester.widget(opacity_finder, |opacity: &AnimatedOpacity| opacity.opacity);
            assert_eq!(opacity, 0.38);
        });
    }

    use crate::card::Card;
    use crate::card_theme::CardThemeData;
    use crate::color_scheme::ColorScheme;
    use crate::icon_button_theme::IconButtonThemeData;
    use crate::list_tile::ListTileTitleAlignment;
    use crate::page::MaterialPageRoute;
    use crate::test_support::PaintInspection;
    use waft_painting::{
        BorderRadiusGeometry, CircleBorder, ContinuousRectangleBorder, RoundedRectangleBorder,
        ShapeBorderRef,
    };
    use waft_rendering::RenderBox;
    use waft_rendering::testing::{PaintCall, assert_paints};
    use waft_widgets::SizedBox;
    use waft_widgets::testing::TestPointer;
    use waft_widgets::{GridView, WidgetState, WidgetStateProperty, WidgetStates};

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Probe {
        Hovered,
        Pressed,
        Focused,
    }

    fn pressed_color() -> Color {
        Colors::RED.primary()
    }
    fn hovered_color() -> Color {
        Colors::ORANGE.primary()
    }
    fn focused_color() -> Color {
        Colors::YELLOW.primary()
    }
    fn default_color() -> Color {
        Colors::GREEN.primary()
    }

    /// Dart's `_getColor`.
    fn get_color(states: &WidgetStates) -> Color {
        if states.contains(&WidgetState::Pressed) {
            pressed_color()
        } else if states.contains(&WidgetState::Hovered) {
            hovered_color()
        } else if states.contains(&WidgetState::Focused) {
            focused_color()
        } else {
            default_color()
        }
    }

    fn color_property() -> WidgetStatePropertyRef<Option<Color>> {
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| Some(get_color(states)))
    }

    /// Dart's `_getTextStyle`, over `bodyLarge` of the default theme.
    fn text_style_property(tester: &WidgetTester) -> WidgetStatePropertyRef<Option<TextStyle>> {
        let body_large = m3(tester)
            .text_theme
            .body_large
            .clone()
            .expect("a bodyLarge");
        <dyn WidgetStateProperty<Option<TextStyle>>>::resolve_with(move |states| {
            if states.contains(&WidgetState::Pressed) {
                Some(body_large.copy_with().color(pressed_color()))
            } else if states.contains(&WidgetState::Hovered) {
                Some(body_large.copy_with().color(hovered_color()))
            } else if states.contains(&WidgetState::Focused) {
                Some(body_large.copy_with().color(focused_color()))
            } else {
                None
            }
        })
    }

    /// Dart's `_pointGestureToSearchBar`: a mouse hovering the middle of the bar.
    fn point_gesture_to_search_bar(tester: &mut WidgetTester) -> TestPointer {
        let center = tester.get_center(find::by_type::<SearchBar>());
        let mut gesture = TestPointer::mouse(tester.next_pointer(), 1);
        tester.add_pointer(&mut gesture, center);
        tester.move_to(&mut gesture, center, false);
        gesture
    }

    /// The hover, press and focus steps every state-driven SearchBar property case walks;
    /// `settle` is where Dart pumps until settled instead of once.
    fn walk_states(
        tester: &mut WidgetTester,
        settle: bool,
        mut check: impl FnMut(&mut WidgetTester, Probe),
    ) {
        let mut gesture = point_gesture_to_search_bar(tester);
        if settle {
            tester.pump_and_settle();
        } else {
            tester.pump();
        }
        check(tester, Probe::Hovered);

        let center = tester.get_center(find::by_type::<SearchBar>());
        tester.down(&mut gesture, center);
        tester.pump_and_settle();
        check(tester, Probe::Pressed);

        tester.up(&gesture);
        if settle {
            tester.pump_and_settle();
        } else {
            tester.pump();
        }
        tester.remove_pointer(&gesture);
        tester.pump();
        check(tester, Probe::Focused);
    }

    fn pump_centered_bar(tester: &mut WidgetTester, bar: SearchBar) {
        pump_material_home(
            tester,
            None,
            Center::new()
                .child(Material::new().child(bar))
                .into_widget(),
        );
    }

    fn pick<T>(probe: Probe, pressed: T, hovered: T, focused: T) -> T {
        match probe {
            Probe::Pressed => pressed,
            Probe::Hovered => hovered,
            Probe::Focused => focused,
        }
    }

    // 'SearchBar focusNode is hot swappable'.
    #[test]
    fn search_bar_focus_node_is_hot_swappable() {
        test_widgets(|tester| {
            let has_focus = |tester: &WidgetTester, node: &Ref<FocusNode>| {
                node.handle().has_focus(&mut tester.cell.borrow_mut())
            };
            let node1 = FocusNode::new(&mut tester.cell.borrow_mut());
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(SearchBar::new().focus_node(node1.erase()))
                    .into_widget(),
            );
            assert!(!has_focus(tester, &node1));
            node1
                .handle()
                .request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump();
            assert!(has_focus(tester, &node1));
            node1
                .handle()
                .unfocus(&mut tester.cell.borrow_mut(), UnfocusDisposition::Scope);
            tester.pump();
            assert!(!has_focus(tester, &node1));

            let node2 = FocusNode::new(&mut tester.cell.borrow_mut());
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(SearchBar::new().focus_node(node2.erase()))
                    .into_widget(),
            );
            assert!(!has_focus(tester, &node1));
            assert!(!has_focus(tester, &node2));
            node2
                .handle()
                .request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump();
            assert!(!has_focus(tester, &node1));
            assert!(has_focus(tester, &node2));
            node2
                .handle()
                .unfocus(&mut tester.cell.borrow_mut(), UnfocusDisposition::Scope);
            tester.pump();
            assert!(!has_focus(tester, &node1));
            assert!(!has_focus(tester, &node2));

            pump_material_home(
                tester,
                None,
                Material::new().child(SearchBar::new()).into_widget(),
            );
            assert!(!has_focus(tester, &node1));
            assert!(!has_focus(tester, &node2));
            tester.tap(find::by_type::<SearchBar>());
            tester.pump();
            assert!(!has_focus(tester, &node1));
            assert!(!has_focus(tester, &node2));
        });
    }

    // 'SearchBar respects leading property'.
    #[test]
    fn search_bar_respects_leading_property() {
        test_widgets(|tester| {
            let color_scheme = m3(tester).color_scheme.clone();
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(SearchBar::new().leading(IconButton::new(
                        Some(Listener::new(|_| {})),
                        Icon::new(Some(Icons::search())),
                    )))
                    .into_widget(),
            );
            expect(
                tester,
                &find::widget_with_icon::<IconButton>(&Icons::search()),
                finds_one_widget(),
            );
            let icon = tester.element(find::by_icon(&Icons::search()));
            let color = tester.glyph_style(icon).color.map(|color| color.color());
            assert_eq!(color, Some(color_scheme.on_surface));
        });
    }

    // 'SearchBar respects trailing property'.
    #[test]
    fn search_bar_respects_trailing_property() {
        test_widgets(|tester| {
            let color_scheme = m3(tester).color_scheme.clone();
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(SearchBar::new().trailing(vec![
                        IconButton::new(
                            Some(Listener::new(|_| {})),
                            Icon::new(Some(Icons::menu())),
                        )
                        .into_widget(),
                    ]))
                    .into_widget(),
            );
            expect(
                tester,
                &find::widget_with_icon::<IconButton>(&Icons::menu()),
                finds_one_widget(),
            );
            let icon = tester.element(find::by_icon(&Icons::menu()));
            let color = tester.glyph_style(icon).color.map(|color| color.color());
            assert_eq!(color, Some(color_scheme.on_surface_variant()));
        });
    }

    // 'SearchBar respects constraints property'.
    #[test]
    fn search_bar_respects_constraints_property() {
        test_widgets(|tester| {
            let mut constraints = BoxConstraints::new();
            constraints.max_width = 350.0;
            constraints.min_height = 80.0;
            pump_centered_bar(tester, SearchBar::new().constraints(constraints));
            let bar_rect = tester.get_rect(find::by_type::<SearchBar>());
            assert_eq!(bar_rect.size(), Size::new(350.0, 80.0));
        });
    }

    // 'SearchBar respects elevation property'.
    #[test]
    fn search_bar_respects_elevation_property() {
        test_widgets(|tester| {
            pump_centered_bar(
                tester,
                SearchBar::new().elevation(<dyn WidgetStateProperty<Option<f64>>>::resolve_with(
                    |states| {
                        Some(if states.contains(&WidgetState::Pressed) {
                            0.0
                        } else if states.contains(&WidgetState::Hovered) {
                            1.0
                        } else if states.contains(&WidgetState::Focused) {
                            2.0
                        } else {
                            3.0
                        })
                    },
                )),
            );
            walk_states(tester, false, |tester, probe| {
                assert_eq!(
                    search_bar_material(tester).elevation,
                    pick(probe, 0.0, 1.0, 2.0)
                );
            });
        });
    }

    // 'SearchBar respects backgroundColor property'.
    #[test]
    fn search_bar_respects_background_color_property() {
        test_widgets(|tester| {
            pump_centered_bar(tester, SearchBar::new().background_color(color_property()));
            walk_states(tester, false, |tester, probe| {
                assert_eq!(
                    search_bar_material(tester).color,
                    Some(pick(
                        probe,
                        pressed_color(),
                        hovered_color(),
                        focused_color()
                    ))
                );
            });
        });
    }

    // 'SearchBar respects shadowColor property'.
    #[test]
    fn search_bar_respects_shadow_color_property() {
        test_widgets(|tester| {
            pump_centered_bar(tester, SearchBar::new().shadow_color(color_property()));
            walk_states(tester, false, |tester, probe| {
                assert_eq!(
                    search_bar_material(tester).shadow_color,
                    Some(pick(
                        probe,
                        pressed_color(),
                        hovered_color(),
                        focused_color()
                    ))
                );
            });
        });
    }

    // 'SearchBar respects surfaceTintColor property'.
    #[test]
    fn search_bar_respects_surface_tint_color_property() {
        test_widgets(|tester| {
            pump_centered_bar(
                tester,
                SearchBar::new().surface_tint_color(color_property()),
            );
            walk_states(tester, false, |tester, probe| {
                assert_eq!(
                    search_bar_material(tester).surface_tint_color,
                    Some(pick(
                        probe,
                        pressed_color(),
                        hovered_color(),
                        focused_color()
                    ))
                );
            });
        });
    }

    // 'SearchBar respects overlayColor property'.
    #[test]
    fn search_bar_respects_overlay_color_property() {
        test_widgets(|tester| {
            let node = FocusNode::new(&mut tester.cell.borrow_mut());
            pump_centered_bar(
                tester,
                SearchBar::new()
                    .focus_node(node.erase())
                    .overlay_color(color_property()),
            );
            let ink_ops = |tester: &WidgetTester| {
                let ink_features = tester.first::<crate::material::RenderInkFeatures>();
                tester.painted_ops_deep(ink_features.as_object())
            };
            walk_states(tester, true, |tester, probe| {
                let color = pick(probe, pressed_color(), hovered_color(), focused_color());
                let ops = ink_ops(tester);
                match probe {
                    Probe::Hovered => assert_paints(&ops, &[PaintCall::rect(Some(color))]),
                    _ => {
                        assert_paints(&ops, &[PaintCall::rect(None), PaintCall::rect(Some(color))])
                    }
                }
            });
        });
    }

    // 'SearchBar respects side and shape properties'.
    #[test]
    fn search_bar_respects_side_and_shape_properties() {
        test_widgets(|tester| {
            let side = |width: f64| BorderSide {
                width,
                ..BorderSide::default()
            };
            let pressed_shape = |side| {
                ShapeBorderRef::new(RoundedRectangleBorder::new(
                    side,
                    BorderRadiusGeometry::ZERO,
                ))
            };
            let hovered_shape = |side| {
                ShapeBorderRef::new(ContinuousRectangleBorder::new(
                    side,
                    BorderRadiusGeometry::ZERO,
                ))
            };
            let focused_shape = |side| ShapeBorderRef::new(CircleBorder::new(side, 0.0));
            pump_centered_bar(
                tester,
                SearchBar::new()
                    .side(<dyn WidgetStateProperty<Option<BorderSide>>>::resolve_with(
                        move |states| {
                            Some(if states.contains(&WidgetState::Pressed) {
                                side(2.0)
                            } else if states.contains(&WidgetState::Hovered) {
                                side(3.0)
                            } else if states.contains(&WidgetState::Focused) {
                                side(4.0)
                            } else {
                                side(5.0)
                            })
                        },
                    ))
                    .shape(
                        <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::resolve_with(
                            |states| {
                                Some(if states.contains(&WidgetState::Pressed) {
                                    RoundedRectangleBorder::default().into()
                                } else if states.contains(&WidgetState::Hovered) {
                                    ContinuousRectangleBorder::default().into()
                                } else if states.contains(&WidgetState::Focused) {
                                    CircleBorder::default().into()
                                } else {
                                    StadiumBorder::default().into()
                                })
                            },
                        ),
                    ),
            );
            walk_states(tester, false, |tester, probe| {
                let expected = match probe {
                    Probe::Pressed => pressed_shape(side(2.0)),
                    Probe::Hovered => hovered_shape(side(3.0)),
                    Probe::Focused => focused_shape(side(4.0)),
                };
                assert_eq!(search_bar_material(tester).shape, Some(expected));
            });
        });
    }

    // 'SearchBar respects padding property'.
    #[test]
    fn search_bar_respects_padding_property() {
        test_widgets(|tester| {
            pump_centered_bar(
                tester,
                SearchBar::new()
                    .leading(Icon::new(Some(Icons::search())))
                    .padding(<dyn WidgetStateProperty<Option<EdgeInsetsGeometry>>>::all(
                        Some(EdgeInsets::all(16.0).into()),
                    ))
                    .trailing(vec![Icon::new(Some(Icons::menu())).into_widget()]),
            );
            let bar_rect = tester.get_rect(find::by_type::<SearchBar>());
            let leading_rect = tester.get_rect(find::by_icon(&Icons::search()));
            let text_field_rect = tester.get_rect(find::by_type::<TextField>());
            let trailing_rect = tester.get_rect(find::by_icon(&Icons::menu()));
            assert_eq!(bar_rect.left, leading_rect.left - 16.0);
            assert_eq!(leading_rect.right, text_field_rect.left - 16.0);
            assert_eq!(text_field_rect.right, trailing_rect.left - 16.0);
            assert_eq!(trailing_rect.right, bar_rect.right - 16.0);
        });
    }

    // 'SearchBar respects hintStyle property'.
    #[test]
    fn search_bar_respects_hint_style_property() {
        test_widgets(|tester| {
            let style = text_style_property(tester);
            pump_centered_bar(
                tester,
                SearchBar::new().hint_text("hint text").hint_style(style),
            );
            walk_states(tester, false, |tester, probe| {
                let color = tester
                    .widget(find::text("hint text"), |text: &Text| text.style.clone())
                    .and_then(|style| style.color)
                    .map(|color| color.color());
                assert_eq!(
                    color,
                    Some(pick(
                        probe,
                        pressed_color(),
                        hovered_color(),
                        focused_color()
                    ))
                );
            });
        });
    }

    // 'SearchBar respects textStyle property'.
    #[test]
    fn search_bar_respects_text_style_property() {
        test_widgets(|tester| {
            let controller = {
                let mut app = tester.cell.borrow_mut();
                TextEditingController::new(&mut app).text(&mut app, "input text")
            };
            let style = text_style_property(tester);
            pump_centered_bar(
                tester,
                SearchBar::new()
                    .controller(controller.erase())
                    .text_style(style),
            );
            walk_states(tester, false, |tester, probe| {
                let color = tester
                    .widget(find::text("input text"), |text: &EditableText| {
                        text.style.clone()
                    })
                    .color
                    .map(|color| color.color());
                assert_eq!(
                    color,
                    Some(pick(
                        probe,
                        pressed_color(),
                        hovered_color(),
                        focused_color()
                    ))
                );
            });
        });
    }

    fn empty_suggestions()
    -> impl Fn(&mut App, BuildContext, Handle<SearchController>) -> Task<Vec<WidgetRef>> {
        |_app, _context, _controller| Task::ready(Vec::new())
    }

    fn text_field_in_view() -> waft_widgets::testing::Finder {
        find::descendant(view_content(), find::by_type::<TextField>())
    }

    fn text_field_focus_node(
        tester: &WidgetTester,
        finder: waft_widgets::testing::Finder,
    ) -> FocusNodeRef {
        tester
            .widget(finder, |field: &TextField| field.focus_node.clone())
            .expect("the text field has a focus node")
    }

    // 'SearchAnchor respects viewOnChanged and viewOnSubmitted properties'.
    #[test]
    fn search_anchor_respects_view_on_changed_and_view_on_submitted_properties() {
        test_widgets(|tester| {
            let controller = SearchController::new(&mut tester.cell.borrow_mut());
            let handle = controller.handle();
            let changed = Rc::new(Cell::new(0));
            let submitted = Rc::new(Cell::new(0));
            let anchor = SearchAnchor::new(
                |_app, _context, controller| {
                    SearchBar::new()
                        .on_tap(Listener::new(move |app: &mut App| {
                            if !controller.is_open(app) {
                                controller.open_view(app);
                            }
                        }))
                        .into_widget()
                },
                empty_suggestions(),
            )
            .search_controller(controller.clone())
            .view_on_changed({
                let changed = Rc::clone(&changed);
                move |_, _| changed.set(changed.get() + 1)
            })
            .view_on_submitted({
                let submitted = Rc::clone(&submitted);
                move |app: &mut App, value| {
                    submitted.set(submitted.get() + 1);
                    handle.close_view(app, Some(value));
                }
            });
            pump_centered_material(tester, anchor);
            tester.tap(find::by_type::<SearchBar>());
            tester.pump_and_settle();
            assert!(controller.handle().is_open(&tester.cell.borrow()));

            tester.enter_text(text_field_in_view(), "a");
            assert_eq!(changed.get(), 1);
            tester.enter_text(text_field_in_view(), "abc");
            assert_eq!(changed.get(), 2);

            tester
                .test_text_input()
                .receive_action(TextInputAction::Done);
            assert_eq!(submitted.get(), 1);
            assert!(!controller.handle().is_open(&tester.cell.borrow()));
        });
    }

    fn pump_centered_material<K>(tester: &mut WidgetTester, child: impl IntoWidget<K>) {
        pump_material_home(
            tester,
            None,
            Center::new()
                .child(Material::new().child(child))
                .into_widget(),
        );
    }

    // 'SearchAnchor.bar respects onChanged and onSubmitted properties'.
    #[test]
    fn search_anchor_bar_respects_on_changed_and_on_submitted_properties() {
        test_widgets(|tester| {
            let controller = SearchController::new(&mut tester.cell.borrow_mut());
            let handle = controller.handle();
            let changed = Rc::new(Cell::new(0));
            let submitted = Rc::new(Cell::new(0));
            let anchor = SearchAnchor::bar(empty_suggestions())
                .search_controller(controller.clone())
                .on_submitted({
                    let submitted = Rc::clone(&submitted);
                    move |app: &mut App, value| {
                        submitted.set(submitted.get() + 1);
                        handle.close_view(app, Some(value));
                    }
                })
                .on_changed({
                    let changed = Rc::clone(&changed);
                    move |_, _| changed.set(changed.get() + 1)
                });
            pump_centered_material(tester, anchor.into_search_anchor());
            tester.tap(find::by_type::<SearchBar>());
            tester.pump_and_settle();
            assert!(controller.handle().is_open(&tester.cell.borrow()));

            tester.enter_text(text_field_in_view(), "a");
            assert_eq!(changed.get(), 1);
            tester.enter_text(text_field_in_view(), "abc");
            assert_eq!(changed.get(), 2);

            tester
                .test_text_input()
                .receive_action(TextInputAction::Done);
            assert_eq!(submitted.get(), 1);
            assert!(!controller.handle().is_open(&tester.cell.borrow()));
        });
    }

    // 'SearchAnchor.bar anchor loses focus after view closes'.
    #[test]
    fn search_anchor_bar_anchor_loses_focus_after_view_closes() {
        test_widgets(|tester| {
            let controller = SearchController::new(&mut tester.cell.borrow_mut());
            let handle = controller.handle();
            let submitted = Rc::new(Cell::new(0));
            let anchor = SearchAnchor::bar(empty_suggestions())
                .search_controller(controller.clone())
                .on_submitted({
                    let submitted = Rc::clone(&submitted);
                    move |app: &mut App, value| {
                        submitted.set(submitted.get() + 1);
                        handle.close_view(app, Some(value));
                    }
                });
            pump_centered_material(tester, anchor.into_search_anchor());
            tester.tap(find::by_type::<SearchBar>());
            tester.pump_and_settle();
            assert!(controller.handle().is_open(&tester.cell.borrow()));

            let anchor_text_field = find::descendant(
                find::by_type::<SearchBar>().first(),
                find::by_type::<TextField>(),
            );
            let view_node = text_field_focus_node(tester, text_field_in_view());
            assert!(view_node.handle().has_focus(&mut tester.cell.borrow_mut()));

            tester
                .test_text_input()
                .receive_action(TextInputAction::Done);
            tester.pump_and_settle();
            assert_eq!(submitted.get(), 1);
            assert!(!controller.handle().is_open(&tester.cell.borrow()));

            let anchor_node = text_field_focus_node(tester, anchor_text_field);
            assert!(
                !anchor_node
                    .handle()
                    .has_focus(&mut tester.cell.borrow_mut())
            );
        });
    }

    // 'hintStyle can override textStyle for hintText'.
    #[test]
    fn hint_style_can_override_text_style_for_hint_text() {
        test_widgets(|tester| {
            let style = text_style_property(tester);
            pump_centered_bar(
                tester,
                SearchBar::new()
                    .hint_text("hint text")
                    .hint_style(style)
                    .text_style(<dyn WidgetStateProperty<Option<TextStyle>>>::all(Some(
                        TextStyle::new().color(Colors::PINK.primary()),
                    ))),
            );
            walk_states(tester, false, |tester, probe| {
                let color = tester
                    .widget(find::text("hint text"), |text: &Text| text.style.clone())
                    .and_then(|style| style.color)
                    .map(|color| color.color());
                assert_eq!(
                    color,
                    Some(pick(
                        probe,
                        pressed_color(),
                        hovered_color(),
                        focused_color()
                    ))
                );
            });
        });
    }

    // 'The text is still centered when SearchBar text field is smaller than 48'.
    #[test]
    fn the_text_is_still_centered_when_the_search_bar_text_field_is_smaller_than_48() {
        test_widgets(|tester| {
            let mut constraints = BoxConstraints::new();
            constraints.min_height = 35.0;
            constraints.max_height = 35.0;
            pump_centered_bar(tester, SearchBar::new().constraints(constraints));
            tester.enter_text(find::by_type::<TextField>(), "input text");
            let text_center_y = tester.get_center(find::text("input text")).dy();
            let search_bar_center_y = tester.get_center(find::by_type::<SearchBar>()).dy();
            assert_eq!(text_center_y, search_bar_center_y);
        });
    }

    /// The anchor most view cases build: an icon button that opens the view, with no suggestions.
    fn view_anchor() -> SearchAnchor {
        SearchAnchor::new(
            |_app, _context, controller| {
                IconButton::new(
                    Some(Listener::new(move |app: &mut App| {
                        controller.open_view(app)
                    })),
                    Icon::new(Some(Icons::search())),
                )
                .into_widget()
            },
            empty_suggestions(),
        )
    }

    fn open_view_of(tester: &mut WidgetTester, anchor: SearchAnchor) {
        pump_material_home(tester, None, Material::new().child(anchor).into_widget());
        tester.tap(find::widget_with_icon::<IconButton>(&Icons::search()));
        tester.pump_and_settle();
    }

    fn view_divider_color(tester: &WidgetTester) -> Color {
        tester.widget(
            find::descendant(find::by_type::<Divider>(), find::by_type::<Container>()).first(),
            |container: &Container| {
                container
                    .decoration
                    .as_ref()
                    .expect("the container has a decoration")
                    .as_any()
                    .downcast_ref::<BoxDecoration>()
                    .expect("a BoxDecoration")
                    .border
                    .as_ref()
                    .expect("the decoration has a border")
                    .as_any()
                    .downcast_ref::<Border>()
                    .expect("a Border")
                    .bottom
                    .color
            },
        )
    }

    fn first_view_padding(tester: &WidgetTester) -> EdgeInsetsGeometry {
        tester.widget(
            find::descendant(view_content(), find::by_type::<Padding>()).first(),
            |padding: &Padding| padding.padding,
        )
    }

    // 'SearchAnchor respects viewBuilder property'.
    #[test]
    fn search_anchor_respects_view_builder_property() {
        test_widgets(|tester| {
            open_view_of(tester, view_anchor());
            // Default is a ListView.
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());

            tester.pump_widget(Container::new());
            open_view_of(
                tester,
                view_anchor().view_builder(|_app, suggestions| {
                    GridView::count(5).children(suggestions).into_widget()
                }),
            );
            expect(tester, &find::by_type::<ListView>(), finds_nothing());
            expect(tester, &find::by_type::<GridView>(), finds_one_widget());
        });
    }

    // 'SearchAnchor.bar respects viewBuilder property'.
    #[test]
    fn search_anchor_bar_respects_view_builder_property() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(SearchAnchor::bar(empty_suggestions()).into_search_anchor())
                    .into_widget(),
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());

            tester.pump_widget(Container::new());
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(
                        SearchAnchor::bar(empty_suggestions())
                            .view_builder(|_app, suggestions| {
                                GridView::count(5).children(suggestions).into_widget()
                            })
                            .into_search_anchor(),
                    )
                    .into_widget(),
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            expect(tester, &find::by_type::<ListView>(), finds_nothing());
            expect(tester, &find::by_type::<GridView>(), finds_one_widget());
        });
    }

    // 'SearchAnchor respects viewLeading property'.
    #[test]
    fn search_anchor_respects_view_leading_property() {
        test_widgets(|tester| {
            open_view_of(tester, view_anchor());
            // Default is an icon button with arrow_back.
            expect(tester, &find::by_type::<BackButton>(), finds_one_widget());

            tester.pump_widget(Container::new());
            open_view_of(
                tester,
                view_anchor().view_leading(Icon::new(Some(Icons::history()))),
            );
            expect(tester, &find::by_type::<BackButton>(), finds_nothing());
            expect(
                tester,
                &find::by_icon(&Icons::history()),
                finds_one_widget(),
            );
        });
    }

    // 'SearchAnchor respects viewTrailing property'.
    #[test]
    fn search_anchor_respects_view_trailing_property() {
        test_widgets(|tester| {
            open_view_of(tester, view_anchor());
            // Default is an icon button with a close icon when the input is not empty.
            tester.enter_text(find::by_type::<TextField>(), "a");
            tester.pump();
            expect(
                tester,
                &find::widget_with_icon::<IconButton>(&Icons::close()),
                finds_one_widget(),
            );

            tester.pump_widget(Container::new());
            open_view_of(
                tester,
                view_anchor().view_trailing(vec![Icon::new(Some(Icons::history())).into_widget()]),
            );
            expect(tester, &find::by_icon(&Icons::close()), finds_nothing());
            expect(
                tester,
                &find::by_icon(&Icons::history()),
                finds_one_widget(),
            );
        });
    }

    // 'SearchAnchor respects viewHintText property'.
    #[test]
    fn search_anchor_respects_view_hint_text_property() {
        test_widgets(|tester| {
            open_view_of(tester, view_anchor().view_hint_text("hint text"));
            expect(tester, &find::text("hint text"), finds_one_widget());
        });
    }

    // 'SearchAnchor respects viewBackgroundColor property'.
    #[test]
    fn search_anchor_respects_view_background_color_property() {
        test_widgets(|tester| {
            open_view_of(
                tester,
                view_anchor().view_background_color(Colors::PURPLE.primary()),
            );
            assert_eq!(
                search_view_material(tester).color,
                Some(Colors::PURPLE.primary())
            );
        });
    }

    // 'SearchAnchor respects viewElevation property'.
    #[test]
    fn search_anchor_respects_view_elevation_property() {
        test_widgets(|tester| {
            open_view_of(tester, view_anchor().view_elevation(3.0));
            assert_eq!(search_view_material(tester).elevation, 3.0);
        });
    }

    // 'SearchAnchor respects viewSurfaceTint property'.
    #[test]
    fn search_anchor_respects_view_surface_tint_property() {
        test_widgets(|tester| {
            open_view_of(
                tester,
                view_anchor().view_surface_tint_color(Colors::PURPLE.primary()),
            );
            assert_eq!(
                search_view_material(tester).surface_tint_color,
                Some(Colors::PURPLE.primary())
            );
        });
    }

    // 'SearchAnchor respects viewSide property'.
    #[test]
    fn search_anchor_respects_view_side_property() {
        test_widgets(|tester| {
            let side = BorderSide {
                color: Colors::PURPLE.primary(),
                width: 5.0,
                ..BorderSide::default()
            };
            open_view_of(tester, view_anchor().is_full_screen(false).view_side(side));
            assert_eq!(
                search_view_material(tester).shape,
                Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
                    side,
                    BorderRadiusGeometry::circular(28.0),
                )))
            );
        });
    }

    // 'SearchAnchor respects viewShape property'.
    #[test]
    fn search_anchor_respects_view_shape_property() {
        test_widgets(|tester| {
            let side = BorderSide {
                color: Colors::PURPLE.primary(),
                width: 5.0,
                ..BorderSide::default()
            };
            let shape = StadiumBorder::new(side);
            open_view_of(
                tester,
                view_anchor().is_full_screen(false).view_shape(shape),
            );
            assert_eq!(
                search_view_material(tester).shape,
                Some(ShapeBorderRef::new(shape))
            );
        });
    }

    // 'SearchAnchor respects headerTextStyle property'.
    #[test]
    fn search_anchor_respects_header_text_style_property() {
        test_widgets(|tester| {
            let body_large = m3(tester).text_theme.body_large.clone().expect("bodyLarge");
            open_view_of(
                tester,
                view_anchor()
                    .header_text_style(body_large.copy_with().color(Colors::RED.primary())),
            );
            tester.enter_text(find::by_type::<SearchBar>(), "input text");
            tester.pump_and_settle();
            let style = tester.widget(find::text("input text"), |text: &EditableText| {
                text.style.clone()
            });
            assert_eq!(
                style.color.map(|color| color.color()),
                Some(Colors::RED.primary())
            );
        });
    }

    // 'SearchAnchor respects headerHintStyle property'.
    #[test]
    fn search_anchor_respects_header_hint_style_property() {
        test_widgets(|tester| {
            let body_large = m3(tester).text_theme.body_large.clone().expect("bodyLarge");
            open_view_of(
                tester,
                view_anchor()
                    .view_hint_text("hint text")
                    .header_hint_style(body_large.copy_with().color(Colors::ORANGE.primary())),
            );
            let style = tester
                .widget(find::text("hint text"), |text: &Text| text.style.clone())
                .expect("a style");
            assert_eq!(
                style.color.map(|color| color.color()),
                Some(Colors::ORANGE.primary())
            );
        });
    }

    // 'SearchAnchor respects viewPadding property'.
    #[test]
    fn search_anchor_respects_view_padding_property() {
        test_widgets(|tester| {
            open_view_of(
                tester,
                view_anchor()
                    .is_full_screen(false)
                    .view_padding(EdgeInsets::all(16.0)),
            );
            assert_eq!(first_view_padding(tester), EdgeInsets::all(16.0).into());
        });
    }

    // 'SearchAnchor ignores viewPadding property if full screen'.
    #[test]
    fn search_anchor_ignores_view_padding_property_if_full_screen() {
        test_widgets(|tester| {
            open_view_of(
                tester,
                view_anchor()
                    .is_full_screen(true)
                    .view_padding(EdgeInsets::all(16.0)),
            );
            assert_eq!(first_view_padding(tester), EdgeInsets::ZERO.into());
        });
    }

    // 'SearchAnchor respects shrinkWrap property'.
    #[test]
    fn search_anchor_respects_shrink_wrap_property() {
        test_widgets(|tester| {
            let anchor = SearchAnchor::new(
                |_app, _context, controller| {
                    IconButton::new(
                        Some(Listener::new(move |app: &mut App| {
                            controller.open_view(app)
                        })),
                        Icon::new(Some(Icons::search())),
                    )
                    .into_widget()
                },
                |app, _context, controller| {
                    let length = controller.text_value(app).chars().count();
                    Task::ready(
                        (0..length)
                            .map(|index| {
                                ListTile::new()
                                    .title(Text::new(format!("Item {index}")))
                                    .into_widget()
                            })
                            .collect(),
                    )
                },
            )
            .is_full_screen(false)
            .shrink_wrap(true)
            .view_constraints(BoxConstraints::new());
            open_view_of(tester, anchor);

            let divider = find::descendant(view_content(), find::by_type::<Divider>());
            // The divider is not shown if there are no suggestions.
            expect(tester, &divider, finds_nothing());
            let material = find::descendant(view_content(), find::by_type::<Material>()).first();
            assert_eq!(
                tester.get_rect(material.clone()),
                Rect::from_ltrb(0.0, 0.0, 800.0, 56.0)
            );

            tester.enter_text(find::by_type::<SearchBar>(), "a");
            tester.pump_and_settle();
            expect(tester, &divider, finds_one_widget());
            assert_eq!(
                tester.get_rect(material),
                Rect::from_ltrb(0.0, 0.0, 800.0, 113.0)
            );
        });
    }

    // 'SearchAnchor respects dividerColor property'.
    #[test]
    fn search_anchor_respects_divider_color_property() {
        test_widgets(|tester| {
            open_view_of(tester, view_anchor().divider_color(Colors::RED.primary()));
            assert_eq!(view_divider_color(tester), Colors::RED.primary());
        });
    }

    // 'SearchAnchor respects viewConstraints property'.
    #[test]
    fn search_anchor_respects_view_constraints_property() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(
                        Center::new().child(
                            view_anchor()
                                .is_full_screen(false)
                                .view_constraints(BoxConstraints::tight(Size::new(280.0, 390.0))),
                        ),
                    )
                    .into_widget(),
            );
            tester.tap(find::widget_with_icon::<IconButton>(&Icons::search()));
            tester.pump_and_settle();
            let size = search_view_size(tester);
            assert_eq!(size.width(), 280.0);
            assert_eq!(size.height(), 390.0);
        });
    }

    // 'SearchAnchor respects viewBarPadding property'.
    #[test]
    fn search_anchor_respects_view_bar_padding_property() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(
                        Center::new().child(
                            view_anchor().view_bar_padding(EdgeInsets::symmetric(0.0, 16.0)),
                        ),
                    )
                    .into_widget(),
            );
            tester.tap(find::widget_with_icon::<IconButton>(&Icons::search()));
            tester.pump_and_settle();
            let search_bar = find::descendant(view_content(), find::by_type::<SearchBar>()).first();
            let padding = tester.widget(
                find::descendant(search_bar, find::by_type::<Padding>()).first(),
                |padding: &Padding| padding.padding,
            );
            assert_eq!(padding, EdgeInsets::symmetric(0.0, 16.0).into());
        });
    }

    fn top_center_anchor(direction: TextDirection) -> WidgetRef {
        Directionality::new(
            direction,
            Material::new().child(
                Align::new().alignment(AlignmentGeometry::TOP_CENTER).child(
                    SearchAnchor::new(
                        |_app, _context, _controller| {
                            Icon::new(Some(Icons::search())).into_widget()
                        },
                        empty_suggestions(),
                    )
                    .is_full_screen(false),
                ),
            ),
        )
        .into_widget()
    }

    // 'SearchAnchor respects builder property - LTR'.
    #[test]
    fn search_anchor_respects_builder_property_ltr() {
        test_widgets(|tester| {
            pump_material_home(tester, None, top_center_anchor(TextDirection::Ltr));
            let anchor_rect = tester.get_rect(find::by_icon(&Icons::search()));
            assert_eq!(anchor_rect.size(), Size::new(24.0, 24.0));
            assert_eq!(anchor_rect, Rect::from_ltrb(388.0, 0.0, 412.0, 24.0));

            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            let search_view_rect = tester.get_rect(
                find::descendant(view_content(), find::by_type::<ConstrainedBox>()).first(),
            );
            assert_eq!(search_view_rect, Rect::from_ltrb(388.0, 0.0, 748.0, 400.0));
            // The search view's top left is the anchor's.
            assert_eq!(search_view_rect.top_left(), anchor_rect.top_left());
        });
    }

    // 'SearchAnchor respects builder property - RTL'.
    #[test]
    fn search_anchor_respects_builder_property_rtl() {
        test_widgets(|tester| {
            pump_material_home(tester, None, top_center_anchor(TextDirection::Rtl));
            let anchor_rect = tester.get_rect(find::by_icon(&Icons::search()));
            assert_eq!(anchor_rect.size(), Size::new(24.0, 24.0));
            assert_eq!(anchor_rect, Rect::from_ltrb(388.0, 0.0, 412.0, 24.0));

            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            let search_view_rect = tester.get_rect(
                find::descendant(view_content(), find::by_type::<ConstrainedBox>()).first(),
            );
            assert_eq!(search_view_rect, Rect::from_ltrb(52.0, 0.0, 412.0, 400.0));
            // The search view's top right is the anchor's.
            assert_eq!(search_view_rect.top_right(), anchor_rect.top_right());
        });
    }

    fn set_view_size(tester: &WidgetTester, width: f64, height: f64) {
        tester.view().set_physical_size(Size::new(width, height));
        tester.view().set_device_pixel_ratio(1.0);
    }

    fn bottom_corner_anchor(
        alignment: AlignmentGeometry,
        direction: TextDirection,
        is_full_screen: bool,
    ) -> WidgetRef {
        Directionality::new(
            direction,
            Material::new().child(
                Align::new()
                    .alignment(alignment)
                    .child(view_anchor().is_full_screen(is_full_screen)),
            ),
        )
        .into_widget()
    }

    // 'Search view does not go off the screen - RTL'.
    #[test]
    fn search_view_does_not_go_off_the_screen_rtl() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                bottom_corner_anchor(AlignmentGeometry::BOTTOM_LEFT, TextDirection::Rtl, false),
            );
            let find_icon_button = find::widget_with_icon::<IconButton>(&Icons::search());
            assert_eq!(
                tester.get_rect(find_icon_button),
                Rect::from_ltrb(0.0, 552.0, 48.0, 600.0)
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            let search_view_rect = tester.get_rect(
                find::descendant(view_content(), find::by_type::<ConstrainedBox>()).first(),
            );
            assert_eq!(search_view_rect, Rect::from_ltrb(0.0, 200.0, 360.0, 600.0));
        });
    }

    fn smaller_window_anchor(direction: TextDirection) -> WidgetRef {
        Directionality::new(
            direction,
            Material::new().child(
                SearchAnchor::new(
                    |_app, _context, controller| {
                        Align::new()
                            .alignment(AlignmentGeometry::BOTTOM_RIGHT)
                            .child(IconButton::new(
                                Some(Listener::new(move |app: &mut App| {
                                    controller.open_view(app)
                                })),
                                Icon::new(Some(Icons::search())),
                            ))
                            .into_widget()
                    },
                    empty_suggestions(),
                )
                .is_full_screen(false),
            ),
        )
        .into_widget()
    }

    // 'Search view becomes smaller if the window size is smaller than the view size'.
    #[test]
    fn search_view_becomes_smaller_if_the_window_size_is_smaller_than_the_view_size() {
        test_widgets(|tester| {
            set_view_size(tester, 200.0, 200.0);
            for direction in [TextDirection::Ltr, TextDirection::Rtl] {
                if direction == TextDirection::Rtl {
                    tester.pump_widget(Container::new());
                }
                pump_material_home(tester, None, smaller_window_anchor(direction));
                // The icon button size is (48.0, 48.0), and the screen size is (200.0, 200.0).
                assert_eq!(
                    tester.get_rect(find::widget_with_icon::<IconButton>(&Icons::search())),
                    Rect::from_ltrb(152.0, 152.0, 200.0, 200.0)
                );
                tester.tap(find::by_icon(&Icons::search()));
                tester.pump_and_settle();
                let search_view_rect = tester.get_rect(
                    find::descendant(view_content(), find::by_type::<ConstrainedBox>()).first(),
                );
                assert_eq!(search_view_rect, Rect::from_ltrb(0.0, 0.0, 200.0, 200.0));
            }
        });
    }

    fn window_size_change_anchor(is_full_screen: bool) -> WidgetRef {
        Material::new()
            .child(
                SearchAnchor::new(
                    |_app, _context, controller| {
                        Align::new()
                            .alignment(AlignmentGeometry::BOTTOM_RIGHT)
                            .child(IconButton::new(
                                Some(Listener::new(move |app: &mut App| {
                                    controller.open_view(app)
                                })),
                                Icon::new(Some(Icons::search())),
                            ))
                            .into_widget()
                    },
                    empty_suggestions(),
                )
                .is_full_screen(is_full_screen),
            )
            .into_widget()
    }

    // 'Docked search view route is popped if the window size changes'.
    #[test]
    fn docked_search_view_route_is_popped_if_the_window_size_changes() {
        test_widgets(|tester| {
            set_view_size(tester, 500.0, 600.0);
            pump_material_home(tester, None, window_size_change_anchor(false));
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            expect(tester, &find::by_type::<BackButton>(), finds_one_widget());

            set_view_size(tester, 250.0, 200.0);
            tester.pump_and_settle();
            expect(tester, &find::by_type::<BackButton>(), finds_nothing());
        });
    }

    // 'Full-screen search view route should stay if the window size changes'.
    #[test]
    fn full_screen_search_view_route_should_stay_if_the_window_size_changes() {
        test_widgets(|tester| {
            set_view_size(tester, 500.0, 600.0);
            pump_material_home(tester, None, window_size_change_anchor(true));
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            expect(tester, &find::by_type::<BackButton>(), finds_one_widget());

            set_view_size(tester, 250.0, 200.0);
            tester.pump_and_settle();
            expect(tester, &find::by_type::<BackButton>(), finds_one_widget());
        });
    }

    // 'Search view route does not throw exception during pop animation'.
    #[test]
    fn search_view_route_does_not_throw_exception_during_pop_animation() {
        test_widgets(|tester| {
            let anchor = SearchAnchor::new(
                |_app, _context, controller| {
                    IconButton::new(
                        Some(Listener::new(move |app: &mut App| {
                            controller.open_view(app)
                        })),
                        Icon::new(Some(Icons::search())),
                    )
                    .into_widget()
                },
                |_app, _context, _controller| {
                    Task::ready(
                        (0..5)
                            .map(|index| {
                                ListTile::new()
                                    .leading(Icon::new(Some(Icons::history())))
                                    .title(Text::new(format!("item {index}")))
                                    .trailing(Icon::new(Some(Icons::chevron_right())))
                                    .on_tap(Listener::new(|_| {}))
                                    .into_widget()
                            })
                            .collect(),
                    )
                },
            );
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(Center::new().child(anchor))
                    .into_widget(),
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            tester.tap(find::by_type::<BackButton>());
            tester.pump_and_settle();
        });
    }

    fn pump_padded_navigator_anchor(
        tester: &mut WidgetTester,
        alignment: Option<AlignmentGeometry>,
    ) {
        let theme = m3(tester);
        let mut anchor: WidgetRef = view_anchor().is_full_screen(false).into_widget();
        if let Some(alignment) = alignment {
            anchor = Align::new()
                .alignment(alignment)
                .child(anchor)
                .into_widget();
        }
        tester.pump_widget(
            crate::app::MaterialApp::new()
                .theme(theme)
                .builder(|_app, _context, child| {
                    Scaffold::new()
                        .body(
                            Padding::new(EdgeInsetsGeometry::from(EdgeInsets::all(100.0)))
                                .child(child.expect("the navigator").clone()),
                        )
                        .into_widget()
                })
                .home(Material::new().child(anchor).into_widget()),
        );
    }

    // 'Docked search should position itself correctly based on closest navigator'.
    #[test]
    fn docked_search_should_position_itself_correctly_based_on_closest_navigator() {
        test_widgets(|tester| {
            pump_padded_navigator_anchor(tester, None);
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            let search_view_rect = tester.get_rect(
                find::descendant(view_content(), find::by_type::<ConstrainedBox>()).first(),
            );
            assert_eq!(search_view_rect.top_left(), Offset::new(100.0, 100.0));
        });
    }

    // 'Docked search view with nested navigator does not go off the screen'.
    #[test]
    fn docked_search_view_with_nested_navigator_does_not_go_off_the_screen() {
        test_widgets(|tester| {
            set_view_size(tester, 400.0, 400.0);
            pump_padded_navigator_anchor(tester, Some(AlignmentGeometry::BOTTOM_RIGHT));
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            let search_view_rect = tester.get_rect(
                find::descendant(view_content(), find::by_type::<ConstrainedBox>()).first(),
            );
            assert_eq!(search_view_rect.bottom_right(), Offset::new(300.0, 300.0));
        });
    }

    /// Dart's `findTextField()`.
    fn find_text_field() -> waft_widgets::testing::Finder {
        find::descendant(find::by_type::<SearchBar>(), find::by_type::<TextField>())
    }

    fn is_hit_testable(tester: &WidgetTester, finder: waft_widgets::testing::Finder) -> bool {
        let element = tester.element(finder);
        let center = tester.get_center(element);
        let target = tester.render_box_of(element).as_object().id();
        let mut app = tester.cell.borrow_mut();
        let view_id = app.platform().views()[0].id();
        let mut result = waft_gestures::HitTestResult::new();
        waft_gestures::GestureBinding::instance(&mut app).hit_test_in_view(
            &mut app,
            &mut result,
            center,
            view_id,
        );
        result
            .path()
            .iter()
            .any(|entry| entry.shared_target().arena_id() == Some(target))
    }

    fn decoration_theme() -> InputDecorationThemeData {
        InputDecorationThemeData::new()
            .focus_color(Colors::GREEN.primary())
            .hover_color(Colors::BLUE.primary())
            .outline_border(BorderSide {
                color: Colors::PINK.primary(),
                width: 10.0,
                ..BorderSide::default()
            })
            .is_dense(true)
            .content_padding(EdgeInsetsGeometry::from(EdgeInsets::symmetric(0.0, 20.0)))
            .hint_style(TextStyle::new().color(Colors::PURPLE_ACCENT.primary()))
            .fill_color(Colors::TEAL_ACCENT.primary())
            .filled(true)
            .is_collapsed(true)
            .border(crate::input_border::OutlineInputBorder::default())
            .focused_border(crate::input_border::UnderlineInputBorder::default())
            .enabled_border(crate::input_border::UnderlineInputBorder::default())
            .error_border(crate::input_border::UnderlineInputBorder::default())
            .focused_error_border(crate::input_border::UnderlineInputBorder::default())
            .disabled_border(crate::input_border::UnderlineInputBorder::default())
            .constraints({
                let mut constraints = BoxConstraints::new();
                constraints.max_width = 300.0;
                constraints
            })
    }

    fn check_decoration_in_search_bar(tester: &WidgetTester, color_scheme: &ColorScheme) {
        let decoration = tester
            .widget(find_text_field(), |field: &TextField| {
                field.decoration.clone()
            })
            .expect("a decoration");
        assert_eq!(decoration.border, Some(InputBorderRef::none()));
        assert_eq!(decoration.focused_border, Some(InputBorderRef::none()));
        assert_eq!(decoration.enabled_border, Some(InputBorderRef::none()));
        assert_eq!(decoration.error_border, None);
        assert_eq!(decoration.focused_error_border, None);
        assert_eq!(decoration.disabled_border, None);
        assert_eq!(decoration.constraints, None);
        assert_eq!(decoration.is_collapsed, Some(false));
        assert_eq!(decoration.filled, Some(false));
        assert!(decoration.fill_color.is_none());
        assert_eq!(decoration.focus_color, None);
        assert_eq!(decoration.hover_color, None);
        assert_eq!(decoration.content_padding, Some(EdgeInsets::ZERO.into()));
        assert_eq!(
            decoration.hint_style.and_then(|style| style
                .value()
                .color
                .as_ref()
                .map(|color| color.color())),
            Some(color_scheme.on_surface_variant())
        );
    }

    // 'Overall InputDecorationThemeData does not override text field style in SearchBar'.
    #[test]
    fn overall_input_decoration_theme_data_does_not_override_text_field_style_in_search_bar() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .input_decoration_theme(decoration_theme())
                .build(&tester.cell.borrow());
            let color_scheme = theme.color_scheme.clone();
            pump_material_home(
                tester,
                Some(theme),
                Center::new()
                    .child(Material::new().child(SearchBar::new().hint_text("hint text")))
                    .into_widget(),
            );
            check_decoration_in_search_bar(tester, &color_scheme);
            let material = search_bar_material(tester);
            check_search_bar_defaults(tester, &color_scheme, &material);
        });
    }

    // 'Overall InputDecorationThemeData does not override text field style in the search view
    // route'.
    #[test]
    fn overall_input_decoration_theme_data_does_not_override_text_field_style_in_the_search_view_route()
     {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .input_decoration_theme(decoration_theme())
                .build(&tester.cell.borrow());
            let color_scheme = theme.color_scheme.clone();
            pump_material_home(
                tester,
                Some(theme),
                Scaffold::new()
                    .body(
                        Material::new().child(
                            Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                                SearchAnchor::new(
                                    |_app, _context, _controller| {
                                        Icon::new(Some(Icons::search())).into_widget()
                                    },
                                    empty_suggestions(),
                                )
                                .view_hint_text("hint text"),
                            ),
                        ),
                    )
                    .into_widget(),
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            check_decoration_in_search_bar(tester, &color_scheme);

            let view_bar = find::descendant(view_content(), find::by_type::<SearchBar>());
            let material = tester.widget(
                find::descendant(view_bar, find::by_type::<Material>()).first(),
                |m: &Material| m.clone(),
            );
            assert_eq!(material.color, Some(Colors::TRANSPARENT));
            assert_eq!(material.elevation, 0.0);
            let hint_color = tester
                .widget(find::text("hint text"), |text: &Text| text.style.clone())
                .and_then(|style| style.color)
                .map(|color| color.color());
            assert_eq!(hint_color, Some(color_scheme.on_surface_variant()));
            let input = "entered text";
            tester.enter_text(find::by_type::<SearchBar>(), input);
            let input_color = tester
                .widget(find::text(input), |text: &EditableText| text.style.clone())
                .color
                .map(|color| color.color());
            assert_eq!(input_color, Some(color_scheme.on_surface));
        });
    }

    // 'SearchAnchor view respects theme brightness'.
    #[test]
    fn search_anchor_view_respects_theme_brightness() {
        test_widgets(|tester| {
            let build = |theme: ThemeData| {
                material_app(
                    theme,
                    Center::new()
                        .child(Material::new().child(SearchAnchor::new(
                            |_app, _context, controller| {
                                IconButton::new(
                                    Some(Listener::new(move |app: &mut App| {
                                        controller.open_view(app)
                                    })),
                                    Icon::new(Some(Icons::ac_unit())),
                                )
                                .into_widget()
                            },
                            empty_suggestions(),
                        )))
                        .into_widget(),
                )
            };
            let light = ThemeData::builder()
                .brightness(Brightness::Light)
                .build(&tester.cell.borrow());
            tester.pump_widget(build(light.clone()));
            tester.tap(find::widget_with_icon::<IconButton>(&Icons::ac_unit()));
            tester.pump_and_settle();
            assert_eq!(
                search_view_material(tester).color,
                Some(light.color_scheme.surface_container_high())
            );

            let dark = ThemeData::builder()
                .brightness(Brightness::Dark)
                .build(&tester.cell.borrow());
            tester.pump_widget(build(dark.clone()));
            tester.pump_and_settle();
            assert_eq!(
                search_view_material(tester).color,
                Some(dark.color_scheme.surface_container_high())
            );
        });
    }

    // 'SearchAnchor.bar respects barFocusNode property'.
    #[test]
    fn search_anchor_bar_respects_bar_focus_node_property() {
        test_widgets(|tester| {
            let node = FocusNode::new(&mut tester.cell.borrow_mut());
            pump_centered_material(
                tester,
                SearchAnchor::bar(empty_suggestions())
                    .bar_focus_node(node.erase())
                    .into_search_anchor(),
            );
            let field_node = text_field_focus_node(tester, find::by_type::<TextField>());
            let expected: FocusNodeRef = node.erase();
            assert_eq!(field_node.handle().id(), expected.handle().id());
        });
    }

    // 'SearchAnchor.bar barFocusNode can be used to control focus'.
    #[test]
    fn search_anchor_bar_bar_focus_node_can_be_used_to_control_focus() {
        test_widgets(|tester| {
            let node = FocusNode::new(&mut tester.cell.borrow_mut());
            pump_centered_material(
                tester,
                SearchAnchor::bar(empty_suggestions())
                    .bar_focus_node(node.erase())
                    .into_search_anchor(),
            );
            let has_focus =
                |tester: &WidgetTester| node.handle().has_focus(&mut tester.cell.borrow_mut());
            assert!(!has_focus(tester));
            node.handle()
                .request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump();
            assert!(has_focus(tester));
            node.handle()
                .unfocus(&mut tester.cell.borrow_mut(), UnfocusDisposition::Scope);
            tester.pump();
            assert!(!has_focus(tester));
        });
    }

    // 'SearchAnchor tap failed when disabled'.
    #[test]
    fn search_anchor_tap_failed_when_disabled() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                Center::new()
                    .child(Material::new().child(search_icon_anchor().enabled(false)))
                    .into_widget(),
            );
            expect(tester, &find::by_type::<SearchAnchor>(), finds_one_widget());
            assert!(!is_hit_testable(tester, find::by_type::<SearchAnchor>()));
        });
    }

    // 'SearchAnchor respects headerHeight'.
    #[test]
    fn search_anchor_respects_header_height() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                Center::new()
                    .child(
                        Material::new().child(
                            search_icon_anchor()
                                .is_full_screen(true)
                                .header_height(32.0),
                        ),
                    )
                    .into_widget(),
            );
            tester.pump();
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            let header = find::descendant(view_content(), find::by_type::<SearchBar>());
            assert_eq!(tester.get_size(header).height(), 32.0);
        });
    }

    // 'SearchAnchor.bar respects viewHeaderHeight'.
    #[test]
    fn search_anchor_bar_respects_view_header_height() {
        test_widgets(|tester| {
            pump_centered_material(
                tester,
                SearchAnchor::bar(empty_suggestions())
                    .is_full_screen(true)
                    .view_header_height(32.0)
                    .into_search_anchor(),
            );
            tester.pump();
            tester.tap(find::by_type::<SearchBar>());
            tester.pump_and_settle();
            let header = find::descendant(view_content(), find::by_type::<SearchBar>());
            assert_eq!(tester.get_size(header).height(), 32.0);
        });
    }

    // 'The default clear button only shows when text input is not empty on the search view'.
    #[test]
    fn the_default_clear_button_only_shows_when_text_input_is_not_empty_on_the_search_view() {
        test_widgets(|tester| {
            pump_centered_material(tester, search_icon_anchor());
            tester.pump();
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            let close = find::widget_with_icon::<IconButton>(&Icons::close());
            expect(tester, &close, finds_nothing());

            tester.enter_text(find_text_field(), "a");
            tester.pump();
            expect(tester, &close, finds_one_widget());

            tester.enter_text(find_text_field(), "");
            tester.pump();
            expect(tester, &close, finds_nothing());
        });
    }

    // 'Tapping outside searchbar should unfocus the searchbar on mobile'.
    #[test]
    fn tapping_outside_search_bar_should_unfocus_the_search_bar_on_mobile() {
        waft_widgets::testing::test_widgets_variant(
            waft_widgets::testing::TargetPlatformVariant::mobile(),
            |tester| {
                let node = FocusNode::new(&mut tester.cell.borrow_mut());
                let focus_node: FocusNodeRef = node.erase();
                let anchor = SearchAnchor::new(
                    {
                        let focus_node = focus_node.clone();
                        move |app, _context, controller| {
                            let unfocus_node = focus_node.clone();
                            let tap_controller = controller;
                            let change_controller = controller;
                            SearchBar::new()
                                .controller(controller.as_controller().keep(app))
                                .on_tap(Listener::new(move |app: &mut App| {
                                    tap_controller.open_view(app)
                                }))
                                .on_tap_outside(Rc::new(move |app: &mut App, _event| {
                                    unfocus_node
                                        .handle()
                                        .unfocus(app, UnfocusDisposition::Scope)
                                }))
                                .on_changed(move |app: &mut App, _| {
                                    change_controller.open_view(app)
                                })
                                .auto_focus(true)
                                .focus_node(focus_node.clone())
                                .into_widget()
                        }
                    },
                    |_app, _context, _controller| {
                        Task::ready(
                            (0..5)
                                .map(|index| {
                                    ListTile::new()
                                        .title(Text::new(format!("item {index}")))
                                        .into_widget()
                                })
                                .collect(),
                        )
                    },
                );
                pump_material_home(tester, None, Scaffold::new().body(anchor).into_widget());
                tester.pump();
                assert!(focus_node.handle().has_primary_focus(&tester.cell.borrow()));
                tester.tap_at(Offset::new(50.0, 50.0), 1);
                tester.pump();
                assert!(!focus_node.handle().has_primary_focus(&tester.cell.borrow()));
            },
        );
    }

    // 'Search view widgets can inherit local themes'.
    #[test]
    fn search_view_widgets_can_inherit_local_themes() {
        test_widgets(|tester| {
            let global_theme = ThemeData::builder()
                .color_scheme_seed(Colors::RED.primary())
                .build(&tester.cell.borrow());
            let local_theme = ThemeData::builder()
                .color_scheme_seed(Colors::GREEN.primary())
                .icon_button_theme(
                    IconButtonThemeData::new().style(
                        IconButton::style_from()
                            .background_color(Color::new(0xffffff00))
                            .build(),
                    ),
                )
                .card_theme(CardThemeData::new().color(Color::new(0xff00ffff)))
                .build(&tester.cell.borrow());
            let local_surface = local_theme.color_scheme.surface_container_high();
            pump_material_home(
                tester,
                Some(global_theme),
                Center::new()
                    .child(Theme::new(
                        local_theme,
                        Material::new().child(
                            SearchAnchor::bar(|_app, _context, _controller| {
                                Task::ready(vec![
                                    Card::new()
                                        .child(
                                            ListTile::new()
                                                .on_tap(Listener::new(|_| {}))
                                                .title(Text::new("Item 1")),
                                        )
                                        .into_widget(),
                                ])
                            })
                            .into_search_anchor(),
                        ),
                    ))
                    .into_widget(),
            );
            tester.tap(find::by_type::<SearchBar>());
            tester.pump_and_settle();

            // The search view background color.
            assert_eq!(search_view_material(tester).color, Some(local_surface));
            // The search view icons' background color.
            let icon_button_material = tester.widget(
                find::descendant(find::by_type::<IconButton>(), find::by_type::<Material>())
                    .first(),
                |material: &Material| material.clone(),
            );
            assert_eq!(icon_button_material.color, Some(Color::new(0xffffff00)));
            // The suggestion card color.
            let suggestion_material = tester.widget(
                find::descendant(find::by_type::<Card>(), find::by_type::<Material>()).first(),
                |material: &Material| material.clone(),
            );
            assert_eq!(suggestion_material.color, Some(Color::new(0xff00ffff)));
        });
    }

    // 'Block entering text on disabled widget'.
    #[test]
    fn block_entering_text_on_disabled_widget() {
        test_widgets(|tester| {
            let controller = {
                let mut app = tester.cell.borrow_mut();
                TextEditingController::new(&mut app).text(&mut app, "init")
            };
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(
                        Center::new().child(
                            SearchBar::new()
                                .controller(controller.erase())
                                .enabled(false),
                        ),
                    )
                    .into_widget(),
            );
            tester.enter_text(find::by_type::<SearchBar>(), "abcdefghi");
            assert_eq!(
                controller.handle().text_value(&tester.cell.borrow()),
                "init"
            );
        });
    }

    fn set_keyboard_insets(tester: &WidgetTester) {
        tester.view().set_view_insets(waft_embedder::ViewPadding {
            bottom: 500.0,
            ..waft_embedder::ViewPadding::ZERO
        });
    }

    // 'suggestionsBuilder is not called when the search value does not change'.
    #[test]
    fn suggestions_builder_is_not_called_when_the_search_value_does_not_change() {
        test_widgets(|tester| {
            let called = Rc::new(Cell::new(0));
            let anchor = SearchAnchor::new(
                |_app, _context, _controller| Icon::new(Some(Icons::search())).into_widget(),
                {
                    let called = Rc::clone(&called);
                    move |_app, _context, _controller| {
                        called.set(called.get() + 1);
                        Task::ready(Vec::new())
                    }
                },
            );
            pump_material_home(
                tester,
                None,
                Scaffold::new()
                    .body(Center::new().child(anchor))
                    .into_widget(),
            );
            tester.pump();
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();

            // Simulate the keyboard opening resizing the view.
            set_keyboard_insets(tester);
            tester.show_keyboard(find::by_type::<TextField>());
            tester.pump_and_settle();
            assert_eq!(called.get(), 2);

            // Remove the view insets, as if the keyboard were hidden, and hide the keyboard.
            tester.view().reset_view_insets();
            tester
                .test_text_input()
                .receive_action(TextInputAction::Done);
            tester.pump_and_settle();
            assert_eq!(called.get(), 2);
        });
    }

    // 'Suggestions gets refreshed after long API call'.
    #[test]
    fn suggestions_gets_refreshed_after_long_api_call() {
        test_widgets(|tester| {
            let api_call_duration = Duration::from_secs(1);
            let debounce_timer: Rc<RefCell<Option<Timer>>> = Rc::default();
            let anchor = SearchAnchor::new(
                |_app, _context, _controller| Icon::new(Some(Icons::search())).into_widget(),
                {
                    let debounce_timer = Rc::clone(&debounce_timer);
                    move |app, _context, _controller| {
                        let completer = Completer::new();
                        let future = completer.future();
                        if let Some(timer) = debounce_timer.borrow_mut().take() {
                            timer.cancel(app);
                        }
                        let timer = Timer::new(
                            app,
                            api_call_duration,
                            Listener::new(move |app: &mut App| {
                                completer.complete(
                                    app,
                                    (0..10)
                                        .map(|index| {
                                            ListTile::new()
                                                .title(Text::new(format!("Item - {index}")))
                                                .into_widget()
                                        })
                                        .collect(),
                                );
                            }),
                        );
                        *debounce_timer.borrow_mut() = Some(timer);
                        app.spawn(async move |_cx| future.await)
                    }
                },
            );
            pump_material_home(
                tester,
                None,
                Scaffold::new()
                    .body(Center::new().child(anchor))
                    .into_widget(),
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();

            // Simulate the keyboard opening resizing the view, and show the keyboard.
            set_keyboard_insets(tester);
            tester.show_keyboard(find::by_type::<TextField>());
            tester.pump_and_settle_by(api_call_duration);
            expect(tester, &find::text("Item - 1"), finds_one_widget());
        });
    }

    // 'SearchAnchor does not dispose external SearchController'.
    #[test]
    fn search_anchor_does_not_dispose_external_search_controller() {
        test_widgets(|tester| {
            let controller = SearchController::new(&mut tester.cell.borrow_mut());
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(view_anchor().search_controller(controller.clone()))
                    .into_widget(),
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            pump_material_home(
                tester,
                None,
                Material::new().child(Text::new("disposed")).into_widget(),
            );
            // The controller is still alive: reading it does not panic.
            let _ = controller.handle().text_value(&tester.cell.borrow());
        });
    }

    // 'SearchAnchor gracefully closes its search view when disposed'.
    #[test]
    fn search_anchor_gracefully_closes_its_search_view_when_disposed() {
        test_widgets(|tester| {
            let disposed = Rc::new(Cell::new(false));
            let set_state: Rc<RefCell<Option<waft_widgets::StateSetter>>> = Rc::default();
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(waft_widgets::StatefulBuilder::new({
                        let disposed = Rc::clone(&disposed);
                        let set_state = Rc::clone(&set_state);
                        move |_app, _context, state_setter| {
                            *set_state.borrow_mut() = Some(state_setter);
                            if disposed.get() {
                                return Text::new("disposed").into_widget();
                            }
                            SearchAnchor::new(
                                |_app, _context, controller| {
                                    IconButton::new(
                                        Some(Listener::new(move |app: &mut App| {
                                            controller.open_view(app)
                                        })),
                                        Icon::new(Some(Icons::search())),
                                    )
                                    .into_widget()
                                },
                                |_app, _context, _controller| {
                                    Task::ready(vec![Text::new("suggestion").into_widget()])
                                },
                            )
                            .into_widget()
                        }
                    }))
                    .into_widget(),
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();

            let state_setter = set_state.borrow().clone().expect("a state setter");
            tester.update(|app| {
                state_setter(app, &mut |_| disposed.set(true));
            });
            tester.cell.checkpoint();
            tester.pump();
            // The search view starts to close but is not disposed yet.
            expect(tester, &find::by_type::<EditableText>(), finds_one_widget());
            tester.pump_and_settle();
            // The search view and its internal search controller are now disposed.
            expect(tester, &find::by_type::<TextField>(), finds_nothing());
        });
    }

    // 'disposing SearchAnchor during search view exit animation does not crash'.
    #[test]
    fn disposing_search_anchor_during_search_view_exit_animation_does_not_crash() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let theme = m3(tester);
            let build = {
                let key = key.clone();
                move |home: WidgetRef| {
                    crate::app::MaterialApp::new()
                        .theme(theme.clone())
                        .navigator_key(key.clone())
                        .home(home)
                        .into_widget()
                }
            };
            tester.pump_widget(build(Material::new().child(view_anchor()).into_widget()));
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            let navigator = key
                .current_state::<waft_widgets::NavigatorState>(&tester.cell.borrow())
                .expect("a navigator");
            navigator.pop(&mut tester.cell.borrow_mut(), None);
            tester.cell.checkpoint();
            tester.pump();
            tester.pump_widget(build(
                Material::new().child(Text::new("disposed")).into_widget(),
            ));
            tester.pump();
        });
    }

    // 'SearchAnchor.bar viewOnClose function test'.
    #[test]
    fn search_anchor_bar_view_on_close_function_test() {
        test_widgets(|tester| {
            let name = Rc::new(RefCell::new("silva".to_string()));
            let anchor = SearchAnchor::bar(|_app, _context, controller| {
                Task::ready(vec![
                    ListTile::new()
                        .title(Text::new("item 0"))
                        .on_tap(Listener::new(move |app: &mut App| {
                            controller.close_view(app, Some("item 0".to_string()))
                        }))
                        .into_widget(),
                ])
            })
            .on_close({
                let name = Rc::clone(&name);
                Listener::new(move |_| *name.borrow_mut() = "Pedro".to_string())
            });
            pump_material_home(
                tester,
                None,
                Material::new()
                    .child(anchor.into_search_anchor())
                    .into_widget(),
            );
            // Open search view.
            tester.tap(find::by_type::<SearchBar>());
            tester.pump_and_settle();
            assert_eq!(*name.borrow(), "silva");
            // Pop search view route.
            tester.tap(find::by_type::<BackButton>());
            tester.pump_and_settle();
            assert_eq!(*name.borrow(), "Pedro");
        });
    }

    // 'SearchAnchor.bar onOpen is called when the search view is opened'.
    #[test]
    fn search_anchor_bar_on_open_is_called_when_the_search_view_is_opened() {
        test_widgets(|tester| {
            let state = Rc::new(RefCell::new("Idle"));
            let anchor = SearchAnchor::bar(|_app, _context, _controller| {
                Task::ready(
                    (0..5)
                        .map(|index| {
                            ListTile::new()
                                .leading(Icon::new(Some(Icons::history())))
                                .title(Text::new(format!("item {index}")))
                                .trailing(Icon::new(Some(Icons::chevron_right())))
                                .on_tap(Listener::new(|_| {}))
                                .into_widget()
                        })
                        .collect(),
                )
            })
            .on_close({
                let state = Rc::clone(&state);
                Listener::new(move |_| *state.borrow_mut() = "Closed")
            })
            .on_open({
                let state = Rc::clone(&state);
                Listener::new(move |_| *state.borrow_mut() = "Opened")
            });
            pump_centered_material(tester, anchor.into_search_anchor());
            expect(tester, &find::by_icon(&Icons::search()), finds_one_widget());
            // Open search view.
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump();
            assert_eq!(*state.borrow(), "Opened");
            // Pop search view route.
            tester.tap(find::by_type::<BackButton>());
            tester.pump();
            assert_eq!(*state.borrow(), "Closed");
        });
    }

    // 'The last element of the suggestion list should be visible when scrolling to the end of
    // list'.
    #[test]
    fn the_last_element_of_the_suggestion_list_should_be_visible_when_scrolling_to_the_end() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                SearchAnchor::bar(|_app, _context, _controller| {
                    Task::ready(
                        (0..30)
                            .map(|index| {
                                ListTile::new()
                                    .title_alignment(ListTileTitleAlignment::Center)
                                    .title(Text::new(format!("Item {index}")))
                                    .into_widget()
                            })
                            .collect(),
                    )
                })
                .into_search_anchor()
                .into_widget(),
            );
            // Open search view.
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();

            let fake_keyboard_height = 500.0;
            let physical_bottom_padding = fake_keyboard_height * tester.device_pixel_ratio();
            // Simulate the keyboard opening resizing the view.
            tester.view().set_view_insets(waft_embedder::ViewPadding {
                bottom: physical_bottom_padding,
                ..waft_embedder::ViewPadding::ZERO
            });
            // Scroll down to the end of the list.
            expect(tester, &find::by_type::<ListView>(), finds_one_widget());
            tester.fling(
                find::by_type::<ListView>(),
                Offset::new(0.0, -5000.0),
                5000.0,
            );
            tester.pump_and_settle();

            // The last item should not be hidden by the keyboard.
            let last_item_bottom = tester.get_rect(find::text("Item 29")).bottom;
            let fake_keyboard_top = tester
                .get_size(find::by_type::<crate::app::MaterialApp>())
                .height()
                - fake_keyboard_height;
            assert!(last_item_bottom <= fake_keyboard_top);
        });
    }

    // 'SearchAnchor.bar respects SearchBarThemeData.padding'.
    #[test]
    fn search_anchor_bar_respects_search_bar_theme_data_padding() {
        test_widgets(|tester| {
            let custom_padding: EdgeInsetsGeometry = EdgeInsets::all(32.0).into();
            pump_material_home(
                tester,
                None,
                Scaffold::new()
                    .body(crate::search_bar_theme::SearchBarTheme::new(
                        crate::search_bar_theme::SearchBarThemeData::new().padding(
                            <dyn WidgetStateProperty<Option<EdgeInsetsGeometry>>>::all(Some(
                                custom_padding,
                            )),
                        ),
                        SearchAnchor::bar(empty_suggestions()).into_search_anchor(),
                    ))
                    .into_widget(),
            );
            let padding = tester.widget(
                find::descendant(find::by_type::<SearchBar>(), find::by_type::<Padding>()).first(),
                |padding: &Padding| padding.padding,
            );
            assert_eq!(padding, custom_padding);
        });
    }

    // 'SearchAnchor does not crash at zero area'.
    #[test]
    fn search_anchor_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.view().set_physical_size(Size::ZERO);
            let controller = SearchController::new(&mut tester.cell.borrow_mut());
            pump_material_home(
                tester,
                None,
                Center::new()
                    .child(
                        SearchAnchor::new(
                            |_app, _context, _controller| Text::new("X").into_widget(),
                            |_app, _context, _controller| {
                                Task::ready(vec![Text::new("Y").into_widget()])
                            },
                        )
                        .search_controller(controller.clone()),
                    )
                    .into_widget(),
            );
            assert_eq!(tester.get_size(find::by_type::<SearchAnchor>()), Size::ZERO);
            controller.handle().set_selection(
                &mut tester.cell.borrow_mut(),
                TextSelection::collapsed(0, TextAffinity::Downstream),
            );
            tester.pump();
            assert!(
                !find::by_type::<Text>()
                    .skip_offstage(false)
                    .evaluate(tester)
                    .is_empty()
            );
        });
    }

    // 'SearchBar does not crash at zero area'.
    #[test]
    fn search_bar_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.view().set_physical_size(Size::ZERO);
            let controller = {
                let mut app = tester.cell.borrow_mut();
                TextEditingController::new(&mut app).text(&mut app, "X")
            };
            pump_material_home(
                tester,
                None,
                Center::new()
                    .child(SearchBar::new().controller(controller.erase()))
                    .into_widget(),
            );
            assert_eq!(tester.get_size(find::by_type::<SearchBar>()), Size::ZERO);
            controller.handle().set_selection(
                &mut tester.cell.borrow_mut(),
                TextSelection::collapsed(0, TextAffinity::Downstream),
            );
            tester.pump();
            expect(tester, &find::text("X"), finds_one_widget());
        });
    }

    // 'SearchAnchor full-screen view expands to fit screen when rotated'.
    #[test]
    fn search_anchor_full_screen_view_expands_to_fit_screen_when_rotated() {
        test_widgets(|tester| {
            // Start in portrait mode.
            set_view_size(tester, 360.0, 800.0);
            pump_material_home(
                tester,
                None,
                Center::new()
                    .child(view_anchor().is_full_screen(true))
                    .into_widget(),
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            let starting_size = search_view_size(tester);
            assert_eq!(starting_size.width(), 360.0);
            assert_eq!(starting_size.height(), 800.0);

            // Rotate to landscape mode.
            tester.view().set_physical_size(Size::new(800.0, 360.0));
            tester.pump_and_settle();
            let rotated_size = search_view_size(tester);
            assert_eq!(rotated_size.width(), 800.0);
            assert_eq!(rotated_size.height(), 360.0);
        });
    }

    // 'SearchAnchor resizes itself to match Navigator parent'.
    #[test]
    fn search_anchor_resizes_itself_to_match_navigator_parent() {
        test_widgets(|tester| {
            let parent_size = Rc::new(Cell::new(Size::new(600.0, 400.0)));
            let set_state: Rc<RefCell<Option<waft_widgets::StateSetter>>> = Rc::default();
            pump_material_home(
                tester,
                None,
                Scaffold::new()
                    .body(waft_widgets::StatefulBuilder::new({
                        let parent_size = Rc::clone(&parent_size);
                        let set_state = Rc::clone(&set_state);
                        move |_app, _context, state_setter| {
                            *set_state.borrow_mut() = Some(state_setter);
                            let size = parent_size.get();
                            SizedBox::new()
                                .width(size.width())
                                .height(size.height())
                                .child(waft_widgets::Navigator::new().on_generate_route(
                                    |app, settings| {
                                        let route = MaterialPageRoute::new(
                                            app,
                                            Rc::new(|_app, _context| {
                                                Scaffold::new()
                                                    .body(view_anchor().is_full_screen(true))
                                                    .into_widget()
                                            }),
                                        );
                                        route.clone().settings(
                                            app,
                                            waft_widgets::RouteSettingsRef::Settings(
                                                settings.clone(),
                                            ),
                                        );
                                        Some(route.erase())
                                    },
                                ))
                                .into_widget()
                        }
                    }))
                    .into_widget(),
            );
            tester.tap(find::by_icon(&Icons::search()));
            tester.pump_and_settle();
            let size = search_view_size(tester);
            assert_eq!(size.height(), 400.0);
            assert_eq!(size.width(), 600.0);

            let resize = |tester: &mut WidgetTester, width: f64, height: f64| {
                parent_size.set(Size::new(width, height));
                let state_setter = set_state.borrow().clone().expect("a state setter");
                tester.update(|app| state_setter(app, &mut |_| {}));
                tester.cell.checkpoint();
                tester.pump_and_settle();
            };
            // Resize the parent container larger.
            resize(tester, 700.0, 500.0);
            let size = search_view_size(tester);
            assert_eq!(size.height(), 500.0);
            assert_eq!(size.width(), 700.0);
            // Resize the parent container smaller.
            resize(tester, 400.0, 300.0);
            let size = search_view_size(tester);
            assert_eq!(size.height(), 300.0);
            assert_eq!(size.width(), 400.0);
        });
    }

    fn render_editable_at(
        tester: &WidgetTester,
        index: usize,
    ) -> waft_rendering::RenderHandle<waft_rendering::RenderEditable> {
        tester.all::<waft_rendering::RenderEditable>()[index]
    }

    /// Dart's `textOffsetToPosition(tester, offset, index:)`.
    fn text_offset_to_position(tester: &WidgetTester, offset: i32, index: usize) -> Offset {
        let editable = render_editable_at(tester, index);
        let mut app = tester.cell.borrow_mut();
        let endpoints = editable.get_endpoints_for_selection(
            &mut app,
            TextSelection::collapsed(offset, TextAffinity::Downstream),
        );
        assert_eq!(endpoints.len(), 1);
        editable
            .as_box()
            .local_to_global(&app, endpoints[0].point, None)
            + Offset::new(0.0, -2.0)
    }

    fn pump_selection_anchor(tester: &mut WidgetTester) -> Ref<SearchController> {
        let controller = SearchController::new(&mut tester.cell.borrow_mut());
        controller
            .handle()
            .set_text(&mut tester.cell.borrow_mut(), "initial text");
        pump_material_home(
            tester,
            None,
            Material::new()
                .child(
                    SearchAnchor::bar(empty_suggestions())
                        .search_controller(controller.clone())
                        .into_search_anchor(),
                )
                .into_widget(),
        );
        expect(tester, &find::text("initial text"), finds_one_widget());
        controller
    }

    fn selection_of(tester: &WidgetTester, controller: &Ref<SearchController>) -> TextSelection {
        controller.handle().selection(&tester.cell.borrow())
    }

    // 'SearchAnchor text selection: can right-click to select word'.
    #[test]
    fn search_anchor_text_selection_can_right_click_to_select_word() {
        waft_widgets::testing::test_widgets_variant(
            waft_widgets::testing::TargetPlatformVariant::only(TargetPlatform::MacOS),
            |tester| {
                let controller = pump_selection_anchor(tester);
                let mut gesture = TestPointer::mouse(
                    tester.next_pointer(),
                    waft_gestures::K_SECONDARY_MOUSE_BUTTON,
                );
                let at = text_offset_to_position(tester, 4, 0) + Offset::new(0.0, -9.0);
                tester.down(&mut gesture, at);
                tester.pump();
                tester.up(&gesture);
                tester.pump_and_settle();
                assert_eq!(selection_of(tester, &controller), TextSelection::new(0, 7));
                tester.remove_pointer(&gesture);
            },
        );
    }

    // 'SearchAnchor text selection: can click to set position'.
    #[test]
    fn search_anchor_text_selection_can_click_to_set_position() {
        waft_widgets::testing::test_widgets_variant(
            waft_widgets::testing::TargetPlatformVariant::desktop(),
            |tester| {
                let controller = pump_selection_anchor(tester);
                let mut gesture = point_gesture_to_search_bar(tester);
                let at = text_offset_to_position(tester, 2, 0) + Offset::new(0.0, -9.0);
                tester.down(&mut gesture, at);
                tester.pump();
                tester.up(&gesture);
                tester.pump_and_settle_by(waft_gestures::K_DOUBLE_TAP_TIMEOUT);
                assert_eq!(
                    selection_of(tester, &controller),
                    TextSelection::collapsed(2, TextAffinity::Downstream)
                );

                let at = text_offset_to_position(tester, 9, 1) + Offset::new(0.0, -9.0);
                tester.down(&mut gesture, at);
                tester.pump();
                tester.up(&gesture);
                tester.pump_and_settle();
                assert_eq!(
                    selection_of(tester, &controller),
                    TextSelection::collapsed(9, TextAffinity::Downstream)
                );
                tester.remove_pointer(&gesture);
            },
        );
    }

    /// The clicks the double- and triple-click cases make after the first one opened the view.
    fn click_after_view_opened(
        tester: &mut WidgetTester,
        gesture: &mut TestPointer,
        at: Offset,
        settle: bool,
    ) {
        tester.down(gesture, at);
        if settle {
            tester.pump_and_settle();
        } else {
            tester.pump();
        }
        tester.up(gesture);
        tester.pump();
    }

    // 'SearchAnchor text selection: can double-click to select word'.
    #[test]
    fn search_anchor_text_selection_can_double_click_to_select_word() {
        waft_widgets::testing::test_widgets_variant(
            waft_widgets::testing::TargetPlatformVariant::desktop(),
            |tester| {
                let controller = pump_selection_anchor(tester);
                let mut gesture = point_gesture_to_search_bar(tester);
                let at = text_offset_to_position(tester, 4, 0) + Offset::new(0.0, -9.0);
                tester.down(&mut gesture, at);
                tester.pump();
                tester.up(&gesture);
                tester.pump_and_settle_by(waft_gestures::K_DOUBLE_TAP_TIMEOUT);

                let after_open = text_offset_to_position(tester, 4, 1) + Offset::new(0.0, -9.0);
                click_after_view_opened(tester, &mut gesture, after_open, true);
                click_after_view_opened(tester, &mut gesture, after_open, false);
                assert_eq!(selection_of(tester, &controller), TextSelection::new(0, 7));
                tester.remove_pointer(&gesture);
            },
        );
    }

    // 'SearchAnchor text selection: can triple-click to select field'.
    #[test]
    fn search_anchor_text_selection_can_triple_click_to_select_field() {
        waft_widgets::testing::test_widgets_variant(
            waft_widgets::testing::TargetPlatformVariant::desktop(),
            |tester| {
                let controller = pump_selection_anchor(tester);
                let mut gesture = point_gesture_to_search_bar(tester);
                let at = text_offset_to_position(tester, 4, 0) + Offset::new(0.0, -9.0);
                tester.down(&mut gesture, at);
                tester.pump();
                tester.up(&gesture);
                tester.pump_and_settle_by(waft_gestures::K_DOUBLE_TAP_TIMEOUT);

                let after_open = text_offset_to_position(tester, 4, 1) + Offset::new(0.0, -9.0);
                click_after_view_opened(tester, &mut gesture, after_open, false);
                click_after_view_opened(tester, &mut gesture, after_open, false);
                tester.down(&mut gesture, after_open);
                tester.pump();
                tester.up(&gesture);
                tester.pump_and_settle();
                assert_eq!(selection_of(tester, &controller), TextSelection::new(0, 12));
                tester.remove_pointer(&gesture);
            },
        );
    }

    // 'SearchAnchor with enableTapHandling: false does not build GestureDetector'.
    #[test]
    fn search_anchor_with_enable_tap_handling_false_does_not_build_gesture_detector() {
        test_widgets(|tester| {
            pump_material_home(
                tester,
                None,
                Scaffold::new()
                    .body(search_icon_anchor().enable_tap_handling(false))
                    .into_widget(),
            );
            expect(
                tester,
                &find::descendant(
                    find::by_type::<SearchAnchor>(),
                    find::by_type::<waft_widgets::GestureDetector>(),
                ),
                finds_nothing(),
            );
        });
    }
}
