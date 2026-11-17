//! Flutter counterpart: material_ui's `search.dart`.
//!
//! A full-screen search page: [`show_search`] pushes it, and a [`SearchDelegate`] says what its
//! app bar, suggestions and results are. The route and the page share the delegate, so the state
//! Dart keeps in the delegate's private fields lives in [`SearchDelegateData`], which every
//! delegate holds and answers from [`SearchDelegate::search_delegate_data`].

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    AnimationRef, AnimationStatus, AnimationStatusListener, ProxyAnimation,
    k_always_dismissed_animation,
};
use waft_embedder::{
    SemanticsInputType, SystemUiOverlayStyle, TextAffinity, TextEditingValue, TextInputAction,
    TextInputType, TextSelection,
};
use waft_foundation::{
    App, CompleterFuture, Handle, ListenableObject, Listener, Ref, ValueKey, ValueNotifier,
};
use waft_painting::TextStyle;
use waft_services::{KeyEvent, LogicalKeyboardKey};
use waft_widgets::{
    AnimatedSwitcher, AnyFocusNode, AnyRoute, BuildContext, FadeTransition, FocusNode,
    FocusNodeLeaf, IntoWidget, KeyEventResult, KeyedSubtree, LocalHistoryRoute,
    LocalHistoryRouteData, ModalRoute, ModalRouteData, Navigator, OverlayRoute, OverlayRouteData,
    PageRoute, PageRouteData, PredictiveBackRoute, PreferredSizeWidgetRef, Route, RouteData,
    RouteResult, Semantics, State, StateData, StatefulWidget, TextEditingController,
    TextEditingControllerLeaf, TransitionRoute, TransitionRouteData, UnfocusDisposition, WidgetRef,
};

use crate::app_bar::AppBar;
use crate::app_bar_theme::AppBarThemeData;
use crate::colors::Colors;
use crate::debug::debug_check_has_material_localizations;
use crate::input_border::InputBorderRef;
use crate::input_decorator::{InputDecoration, InputDecorationThemeData};
use crate::material_localizations::MaterialLocalizations;
use crate::scaffold::Scaffold;
use crate::text_field::TextField;
use crate::theme::Theme;
use crate::theme_data::ThemeData;

/// Shows a full screen search page and returns the search result selected by the user when the
/// page is closed.
///
/// The search page consists of an app bar with a search field and a body which can either show
/// suggested search queries or the search results.
///
/// The appearance of the search page is determined by the provided `delegate`. The initial query
/// string is given by `query`; Dart's default is the empty string, `Some("")`. When `query` is
/// `None`, `delegate.query` will be used as the initial query.
///
/// The `use_root_navigator` argument is used to determine whether to push the search page to the
/// [`Navigator`] furthest from or nearest to the given `context`. By default, `use_root_navigator`
/// is `false` and the search page route created by this method is pushed to the nearest
/// navigator to the given `context`. It can not be `None`.
///
/// The `maintain_state` argument is used to determine whether the route should remain in memory
/// when it is inactive. By default, `maintain_state` is `false`.
///
/// The delegate's [`SearchDelegate::build_results`] method result is given back to the caller
/// through the future this function returns, when the page is closed with
/// [`SearchDelegate::close`].
pub fn show_search(
    app: &mut App,
    context: BuildContext,
    delegate: Rc<dyn SearchDelegate>,
    query: Option<&str>,
    use_root_navigator: bool,
    maintain_state: bool,
) -> CompleterFuture<RouteResult> {
    let data = delegate.search_delegate_data();
    delegate.set_query(app, query.unwrap_or(&delegate.query(app)).to_owned());
    data.current_body_notifier
        .handle()
        .set_value(app, Some(SearchBody::Suggestions));
    let route = SearchPageRoute::new(app, delegate, maintain_state);
    Navigator::of(app, context, use_root_navigator).push(app, route.into_erased())
}

/// Which of the delegate's two bodies the page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SearchBody {
    /// Suggested queries, [`SearchDelegate::build_suggestions`].
    Suggestions,

    /// The results of the query, [`SearchDelegate::build_results`].
    Results,
}

/// The state of a [`SearchDelegate`] that Dart keeps in the delegate's fields: every delegate
/// holds one, and answers it from [`SearchDelegate::search_delegate_data`].
///
/// The fields Dart gives as constructor arguments are plain fields; the setters
/// ([`automatically_imply_leading`](Self::automatically_imply_leading),
/// [`leading_width`](Self::leading_width)) are for what Dart leaves mutable.
pub struct SearchDelegateData {
    /// The hint text shown in the search field when it is empty.
    ///
    /// If this value is set to `None`, the value of
    /// [`MaterialLocalizations::search_field_label`] will be used instead.
    pub search_field_label: Option<String>,

    /// The style of the [`search_field_label`](Self::search_field_label).
    ///
    /// If this value is set to `None`, the value of the ambient [`Theme`]'s
    /// `InputDecorationThemeData.hint_style` will be used instead.
    ///
    /// Only one of [`search_field_style`](Self::search_field_style) or
    /// [`search_field_decoration_theme`](Self::search_field_decoration_theme) can be non-`None`.
    pub search_field_style: Option<TextStyle>,

    /// The [`InputDecorationThemeData`] used to configure the search field's visuals.
    ///
    /// Only one of [`search_field_style`](Self::search_field_style) or
    /// [`search_field_decoration_theme`](Self::search_field_decoration_theme) can be non-`None`.
    pub search_field_decoration_theme: Option<InputDecorationThemeData>,

    /// The type of action button to use for the keyboard.
    ///
    /// Defaults to the default value specified in [`TextField`].
    pub keyboard_type: Option<TextInputType>,

    /// The text input action configuring the soft keyboard to a particular action button.
    ///
    /// Defaults to [`TextInputAction::Search`].
    pub text_input_action: TextInputAction,

    /// Whether to enable autocorrection in the search field. Defaults to true.
    pub autocorrect: bool,

    /// Whether to show input suggestions as the user types in the search field. Defaults to true.
    pub enable_suggestions: bool,

    automatically_imply_leading: Cell<Option<bool>>,
    leading_width: Cell<Option<f64>>,

    /// The focus node to use for manipulating focus on the search page. This is managed, owned,
    /// and set by the [`SearchPageRoute`] using this delegate.
    focus_node: Cell<Option<Handle<FocusNode>>>,
    query_text_controller: Ref<TextEditingController>,
    proxy_animation: Ref<ProxyAnimation>,
    current_body_notifier: Ref<ValueNotifier<Option<SearchBody>>>,
    route: Cell<Option<Handle<SearchPageRoute>>>,
}

impl SearchDelegateData {
    /// Creates the state of a delegate; Dart's `SearchDelegate({searchFieldLabel,
    /// searchFieldStyle, searchFieldDecorationTheme, keyboardType, textInputAction, autocorrect,
    /// enableSuggestions})` arguments are the setters.
    pub fn new(app: &mut App) -> SearchDelegateData {
        let always_dismissed = k_always_dismissed_animation(app);
        SearchDelegateData {
            search_field_label: None,
            search_field_style: None,
            search_field_decoration_theme: None,
            keyboard_type: None,
            text_input_action: TextInputAction::Search,
            autocorrect: true,
            enable_suggestions: true,
            automatically_imply_leading: Cell::new(None),
            leading_width: Cell::new(None),
            focus_node: Cell::new(None),
            query_text_controller: TextEditingController::new(app),
            proxy_animation: ProxyAnimation::new(app, Some(always_dismissed)),
            current_body_notifier: app.create(ValueNotifier::new(None)),
            route: Cell::new(None),
        }
    }

    /// Dart `SearchDelegate(searchFieldLabel:)`.
    pub fn search_field_label(
        mut self,
        search_field_label: impl Into<String>,
    ) -> SearchDelegateData {
        self.search_field_label = Some(search_field_label.into());
        self
    }

    /// Dart `SearchDelegate(searchFieldStyle:)`.
    pub fn search_field_style(mut self, search_field_style: TextStyle) -> SearchDelegateData {
        self.search_field_style = Some(search_field_style);
        debug_assert!(self.search_field_decoration_theme.is_none());
        self
    }

    /// Dart `SearchDelegate(searchFieldDecorationTheme:)`.
    pub fn search_field_decoration_theme(
        mut self,
        search_field_decoration_theme: InputDecorationThemeData,
    ) -> SearchDelegateData {
        self.search_field_decoration_theme = Some(search_field_decoration_theme);
        debug_assert!(self.search_field_style.is_none());
        self
    }

    /// Dart `SearchDelegate(keyboardType:)`.
    pub fn keyboard_type(mut self, keyboard_type: TextInputType) -> SearchDelegateData {
        self.keyboard_type = Some(keyboard_type);
        self
    }

    /// Dart `SearchDelegate(textInputAction:)`.
    pub fn text_input_action(mut self, text_input_action: TextInputAction) -> SearchDelegateData {
        self.text_input_action = text_input_action;
        self
    }

    /// Dart `SearchDelegate(autocorrect:)`.
    pub fn autocorrect(mut self, autocorrect: bool) -> SearchDelegateData {
        self.autocorrect = autocorrect;
        self
    }

    /// Dart `SearchDelegate(enableSuggestions:)`.
    pub fn enable_suggestions(mut self, enable_suggestions: bool) -> SearchDelegateData {
        self.enable_suggestions = enable_suggestions;
        self
    }

    /// Dart's `SearchDelegate.automaticallyImplyLeading`: whether the app bar of the search page
    /// implies a leading widget. When `None`, it is true.
    pub fn automatically_imply_leading(&self) -> Option<bool> {
        self.automatically_imply_leading.get()
    }

    /// Sets [`automatically_imply_leading`](Self::automatically_imply_leading).
    pub fn set_automatically_imply_leading(&self, value: Option<bool>) {
        self.automatically_imply_leading.set(value);
    }

    /// Dart's `SearchDelegate.leadingWidth`: the width of the app bar's leading widget.
    pub fn leading_width(&self) -> Option<f64> {
        self.leading_width.get()
    }

    /// Sets [`leading_width`](Self::leading_width).
    pub fn set_leading_width(&self, value: Option<f64>) {
        self.leading_width.set(value);
    }
}

/// Delegate for [`show_search`] to define the content of the search page.
///
/// The search page always shows an [`AppBar`] at the top where users can enter their search
/// queries. The buttons shown before and after the search query text field can be customized via
/// [`build_leading`](Self::build_leading) and [`build_actions`](Self::build_actions). Additionally,
/// a widget can be placed across the bottom of the [`AppBar`] via
/// [`build_bottom`](Self::build_bottom).
///
/// The body below the [`AppBar`] can either show suggested queries
/// ([`build_suggestions`](Self::build_suggestions)) or, once the user has submitted a search,
/// the results of the search as returned by [`build_results`](Self::build_results).
///
/// [`query`](Self::query) always contains the current query entered by the user and should be
/// used to build the suggestions and results.
///
/// The results can be brought on screen by calling [`show_results`](Self::show_results) and you
/// can go back to showing the suggestions by calling [`show_suggestions`](Self::show_suggestions).
///
/// Once the user has selected a search result, [`close`](Self::close) should be called to remove
/// the search page from the screen and to return the selected search result back to the caller of
/// [`show_search`].
///
/// A delegate in use by [`show_search`] is shared with the route, so what Dart keeps in the
/// delegate's fields is read through [`search_delegate_data`](Self::search_delegate_data).
pub trait SearchDelegate: 'static {
    /// The state this delegate shares with its route and page.
    fn search_delegate_data(&self) -> &SearchDelegateData;

    /// Suggestions shown in the body of the search page while the user types a query into the
    /// search field.
    ///
    /// The delegate method will be called whenever the content of [`query`](Self::query) changes.
    /// The suggestions should be based on the current [`query`](Self::query) string. If the query
    /// string is empty, it is good practice to show suggested queries based on past queries or the
    /// current context.
    ///
    /// Usually, this method will return a [`ListView`](waft_widgets::ListView) with one
    /// [`ListTile`](crate::ListTile) per suggestion. When [`ListTile::on_tap`] is called,
    /// [`query`](Self::query) should be updated with the corresponding suggestion and the results
    /// page should be shown by calling [`show_results`](Self::show_results).
    fn build_suggestions(&self, app: &mut App, context: BuildContext) -> WidgetRef;

    /// The results shown after the user submits a search from the search page.
    ///
    /// The current value of [`query`](Self::query) can be used to determine what the user searched
    /// for.
    ///
    /// This method might be applied more than once to the same query. If your method is
    /// computationally expensive, you may want to cache the search results for one or more queries.
    ///
    /// Typically, this method returns a [`ListView`](waft_widgets::ListView) with the search
    /// results. When the user taps on a particular search result, [`close`](Self::close) should be
    /// called with the selected result as argument. This will close the search page and communicate
    /// the result back to the initial caller of [`show_search`].
    fn build_results(&self, app: &mut App, context: BuildContext) -> WidgetRef;

    /// A widget to display before the current query in the [`AppBar`].
    ///
    /// Typically an [`IconButton`](crate::IconButton) configured with a
    /// [`BackButtonIcon`](crate::BackButtonIcon) that exits the search with
    /// [`close`](Self::close). One can also use an [`AnimatedIcon`](waft_widgets::AnimatedIcon)
    /// driven by [`transition_animation`](Self::transition_animation), which animates from e.g. a
    /// hamburger menu to the back button as the search overlay fades in.
    ///
    /// Returns `None` if no widget should be shown.
    fn build_leading(&self, app: &mut App, context: BuildContext) -> Option<WidgetRef>;

    /// Widgets to display after the search query in the [`AppBar`].
    ///
    /// If the [`query`](Self::query) is not empty, this should typically contain a button to clear
    /// the query and show the suggestions again (via [`show_suggestions`](Self::show_suggestions))
    /// if the results are currently shown.
    ///
    /// Returns `None` if no widget should be shown.
    fn build_actions(&self, app: &mut App, context: BuildContext) -> Option<Vec<WidgetRef>>;

    /// Widget to display across the bottom of the [`AppBar`].
    ///
    /// Typically a [`PreferredSize`](waft_widgets::PreferredSize) widget containing a widget that
    /// provides additional information for the current search.
    ///
    /// Returns `None` by default, i.e. a bottom widget is not included.
    fn build_bottom(&self, app: &mut App, context: BuildContext) -> Option<PreferredSizeWidgetRef> {
        let _ = (app, context);
        None
    }

    /// Widget to display behind the current query in the [`AppBar`].
    ///
    /// Returns `None` by default, i.e. a flexible space widget is not included.
    fn build_flexible_space(&self, app: &mut App, context: BuildContext) -> Option<WidgetRef> {
        let _ = (app, context);
        None
    }

    /// The theme used to style the [`AppBar`].
    ///
    /// By default, a white theme is used.
    ///
    /// If [`SearchDelegateData::search_field_decoration_theme`] is set, it will be used to
    /// override [`ThemeData::input_decoration_theme`]. Otherwise a theme whose hint style is
    /// [`SearchDelegateData::search_field_style`] (or the ambient hint style) and whose border is
    /// none is used.
    fn app_bar_theme(&self, app: &mut App, context: BuildContext) -> ThemeData {
        default_app_bar_theme(self.search_delegate_data(), app, context)
    }

    /// The current query string shown in the [`AppBar`].
    ///
    /// The user manipulates this string via the keyboard.
    ///
    /// If the user taps on a suggestion provided by
    /// [`build_suggestions`](Self::build_suggestions) this string should be updated to that
    /// suggestion via the setter.
    fn query(&self, app: &App) -> String {
        self.search_delegate_data()
            .query_text_controller
            .handle()
            .value(app)
            .text
            .clone()
    }

    /// Changes the current query string.
    ///
    /// Setting the query string programmatically moves the cursor to the end of the text field.
    fn set_query(&self, app: &mut App, value: String) {
        let offset = value.chars().count();
        self.search_delegate_data()
            .query_text_controller
            .handle()
            .set_value(
                app,
                TextEditingValue {
                    text: value,
                    selection: TextSelection::collapsed(offset as i32, TextAffinity::Downstream),
                    ..TextEditingValue::default()
                },
            );
    }

    /// Transition from the suggestions returned by [`build_suggestions`](Self::build_suggestions)
    /// to the `query` results returned by [`build_results`](Self::build_results).
    ///
    /// If the user taps on a suggestion provided by
    /// [`build_suggestions`](Self::build_suggestions) the existing search page will be
    /// transformed to show the results of the suggestion.
    fn show_results(&self, app: &mut App, context: BuildContext) {
        let _ = context;
        let data = self.search_delegate_data();
        if let Some(focus_node) = data.focus_node.get() {
            focus_node.unfocus(app, UnfocusDisposition::Scope);
        }
        data.current_body_notifier
            .handle()
            .set_value(app, Some(SearchBody::Results));
    }

    /// Transition from showing the results returned by
    /// [`build_results`](Self::build_results) to showing the suggestions returned by
    /// [`build_suggestions`](Self::build_suggestions).
    ///
    /// Calling this method will also put the input focus back into the search field of the
    /// [`AppBar`].
    ///
    /// If the results are currently shown this method can be used to go back to showing the
    /// search suggestions.
    fn show_suggestions(&self, app: &mut App, context: BuildContext) {
        let _ = context;
        let data = self.search_delegate_data();
        let focus_node = data
            .focus_node
            .get()
            .expect("_focusNode must be set by route before showSuggestions is called.");
        focus_node.request_focus(app, None);
        data.current_body_notifier
            .handle()
            .set_value(app, Some(SearchBody::Suggestions));
    }

    /// Closes the search page and returns `result` to the caller of [`show_search`].
    ///
    /// Might be called when the user taps on a search result shown by
    /// [`build_results`](Self::build_results).
    fn close(&self, app: &mut App, context: BuildContext, result: RouteResult) {
        let data = self.search_delegate_data();
        data.current_body_notifier.handle().set_value(app, None);
        if let Some(focus_node) = data.focus_node.get() {
            focus_node.unfocus(app, UnfocusDisposition::Scope);
        }
        pop_until_route_then_pop(app, context, data.route.get(), result);
    }

    /// The animation that drives the route's transition and the support for changing the current
    /// body.
    fn transition_animation(&self) -> AnimationRef<f64> {
        self.search_delegate_data().proxy_animation.erase()
    }

    /// Releases the resources of this delegate.
    ///
    /// Call this when the delegate is no longer used.
    fn dispose(&self, app: &mut App) {
        let data = self.search_delegate_data();
        data.current_body_notifier.handle().dispose(app);
        if let Some(focus_node) = data.focus_node.get() {
            focus_node.dispose(app);
        }
        data.query_text_controller.handle().dispose(app);
        data.proxy_animation.handle().set_parent(app, None);
    }
}

/// [`SearchDelegate::app_bar_theme`]'s default, which a delegate that overrides the method
/// reaches as Dart's `super.appBarTheme(context)`.
pub fn default_app_bar_theme(
    data: &SearchDelegateData,
    app: &mut App,
    context: BuildContext,
) -> ThemeData {
    let theme = Theme::of(app, context);
    let color_scheme = &theme.color_scheme;
    let dark = color_scheme.brightness == waft_embedder::Brightness::Dark;
    let input_decoration_theme = data
        .search_field_decoration_theme
        .clone()
        .unwrap_or_else(|| {
            let hint_style = data
                .search_field_style
                .clone()
                .map(waft_painting::AnyTextStyle::from)
                .or_else(|| theme.input_decoration_theme.hint_style.clone());
            let mut decoration_theme =
                InputDecorationThemeData::new().border(InputBorderRef::none());
            if let Some(hint_style) = hint_style {
                decoration_theme = decoration_theme.hint_style(hint_style);
            }
            decoration_theme
        });
    let mut app_bar_theme = AppBarThemeData::new()
        .system_overlay_style(if dark {
            SystemUiOverlayStyle::LIGHT
        } else {
            SystemUiOverlayStyle::DARK
        })
        .background_color(if dark {
            Colors::GREY.shade900()
        } else {
            Colors::WHITE
        })
        .icon_theme(
            theme
                .primary_icon_theme
                .copy_with()
                .color(Colors::GREY.primary()),
        );
    if let Some(title_large) = theme.text_theme.title_large.clone() {
        app_bar_theme = app_bar_theme.title_text_style(title_large);
    }
    if let Some(body_medium) = theme.text_theme.body_medium.clone() {
        app_bar_theme = app_bar_theme.toolbar_text_style(body_medium);
    }
    theme
        .copy_with()
        .app_bar_theme(app_bar_theme)
        .input_decoration_theme(input_decoration_theme)
        .build()
}

/// Dart's `Navigator.of(context)..popUntil((route) => route == _route)..pop(result)`.
fn pop_until_route_then_pop(
    app: &mut App,
    context: BuildContext,
    route: Option<Handle<SearchPageRoute>>,
    result: RouteResult,
) {
    let navigator = Navigator::of(app, context, false);
    navigator.pop_until(
        app,
        Rc::new(move |_app, candidate: AnyRoute| {
            route.is_some_and(|route| candidate.id() == route.id())
        }),
    );
    navigator.pop(app, result);
}

/// Dart's `_pop`: the escape key's close, which answers no result.
fn pop_search(delegate: &dyn SearchDelegate, app: &mut App, context: BuildContext) {
    let data = delegate.search_delegate_data();
    data.current_body_notifier.handle().set_value(app, None);
    if let Some(focus_node) = data.focus_node.get() {
        focus_node.unfocus(app, UnfocusDisposition::Scope);
    }
    pop_until_route_then_pop(app, context, data.route.get(), None);
}

/// Dart's `_SearchPageRoute`.
pub struct SearchPageRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,
    page_route: PageRouteData,
    delegate: Rc<dyn SearchDelegate>,
    maintain_state: bool,
}

impl SearchPageRoute {
    fn new(
        app: &mut App,
        delegate: Rc<dyn SearchDelegate>,
        maintain_state: bool,
    ) -> Ref<SearchPageRoute> {
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let this = app.create(SearchPageRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            page_route: PageRouteData::new(),
            delegate: Rc::clone(&delegate),
            maintain_state,
        });
        assert!(
            delegate.search_delegate_data().route.get().is_none(),
            "The delegate instance is currently used by another active search. Please close that \
             search by calling close() on the SearchDelegate before opening another search with \
             the same delegate instance."
        );
        delegate
            .search_delegate_data()
            .route
            .set(Some(this.handle()));
        this
    }
}

impl Route for SearchPageRoute {
    waft_widgets::page_route_overrides!();

    fn did_complete(self: Handle<Self>, app: &mut App, result: RouteResult) {
        waft_widgets::RouteBase::did_complete(self, app, result);
        let delegate = Rc::clone(&app.get(self).delegate);
        let data = delegate.search_delegate_data();
        debug_assert!(
            data.route
                .get()
                .is_some_and(|route| route.id() == self.id())
        );
        data.route.set(None);
        data.current_body_notifier.handle().set_value(app, None);
    }
}

impl OverlayRoute for SearchPageRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for SearchPageRoute {
    waft_widgets::page_predictive_back_route_overrides!();
}

impl TransitionRoute for SearchPageRoute {
    waft_widgets::page_transition_route_overrides!();

    fn transition_duration(self: Handle<Self>, app: &mut App) -> Duration {
        let _ = app;
        Duration::from_millis(300)
    }

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PageRoute::opaque(self, app)
    }

    fn create_animation(self: Handle<Self>, app: &mut App) -> AnimationRef<f64> {
        // Dart's `super.createAnimation()`: the trait's default body.
        debug_assert!(
            !TransitionRoute::debug_transition_completed(self, app),
            "cannot reuse a route after disposing it"
        );
        let animation: AnimationRef<f64> = TransitionRoute::controller(self, app)
            .expect("createAnimationController ran first")
            .keep(app)
            .into_erased();
        let delegate = Rc::clone(&app.get(self).delegate);
        delegate
            .search_delegate_data()
            .proxy_animation
            .handle()
            .set_parent(app, Some(animation.clone()));
        animation
    }
}

impl LocalHistoryRoute for SearchPageRoute {
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for SearchPageRoute {
    waft_widgets::modal_route_accessors!();

    fn barrier_dismissible(self: Handle<Self>, app: &App) -> bool {
        PageRoute::barrier_dismissible(self, app)
    }

    fn barrier_color(self: Handle<Self>, app: &App) -> Option<waft_painting::Color> {
        let _ = app;
        None
    }

    fn barrier_label(self: Handle<Self>, app: &App) -> Option<String> {
        let _ = app;
        None
    }

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        app.get(self).maintain_state
    }

    fn fullscreen_dialog(self: Handle<Self>, app: &App) -> bool {
        PageRoute::fullscreen_dialog(self, app)
    }

    fn build_page(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let _ = (context, secondary_animation);
        SearchPage {
            delegate: Rc::clone(&app.get(self).delegate),
            animation,
        }
        .into_widget()
    }

    fn build_transitions(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
        child: WidgetRef,
    ) -> WidgetRef {
        let _ = (app, context, secondary_animation);
        FadeTransition::new(animation).child(child).into_widget()
    }
}

impl PageRoute for SearchPageRoute {
    waft_widgets::page_route_accessors!();
}

/// Dart's `_SearchPage`.
struct SearchPage {
    delegate: Rc<dyn SearchDelegate>,
    animation: AnimationRef<f64>,
}

impl fmt::Debug for SearchPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SearchPage").finish_non_exhaustive()
    }
}

impl StatefulWidget for SearchPage {
    type State = SearchPageState;

    fn create_state(&self) -> SearchPageState {
        SearchPageState {
            state: StateData::new(),
            focus_node: None,
        }
    }
}

/// Dart's `_SearchPageState`.
struct SearchPageState {
    state: StateData<SearchPage>,
    /// This node is owned, but not hosted by, the search page. Hosting is done by the text
    /// field.
    focus_node: Option<Ref<FocusNode>>,
}

impl SearchPageState {
    fn on_query_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
    }

    fn on_search_body_changed(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_| {});
    }

    fn on_focus_changed(self: Handle<Self>, app: &mut App) {
        let focus_node = self.focus_node(app);
        let delegate = Rc::clone(&self.widget(app).delegate);
        let current_body = *delegate
            .search_delegate_data()
            .current_body_notifier
            .handle()
            .value(app);
        if focus_node.has_focus(app) && current_body != Some(SearchBody::Suggestions) {
            delegate.show_suggestions(app, self.context(app));
        }
    }

    fn on_animation_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if !status.is_completed() {
            return;
        }
        let animation = self.widget(app).animation.clone();
        animation.remove_status_listener(app, &self.status_listener());
        let delegate = Rc::clone(&self.widget(app).delegate);
        let current_body = *delegate
            .search_delegate_data()
            .current_body_notifier
            .handle()
            .value(app);
        if current_body == Some(SearchBody::Suggestions) {
            self.focus_node(app).request_focus(app, None);
        }
    }

    fn status_listener(self: Handle<Self>) -> AnimationStatusListener {
        AnimationStatusListener::handle_method(self, Self::on_animation_status_changed)
    }

    fn query_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::on_query_changed)
    }

    fn body_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::on_search_body_changed)
    }

    fn focus_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::on_focus_changed)
    }

    fn focus_node(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self)
            .focus_node
            .as_ref()
            .expect("initState made it")
            .handle()
    }
}

impl State for SearchPageState {
    type Widget = SearchPage;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let delegate = Rc::clone(&self.widget(app).delegate);
        let focus_node = FocusNode::new(app);
        {
            let handle = self;
            focus_node.handle().set_on_key_event(
                app,
                Some(Rc::new(
                    move |app, _node: AnyFocusNode, event: &KeyEvent| {
                        // When the user presses the escape key, close the search page.
                        if matches!(event, KeyEvent::Down(_))
                            && event.logical_key() == LogicalKeyboardKey::ESCAPE
                        {
                            let delegate = Rc::clone(&handle.widget(app).delegate);
                            let context = handle.context(app);
                            pop_search(&*delegate, app, context);
                            return KeyEventResult::Handled;
                        }
                        KeyEventResult::Ignored
                    },
                )),
            );
        }
        app.get_mut(self).focus_node = Some(focus_node);
        let data = delegate.search_delegate_data();
        data.query_text_controller
            .handle()
            .add_listener(app, self.query_listener());
        let animation = self.widget(app).animation.clone();
        animation.add_status_listener(app, self.status_listener());
        data.current_body_notifier
            .handle()
            .add_listener(app, self.body_listener());
        self.focus_node(app)
            .add_listener(app, self.focus_listener());
        data.focus_node.set(Some(self.focus_node(app)));
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let delegate = Rc::clone(&self.widget(app).delegate);
        let data = delegate.search_delegate_data();
        data.query_text_controller
            .handle()
            .remove_listener(app, &self.query_listener());
        let animation = self.widget(app).animation.clone();
        animation.remove_status_listener(app, &self.status_listener());
        data.current_body_notifier
            .handle()
            .remove_listener(app, &self.body_listener());
        data.focus_node.set(None);
        self.focus_node(app).dispose(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SearchPage) {
        let delegate = Rc::clone(&self.widget(app).delegate);
        if !Rc::ptr_eq(&delegate, &old_widget.delegate) {
            let old = old_widget.delegate.search_delegate_data();
            let new = delegate.search_delegate_data();
            old.query_text_controller
                .handle()
                .remove_listener(app, &self.query_listener());
            new.query_text_controller
                .handle()
                .add_listener(app, self.query_listener());
            old.current_body_notifier
                .handle()
                .remove_listener(app, &self.body_listener());
            new.current_body_notifier
                .handle()
                .add_listener(app, self.body_listener());
            old.focus_node.set(None);
            new.focus_node.set(Some(self.focus_node(app)));
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material_localizations(app, context));
        let delegate = Rc::clone(&self.widget(app).delegate);
        let data = delegate.search_delegate_data();
        let theme = Rc::new(delegate.app_bar_theme(app, context));
        let search_field_label = data
            .search_field_label
            .clone()
            .unwrap_or_else(|| <dyn MaterialLocalizations>::of(app, context).search_field_label());
        let body = match *data.current_body_notifier.handle().value(app) {
            Some(SearchBody::Suggestions) => Some(
                KeyedSubtree::new(delegate.build_suggestions(app, context))
                    .key(ValueKey::new(SearchBody::Suggestions).into())
                    .into_widget(),
            ),
            Some(SearchBody::Results) => Some(
                KeyedSubtree::new(delegate.build_results(app, context))
                    .key(ValueKey::new(SearchBody::Results).into())
                    .into_widget(),
            ),
            None => None,
        };

        let route_name = match theme.platform {
            waft_embedder::TargetPlatform::IOS | waft_embedder::TargetPlatform::MacOS => {
                String::new()
            }
            _ => search_field_label.clone(),
        };

        let delegate_for_submit = Rc::clone(&delegate);
        let text_field = TextField::new()
            .controller(data.query_text_controller.erase())
            .focus_node(
                app.get(self)
                    .focus_node
                    .clone()
                    .expect("initState made it")
                    .into_erased(),
            )
            .text_input_action(data.text_input_action)
            .autocorrect(data.autocorrect)
            .enable_suggestions(data.enable_suggestions)
            .on_submitted(move |app, _| {
                delegate_for_submit.show_results(app, context);
            })
            .decoration(Some(InputDecoration::new().hint_text(search_field_label)));
        let text_field = match data
            .search_field_style
            .clone()
            .or_else(|| theme.text_theme.title_large.clone())
        {
            Some(style) => text_field.style(style),
            None => text_field,
        };
        let text_field = match data.keyboard_type {
            Some(keyboard_type) => text_field.keyboard_type(keyboard_type),
            None => text_field,
        };

        let mut app_bar = AppBar::new()
            .automatically_imply_leading(data.automatically_imply_leading().unwrap_or(true))
            .title(
                Semantics::new()
                    .input_type(SemanticsInputType::Search)
                    .child(text_field),
            );
        if let Some(leading_width) = data.leading_width() {
            app_bar = app_bar.leading_width(leading_width);
        }
        if let Some(leading) = delegate.build_leading(app, context) {
            app_bar = app_bar.leading(leading);
        }
        if let Some(flexible_space) = delegate.build_flexible_space(app, context) {
            app_bar = app_bar.flexible_space(flexible_space);
        }
        if let Some(actions) = delegate.build_actions(app, context) {
            app_bar = app_bar.actions(actions);
        }
        if let Some(bottom) = delegate.build_bottom(app, context) {
            app_bar = app_bar.bottom(bottom);
        }

        let mut switcher = AnimatedSwitcher::new(Duration::from_millis(300));
        if let Some(body) = body {
            switcher = switcher.child(body);
        }
        Semantics::new()
            .explicit_child_nodes(true)
            .scopes_route(true)
            .names_route(true)
            .label(route_name)
            .child(Theme::new(
                theme,
                Scaffold::new()
                    .app_bar(PreferredSizeWidgetRef::new(app_bar))
                    .body(switcher),
            ))
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_animation::AnimationStatus;
    use waft_embedder::{Color, Offset, Size};
    use waft_foundation::Retained;
    use waft_services::PhysicalKeyboardKey;
    use waft_test::{MockClipboard, TestPlatform};
    use waft_widgets::testing::{
        WidgetTester, expect, find, finds_nothing, finds_one_widget, finds_widgets, test_widgets,
    };
    use waft_widgets::{EditableText, PreferredSize, RouteSettingsRef, Text, WidgetsBinding};

    use super::*;
    use crate::elevated_button::ElevatedButton;
    use crate::icon_button::IconButton;
    use crate::icons::Icons;
    use crate::test_support::material_app;
    use crate::tooltip::tests::m3;

    /// The constructor arguments of Dart's `_TestSearchDelegate` that are not the base class's.
    struct Options {
        suggestions: &'static str,
        result: &'static str,
        actions: Vec<WidgetRef>,
        flexible_space: Option<WidgetRef>,
        default_app_bar_theme: bool,
    }

    impl Default for Options {
        fn default() -> Options {
            Options {
                suggestions: "Suggestions",
                result: "Result",
                actions: Vec::new(),
                flexible_space: None,
                default_app_bar_theme: false,
            }
        }
    }

    /// Dart's `_TestSearchDelegate`.
    struct TestSearchDelegate {
        me: std::rc::Weak<TestSearchDelegate>,
        data: SearchDelegateData,
        options: Options,
        queries_for_suggestions: RefCell<Vec<String>>,
        queries_for_results: RefCell<Vec<String>>,
    }

    impl TestSearchDelegate {
        fn new(
            app: &mut App,
            data: impl FnOnce(SearchDelegateData) -> SearchDelegateData,
        ) -> Rc<TestSearchDelegate> {
            TestSearchDelegate::with_options(app, data, Options::default())
        }

        fn with_options(
            app: &mut App,
            data: impl FnOnce(SearchDelegateData) -> SearchDelegateData,
            options: Options,
        ) -> Rc<TestSearchDelegate> {
            let data = data(SearchDelegateData::new(app));
            Rc::new_cyclic(|me| TestSearchDelegate {
                me: me.clone(),
                data,
                options,
                queries_for_suggestions: RefCell::default(),
                queries_for_results: RefCell::default(),
            })
        }

        fn this(&self) -> Rc<TestSearchDelegate> {
            self.me.upgrade().expect("the delegate is alive")
        }
    }

    impl SearchDelegate for TestSearchDelegate {
        fn search_delegate_data(&self) -> &SearchDelegateData {
            &self.data
        }

        fn app_bar_theme(&self, app: &mut App, context: BuildContext) -> ThemeData {
            if self.options.default_app_bar_theme {
                return default_app_bar_theme(&self.data, app, context);
            }
            let theme = Theme::of(app, context);
            let decoration_theme = self
                .data
                .search_field_decoration_theme
                .clone()
                .unwrap_or_else(|| {
                    InputDecorationThemeData::new().hint_style(
                        self.data
                            .search_field_style
                            .clone()
                            .unwrap_or_else(|| TextStyle::new().color(hint_text_color())),
                    )
                });
            theme
                .copy_with()
                .input_decoration_theme(decoration_theme)
                .build()
        }

        fn build_leading(&self, _app: &mut App, context: BuildContext) -> Option<WidgetRef> {
            let delegate = self.this();
            Some(
                IconButton::new(
                    Some(Listener::new(move |app: &mut App| {
                        delegate.close(
                            app,
                            context,
                            Some(Rc::new(delegate.options.result.to_owned())),
                        );
                    })),
                    waft_widgets::Icon::new(Some(Icons::arrow_back())),
                )
                .tooltip("Back")
                .into_widget(),
            )
        }

        fn build_suggestions(&self, app: &mut App, context: BuildContext) -> WidgetRef {
            let query = self.query(app);
            self.queries_for_suggestions.borrow_mut().push(query);
            let delegate = self.this();
            ElevatedButton::new(
                Some(Listener::new(move |app: &mut App| {
                    delegate.show_results(app, context);
                })),
                Some(Text::new(self.options.suggestions)),
            )
            .into_widget()
        }

        fn build_results(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
            let query = self.query(app);
            self.queries_for_results.borrow_mut().push(query);
            Text::new("Results").into_widget()
        }

        fn build_actions(&self, _app: &mut App, _context: BuildContext) -> Option<Vec<WidgetRef>> {
            Some(self.options.actions.clone())
        }

        fn build_flexible_space(
            &self,
            _app: &mut App,
            _context: BuildContext,
        ) -> Option<WidgetRef> {
            self.options.flexible_space.clone()
        }

        fn build_bottom(
            &self,
            _app: &mut App,
            _context: BuildContext,
        ) -> Option<PreferredSizeWidgetRef> {
            Some(PreferredSizeWidgetRef::new(PreferredSize::new(
                Size::new(f64::INFINITY, 56.0),
                Text::new("Bottom"),
            )))
        }
    }

    /// Dart's `_TestSearchDelegate.hintTextColor`.
    fn hint_text_color() -> Color {
        Colors::GREEN.primary()
    }

    /// Dart's `_TestEmptySearchDelegate`: no leading, no actions, and a `Close` button as the
    /// bottom.
    struct EmptySearchDelegate {
        me: std::rc::Weak<EmptySearchDelegate>,
        data: SearchDelegateData,
    }

    impl EmptySearchDelegate {
        fn new(app: &mut App) -> Rc<EmptySearchDelegate> {
            let data = SearchDelegateData::new(app);
            Rc::new_cyclic(|me| EmptySearchDelegate {
                me: me.clone(),
                data,
            })
        }
    }

    impl SearchDelegate for EmptySearchDelegate {
        fn search_delegate_data(&self) -> &SearchDelegateData {
            &self.data
        }

        fn build_leading(&self, _app: &mut App, _context: BuildContext) -> Option<WidgetRef> {
            None
        }

        fn build_actions(&self, _app: &mut App, _context: BuildContext) -> Option<Vec<WidgetRef>> {
            None
        }

        fn build_suggestions(&self, _app: &mut App, context: BuildContext) -> WidgetRef {
            let delegate = self.me.upgrade().expect("the delegate is alive");
            ElevatedButton::new(
                Some(Listener::new(move |app: &mut App| {
                    delegate.show_results(app, context);
                })),
                Some(Text::new("Suggestions")),
            )
            .into_widget()
        }

        fn build_results(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
            Text::new("Results").into_widget()
        }

        fn build_bottom(
            &self,
            _app: &mut App,
            context: BuildContext,
        ) -> Option<PreferredSizeWidgetRef> {
            let delegate = self.me.upgrade().expect("the delegate is alive");
            Some(PreferredSizeWidgetRef::new(PreferredSize::new(
                Size::new(f64::INFINITY, 56.0),
                IconButton::new(
                    Some(Listener::new(move |app: &mut App| {
                        delegate.close(app, context, Some(Rc::new("Result".to_owned())));
                    })),
                    waft_widgets::Icon::new(Some(Icons::arrow_back())),
                )
                .tooltip("Close"),
            )))
        }
    }

    /// The arguments of Dart's `TestHomePage` besides the delegate and the results.
    #[derive(Default)]
    struct HomeOptions {
        pass_in_initial_query: bool,
        initial_query: Option<&'static str>,
        theme: Option<ThemeData>,
        use_root_navigator: bool,
        maintain_state: bool,
    }

    type Results = Rc<RefCell<Vec<Option<String>>>>;

    /// Dart's `TestHomePage`.
    fn home_page_with(
        tester: &mut WidgetTester,
        delegate: Rc<dyn SearchDelegate>,
        results: &Results,
        options: HomeOptions,
    ) {
        let results = Rc::clone(results);
        let theme = options
            .theme
            .clone()
            .unwrap_or_else(|| m3(&tester.cell.borrow()));
        let HomeOptions {
            pass_in_initial_query,
            initial_query,
            use_root_navigator,
            maintain_state,
            ..
        } = options;
        tester.pump_widget(material_app(
            theme,
            waft_widgets::Builder::new(move |_app, _context| {
                let delegate = Rc::clone(&delegate);
                let results = Rc::clone(&results);
                Scaffold::new()
                    .app_bar(PreferredSizeWidgetRef::new(
                        AppBar::new().title(Text::new("HomeTitle")).actions(vec![
                            waft_widgets::Builder::new(move |_app, context| {
                                let delegate = Rc::clone(&delegate);
                                let results = Rc::clone(&results);
                                IconButton::new(
                                    Some(Listener::new(move |app: &mut App| {
                                        let results = Rc::clone(&results);
                                        show_search(
                                            app,
                                            context,
                                            Rc::clone(&delegate),
                                            // Dart's `query` argument defaults to `''`.
                                            if pass_in_initial_query {
                                                initial_query
                                            } else {
                                                Some("")
                                            },
                                            use_root_navigator,
                                            maintain_state,
                                        )
                                        .then(
                                            app,
                                            move |_app, result| {
                                                results.borrow_mut().push(result.and_then(|r| {
                                                    r.downcast_ref::<String>().cloned()
                                                }));
                                            },
                                        );
                                    })),
                                    waft_widgets::Icon::new(Some(Icons::search())),
                                )
                                .tooltip("Search")
                                .into_widget()
                            })
                            .into_widget(),
                        ]),
                    ))
                    .body(Text::new("HomeBody"))
                    .into_widget()
            })
            .into_widget(),
        ));
    }

    /// Dart's `TestHomePage(delegate:, results:)`.
    fn home_page(
        tester: &mut WidgetTester,
        delegate: &Rc<TestSearchDelegate>,
        results: &Results,
        initial_query: Option<&'static str>,
    ) {
        home_page_with(
            tester,
            Rc::clone(delegate) as Rc<dyn SearchDelegate>,
            results,
            HomeOptions {
                pass_in_initial_query: initial_query.is_some(),
                initial_query,
                ..HomeOptions::default()
            },
        );
    }

    fn open_search(tester: &mut WidgetTester) {
        tester.tap(find::by_tooltip("Search"));
        tester.pump_and_settle();
    }

    fn text_present(tester: &WidgetTester, text: &str) -> bool {
        !find::text(text).evaluate(tester).is_empty()
    }

    fn field_has_focus(tester: &WidgetTester) -> bool {
        let focus_node = tester.widget(find::by_type::<TextField>(), |field: &TextField| {
            field.focus_node.clone().expect("a focus node")
        });
        focus_node.handle().has_focus(&mut tester.cell.borrow_mut())
    }

    #[test]
    fn changing_query_moves_cursor_to_the_end_of_query() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            tester.tap(find::by_tooltip("Search"));
            tester.pump();
            tester.pump_by(Duration::from_millis(300));

            delegate.set_query(&mut tester.cell.borrow_mut(), "Foo".to_owned());
            let selection = delegate
                .search_delegate_data()
                .query_text_controller
                .handle()
                .value(&tester.cell.borrow())
                .selection;
            assert_eq!(selection.base_offset, "Foo".len() as i32);
            assert_eq!(selection.extent_offset, "Foo".len() as i32);

            delegate.set_query(&mut tester.cell.borrow_mut(), String::new());
            let selection = delegate
                .search_delegate_data()
                .query_text_controller
                .handle()
                .value(&tester.cell.borrow())
                .selection;
            assert_eq!((selection.base_offset, selection.extent_offset), (0, 0));
        });
    }

    #[test]
    fn can_open_and_close_search() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);

            expect(tester, &find::text("HomeBody"), finds_one_widget());
            expect(tester, &find::text("HomeTitle"), finds_one_widget());
            expect(tester, &find::text("Suggestions"), finds_nothing());

            open_search(tester);
            expect(tester, &find::text("HomeBody"), finds_nothing());
            expect(tester, &find::text("HomeTitle"), finds_nothing());
            expect(tester, &find::text("Suggestions"), finds_one_widget());
            assert!(results.borrow().is_empty());
            assert!(field_has_focus(tester));

            tester.tap(find::by_tooltip("Back"));
            tester.pump_and_settle();
            expect(tester, &find::text("HomeBody"), finds_one_widget());
            expect(tester, &find::text("HomeTitle"), finds_one_widget());
            expect(tester, &find::text("Suggestions"), finds_nothing());
            assert_eq!(*results.borrow(), vec![Some("Result".to_owned())]);
        });
    }

    #[test]
    fn requests_suggestions() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            open_search(tester);
            assert_eq!(delegate.query(&tester.cell.borrow()), "");
            assert_eq!(
                delegate
                    .queries_for_suggestions
                    .borrow()
                    .last()
                    .map(String::as_str),
                Some("")
            );
            assert!(delegate.queries_for_results.borrow().is_empty());

            delegate.queries_for_suggestions.borrow_mut().clear();
            for typed in ["W", "Wo", "Wow"] {
                tester.enter_text(find::by_type::<EditableText>(), typed);
                tester.pump_and_settle();
                assert_eq!(delegate.query(&tester.cell.borrow()), typed);
            }
            assert_eq!(
                *delegate.queries_for_suggestions.borrow(),
                vec!["W", "Wo", "Wow"]
            );
            assert!(delegate.queries_for_results.borrow().is_empty());
        });
    }

    #[test]
    fn shows_results_and_closes_search() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            open_search(tester);
            tester.enter_text(find::by_type::<EditableText>(), "Wow");
            tester.pump_and_settle();
            tester.tap(find::text("Suggestions"));
            tester.pump_and_settle();

            expect(tester, &find::text("HomeBody"), finds_nothing());
            expect(tester, &find::text("HomeTitle"), finds_nothing());
            expect(tester, &find::text("Suggestions"), finds_nothing());
            expect(tester, &find::text("Results"), finds_one_widget());
            assert!(!field_has_focus(tester));
            assert_eq!(*delegate.queries_for_results.borrow(), vec!["Wow"]);

            tester.tap(find::by_tooltip("Back"));
            tester.pump_and_settle();
            expect(tester, &find::text("HomeBody"), finds_one_widget());
            expect(tester, &find::text("HomeTitle"), finds_one_widget());
            expect(tester, &find::text("Results"), finds_nothing());
            assert_eq!(*results.borrow(), vec![Some("Result".to_owned())]);
        });
    }

    #[test]
    fn can_switch_between_results_and_suggestions() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            open_search(tester);
            expect(tester, &find::text("Suggestions"), finds_one_widget());
            expect(tester, &find::text("Results"), finds_nothing());

            delegate.queries_for_suggestions.borrow_mut().clear();
            tester.enter_text(find::by_type::<EditableText>(), "Wow");
            tester.pump_and_settle();
            assert_eq!(*delegate.queries_for_suggestions.borrow(), vec!["Wow"]);
            assert!(delegate.queries_for_results.borrow().is_empty());

            tester.tap(find::text("Suggestions"));
            tester.pump_and_settle();
            expect(tester, &find::text("Suggestions"), finds_nothing());
            expect(tester, &find::text("Results"), finds_one_widget());
            assert_eq!(*delegate.queries_for_suggestions.borrow(), vec!["Wow"]);
            assert_eq!(*delegate.queries_for_results.borrow(), vec!["Wow"]);
            assert!(!field_has_focus(tester));

            tester.tap(find::by_type::<TextField>());
            tester.pump_and_settle();
            assert!(field_has_focus(tester));
            expect(tester, &find::text("Suggestions"), finds_one_widget());
            expect(tester, &find::text("Results"), finds_nothing());
            assert_eq!(
                *delegate.queries_for_suggestions.borrow(),
                vec!["Wow", "Wow"]
            );
            assert_eq!(*delegate.queries_for_results.borrow(), vec!["Wow"]);

            tester.enter_text(find::by_type::<EditableText>(), "Foo");
            tester.pump_and_settle();
            assert_eq!(delegate.query(&tester.cell.borrow()), "Foo");
            assert_eq!(
                *delegate.queries_for_suggestions.borrow(),
                vec!["Wow", "Wow", "Foo"]
            );

            tester.tap(find::text("Suggestions"));
            tester.pump_and_settle();
            expect(tester, &find::text("Results"), finds_one_widget());
            assert_eq!(*delegate.queries_for_results.borrow(), vec!["Wow", "Foo"]);
            assert!(!field_has_focus(tester));
        });
    }

    #[test]
    fn fresh_search_always_starts_with_empty_query() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            open_search(tester);
            assert_eq!(delegate.query(&tester.cell.borrow()), "");
            delegate.set_query(&mut tester.cell.borrow_mut(), "Foo".to_owned());

            tester.tap(find::by_tooltip("Back"));
            tester.pump_and_settle();
            open_search(tester);
            assert_eq!(delegate.query(&tester.cell.borrow()), "");
        });
    }

    #[test]
    fn initial_queries_are_honored() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            assert_eq!(delegate.query(&tester.cell.borrow()), "");
            home_page(tester, &delegate, &results, Some("Foo"));
            open_search(tester);
            assert_eq!(delegate.query(&tester.cell.borrow()), "Foo");
        });
    }

    #[test]
    fn changing_query_shows_up_in_search_field() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            open_search(tester);

            delegate.set_query(&mut tester.cell.borrow_mut(), "Foo".to_owned());
            assert!(text_present(tester, "Foo"));
            assert!(!text_present(tester, "Bar"));

            delegate.set_query(&mut tester.cell.borrow_mut(), "Bar".to_owned());
            assert!(!text_present(tester, "Foo"));
            assert!(text_present(tester, "Bar"));
        });
    }

    #[test]
    fn transition_animation_runs_while_search_fades_in_and_out() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            let status = |tester: &WidgetTester| {
                delegate
                    .transition_animation()
                    .status(&tester.cell.borrow())
            };

            assert_eq!(status(tester), AnimationStatus::Dismissed);
            tester.tap(find::by_tooltip("Search"));
            assert_eq!(status(tester), AnimationStatus::Forward);
            tester.pump_and_settle();
            assert_eq!(status(tester), AnimationStatus::Completed);

            tester.tap(find::text("Suggestions"));
            assert_eq!(status(tester), AnimationStatus::Completed);
            tester.pump_and_settle();
            assert_eq!(status(tester), AnimationStatus::Completed);

            tester.tap(find::by_tooltip("Back"));
            assert_eq!(status(tester), AnimationStatus::Reverse);
            tester.pump_and_settle();
            assert_eq!(status(tester), AnimationStatus::Dismissed);
        });
    }

    #[test]
    fn custom_and_default_search_field_labels() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| {
                d.search_field_label("Enter search terms")
            });
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            open_search(tester);
            expect(
                tester,
                &find::text("Enter search terms"),
                finds_one_widget(),
            );
            expect(tester, &find::text("Search"), finds_nothing());
        });
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            open_search(tester);
            expect(tester, &find::text("Search"), finds_one_widget());
        });
    }

    #[test]
    fn custom_and_default_autocorrect_and_enable_suggestions() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            open_search(tester);
            tester.widget(find::by_type::<TextField>(), |field: &TextField| {
                assert_eq!(field.autocorrect, Some(true));
                assert!(field.enable_suggestions);
            });
        });
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| {
                d.autocorrect(false).enable_suggestions(false)
            });
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            open_search(tester);
            tester.widget(find::by_type::<TextField>(), |field: &TextField| {
                assert_eq!(field.autocorrect, Some(false));
                assert!(!field.enable_suggestions);
            });
        });
    }

    #[test]
    fn can_close_search_with_escape_button_and_return_null() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);
            open_search(tester);
            expect(tester, &find::text("Suggestions"), finds_one_widget());

            tester.send_key_event(PhysicalKeyboardKey::ESCAPE, LogicalKeyboardKey::ESCAPE);
            tester.pump_and_settle();
            assert_eq!(*results.borrow(), vec![None]);
            expect(tester, &find::text("HomeBody"), finds_one_widget());
            expect(tester, &find::text("Suggestions"), finds_nothing());

            open_search(tester);
            expect(tester, &find::text("Suggestions"), finds_one_widget());
        });
    }

    #[test]
    fn leading_width_size_is_16() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            delegate
                .search_delegate_data()
                .set_leading_width(Some(16.0));
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);

            // Open the search page with check leading width smaller than 16.
            open_search(tester);
            tester.tap_at(Offset::new(16.0, 16.0), 1);
            expect(tester, &find::text("Suggestions"), finds_one_widget());
            tester.widget(find::by_type::<AppBar>(), |app_bar: &AppBar| {
                assert_eq!(app_bar.leading_width, Some(16.0));
            });
            tester.tap_at(Offset::new(8.0, 16.0), 1);
            tester.pump_and_settle();
            expect(tester, &find::text("Suggestions"), finds_nothing());
            expect(tester, &find::text("HomeBody"), finds_one_widget());
        });
    }

    /// Dart's `_MyNavigatorObserver`: the routes pushed but the root and `nested` ones, and the
    /// `maintainState` of the last search route.
    #[derive(Default)]
    struct CountingObserver {
        navigator_observer: waft_widgets::NavigatorObserverData,
        push_count: Cell<u32>,
        maintain_state: Cell<bool>,
    }

    impl waft_widgets::NavigatorObserver for CountingObserver {
        waft_widgets::navigator_observer_accessors!();

        fn did_push(
            self: Handle<Self>,
            app: &mut App,
            route: AnyRoute,
            _previous: Option<AnyRoute>,
        ) {
            // Don't count the root route.
            if matches!(route.settings(app).name(), Some("nested" | "/")) {
                return;
            }
            if let Some(search_route) = route.downcast::<SearchPageRoute>(app) {
                let maintain_state = ModalRoute::maintain_state(search_route, app);
                app.get(self).maintain_state.set(maintain_state);
            }
            let count = &app.get(self).push_count;
            count.set(count.get() + 1);
        }
    }

    fn observer(tester: &WidgetTester) -> Retained<Handle<CountingObserver>> {
        tester.update(|app| app.create(CountingObserver::default()))
    }

    fn push_count(tester: &WidgetTester, observer: &Retained<Handle<CountingObserver>>) -> u32 {
        tester.cell.borrow().get(observer).push_count.get()
    }

    /// An action that runs `on_pressed` with the button's own context, as Dart's
    /// `Builder(builder: (context) => IconButton(tooltip:, icon:, onPressed:))`.
    fn action(
        tooltip: &'static str,
        icon: waft_widgets::IconData,
        on_pressed: impl Fn(&mut App, BuildContext) + 'static,
    ) -> WidgetRef {
        let on_pressed = Rc::new(on_pressed);
        waft_widgets::Builder::new(move |_app, context| {
            let on_pressed = Rc::clone(&on_pressed);
            IconButton::new(
                Some(Listener::new(move |app: &mut App| on_pressed(app, context))),
                waft_widgets::Icon::new(Some(icon.clone())),
            )
            .tooltip(tooltip)
            .into_widget()
        })
        .into_widget()
    }

    /// Dart's `showSearch(context:, delegate:)` whose result is added to `results`.
    fn show_search_into(
        app: &mut App,
        context: BuildContext,
        delegate: &Rc<TestSearchDelegate>,
        results: &Results,
    ) {
        let results = Rc::clone(results);
        show_search(
            app,
            context,
            Rc::clone(delegate) as Rc<dyn SearchDelegate>,
            None,
            false,
            false,
        )
        .then(app, move |_app, result| {
            results
                .borrow_mut()
                .push(result.and_then(|r| r.downcast_ref::<String>().cloned()));
        });
    }

    /// The engine's `popRoute` on `flutter/navigation`.
    fn pop_route(tester: &WidgetTester) {
        let mut app = tester.cell.borrow_mut();
        let view = app.platform().views().first().expect("a view").id();
        WidgetsBinding::instance(&mut app).handle_pop_route(&mut app, view);
        app.drain_microtasks();
    }

    /// Dart's `tester.showKeyboard(find.byType(TextField))`: what the text input was configured
    /// with.
    fn keyboard_input_action(tester: &mut WidgetTester) -> TextInputAction {
        tester.show_keyboard(find::by_type::<EditableText>());
        tester
            .view()
            .test_host()
            .text_input_calls()
            .into_iter()
            .rev()
            .find_map(|call| match call {
                waft_test::TextInputCall::SetClient(configuration) => {
                    Some(configuration.input_action)
                }
                _ => None,
            })
            .expect("the text input was given a client")
    }

    #[test]
    fn can_close_search_with_system_back_button_to_return_null() {
        // Regression test for https://github.com/flutter/flutter/issues/18145
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            let results = Rc::default();
            home_page(tester, &delegate, &results, None);

            // We are on the homepage.
            expect(tester, &find::text("HomeBody"), finds_one_widget());
            expect(tester, &find::text("HomeTitle"), finds_one_widget());
            expect(tester, &find::text("Suggestions"), finds_nothing());

            open_search(tester);
            expect(tester, &find::text("HomeBody"), finds_nothing());
            expect(tester, &find::text("HomeTitle"), finds_nothing());
            expect(tester, &find::text("Suggestions"), finds_one_widget());
            expect(tester, &find::text("Bottom"), finds_one_widget());

            // Simulate system back button.
            pop_route(tester);
            tester.pump_and_settle();
            assert_eq!(*results.borrow(), vec![None]);

            // We are on the homepage again.
            expect(tester, &find::text("HomeBody"), finds_one_widget());
            expect(tester, &find::text("HomeTitle"), finds_one_widget());
            expect(tester, &find::text("Suggestions"), finds_nothing());

            // Open search again.
            open_search(tester);
            expect(tester, &find::text("HomeBody"), finds_nothing());
            expect(tester, &find::text("HomeTitle"), finds_nothing());
            expect(tester, &find::text("Suggestions"), finds_one_widget());
        });
    }

    #[test]
    fn hint_text_color_overridden() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| {
                d.search_field_label("Enter search terms")
            });
            home_page(tester, &delegate, &Results::default(), None);
            open_search(tester);

            let color = tester.widget(find::text("Enter search terms"), |text: &Text| {
                text.style.as_ref().and_then(|style| style.color.clone())
            });
            assert_eq!(color.map(|color| color.color()), Some(hint_text_color()));
        });
    }

    #[test]
    fn initial_query_null_reuses_previous_query() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            delegate.set_query(&mut tester.cell.borrow_mut(), "Foo".to_owned());
            home_page_with(
                tester,
                Rc::clone(&delegate) as Rc<dyn SearchDelegate>,
                &Results::default(),
                HomeOptions {
                    pass_in_initial_query: true,
                    ..HomeOptions::default()
                },
            );
            open_search(tester);
            assert_eq!(delegate.query(&tester.cell.borrow()), "Foo");
        });
    }

    #[test]
    fn closing_nested_search_returns_to_search() {
        test_widgets(|tester| {
            let nested_results = Results::default();
            let nested = TestSearchDelegate::with_options(
                &mut tester.cell.borrow_mut(),
                |d| d,
                Options {
                    suggestions: "Nested Suggestions",
                    result: "Nested Result",
                    ..Options::default()
                },
            );
            let delegate = TestSearchDelegate::with_options(
                &mut tester.cell.borrow_mut(),
                |d| d,
                Options {
                    actions: vec![action("Nested Search", Icons::search(), {
                        let nested = Rc::clone(&nested);
                        let nested_results = Rc::clone(&nested_results);
                        move |app, context| show_search_into(app, context, &nested, &nested_results)
                    })],
                    ..Options::default()
                },
            );
            let results = Results::default();
            home_page(tester, &delegate, &results, None);
            expect(tester, &find::text("HomeBody"), finds_one_widget());
            open_search(tester);

            expect(tester, &find::text("HomeBody"), finds_nothing());
            expect(tester, &find::text("Suggestions"), finds_one_widget());
            expect(tester, &find::text("Nested Suggestions"), finds_nothing());

            tester.tap(find::by_tooltip("Nested Search"));
            tester.pump_and_settle();
            expect(tester, &find::text("HomeBody"), finds_nothing());
            expect(tester, &find::text("Suggestions"), finds_nothing());
            expect(
                tester,
                &find::text("Nested Suggestions"),
                finds_one_widget(),
            );

            tester.tap(find::by_tooltip("Back"));
            tester.pump_and_settle();
            assert_eq!(
                *nested_results.borrow(),
                vec![Some("Nested Result".to_owned())]
            );
            expect(tester, &find::text("HomeBody"), finds_nothing());
            expect(tester, &find::text("Suggestions"), finds_one_widget());
            expect(tester, &find::text("Nested Suggestions"), finds_nothing());

            tester.tap(find::by_tooltip("Back"));
            tester.pump_and_settle();
            expect(tester, &find::text("HomeBody"), finds_one_widget());
            expect(tester, &find::text("Suggestions"), finds_nothing());
            expect(tester, &find::text("Nested Suggestions"), finds_nothing());
            assert_eq!(*results.borrow(), vec![Some("Result".to_owned())]);
        });
    }

    #[test]
    fn closing_search_with_nested_search_shown_goes_back_to_the_underlying_route() {
        test_widgets(|tester| {
            let delegate: Rc<RefCell<Option<Rc<TestSearchDelegate>>>> = Rc::default();
            let nested_results = Results::default();
            let nested = TestSearchDelegate::with_options(
                &mut tester.cell.borrow_mut(),
                |d| d,
                Options {
                    suggestions: "Nested Suggestions",
                    result: "Nested Result",
                    actions: vec![action("Close Search", Icons::close(), {
                        let delegate = Rc::clone(&delegate);
                        move |app, context| {
                            let delegate = delegate.borrow().clone().expect("the delegate");
                            delegate.close(app, context, Some(Rc::new("Result Foo".to_owned())));
                        }
                    })],
                    ..Options::default()
                },
            );
            *delegate.borrow_mut() = Some(TestSearchDelegate::with_options(
                &mut tester.cell.borrow_mut(),
                |d| d,
                Options {
                    actions: vec![action("Nested Search", Icons::search(), {
                        let nested = Rc::clone(&nested);
                        let nested_results = Rc::clone(&nested_results);
                        move |app, context| show_search_into(app, context, &nested, &nested_results)
                    })],
                    ..Options::default()
                },
            ));
            let outer = delegate.borrow().clone().expect("the delegate");
            let results = Results::default();
            home_page(tester, &outer, &results, None);

            expect(tester, &find::text("HomeBody"), finds_one_widget());
            open_search(tester);
            expect(tester, &find::text("HomeBody"), finds_nothing());
            expect(tester, &find::text("Suggestions"), finds_one_widget());
            expect(tester, &find::text("Nested Suggestions"), finds_nothing());

            tester.tap(find::by_tooltip("Nested Search"));
            tester.pump_and_settle();
            expect(tester, &find::text("HomeBody"), finds_nothing());
            expect(tester, &find::text("Suggestions"), finds_nothing());
            expect(
                tester,
                &find::text("Nested Suggestions"),
                finds_one_widget(),
            );

            tester.tap(find::by_tooltip("Close Search"));
            tester.pump_and_settle();
            expect(tester, &find::text("HomeBody"), finds_one_widget());
            expect(tester, &find::text("Suggestions"), finds_nothing());
            expect(tester, &find::text("Nested Suggestions"), finds_nothing());
            assert_eq!(*nested_results.borrow(), vec![None]);
            assert_eq!(*results.borrow(), vec![Some("Result Foo".to_owned())]);
        });
    }

    #[test]
    fn custom_search_field_style_value() {
        test_widgets(|tester| {
            let style = TextStyle::new().color(Colors::RED.primary()).font_size(3.0);
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), {
                let style = style.clone();
                |d| {
                    d.search_field_label("Enter search terms")
                        .search_field_style(style)
                }
            });
            home_page(tester, &delegate, &Results::default(), None);
            open_search(tester);

            let hint = tester.widget(find::text("Enter search terms"), |text: &Text| {
                text.style.clone()
            });
            let field_style = tester.widget(find::by_type::<TextField>(), |field: &TextField| {
                field.style.clone().map(|style| style.value().clone())
            });
            for actual in [hint, field_style] {
                let actual = actual.expect("a style");
                assert_eq!(
                    actual.color.map(|color| color.color()),
                    Some(Colors::RED.primary())
                );
                assert_eq!(actual.font_size, Some(3.0));
            }
        });
    }

    #[test]
    fn keyboard_shows_the_search_button_by_default() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            home_page(tester, &delegate, &Results::default(), None);
            open_search(tester);
            assert_eq!(keyboard_input_action(tester), TextInputAction::Search);
        });
    }

    #[test]
    fn custom_text_input_action_results_in_keyboard_with_the_corresponding_button() {
        test_widgets(|tester| {
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| {
                d.text_input_action(TextInputAction::Done)
            });
            home_page(tester, &delegate, &Results::default(), None);
            open_search(tester);
            assert_eq!(keyboard_input_action(tester), TextInputAction::Done);
        });
    }

    #[test]
    fn custom_flexible_space_value() {
        test_widgets(|tester| {
            let flexible_space = Text::new("custom flexibleSpace").into_widget();
            let delegate = TestSearchDelegate::with_options(
                &mut tester.cell.borrow_mut(),
                |d| d,
                Options {
                    flexible_space: Some(flexible_space.clone()),
                    ..Options::default()
                },
            );
            home_page(tester, &delegate, &Results::default(), None);
            open_search(tester);
            expect(
                tester,
                &find::by_widget(&flexible_space),
                finds_one_widget(),
            );
        });
    }

    #[test]
    fn custom_search_field_decoration_theme_value() {
        test_widgets(|tester| {
            let decoration_theme = InputDecorationThemeData::new()
                .hint_style(TextStyle::new().color(hint_text_color()));
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), {
                let decoration_theme = decoration_theme.clone();
                |d| d.search_field_decoration_theme(decoration_theme)
            });
            home_page(tester, &delegate, &Results::default(), None);
            open_search(tester);

            let text_field = tester.element(find::by_type::<TextField>());
            let theme = Theme::of(&mut tester.cell.borrow_mut(), text_field);
            assert_eq!(theme.input_decoration_theme, decoration_theme);
        });
    }

    /// Dart's 'text in search bar contrasts background' cases, which differ in the theme only.
    fn text_in_search_bar_contrasts_background(
        tester: &mut WidgetTester,
        theme: ThemeData,
        background: Color,
    ) {
        let delegate = TestSearchDelegate::with_options(
            &mut tester.cell.borrow_mut(),
            |d| d,
            Options {
                default_app_bar_theme: true,
                ..Options::default()
            },
        );
        home_page_with(
            tester,
            Rc::clone(&delegate) as Rc<dyn SearchDelegate>,
            &Results::default(),
            HomeOptions {
                pass_in_initial_query: true,
                initial_query: Some("search query"),
                theme: Some(theme.clone()),
                ..HomeOptions::default()
            },
        );
        open_search(tester);

        let app_bar_background = find::descendant(
            find::by_type::<AppBar>(),
            find::by_type::<crate::material::Material>(),
        )
        .first();
        let color = tester.widget(
            app_bar_background,
            |material: &crate::material::Material| material.color,
        );
        assert_eq!(color, Some(background));

        let text_color = tester
            .widget(find::by_type::<TextField>(), |field: &TextField| {
                field.style.clone().map(|style| style.value().clone())
            })
            .expect("a style")
            .color
            .map(|color| color.color());
        let body_large_color = theme
            .text_theme
            .body_large
            .as_ref()
            .and_then(|style| style.color.clone())
            .map(|color| color.color());
        assert_eq!(text_color, body_large_color);
        assert_ne!(text_color, Some(background));
    }

    // Regression test for https://github.com/flutter/flutter/issues/66781
    #[test]
    fn text_in_search_bar_contrasts_background_light_mode() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            text_in_search_bar_contrasts_background(tester, theme, Colors::WHITE);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/66781
    #[test]
    fn text_in_search_bar_contrasts_background_dark_mode() {
        test_widgets(|tester| {
            let theme = ThemeData::dark(&tester.cell.borrow(), Some(false));
            let primary = theme.primary_color;
            text_in_search_bar_contrasts_background(tester, theme, primary);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/78144
    #[test]
    fn leading_actions_and_flexible_space_nullable_test() {
        test_widgets(|tester| {
            // The search delegate page is displayed with no issues even with `None` return
            // values for `build_leading`, `build_actions` and `build_flexible_space`.
            let delegate = EmptySearchDelegate::new(&mut tester.cell.borrow_mut());
            let results = Results::default();
            home_page_with(tester, delegate, &results, HomeOptions::default());

            expect(tester, &find::text("HomeBody"), finds_one_widget());
            expect(tester, &find::text("HomeTitle"), finds_one_widget());
            expect(tester, &find::text("Suggestions"), finds_nothing());

            open_search(tester);
            expect(tester, &find::text("HomeBody"), finds_nothing());
            expect(tester, &find::text("HomeTitle"), finds_nothing());
            expect(tester, &find::text("Suggestions"), finds_one_widget());
            assert!(results.borrow().is_empty());
            assert!(field_has_focus(tester));

            tester.tap(find::by_tooltip("Close"));
            tester.pump_and_settle();
            expect(tester, &find::text("HomeBody"), finds_one_widget());
            expect(tester, &find::text("HomeTitle"), finds_one_widget());
            expect(tester, &find::text("Suggestions"), finds_nothing());
            assert_eq!(*results.borrow(), vec![Some("Result".to_owned())]);
        });
    }

    #[test]
    fn show_search_with_use_root_navigator() {
        test_widgets(|tester| {
            let root_observer = observer(tester);
            let local_observer = observer(tester);
            let delegate = EmptySearchDelegate::new(&mut tester.cell.borrow_mut());

            let button_delegate = Rc::clone(&delegate);
            let search_button = move |label: &'static str, use_root_navigator: bool| {
                let delegate = Rc::clone(&button_delegate);
                waft_widgets::Builder::new(move |_app, context| {
                    let delegate = Rc::clone(&delegate);
                    crate::text_button::TextButton::new(
                        Some(Listener::new(move |app: &mut App| {
                            drop(show_search(
                                app,
                                context,
                                Rc::clone(&delegate) as Rc<dyn SearchDelegate>,
                                None,
                                use_root_navigator,
                                false,
                            ));
                        })),
                        Some(Text::new(label)),
                    )
                    .into_widget()
                })
                .into_widget()
            };
            let theme = m3(&tester.cell.borrow());
            let home = {
                let local = local_observer.erase::<waft_widgets::AnyNavigatorObserver>();
                waft_widgets::Navigator::new()
                    .observers([local])
                    .initial_route("nested")
                    .on_generate_route(move |app, settings| {
                        let name = settings.name.clone();
                        if name.as_deref() != Some("nested") {
                            panic!("only the nested route is generated");
                        }
                        let showers = (
                            search_button("showSearchRootNavigator", true),
                            search_button("showSearchLocalNavigator", false),
                        );
                        let route = crate::page::MaterialPageRoute::new(
                            app,
                            Rc::new(move |_app, _context| {
                                waft_widgets::Column::new()
                                    .main_axis_alignment(waft_rendering::MainAxisAlignment::Center)
                                    .children(vec![showers.0.clone(), showers.1.clone()])
                                    .into_widget()
                            }),
                        )
                        .settings(app, RouteSettingsRef::Settings(settings.clone()));
                        Some(route.erase())
                    })
            };
            tester.pump_widget(
                crate::app::MaterialApp::new()
                    .theme(theme)
                    .navigator_observers([
                        root_observer.erase::<waft_widgets::AnyNavigatorObserver>()
                    ])
                    .home(home)
                    .into_widget(),
            );

            assert_eq!(push_count(tester, &root_observer), 0);
            assert_eq!(push_count(tester, &local_observer), 0);

            // showSearch normal and back.
            tester.tap(find::text("showSearchLocalNavigator"));
            tester.pump_and_settle();
            expect(
                tester,
                &find::by_type::<crate::action_buttons::BackButton>(),
                finds_widgets(),
            );
            tester.tap(find::by_tooltip("Close"));
            tester.pump_and_settle();
            assert_eq!(push_count(tester, &root_observer), 0);
            assert_eq!(push_count(tester, &local_observer), 1);

            // showSearch with rootNavigator.
            tester.tap(find::text("showSearchRootNavigator"));
            tester.pump_and_settle();
            tester.tap(find::by_tooltip("Close"));
            tester.pump_and_settle();

            // showSearch without back button.
            delegate
                .search_delegate_data()
                .set_automatically_imply_leading(Some(false));
            tester.tap(find::text("showSearchRootNavigator"));
            tester.pump_and_settle();
            tester.widget(find::by_type::<AppBar>(), |app_bar: &AppBar| {
                assert!(!app_bar.automatically_imply_leading);
            });
            expect(tester, &find::by_tooltip("Back"), finds_nothing());
            tester.tap(find::by_tooltip("Close"));
            tester.pump_and_settle();
            assert_eq!(push_count(tester, &root_observer), 2);
            assert_eq!(push_count(tester, &local_observer), 1);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/95588
    #[test]
    fn query_text_field_shows_toolbar_initially() {
        test_widgets(|_| {
            let clipboard = MockClipboard::new();
            let mut tester = WidgetTester::with_platform(
                TestPlatform::new().with_clipboard(Rc::new(clipboard.clone())),
            );
            let tester = &mut tester;
            let delegate = TestSearchDelegate::new(&mut tester.cell.borrow_mut(), |d| d);
            home_page(tester, &delegate, &Results::default(), None);
            open_search(tester);

            assert_eq!(delegate.query(&tester.cell.borrow()).len(), 0);
            clipboard.set_clipboard_data(Some("pasteablestring".to_owned()));

            // Long press shows toolbar.
            tester.long_press(find::by_type::<TextField>());
            tester.pump();
            expect(tester, &find::text("Paste"), finds_one_widget());

            tester.tap(find::text("Paste"));
            tester.pump();
            assert_eq!(delegate.query(&tester.cell.borrow()).len(), 15);
        });
    }

    #[test]
    fn show_search_with_maintain_state_on_the_route() {
        test_widgets(|tester| {
            let navigation_observer = observer(tester);
            let delegate = EmptySearchDelegate::new(&mut tester.cell.borrow_mut());
            let search_button = |label: &'static str, maintain_state: bool| {
                let delegate = Rc::clone(&delegate);
                waft_widgets::Builder::new(move |_app, context| {
                    let delegate = Rc::clone(&delegate);
                    crate::text_button::TextButton::new(
                        Some(Listener::new(move |app: &mut App| {
                            drop(show_search(
                                app,
                                context,
                                Rc::clone(&delegate) as Rc<dyn SearchDelegate>,
                                None,
                                false,
                                maintain_state,
                            ));
                        })),
                        Some(Text::new(label)),
                    )
                    .into_widget()
                })
                .into_widget()
            };
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(
                crate::app::MaterialApp::new()
                    .theme(theme)
                    .navigator_observers([
                        navigation_observer.erase::<waft_widgets::AnyNavigatorObserver>()
                    ])
                    .home(
                        waft_widgets::Column::new()
                            .main_axis_alignment(waft_rendering::MainAxisAlignment::Center)
                            .children(vec![
                                search_button("showSearch", false),
                                search_button("showSearchWithMaintainState", true),
                            ]),
                    )
                    .into_widget(),
            );

            let maintain_state = |tester: &WidgetTester| {
                tester
                    .cell
                    .borrow()
                    .get(&navigation_observer)
                    .maintain_state
                    .get()
            };
            assert_eq!(push_count(tester, &navigation_observer), 0);
            assert!(!maintain_state(tester));

            // showSearch normal and back.
            tester.tap(find::text("showSearch"));
            tester.pump_and_settle();
            expect(
                tester,
                &find::by_type::<crate::action_buttons::BackButton>(),
                finds_widgets(),
            );
            tester.tap(find::by_tooltip("Close"));
            tester.pump_and_settle();
            assert_eq!(push_count(tester, &navigation_observer), 1);
            assert!(!maintain_state(tester));

            // showSearch with maintainState.
            tester.tap(find::text("showSearchWithMaintainState"));
            tester.pump_and_settle();
            expect(
                tester,
                &find::by_type::<crate::action_buttons::BackButton>(),
                finds_widgets(),
            );
            tester.tap(find::by_tooltip("Close"));
            tester.pump_and_settle();
            assert_eq!(push_count(tester, &navigation_observer), 2);
            assert!(maintain_state(tester));
        });
    }
}
