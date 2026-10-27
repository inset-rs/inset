//! NavigationView.cpp's pane-button template settings and NavigationView.xaml chrome.

use super::{
    NavigationView, NavigationViewBackButtonVisible, NavigationViewDisplayMode,
    NavigationViewPaneDisplayMode,
};
use crate::*;
use inset_embedder::FontWeight;
use inset_foundation::App;
use inset_painting::{Alignment, EdgeInsetsGeometry, TextStyle};
use inset_widgets::*;

/// The values produced by the source's button-visibility, title-parent and size updates.
pub(super) struct PaneChrome {
    /// Whether the source Back button is visible.
    pub show_back: bool,
    /// Whether the source Close button replaces Back in an open minimal pane.
    pub show_close: bool,
    /// Whether the left pane toggle button is visible.
    pub show_toggle: bool,
    /// Whether PaneTitleHolder hosts the title independently of the toggle.
    pub show_title_holder: bool,
    /// Whether PaneTitleOnTopPane hosts the title.
    pub show_top_title: bool,
    /// ListSizeCompact collapses title text and the custom pane header.
    pub closed_compact: bool,
    /// RootSplitView uses Overlay only for left Minimal mode.
    pub overlay: bool,
    /// Back's bound SmallerPaneToggleButtonWidth.
    pub back_width: f64,
    /// Width assigned by UpdatePaneToggleSize, before the template's inner margin.
    pub toggle_width: f64,
    /// Horizontal margin applied to the toggle/title holder in Overlay mode.
    pub toggle_left: f64,
    /// Vertical margin applied to the toggle/title holder outside Overlay mode.
    pub toggle_top: f64,
    /// The separate back-button row inside PaneContentGrid.
    pub back_row: f64,
    /// PaneContentGridToggleButtonRow's visible-state minimum.
    pub toggle_row_min_height: f64,
    /// PaneHeaderContentBorderRow's minimum from toggle and close visibility.
    pub header_row_min_height: f64,
    /// PaneHeaderCloseButtonColumn width.
    pub header_close_width: f64,
    /// PaneHeaderToggleButtonColumn width.
    pub header_toggle_width: f64,
    /// ContentLeftPadding for Minimal header layout.
    pub content_left_padding: f64,
}

impl PaneChrome {
    /// Resolves the source's desktop branch without conflating Top with left Minimal mode.
    pub fn resolve(
        r: &ThemeResources,
        widget: &NavigationView,
        mode: NavigationViewDisplayMode,
        open: bool,
        width: f64,
    ) -> Self {
        let top = widget.pane_display_mode == NavigationViewPaneDisplayMode::Top;
        let overlay = !top && mode == NavigationViewDisplayMode::Minimal;
        let permitted = widget.is_back_button_visible != NavigationViewBackButtonVisible::Collapsed;
        let show_back = permitted && !(mode == NavigationViewDisplayMode::Minimal && open);
        let show_close = permitted
            && open
            && (widget.pane_display_mode == NavigationViewPaneDisplayMode::LeftMinimal
                || (widget.pane_display_mode == NavigationViewPaneDisplayMode::Auto
                    && mode == NavigationViewDisplayMode::Minimal));
        let show_toggle = !top && widget.is_pane_visible && widget.is_pane_toggle_button_visible;
        let closed_compact = !open && !top && mode != NavigationViewDisplayMode::Minimal;
        let show_title_holder = !widget.is_pane_toggle_button_visible
            && !top
            && !widget.pane_title.is_empty()
            && !(widget.pane_display_mode == NavigationViewPaneDisplayMode::LeftMinimal && !open);
        let back_width = (widget.compact_pane_length - 8.0).max(0.0);
        let toggle_width =
            if !closed_compact && !widget.pane_title.is_empty() && !(overlay && !open) {
                (widget.open_pane_length.min(width)
                    - if overlay && (show_back || show_close) {
                        r.navigation_back_button_width
                    } else {
                        0.0
                    })
                .max(0.0)
            } else {
                widget.compact_pane_length
            };
        let toggle_left = if overlay && (show_back || show_close) {
            r.navigation_back_button_width
        } else {
            0.0
        };
        // c_backButtonHeight is the source's 40px row, distinct from the 36px styled button.
        let back_row = if !overlay && show_back {
            r.navigation_view_pane_header_row_min_height
        } else {
            0.0
        };
        let header_toggle_width = if show_toggle {
            r.pane_toggle_button_width
        } else {
            0.0
        };
        let header_close_width = if overlay && show_close {
            r.navigation_back_button_width
        } else {
            0.0
        };
        Self {
            show_back,
            show_close,
            show_toggle,
            show_title_holder,
            show_top_title: top
                && !widget.is_pane_toggle_button_visible
                && !widget.pane_title.is_empty(),
            closed_compact,
            overlay,
            back_width,
            toggle_width,
            toggle_left,
            toggle_top: back_row,
            back_row,
            toggle_row_min_height: if show_toggle {
                r.navigation_view_pane_header_row_min_height
            } else {
                0.0
            },
            header_row_min_height: if show_toggle || show_close {
                r.pane_toggle_button_height
            } else {
                0.0
            },
            header_close_width,
            header_toggle_width,
            content_left_padding: if overlay {
                header_toggle_width + if show_back { back_width } else { 0.0 } + header_close_width
            } else {
                0.0
            },
        }
    }
}

/// The source PaneTitleTextBlock's typography and non-wrapping content.
pub(super) fn pane_title(r: &ThemeResources, title: String) -> WidgetRef {
    Margin::new(
        [0.0, -2.0, 0.0, 0.0],
        Text::new(title).soft_wrap(false).max_lines(1).style(
            TextStyle::new()
                .font_family(r.content_control_theme_font_family)
                .font_size(14.0)
                .font_weight(FontWeight::W600),
        ),
    )
    .into_widget()
}

/// PaneToggleButtonStyle keeps the icon column separate from the title content column.
pub(super) fn pane_toggle_template(
    app: &mut App,
    context: BuildContext,
    states: ControlStates,
    content: WidgetRef,
    icon_width: f64,
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
        CommonState::Disabled => (
            r.navigation_view_button_background_disabled,
            r.navigation_view_button_foreground_disabled,
        ),
        _ => (
            r.navigation_view_item_background,
            r.navigation_view_item_foreground,
        ),
    };
    let margin = r.navigation_view_item_button_margin;
    let content = Padding::new(EdgeInsetsGeometry::only(
        margin[0], margin[1], margin[2], margin[3],
    ))
    .child(
        SizedBox::new().height(r.pane_toggle_button_height).child(
            Grid::new()
                .background(Brush::Solid(background))
                .corner_radius(r.control_corner_radius[0])
                .column_definitions([
                    ColumnDefinition::new(GridLength::pixel(icon_width)),
                    ColumnDefinition::new(GridLength::STAR),
                ])
                .children([
                    GridCell::new(
                        Center::new().child(
                            FontIcon::symbol(FluentSymbol::Navigation)
                                .font_size(16.0)
                                .foreground(foreground),
                        ),
                    )
                    .into_widget(),
                    GridCell::new(
                        Padding::new(EdgeInsetsGeometry::only(4.0, 0.0, 0.0, 0.0)).child(
                            Align::new().alignment(Alignment::CENTER_LEFT.into()).child(
                                DefaultTextStyle::new(
                                    control_text_style(
                                        &resources,
                                        16.0,
                                        FontWeight::NORMAL,
                                        foreground,
                                    ),
                                    content,
                                ),
                            ),
                        ),
                    )
                    .column(1)
                    .into_widget(),
                ]),
        ),
    )
    .into_widget();
    FocusVisual::new(content, resources.theme)
        .visible(states.focused)
        .corner_radius(r.control_corner_radius[0])
        .into_widget()
}

/// NavigationBackButtonNormalStyle and SmallStyle differ only in their outer margin.
pub(super) fn back_button_template(
    app: &mut App,
    context: BuildContext,
    states: ControlStates,
    content: WidgetRef,
    width: f64,
    small: bool,
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
        CommonState::Disabled => (
            resources.navigation_view_back_button_background,
            r.navigation_view_button_foreground_disabled,
        ),
        _ => (
            resources.navigation_view_back_button_background,
            r.navigation_view_item_foreground,
        ),
    };
    let content = Padding::new(EdgeInsetsGeometry::only(
        4.0,
        2.0,
        if small { 0.0 } else { 4.0 },
        2.0,
    ))
    .child(
        SizedBox::new()
            .width(width)
            .height(r.navigation_back_button_height)
            .child(
                Grid::new()
                    .background(Brush::Solid(background))
                    .corner_radius(r.control_corner_radius[0])
                    .children([Center::new()
                        .child(DefaultTextStyle::new(
                            control_text_style(&resources, 16.0, FontWeight::NORMAL, foreground),
                            content,
                        ))
                        .into_widget()]),
            ),
    )
    .into_widget();
    FocusVisual::new(content, resources.theme)
        .visible(states.focused)
        .margin(BUTTON_FOCUS_VISUAL_MARGIN)
        .corner_radius(r.control_corner_radius[0])
        .into_widget()
}
