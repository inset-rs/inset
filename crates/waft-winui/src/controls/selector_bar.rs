//! SelectorBar.cpp, SelectorBarItem.cpp and `SelectorBar.xaml`: a row of a few views' names, one
//! of them selected. The template holds an `ItemsView` in a horizontal `StackLayout`, whose
//! single selection, keys and focus the bar takes, and each `SelectorBarItem` is an
//! `ItemContainer` in the item style's template. The bar keeps `SelectedItem` and the view's
//! selection in step both ways, as `UpdateItemsViewSelectionFromSelectedItem` and
//! `UpdateSelectedItemFromItemsView` do.

use crate::*;
use std::{fmt, rc::Rc, time::Duration};
use waft_animation::{Cubic, Curve};
use waft_embedder::{Color, FontWeight, Radius};
use waft_foundation::{App, Handle, ListenableObject, Listener, ValueKey};
use waft_painting::{
    Alignment, AlignmentGeometry, Axis, BorderRadius, BoxDecoration, EdgeInsetsGeometry,
};
use waft_rendering::CrossAxisAlignment;
use waft_rendering::MainAxisSize;
use waft_scheduler::{FrameCallback, Ticker};
use waft_widgets::*;

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
            selection: None,
            listener: None,
            asked: None,
        }
    }
}

/// The selection of `PART_ItemsView`, and the selection last asked of the application.
pub struct SelectorBarState {
    state: StateData<SelectorBar>,

    /// `PART_ItemsView`'s selection, which follows `SelectedItem` both ways.
    selection: Option<Handle<ItemsViewSelection>>,
    listener: Option<Listener>,

    /// The selection asked for since the application last built the bar, so that a click and the
    /// focus it brings ask once.
    asked: Option<Option<usize>>,
}

impl SelectorBarState {
    fn items_view_selection(self: Handle<Self>, app: &App) -> Handle<ItemsViewSelection> {
        app.get(self).selection.expect("made in `init_state`")
    }

    /// The selection as the bar last saw or asked for it.
    fn selected_item(self: Handle<Self>, app: &App) -> Option<usize> {
        app.get(self).asked.unwrap_or(self.widget(app).selected)
    }

    /// `UpdateItemsViewSelectionFromSelectedItem`.
    fn update_items_view_selection_from_selected_item(self: Handle<Self>, app: &mut App) {
        let selection = self.items_view_selection(app);
        selection.set_item_count(app, self.widget(app).items.len());
        selection.set_selection_mode(app, ItemsViewSelectionMode::Single);
        match self.selected_item(app) {
            Some(index) => selection.select(app, index),
            None => selection.deselect_all(app),
        }
    }

    /// `UpdateSelectedItemFromItemsView`: asks the application to select what `PART_ItemsView` selected.
    fn update_selected_item_from_items_view(self: Handle<Self>, app: &mut App) {
        let selected = self.items_view_selection(app).selected_index(app);
        if selected == self.selected_item(app) {
            return;
        }
        app.get_mut(self).asked = Some(selected);
        let changed = self.widget(app).selection_changed.clone();
        changed(app, selected);
    }

    /// `SelectorBar::OnGotFocus`: focus reaching a bar with nothing selected selects the current
    /// item, else the first enabled one.
    fn got_focus(self: Handle<Self>, app: &mut App) {
        if self.selected_item(app).is_some() {
            return;
        }
        let selection = self.items_view_selection(app);
        let first_enabled = self
            .widget(app)
            .items
            .iter()
            .position(|item| item.is_enabled);
        if let Some(index) = selection.current_item_index(app).or(first_enabled) {
            selection.select(app, index);
        }
    }

    /// `SelectorBarItem`: an `ItemContainer` in `DefaultSelectorBarItemStyle`.
    fn item_container(item: &SelectorBarItem) -> ItemContainer {
        let shown = item.clone();
        ItemContainer::new(SizedBox::shrink())
            .is_enabled(item.is_enabled)
            .key(Rc::new(ValueKey::new(item.id.clone())))
            .template(Rc::new(move |_, _, states| {
                SelectorBarItemVisual {
                    item: shown.clone(),
                    states,
                }
                .into_widget()
            }))
    }
}

impl State for SelectorBarState {
    type Widget = SelectorBar;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let selection = ItemsViewSelection::new(app);
        let listener = Listener::new(move |app| self.update_selected_item_from_items_view(app));
        app.get_mut(self).selection = Some(selection);
        self.update_items_view_selection_from_selected_item(app);
        selection.add_listener(app, listener.clone());
        app.get_mut(self).listener = Some(listener);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &SelectorBar) {
        app.get_mut(self).asked = None;
        self.update_items_view_selection_from_selected_item(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let selection = self.items_view_selection(app);
        if let Some(listener) = app.get_mut(self).listener.take() {
            selection.remove_listener(app, &listener);
        }
        selection.dispose(app);
        app.destroy(selection);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let items = widget.items.clone();
        // `PART_ItemsView`: the items in a horizontal `StackLayout`, at the bar's `Padding`, in a
        // scroll view that scrolls sideways.
        let items_view = ItemsView::new(items.len(), move |_, _, index| {
            Self::item_container(&items[index])
        })
        .layout(StackLayout {
            orientation: Axis::Horizontal,
            spacing: 0.0,
        })
        .selection_mode(ItemsViewSelectionMode::Single)
        .selection(self.items_view_selection(app))
        .padding(widget.padding.unwrap_or(resources.selector_bar_padding));
        let bar = Focus::new(items_view)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_focus_change(move |app, has_focus| {
                if has_focus {
                    self.got_focus(app);
                }
            });
        // `HorizontalAlignment` `Left` and `VerticalAlignment` `Top`.
        Align::new()
            .alignment(AlignmentGeometry::TOP_LEFT)
            .width_factor(1.0)
            .height_factor(1.0)
            .child(bar)
            .into_widget()
    }
}

/// `DefaultSelectorBarItemStyle`'s template for an item in its container's states, with the
/// pill under the selected item.
#[derive(Clone)]
struct SelectorBarItemVisual {
    item: SelectorBarItem,
    states: ItemContainerStates,
}

impl fmt::Debug for SelectorBarItemVisual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SelectorBarItemVisual")
            .field("item", &self.item)
            .field("states", &self.states)
            .finish()
    }
}

impl StatefulWidget for SelectorBarItemVisual {
    type State = SelectorBarItemVisualState;

    fn create_state(&self) -> Self::State {
        SelectorBarItemVisualState {
            state: StateData::new(),
            single_ticker_provider: Default::default(),
            ticker: None,
            selected_for: Duration::ZERO,
        }
    }
}

/// The clock of the selected states' storyboard, which grows the pill in.
struct SelectorBarItemVisualState {
    state: StateData<SelectorBarItemVisual>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    ticker: Option<Handle<Ticker>>,

    /// How long the item has been selected, up to the storyboard's end.
    selected_for: Duration,
}

impl SingleTickerProviderStateMixin for SelectorBarItemVisualState {
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

impl SelectorBarItemVisualState {
    /// Starts the selected states' storyboard when the item is selected, and on leaving them
    /// returns the pill to its base values at once, as stopping a storyboard does.
    fn sync_storyboard(self: Handle<Self>, app: &mut App) {
        let ticker = app.get(self).ticker.expect("made in `init_state`");
        if self.widget(app).states.is_selected {
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
        if !self.widget(app).states.is_selected {
            return 0.0;
        }
        let progress = (app.get(self).selected_for.as_secs_f64()
            / r.combo_box_item_scale_animation_duration.as_secs_f64())
        .min(1.0);
        Cubic::new(0.0, 0.0, 0.0, 1.0).transform(progress)
    }
}

impl State for SelectorBarItemVisualState {
    type Widget = SelectorBarItemVisual;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let ticker = SingleTickerProviderStateMixin::create_ticker(
            self,
            app,
            FrameCallback::new(move |app, elapsed| self.tick(app, elapsed)),
        );
        app.get_mut(self).ticker = Some(ticker);
        self.sync_storyboard(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &SelectorBarItemVisual) {
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
        let pill = self.pill_progress(&resources, app);
        let widget = self.widget(app);
        item_template(&resources, &widget.item, widget.states, pill)
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
    states: ItemContainerStates,
    pill: f64,
) -> WidgetRef {
    let (background, foreground, pill_fill) = item_colors(r, states.is_selected, states.common);
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
