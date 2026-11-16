//! One suggestion of `SuggestionsList`: a `ListViewItem` in `DefaultListViewItemStyle`, drawn as
//! ListViewBaseItemChrome.cpp draws a `ListViewItemPresenter` with rounded chrome in Single
//! selection mode: the backplate, the selection indicator over the content's start, and the
//! content.
//!
//! The suggestion list is the only list of the kit's that holds these, so only what it uses is
//! here: no check boxes, no drag and drop, no focus (the text box keeps it while the arrow keys
//! move the selection), and the indicator's press is a change of height without its animation.

use crate::*;
use std::fmt;
use waft_embedder::{Color, FontWeight};
use waft_foundation::{App, Listener};
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry};
use waft_rendering::BoxConstraints;
use waft_widgets::*;

/// `s_backplateMargin`.
const BACKPLATE_MARGIN: [f64; 4] = [4.0, 2.0, 4.0, 2.0];

/// `s_selectionIndicatorSize`.
const SELECTION_INDICATOR_WIDTH: f64 = 3.0;
const SELECTION_INDICATOR_HEIGHT: f64 = 16.0;

/// `s_selectionIndicatorMargin`: its start, and the room it leaves above and below.
const SELECTION_INDICATOR_START: f64 = 4.0;
const SELECTION_INDICATOR_VERTICAL_MARGIN: f64 = 20.0;

/// `s_selectionIndicatorHeightShrinkage`: how much shorter a pressed indicator is.
const SELECTION_INDICATOR_HEIGHT_SHRINKAGE: f64 = 6.0;

/// `DefaultListViewItemStyle`'s `Padding`, the presenter's `ContentMargin`.
const CONTENT_MARGIN: [f64; 4] = [16.0, 0.0, 12.0, 0.0];

/// A suggestion's container.
pub(super) struct SuggestionItem {
    /// What the item shows: its `ItemTemplate`, or its text.
    pub(super) content: WidgetRef,

    /// Whether the list's selection is this item.
    pub(super) selected: bool,

    /// `ItemClick`.
    pub(super) clicked: Listener,
}

impl fmt::Debug for SuggestionItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SuggestionItem")
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for SuggestionItem {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let content = self.content.clone();
        let selected = self.selected;
        CommonStates::new(self.clicked.clone(), move |app, context, states| {
            let r = ThemeResources::of(app, context);
            chrome(&r, states.common, selected, content.clone())
        })
        .is_tab_stop(false)
        .into_widget()
    }
}

/// The brushes `SetBackplateBackground`, `SetForegroundBrush` and `SetSelectionIndicatorBackground`
/// choose for a state.
struct ChromeBrushes {
    backplate: Color,
    foreground: Color,
    indicator: Color,
}

impl ChromeBrushes {
    fn of(r: &ThemeResources, common: CommonState, selected: bool) -> ChromeBrushes {
        let (backplate, indicator) = match (common, selected) {
            (CommonState::Disabled, true) => (
                r.list_view_item_background_selected_disabled,
                r.list_view_item_selection_indicator_disabled_brush,
            ),
            (CommonState::Disabled, false) => (
                Color::new(0),
                r.list_view_item_selection_indicator_disabled_brush,
            ),
            (CommonState::Pressed, true) => (
                r.list_view_item_background_selected_pressed,
                r.list_view_item_selection_indicator_pressed_brush,
            ),
            (CommonState::Pressed, false) => (
                r.list_view_item_background_pressed,
                r.list_view_item_selection_indicator_brush,
            ),
            (CommonState::PointerOver, true) => (
                r.list_view_item_background_selected_pointer_over,
                r.list_view_item_selection_indicator_pointer_over_brush,
            ),
            (CommonState::PointerOver, false) => (
                r.list_view_item_background_pointer_over,
                r.list_view_item_selection_indicator_brush,
            ),
            (CommonState::Normal, true) => (
                r.list_view_item_background_selected,
                r.list_view_item_selection_indicator_brush,
            ),
            (CommonState::Normal, false) => (
                r.list_view_item_background,
                r.list_view_item_selection_indicator_brush,
            ),
        };
        let foreground = if selected {
            r.list_view_item_foreground_selected
        } else if matches!(common, CommonState::PointerOver | CommonState::Pressed) {
            r.list_view_item_foreground_pointer_over
        } else {
            r.list_view_item_foreground
        };
        ChromeBrushes {
            backplate,
            foreground,
            indicator,
        }
    }
}

/// The presenter: at least `ListViewItemMinWidth` by `ListViewItemMinHeight`, the content
/// stretched across and centred down within `ContentMargin`, over the backplate and under the
/// selection indicator.
fn chrome(
    r: &ThemeResources,
    common: CommonState,
    selected: bool,
    content: WidgetRef,
) -> WidgetRef {
    let brushes = ChromeBrushes::of(r, common, selected);
    let content = ConstrainedBox::new(
        BoxConstraints::new()
            .min_width(r.list_view_item_min_width)
            .min_height(r.list_view_item_min_height),
    )
    .child(
        Padding::new(insets(CONTENT_MARGIN)).child(
            Align::new()
                .alignment(AlignmentGeometry::CENTER_START)
                .child(
                    SizedBox::new()
                        .width(f64::INFINITY)
                        .child(DefaultTextStyle::new(
                            control_text_style(
                                r,
                                r.control_content_theme_font_size,
                                FontWeight::NORMAL,
                                brushes.foreground,
                            ),
                            content,
                        )),
                ),
        ),
    );
    let backplate = Padding::new(insets(BACKPLATE_MARGIN)).child(
        ControlBorder::new(Brush::Solid(brushes.backplate), Brush::Solid(Color::new(0)))
            .border_thickness(0.0)
            .corner_radius_corners(r.list_view_item_corner_radius),
    );
    let mut layers = vec![
        Positioned::fill(backplate).into_widget(),
        content.into_widget(),
    ];
    if selected {
        let pressed = common == CommonState::Pressed;
        let color = brushes.indicator;
        let corner_radius = r.list_view_item_selection_indicator_corner_radius;
        layers.push(
            Positioned::fill(
                IgnorePointer::new().child(
                    LayoutBuilder::new(move |_, _, constraints| {
                        let height = selection_indicator_height(constraints.max_height, pressed);
                        Padding::new(EdgeInsetsGeometry::directional(
                            SELECTION_INDICATOR_START,
                            0.0,
                            0.0,
                            0.0,
                        ))
                        .child(
                            Align::new()
                                .alignment(AlignmentGeometry::CENTER_START)
                                .child(
                                    SizedBox::new()
                                        .width(SELECTION_INDICATOR_WIDTH)
                                        .height(height)
                                        .child(
                                            ControlBorder::new(
                                                Brush::Solid(color),
                                                Brush::Solid(Color::new(0)),
                                            )
                                            .border_thickness(0.0)
                                            .corner_radius_corners(corner_radius),
                                        ),
                                ),
                        )
                        .into_widget()
                    })
                    .into_widget(),
                ),
            )
            .into_widget(),
        );
    }
    Stack::new().children(layers).into_widget()
}

/// `GetSelectionIndicatorHeightFromAvailableHeight`, less the shrinkage while pressed.
fn selection_indicator_height(available: f64, pressed: bool) -> f64 {
    let height = if available <= SELECTION_INDICATOR_HEIGHT {
        available
    } else {
        SELECTION_INDICATOR_HEIGHT.max(available - 2.0 * SELECTION_INDICATOR_VERTICAL_MARGIN)
    };
    if pressed {
        (height - SELECTION_INDICATOR_HEIGHT_SHRINKAGE).max(0.0)
    } else {
        height
    }
}

/// Converts XAML thickness order into native logical padding.
fn insets(value: [f64; 4]) -> EdgeInsetsGeometry {
    EdgeInsetsGeometry::from_ltrb(value[0], value[1], value[2], value[3])
}
