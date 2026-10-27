//! TabScrollViewerStyle from controls/dev/TabView/TabView.xaml.
//!
//! The scrolling child is supplied by TabView, so the native virtualized header list
//! remains the only horizontal viewport. The two RepeatButtons follow TabView.cpp.

use crate::{
    Brush, ColumnDefinition, CommonState, ControlBorder, ControlStates, FluentSymbol, FocusVisual,
    FontIcon, Grid, GridCell, GridLength, RepeatButton, ScrollMetricsObserver,
    ScrollViewportController, ThemeResources, ToolTip, ToolTipService, control_text_style,
};
use inset_embedder::{Color, FontWeight};
use inset_foundation::{App, Handle, Listener, ValueKey};
use inset_painting::{Alignment, EdgeInsetsGeometry};
use inset_widgets::*;
use std::{rc::Rc, time::Duration};

/// TabView's custom scrolling template, independent of the default ScrollViewer style.
#[derive(Clone, Debug)]
pub(crate) struct TabScrollViewer {
    /// Controller attached to the virtualized header list.
    pub controller: Handle<ScrollViewportController>,

    /// The native horizontal header list, already attached to the controller.
    pub scrolling_child: WidgetRef,

    /// ComputedHorizontalScrollBarVisibility, determined by TabView's width algorithm.
    pub scroll_buttons_visible: bool,

    /// NormalBottomBorderLine versus NoBottomBorderLine.
    pub bottom_border_visible: bool,
}

impl TabScrollViewer {
    /// Wraps the tab header list in its source scroll-button template.
    pub fn new<K>(
        controller: Handle<ScrollViewportController>,
        scrolling_child: impl IntoWidget<K>,
    ) -> Self {
        Self {
            controller,
            scrolling_child: scrolling_child.into_widget(),
            scroll_buttons_visible: false,
            bottom_border_visible: true,
        }
    }

    /// Sets ComputedHorizontalScrollBarVisibility from the owning TabView.
    pub fn scroll_buttons_visible(mut self, visible: bool) -> Self {
        self.scroll_buttons_visible = visible;
        self
    }

    /// Selects the bottom border visual state.
    pub fn bottom_border_visible(mut self, visible: bool) -> Self {
        self.bottom_border_visible = visible;
        self
    }

    /// Builds the named template parts for the current scroll offset.
    fn build_template(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let metrics = self.controller.metrics(app);
        let (decrease_enabled, increase_enabled) =
            button_states(metrics.offset, metrics.scrollable_length);
        let mut children = Vec::new();
        if self.bottom_border_visible {
            for (name, column) in [("LeftBottomBorderLine", 0), ("RightBottomBorderLine", 2)] {
                children.push(
                    bottom_border_line(name, resources.tab_view_border_brush)
                        .column(column)
                        .into_widget(),
                );
            }
        }
        if self.scroll_buttons_visible {
            // ScrollDecreaseButtonContainer, ScrollIncreaseButtonContainer.
            for (increase, enabled, padding, column) in [
                (
                    false,
                    decrease_enabled,
                    resources.tab_view_item_left_scroll_button_container_padding,
                    0,
                ),
                (
                    true,
                    increase_enabled,
                    resources.tab_view_item_right_scroll_button_container_padding,
                    2,
                ),
            ] {
                let controller = self.controller;
                let button = RepeatButton::new(
                    FontIcon::symbol(if increase {
                        FluentSymbol::ChevronRight
                    } else {
                        FluentSymbol::ChevronLeft
                    })
                    .font_size(resources.tab_view_item_scroll_buton_font_size)
                    .mirrored_when_right_to_left(true),
                    Listener::new(move |app| {
                        controller.scroll_by(app, if increase { 50.0 } else { -50.0 }, false);
                    }),
                )
                .delay(Duration::from_millis(50))
                .interval(Duration::from_millis(100))
                .is_tab_stop(false)
                .is_enabled(enabled)
                .template(scroll_button_template);
                let tip = if increase {
                    "Scroll tab list forward"
                } else {
                    "Scroll tab list backward"
                };
                let button = ToolTipService::new(button, ToolTip::new(Text::new(tip)));
                children.push(
                    GridCell::new(
                        Align::new()
                            .alignment(Alignment::BOTTOM_CENTER.into())
                            .child(
                                Padding::new(EdgeInsetsGeometry::from_ltrb(
                                    padding[0], padding[1], padding[2], padding[3],
                                ))
                                .child(button),
                            ),
                    )
                    .column(column)
                    .key(Rc::new(ValueKey::new(if increase {
                        "ScrollIncreaseButtonContainer"
                    } else {
                        "ScrollDecreaseButtonContainer"
                    })))
                    .into_widget(),
                );
            }
        }
        // ScrollContentPresenter. Padding is outside the list so its own viewport measures
        // the same available width as the source presenter's content area.
        let presenter_index = children.len() - usize::from(self.scroll_buttons_visible);
        children.insert(
            presenter_index,
            GridCell::new(
                Padding::new(EdgeInsetsGeometry::only(1.0, 0.0, 0.0, 0.0)).child(
                    ScrollMetricsObserver::new(self.controller, self.scrolling_child.clone()),
                ),
            )
            .column(1)
            .key(Rc::new(ValueKey::new("ScrollContentPresenter")))
            .into_widget(),
        );
        // Root (Border), then its three-column Grid.
        ControlBorder::new(Brush::Solid(Color::new(0)), Brush::Solid(Color::new(0)))
            .border_thickness(0.0)
            .child(
                Grid::new()
                    .column_definitions([
                        ColumnDefinition::new(GridLength::AUTO).min_width(2.0),
                        ColumnDefinition::new(GridLength::STAR),
                        ColumnDefinition::new(GridLength::AUTO),
                    ])
                    .children(children),
            )
            .into_widget()
    }
}

impl StatelessWidget for TabScrollViewer {
    fn build(&self, _: &mut App, _: BuildContext) -> WidgetRef {
        let template = self.clone();
        ListenableBuilder::new(Rc::new(self.controller), move |app, context, _| {
            template.build_template(app, context)
        })
        .into_widget()
    }
}

/// `LeftBottomBorderLine` or `RightBottomBorderLine` of TabView's templates, keyed `name`: a
/// `Border` with a `color` background, `Height="1"` and `VerticalAlignment="Bottom"`, stretched
/// across its cell. As a bottom border it paints and hit-tests that one-pixel row while measuring
/// 0 × 1, as the source element does; an aligned `ColoredBox` would paint nothing.
pub(crate) fn bottom_border_line(name: &'static str, color: Color) -> GridCell {
    GridCell::new(
        ControlBorder::new_optional(None, Some(Brush::Solid(color)))
            .border_thickness_ltrb([0.0, 0.0, 0.0, 1.0]),
    )
    .key(Rc::new(ValueKey::new(name)))
}

/// TabView::UpdateScrollViewerDecreaseAndIncreaseButtonsViewState's edge tolerance.
fn button_states(offset: f64, scrollable: f64) -> (bool, bool) {
    if (offset - scrollable).abs() < 0.1 {
        (true, false)
    } else if offset.abs() < 0.1 {
        (false, true)
    } else {
        (true, true)
    }
}

/// TabViewScrollButtonStyle's ContentPresenter and CommonStates.
fn scroll_button_template(
    app: &mut App,
    context: BuildContext,
    states: ControlStates,
    content: WidgetRef,
) -> WidgetRef {
    let theme = ThemeResources::of(app, context);
    let r = &theme;
    let (background, foreground, border) = match states.common {
        CommonState::Normal => (
            r.tab_view_scroll_button_background,
            r.tab_view_scroll_button_foreground,
            r.tab_view_scroll_button_border_brush,
        ),
        CommonState::PointerOver => (
            r.tab_view_scroll_button_background_pointer_over,
            r.tab_view_scroll_button_foreground_pointer_over,
            r.tab_view_scroll_button_border_brush_pointer_over,
        ),
        CommonState::Pressed => (
            r.tab_view_scroll_button_background_pressed,
            r.tab_view_scroll_button_foreground_pressed,
            r.tab_view_scroll_button_border_brush_pressed,
        ),
        CommonState::Disabled => (
            r.tab_view_scroll_button_background_disabled,
            r.tab_view_scroll_button_foreground_disabled,
            r.tab_view_scroll_button_border_brush_disabled,
        ),
    };
    let presenter = SizedBox::new()
        .width(r.tab_view_item_scroll_button_width)
        .height(r.tab_view_item_scroll_button_height)
        .child(
            ControlBorder::new(Brush::Solid(background), Brush::Solid(border))
                .border_thickness_ltrb(r.tab_view_button_border_thickness)
                .corner_radius(r.control_corner_radius[0])
                .child(Center::new().child(DefaultTextStyle::new(
                    control_text_style(
                        &theme,
                        r.tab_view_item_scroll_buton_font_size,
                        FontWeight::W400,
                        foreground,
                    ),
                    content,
                ))),
        );
    FocusVisual::new(presenter, theme.theme)
        .visible(states.focused)
        .margin([-3.0; 4])
        .corner_radius(r.control_corner_radius[0])
        .into_widget()
}

#[cfg(test)]
mod tests {
    use super::button_states;

    #[test]
    fn tab_scroll_buttons_use_the_source_edge_tolerance() {
        assert_eq!(button_states(0.0, 500.0), (false, true));
        assert_eq!(button_states(0.09, 500.0), (false, true));
        assert_eq!(button_states(250.0, 500.0), (true, true));
        assert_eq!(button_states(499.91, 500.0), (true, false));
        assert_eq!(button_states(500.0, 500.0), (true, false));
    }
}
