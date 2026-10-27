//! NavigationView's adaptive pane, hierarchical selection and top projection from NavigationView.cpp.

use super::navigation_indicator_transition::NAVIGATION_INDICATOR_DURATION;
use super::navigation_view_chrome::{
    PaneChrome, back_button_template, pane_title, pane_toggle_template,
};
use super::navigation_view_item::{NavigationViewItemPosition, NavigationViewItemPresenter};
use super::top_navigation_view_data_provider::TopNavigationViewDataProvider;
use crate::*;
use inset_animation::{AnimationBehavior, AnimationController, Curves};
use inset_embedder::{Color, FontWeight, Offset, Rect, TextDirection};
use inset_foundation::{App, Handle, Listener};
use inset_painting::{Alignment, EdgeInsetsGeometry};
use inset_rendering::{BoxConstraints, CrossAxisAlignment, MainAxisSize, StackFit};
use inset_scheduler::{
    FrameCallback, SchedulerBinding, Ticker, TickerCallback, TickerProviderObject,
};
use inset_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    fmt,
    rc::Rc,
    time::Duration,
};

/// The space available to the left navigation pane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationViewDisplayMode {
    /// Only the toggle button is visible while the pane is closed.
    #[default]
    Minimal,
    /// A compact icon rail remains visible while the pane is closed.
    Compact,
    /// The pane occupies space beside the content.
    Expanded,
}

/// Selects adaptive left navigation or an explicit pane configuration.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationViewPaneDisplayMode {
    /// Uses the compact and expanded width thresholds.
    #[default]
    Auto,
    /// Keeps the expanded left layout at every width.
    Left,
    /// Places primary items in a horizontal strip with an overflow menu.
    Top,
    /// Keeps a compact left rail at every width.
    LeftCompact,
    /// Keeps a light-dismiss overlay pane at every width.
    LeftMinimal,
}

/// Controls whether the navigation back button occupies its template slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationViewBackButtonVisible {
    /// Hides the back button and its placeholder.
    Collapsed,
    /// Shows the back button.
    Visible,
    /// Shows the desktop back button, matching the source's non-console branch.
    #[default]
    Auto,
}

/// Whether moving keyboard focus also invokes selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationViewSelectionFollowsFocus {
    /// Focus moves independently of selection.
    #[default]
    Disabled,
    /// Keyboard focus changes select eligible items.
    Enabled,
}

/// The source's suggested page-transition direction; the owner performs navigation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationRecommendedTransitionDirection {
    /// The default transition used by left navigation and settings.
    #[default]
    Default,
    /// A top selection moved toward an item to the left.
    FromLeft,
    /// A top selection moved toward an item to the right.
    FromRight,
    /// A top selection came from the overflow menu.
    FromOverflow,
}

/// Identifies a menu item independently of its current visual projection.
#[derive(Clone, Debug)]
pub struct NavigationViewItemEventArgs {
    /// The owner's item snapshot at the time of the event.
    pub item: NavigationViewItem,
    /// Main/footer block, root index and any child indices.
    pub index_path: Vec<usize>,
    /// Whether the built-in settings item was invoked or selected.
    pub is_settings: bool,
    /// The source's suggestion for the owner's page transition.
    pub recommended_transition_direction: NavigationRecommendedTransitionDirection,
}

/// Describes selection, including clearing selection when the selected item is removed.
#[derive(Clone, Debug)]
pub struct NavigationViewSelectionChangedEventArgs {
    /// Selected source item, or None when the selection is cleared.
    pub item: Option<NavigationViewItem>,
    /// Main/footer block and child indices, empty when selection is cleared.
    pub index_path: Vec<usize>,
    /// Whether the built-in settings item is selected.
    pub is_settings: bool,
    /// The suggested transition for the owner's page navigation.
    pub recommended_transition_direction: NavigationRecommendedTransitionDirection,
}

impl From<NavigationViewItemEventArgs> for NavigationViewSelectionChangedEventArgs {
    fn from(args: NavigationViewItemEventArgs) -> Self {
        Self {
            item: Some(args.item),
            index_path: args.index_path,
            is_settings: args.is_settings,
            recommended_transition_direction: args.recommended_transition_direction,
        }
    }
}

/// Receives both selection and deselection requests.
pub type NavigationViewSelectionChanged =
    Rc<dyn Fn(&mut App, NavigationViewSelectionChangedEventArgs)>;

/// Requests owner-controlled pane openness.
pub type NavigationViewPaneOpenChanged = Rc<dyn Fn(&mut App, bool)>;

/// Reports the effective adaptive display mode.
pub type NavigationViewDisplayModeChanged = Rc<dyn Fn(&mut App, NavigationViewDisplayMode)>;

/// Receives selection, invocation, expansion or collapse with a stable item identity.
pub type NavigationViewItemEvent = Rc<dyn Fn(&mut App, NavigationViewItemEventArgs)>;

/// A navigation shell; the owner supplies content and handles navigation events.
#[derive(Clone)]
pub struct NavigationView {
    /// Identity retained across owner rebuilds.
    pub key: Option<KeyRef>,
    /// Main menu items and their nested children.
    pub menu_items: Vec<NavigationViewItem>,
    /// Footer menu items, before the built-in settings item.
    pub footer_menu_items: Vec<NavigationViewItem>,
    /// Stable identity selected by the owner.
    pub selected_item: Option<String>,
    /// Requests selection after ItemInvoked has been raised.
    pub selection_changed: NavigationViewSelectionChanged,
    /// Receives every invocation, including the already selected item.
    pub item_invoked: Option<NavigationViewItemEvent>,
    /// The page content; NavigationView does not instantiate pages itself.
    pub content: WidgetRef,
    /// Chooses the adaptive or explicit layout.
    pub pane_display_mode: NavigationViewPaneDisplayMode,
    /// Below this positive width Auto uses Minimal.
    pub compact_mode_threshold_width: f64,
    /// At and above this width Auto uses Expanded.
    pub expanded_mode_threshold_width: f64,
    /// The width of the closed icon rail.
    pub compact_pane_length: f64,
    /// The requested open pane width, clamped to the available control width.
    pub open_pane_length: f64,
    /// Optional owner override; None lets the adaptive pane maintain its openness.
    pub is_pane_open: Option<bool>,
    /// Requests an owner update when a controlled pane opens or closes.
    pub pane_open_changed: Option<NavigationViewPaneOpenChanged>,
    /// Whether either pane is visible.
    pub is_pane_visible: bool,
    /// Whether the menu toggle is visible in left layouts.
    pub is_pane_toggle_button_visible: bool,
    /// Whether the settings entry is appended to the footer.
    pub is_settings_visible: bool,
    /// Whether the back button is visible.
    pub is_back_button_visible: NavigationViewBackButtonVisible,
    /// Whether the visible back button accepts input.
    pub is_back_enabled: bool,
    /// Receives back-button activation without changing the selected page.
    pub back_requested: Option<Listener>,
    /// Whether keyboard focus selects menu items.
    pub selection_follows_focus: NavigationViewSelectionFollowsFocus,
    /// Optional page header above content.
    pub header: Option<WidgetRef>,
    /// Keeps the page header visible outside Minimal mode.
    pub always_show_header: bool,
    /// Text next to the pane toggle, or preceding top items.
    pub pane_title: String,
    /// Custom header in the pane.
    pub pane_header: Option<WidgetRef>,
    /// Custom content between pane header and menu items.
    pub pane_custom_content: Option<WidgetRef>,
    /// Native search content hosted in the source AutoSuggestBox presenter slot.
    pub auto_suggest_box: Option<WidgetRef>,
    /// Custom content above footer menu items.
    pub pane_footer: Option<WidgetRef>,
    /// Content placed below the top pane, before page content.
    pub content_overlay: Option<WidgetRef>,
    /// Receives adaptive display-mode changes.
    pub display_mode_changed: Option<NavigationViewDisplayModeChanged>,
    /// Fires before expanded children become visible.
    pub expanding: Option<NavigationViewItemEvent>,
    /// Fires after collapsed children are hidden.
    pub collapsed: Option<NavigationViewItemEvent>,
    /// Fires when the pane starts opening.
    pub pane_opening: Option<Listener>,
    /// Fires when the pane finishes opening.
    pub pane_opened: Option<Listener>,
    /// Can cancel a pane closing request.
    pub pane_closing: Option<SplitViewPaneClosingHandler>,
    /// Fires when the pane finishes closing.
    pub pane_closed: Option<Listener>,
}

impl fmt::Debug for NavigationView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NavigationView")
            .field("selected_item", &self.selected_item)
            .field("pane_display_mode", &self.pane_display_mode)
            .finish_non_exhaustive()
    }
}

impl NavigationView {
    /// Stable identity of the built-in settings item; owner item ids must not use it.
    pub const SETTINGS_ITEM_ID: &'static str = "__navigation_settings";

    /// Creates an adaptive navigation shell with owner-managed selection and page content.
    pub fn new<C>(
        menu_items: Vec<NavigationViewItem>,
        selected_item: Option<String>,
        selection_changed: impl Fn(&mut App, NavigationViewSelectionChangedEventArgs) + 'static,
        content: impl IntoWidget<C>,
    ) -> Self {
        Self {
            key: None,
            menu_items,
            footer_menu_items: Vec::new(),
            selected_item,
            selection_changed: Rc::new(selection_changed),
            item_invoked: None,
            content: content.into_widget(),
            pane_display_mode: NavigationViewPaneDisplayMode::Auto,
            compact_mode_threshold_width: 641.0,
            expanded_mode_threshold_width: 1008.0,
            compact_pane_length: 48.0,
            open_pane_length: 320.0,
            is_pane_open: None,
            pane_open_changed: None,
            is_pane_visible: true,
            is_pane_toggle_button_visible: true,
            is_settings_visible: true,
            is_back_button_visible: NavigationViewBackButtonVisible::Auto,
            is_back_enabled: false,
            back_requested: None,
            selection_follows_focus: NavigationViewSelectionFollowsFocus::Disabled,
            header: None,
            always_show_header: true,
            pane_title: String::new(),
            pane_header: None,
            pane_custom_content: None,
            auto_suggest_box: None,
            pane_footer: None,
            content_overlay: None,
            display_mode_changed: None,
            expanding: None,
            collapsed: None,
            pane_opening: None,
            pane_opened: None,
            pane_closing: None,
            pane_closed: None,
        }
    }
}

/// Retains one logical container through pane/overflow reparenting.
#[derive(Clone)]
struct NavigationContainer {
    /// Global identity preserving state in caller-supplied item content.
    key: Rc<GlobalKey>,
    /// Focus shared with the item's native input presenter.
    focus: AnyFocusNode,
    /// Exact source indicator geometry, including while its opacity is zero.
    indicator_key: Rc<GlobalKey>,
    /// Source compositor CenterPoint.X survives completion and is retained per owner.
    indicator_pivot_x: Rc<Cell<f64>>,
    /// An existing CenterPoint animation keeps its final step even if Scale is replaced.
    indicator_pivot_step: Cell<Option<(Duration, f64)>>,
}

/// Source state that cannot be derived from the current width alone.
pub struct NavigationViewState {
    /// Native widget lifecycle data.
    state: StateData<NavigationView>,
    /// Stable item containers, including currently collapsed children.
    containers: HashMap<String, NavigationContainer>,
    /// Stable native identities for owner content reparented by pane-mode changes.
    slot_keys: HashMap<&'static str, Rc<GlobalKey>>,
    /// Expanded ids independent of where their children are presented.
    expanded: HashSet<String>,
    /// Expanded left items restored when a compact pane reopens.
    restore_expanded: HashSet<String>,
    /// Current adaptive display mode.
    display_mode: NavigationViewDisplayMode,
    /// Native pane openness when not controlled by the owner.
    pane_open: bool,
    /// Prevents a second PaneClosing event after a NavigationView light-close request.
    block_next_closing: bool,
    /// Explicit closure prevents automatic reopening on later Expanded layouts.
    was_force_closed: bool,
    /// Whether the first non-forced width update has occurred.
    initial_mode_update: bool,
    /// Last width used for adaptive updates.
    width: f64,
    /// Last pane mode applied by the layout callback.
    pane_mode: NavigationViewPaneDisplayMode,
    /// Independent native scrolling for main and footer groups.
    scrolls: Vec<Handle<ScrollViewportController>>,
    /// Native focus scope retained when the search presenter moves between pane modes.
    search_focus: Option<Handle<FocusScopeNode>>,
    /// Natural heights of the main, footer and pane-footer content.
    group_heights: [f64; 3],
    /// Top-level source order and attached natural width cache.
    top: TopNavigationViewDataProvider,
    /// Forces a full primary measure before partitioning changed content.
    top_measure_pending: bool,
    /// Width consumed by top header, custom content and footer slots.
    top_chrome_width: f64,
    /// Natural leading and trailing top slots.
    top_chrome_sizes: [f64; 3],
    /// Whether the top overflow flyout is open.
    overflow_open: bool,
    /// Selected identity used to detect owner changes and transitions.
    selected: Option<String>,
    /// Actual outgoing/incoming owners and initial geometry for source relative transforms.
    indicator_transition: Option<(NavigationIndicatorOwner, NavigationIndicatorOwner, bool)>,
    /// Keeps the outgoing owner visible while the destination receives its first layout.
    pending_indicator: Option<NavigationIndicatorOwner>,
    /// Shared native clock for both actual item visuals.
    indicator_controller: Option<Handle<AnimationController>>,
    /// Native ticker ownership for the selection storyboard.
    indicator_ticker: SingleTickerProviderStateMixinData,
    /// Invalidates geometry/completion callbacks when selection changes again.
    indicator_epoch: u64,
}

impl StatefulWidget for NavigationView {
    type State = NavigationViewState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> Self::State {
        NavigationViewState {
            state: StateData::new(),
            containers: HashMap::new(),
            slot_keys: [
                "content",
                "header",
                "pane_header",
                "custom",
                "footer",
                "search",
            ]
            .into_iter()
            .map(|slot| {
                (
                    slot,
                    Rc::new(GlobalKey::labeled(format!("Navigation {slot}"))),
                )
            })
            .collect(),
            expanded: HashSet::new(),
            restore_expanded: HashSet::new(),
            display_mode: NavigationViewDisplayMode::Minimal,
            pane_open: true,
            block_next_closing: false,
            was_force_closed: false,
            initial_mode_update: true,
            width: f64::NAN,
            pane_mode: self.pane_display_mode,
            scrolls: Vec::new(),
            search_focus: None,
            group_heights: [0.0; 3],
            top: TopNavigationViewDataProvider::default(),
            top_measure_pending: true,
            top_chrome_width: 56.0,
            top_chrome_sizes: [0.0; 3],
            overflow_open: false,
            selected: self.selected_item.clone(),
            indicator_transition: None,
            pending_indicator: None,
            indicator_controller: None,
            indicator_ticker: SingleTickerProviderStateMixinData::default(),
            indicator_epoch: 0,
        }
    }
}

/// Actual indicator identity and its settled geometry at animation setup.
#[derive(Clone, Debug)]
struct NavigationIndicatorOwner {
    /// Stable menu identity whose actual SelectionIndicator receives the transform.
    id: String,
    /// Settled bounds at setup; parent motion remains inherited by the real visual.
    rect: Rect,
}

impl SingleTickerProviderStateMixin for NavigationViewState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).indicator_ticker
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).indicator_ticker
    }
}

impl TickerProviderObject for NavigationViewState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

/// Flattens source identities and paths without depending on visual parentage.
fn collect_items(
    items: &[NavigationViewItem],
    parent: &[usize],
    output: &mut Vec<(NavigationViewItem, Vec<usize>)>,
) {
    for (index, item) in items.iter().enumerate() {
        let mut path = parent.to_vec();
        path.push(index);
        output.push((item.clone(), path.clone()));
        collect_items(&item.menu_items, &path, output);
    }
}

impl NavigationViewState {
    /// Returns the current adaptive display mode.
    pub fn display_mode(self: Handle<Self>, app: &App) -> NavigationViewDisplayMode {
        app.get(self).display_mode
    }

    /// Returns the current pane openness, including an owner override.
    pub fn is_pane_open(self: Handle<Self>, app: &App) -> bool {
        self.widget(app)
            .is_pane_open
            .unwrap_or(app.get(self).pane_open)
    }

    /// Returns a mounted container by stable identity, including flyout children.
    pub fn container_from_menu_item(
        self: Handle<Self>,
        app: &mut App,
        id: &str,
    ) -> Option<BuildContext> {
        let key = app.get(self).containers.get(id)?.key.clone();
        key.current_context(app)
    }

    /// Returns an item's current index path independent of the current projection.
    pub fn index_path_from_item(self: Handle<Self>, app: &App, id: &str) -> Option<Vec<usize>> {
        self.all_items(app)
            .into_iter()
            .find(|(item, _)| item.id == id)
            .map(|(_, path)| path)
    }

    /// Retains an owner-supplied content subtree when the source template reparents its slot.
    fn slot(self: Handle<Self>, app: &App, name: &'static str, child: WidgetRef) -> WidgetRef {
        KeyedSubtree::new(child)
            .key(app.get(self).slot_keys[name].clone())
            .into_widget()
    }

    /// The built-in settings item follows user footer entries.
    fn footer_items(self: Handle<Self>, app: &App) -> Vec<NavigationViewItem> {
        let widget = self.widget(app);
        let mut items = widget.footer_menu_items.clone();
        if widget.is_settings_visible {
            let mut settings =
                NavigationViewItem::text(NavigationView::SETTINGS_ITEM_ID, "Settings")
                    .icon(FontIcon::symbol(FluentSymbol::Settings));
            if widget.pane_display_mode == NavigationViewPaneDisplayMode::Top {
                settings = settings
                    .without_content()
                    .tool_tip(ToolTip::text("Settings"));
            }
            items.push(settings);
        }
        items
    }

    /// Gets the main and footer source blocks in SelectionModel path order.
    fn all_items(self: Handle<Self>, app: &App) -> Vec<(NavigationViewItem, Vec<usize>)> {
        let mut result = Vec::new();
        collect_items(&self.widget(app).menu_items, &[0], &mut result);
        collect_items(&self.footer_items(app), &[1], &mut result);
        result
    }

    /// Reconciles container ownership and releases removed nodes after native detach.
    fn sync_items(self: Handle<Self>, app: &mut App) {
        let items = self.all_items(app);
        let ids: HashSet<_> = items.iter().map(|(item, _)| item.id.clone()).collect();
        assert_eq!(
            ids.len(),
            items.len(),
            "NavigationView item ids must be unique, including nested items"
        );
        let removed: Vec<_> = app
            .get(self)
            .containers
            .keys()
            .filter(|id| !ids.contains(*id))
            .cloned()
            .collect();
        for id in removed {
            let entry = app.get_mut(self).containers.remove(&id).unwrap();
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    entry.focus.dispose(app);
                    app.destroy(entry.focus.id());
                }),
                View::maybe_id_of(app, self.context(app)),
            );
        }
        for id in ids {
            if !app.get(self).containers.contains_key(&id) {
                let entry = NavigationContainer {
                    key: Rc::new(GlobalKey::labeled(id.clone())),
                    focus: FocusNode::new(app).as_node(),
                    indicator_key: Rc::new(GlobalKey::labeled(format!("Indicator {id}"))),
                    indicator_pivot_x: Rc::new(Cell::new(0.0)),
                    indicator_pivot_step: Cell::new(None),
                };
                app.get_mut(self).containers.insert(id, entry);
            }
        }
        let ids = self
            .widget(app)
            .menu_items
            .iter()
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        app.get_mut(self).top.set_data_source(ids);
        let hidden: Vec<_> = self
            .widget(app)
            .menu_items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (!item.is_visible).then_some(index))
            .collect();
        for index in hidden {
            app.get_mut(self).top.set_width_for_item(index, 0.0);
        }
    }

    /// Raises a callback after the current build/layout phase.
    fn after_frame(self: Handle<Self>, app: &mut App, callback: impl Fn(&mut App) + 'static) {
        after_next_frame(
            app,
            View::maybe_id_of(app, self.context(app)),
            FrameCallback::new(move |app, _| {
                if app.contains(self) && self.mounted(app) {
                    callback(app);
                }
            }),
        );
    }

    /// OpenPane/ClosePane requests, with explicit closure remembered for adaptive resizing.
    fn set_pane_open(self: Handle<Self>, app: &mut App, open: bool, explicit: bool) {
        if explicit {
            app.get_mut(self).was_force_closed = !open;
        }
        if self.is_pane_open(app) == open {
            return;
        }
        self.set_state(app, |state| {
            state.pane_open = open;
            if open {
                state.was_force_closed = false;
            }
        });
        self.apply_pane_expansion_memory(app, open);
        if let Some(callback) = self.widget(app).pane_open_changed.clone() {
            callback(app, open);
        }
    }

    /// Source HandleExpansionStateMemory changes only top-level owners, preserving nested expansion.
    fn apply_pane_expansion_memory(self: Handle<Self>, app: &mut App, open: bool) {
        let roots: Vec<_> = self
            .all_items(app)
            .into_iter()
            .filter(|(_, path)| path.len() == 2)
            .map(|(item, _)| item.id)
            .collect();
        for id in roots {
            if open {
                if app.get_mut(self).restore_expanded.remove(&id)
                    && !app.get(self).expanded.contains(&id)
                {
                    self.toggle_expanded(app, &id);
                }
            } else if app.get(self).expanded.contains(&id) {
                app.get_mut(self).restore_expanded.insert(id.clone());
                self.toggle_expanded(app, &id);
            }
        }
    }

    /// UpdateAdaptiveLayout retains source initial-update and force-closed flags.
    fn update_adaptive_layout(self: Handle<Self>, app: &mut App, width: f64) {
        let widget = self.widget(app).clone();
        if app.get(self).width == width && app.get(self).pane_mode == widget.pane_display_mode {
            return;
        }
        let old_mode = app.get(self).display_mode;
        let old_pane_mode = app.get(self).pane_mode;
        let mode = match widget.pane_display_mode {
            NavigationViewPaneDisplayMode::Top | NavigationViewPaneDisplayMode::LeftMinimal => {
                NavigationViewDisplayMode::Minimal
            }
            NavigationViewPaneDisplayMode::Left => NavigationViewDisplayMode::Expanded,
            NavigationViewPaneDisplayMode::LeftCompact => NavigationViewDisplayMode::Compact,
            NavigationViewPaneDisplayMode::Auto
                if width >= widget.expanded_mode_threshold_width =>
            {
                NavigationViewDisplayMode::Expanded
            }
            NavigationViewPaneDisplayMode::Auto
                if width > 0.0 && width < widget.compact_mode_threshold_width =>
            {
                NavigationViewDisplayMode::Minimal
            }
            _ => NavigationViewDisplayMode::Compact,
        };
        let initial = app.get(self).initial_mode_update;
        {
            let state = app.get_mut(self);
            state.width = width;
            state.pane_mode = widget.pane_display_mode;
            state.display_mode = mode;
            state.initial_mode_update = false;
        }
        let close = widget.pane_display_mode == NavigationViewPaneDisplayMode::Top
            || mode == NavigationViewDisplayMode::Minimal
            || (initial && mode == NavigationViewDisplayMode::Compact)
            || (old_mode == NavigationViewDisplayMode::Expanded
                && mode == NavigationViewDisplayMode::Compact);
        let open = mode == NavigationViewDisplayMode::Expanded
            && widget.is_pane_visible
            && (!app.get(self).was_force_closed
                || old_pane_mode == NavigationViewPaneDisplayMode::LeftMinimal);
        if close || open {
            self.after_frame(app, move |app| self.set_pane_open(app, !close, false));
        }
        if old_mode != mode
            && let Some(callback) = widget.display_mode_changed
        {
            self.after_frame(app, move |app| callback(app, mode));
        }
    }

    /// Builds event arguments from stable source paths and the current top projection.
    fn event_args(
        self: Handle<Self>,
        app: &App,
        id: &str,
        overflow: bool,
    ) -> Option<NavigationViewItemEventArgs> {
        let (item, index_path) = self
            .all_items(app)
            .into_iter()
            .find(|(item, _)| item.id == id)?;
        let settings = id == NavigationView::SETTINGS_ITEM_ID;
        let direction = if settings
            || self.widget(app).pane_display_mode != NavigationViewPaneDisplayMode::Top
        {
            NavigationRecommendedTransitionDirection::Default
        } else if overflow {
            NavigationRecommendedTransitionDirection::FromOverflow
        } else {
            let old = app
                .get(self)
                .selected
                .as_ref()
                .and_then(|id| self.index_path_from_item(app, id));
            match old {
                Some(old) if old < index_path => {
                    NavigationRecommendedTransitionDirection::FromRight
                }
                Some(old) if old > index_path => NavigationRecommendedTransitionDirection::FromLeft,
                _ => NavigationRecommendedTransitionDirection::Default,
            }
        };
        Some(NavigationViewItemEventArgs {
            item,
            index_path,
            is_settings: settings,
            recommended_transition_direction: direction,
        })
    }

    /// Raises invocation before selection, then toggles children and dismisses leaf navigation.
    fn invoke(self: Handle<Self>, app: &mut App, id: &str, overflow: bool) {
        let Some(args) = self.event_args(app, id, overflow) else {
            return;
        };
        if !args.item.is_enabled
            || !args.item.is_visible
            || args.item.kind != NavigationViewItemKind::Item
        {
            return;
        }
        let before = app.get(self).selected.clone();
        let old_indicator = before.as_ref().and_then(|id| self.indicator_rect(app, id));
        if let Some(callback) = self.widget(app).item_invoked.clone() {
            callback(app, args.clone());
        }
        if args.item.selects_on_invoked
            && before == app.get(self).selected
            && before.as_deref() != Some(id)
        {
            let callback = self.widget(app).selection_changed.clone();
            self.set_state(app, |state| state.selected = Some(id.to_owned()));
            if overflow && args.index_path[0] == 0 {
                let width = (app.get(self).width - app.get(self).top_chrome_width).max(0.0);
                app.get_mut(self)
                    .top
                    .select_overflow_item(args.index_path[1], width);
            }
            callback(app, args.clone().into());
            self.animate_selection(app, old_indicator);
        }
        if !args.item.menu_items.is_empty() || args.item.has_unrealized_children {
            self.toggle_expanded(app, id);
        } else {
            self.set_state(app, |state| {
                state.overflow_open = false;
            });
            if args.item.selects_on_invoked
                && args.index_path.len() > 2
                && (self.widget(app).pane_display_mode == NavigationViewPaneDisplayMode::Top
                    || !self.is_pane_open(app))
            {
                let root = self
                    .all_items(app)
                    .into_iter()
                    .find(|(_, path)| path.as_slice() == &args.index_path[..2]);
                if let Some((root, _)) = root
                    && app.get(self).expanded.contains(&root.id)
                {
                    self.toggle_expanded(app, &root.id);
                }
            }
            if self.is_pane_open(app)
                && app.get(self).display_mode != NavigationViewDisplayMode::Expanded
            {
                self.set_pane_open(app, false, false);
            }
        }
    }

    /// Expanding precedes children; Collapsed follows the rebuild that hides them.
    fn toggle_expanded(self: Handle<Self>, app: &mut App, id: &str) {
        let Some(args) = self.event_args(app, id, false) else {
            return;
        };
        let expanded = app.get(self).expanded.contains(id);
        let selected = app.get(self).selected.clone();
        let previous_indicator = selected.as_ref().and_then(|selected| {
            let path = self.index_path_from_item(app, selected)?;
            (selected != id && path.starts_with(&args.index_path))
                .then(|| self.indicator_rect(app, selected))
                .flatten()
        });
        if !expanded && let Some(callback) = self.widget(app).expanding.clone() {
            callback(app, args.clone());
        }
        self.set_state(app, |state| {
            if expanded {
                state.expanded.remove(id);
            } else {
                state.expanded.insert(id.to_owned());
            }
        });
        if previous_indicator.is_some() {
            self.animate_selection(app, previous_indicator);
        }
        if expanded && let Some(callback) = self.widget(app).collapsed.clone() {
            self.after_frame(app, move |app| callback(app, args.clone()));
        }
    }
}

impl NavigationViewState {
    /// Finds the visible selected indicator or its collapsed ancestor in control coordinates.
    fn indicator_rect(
        self: Handle<Self>,
        app: &mut App,
        selected: &str,
    ) -> Option<NavigationIndicatorOwner> {
        let all = self.all_items(app);
        let (_, path) = all.iter().find(|(item, _)| item.id == selected)?;
        let mut id = selected.to_owned();
        for length in 2..path.len() {
            let (parent, _) = all
                .iter()
                .find(|(_, candidate)| candidate.as_slice() == &path[..length])?;
            if !app.get(self).expanded.contains(&parent.id) {
                id = parent.id.clone();
                break;
            }
        }
        let key = app.get(self).containers.get(&id)?.indicator_key.clone();
        let context = key.current_context(app)?;
        let object = context.find_render_object(app)?.as_box()?;
        let root = self.context(app).find_render_object(app)?.as_box()?;
        let origin = object.local_to_global(app, Offset::ZERO, None)
            - root.local_to_global(app, Offset::ZERO, None);
        Some(NavigationIndicatorOwner {
            id,
            rect: origin & object.size(app),
        })
    }

    /// Source CenterPoint's 200ms step outlives an interrupted Scale storyboard.
    fn update_indicator_pivots(self: Handle<Self>, app: &mut App) {
        let now = SchedulerBinding::current_frame_time_stamp(app);
        for container in app.get(self).containers.values() {
            if let Some((deadline, final_pivot)) = container.indicator_pivot_step.get()
                && now >= deadline
            {
                container.indicator_pivot_x.set(final_pivot);
                container.indicator_pivot_step.set(None);
            }
        }
    }

    /// PlayIndicatorAnimations replaces CenterPoint.X only for same-level top navigation.
    fn schedule_indicator_pivots(self: Handle<Self>, app: &mut App) {
        let now = SchedulerBinding::current_frame_time_stamp(app);
        let Some((from, to, true)) = app.get(self).indicator_transition.as_ref() else {
            return;
        };
        if from.rect.top != to.rect.top {
            return;
        }
        let increasing = from.rect.left < to.rect.left;
        for owner in [from, to] {
            if let Some(container) = app.get(self).containers.get(&owner.id) {
                container
                    .indicator_pivot_x
                    .set(if increasing { 0.0 } else { owner.rect.width() });
                container.indicator_pivot_step.set(Some((
                    now + Duration::from_millis(200),
                    if increasing { owner.rect.width() } else { 0.0 },
                )));
            }
        }
    }

    /// Measures the destination after layout, then runs the source compositor tracks natively.
    fn animate_selection(
        self: Handle<Self>,
        app: &mut App,
        from: Option<NavigationIndicatorOwner>,
    ) {
        let selected = app.get(self).selected.clone();
        let next = selected
            .as_ref()
            .and_then(|id| self.indicator_rect(app, id));
        if app
            .get(self)
            .indicator_transition
            .as_ref()
            .is_some_and(|(_, to, _)| next.as_ref().is_some_and(|next| next.id == to.id))
        {
            return;
        }
        app.get_mut(self).indicator_epoch += 1;
        app.get_mut(self).indicator_transition = None;
        let from = from.filter(|owner| owner.rect.width() > 0.0 && owner.rect.height() > 0.0);
        app.get_mut(self).pending_indicator = from.clone();
        let epoch = app.get(self).indicator_epoch;
        let Some(from) = from else {
            return;
        };
        self.after_frame(app, move |app| {
            if app.get(self).indicator_epoch != epoch {
                return;
            }
            let selected = app.get(self).selected.clone();
            let to = selected
                .as_ref()
                .and_then(|id| self.indicator_rect(app, id));
            let top = self.widget(app).pane_display_mode == NavigationViewPaneDisplayMode::Top;
            self.set_state(app, |state| {
                state.pending_indicator = None;
                state.indicator_transition = to
                    .filter(|to| {
                        from.id != to.id && to.rect.width() > 0.0 && to.rect.height() > 0.0
                    })
                    .map(|to| (from.clone(), to, top));
            });
            if app.get(self).indicator_transition.is_some() {
                self.schedule_indicator_pivots(app);
                let controller = app.get(self).indicator_controller.unwrap();
                controller.set_value(app, 0.0);
                controller.animate_to(app, 1.0, None, Curves::linear());
            }
        });
    }

    /// Focuses a logical item and scrolls its current visual container into view.
    fn focus_item(self: Handle<Self>, app: &mut App, id: &str) {
        let Some(container) = app.get(self).containers.get(id).cloned() else {
            return;
        };
        container.focus.request_focus(app, None);
        if let Some(context) = container.key.current_context(app) {
            drop(Scrollable::ensure_visible(
                app,
                context,
                0.0,
                Duration::ZERO,
                Curves::linear(),
                ScrollPositionAlignmentPolicy::KeepVisibleAtStart,
            ));
            drop(Scrollable::ensure_visible(
                app,
                context,
                1.0,
                Duration::ZERO,
                Curves::linear(),
                ScrollPositionAlignmentPolicy::KeepVisibleAtEnd,
            ));
        }
    }

    /// Visible preorder excludes collapsed descendants, headers and separators.
    fn focus_order(self: Handle<Self>, app: &App, current_id: &str) -> Vec<String> {
        let all = self.all_items(app);
        let Some((_, current_path)) = all.iter().find(|(item, _)| item.id == current_id) else {
            return Vec::new();
        };
        let top = self.widget(app).pane_display_mode == NavigationViewPaneDisplayMode::Top;
        let compact = !self.is_pane_open(app)
            && app.get(self).display_mode != NavigationViewDisplayMode::Minimal;
        let flyout_root = current_path.len() > 2 && (top || compact);
        let prefix = if flyout_root {
            &current_path[..2]
        } else {
            &current_path[..1]
        };
        let overflow = app.get(self).top.overflow_items();
        let current_in_overflow =
            top && current_path[0] == 0 && overflow.contains(&current_path[1]);
        all.iter()
            .filter(|(item, path)| {
                if !item.is_enabled
                    || !item.is_visible
                    || item.kind != NavigationViewItemKind::Item
                    || !path.starts_with(prefix)
                    || path.len() <= prefix.len()
                {
                    return false;
                }
                if top && !flyout_root {
                    if path.len() != 2 {
                        return false;
                    }
                    if path[0] == 0 && overflow.contains(&path[1]) != current_in_overflow {
                        return false;
                    }
                }
                (2..path.len()).all(|length| {
                    all.iter()
                        .find(|(_, candidate)| candidate.as_slice() == &path[..length])
                        .is_some_and(|(parent, _)| {
                            parent.is_visible && app.get(self).expanded.contains(&parent.id)
                        })
                })
            })
            .map(|(item, _)| item.id.clone())
            .collect()
    }

    /// Source Home/End and hierarchical direction policy over the native focus tree.
    fn on_item_key(
        self: Handle<Self>,
        app: &mut App,
        id: &str,
        event: &KeyEvent,
        top: bool,
    ) -> KeyEventResult {
        if !matches!(event, KeyEvent::Down(_) | KeyEvent::Repeat(_)) {
            return KeyEventResult::Ignored;
        }
        let key = event.logical_key();
        if HardwareKeyboard::instance(app).is_alt_pressed(app) {
            return KeyEventResult::Ignored;
        }
        let mut order = self.focus_order(app, id);
        if matches!(key, LogicalKeyboardKey::HOME | LogicalKeyboardKey::END) {
            let root_depth = order
                .iter()
                .filter_map(|id| self.index_path_from_item(app, id).map(|path| path.len()))
                .min()
                .unwrap_or(2);
            order.retain(|id| {
                self.index_path_from_item(app, id)
                    .is_some_and(|path| path.len() == root_depth)
            });
        }
        let current = order
            .iter()
            .position(|candidate| candidate == id)
            .unwrap_or(0);
        if order.is_empty() {
            return KeyEventResult::Ignored;
        }
        let rtl = Directionality::of(app, self.context(app)) == TextDirection::Rtl;
        let forward = if top {
            if rtl {
                LogicalKeyboardKey::ARROW_LEFT
            } else {
                LogicalKeyboardKey::ARROW_RIGHT
            }
        } else {
            LogicalKeyboardKey::ARROW_DOWN
        };
        let backward = if top {
            if rtl {
                LogicalKeyboardKey::ARROW_RIGHT
            } else {
                LogicalKeyboardKey::ARROW_LEFT
            }
        } else {
            LogicalKeyboardKey::ARROW_UP
        };
        let next = if key == LogicalKeyboardKey::HOME {
            Some(0)
        } else if key == LogicalKeyboardKey::END {
            order.len().checked_sub(1)
        } else if key == forward {
            (current + 1 < order.len()).then_some(current + 1)
        } else if key == backward {
            current.checked_sub(1)
        } else {
            None
        };
        if let Some(next) = next {
            self.focus_item(app, &order[next]);
            return KeyEventResult::Handled;
        }
        let expand_key = if rtl {
            LogicalKeyboardKey::ARROW_LEFT
        } else {
            LogicalKeyboardKey::ARROW_RIGHT
        };
        let collapse_key = if rtl {
            LogicalKeyboardKey::ARROW_RIGHT
        } else {
            LogicalKeyboardKey::ARROW_LEFT
        };
        if !top && key == expand_key {
            if let Some(args) = self.event_args(app, id, false)
                && (!args.item.menu_items.is_empty() || args.item.has_unrealized_children)
            {
                if !app.get(self).expanded.contains(id) {
                    self.toggle_expanded(app, id);
                } else if let Some(child) = args.item.menu_items.iter().find(|item| {
                    item.is_enabled && item.is_visible && item.kind == NavigationViewItemKind::Item
                }) {
                    self.focus_item(app, &child.id);
                }
                return KeyEventResult::Handled;
            }
        } else if !top && key == collapse_key {
            if app.get(self).expanded.contains(id) {
                self.toggle_expanded(app, id);
                return KeyEventResult::Handled;
            }
            if let Some(mut path) = self.index_path_from_item(app, id)
                && path.len() > 2
            {
                path.pop();
                if let Some((parent, _)) = self
                    .all_items(app)
                    .into_iter()
                    .find(|(_, candidate)| *candidate == path)
                {
                    self.focus_item(app, &parent.id);
                    return KeyEventResult::Handled;
                }
            }
        }
        KeyEventResult::Ignored
    }

    /// Builds one keyed eager item and its inline or attached child repeater.
    fn item_widget(
        self: Handle<Self>,
        r: &ThemeResources,
        app: &mut App,
        item: NavigationViewItem,
        position: NavigationViewItemPosition,
        depth: usize,
        closed_compact: bool,
    ) -> WidgetRef {
        let id = item.id.clone();
        let container = app.get(self).containers[&id].clone();
        let expanded = app.get(self).expanded.contains(&id);
        let selected = app.get(self).selected.as_deref() == Some(&id);
        let child_selected = app.get(self).selected.as_ref().is_some_and(|selected| {
            let mut descendants = Vec::new();
            collect_items(&item.menu_items, &[], &mut descendants);
            descendants.iter().any(|(item, _)| &item.id == selected)
        });
        let top_primary = position == NavigationViewItemPosition::TopPrimary;
        let top = matches!(
            position,
            NavigationViewItemPosition::TopPrimary | NavigationViewItemPosition::TopFooter
        );
        let overflow = position == NavigationViewItemPosition::TopOverflow;
        let is_top_level_item = self
            .index_path_from_item(app, &id)
            .is_some_and(|path| path.len() == 2);
        let flyout_children = (closed_compact && is_top_level_item) || top_primary;
        let invoke_id = id.clone();
        let expand_id = id.clone();
        let focus_id = id.clone();
        let key_id = id.clone();
        let presenter = NavigationViewItemPresenter {
            item: item.clone(),
            position,
            is_selected: selected,
            is_child_selected: child_selected,
            is_expanded: expanded,
            is_closed_compact: closed_compact,
            is_top_level_item,
            depth,
            compact_pane_length: self.widget(app).compact_pane_length,
            focus_node: container.focus,
            is_keyboard_focused: false,
            indicator_key: container.indicator_key.clone(),
            indicator_controller: app.get(self).indicator_controller.unwrap(),
            indicator_animation: app.get(self).indicator_transition.as_ref().and_then(
                |(from, to, top)| {
                    (item.id == from.id || item.id == to.id).then(|| {
                        NavigationIndicatorTransition {
                            from: from.rect,
                            to: to.rect,
                            top: *top,
                            outgoing: item.id == from.id,
                            pivot_x: container.indicator_pivot_x.clone(),
                        }
                    })
                },
            ),
            indicator_opacity: app
                .get(self)
                .pending_indicator
                .as_ref()
                .map(|from| if item.id == from.id { 1.0 } else { 0.0 }),
            on_invoked: Listener::new(move |app| self.invoke(app, &invoke_id, overflow)),
            on_expansion_toggled: Listener::new(move |app| self.toggle_expanded(app, &expand_id)),
        };
        let presenter = Focus::new(presenter)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _, event| {
                self.on_item_key(app, &key_id, event, top)
            }))
            .on_focus_change(move |app, focused| {
                if focused
                    && self.widget(app).selection_follows_focus
                        == NavigationViewSelectionFollowsFocus::Enabled
                    && app.get(self).selected.as_deref() != Some(&focus_id)
                    && self
                        .event_args(app, &focus_id, false)
                        .is_some_and(|args| args.item.selects_on_invoked)
                {
                    // Source excludes focus received inside an overflow flyout from selection-following.
                    if !overflow {
                        self.invoke(app, &focus_id, false);
                    }
                }
            })
            .into_widget();
        let presenter = if top {
            let measure_id = id.clone();
            SizeObserver::new(
                presenter,
                Rc::new(move |app, _, size| {
                    if !app.contains(self) || !self.mounted(app) {
                        return;
                    }
                    let index = self
                        .widget(app)
                        .menu_items
                        .iter()
                        .position(|item| item.id == measure_id);
                    if let Some(index) = index
                        && app
                            .get_mut(self)
                            .top
                            .set_width_for_item(index, size.width())
                    {
                        self.set_state(app, |_| {});
                    }
                }),
            )
            .into_widget()
        } else {
            presenter
        };
        let children: Vec<_> = item
            .menu_items
            .iter()
            .filter(|item| item.is_visible)
            .cloned()
            .map(|child| {
                self.item_widget(
                    r,
                    app,
                    child,
                    if top {
                        NavigationViewItemPosition::TopOverflow
                    } else {
                        position
                    },
                    if flyout_children { 0 } else { depth + 1 },
                    closed_compact,
                )
            })
            .collect();
        let child_column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .children(children)
            .into_widget();
        // Vertical child repeaters measure naturally inside the unbounded horizontal top strip too.
        let child_column = if flyout_children || top {
            IntrinsicWidth::new().child(child_column).into_widget()
        } else {
            child_column
        };
        let result =
            if flyout_children && (!item.menu_items.is_empty() || item.has_unrealized_children) {
                let dismiss_id = id.clone();
                let popup = child_column.clone();
                let anchor = AnchoredFlyout::new(
                    expanded,
                    presenter,
                    popup,
                    Listener::new(move |app| {
                        if app.get(self).expanded.contains(&dismiss_id) {
                            self.toggle_expanded(app, &dismiss_id);
                        }
                    }),
                )
                .placement(if top {
                    AnchoredFlyoutPlacement::BottomEdgeAlignedLeft
                } else {
                    AnchoredFlyoutPlacement::RightEdgeAlignedTop
                })
                .padding(r.navigation_view_item_children_menu_flyout_padding)
                .offset(if top {
                    Offset::ZERO
                } else {
                    Offset::new(0.0, -4.0)
                });
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(if top {
                        CrossAxisAlignment::Start
                    } else {
                        CrossAxisAlignment::Stretch
                    })
                    .children([
                        anchor.into_widget(),
                        Offstage::new()
                            .offstage(true)
                            .child(if expanded {
                                SizedBox::shrink().into_widget()
                            } else {
                                child_column
                            })
                            .into_widget(),
                    ])
                    .into_widget()
            } else {
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(if top {
                        CrossAxisAlignment::Start
                    } else {
                        CrossAxisAlignment::Stretch
                    })
                    .children([
                        presenter,
                        Offstage::new()
                            .offstage(!expanded)
                            .child(
                                Focus::new(child_column)
                                    .can_request_focus(false)
                                    .descendants_are_focusable(expanded),
                            )
                            .into_widget(),
                    ])
                    .into_widget()
            };
        KeyedSubtree::new(result).key(container.key).into_widget()
    }

    /// Measures a group's natural height without replacing its source-owned scroll controller.
    fn measured_group(self: Handle<Self>, children: Vec<WidgetRef>, group: usize) -> WidgetRef {
        SizeObserver::new(
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .children(children),
            Rc::new(move |app, _, size| {
                if app.contains(self)
                    && self.mounted(app)
                    && app.get(self).group_heights[group] != size.height()
                {
                    self.set_state(app, |state| state.group_heights[group] = size.height());
                }
            }),
        )
        .into_widget()
    }

    /// Main/footer scroll allocation from UpdatePaneLayout, preserving the smaller group.
    fn pane_items(self: Handle<Self>, app: &mut App, closed_compact: bool) -> WidgetRef {
        let r = ThemeResources::of(app, self.context(app));
        let main_items = self.widget(app).menu_items.clone();
        let footer_items = self.footer_items(app);
        let main = main_items
            .into_iter()
            .filter(|item| item.is_visible)
            .map(|item| {
                self.item_widget(
                    &r,
                    app,
                    item,
                    NavigationViewItemPosition::Left,
                    0,
                    closed_compact,
                )
            })
            .collect();
        let footer = footer_items
            .into_iter()
            .filter(|item| item.is_visible)
            .map(|item| {
                self.item_widget(
                    &r,
                    app,
                    item,
                    NavigationViewItemPosition::Left,
                    0,
                    closed_compact,
                )
            })
            .collect();
        let main = self.measured_group(main, 0);
        let footer = self.measured_group(footer, 1);
        let custom = self
            .widget(app)
            .pane_footer
            .clone()
            .map(|child| self.slot(app, "footer", child))
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        let custom = self.measured_group(vec![custom], 2);
        LayoutBuilder::new(move |app, context, constraints| {
            let height = constraints.max_height;
            let [main_height, footer_height, custom_height] = app.get(self).group_heights;
            let footer_group = footer_height + custom_height;
            let main_space = if footer_height == 0.0 {
                (height - custom_height).max(0.0)
            } else if main_height == 0.0 {
                0.0
            } else if main_height + footer_group <= height {
                height - footer_group
            } else if main_height <= height / 2.0 {
                main_height
            } else if footer_group <= height / 2.0 {
                height - footer_group
            } else {
                height / 2.0
            };
            let separator =
                main_height > 0.0 && footer_height > 0.0 && main_height + footer_group > height;
            let theme = ThemeResources::of(app, context);
            let line_height = if separator { 3.0 } else { 0.0 };
            let main_space = main_space.min((height - line_height).max(0.0));
            let footer_space = (height - main_space - custom_height - line_height).max(0.0);
            let scrolls = app.get(self).scrolls.clone();
            Column::new()
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .children([
                    SizedBox::new()
                        .height(main_space)
                        .child(ScrollBarViewport::new(scrolls[0], main.clone()))
                        .into_widget(),
                    SizedBox::new()
                        .height(line_height)
                        .child(Align::new().alignment(Alignment::TOP_CENTER.into()).child(
                            SizedBox::new().height(1.0).child(ColoredBox::new(
                                theme.navigation_view_item_separator_foreground,
                            )),
                        ))
                        .into_widget(),
                    ConstrainedBox::new(BoxConstraints::new().max_height(height))
                        .child(custom.clone())
                        .into_widget(),
                    SizedBox::new()
                        .height(footer_space)
                        .child(ScrollBarViewport::new(scrolls[1], footer.clone()))
                        .into_widget(),
                ])
                .into_widget()
        })
        .into_widget()
    }
}

impl NavigationViewState {
    /// Builds the source toggle/back/overflow button template with its discrete states.
    fn icon_button(
        self: Handle<Self>,
        icon: FluentSymbol,
        callback: Listener,
        enabled: bool,
        tooltip: &str,
        focus_margin: [f64; 4],
    ) -> WidgetRef {
        ToolTipService::new(
            Button::new(
                FontIcon::symbol(icon)
                    .font_size(16.0)
                    .mirrored_when_right_to_left(true),
                callback,
            )
            .is_enabled(enabled)
            .template(move |app, context, states, content| {
                navigation_button_template(app, context, states, content, focus_margin)
            }),
            ToolTip::text(tooltip),
        )
        .into_widget()
    }

    /// Keeps the native search editor mounted while a compact pane shows its search button.
    fn search_presenter(
        self: Handle<Self>,
        r: &ThemeResources,
        app: &mut App,
        top: bool,
        compact: bool,
    ) -> WidgetRef {
        let Some(child) = self.widget(app).auto_suggest_box.clone() else {
            return SizedBox::shrink().into_widget();
        };
        let scope = app.get(self).search_focus.unwrap();
        let editor = FocusScope::new(self.slot(app, "search", child))
            .node(scope)
            .descendants_are_focusable(!compact);
        let padding = if top {
            r.top_navigation_view_auto_suggest_box_margin
        } else {
            r.navigation_view_auto_suggest_box_margin
        };
        let editor = Padding::new(EdgeInsetsGeometry::only(
            padding[0], padding[1], padding[2], padding[3],
        ))
        .child(
            ConstrainedBox::new(
                BoxConstraints::new()
                    .min_height(r.navigation_view_auto_suggest_area_height)
                    .min_width(if top { 216.0 } else { 0.0 }),
            )
            .child(editor),
        );
        if compact {
            let button = self.icon_button(
                FluentSymbol::Search,
                Listener::new(move |app| {
                    self.set_pane_open(app, true, true);
                    self.after_frame(app, move |app| {
                        if let Some(node) =
                            scope.as_node().traversal_descendants(app).first().copied()
                        {
                            node.request_focus(app, None);
                        }
                    });
                }),
                true,
                "Search",
                [-4.0, 0.0, -4.0, 0.0],
            );
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .children([
                    button,
                    Offstage::new().offstage(true).child(editor).into_widget(),
                ])
                .into_widget()
        } else {
            Padding::new(EdgeInsetsGeometry::only(
                0.0,
                0.0,
                0.0,
                if top { 0.0 } else { 8.0 },
            ))
            .child(editor)
            .into_widget()
        }
    }

    /// The page HeaderContent and ContentPresenter preserve owner state across pane layouts.
    fn page_content(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        top: bool,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let resources = ThemeResources::of(app, context);
        let r = &resources;
        let minimal = app.get(self).display_mode == NavigationViewDisplayMode::Minimal;
        let (border, corners) = if top {
            (
                r.top_navigation_view_content_grid_border_thickness,
                r.top_navigation_view_content_grid_corner_radius,
            )
        } else if minimal {
            (
                r.navigation_view_minimal_content_grid_border_thickness,
                r.navigation_view_minimal_content_grid_corner_radius,
            )
        } else {
            (
                r.navigation_view_content_grid_border_thickness,
                r.navigation_view_content_grid_corner_radius,
            )
        };
        let chrome = PaneChrome::resolve(
            &resources,
            &widget,
            app.get(self).display_mode,
            self.is_pane_open(app),
            app.get(self).width,
        );
        let mut children = vec![
            GridCell::new(SizedBox::new().width(chrome.content_left_padding))
                .row(1)
                .into_widget(),
        ];
        if let Some(header) = widget
            .header
            .clone()
            .filter(|_| widget.always_show_header || (minimal && !top))
        {
            let margin = if minimal && !top {
                r.navigation_view_minimal_header_margin
            } else {
                r.navigation_view_header_margin
            };
            children.push(
                GridCell::new(Margin::new(
                    margin,
                    ConstrainedBox::new(
                        BoxConstraints::new().min_height(r.pane_toggle_button_height),
                    )
                    .child(DefaultTextStyle::new(
                        control_text_style(
                            &resources,
                            28.0,
                            FontWeight::W600,
                            resources.text_fill_color_primary,
                        ),
                        self.slot(app, "header", header),
                    )),
                ))
                .row(1)
                .column(1)
                .into_widget(),
            );
        }
        children.push(
            GridCell::new(Margin::new(
                r.navigation_view_content_presenter_margin,
                self.slot(app, "content", widget.content),
            ))
            .row(2)
            .column_span(2)
            .into_widget(),
        );
        Grid::new()
            .background(Brush::Solid(r.navigation_view_content_background))
            .border_brush(Brush::Solid(r.navigation_view_content_grid_border_brush))
            .border_thickness_ltrb(border)
            .corner_radius_corners(corners)
            .column_definitions([
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::STAR),
            ])
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::STAR),
            ])
            .children(children)
            .into_widget()
    }

    /// PaneContentGrid reserves the toggle row before custom content and item groups.
    fn left_pane(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let r = ThemeResources::of(app, self.context(app));
        let widget = self.widget(app).clone();
        let chrome = PaneChrome::resolve(
            &r,
            &widget,
            app.get(self).display_mode,
            self.is_pane_open(app),
            app.get(self).width,
        );
        let compact = chrome.closed_compact;
        let header = widget
            .pane_header
            .clone()
            .map(|child| self.slot(app, "pane_header", child))
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        // PaneHeaderContentBorder is independent of PaneTitleTextBlock and collapses in ListSizeCompact.
        let header = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::AUTO).min_height(chrome.header_row_min_height)
            ])
            .column_definitions([
                ColumnDefinition::new(GridLength::pixel(chrome.header_close_width)),
                ColumnDefinition::new(GridLength::pixel(chrome.header_toggle_width)),
                ColumnDefinition::new(GridLength::STAR),
            ])
            .children([GridCell::new(
                Offstage::new()
                    .offstage(compact)
                    .child(Focus::new(header).descendants_are_focusable(!compact)),
            )
            .column(2)
            .into_widget()]);
        let custom = widget
            .pane_custom_content
            .map(|child| self.slot(app, "custom", child))
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        let search = self.search_presenter(&r, app, false, compact);
        let items = self.pane_items(app, compact);
        // PaneContentGrid follows the template's padding, Back, toggle, search, custom and item rows.
        let pane = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::pixel(chrome.back_row)),
                RowDefinition::new(GridLength::AUTO).min_height(chrome.toggle_row_min_height),
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::pixel(0.0)),
                RowDefinition::new(GridLength::STAR),
            ])
            .children([
                GridCell::new(header).row(2).into_widget(),
                GridCell::new(search).row(3).into_widget(),
                GridCell::new(custom).row(4).into_widget(),
                GridCell::new(items).row(6).into_widget(),
            ])
            .into_widget();
        // ListSizeCompact sizes PaneContentGrid itself; SplitView's outer clip is not its layout width.
        let mut pane = SizedBox::new().child(pane);
        if compact {
            pane = pane.width(widget.compact_pane_length);
        }
        Align::new()
            .alignment(Alignment::TOP_LEFT.into())
            .child(pane)
            .into_widget()
    }

    /// RootSplitView maps Expanded/Compact/Minimal to the three source SplitView modes.
    fn left_template(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let resources = ThemeResources::of(app, context);
        let r = &resources;
        let open = self.is_pane_open(app) && widget.is_pane_visible;
        let mode = app.get(self).display_mode;
        let chrome = PaneChrome::resolve(&resources, &widget, mode, open, app.get(self).width);
        let pane = self.left_pane(app);
        let content = self.page_content(app, context, false);
        let mut split = SplitView::new(open, move |app, open| self.set_pane_open(app, open, false))
            .display_mode(if !widget.is_pane_visible {
                SplitViewDisplayMode::CompactOverlay
            } else {
                match mode {
                    NavigationViewDisplayMode::Expanded => SplitViewDisplayMode::CompactInline,
                    NavigationViewDisplayMode::Compact => SplitViewDisplayMode::CompactOverlay,
                    NavigationViewDisplayMode::Minimal => SplitViewDisplayMode::Overlay,
                }
            })
            .compact_pane_length(if widget.is_pane_visible {
                widget.compact_pane_length
            } else {
                0.0
            })
            .open_pane_length(widget.open_pane_length.min(app.get(self).width).max(0.0))
            .pane_background(if mode == NavigationViewDisplayMode::Expanded {
                Brush::Solid(r.navigation_view_expanded_pane_background)
            } else {
                Brush::Acrylic(r.navigation_view_default_pane_background)
            })
            .border_brush(Brush::Solid(
                if mode == NavigationViewDisplayMode::Expanded {
                    Color::new(0)
                } else {
                    r.navigation_view_item_separator_foreground
                },
            ))
            .border_thickness_ltrb(r.navigation_view_border_thickness)
            .corner_radius_corners([
                0.0,
                r.overlay_corner_radius[1],
                r.overlay_corner_radius[2],
                0.0,
            ])
            .pane(Margin::new(
                r.navigation_view_pane_content_grid_margin,
                pane,
            ))
            .content(content);
        if let Some(callback) = widget.pane_opening {
            split = split.pane_opening(callback);
        }
        if let Some(callback) = widget.pane_opened {
            split = split.pane_opened(callback);
        }
        if let Some(callback) = widget.pane_closed {
            split = split.pane_closed(callback);
        }
        // SplitView's cancelable light-dismiss request is forwarded before its value callback.
        split = split.pane_closing(move |app, args| {
            if std::mem::take(&mut app.get_mut(self).block_next_closing) {
                return;
            }
            if let Some(callback) = self.widget(app).pane_closing.clone() {
                callback(app, args);
            }
        });
        let mut children = vec![split.into_widget()];
        if widget.is_pane_visible && (chrome.show_back || chrome.show_close) {
            let close = chrome.show_close;
            let callback = if close {
                Listener::new(move |app| self.set_pane_open(app, false, true))
            } else {
                widget
                    .back_requested
                    .clone()
                    .unwrap_or_else(|| Listener::new(|_| {}))
            };
            let back_width = if close {
                r.navigation_back_button_width
            } else {
                chrome.back_width
            };
            let small = chrome.overlay && !close;
            let button = Button::new(
                FontIcon::symbol(FluentSymbol::Back)
                    .font_size(16.0)
                    .mirrored_when_right_to_left(true),
                callback,
            )
            .key(Rc::new(inset_foundation::ValueKey::new(if close {
                "NavigationViewCloseButton"
            } else {
                "NavigationViewBackButton"
            })))
            .is_enabled(close || widget.is_back_enabled)
            .template(move |app, context, states, child| {
                back_button_template(app, context, states, child, back_width, small)
            });
            children.push(
                Positioned::new(ToolTipService::new(
                    button,
                    ToolTip::text(if close { "Close navigation" } else { "Back" }),
                ))
                .left(0.0)
                .top(4.0)
                .into_widget(),
            );
        }
        if chrome.show_toggle {
            let icon_width = (widget.compact_pane_length - 8.0).max(0.0);
            let title = Offstage::new()
                .offstage(chrome.closed_compact)
                .child(pane_title(&resources, widget.pane_title.clone()));
            let button = Button::new(
                title,
                Listener::new(move |app| self.set_pane_open(app, !self.is_pane_open(app), true)),
            )
            .key(Rc::new(inset_foundation::ValueKey::new("TogglePaneButton")))
            .template(move |app, context, states, child| {
                pane_toggle_template(app, context, states, child, icon_width)
            });
            children.push(
                Positioned::new(SizedBox::new().width(chrome.toggle_width).child(
                    ToolTipService::new(
                        button,
                        ToolTip::text(if open {
                            "Close navigation"
                        } else {
                            "Open navigation"
                        }),
                    ),
                ))
                .left(chrome.toggle_left)
                .top(4.0 + chrome.toggle_top)
                .into_widget(),
            );
        } else if chrome.show_title_holder && widget.is_pane_visible {
            children.push(
                Positioned::new(
                    SizedBox::new()
                        .height(r.navigation_view_pane_header_row_min_height)
                        .child(Margin::new(
                            r.navigation_view_pane_title_presenter_margin,
                            Offstage::new()
                                .offstage(chrome.closed_compact)
                                .child(pane_title(&resources, widget.pane_title.clone())),
                        )),
                )
                .left(chrome.toggle_left)
                .top(4.0 + chrome.toggle_top)
                .into_widget(),
            );
        }
        Stack::new()
            .fit(StackFit::Expand)
            .children(children)
            .into_widget()
    }

    /// Measures top chrome independently of the star column's allocated width.
    fn top_chrome(
        self: Handle<Self>,
        r: &ThemeResources,
        child: WidgetRef,
        slot: usize,
    ) -> WidgetRef {
        let custom_content_min_width = r.top_navigation_view_pane_custom_content_min_width;
        SizeObserver::new(
            child,
            Rc::new(move |app, _, size| {
                if app.contains(self)
                    && self.mounted(app)
                    && app.get(self).top_chrome_sizes[slot] != size.width()
                {
                    self.set_state(app, |state| {
                        state.top_chrome_sizes[slot] = size.width();
                        state.top_chrome_width = state.top_chrome_sizes[0]
                            + state.top_chrome_sizes[1]
                            + state.top_chrome_sizes[2].max(custom_content_min_width)
                            + 8.0;
                    });
                }
            }),
        )
        .into_widget()
    }

    /// TopNavGrid projects original indices without turning overflow into horizontal scrolling.
    fn top_template(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let resources = ThemeResources::of(app, context);
        let r = &resources;
        let selected = app
            .get(self)
            .selected
            .as_ref()
            .and_then(|id| self.index_path_from_item(app, id))
            .filter(|path| path[0] == 0)
            .map(|path| path[1]);
        let available = (app.get(self).width - app.get(self).top_chrome_width).max(0.0);
        app.get_mut(self).top.overflow_button_width = r.top_navigation_view_overflow_button_width;
        if app.get(self).top_measure_pending {
            self.after_frame(app, move |app| {
                let ids: Vec<_> = self
                    .widget(app)
                    .menu_items
                    .iter()
                    .map(|item| item.id.clone())
                    .collect();
                for (index, id) in ids.iter().enumerate() {
                    if !self.widget(app).menu_items[index].is_visible {
                        app.get_mut(self).top.set_width_for_item(index, 0.0);
                        continue;
                    }
                    if let Some(context) = self.container_from_menu_item(app, id)
                        && let Some(object) = context
                            .find_render_object(app)
                            .and_then(|object| object.as_box())
                    {
                        let width = object.size(app).width();
                        app.get_mut(self).top.set_width_for_item(index, width);
                    }
                }
                self.set_state(app, |state| state.top_measure_pending = false);
            });
        } else {
            app.get_mut(self).top.arrange(available, selected);
        }
        let primary_indices = app.get(self).top.primary_items();
        let overflow_indices = app.get(self).top.overflow_items();
        let overflow_open = app.get(self).overflow_open && !overflow_indices.is_empty();
        let primary: Vec<_> = primary_indices
            .into_iter()
            .filter(|index| widget.menu_items[*index].is_visible)
            .map(|index| {
                self.item_widget(
                    &resources,
                    app,
                    widget.menu_items[index].clone(),
                    NavigationViewItemPosition::TopPrimary,
                    0,
                    false,
                )
            })
            .collect();
        let overflow: Vec<_> = overflow_indices
            .iter()
            .filter(|index| widget.menu_items[**index].is_visible)
            .map(|index| {
                self.item_widget(
                    &resources,
                    app,
                    widget.menu_items[*index].clone(),
                    if overflow_open {
                        NavigationViewItemPosition::TopOverflow
                    } else {
                        NavigationViewItemPosition::TopPrimary
                    },
                    0,
                    false,
                )
            })
            .collect();
        let primary = ClipRect::new().child(
            Row::new()
                .main_axis_size(MainAxisSize::Min)
                .children(primary),
        );
        let has_overflow = !overflow_indices.is_empty();
        let overflow_button = self.icon_button(
            FluentSymbol::More,
            Listener::new(move |app| {
                self.set_state(app, |state| state.overflow_open = !state.overflow_open)
            }),
            true,
            "More",
            [0.0; 4],
        );
        let popup = IntrinsicWidth::new().child(
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .children(if overflow_open {
                    overflow.clone()
                } else {
                    Vec::new()
                }),
        );
        let overflow_button = AnchoredFlyout::new(
            overflow_open,
            overflow_button,
            popup,
            Listener::new(move |app| self.set_state(app, |state| state.overflow_open = false)),
        )
        .padding(r.top_navigation_view_overflow_menu_padding)
        .max_width(r.flyout_theme_max_width);
        let chrome = PaneChrome::resolve(
            &resources,
            &widget,
            app.get(self).display_mode,
            self.is_pane_open(app),
            app.get(self).width,
        );
        // PaneHeaderOnTopPane and PaneTitleOnTopPane share column 2 in the source template.
        let title_column = Stack::new().children([
            widget
                .pane_header
                .clone()
                .map(|child| self.slot(app, "pane_header", child))
                .unwrap_or_else(|| SizedBox::shrink().into_widget()),
            if chrome.show_top_title {
                Margin::new(
                    r.navigation_view_item_inner_header_margin,
                    pane_title(&resources, widget.pane_title.clone()),
                )
                .into_widget()
            } else {
                SizedBox::shrink().into_widget()
            },
        ]);
        let leading = Row::new()
            .main_axis_size(MainAxisSize::Min)
            .children([
                SizedBox::new()
                    .width(if chrome.show_back {
                        r.navigation_back_button_width
                    } else {
                        0.0
                    })
                    .into_widget(),
                title_column.into_widget(),
            ])
            .into_widget();
        let footer_items = self
            .footer_items(app)
            .into_iter()
            .filter(|item| item.is_visible)
            .map(|item| {
                self.item_widget(
                    &resources,
                    app,
                    item,
                    NavigationViewItemPosition::TopFooter,
                    0,
                    false,
                )
            })
            .collect::<Vec<_>>();
        let search = self.search_presenter(&resources, app, true, false);
        let trailing = Row::new()
            .main_axis_size(MainAxisSize::Min)
            .children([
                search,
                widget
                    .pane_footer
                    .clone()
                    .map(|child| self.slot(app, "footer", child))
                    .unwrap_or_else(|| SizedBox::shrink().into_widget()),
                Row::new()
                    .main_axis_size(MainAxisSize::Min)
                    .children(footer_items)
                    .into_widget(),
            ])
            .into_widget();
        let leading = self.top_chrome(&resources, leading, 0);
        let trailing = self.top_chrome(&resources, trailing, 1);
        let custom = widget
            .pane_custom_content
            .clone()
            .map(|child| self.slot(app, "custom", child))
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        let custom = Align::new()
            .alignment(Alignment::CENTER_LEFT.into())
            .child(self.top_chrome(&resources, custom, 2));
        let top_row = Grid::new()
            .column_definitions([
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::AUTO),
                // WinUI measures TopNavGrid at infinite width before partitioning items.
                ColumnDefinition::new(if app.get(self).top_measure_pending {
                    GridLength::AUTO
                } else {
                    GridLength::STAR
                })
                .min_width(
                    app.get(self).top_chrome_sizes[2]
                        .max(r.top_navigation_view_pane_custom_content_min_width),
                ),
                ColumnDefinition::new(GridLength::AUTO),
            ])
            .children([
                GridCell::new(leading).into_widget(),
                GridCell::new(
                    ConstrainedBox::new(
                        BoxConstraints::new().max_width(
                            (available
                                - if has_overflow {
                                    r.top_navigation_view_overflow_button_width
                                } else {
                                    0.0
                                })
                            .max(0.0),
                        ),
                    )
                    .child(primary),
                )
                .column(1)
                .into_widget(),
                GridCell::new(if has_overflow {
                    overflow_button.into_widget()
                } else {
                    SizedBox::shrink().into_widget()
                })
                .column(2)
                .into_widget(),
                GridCell::new(custom).column(3).into_widget(),
                GridCell::new(trailing).column(4).into_widget(),
            ]);
        let top_row = Offstage::new().offstage(!widget.is_pane_visible).child(
            SizedBox::new()
                .height(r.navigation_view_top_pane_height)
                .child(Padding::new(EdgeInsetsGeometry::only(4.0, 0.0, 4.0, 0.0)).child(top_row)),
        );
        let content = self.page_content(app, context, true);
        let overlay = widget
            .content_overlay
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        let rows = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::star(1.0)),
            ])
            .children([
                GridCell::new(
                    ColoredBox::new(r.navigation_view_top_pane_background).child(top_row),
                )
                .into_widget(),
                GridCell::new(overlay).row(1).into_widget(),
                GridCell::new(content).row(2).into_widget(),
            ]);
        let mut children = vec![
            rows.into_widget(),
            Offstage::new()
                .offstage(true)
                .child(
                    Row::new()
                        .main_axis_size(MainAxisSize::Min)
                        .children(if overflow_open { Vec::new() } else { overflow }),
                )
                .into_widget(),
        ];
        if widget.is_pane_visible && chrome.show_back {
            let width = chrome.back_width;
            let button = Button::new(
                FontIcon::symbol(FluentSymbol::Back)
                    .font_size(16.0)
                    .mirrored_when_right_to_left(true),
                widget
                    .back_requested
                    .clone()
                    .unwrap_or_else(|| Listener::new(|_| {})),
            )
            .key(Rc::new(inset_foundation::ValueKey::new(
                "NavigationViewBackButton",
            )))
            .is_enabled(widget.is_back_enabled)
            .template(move |app, context, states, child| {
                back_button_template(app, context, states, child, width, false)
            });
            children.push(
                Positioned::new(ToolTipService::new(button, ToolTip::text("Back")))
                    .left(0.0)
                    .top(4.0)
                    .into_widget(),
            );
        }
        Stack::new()
            .fit(StackFit::Expand)
            .children(children)
            .into_widget()
    }
}

impl State for NavigationViewState {
    type Widget = NavigationView;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = AnimationController::create(
            app,
            Some(0.0),
            Some(NAVIGATION_INDICATOR_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).indicator_controller = Some(controller);
        controller.add_listener(
            app,
            Listener::new(move |app| {
                self.update_indicator_pivots(app);
                if controller.value(app) >= 1.0 && app.get(self).indicator_transition.is_some() {
                    self.set_state(app, |state| state.indicator_transition = None);
                }
            }),
        );
        let scope = FocusScopeNode::new(app);
        app.get_mut(self).search_focus = Some(scope);
        for _ in 0..2 {
            let scroll = ScrollViewportController::new(app);
            app.get_mut(self).scrolls.push(scroll);
        }
        self.sync_items(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &NavigationView) {
        if old.pane_display_mode != self.widget(app).pane_display_mode {
            app.get_mut(self).was_force_closed = false;
        }
        if old.is_pane_open != self.widget(app).is_pane_open
            && let Some(open) = self.widget(app).is_pane_open
        {
            app.get_mut(self).was_force_closed = !open;
            if app.get(self).pane_open != open {
                app.get_mut(self).pane_open = open;
                self.apply_pane_expansion_memory(app, open);
            }
        }
        if old.is_pane_visible != self.widget(app).is_pane_visible {
            let open = self.widget(app).is_pane_visible
                && app.get(self).display_mode == NavigationViewDisplayMode::Expanded;
            self.after_frame(app, move |app| self.set_pane_open(app, open, false));
        }
        self.sync_items(app);
        if old.selected_item != self.widget(app).selected_item
            && app.get(self).selected != self.widget(app).selected_item
        {
            let previous = app
                .get(self)
                .selected
                .clone()
                .and_then(|id| self.indicator_rect(app, &id));
            app.get_mut(self).selected = self.widget(app).selected_item.clone();
            self.animate_selection(app, previous);
        }
        if app
            .get(self)
            .selected
            .as_ref()
            .is_some_and(|id| !self.all_items(app).iter().any(|(item, _)| &item.id == id))
        {
            app.get_mut(self).selected = None;
            app.get_mut(self).indicator_transition = None;
            app.get_mut(self).pending_indicator = None;
            app.get_mut(self).indicator_epoch += 1;
            let callback = self.widget(app).selection_changed.clone();
            self.after_frame(app, move |app| {
                callback(
                    app,
                    NavigationViewSelectionChangedEventArgs {
                        item: None,
                        index_path: Vec::new(),
                        is_settings: false,
                        recommended_transition_direction:
                            NavigationRecommendedTransitionDirection::Default,
                    },
                )
            });
        }
        if old.compact_mode_threshold_width != self.widget(app).compact_mode_threshold_width
            || old.expanded_mode_threshold_width != self.widget(app).expanded_mode_threshold_width
        {
            app.get_mut(self).width = f64::NAN;
        }
        if old
            .pane_custom_content
            .as_ref()
            .zip(self.widget(app).pane_custom_content.as_ref())
            .is_some_and(|(old, new)| !Rc::ptr_eq(old, new))
            || old.pane_custom_content.is_some() != self.widget(app).pane_custom_content.is_some()
            || old
                .menu_items
                .iter()
                .zip(&self.widget(app).menu_items)
                .any(|(old, new)| {
                    old.text != new.text
                        || old.has_content != new.has_content
                        || old.is_visible != new.is_visible
                        || old.icon.is_some() != new.icon.is_some()
                        || (old.text.is_none() && !Rc::ptr_eq(&old.content, &new.content))
                })
        {
            app.get_mut(self).top.invalidate_width_cache();
            app.get_mut(self).top_measure_pending = true;
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = app.get_mut(self).indicator_controller.take().unwrap();
        controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                app.destroy(controller);
            }),
            // Disposed, the state is in no view any more.
            None,
        );

        if let Some(scope) = app.get_mut(self).search_focus.take() {
            scope.as_node().dispose(app);
            app.destroy(scope);
        }
        for scroll in std::mem::take(&mut app.get_mut(self).scrolls) {
            scroll.dispose(app);
            app.destroy(scroll);
        }
        for (_, container) in std::mem::take(&mut app.get_mut(self).containers) {
            container.focus.dispose(app);
            app.destroy(container.focus.id());
        }
    }

    fn build(self: Handle<Self>, _: &mut App, _: BuildContext) -> WidgetRef {
        let content = LayoutBuilder::new(move |app, context, constraints| {
            self.update_adaptive_layout(app, constraints.max_width);
            let resources = ThemeResources::of(app, context);
            let child = if self.widget(app).pane_display_mode == NavigationViewPaneDisplayMode::Top
            {
                self.top_template(app, context)
            } else {
                self.left_template(app, context)
            };
            DefaultTextStyle::new(
                control_text_style(
                    &resources,
                    14.0,
                    FontWeight::NORMAL,
                    resources.text_fill_color_primary,
                ),
                child,
            )
            .into_widget()
        })
        .into_widget();
        Focus::new(content)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _, event| {
                if matches!(event, KeyEvent::Down(_))
                    && event.logical_key() == LogicalKeyboardKey::ARROW_LEFT
                    && HardwareKeyboard::instance(app).is_alt_pressed(app)
                    && self.is_pane_open(app)
                    && app.get(self).display_mode != NavigationViewDisplayMode::Expanded
                {
                    let mut args = SplitViewPaneClosingEventArgs::default();
                    if let Some(callback) = self.widget(app).pane_closing.clone() {
                        callback(app, &mut args);
                    }
                    if !args.cancel || app.get(self).was_force_closed {
                        app.get_mut(self).block_next_closing = true;
                        self.set_pane_open(app, false, false);
                        return KeyEventResult::Handled;
                    }
                }
                KeyEventResult::Ignored
            }))
            .into_widget()
    }
}

/// RootGrid discrete button states shared by the navigation toggle and back styles.
fn navigation_button_template(
    app: &mut App,
    context: BuildContext,
    states: ControlStates,
    content: WidgetRef,
    focus_margin: [f64; 4],
) -> WidgetRef {
    let resources = ThemeResources::of(app, context);
    let r = &resources;
    let (background, foreground) = match states.common {
        CommonState::PointerOver => (
            r.navigation_view_button_background_pointer_over,
            r.navigation_view_button_foreground_pointer_over,
        ),
        CommonState::Pressed => (
            r.navigation_view_button_background_pressed,
            r.navigation_view_button_foreground_pressed,
        ),
        CommonState::Disabled => (Color::new(0), r.navigation_view_button_foreground_disabled),
        _ => (Color::new(0), r.navigation_view_item_foreground),
    };
    let content = Padding::new(EdgeInsetsGeometry::only(4.0, 2.0, 4.0, 2.0))
        .child(
            SizedBox::new()
                .width(r.pane_toggle_button_width)
                .height(r.pane_toggle_button_height)
                .child(
                    ControlBorder::new(Brush::Solid(background), Brush::Solid(Color::new(0)))
                        .corner_radius(r.control_corner_radius[0])
                        .child(Center::new().child(DefaultTextStyle::new(
                            control_text_style(&resources, 16.0, FontWeight::NORMAL, foreground),
                            content,
                        ))),
                ),
        )
        .into_widget();
    FocusVisual::new(content, resources.theme)
        .visible(states.focused)
        .margin(focus_margin)
        .corner_radius(r.control_corner_radius[0])
        .into_widget()
}

impl NavigationView {
    /// Sets `pane_display_mode`.
    pub fn pane_display_mode(mut self, value: NavigationViewPaneDisplayMode) -> Self {
        self.pane_display_mode = value;
        self
    }

    /// Sets `is_pane_visible`.
    pub fn is_pane_visible(mut self, value: bool) -> Self {
        self.is_pane_visible = value;
        self
    }

    /// Sets `is_pane_toggle_button_visible`.
    pub fn is_pane_toggle_button_visible(mut self, value: bool) -> Self {
        self.is_pane_toggle_button_visible = value;
        self
    }

    /// Sets `is_settings_visible`.
    pub fn is_settings_visible(mut self, value: bool) -> Self {
        self.is_settings_visible = value;
        self
    }

    /// Sets `is_back_button_visible`.
    pub fn is_back_button_visible(mut self, value: NavigationViewBackButtonVisible) -> Self {
        self.is_back_button_visible = value;
        self
    }

    /// Sets `is_back_enabled`.
    pub fn is_back_enabled(mut self, value: bool) -> Self {
        self.is_back_enabled = value;
        self
    }

    /// Sets `always_show_header`.
    pub fn always_show_header(mut self, value: bool) -> Self {
        self.always_show_header = value;
        self
    }

    /// Sets `selection_follows_focus`.
    pub fn selection_follows_focus(mut self, value: NavigationViewSelectionFollowsFocus) -> Self {
        self.selection_follows_focus = value;
        self
    }

    /// Sets `compact_mode_threshold_width`, coercing negative values to zero like the source property.
    pub fn compact_mode_threshold_width(mut self, value: f64) -> Self {
        assert!(value.is_finite());
        self.compact_mode_threshold_width = value.max(0.0);
        self
    }

    /// Sets `expanded_mode_threshold_width`, coercing negative values to zero like the source property.
    pub fn expanded_mode_threshold_width(mut self, value: f64) -> Self {
        assert!(value.is_finite());
        self.expanded_mode_threshold_width = value.max(0.0);
        self
    }

    /// Sets `compact_pane_length`, coercing negative values to zero like the source property.
    pub fn compact_pane_length(mut self, value: f64) -> Self {
        assert!(value.is_finite());
        self.compact_pane_length = value.max(0.0);
        self
    }

    /// Sets `open_pane_length`, coercing negative values to zero like the source property.
    pub fn open_pane_length(mut self, value: f64) -> Self {
        assert!(value.is_finite());
        self.open_pane_length = value.max(0.0);
        self
    }

    /// Supplies the source `header` content slot.
    pub fn header<K>(mut self, child: impl IntoWidget<K>) -> Self {
        self.header = Some(child.into_widget());
        self
    }

    /// Supplies the source `pane_header` content slot.
    pub fn pane_header<K>(mut self, child: impl IntoWidget<K>) -> Self {
        self.pane_header = Some(child.into_widget());
        self
    }

    /// Supplies the source `pane_custom_content` content slot.
    pub fn pane_custom_content<K>(mut self, child: impl IntoWidget<K>) -> Self {
        self.pane_custom_content = Some(child.into_widget());
        self
    }

    /// Supplies the source `pane_footer` content slot.
    pub fn pane_footer<K>(mut self, child: impl IntoWidget<K>) -> Self {
        self.pane_footer = Some(child.into_widget());
        self
    }

    /// Supplies the source `content_overlay` content slot.
    pub fn content_overlay<K>(mut self, child: impl IntoWidget<K>) -> Self {
        self.content_overlay = Some(child.into_widget());
        self
    }

    /// Supplies the source `auto_suggest_box` content slot.
    pub fn auto_suggest_box<K>(mut self, child: impl IntoWidget<K>) -> Self {
        self.auto_suggest_box = Some(child.into_widget());
        self
    }

    /// Sets the pane title shown beside the toggle or before top items.
    pub fn pane_title(mut self, title: impl Into<String>) -> Self {
        self.pane_title = title.into();
        self
    }

    /// Supplies footer entries before the built-in settings item.
    pub fn footer_menu_items(
        mut self,
        items: impl IntoIterator<Item = NavigationViewItem>,
    ) -> Self {
        self.footer_menu_items = items.into_iter().collect();
        self
    }

    /// Gives the owner control of pane openness while retaining adaptive change requests.
    pub fn is_pane_open(mut self, open: bool, changed: impl Fn(&mut App, bool) + 'static) -> Self {
        self.is_pane_open = Some(open);
        self.pane_open_changed = Some(Rc::new(changed));
        self
    }

    /// Sets the widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
    /// Receives the source `item_invoked` event.
    pub fn item_invoked(
        mut self,
        callback: impl Fn(&mut App, NavigationViewItemEventArgs) + 'static,
    ) -> Self {
        self.item_invoked = Some(Rc::new(callback));
        self
    }

    /// Receives the source `expanding` event.
    pub fn expanding(
        mut self,
        callback: impl Fn(&mut App, NavigationViewItemEventArgs) + 'static,
    ) -> Self {
        self.expanding = Some(Rc::new(callback));
        self
    }

    /// Receives the source `collapsed` event.
    pub fn collapsed(
        mut self,
        callback: impl Fn(&mut App, NavigationViewItemEventArgs) + 'static,
    ) -> Self {
        self.collapsed = Some(Rc::new(callback));
        self
    }
}
