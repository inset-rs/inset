//! TreeViewItem.xaml's `MUX_TreeViewItemStyle` and the pointer input of ListViewBaseItem
//! (ListViewBaseItem_Partial.cpp): one row of the tree, its visual states as
//! `VisualStatesHelper::GetValidVisualStatesListViewBaseItem` chooses them for this template, the
//! expand and collapse chevron, and the check box of Multiple mode.
//!
//! Template, outside in: the item (`MinWidth` and `MinHeight`, the system focus visual) holds
//! `ContentPresenterGrid` (`TreeViewItemPresenterMargin`, `TreeViewItemPresenterPadding`, the
//! state's background), which holds `SelectionIndicator` and `MultiSelectGrid`; that grid,
//! padded by the indentation, has three columns: `MultiSelectCheckBox` with
//! `MultiArrangeOverlayTextBorder`, `ExpandCollapseChevron` with `CollapsedGlyph` and
//! `ExpandedGlyph`, and `ContentPresenter`.

use super::TreeViewState;
use super::view_model::NodeSelectionState;
use crate::*;
use inset_embedder::{Color, FontWeight};
use inset_foundation::{App, Handle, Listener};
use inset_gestures::K_PRIMARY_BUTTON;
use inset_painting::{Alignment, EdgeInsetsGeometry};
use inset_rendering::{BoxConstraints, HitTestBehavior};
use inset_widgets::Listener as PointerListener;
use inset_widgets::*;
use std::{fmt, rc::Rc};

/// `TreeViewItem::UpdateIndentation`: the indentation of one level of depth.
pub(super) const INDENTATION_PER_DEPTH: f64 = 16.0;

/// `SelectionIndicator`'s `Width`, `Height` and `RadiusX`.
const SELECTION_INDICATOR_SIZE: (f64, f64) = (3.0, 16.0);
const SELECTION_INDICATOR_RADIUS: f64 = 2.0;

/// `MultiSelectCheckBox`'s `Width` and `Margin`.
const CHECK_BOX_WIDTH: f64 = 32.0;
const CHECK_BOX_MARGIN: [f64; 4] = [10.0, 0.0, 0.0, 0.0];

/// `ExpandCollapseChevron`'s `Padding`, and the one `TreeViewMultiSelectEnabled*` sets.
const CHEVRON_PADDING: [f64; 4] = [14.0, 0.0, 14.0, 0.0];
const MULTI_SELECT_CHEVRON_PADDING: [f64; 4] = [0.0, 0.0, 14.0, 0.0];

/// The glyphs' `Width` and `Height`, `Padding`, and TreeViewItem's default `GlyphSize`.
const GLYPH_BOX: f64 = 12.0;
const GLYPH_PADDING: f64 = 2.0;
const GLYPH_SIZE: f64 = 8.0;

/// `MultiArrangeOverlayTextBorder`'s `MinWidth`, `Height` and `BorderThickness`.
const DRAG_COUNT_MIN_WIDTH: f64 = 20.0;
const DRAG_COUNT_HEIGHT: f64 = 20.0;
const DRAG_COUNT_BORDER_THICKNESS: f64 = 1.0;

/// `MUX_TreeViewItemStyle`'s `FocusVisualMargin`.
const FOCUS_VISUAL_MARGIN: [f64; 4] = [0.0, -1.0, 0.0, -1.0];

/// The height a row of one line of text takes: `ContentPresenterGrid`'s padding around the
/// content's `MinHeight`, within the item's `MinHeight`, and the grid's margin.
pub(super) fn estimated_row_height(r: &ThemeResources) -> f64 {
    let padding = r.tree_view_item_presenter_padding;
    let margin = r.tree_view_item_presenter_margin;
    (padding[1] + padding[3] + r.tree_view_item_content_height).max(r.tree_view_item_min_height)
        + margin[1]
        + margin[3]
}

/// The state of a list in a drag that decides an item's `CommonStates` and `DragStates`:
/// `ListViewBaseItemVisualStatesCriteria`'s drag fields and TreeViewList's own.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct ItemDrag {
    /// `isDragging`: the list is in a drag that started on one of its items.
    pub(super) in_drag: bool,
    /// `isItemDragPrimary`: the drag started on this item.
    pub(super) primary: bool,
    /// `isDragVisualCaptured`: the drag visual has been taken from the item.
    pub(super) captured: bool,
    /// `isDraggedOver`: the pointer of the drag is over the middle of this item.
    pub(super) dragged_over: bool,
    /// `canReorder`: the tree's `CanReorderItems`.
    pub(super) can_reorder: bool,
    /// `dragItemsCount`: the items the drag carries.
    pub(super) drag_items_count: usize,
    /// TreeViewList's `MultipleDraggingPrimary` state, for the item a drag of several selected
    /// nodes started on.
    pub(super) multiple_dragging_primary: bool,
    /// `TreeViewItemTemplateSettings.DragItemsCount`.
    pub(super) drag_items_count_display: usize,
}

/// The focus of a row of the list; the drag preview has none.
#[derive(Clone, Debug)]
pub(super) struct ItemFocus {
    /// The item's focus, which the tree owns.
    pub(super) node: Handle<FocusNode>,
    /// Whether Tab reaches the item: TreeViewList's `TabNavigation="Once"` gives the list one
    /// tab stop.
    pub(super) is_tab_stop: bool,
    /// Whether the item has the focus with the focus visual showing.
    pub(super) shows_focus_visual: bool,
}

/// The part of an item a press went down on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PressedPart {
    /// The item itself.
    Item,
    /// `ExpandCollapseChevron`, whose `PointerPressed` handler handles the press.
    Chevron,
    /// `MultiSelectCheckBox`, a button, which handles the press.
    CheckBox,
}

/// The `CommonStates` an item takes. The template's `Disabled` and `SelectedDisabled` are never
/// shown: `GetValidVisualStatesListViewBaseItem` asks a disabled item for `Disabled`, then in
/// the same group for `Normal` or `Selected`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ItemCommonState {
    Normal,
    PointerOver,
    Pressed,
    Selected,
    PointerOverSelected,
    PressedSelected,
    /// The drag state of the item a reorder drag started on, which this template puts in
    /// `CommonStates`: `MultiSelectGrid` fades out, and the other states' setters are lifted.
    ReorderedPlaceholder,
}

/// `GetValidVisualStatesListViewBaseItem`'s enabled, selection and drag states, as this template
/// names them. `is_selected` is a ListView selection, which only Single mode has.
pub(super) fn common_state(
    is_enabled: bool,
    is_selected: bool,
    is_pressed: bool,
    is_pointer_over: bool,
    drag: ItemDrag,
) -> ItemCommonState {
    use ItemCommonState::*;
    let retains_press = drag.in_drag && drag.primary && !drag.captured;
    let state = if !is_enabled {
        if is_selected { Selected } else { Normal }
    } else if drag.dragged_over {
        if is_selected {
            PointerOverSelected
        } else {
            PointerOver
        }
    } else if is_selected {
        if is_pressed || (!is_pointer_over && retains_press) {
            PressedSelected
        } else if is_pointer_over {
            PointerOverSelected
        } else {
            Selected
        }
    } else if is_pointer_over {
        if is_pressed { Pressed } else { PointerOver }
    } else if is_pressed || retains_press {
        Pressed
    } else {
        Normal
    };
    if drag.in_drag && drag.primary && drag.captured && drag.can_reorder {
        ReorderedPlaceholder
    } else {
        state
    }
}

/// Whether the item is in `DragStates`' `MultipleDraggingPrimary`: TreeViewList's state for a
/// drag of several selected nodes, or the helper's for one that cannot reorder, until its
/// visual is captured.
fn multiple_dragging_primary(drag: ItemDrag) -> bool {
    drag.in_drag
        && drag.primary
        && (drag.multiple_dragging_primary
            || (drag.drag_items_count > 1 && !drag.captured && !drag.can_reorder))
}

/// The values the item's `CommonStates` set.
struct StateBrushes {
    /// `ContentPresenterGrid.Background`.
    background: Color,
    /// `ContentPresenterGrid.BorderBrush`.
    border: Color,
    /// `ContentPresenter.Foreground`.
    foreground: Color,
    /// `CollapsedGlyph.Foreground` and `ExpandedGlyph.Foreground`.
    glyph: Color,
    /// `SelectionIndicator.Fill`.
    indicator: Color,
    /// `SelectionIndicator.Opacity`.
    indicator_opacity: f64,
}

impl StateBrushes {
    /// The setters of `state`, or the template's own values where it sets none.
    fn of(r: &ThemeResources, state: ItemCommonState) -> StateBrushes {
        use ItemCommonState::*;
        match state {
            // The template's values: `Background`, `BorderBrush`, `Foreground` and `GlyphBrush`
            // of the style, and the indicator's own.
            Normal | ReorderedPlaceholder => StateBrushes {
                background: r.tree_view_item_background,
                border: r.tree_view_item_border_brush,
                foreground: r.list_view_item_foreground,
                glyph: r.tree_view_item_foreground,
                indicator: r.tree_view_item_selection_indicator_foreground,
                indicator_opacity: 0.0,
            },
            PointerOver => StateBrushes {
                background: r.tree_view_item_background_pointer_over,
                border: r.tree_view_item_border_brush_pointer_over,
                foreground: r.tree_view_item_foreground_pointer_over,
                glyph: r.tree_view_item_foreground_pointer_over,
                indicator: r.tree_view_item_selection_indicator_foreground_pointer_over,
                indicator_opacity: 0.0,
            },
            Pressed => StateBrushes {
                background: r.tree_view_item_background_pressed,
                border: r.tree_view_item_border_brush_pressed,
                foreground: r.tree_view_item_foreground_pressed,
                glyph: r.tree_view_item_foreground_pressed,
                indicator: r.tree_view_item_selection_indicator_foreground_pressed,
                indicator_opacity: 0.0,
            },
            Selected => StateBrushes {
                background: r.tree_view_item_background_selected,
                border: r.tree_view_item_border_brush_selected,
                foreground: r.tree_view_item_foreground_selected,
                glyph: r.tree_view_item_foreground_selected,
                indicator: r.tree_view_item_selection_indicator_foreground,
                indicator_opacity: 1.0,
            },
            PointerOverSelected => StateBrushes {
                background: r.tree_view_item_background_selected_pointer_over,
                border: r.tree_view_item_border_brush_selected_pointer_over,
                foreground: r.tree_view_item_foreground_selected_pointer_over,
                glyph: r.tree_view_item_foreground_selected_pointer_over,
                indicator: r.tree_view_item_selection_indicator_foreground_pointer_over,
                indicator_opacity: 1.0,
            },
            PressedSelected => StateBrushes {
                background: r.tree_view_item_background_selected_pressed,
                border: r.tree_view_item_border_brush_selected_pressed,
                foreground: r.tree_view_item_foreground_selected_pressed,
                glyph: r.tree_view_item_foreground_selected_pressed,
                indicator: r.tree_view_item_selection_indicator_foreground_pressed,
                indicator_opacity: 1.0,
            },
        }
    }
}

/// A row of the tree: the `TreeViewItem` container TreeViewList makes for a node.
#[derive(Clone)]
pub(super) struct TreeViewItemContainer {
    /// The tree, which owns the item's state and receives its input.
    pub(super) owner: Handle<TreeViewState>,
    /// The node's id.
    pub(super) id: String,
    /// The node's `Content`.
    pub(super) content: WidgetRef,
    /// The node's `Depth`.
    pub(super) depth: usize,
    /// The node's `HasChildren`, which shows the chevron: `GlyphOpacity`.
    pub(super) has_children: bool,
    /// The node's `IsExpanded`, which picks the glyph.
    pub(super) is_expanded: bool,
    /// The tree's `IsEnabled`.
    pub(super) is_enabled: bool,
    /// The node's state in Multiple mode, which shows the check box; `None` in the other modes.
    pub(super) multi_select: Option<NodeSelectionState>,
    /// `IsSelected`: the node is the list's selection, in Single mode.
    pub(super) is_selected: bool,
    /// What the list's drag means for the item.
    pub(super) drag: ItemDrag,
    /// The item's focus; `None` for the drag preview, which takes no input.
    pub(super) focus: Option<ItemFocus>,
}

impl fmt::Debug for TreeViewItemContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TreeViewItemContainer")
            .field("id", &self.id)
            .field("depth", &self.depth)
            .field("is_expanded", &self.is_expanded)
            .field("is_selected", &self.is_selected)
            .field("multi_select", &self.multi_select)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for TreeViewItemContainer {
    type State = TreeViewItemContainerState;

    fn create_state(&self) -> TreeViewItemContainerState {
        TreeViewItemContainerState {
            state: StateData::new(),
            pointer_over: false,
            pressed: false,
        }
    }
}

/// ListViewBaseItem's pointer flags.
pub(super) struct TreeViewItemContainerState {
    state: StateData<TreeViewItemContainer>,

    /// `m_shouldEnterPointerOver`.
    pointer_over: bool,

    /// `m_isLeftButtonPressed`.
    pressed: bool,
}

impl TreeViewItemContainerState {
    /// The item's `CommonStates` state. A drag takes the pointer: items take no pointer-over or
    /// pressed state from it.
    fn common_state(self: Handle<Self>, app: &App) -> ItemCommonState {
        let widget = self.widget(app);
        let state = app.get(self);
        let (pointer_over, pressed) = if widget.drag.in_drag {
            (false, false)
        } else {
            (state.pointer_over, state.pressed)
        };
        common_state(
            widget.is_enabled,
            widget.is_selected,
            pressed,
            pointer_over,
            widget.drag,
        )
    }

    /// `ContentPresenterGrid` and what it holds.
    fn template(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let resources = ThemeResources::of(app, context);
        let r = &resources;
        let state = self.common_state(app);
        let brushes = StateBrushes::of(r, state);
        let multi = widget.multi_select.is_some();
        // `SelectionIndicator`.
        let (width, height) = SELECTION_INDICATOR_SIZE;
        let indicator = Align::new().alignment(Alignment::CENTER_LEFT.into()).child(
            Opacity::new(brushes.indicator_opacity).child(
                SizedBox::new().width(width).height(height).child(
                    ControlBorder::new(
                        Brush::Solid(brushes.indicator),
                        Brush::Solid(Color::new(0)),
                    )
                    .border_thickness(0.0)
                    .corner_radius(SELECTION_INDICATOR_RADIUS),
                ),
            ),
        );
        let multi_select_grid = self.multi_select_grid(app, context, r, &brushes);
        // `ReorderedPlaceholder`'s `FadeOutThemeAnimation` on `MultiSelectGrid`.
        let shown = state != ItemCommonState::ReorderedPlaceholder;
        let presenter_grid = Grid::new()
            .background(Brush::Solid(brushes.background))
            .border_brush(Brush::Solid(brushes.border))
            .border_thickness_ltrb(r.tree_view_item_border_theme_thickness)
            .corner_radius_corners(r.control_corner_radius)
            // `TreeViewMultiSelectEnabled*` sets the padding to 0.
            .padding(if multi {
                [0.0; 4]
            } else {
                r.tree_view_item_presenter_padding
            })
            .children([
                GridCell::new(indicator).into_widget(),
                GridCell::new(Opacity::new(if shown { 1.0 } else { 0.0 }).child(multi_select_grid))
                    .into_widget(),
            ]);
        let item = ConstrainedBox::new(
            BoxConstraints::new()
                .min_width(r.list_view_item_min_width)
                .min_height(r.tree_view_item_min_height),
        )
        .child(Margin::new(
            r.tree_view_item_presenter_margin,
            presenter_grid,
        ));
        let focused = widget
            .focus
            .as_ref()
            .is_some_and(|focus| focus.shows_focus_visual);
        FocusVisual::new(item, resources.theme)
            .visible(focused)
            .margin(FOCUS_VISUAL_MARGIN)
            .corner_radius(r.control_corner_radius[0])
            .primary_brush(r.list_view_item_focus_visual_primary_brush)
            .secondary_brush(r.list_view_item_focus_visual_secondary_brush)
            .into_widget()
    }

    /// `MultiSelectGrid`: the indentation, then the check box and drag count, the chevron and
    /// `ContentPresenter`.
    fn multi_select_grid(
        self: Handle<Self>,
        app: &mut App,
        _context: BuildContext,
        r: &ThemeResources,
        brushes: &StateBrushes,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let multi_selected = widget.multi_select == Some(NodeSelectionState::Selected);
        let background = if multi_selected {
            r.tree_view_item_background_selected
        } else {
            Color::new(0)
        };
        let border = if multi_selected {
            r.tree_view_item_multi_select_border_brush_selected
        } else {
            Color::new(0)
        };
        let content =
            ConstrainedBox::new(BoxConstraints::new().min_height(r.tree_view_item_content_height))
                .child(
                    // `VerticalContentAlignment` is `Center`; `HorizontalContentAlignment` `Stretch`.
                    Align::new().alignment(Alignment::CENTER_LEFT.into()).child(
                        FractionallySizedBox::new()
                            .width_factor(1.0)
                            .alignment(Alignment::CENTER_LEFT.into())
                            .child(DefaultTextStyle::new(
                                control_text_style(
                                    r,
                                    r.control_content_theme_font_size,
                                    FontWeight::NORMAL,
                                    brushes.foreground,
                                ),
                                widget.content.clone(),
                            )),
                    ),
                );
        let mut cells = Vec::new();
        if let Some(check) = self.selection_column(app, r) {
            cells.push(GridCell::new(check).into_widget());
        }
        cells.push(
            GridCell::new(self.chevron(app, brushes.glyph))
                .column(1)
                .into_widget(),
        );
        cells.push(GridCell::new(content).column(2).into_widget());
        Grid::new()
            .column_definitions([
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::STAR),
            ])
            .background(Brush::Solid(background))
            .border_brush(Brush::Solid(border))
            .border_thickness_ltrb(r.tree_view_item_border_theme_thickness)
            .corner_radius_corners(r.control_corner_radius)
            .padding([
                widget.depth as f64 * INDENTATION_PER_DEPTH
                    + r.tree_view_item_multi_select_selected_item_border_margin[0],
                r.tree_view_item_multi_select_selected_item_border_margin[1],
                r.tree_view_item_multi_select_selected_item_border_margin[2],
                r.tree_view_item_multi_select_selected_item_border_margin[3],
            ])
            .children(cells)
            .into_widget()
    }

    /// Column 0: `MultiSelectCheckBox`, shown in Multiple mode, and
    /// `MultiArrangeOverlayTextBorder`, shown in `MultipleDraggingPrimary`; nothing otherwise.
    fn selection_column(
        self: Handle<Self>,
        app: &mut App,
        r: &ThemeResources,
    ) -> Option<WidgetRef> {
        let widget = self.widget(app).clone();
        let dragging = multiple_dragging_primary(widget.drag);
        let mut children = Vec::new();
        if let Some(selection) = widget.multi_select {
            let is_checked = match selection {
                NodeSelectionState::Selected => Some(true),
                NodeSelectionState::PartialSelected => None,
                NodeSelectionState::UnSelected => Some(false),
            };
            let owner = widget.owner;
            let id = widget.id.clone();
            let check_box = CheckBox::new(is_checked, move |app, value| {
                owner.check_toggled(app, &id, value)
            })
            .is_enabled(widget.is_enabled);
            let id = widget.id.clone();
            // `IsTabStop="False"`; the press is the check box's.
            let check_box = PointerListener::new()
                .on_pointer_down(Rc::new(move |app, event| {
                    owner.part_pressed(app, &id, PressedPart::CheckBox, event.pointer)
                }))
                .child(ExcludeFocus::new(
                    SizedBox::new()
                        .width(CHECK_BOX_WIDTH)
                        .height(r.tree_view_item_multi_select_check_box_min_height)
                        .child(check_box),
                ));
            // `MultipleDraggingPrimary` hides the check box by its opacity: it keeps its place.
            children.push(
                Margin::new(
                    CHECK_BOX_MARGIN,
                    Center::new()
                        .height_factor(1.0)
                        .child(Opacity::new(if dragging { 0.0 } else { 1.0 }).child(check_box)),
                )
                .into_widget(),
            );
        }
        if dragging {
            children.push(
                Center::new()
                    .child(drag_count(r, widget.drag))
                    .into_widget(),
            );
        }
        if children.is_empty() {
            return None;
        }
        Some(
            Grid::new()
                .children(
                    children
                        .into_iter()
                        .map(|child| GridCell::new(child).into_widget()),
                )
                .into_widget(),
        )
    }

    /// `ExpandCollapseChevron`: the glyph of the node's expansion, shown when the node has
    /// children; a press anywhere on it, shown or not, toggles the node's `IsExpanded`.
    fn chevron(self: Handle<Self>, app: &App, color: Color) -> WidgetRef {
        let widget = self.widget(app).clone();
        let symbol = if widget.is_expanded {
            FluentSymbol::ChevronDown
        } else {
            FluentSymbol::ChevronRight
        };
        let padding = if widget.multi_select.is_some() {
            MULTI_SELECT_CHEVRON_PADDING
        } else {
            CHEVRON_PADDING
        };
        let glyph = SizedBox::new().width(GLYPH_BOX).height(GLYPH_BOX).child(
            Padding::new(EdgeInsetsGeometry::all(GLYPH_PADDING)).child(
                FontIcon::symbol(symbol)
                    .font_size(GLYPH_SIZE)
                    .foreground(color),
            ),
        );
        let chevron = Opacity::new(if widget.has_children { 1.0 } else { 0.0 }).child(
            Padding::new(EdgeInsetsGeometry::from_ltrb(
                padding[0], padding[1], padding[2], padding[3],
            ))
            .child(Center::new().width_factor(1.0).child(glyph)),
        );
        if widget.focus.is_none() || !widget.is_enabled {
            return chevron.into_widget();
        }
        let owner = widget.owner;
        let id = widget.id.clone();
        // `Background="Transparent"`: the whole cell takes the press.
        PointerListener::new()
            .behavior(HitTestBehavior::Opaque)
            .on_pointer_down(Rc::new(move |app, event| {
                if event.buttons & K_PRIMARY_BUTTON != 0 {
                    owner.part_pressed(app, &id, PressedPart::Chevron, event.pointer);
                    // `OnExpandCollapseChevronPointerPressed`.
                    owner.chevron_pressed(app, &id);
                }
            }))
            .child(chevron)
            .into_widget()
    }

    /// The item's pointer input: ListViewBaseItem's `OnPointerPressed`, `OnPointerReleased`,
    /// `OnPointerEntered`, `OnPointerExited` and `OnRightTapped`.
    fn pointer_input(self: Handle<Self>, app: &App, child: WidgetRef) -> WidgetRef {
        let widget = self.widget(app).clone();
        let owner = widget.owner;
        let id = widget.id.clone();
        let tap_id = id.clone();
        let secondary_id = id.clone();
        let down_id = id.clone();
        let gesture = GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            .on_tap_down(Rc::new(move |app, _| {
                if !owner.pressed_part_is(app, &down_id, PressedPart::Chevron) {
                    self.set_state(app, |state| state.pressed = true);
                }
            }))
            .on_tap_up(Rc::new(move |app, _| {
                self.set_state(app, |state| state.pressed = false)
            }))
            .on_tap_cancel(Listener::new(move |app| {
                self.set_state(app, |state| state.pressed = false)
            }))
            .on_tap(Listener::new(move |app| {
                // `DoTapInteraction`, unless the chevron handled the press.
                if !owner.pressed_part_is(app, &tap_id, PressedPart::Chevron) {
                    owner.item_tapped(app, &tap_id);
                }
            }))
            .on_secondary_tap(Listener::new(move |app| {
                owner.item_right_tapped(app, &secondary_id)
            }))
            .child(child);
        let pressed = PointerListener::new()
            .on_pointer_down(Rc::new(move |app, event| {
                owner.part_pressed(app, &id, PressedPart::Item, event.pointer)
            }))
            .child(gesture);
        MouseRegion::new()
            .on_enter(Rc::new(move |app, _| {
                self.set_state(app, |state| state.pointer_over = true)
            }))
            .on_exit(Rc::new(move |app, _| {
                self.set_state(app, |state| {
                    state.pointer_over = false;
                    state.pressed = false;
                })
            }))
            .child(pressed)
            .into_widget()
    }
}

/// `MultiArrangeOverlayTextBorder` with `MultiArrangeOverlayText`: the number of nodes a drag
/// carries.
fn drag_count(r: &ThemeResources, drag: ItemDrag) -> WidgetRef {
    ConstrainedBox::new(
        BoxConstraints::new()
            .min_width(DRAG_COUNT_MIN_WIDTH)
            .min_height(DRAG_COUNT_HEIGHT)
            .max_height(DRAG_COUNT_HEIGHT),
    )
    .child(
        ControlBorder::new(
            Brush::Solid(r.system_control_background_accent_brush),
            Brush::Solid(r.system_control_background_chrome_white_brush),
        )
        .border_thickness(DRAG_COUNT_BORDER_THICKNESS)
        .corner_radius_corners(r.control_corner_radius)
        .child(
            Center::new().width_factor(1.0).child(
                Text::new(drag.drag_items_count_display.to_string())
                    .style(
                        TextBlockStyle::Caption
                            .text_style(r, r.system_control_foreground_chrome_white_brush),
                    )
                    .soft_wrap(false),
            ),
        ),
    )
    .into_widget()
}

impl State for TreeViewItemContainerState {
    type Widget = TreeViewItemContainer;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        if widget.focus.is_some() {
            widget.owner.row_realized(app, &widget.id, true);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        if widget.focus.is_some() {
            widget.owner.row_realized(app, &widget.id, false);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &TreeViewItemContainer) {
        // `ClearPointerState` of an item a drag starts on.
        if self.widget(app).drag.in_drag {
            let state = app.get_mut(self);
            state.pointer_over = false;
            state.pressed = false;
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let template = self.template(app, context);
        let Some(focus) = widget.focus.clone() else {
            return IgnorePointer::new().child(template).into_widget();
        };
        let interactive = if widget.is_enabled {
            self.pointer_input(app, template)
        } else {
            template
        };
        let owner = widget.owner;
        let id = widget.id.clone();
        Focus::new(interactive)
            .focus_node(focus.node.as_node())
            .can_request_focus(widget.is_enabled)
            .skip_traversal(!focus.is_tab_stop)
            .on_focus_change(move |app, focused| owner.item_focus_changed(app, &id, focused))
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ItemCommonState::*;

    fn dragging(primary: bool, captured: bool, can_reorder: bool) -> ItemDrag {
        ItemDrag {
            in_drag: true,
            primary,
            captured,
            can_reorder,
            drag_items_count: 1,
            ..ItemDrag::default()
        }
    }

    #[test]
    fn a_disabled_item_is_normal_or_selected() {
        let none = ItemDrag::default();
        assert_eq!(common_state(false, false, true, true, none), Normal);
        assert_eq!(common_state(false, true, false, false, none), Selected);
    }

    #[test]
    fn pressing_and_hovering_combine_with_the_selection() {
        let none = ItemDrag::default();
        assert_eq!(common_state(true, false, false, true, none), PointerOver);
        assert_eq!(common_state(true, false, true, true, none), Pressed);
        assert_eq!(
            common_state(true, true, false, true, none),
            PointerOverSelected
        );
        assert_eq!(common_state(true, true, true, true, none), PressedSelected);
        assert_eq!(common_state(true, true, false, false, none), Selected);
    }

    #[test]
    fn a_dragged_over_item_looks_pointed_at() {
        let over = ItemDrag {
            dragged_over: true,
            ..dragging(false, true, true)
        };
        assert_eq!(common_state(true, false, false, false, over), PointerOver);
        assert_eq!(
            common_state(true, true, false, false, over),
            PointerOverSelected
        );
    }

    #[test]
    fn the_item_a_reorder_starts_on_is_a_placeholder_once_captured() {
        assert_eq!(
            common_state(true, true, false, false, dragging(true, false, true)),
            PressedSelected
        );
        assert_eq!(
            common_state(true, false, false, false, dragging(true, false, true)),
            Pressed
        );
        assert_eq!(
            common_state(true, true, false, false, dragging(true, true, true)),
            ReorderedPlaceholder
        );
        // A drag that cannot reorder leaves the item in place.
        assert_eq!(
            common_state(true, false, false, false, dragging(true, true, false)),
            Normal
        );
    }

    #[test]
    fn the_drag_count_shows_for_a_drag_of_several_selected_nodes() {
        let several = ItemDrag {
            multiple_dragging_primary: true,
            drag_items_count: 3,
            ..dragging(true, true, true)
        };
        assert!(multiple_dragging_primary(several));
        assert!(!multiple_dragging_primary(dragging(true, true, true)));
        let unordered = ItemDrag {
            drag_items_count: 2,
            ..dragging(true, false, false)
        };
        assert!(multiple_dragging_primary(unordered));
    }
}
