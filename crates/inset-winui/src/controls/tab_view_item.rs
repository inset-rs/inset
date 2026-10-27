//! TabViewItem's header template and pointer behavior from TabView.xaml and TabViewItem.cpp.

use crate::*;
use inset_embedder::Canvas;
use inset_embedder::valo::{FillRule, Paint};
use inset_embedder::{Color, FontWeight, Path, PathBuilder, Size};
use inset_foundation::{App, Listener};
use inset_painting::{Alignment, EdgeInsetsGeometry};
use inset_rendering::{BoxConstraints, CustomPainter};
use inset_widgets::*;
use std::{any::Any, fmt, rc::Rc};

/// How the tab strip allocates width to its headers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabViewWidthMode {
    /// Equal widths constrained by TabViewItemMinWidth and TabViewItemMaxWidth.
    #[default]
    Equal,

    /// Each header takes the width required by its content.
    SizeToContent,

    /// Unselected headers show their icon without their label.
    Compact,
}

/// When closable tabs expose their close button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabViewCloseButtonOverlayMode {
    /// The desktop default displays the close button.
    #[default]
    Auto,

    /// Displays the button for a selected or hovered tab.
    OnPointerOver,

    /// Displays the close button whenever the tab is closable.
    Always,
}

/// A stable tab identity, header, and content supplied to TabView.
///
/// The identity must remain unchanged when the tab moves within the collection.
#[derive(Clone)]
pub struct TabViewItem {
    /// Stable item identity, independent of its current collection index.
    pub id: String,

    /// Header content presented in the tab strip.
    pub header: WidgetRef,

    /// The original string header used for the default header ToolTip.
    pub header_text: Option<String>,

    /// The icon element presented beside the header.
    pub icon_source: Option<WidgetRef>,

    /// Content presented when this tab is selected.
    pub content: WidgetRef,

    /// Whether the close button and middle-click close request are available.
    pub is_closable: bool,

    /// Whether the tab accepts input and keyboard selection.
    pub is_enabled: bool,

    /// Whether the tab participates in layout and keyboard navigation.
    pub is_visible: bool,

    /// An explicit foreground used in the unselected resting state.
    pub foreground: Option<Color>,

    /// An explicit tooltip, overriding the nonempty string header fallback.
    pub tool_tip: Option<ToolTip>,

    /// Raised after the parent TabCloseRequested event; the owner decides whether to remove it.
    pub close_requested: Option<Listener>,
}

impl fmt::Debug for TabViewItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TabViewItem")
            .field("id", &self.id)
            .field("header_text", &self.header_text)
            .finish_non_exhaustive()
    }
}

impl TabViewItem {
    /// Creates a tab with a stable identity, header widget and content widget.
    pub fn new<H, C>(
        id: impl Into<String>,
        header: impl IntoWidget<H>,
        content: impl IntoWidget<C>,
    ) -> Self {
        Self {
            id: id.into(),
            header: header.into_widget(),
            header_text: None,
            icon_source: None,
            content: content.into_widget(),
            is_closable: true,
            is_enabled: true,
            is_visible: true,
            foreground: None,
            tool_tip: None,
            close_requested: None,
        }
    }

    /// Creates a string header with the source's automatic header tooltip.
    pub fn text<C>(
        id: impl Into<String>,
        header: impl Into<String>,
        content: impl IntoWidget<C>,
    ) -> Self {
        let text = header.into();
        let mut item = Self::new(id, Text::new(text.clone()), content);
        item.header_text = Some(text);
        item
    }

    /// Sets the icon element; native widgets replace XAML IconSource factories.
    pub fn icon_source<K>(mut self, icon: impl IntoWidget<K>) -> Self {
        self.icon_source = Some(icon.into_widget());
        self
    }

    /// Sets whether closing can be requested for this tab.
    pub fn is_closable(mut self, value: bool) -> Self {
        self.is_closable = value;
        self
    }

    /// Sets whether the tab accepts pointer and keyboard input.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Sets whether the tab participates in layout and navigation.
    pub fn is_visible(mut self, value: bool) -> Self {
        self.is_visible = value;
        self
    }

    /// Sets the unselected resting foreground.
    pub fn foreground(mut self, value: Color) -> Self {
        self.foreground = Some(value);
        self
    }

    /// Overrides the automatic string header tooltip.
    pub fn tool_tip(mut self, value: ToolTip) -> Self {
        self.tool_tip = Some(value);
        self
    }

    /// Receives close requests after the parent's event.
    pub fn close_requested(mut self, callback: Listener) -> Self {
        self.close_requested = Some(callback);
        self
    }
}

/// Source bottom border states for the item beside the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TabBottomBorderState {
    /// Draws the continuous resting border.
    Normal,

    /// Draws the arc joining the selected tab on the right.
    LeftOfSelectedTab,

    /// Draws the arc joining the selected tab on the left.
    RightOfSelectedTab,

    /// Hides the border while selected or dragging.
    NoBottomBorderLine,
}

/// Reports pointer entry and exit to the owning strip.
type TabViewItemHoverChanged = Rc<dyn Fn(&mut App, bool)>;

/// The realized header container; selection and item lifetime belong to its parent TabView.
#[derive(Clone)]
pub(crate) struct TabViewItemHeader {
    /// Owner-supplied identity and visual content.
    pub item: TabViewItem,

    /// Whether this header represents the selected page.
    pub selected: bool,

    /// Controls compact icon-only presentation.
    pub width_mode: TabViewWidthMode,

    /// Controls close-button visibility.
    pub close_overlay_mode: TabViewCloseButtonOverlayMode,

    /// Position of this item relative to the selected tab.
    pub bottom_border: TabBottomBorderState,

    /// Whether the trailing separator is present in the resting state.
    pub separator_visible: bool,

    /// Header focus node retained by the owner strip.
    pub focus_node: AnyFocusNode,

    /// Close-button focus node retained by the owner strip.
    pub close_focus_node: AnyFocusNode,

    /// Requests selecting this item.
    pub select: Listener,

    /// Requests closing this item.
    pub close: Listener,

    /// Updates the strip’s hovered identity and width-deferral state.
    pub hover_changed: TabViewItemHoverChanged,
}

impl fmt::Debug for TabViewItemHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TabViewItemHeader")
            .field("item", &self.item)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for TabViewItemHeader {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let header = self.clone();
        let content = CommonStates::new(self.select.clone(), move |app, context, states| {
            header.template(app, context, states, true)
        })
        .is_enabled(self.item.is_enabled)
        .is_tab_stop(self.selected)
        .focus_node(self.focus_node);
        let enter = self.hover_changed.clone();
        let leave = self.hover_changed.clone();
        let close = self.close.clone();
        let mut content = GestureDetector::new().child(content);
        if self.item.is_enabled && self.item.is_closable {
            content = content.on_tertiary_tap_up(Rc::new(move |app, _| close.call(app)));
        }
        let content = MouseRegion::new()
            .on_enter(Rc::new(move |app, _| enter(app, true)))
            .on_exit(Rc::new(move |app, _| leave(app, false)))
            .child(content)
            .into_widget();
        let tooltip = self.item.tool_tip.clone().or_else(|| {
            self.item
                .header_text
                .as_ref()
                .filter(|text| !text.is_empty())
                .map(|text| ToolTip::text(text).placement(PlacementMode::Mouse))
        });
        if let Some(tooltip) = tooltip {
            ToolTipService::new(content, tooltip)
                .is_enabled(self.item.is_enabled)
                .into_widget()
        } else {
            content
        }
    }
}

impl TabViewItemHeader {
    /// UpdateCloseButton: OnPointerOver still shows the close button on a selected tab.
    fn close_visible(&self, states: ControlStates) -> bool {
        self.item.is_closable
            && (self.close_overlay_mode != TabViewCloseButtonOverlayMode::OnPointerOver
                || self.selected
                || matches!(
                    states.common,
                    CommonState::PointerOver | CommonState::Pressed
                ))
    }

    /// Builds LayoutRoot's named parts with the source visual-state resource substitutions.
    pub(crate) fn feedback(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut visual = self.clone();
        visual.item.header = self.item.header_text.as_ref().map_or_else(
            || SizedBox::new().into_widget(),
            |text| Text::new(text.clone()).into_widget(),
        );
        visual.item.icon_source = self.item.icon_source.as_ref().and_then(|icon| {
            downcast_widget::<FontIcon>(icon.as_ref()).map(|icon| {
                let mut icon = icon.clone();
                icon.key = None;
                icon.into_widget()
            })
        });
        visual.template(
            app,
            context,
            ControlStates {
                common: CommonState::Normal,
                focused: false,
            },
            false,
        )
    }

    /// Builds the source template for the current layout and visual state.
    fn template(
        &self,
        app: &mut App,
        context: BuildContext,
        states: ControlStates,
        interactive: bool,
    ) -> WidgetRef {
        let theme = ThemeResources::of(app, context);
        let r = &theme;
        let selected = self.selected;
        let compact = self.width_mode == TabViewWidthMode::Compact && !selected;
        let close_visible = self.close_visible(states);
        let (background, foreground, icon_foreground, close_background, close_foreground) =
            if !self.item.is_enabled {
                (
                    r.tab_view_item_header_background_disabled,
                    r.tab_view_item_header_foreground_disabled,
                    r.tab_view_button_foreground_disabled,
                    r.tab_view_item_header_disabled_close_button_background,
                    r.tab_view_item_header_disabled_close_button_foreground,
                )
            } else if selected {
                (
                    r.tab_view_item_header_background,
                    r.tab_view_item_header_foreground_selected,
                    r.tab_view_item_icon_foreground_selected,
                    r.tab_view_item_header_selected_close_button_background,
                    r.tab_view_item_header_selected_close_button_foreground,
                )
            } else {
                match states.common {
                    CommonState::PointerOver => (
                        r.tab_view_item_header_background_pointer_over,
                        r.tab_view_item_header_foreground_pointer_over,
                        r.tab_view_item_icon_foreground_pointer_over,
                        r.tab_view_item_header_pointer_over_close_button_background,
                        r.tab_view_item_header_pointer_over_close_button_foreground,
                    ),
                    CommonState::Pressed => (
                        r.tab_view_item_header_background_pressed,
                        r.tab_view_item_header_foreground_pressed,
                        r.tab_view_item_icon_foreground_pressed,
                        r.tab_view_item_header_pressed_close_button_background,
                        r.tab_view_item_header_pressed_close_button_foreground,
                    ),
                    _ => (
                        r.tab_view_item_header_background,
                        self.item
                            .foreground
                            .unwrap_or(r.tab_view_item_header_foreground),
                        self.item
                            .foreground
                            .unwrap_or(r.tab_view_item_icon_foreground),
                        r.tab_view_item_header_close_button_background,
                        r.tab_view_item_header_close_button_foreground,
                    ),
                }
            };
        let mut children = Vec::new();
        if let Some(icon) = &self.item.icon_source {
            // IconBox / IconControl, independently coloured from ContentPresenter.
            let icon = SizedBox::new()
                .width(r.tab_view_item_header_icon_size)
                .height(r.tab_view_item_header_icon_size)
                .child(FittedBox::new().child(DefaultTextStyle::new(
                    control_text_style(
                        &theme,
                        r.tab_view_item_header_icon_size,
                        FontWeight::NORMAL,
                        icon_foreground,
                    ),
                    icon.clone(),
                )));
            children.push(
                GridCell::new(
                    Center::new().child(
                        Padding::new(EdgeInsetsGeometry::only(
                            0.0,
                            0.0,
                            if compact {
                                0.0
                            } else {
                                r.tab_view_item_header_icon_margin[2]
                            },
                            0.0,
                        ))
                        .child(icon),
                    ),
                )
                .column(0)
                .into_widget(),
            );
        }
        if !compact {
            // ContentPresenter never implicitly binds to the tab's page content.
            children.push(
                GridCell::new(
                    Align::new().alignment(Alignment::CENTER_LEFT.into()).child(
                        DefaultTextStyle::new(
                            control_text_style(
                                &theme,
                                r.tab_view_item_header_font_size,
                                if selected {
                                    FontWeight::W600
                                } else {
                                    FontWeight::NORMAL
                                },
                                foreground,
                            ),
                            self.item.header.clone(),
                        )
                        .max_lines(1)
                        .overflow(inset_painting::TextOverflow::Ellipsis),
                    ),
                )
                .column(1)
                .into_widget(),
            );
        }
        if close_visible {
            let r2 = r.clone();
            let theme_kind = theme.theme;
            let close = if interactive {
                Button::new(
                    FontIcon::symbol(FluentSymbol::Dismiss)
                        .font_size(r.tab_view_item_header_close_font_size),
                    self.close.clone(),
                )
                .is_enabled(self.item.is_enabled)
                .is_tab_stop(false)
                .focus_node(self.close_focus_node)
                .template(move |_, _, states, content| {
                    close_button_template(
                        &r2,
                        theme_kind,
                        states,
                        content,
                        close_background,
                        close_foreground,
                    )
                })
                .into_widget()
            } else {
                close_button_template(
                    r,
                    theme_kind,
                    ControlStates {
                        common: CommonState::Normal,
                        focused: false,
                    },
                    FontIcon::symbol(FluentSymbol::Dismiss)
                        .font_size(r.tab_view_item_header_close_font_size)
                        .into_widget(),
                    close_background,
                    close_foreground,
                )
            };
            children.push(
                GridCell::new(
                    Padding::new(EdgeInsetsGeometry::only(
                        r.tab_view_item_header_close_margin[0],
                        0.0,
                        0.0,
                        0.0,
                    ))
                    .child(if interactive {
                        ToolTipService::new(close, ToolTip::text("Close tab (Ctrl+F4)"))
                            .into_widget()
                    } else {
                        close
                    }),
                )
                .column(2)
                .into_widget(),
            );
        }
        let grid = Grid::new()
            .column_definitions([
                ColumnDefinition::new(if compact {
                    GridLength::pixel(r.tab_view_item_header_icon_size)
                } else {
                    GridLength::AUTO
                }),
                ColumnDefinition::new(GridLength::star(1.0)),
                ColumnDefinition::new(GridLength::AUTO),
            ])
            .children(children);
        let border = if selected {
            Brush::VerticalElevation {
                stops: r.tab_view_selected_item_border_brush,
                length: 4.0,
            }
        } else {
            Brush::Solid(r.tab_view_item_border_brush)
        };
        // TabContainer: top corners only; the selected background supplies the lower outward curves.
        let content = ControlBorder::new(Brush::Solid(background), border)
            .border_thickness_ltrb(if selected {
                r.tab_view_selected_item_border_thickness
            } else {
                r.tab_view_item_border_thickness
            })
            .corner_radius_corners([
                r.overlay_corner_radius[0],
                r.overlay_corner_radius[1],
                0.0,
                0.0,
            ])
            .padding(if close_visible {
                r.tab_view_item_header_padding_with_close_button
            } else {
                r.tab_view_item_header_padding_without_close_button
            })
            .child(grid);
        let content = Margin::new(
            if selected {
                r.tab_view_selected_item_header_margin
            } else {
                [0.0; 4]
            },
            content,
        );
        let content =
            ConstrainedBox::new(BoxConstraints::new().min_height(r.tab_view_item_min_height))
                .child(content);
        let content = CustomPaint::new()
            .painter(TabChromePainter {
                selected,
                border_state: self.bottom_border,
                separator: self.separator_visible
                    && !selected
                    && !matches!(
                        states.common,
                        CommonState::PointerOver | CommonState::Pressed
                    ),
                selected_fill: r.tab_view_item_header_background_selected,
                border: r.tab_view_border_brush,
                separator_color: r.tab_view_item_separator,
                separator_margin: r.tab_view_item_separator_margin,
                selected_corner_radius: r.overlay_corner_radius,
            })
            .child(content);
        FocusVisual::new(content, theme.theme)
            .visible(states.focused)
            .margin([-3.0; 4])
            .corner_radius(r.overlay_corner_radius[0])
            .into_widget()
    }
}

/// TabViewCloseButtonStyle receives the parent item's rest brushes through template bindings.
/// Builds the source close-button chrome independently of its interaction wrapper.
fn close_button_template(
    r: &ThemeResources,
    theme: Theme,
    states: ControlStates,
    content: WidgetRef,
    background: Color,
    foreground: Color,
) -> WidgetRef {
    let (background, foreground, border) = match states.common {
        CommonState::PointerOver => (
            r.tab_view_item_header_close_button_background_pointer_over,
            r.tab_view_item_header_close_button_foreground_pointer_over,
            r.tab_view_item_header_close_button_border_brush_pointer_over,
        ),
        CommonState::Pressed => (
            r.tab_view_item_header_close_button_background_pressed,
            r.tab_view_item_header_close_button_foreground_pressed,
            r.tab_view_item_header_close_button_border_brush_pressed,
        ),
        CommonState::Disabled => (
            background,
            foreground,
            r.tab_view_item_header_close_button_border_brush_disabled,
        ),
        CommonState::Normal => (
            background,
            foreground,
            r.tab_view_item_header_close_button_border_brush,
        ),
    };
    let content = SizedBox::new()
        .width(r.tab_view_item_header_close_button_width)
        .height(r.tab_view_item_header_close_button_height)
        .child(
            ControlBorder::new(Brush::Solid(background), Brush::Solid(border))
                .border_thickness_ltrb(r.tab_view_item_header_close_button_border_thickness)
                .corner_radius(r.control_corner_radius[0])
                .child(Center::new().child(DefaultTextStyle::new(
                    control_text_style(
                        r,
                        r.tab_view_item_header_close_font_size,
                        FontWeight::NORMAL,
                        foreground,
                    ),
                    content,
                ))),
        );
    FocusVisual::new(content, theme)
        .visible(states.focused)
        .margin([-3.0; 4])
        .corner_radius(r.control_corner_radius[0])
        .into_widget()
}

/// LayoutRoot's bottom line, separator, selected background and outward corner arcs.
/// Draws the source selected geometry, adjacent arcs and trailing separator.
#[derive(Clone, Debug, PartialEq)]
struct TabChromePainter {
    /// Whether to draw the selected background geometry.
    selected: bool,

    /// Controls the source bottom-border and arc state.
    border_state: TabBottomBorderState,

    /// Whether to draw the trailing vertical separator.
    separator: bool,

    /// Theme brush for the selected background.
    selected_fill: Color,

    /// Theme brush for the lower border and radius arcs.
    border: Color,

    /// Theme brush for the trailing separator.
    separator_color: Color,

    /// `TabViewItemSeparatorMargin`: the separator's inset in left, top, right, bottom order.
    separator_margin: [f64; 4],

    /// `OverlayCornerRadius`: the selected background's top corners.
    selected_corner_radius: [f64; 4],
}

impl CustomPainter for TabChromePainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let (w, h) = (size.width() as f32, size.height() as f32);
        if self.selected {
            canvas.draw_path(
                &selected_tab_geometry(self.selected_corner_radius, w, h),
                FillRule::NonZero,
                &Paint::from_color(self.selected_fill.into()),
            );
            // LeftRadiusRenderArc and RightRadiusRenderArc: literal source cubic paths.
            let mut left = PathBuilder::new();
            left.move_to((0.0, h - 4.0))
                .cubic_to(
                    (0.0, h - 2.80531),
                    (-0.52376, h - 1.73294),
                    (-1.35418, h - 1.0),
                )
                .line_to((-4.0, h - 1.0))
                .cubic_to((-2.34315, h - 1.0), (-1.0, h - 2.34315), (-1.0, h - 4.0))
                .line_to((0.0, h - 4.0))
                .close();
            canvas.draw_path(
                &left.build(),
                FillRule::NonZero,
                &Paint::from_color(self.border.into()),
            );
            let mut right = PathBuilder::new();
            right
                .move_to((w, h - 4.0))
                .cubic_to(
                    (w, h - 2.80531),
                    (w + 0.523755, h - 1.73294),
                    (w + 1.35418, h - 1.0),
                )
                .line_to((w + 4.0, h - 1.0))
                .cubic_to(
                    (w + 2.34315, h - 1.0),
                    (w + 1.0, h - 2.34315),
                    (w + 1.0, h - 4.0),
                )
                .line_to((w, h - 4.0))
                .close();
            canvas.draw_path(
                &right.build(),
                FillRule::NonZero,
                &Paint::from_color(self.border.into()),
            );
        } else if self.border_state != TabBottomBorderState::NoBottomBorderLine {
            let left = if self.border_state == TabBottomBorderState::RightOfSelectedTab {
                2.0
            } else {
                0.0
            };
            let right = if self.border_state == TabBottomBorderState::LeftOfSelectedTab {
                2.0
            } else {
                0.0
            };
            canvas.draw_rect(
                inset_embedder::valo::Rect::new(left, h - 1.0, w - left - right, 1.0),
                &Paint::from_color(self.border.into()),
            );
        }
        if self.separator {
            canvas.draw_rect(
                inset_embedder::valo::Rect::new(
                    w - 1.0,
                    self.separator_margin[1] as f32,
                    1.0,
                    (h - self.separator_margin[1] as f32 - self.separator_margin[3] as f32)
                        .max(0.0),
                ),
                &Paint::from_color(self.separator_color.into()),
            );
        }
    }

    fn should_repaint(&self, _app: &App, old: &dyn CustomPainter) -> bool {
        old.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// UpdateTabGeometry: four-pixel outward lower corners and the theme's upper radii.
fn selected_tab_geometry(corner_radius: [f64; 4], width: f32, height: f32) -> std::sync::Arc<Path> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let left = corner_radius[0] as f32;
    let right = corner_radius[1] as f32;
    let mut path = PathBuilder::new();
    path.move_to((-4.0, height))
        .arc((-4.0, height - 4.0), 4.0, FRAC_PI_2, -FRAC_PI_2)
        .line_to((0.0, left))
        .arc((left, left), left, PI, FRAC_PI_2)
        .line_to((width - right, 0.0))
        .arc((width - right, right), right, -FRAC_PI_2, FRAC_PI_2)
        .line_to((width, height - 4.0))
        .arc((width + 4.0, height - 4.0), 4.0, PI, -FRAC_PI_2)
        .close();
    path.build()
}
