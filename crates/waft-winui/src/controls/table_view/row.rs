//! `TableViewRow` (`controls/dev/TableView/TableViewRow.cpp` and its template in `TableView.xaml`): one row of a `TableView`, its cells in a `TableViewCellsPanel`, its background in the `CommonStates` group, the accent selection indicator at its leading edge, and the press that selects it on release.
//!
//! A row listens to its table's selection for its own index and rebuilds only when its own selected state changes; the table registers each built row by index so keyboard navigation can focus it. A later editor who has the table rebuild every row on a selection change instead must keep that listener, or a row shows the selection it was built with.

use super::selection::TableViewSelection;
use crate::FocusState;
use crate::*;
use std::fmt;
use std::rc::Rc;
use std::time::Duration;
use waft_embedder::{Color, PointerDeviceKind, Radius};
use waft_foundation::{App, Handle, ListenableObject, Listener, Ref};
use waft_gestures::TapDownDetails;
use waft_painting::{AlignmentGeometry, BorderRadius, EdgeInsetsGeometry};
use waft_rendering::HitTestBehavior;
use waft_widgets::*;

/// A row's index, told to its table.
pub(crate) type TableViewRowHandler = Rc<dyn Fn(&mut App, usize)>;

/// A row's index and the node that takes its focus, told to its table.
pub(crate) type TableViewRowRegistration = Rc<dyn Fn(&mut App, usize, Handle<FocusNode>)>;

/// What a row tells its table.
pub(crate) struct TableViewRowEvents {
    /// `OnRowPointerSelect`: the press that started on the row was released on it.
    pub(crate) pointer_select: TableViewRowHandler,
    /// The row was built, with the node that takes its focus.
    pub(crate) register: TableViewRowRegistration,
    /// The row for `index` went, its node with it.
    pub(crate) unregister: TableViewRowRegistration,
}

/// XAML `TableViewRow`.
#[derive(Clone)]
pub(crate) struct TableViewRow {
    /// The row's index in the view.
    pub(crate) index: usize,
    /// `PART_CellsHost` with the row's cells.
    pub(crate) cells: WidgetRef,
    /// `Background`: `RowBackground` or `AlternatingRowBackground`, else the style's.
    pub(crate) background: Brush,
    /// `BorderThickness` `0,0,0,1` when horizontal grid lines show.
    pub(crate) horizontal_lines: bool,
    /// `MinHeight`: the density's row height.
    pub(crate) min_height: f64,
    /// The table's selection, if it was given one.
    pub(crate) selection: Option<Ref<TableViewSelection>>,
    pub(crate) events: Rc<TableViewRowEvents>,
}

impl fmt::Debug for TableViewRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TableViewRow")
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for TableViewRow {
    type State = TableViewRowState;

    fn create_state(&self) -> TableViewRowState {
        TableViewRowState {
            state: StateData::new(),
            focus_node: None,
            registered: None,
            is_pointer_over: false,
            is_pressed: false,
            focused: false,
            is_selected: false,
            listening: None,
        }
    }
}

pub(crate) struct TableViewRowState {
    state: StateData<TableViewRow>,
    focus_node: Option<Ref<FocusNode>>,
    /// The index the row is registered under.
    registered: Option<usize>,
    /// `m_isPointerOver`.
    is_pointer_over: bool,
    /// `m_isPressed`.
    is_pressed: bool,
    focused: bool,
    /// `IsSelected`, as last read from the selection.
    is_selected: bool,
    listening: Option<(Ref<TableViewSelection>, Listener)>,
}

/// The `CommonStates` a row can be in: `UpdateVisualState`, selected first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowState {
    Normal,
    PointerOver,
    Pressed,
    Selected,
    SelectedPointerOver,
    SelectedPressed,
}

impl TableViewRowState {
    fn focus_node(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self)
            .focus_node
            .as_ref()
            .map(|node| node.handle())
            .expect("made in `init_state`")
    }

    fn read_selected(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget
            .selection
            .as_ref()
            .is_some_and(|selection| selection.is_selected(app, widget.index))
    }

    /// `RefreshRowSelectionState`: the row restamps itself when its own state changed.
    fn selection_changed(self: Handle<Self>, app: &mut App) {
        if !app.contains(self) || !self.mounted(app) {
            return;
        }
        let selected = self.read_selected(app);
        if selected != app.get(self).is_selected {
            self.set_state(app, |state| state.is_selected = selected);
        }
    }

    fn sync(self: Handle<Self>, app: &mut App) {
        let selection = self.widget(app).selection.clone();
        let listened = app
            .get(self)
            .listening
            .as_ref()
            .map(|(held, _)| held.handle());
        if listened != selection.as_ref().map(|selection| selection.handle()) {
            self.stop_listening(app);
            if let Some(selection) = selection {
                let listener = Listener::new(move |app| self.selection_changed(app));
                selection.add_listener(app, listener.clone());
                app.get_mut(self).listening = Some((selection, listener));
            }
        }
        let selected = self.read_selected(app);
        app.get_mut(self).is_selected = selected;
        let index = self.widget(app).index;
        if app.get(self).registered != Some(index) {
            self.unregister(app);
            let events = self.widget(app).events.clone();
            (events.register)(app, index, self.focus_node(app));
            app.get_mut(self).registered = Some(index);
        }
    }

    fn unregister(self: Handle<Self>, app: &mut App) {
        if let Some(index) = app.get_mut(self).registered.take() {
            let events = self.widget(app).events.clone();
            (events.unregister)(app, index, self.focus_node(app));
        }
    }

    fn stop_listening(self: Handle<Self>, app: &mut App) {
        if let Some((selection, listener)) = app.get_mut(self).listening.take()
            && app.contains(selection.handle())
        {
            selection.remove_listener(app, &listener);
        }
    }

    /// `OnPointerPressed`: the primary button or a touch presses the row and gives it the focus; the selection waits for the release.
    fn pointer_pressed(self: Handle<Self>, app: &mut App, details: &TapDownDetails) {
        let touch = !matches!(details.kind, Some(PointerDeviceKind::Mouse) | None);
        self.set_state(app, |state| {
            state.is_pressed = true;
            if touch {
                state.is_pointer_over = true;
            }
        });
        let node = self.focus_node(app);
        FocusState::name(app, node.as_node(), FocusState::Pointer);
        node.request_focus(app, None);
    }

    /// `OnPointerReleased`: a press released on the row selects it.
    fn pointer_released(self: Handle<Self>, app: &mut App) {
        if !app.get(self).is_pressed {
            return;
        }
        self.set_state(app, |state| state.is_pressed = false);
        let widget = self.widget(app).clone();
        (widget.events.pointer_select)(app, widget.index);
    }

    /// `OnPointerCaptureLost` and `OnPointerCanceled`: a pan took the pointer, so it was no tap.
    fn pointer_canceled(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| state.is_pressed = false);
    }

    fn row_state(self: Handle<Self>, app: &App) -> RowState {
        let state = app.get(self);
        match (state.is_selected, state.is_pressed, state.is_pointer_over) {
            (true, true, _) => RowState::SelectedPressed,
            (true, false, true) => RowState::SelectedPointerOver,
            (true, false, false) => RowState::Selected,
            (false, true, _) => RowState::Pressed,
            (false, false, true) => RowState::PointerOver,
            (false, false, false) => RowState::Normal,
        }
    }
}

impl State for TableViewRowState {
    type Widget = TableViewRow;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        InputDevice::initialize(app);
        app.get_mut(self).focus_node = Some(FocusNode::new(app));
        self.sync(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &TableViewRow) {
        self.sync(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.stop_listening(app);
        self.unregister(app);
        if let Some(node) = app.get_mut(self).focus_node.take() {
            node.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let row_state = self.row_state(app);
        // `CommonStates`: PART_RootBorder.Background, PART_CellForegroundPresenter.Foreground and PART_SelectionIndicator.Opacity.
        let (background, foreground, selected) = match row_state {
            RowState::Normal => (
                widget.background,
                r.tabular_surface_row_foreground_brush,
                false,
            ),
            RowState::PointerOver => (
                Brush::Solid(r.tabular_surface_row_background_pointer_over_brush),
                r.tabular_surface_row_foreground_brush,
                false,
            ),
            RowState::Pressed => (
                Brush::Solid(r.tabular_surface_row_background_pressed_brush),
                r.tabular_surface_row_foreground_brush,
                false,
            ),
            RowState::Selected => (
                Brush::Solid(r.tabular_surface_row_background_selected_brush),
                r.tabular_surface_row_foreground_selected_brush,
                true,
            ),
            RowState::SelectedPointerOver => (
                Brush::Solid(r.tabular_surface_row_background_selected_pointer_over_brush),
                r.tabular_surface_row_foreground_selected_pointer_over_brush,
                true,
            ),
            RowState::SelectedPressed => (
                Brush::Solid(r.tabular_surface_row_background_selected_pressed_brush),
                r.tabular_surface_row_foreground_selected_pressed_brush,
                true,
            ),
        };
        // PART_CellForegroundPresenter > PART_CellsHost, `HorizontalAlignment="Left"`.
        let cells = DefaultTextStyle::new(
            waft_painting::TextStyle::new().color(foreground),
            Align::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .height_factor(1.0)
                .child(widget.cells.clone()),
        );
        // PART_SelectionIndicator: 3 by 16, radius 1.5, 2 from the leading edge, its opacity
        // animated to 1 over 83 ms by the Selected states and back at once.
        let indicator = IgnorePointer::new().child(
            Align::new()
                .alignment(AlignmentGeometry::CENTER_LEFT)
                .child(
                    Padding::new(EdgeInsetsGeometry::from_ltrb(2.0, 0.0, 0.0, 0.0)).child(
                        AnimatedOpacity::new(
                            if selected { 1.0 } else { 0.0 },
                            if selected {
                                Duration::from_millis(83)
                            } else {
                                Duration::ZERO
                            },
                        )
                        .child(
                            SizedBox::new()
                                .width(3.0)
                                .height(16.0)
                                .child(DecoratedBox::new(
                                    BoxDecoration::new()
                                        .color(r.tabular_surface_selection_indicator_brush)
                                        .border_radius(BorderRadius::all(Radius::circular(1.5))),
                                )),
                        ),
                    ),
                ),
        );
        let content = Stack::new().children([
            cells.into_widget(),
            Positioned::fill(indicator).into_widget(),
        ]);
        // PART_RootBorder
        let root = ControlBorder::new(background, Brush::Solid(r.tabular_surface_grid_line_brush))
            .border_thickness_ltrb(if widget.horizontal_lines {
                [0.0, 0.0, 0.0, 1.0]
            } else {
                [0.0; 4]
            })
            .child(content);
        let root =
            ConstrainedBox::new(BoxConstraints::new().min_height(widget.min_height)).child(root);
        let focused = app.get(self).focused && FocusState::current(app).shows_focus_visual();
        let visual = FocusVisual::new(root, r.theme)
            .visible(focused)
            .corner_radius(0.0);
        let gesture = GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            .on_tap_down(Rc::new(move |app, details| {
                self.pointer_pressed(app, &details)
            }))
            .on_tap_up(Rc::new(move |app, _| self.pointer_released(app)))
            .on_tap_cancel(Listener::new(move |app| self.pointer_canceled(app)))
            .child(visual);
        let mouse = MouseRegion::new()
            .on_enter(Rc::new(move |app, _| {
                self.set_state(app, |state| state.is_pointer_over = true)
            }))
            .on_exit(Rc::new(move |app, _| {
                self.set_state(app, |state| {
                    state.is_pointer_over = false;
                    state.is_pressed = false;
                })
            }))
            .child(gesture);
        Focus::new(mouse)
            .focus_node(
                app.get(self)
                    .focus_node
                    .as_ref()
                    .expect("made in `init_state`")
                    .erase(),
            )
            .on_focus_change(move |app, focused| {
                self.set_state(app, |state| state.focused = focused)
            })
            .into_widget()
    }
}

/// The background a row's `Normal` state shows: `RefreshRowBackground`. `RowBackground` fills every row and `AlternatingRowBackground` the odd ones; with neither, the style's.
pub(crate) fn row_background(
    r: &ThemeResources,
    index: usize,
    row_background: Option<Brush>,
    alternating: Option<Brush>,
) -> Brush {
    let base = row_background.unwrap_or(Brush::Solid(r.tabular_surface_row_background_brush));
    match alternating {
        Some(alternating) if index % 2 != 0 => alternating,
        _ => base,
    }
}

/// A cell wrapper: `cellWrapper`, a transparent `Border` so its whole rectangle takes presses, with the vertical grid line on its trailing edge (`RefreshGridLines`) and the row's height at least.
pub(crate) fn cell_wrapper(
    r: &ThemeResources,
    vertical_lines: bool,
    min_height: f64,
    child: WidgetRef,
) -> WidgetRef {
    let border = ControlBorder::new(
        Brush::Solid(Color::new(0)),
        Brush::Solid(r.tabular_surface_grid_line_brush),
    )
    .border_thickness_ltrb(if vertical_lines {
        [0.0, 0.0, 1.0, 0.0]
    } else {
        [0.0; 4]
    })
    .child(child);
    ConstrainedBox::new(BoxConstraints::new().min_height(min_height))
        .child(border)
        .into_widget()
}
