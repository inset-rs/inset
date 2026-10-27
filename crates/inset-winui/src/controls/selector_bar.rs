//! SelectorBar.cpp, SelectorBarItem.cpp and `SelectorBar.xaml`: a row of a few views' names, one
//! of them selected, with the keys and single selection of the `ItemsView` its template holds.

use crate::*;
use inset_animation::{Cubic, Curve};
use inset_embedder::{Color, FontWeight, Radius, TextDirection};
use inset_foundation::{App, Handle, Listener};
use inset_gestures::K_PRIMARY_BUTTON;
use inset_painting::{
    Alignment, AlignmentGeometry, Axis, BorderRadius, BoxDecoration, EdgeInsetsGeometry,
};
use inset_rendering::CrossAxisAlignment;
use inset_rendering::MainAxisSize;
use inset_scheduler::{FrameCallback, Ticker};
use inset_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey};
use inset_widgets::Listener as PointerListener;
use inset_widgets::*;
use std::{collections::HashMap, fmt, rc::Rc, time::Duration};

/// One item of a `SelectorBar`: a XAML `SelectorBarItem`.
#[derive(Clone)]
pub struct SelectorBarItem {
    /// Identity across rebuilds and moves; unique within the bar.
    pub id: String,

    /// `Text`; an empty text shows none.
    pub text: String,

    /// `Icon`, drawn at `SelectorBarItemIconScale` in the item's foreground.
    pub icon: Option<WidgetRef>,

    /// `IsEnabled`: a disabled item is drawn disabled and takes no input or focus.
    pub is_enabled: bool,
}

impl SelectorBarItem {
    /// Creates an item named `text`.
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            icon: None,
            is_enabled: true,
        }
    }

    /// Sets `Icon`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> Self {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Sets IsEnabled.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }
}

impl fmt::Debug for SelectorBarItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectorBarItem")
            .field("id", &self.id)
            .field("text", &self.text)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

/// Receives the selection a person asks for: an item's index, or none.
pub type SelectorBarSelectionChanged = Rc<dyn Fn(&mut App, Option<usize>)>;

/// A XAML `SelectorBar`: a few views' names in a row, one of them selected, the pill under it.
/// The application owns the selection: `selection_changed` asks it to select an item, which it
/// passes back as `selected`.
#[derive(Clone)]
pub struct SelectorBar {
    /// `Items`.
    pub items: Vec<SelectorBarItem>,

    /// `SelectedItem`, by its index in `items`.
    pub selected: Option<usize>,

    /// `SelectionChanged`.
    pub selection_changed: SelectorBarSelectionChanged,

    /// `Padding`, around the items; the theme's `SelectorBarPadding` when `None`.
    pub padding: Option<[f64; 4]>,
}

impl SelectorBar {
    /// Creates a bar of `items` showing `selected`. Panics when `selected` is not an item's
    /// index, as setting a `SelectedItem` outside `Items` throws.
    pub fn new(
        items: Vec<SelectorBarItem>,
        selected: Option<usize>,
        selection_changed: impl Fn(&mut App, Option<usize>) + 'static,
    ) -> Self {
        assert!(
            selected.is_none_or(|index| index < items.len()),
            "SelectedItem must be an element of Items."
        );
        Self {
            items,
            selected,
            selection_changed: Rc::new(selection_changed),
            padding: None,
        }
    }

    /// Sets Padding.
    pub fn padding(mut self, value: [f64; 4]) -> Self {
        self.padding = Some(value);
        self
    }
}

impl fmt::Debug for SelectorBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectorBar")
            .field("items", &self.items)
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SelectorBar {
    type State = SelectorBarState;

    fn create_state(&self) -> Self::State {
        SelectorBarState {
            state: StateData::new(),
            focus_nodes: HashMap::new(),
            scroll: None,
            asked: None,
        }
    }
}

/// The items' focus, the row's scrolling, and the selection last asked for.
pub struct SelectorBarState {
    state: StateData<SelectorBar>,

    /// Each item's focus node by id, so that focus survives rebuilds and the arrow keys can move
    /// it.
    focus_nodes: HashMap<String, Handle<FocusNode>>,

    /// `ItemsView`'s scroll view, which scrolls the row sideways when the bar is narrower.
    scroll: Option<Handle<ScrollViewportController>>,

    /// The selection asked for since the application last built the bar, so that a click and the
    /// focus it brings ask once.
    asked: Option<Option<usize>>,
}

impl SelectorBarState {
    /// Asks the application to select `index`, unless that is already selected or asked for.
    fn select(self: Handle<Self>, app: &mut App, index: Option<usize>) {
        let current = app.get(self).asked.unwrap_or(self.widget(app).selected);
        if current == index {
            return;
        }
        app.get_mut(self).asked = Some(index);
        let changed = self.widget(app).selection_changed.clone();
        changed(app, index);
    }

    /// The selection as the bar last saw or asked for it.
    fn selection(self: Handle<Self>, app: &App) -> Option<usize> {
        app.get(self).asked.unwrap_or(self.widget(app).selected)
    }

    /// `ItemContainer::OnPointerPressed`: a press focuses the item, before its click.
    fn pressed(self: Handle<Self>, app: &mut App, index: usize) {
        if let Some(node) = self.focus_node(app, index) {
            node.request_focus(app, None);
        }
    }

    /// `SingleSelector::OnInteractedAction`: a click selects the item, and with Control held
    /// deselects it when it is selected.
    fn clicked(self: Handle<Self>, app: &mut App, index: usize) {
        let control = HardwareKeyboard::instance(app).is_control_pressed(app);
        if control && self.selection(app) == Some(index) {
            self.select(app, None);
        } else {
            self.select(app, Some(index));
        }
    }

    /// `SingleSelector::OnFocusedAction` and `SelectorBar::OnGotFocus`: selection follows the
    /// focus unless Control is held, and focus reaching a bar with nothing selected selects the
    /// focused item.
    fn focused(self: Handle<Self>, app: &mut App, index: usize) {
        let control = HardwareKeyboard::instance(app).is_control_pressed(app);
        if !control || self.selection(app).is_none() {
            self.select(app, Some(index));
        }
    }

    /// `ItemsView`'s Left, Right, Home and End: the focus moves to the next enabled item that way,
    /// staying at the last one, and selection follows it.
    fn key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if matches!(event, KeyEvent::Up(_)) {
            return KeyEventResult::Ignored;
        }
        let Some(from) = self.focused_index(app) else {
            return KeyEventResult::Ignored;
        };
        let rtl = Directionality::of(app, self.context(app)) == TextDirection::Rtl;
        let enabled: Vec<usize> = self
            .widget(app)
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.is_enabled)
            .map(|(index, _)| index)
            .collect();
        let previous = enabled.iter().rev().find(|index| **index < from).copied();
        let next = enabled.iter().find(|index| **index > from).copied();
        let target = match event.logical_key() {
            LogicalKeyboardKey::ARROW_LEFT if rtl => next,
            LogicalKeyboardKey::ARROW_LEFT => previous,
            LogicalKeyboardKey::ARROW_RIGHT if rtl => previous,
            LogicalKeyboardKey::ARROW_RIGHT => next,
            LogicalKeyboardKey::HOME => enabled.first().copied(),
            LogicalKeyboardKey::END => enabled.last().copied(),
            _ => return KeyEventResult::Ignored,
        };
        if let Some(node) = target.and_then(|index| self.focus_node(app, index)) {
            node.request_focus(app, None);
        }
        KeyEventResult::Handled
    }

    /// The index of the item with the focus.
    fn focused_index(self: Handle<Self>, app: &mut App) -> Option<usize> {
        let nodes: Vec<Option<Handle<FocusNode>>> = self
            .widget(app)
            .items
            .iter()
            .map(|item| app.get(self).focus_nodes.get(&item.id).copied())
            .collect();
        nodes
            .into_iter()
            .position(|node| node.is_some_and(|node| node.has_focus(app)))
    }

    fn focus_node(self: Handle<Self>, app: &App, index: usize) -> Option<Handle<FocusNode>> {
        let id = &self.widget(app).items.get(index)?.id;
        app.get(self).focus_nodes.get(id).copied()
    }

    /// Makes a focus node for each new item and disposes those of items gone.
    fn sync_focus_nodes(self: Handle<Self>, app: &mut App) {
        let ids: Vec<String> = self
            .widget(app)
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect();
        let gone: Vec<String> = app
            .get(self)
            .focus_nodes
            .keys()
            .filter(|id| !ids.contains(id))
            .cloned()
            .collect();
        for id in gone {
            if let Some(node) = app.get_mut(self).focus_nodes.remove(&id) {
                node.dispose(app);
            }
        }
        for id in ids {
            if !app.get(self).focus_nodes.contains_key(&id) {
                let node = FocusNode::new(app);
                app.get_mut(self).focus_nodes.insert(id, node);
            }
        }
    }

    /// The item Tab stops at, as `TabNavigation` `Once` lets it: the selected one, else the first
    /// enabled.
    fn tab_stop(self: Handle<Self>, app: &App) -> Option<usize> {
        let widget = self.widget(app);
        widget
            .selected
            .filter(|index| widget.items[*index].is_enabled)
            .or_else(|| widget.items.iter().position(|item| item.is_enabled))
    }
}

impl State for SelectorBarState {
    type Widget = SelectorBar;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let scroll = ScrollViewportController::new(app);
        app.get_mut(self).scroll = Some(scroll);
        self.sync_focus_nodes(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &SelectorBar) {
        app.get_mut(self).asked = None;
        self.sync_focus_nodes(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let nodes: Vec<_> = app
            .get_mut(self)
            .focus_nodes
            .drain()
            .map(|(_, node)| node)
            .collect();
        for node in nodes {
            node.dispose(app);
        }
        if let Some(scroll) = app.get_mut(self).scroll.take() {
            scroll.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let tab_stop = self.tab_stop(app);
        let pressed: ItemEvent = Rc::new(move |app, index| self.pressed(app, index));
        let clicked: ItemEvent = Rc::new(move |app, index| self.clicked(app, index));
        let focused: ItemEvent = Rc::new(move |app, index| self.focused(app, index));
        let items = widget.items.iter().enumerate().map(|(index, item)| {
            SelectorBarItemContainer {
                item: item.clone(),
                index,
                selected: widget.selected == Some(index),
                is_tab_stop: tab_stop == Some(index),
                focus_node: app.get(self).focus_nodes[&item.id],
                pressed: pressed.clone(),
                clicked: clicked.clone(),
                focused: focused.clone(),
            }
            .into_widget()
        });
        // `PART_ItemsView`: the items in a horizontal `StackLayout`, at the bar's `Padding`, in a
        // scroll view that scrolls sideways.
        let padding = widget.padding.unwrap_or(resources.selector_bar_padding);
        let row = Padding::new(EdgeInsetsGeometry::from_ltrb(
            padding[0], padding[1], padding[2], padding[3],
        ))
        .child(
            Row::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .children(items.collect::<Vec<_>>()),
        );
        let scroll = app.get(self).scroll.expect("made in `init_state`");
        let items_view = Focus::new(ScrollBarViewport::new(scroll, row).axis(Axis::Horizontal))
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _, event| self.key(app, event)));
        // `HorizontalAlignment` `Left` and `VerticalAlignment` `Top`.
        Align::new()
            .alignment(AlignmentGeometry::TOP_LEFT)
            .width_factor(1.0)
            .height_factor(1.0)
            .child(items_view)
            .into_widget()
    }
}

/// Receives a press, a click or the focus reaching the item at an index.
type ItemEvent = Rc<dyn Fn(&mut App, usize)>;

/// The container the bar makes for an item: `DefaultSelectorBarItemStyle`'s template, with the
/// pill under the selected item.
#[derive(Clone)]
struct SelectorBarItemContainer {
    item: SelectorBarItem,
    index: usize,
    selected: bool,
    /// Whether Tab stops at the item.
    is_tab_stop: bool,
    /// The item's focus, owned by the bar so that the arrow keys can move it.
    focus_node: Handle<FocusNode>,
    pressed: ItemEvent,
    clicked: ItemEvent,
    focused: ItemEvent,
}

impl fmt::Debug for SelectorBarItemContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectorBarItemContainer")
            .field("item", &self.item)
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SelectorBarItemContainer {
    type State = SelectorBarItemContainerState;

    fn create_state(&self) -> Self::State {
        SelectorBarItemContainerState {
            state: StateData::new(),
            single_ticker_provider: Default::default(),
            ticker: None,
            selected_for: Duration::ZERO,
        }
    }
}

/// The clock of the selected states' storyboard, which grows the pill in.
struct SelectorBarItemContainerState {
    state: StateData<SelectorBarItemContainer>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    ticker: Option<Handle<Ticker>>,

    /// How long the item has been selected, up to the storyboard's end.
    selected_for: Duration,
}

impl SingleTickerProviderStateMixin for SelectorBarItemContainerState {
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

impl SelectorBarItemContainerState {
    /// Starts the selected states' storyboard when the item is selected, and on leaving them
    /// returns the pill to its base values at once, as stopping a storyboard does.
    fn sync_storyboard(self: Handle<Self>, app: &mut App) {
        let ticker = app.get(self).ticker.expect("made in `init_state`");
        if self.widget(app).selected {
            if !ticker.is_active(app) && app.get(self).selected_for.is_zero() {
                ticker.start(app);
            }
        } else {
            ticker.stop(app, false);
            app.get_mut(self).selected_for = Duration::ZERO;
        }
    }

    /// One frame of the storyboard.
    fn tick(self: Handle<Self>, app: &mut App, elapsed: Duration) {
        self.set_state(app, |state| state.selected_for = elapsed);
        let duration =
            ThemeResources::of(app, self.context(app)).combo_box_item_scale_animation_duration;
        if elapsed >= duration {
            app.get(self).ticker.unwrap().stop(app, false);
        }
    }

    /// How far the pill has grown in: 0 unselected, then up to 1 along the storyboard's
    /// `0,0,0,1` spline.
    fn pill_progress(self: Handle<Self>, r: &ThemeResources, app: &App) -> f64 {
        if !self.widget(app).selected {
            return 0.0;
        }
        let progress = (app.get(self).selected_for.as_secs_f64()
            / r.combo_box_item_scale_animation_duration.as_secs_f64())
        .min(1.0);
        Cubic::new(0.0, 0.0, 0.0, 1.0).transform(progress)
    }
}

impl State for SelectorBarItemContainerState {
    type Widget = SelectorBarItemContainer;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let ticker = SingleTickerProviderStateMixin::create_ticker(
            self,
            app,
            FrameCallback::new(move |app, elapsed| self.tick(app, elapsed)),
        );
        app.get_mut(self).ticker = Some(ticker);
        self.sync_storyboard(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &SelectorBarItemContainer) {
        self.sync_storyboard(app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).ticker.unwrap().dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let pill = self.pill_progress(&resources, app);
        let (item, index, selected) = (widget.item.clone(), widget.index, widget.selected);
        let clicked = widget.clicked.clone();
        let states = CommonStates::new(
            Listener::new(move |app| clicked(app, index)),
            move |_, _, states| item_template(&resources, &item, selected, pill, states),
        )
        .accepts_return(true)
        .is_enabled(widget.item.is_enabled)
        .is_tab_stop(widget.is_tab_stop)
        .focus_node(widget.focus_node.as_node());
        let (focused, pressed) = (widget.focused.clone(), widget.pressed.clone());
        let enabled = widget.item.is_enabled;
        let states = PointerListener::new()
            .on_pointer_down(Rc::new(move |app, event| {
                if enabled && event.buttons & K_PRIMARY_BUTTON != 0 {
                    pressed(app, index);
                }
            }))
            .child(states);
        Focus::new(states)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_focus_change(move |app, has_focus| {
                if has_focus {
                    focused(app, index);
                }
            })
            .into_widget()
    }
}

/// `SelectorBarItem`'s `CombinedStates` and `DisabledStates`: the item's background and
/// foreground, and the pill's fill.
fn item_colors(r: &ThemeResources, selected: bool, common: CommonState) -> (Color, Color, Color) {
    let (background, foreground) = match (selected, common) {
        (_, CommonState::Disabled) => (
            r.selector_bar_item_background_disabled,
            r.selector_bar_item_foreground_disabled,
        ),
        (false, CommonState::Normal) => (Color::new(0), r.selector_bar_item_foreground),
        (false, CommonState::PointerOver) => (
            r.selector_bar_item_background_pointer_over,
            r.selector_bar_item_foreground_pointer_over,
        ),
        (false, CommonState::Pressed) => (
            r.selector_bar_item_background_pressed,
            r.selector_bar_item_foreground_pressed,
        ),
        (true, CommonState::Normal) => (
            r.selector_bar_item_background_selected,
            r.selector_bar_item_foreground_selected,
        ),
        // `SelectedPointerOver` and `SelectedPressed` set no background, and both take the
        // pointer-over foreground.
        (true, _) => (Color::new(0), r.selector_bar_item_foreground_pointer_over),
    };
    let pill = if common == CommonState::Disabled {
        r.selector_bar_item_disabled_pill_fill
    } else {
        r.selector_bar_item_pill_fill
    };
    (background, foreground, pill)
}

/// The item's template, `PART_ContainerRoot` with its icon and text, `PART_SelectionVisual` and
/// `PART_CommonVisual`, in `states`, the pill grown in by `pill`.
fn item_template(
    r: &ThemeResources,
    item: &SelectorBarItem,
    selected: bool,
    pill: f64,
    states: ControlStates,
) -> WidgetRef {
    let (background, foreground, pill_fill) = item_colors(r, selected, states.common);
    let corner = r.control_corner_radius;
    let mut cells = Vec::new();
    if let Some(content) = item_content(r, item, foreground) {
        cells.push(GridCell::new(content).column(1).into_widget());
    }
    cells.push(
        GridCell::new(selection_visual(r, pill, pill_fill))
            .row(1)
            .column_span(2)
            .into_widget(),
    );
    // `PART_CommonVisual`, over the rest, taking no input.
    cells.push(
        GridCell::new(
            IgnorePointer::new().child(
                ControlBorder::new(
                    Brush::Solid(r.selector_bar_item_background),
                    Brush::Solid(r.selector_bar_item_border_brush),
                )
                .border_thickness(r.selector_bar_selected_inner_thickness[0])
                .corner_radius_corners(corner),
            ),
        )
        .row_span(2)
        .column_span(2)
        .into_widget(),
    );
    let root = Grid::new()
        .row_definitions([RowDefinition::new(GridLength::AUTO); 2])
        .column_definitions([ColumnDefinition::new(GridLength::AUTO); 2])
        .background(Brush::Solid(background))
        .corner_radius_corners(corner)
        .children(cells);
    FocusVisual::new(root, r.theme)
        .visible(states.focused)
        .margin(r.selector_bar_item_focus_visual_margin)
        .corner_radius(corner[0])
        .into_widget()
}

/// The template's `StackPanel` of `PART_IconVisual` and `PART_TextVisual` at the item's
/// `Padding`, each shown only when the item has one, and none when it has neither.
fn item_content(
    r: &ThemeResources,
    item: &SelectorBarItem,
    foreground: Color,
) -> Option<WidgetRef> {
    let mut parts = Vec::new();
    if let Some(icon) = &item.icon {
        let m = r.selector_bar_item_icon_visual_margin;
        parts.push(
            Margin::new(
                m,
                Transform::scale(None, Some(r.selector_bar_item_icon_scale), None)
                    .child(icon.clone()),
            )
            .into_widget(),
        );
    }
    if !item.text.is_empty() {
        let m = r.selector_bar_item_text_visual_margin;
        parts.push(Margin::new(m, Text::new(item.text.clone())).into_widget());
    }
    if parts.is_empty() {
        return None;
    }
    let p = r.selector_bar_item_padding;
    let content = Padding::new(EdgeInsetsGeometry::from_ltrb(p[0], p[1], p[2], p[3])).child(
        Row::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .spacing(r.selector_bar_item_spacing)
            .children(parts),
    );
    let style = control_text_style(
        r,
        r.control_content_theme_font_size,
        FontWeight::NORMAL,
        foreground,
    );
    Some(DefaultTextStyle::new(style, content).into_widget())
}

/// `PART_SelectionVisual`, in `SelectorBarItemPill`: a 4 by 3 pill centred at the bottom, which
/// the selected states fade in and widen fourfold about its centre.
fn selection_visual(r: &ThemeResources, pill: f64, fill: Color) -> WidgetRef {
    let m = r.selector_bar_item_selection_visual_margin;
    let shape = SizedBox::new()
        .width(r.selector_bar_item_pill_width)
        .height(r.selector_bar_item_pill_height)
        .child(DecoratedBox::new(
            BoxDecoration::new()
                .color(fill)
                .border_radius(BorderRadius::all(Radius::elliptical(0.5, 1.0))),
        ));
    let shape = Opacity::new(pill)
        .child(Transform::scale(None, Some(1.0 + 3.0 * pill), Some(1.0)).child(shape));
    Margin::new(
        m,
        Align::new()
            .alignment(Alignment::BOTTOM_CENTER.into())
            .height_factor(1.0)
            .child(shape),
    )
    .into_widget()
}
