//! NavigationView item descriptors and the three source presenter templates.

use crate::*;
use inset_animation::{Cubic, Interval};
use inset_embedder::{Color, FontWeight};
use inset_foundation::{App, Listener};
use inset_painting::Alignment;
use inset_rendering::{BoxConstraints, HitTestBehavior};
use inset_widgets::*;
use std::{fmt, rc::Rc, time::Duration};

/// The three container types accepted by NavigationView's item factory.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigationViewItemKind {
    /// An invokable and optionally selectable entry.
    #[default]
    Item,

    /// A noninteractive section heading.
    Header,

    /// A noninteractive dividing line.
    Separator,
}

/// Stable item data; the parent owns selection, expansion and focus across reparenting.
#[derive(Clone)]
pub struct NavigationViewItem {
    /// Identity that remains unchanged when an item moves or enters a flyout.
    pub id: String,

    /// The source container class to present.
    pub kind: NavigationViewItemKind,

    /// The content widget shown beside the icon.
    pub content: WidgetRef,

    /// Whether Content is non-null; false selects the icon-only template state.
    pub has_content: bool,

    /// Original string content used for the suggested compact tooltip.
    pub text: Option<String>,

    /// Optional icon shown in the presenter's icon column.
    pub icon: Option<WidgetRef>,

    /// Eager child descriptors in their source order.
    pub menu_items: Vec<NavigationViewItem>,

    /// Whether invocation changes selection as well as raising ItemInvoked.
    pub selects_on_invoked: bool,

    /// Whether pointer and keyboard input are accepted.
    pub is_enabled: bool,

    /// Whether the item participates in layout and navigation.
    pub is_visible: bool,

    /// Content for the template's InfoBadgePresenter.
    pub info_badge: Option<WidgetRef>,

    /// Allows the Expanding callback to populate children on demand.
    pub has_unrealized_children: bool,

    /// Explicit tooltip content overriding the suggested string label.
    pub tool_tip: Option<ToolTip>,
}

impl fmt::Debug for NavigationViewItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NavigationViewItem")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

impl NavigationViewItem {
    /// Creates an entry with a stable identity and arbitrary content.
    pub fn new<K>(id: impl Into<String>, content: impl IntoWidget<K>) -> Self {
        Self {
            id: id.into(),
            kind: NavigationViewItemKind::Item,
            content: content.into_widget(),
            has_content: true,
            text: None,
            icon: None,
            menu_items: vec![],
            selects_on_invoked: true,
            is_enabled: true,
            is_visible: true,
            info_badge: None,
            has_unrealized_children: false,
            tool_tip: None,
        }
    }

    /// Clears Content while preserving a stable descriptor identity.
    pub fn without_content(mut self) -> Self {
        self.has_content = false;
        self.content = SizedBox::shrink().into_widget();
        self.text = None;
        self
    }

    /// Creates string content and retains its suggested tooltip.
    pub fn text(id: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        let mut item = Self::new(id, Text::new(text.clone()));
        item.text = Some(text);
        item
    }

    /// Creates a noninteractive section heading.
    pub fn header(id: impl Into<String>, text: impl Into<String>) -> Self {
        let mut item = Self::text(id, text);
        item.kind = NavigationViewItemKind::Header;
        item.is_enabled = false;
        item.selects_on_invoked = false;
        item
    }

    /// Creates a noninteractive section separator.
    pub fn separator(id: impl Into<String>) -> Self {
        let mut item = Self::new(id, SizedBox::shrink());
        item.kind = NavigationViewItemKind::Separator;
        item.is_enabled = false;
        item.selects_on_invoked = false;
        item
    }

    /// Sets the icon column's content.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> Self {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Sets the ordered child collection.
    pub fn menu_items(mut self, items: impl IntoIterator<Item = Self>) -> Self {
        self.menu_items = items.into_iter().collect();
        self
    }

    /// Sets whether invocation also changes selection.
    pub fn selects_on_invoked(mut self, value: bool) -> Self {
        self.selects_on_invoked = value;
        self
    }

    /// Sets whether the item accepts input.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Sets whether the item participates in layout.
    pub fn is_visible(mut self, value: bool) -> Self {
        self.is_visible = value;
        self
    }

    /// Sets the badge presenter content.
    pub fn info_badge<K>(mut self, badge: impl IntoWidget<K>) -> Self {
        self.info_badge = Some(badge.into_widget());
        self
    }

    /// Overrides the automatic suggested tooltip for this item.
    pub fn tool_tip(mut self, tool_tip: ToolTip) -> Self {
        self.tool_tip = Some(tool_tip);
        self
    }

    /// Enables expansion before children have been supplied.
    pub fn has_unrealized_children(mut self, value: bool) -> Self {
        self.has_unrealized_children = value;
        self
    }
}

/// Source repeater position; top footer shares primary presentation with distinct hierarchy behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NavigationViewItemPosition {
    Left,
    TopPrimary,
    /// Top footer shares primary presentation but does not open its children in a flyout.
    TopFooter,
    TopOverflow,
}

/// Visual inputs supplied by NavigationView; no hierarchy state lives in this widget.
#[derive(Clone, Debug)]
pub(crate) struct NavigationViewItemPresenter {
    /// Descriptor whose content is retained across presenter rebuilds.
    pub item: NavigationViewItem,
    /// Repeater position, selecting its template and hierarchy behavior.
    pub position: NavigationViewItemPosition,
    /// Whether this item owns selection.
    pub is_selected: bool,
    /// Whether the parent currently exposes children.
    pub is_expanded: bool,
    /// The containing SplitView is closed in a compact mode.
    pub is_closed_compact: bool,
    /// Membership in a root menu/footer repeater, independent of flyout indentation.
    pub is_top_level_item: bool,
    /// Parent-provided focus highlight, combined with native CommonStates.
    pub is_keyboard_focused: bool,
    /// Whether a descendant owns selection.
    pub is_child_selected: bool,
    /// Owner-managed focus node used for hierarchy traversal.
    pub focus_node: AnyFocusNode,
    /// Inline depth; reset to zero when children enter a flyout.
    pub depth: usize,
    /// Supplies TemplateSettings.SmallerIconWidth.
    pub compact_pane_length: f64,
    /// Activates the item without taking ownership of selection.
    pub on_invoked: Listener,
    /// Chevron activation changes expansion independently of invocation.
    pub on_expansion_toggled: Listener,

    /// Stable source SelectionIndicator geometry for the parent transition.
    pub indicator_key: Rc<GlobalKey>,

    /// Shared source clock, retained by the parent for the lifetime of this visual.
    pub indicator_controller: inset_foundation::Handle<inset_animation::AnimationController>,

    /// Local compositor animation of this actual indicator visual.
    pub indicator_animation: Option<NavigationIndicatorTransition>,

    /// Parent-coordinated opacity while the next visual receives its layout.
    pub indicator_opacity: Option<f64>,
}

impl StatelessWidget for NavigationViewItemPresenter {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        if !self.item.is_visible {
            return SizedBox::shrink().into_widget();
        }
        if self.item.kind != NavigationViewItemKind::Item {
            return self.section_template(&ThemeResources::of(app, context));
        }
        let w = self.clone();
        let content = CommonStates::new(self.on_invoked.clone(), move |app, context, states| {
            let resources = ThemeResources::of(app, context);
            FocusVisual::new(w.template(&resources, states.common), resources.theme)
                .visible(states.focused || w.is_keyboard_focused)
                .corner_radius(resources.control_corner_radius[0])
                .into_widget()
        })
        .is_enabled(self.item.is_enabled)
        .focus_node(self.focus_node)
        .into_widget();
        let tooltip = self.item.tool_tip.clone().or_else(|| {
            (self.is_closed_compact && self.position == NavigationViewItemPosition::Left)
                .then(|| self.item.text.as_ref().map(ToolTip::text))
                .flatten()
        });
        if let Some(tooltip) = tooltip {
            return ToolTipService::new(content, tooltip)
                .is_enabled(self.item.is_enabled)
                .into_widget();
        }
        content
    }
}

impl NavigationViewItemPresenter {
    /// Header and separator templates have no input or focus handlers.
    fn section_template(&self, r: &ThemeResources) -> WidgetRef {
        let top = matches!(
            self.position,
            NavigationViewItemPosition::TopPrimary | NavigationViewItemPosition::TopFooter
        );
        if self.item.kind == NavigationViewItemKind::Header {
            let collapsed = self.is_closed_compact && self.is_top_level_item;
            let mut margin = if self.position != NavigationViewItemPosition::Left {
                r.top_navigation_view_item_inner_header_margin
            } else {
                r.navigation_view_item_inner_header_margin
            };
            margin[0] += self.depth as f64 * 31.0;
            // InnerHeaderGrid changes height immediately; HeaderText has the source's two-phase fade.
            let text = AnimatedOpacity::new(
                if collapsed { 0.0 } else { 1.0 },
                Duration::from_millis(200),
            )
            .curve(Rc::new(Interval::new(
                if collapsed { 0.0 } else { 0.5 },
                if collapsed { 0.5 } else { 1.0 },
                Rc::new(Cubic::new(0.0, 0.35, 0.15, 1.0)),
            )))
            .child(Margin::new(
                [0.0, -1.0, 0.0, -1.0],
                DefaultTextStyle::new(
                    control_text_style(
                        r,
                        14.0,
                        FontWeight::W600,
                        r.navigation_view_item_header_foreground,
                    ),
                    self.item.content.clone(),
                )
                .soft_wrap(false),
            ));
            return PresenterGrid::new()
                .height(if collapsed { 0.0 } else { 40.0 })
                .margin(margin)
                .child(
                    Align::new()
                        .alignment(Alignment::CENTER_LEFT.into())
                        .child(text),
                )
                .into_widget();
        }
        let line = PresenterGrid::new().background(Brush::Solid(if top {
            r.top_navigation_view_item_separator_foreground
        } else {
            r.navigation_view_item_separator_foreground
        }));
        if top {
            line.width(r.top_navigation_view_item_separator_width)
                .height(24.0)
                .margin(r.top_navigation_view_item_separator_margin)
                .into_widget()
        } else {
            let mut margin = if self.is_closed_compact {
                r.navigation_view_compact_item_separator_margin
            } else {
                r.navigation_view_item_separator_margin
            };
            margin[0] += self.depth as f64 * 31.0;
            line.height(r.navigation_view_item_separator_height)
                .margin(margin)
                .into_widget()
        }
    }

    /// Transcribes LayoutRoot, ContentGrid and the separate selection-indicator layer.
    fn template(&self, r: &ThemeResources, state: CommonState) -> WidgetRef {
        let top = matches!(
            self.position,
            NavigationViewItemPosition::TopPrimary | NavigationViewItemPosition::TopFooter
        );
        let overflow = self.position == NavigationViewItemPosition::TopOverflow;
        let (background, mut foreground) = item_colors(
            r,
            top,
            self.is_selected,
            if state == CommonState::Disabled {
                CommonState::Normal
            } else {
                state
            },
        );
        if state == CommonState::Disabled && (top || overflow) {
            foreground = r.top_navigation_view_item_foreground_disabled;
        }
        let compact = self.is_closed_compact && !top && !overflow && self.is_top_level_item;
        let show_chevron =
            (!self.item.menu_items.is_empty() || self.item.has_unrealized_children) && !compact;
        let icon_width = if top && !self.item.has_content {
            36.0
        } else if top {
            28.0
        } else if overflow {
            32.0
        } else if self.item.icon.is_none() {
            8.0
        } else {
            (self.compact_pane_length - 8.0).max(0.0)
        };
        let icon = self
            .item
            .icon
            .clone()
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        // IconBox scales the unconstrained content uniformly, as the source Viewbox does.
        let mut icon_box = SizedBox::new().height(16.0);
        if (top && self.item.has_content) || overflow {
            icon_box = icon_box.width(16.0);
        }
        let icon = SizedBox::new().width(icon_width).child(
            Align::new()
                .alignment(if (top && self.item.has_content) || overflow {
                    Alignment::CENTER_RIGHT.into()
                } else {
                    Alignment::CENTER.into()
                })
                .child(icon_box.child(FittedBox::new().child(icon))),
        );
        let margin = if compact {
            r.navigation_view_compact_item_content_presenter_margin
        } else if top {
            if self.item.icon.is_some() {
                r.top_navigation_view_item_content_presenter_margin
            } else {
                r.top_navigation_view_item_content_only_content_presenter_margin
            }
        } else if overflow {
            if self.item.icon.is_some() {
                r.top_navigation_view_item_on_overflow_content_presenter_margin
            } else {
                r.top_navigation_view_item_on_overflow_no_icon_content_presenter_margin
            }
        } else {
            r.navigation_view_item_content_presenter_margin
        };
        let mut cells = vec![];
        if self.item.icon.is_some() || (!top && !overflow) {
            cells.push(GridCell::new(icon).column(0));
        }
        if self.item.has_content {
            cells.push(
                GridCell::new(
                    PresenterGrid::new().margin(margin).child(
                        Align::new()
                            .alignment(Alignment::CENTER_LEFT.into())
                            .child(self.item.content.clone()),
                    ),
                )
                .column(1),
            );
        }
        if let Some(badge) = &self.item.info_badge {
            let badge = Margin::new(
                if compact {
                    [0.0, 2.0, 2.0, 0.0]
                } else if top {
                    [-16.0, 0.0, 2.0, 13.0]
                } else {
                    [0.0; 4]
                },
                Align::new()
                    .alignment(if compact {
                        Alignment::TOP_RIGHT.into()
                    } else {
                        Alignment::CENTER.into()
                    })
                    .child(badge.clone()),
            );
            cells.push(
                GridCell::new(badge)
                    .column(if compact {
                        0
                    } else if top {
                        3
                    } else {
                        2
                    })
                    .column_span(if compact { 4 } else { 1 }),
            );
        }
        if show_chevron {
            let callback = self.on_expansion_toggled.clone();
            let enabled = self.item.is_enabled;
            let glyph = FontIcon::symbol(FluentSymbol::ChevronDown)
                .font_size(r.navigation_view_item_expanded_glyph_font_size);
            let glyph = AnimatedRotation::new(
                if self.is_expanded { 0.5 } else { 0.0 },
                r.control_normal_animation_duration,
            )
            .child(glyph);
            let chevron = GestureDetector::new()
                .behavior(HitTestBehavior::Opaque)
                .on_tap(Listener::new(move |app| {
                    if enabled {
                        callback.call(app);
                    }
                }))
                .child(
                    SizedBox::new()
                        .width(40.0)
                        .child(Center::new().child(glyph)),
                );
            cells.push(
                GridCell::new(
                    PresenterGrid::new()
                        .margin(if top {
                            if !self.item.has_content {
                                r.top_navigation_view_item_icon_only_expand_chevron_margin
                            } else if self.item.icon.is_some() {
                                r.top_navigation_view_item_expand_chevron_margin
                            } else {
                                r.top_navigation_view_item_content_only_expand_chevron_margin
                            }
                        } else if overflow {
                            r.top_navigation_view_item_on_overflow_expand_chevron_margin
                        } else {
                            r.navigation_view_item_expand_chevron_margin
                        })
                        .child(chevron),
                )
                .column(if top { 2 } else { 3 }),
            );
        }
        let content = PresenterGrid::new()
            .columns([
                ColumnDefinition::new(GridLength::AUTO),
                if top {
                    ColumnDefinition::new(GridLength::AUTO)
                } else {
                    ColumnDefinition::new(GridLength::star(1.0))
                },
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::AUTO),
            ])
            .min_height(36.0)
            .margin([
                self.depth as f64 * 31.0,
                0.0,
                if top || compact { 0.0 } else { 14.0 },
                0.0,
            ])
            .cells(cells);
        let mut layers = vec![content.into_widget()];
        {
            let indicator = PresenterGrid::new()
                .background(Brush::Solid(
                    r.navigation_view_selection_indicator_foreground,
                ))
                .corner_radius(r.navigation_view_selection_indicator_radius)
                .width(if top {
                    16.0
                } else if overflow {
                    2.0
                } else {
                    r.navigation_view_selection_indicator_width
                })
                .height(if top {
                    3.0
                } else if overflow {
                    24.0
                } else {
                    r.navigation_view_selection_indicator_height
                });
            let visible = self.is_selected || (self.is_child_selected && !self.is_expanded);
            let indicator = NavigationIndicatorTransition::build(
                self.indicator_controller,
                self.indicator_animation.clone(),
                self.indicator_opacity
                    .unwrap_or(if visible { 1.0 } else { 0.0 }),
                indicator.into_widget(),
            );
            let indicator = KeyedSubtree::new(indicator).key(self.indicator_key.clone());
            layers.push(
                Positioned::fill(
                    IgnorePointer::new().child(
                        Align::new()
                            .alignment(if top {
                                Alignment::BOTTOM_CENTER.into()
                            } else {
                                Alignment::CENTER_LEFT.into()
                            })
                            .child(
                                PresenterGrid::new()
                                    .margin(if top {
                                        if !self.item.has_content {
                                            [0.0; 4]
                                        } else if self.item.icon.is_none() {
                                            [12.0, 0.0, 12.0, 4.0]
                                        } else {
                                            [16.0, 0.0, 16.0, 4.0]
                                        }
                                    } else if overflow {
                                        [4.0, 0.0, 0.0, 0.0]
                                    } else {
                                        [0.0; 4]
                                    })
                                    .child(indicator),
                            ),
                    ),
                )
                .into_widget(),
            );
        }
        let root = PresenterGrid::new()
            .background(Brush::Solid(background))
            .corner_radius(r.control_corner_radius[0])
            .margin(if overflow {
                [0.0; 4]
            } else if top && !self.item.has_content {
                [2.0; 4]
            } else {
                r.navigation_view_item_button_margin
            })
            .child(Stack::new().children(layers));
        DefaultTextStyle::new(
            control_text_style(
                r,
                r.control_content_theme_font_size,
                FontWeight::W400,
                foreground,
            ),
            Opacity::new(if !top && !overflow && state == CommonState::Disabled {
                r.list_view_item_disabled_theme_opacity
            } else {
                1.0
            })
            .child(root),
        )
        .soft_wrap(false)
        .into_widget()
    }
}

/// CommonStates colors from the left and top-primary presenter visual states.
fn item_colors(
    r: &ThemeResources,
    top: bool,
    selected: bool,
    state: CommonState,
) -> (Color, Color) {
    use CommonState::*;
    if top {
        match (selected, state) {
            (_, Disabled) => (
                r.navigation_view_item_background,
                r.top_navigation_view_item_foreground_disabled,
            ),
            (true, Pressed) => (
                r.top_navigation_view_item_background_selected_pressed,
                r.top_navigation_view_item_foreground_selected_pressed,
            ),
            (true, PointerOver) => (
                r.top_navigation_view_item_background_selected_pointer_over,
                r.top_navigation_view_item_foreground_selected_pointer_over,
            ),
            (true, Normal) => (
                r.top_navigation_view_item_background_selected,
                r.top_navigation_view_item_foreground_selected,
            ),
            (false, Pressed) => (
                r.top_navigation_view_item_background_pressed,
                r.top_navigation_view_item_foreground_pressed,
            ),
            (false, PointerOver) => (
                r.top_navigation_view_item_background_pointer_over,
                r.top_navigation_view_item_foreground_pointer_over,
            ),
            (false, Normal) => (
                r.navigation_view_item_background,
                r.top_navigation_view_item_foreground,
            ),
        }
    } else {
        match (selected, state) {
            (_, Disabled) => (
                r.navigation_view_item_background_disabled,
                r.navigation_view_item_foreground_disabled,
            ),
            (true, Pressed) => (
                r.navigation_view_item_background_selected_pressed,
                r.navigation_view_item_foreground_selected_pressed,
            ),
            (true, PointerOver) => (
                r.navigation_view_item_background_selected_pointer_over,
                r.navigation_view_item_foreground_selected_pointer_over,
            ),
            (true, Normal) => (
                r.navigation_view_item_background_selected,
                r.navigation_view_item_foreground_selected,
            ),
            (false, Pressed) => (
                r.navigation_view_item_background_pressed,
                r.navigation_view_item_foreground_pressed,
            ),
            (false, PointerOver) => (
                r.navigation_view_item_background_pointer_over,
                r.navigation_view_item_foreground_pointer_over,
            ),
            (false, Normal) => (
                r.navigation_view_item_background,
                r.navigation_view_item_foreground,
            ),
        }
    }
}

/// Local XAML FrameworkElement sizing around the shared Grid panel.
#[derive(Debug, Default)]
struct PresenterGrid {
    children: Vec<WidgetRef>,
    columns: Vec<ColumnDefinition>,
    width: Option<f64>,
    height: Option<f64>,
    minimum_height: f64,
    margin: [f64; 4],
    background: Option<Brush>,
    radius: f64,
}

impl PresenterGrid {
    fn new() -> Self {
        Self::default()
    }
    fn width(mut self, value: f64) -> Self {
        self.width = Some(value);
        self
    }
    fn height(mut self, value: f64) -> Self {
        self.height = Some(value);
        self
    }
    fn min_height(mut self, value: f64) -> Self {
        self.minimum_height = value;
        self
    }
    fn margin(mut self, value: [f64; 4]) -> Self {
        self.margin = value;
        self
    }
    fn background(mut self, value: Brush) -> Self {
        self.background = Some(value);
        self
    }
    fn corner_radius(mut self, value: f64) -> Self {
        self.radius = value;
        self
    }
    fn columns(mut self, value: impl IntoIterator<Item = ColumnDefinition>) -> Self {
        self.columns = value.into_iter().collect();
        self
    }
    fn cells(mut self, value: Vec<GridCell>) -> Self {
        self.children = value.into_iter().map(IntoWidget::into_widget).collect();
        self
    }
    fn child<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.children = vec![value.into_widget()];
        self
    }
}

impl StatelessWidget for PresenterGrid {
    fn build(&self, _: &mut App, _: BuildContext) -> WidgetRef {
        let mut grid = Grid::new()
            .column_definitions(self.columns.clone())
            .children(self.children.clone())
            .corner_radius(self.radius);
        grid.background = self.background;
        let mut outer = Container::new()
            .constraints(BoxConstraints::new().min_height(self.minimum_height))
            .child(grid);
        if let Some(width) = self.width {
            outer = outer.width(width);
        }
        if let Some(height) = self.height {
            outer = outer.height(height);
        }
        Margin::new(self.margin, outer).into_widget()
    }
}
