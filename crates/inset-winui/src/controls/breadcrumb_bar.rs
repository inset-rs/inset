//! BreadcrumbBar.cpp, BreadcrumbBarItem.cpp and BreadcrumbBar.xaml.

use crate::FocusState;
use crate::*;
use inset_embedder::{FontWeight, Size, TextDirection};
use inset_foundation::{App, Handle, Listener, ValueKey};
use inset_painting::EdgeInsetsGeometry;
use inset_rendering::{CrossAxisAlignment, MainAxisSize};
use inset_services::{KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

/// Item data and the content used by its inline and overflow containers.
#[derive(Clone, Debug)]
pub struct BreadcrumbBarItem {
    /// Unique stable identity, independent of display order.
    pub id: String,

    /// Content description; inline and overflow presentations have separate widget State.
    pub content: WidgetRef,

    /// Whether this item accepts keyboard and pointer activation.
    pub is_enabled: bool,
}

impl BreadcrumbBarItem {
    /// Creates an item with arbitrary widget content.
    pub fn new<K>(id: impl Into<String>, content: impl IntoWidget<K>) -> Self {
        Self {
            id: id.into(),
            content: content.into_widget(),
            is_enabled: true,
        }
    }

    /// Creates an item whose text inherits the source template's foreground.
    pub fn text(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(id, Text::new(text.into()).soft_wrap(false))
    }

    /// Sets IsEnabled on both presentations of the item.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }
}

/// ItemClicked identifies the original collection entry, including clicks in the reversed flyout.
#[derive(Clone, Debug)]
pub struct BreadcrumbBarItemClickedEventArgs {
    /// Zero-based index in ItemsSource.
    pub index: usize,

    /// Item description at the time it was invoked.
    pub item: BreadcrumbBarItem,
}

/// Handler for the original item and index selected by a breadcrumb invocation.
pub type BreadcrumbBarItemClickedHandler = Rc<dyn Fn(&mut App, BreadcrumbBarItemClickedEventArgs)>;

/// Application-owned navigation; clicking a breadcrumb does not edit its collection.
#[derive(Clone)]
pub struct BreadcrumbBar {
    /// ItemsSource in root-to-current order; the last item is the current destination.
    pub items_source: Vec<BreadcrumbBarItem>,

    /// ItemClicked callback; the application chooses whether to shorten the path.
    pub item_clicked: BreadcrumbBarItemClickedHandler,

    /// Enables all item containers.
    pub is_enabled: bool,

    /// Identity across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl std::fmt::Debug for BreadcrumbBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BreadcrumbBar")
            .field("items_source", &self.items_source)
            .finish()
    }
}

impl BreadcrumbBar {
    /// Creates a breadcrumb path and its ItemClicked handler.
    pub fn new(
        items_source: Vec<BreadcrumbBarItem>,
        item_clicked: impl Fn(&mut App, BreadcrumbBarItemClickedEventArgs) + 'static,
    ) -> Self {
        Self {
            items_source,
            item_clicked: Rc::new(item_clicked),
            is_enabled: true,
            key: None,
        }
    }

    /// Sets IsEnabled.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Sets widget identity.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }
}

/// Inline containers survive overflow; the flyout takes a snapshot when opened.
pub struct BreadcrumbBarState {
    /// Native widget state.
    state: StateData<BreadcrumbBar>,

    /// Live inline controls, keyed independently of their indices.
    items: HashMap<String, Handle<BreadcrumbItemState>>,

    /// Ellipsis control, which has no public collection index.
    ellipsis: Option<Handle<BreadcrumbItemState>>,

    /// Latest completed measure/arrange decisions.
    layout: BreadcrumbLayoutInfo,

    /// Source ellipsis Flyout.
    flyout: Option<Handle<Flyout>>,

    /// Source vertical overflow ScrollViewer.
    scroll: Option<Handle<ScrollViewportController>>,
}

impl StatefulWidget for BreadcrumbBar {
    type State = BreadcrumbBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> Self::State {
        BreadcrumbBarState {
            state: StateData::new(),
            items: HashMap::new(),
            ellipsis: None,
            layout: BreadcrumbLayoutInfo::default(),
            flyout: None,
            scroll: None,
        }
    }
}

impl BreadcrumbBarState {
    /// Layout information also chooses the single Tab entry, as OnGettingFocus does.
    fn arranged(self: Handle<Self>, app: &mut App, layout: BreadcrumbLayoutInfo) {
        app.get_mut(self).layout = layout;
        let widget = self.widget(app).clone();
        if let Some(ellipsis) = app.get(self).ellipsis {
            ellipsis.node(app).set_skip_traversal(app, !layout.ellipsis);
        }
        let first_enabled = widget.items_source.iter().position(|item| item.is_enabled);
        for (index, item) in widget.items_source.iter().enumerate() {
            if let Some(control) = app.get(self).items.get(&item.id).copied() {
                control
                    .node(app)
                    .set_skip_traversal(app, layout.ellipsis || Some(index) != first_enabled);
            }
        }
    }

    /// MoveFocusPrevious/Next skip the hidden prefix and stop at either end.
    fn move_focus(self: Handle<Self>, app: &mut App, from: Option<usize>, forward: bool) -> bool {
        let layout = app.get(self).layout;
        let widget = self.widget(app).clone();
        let mut visible = Vec::new();
        if layout.ellipsis {
            visible.push(None);
        }
        visible.extend((layout.first.saturating_sub(1)..widget.items_source.len()).map(Some));
        let Some(position) = visible.iter().position(|index| *index == from) else {
            return false;
        };
        let mut next = position as isize + if forward { 1 } else { -1 };
        while next >= 0 && (next as usize) < visible.len() {
            let control = match visible[next as usize] {
                None => app.get(self).ellipsis,
                Some(index) => app
                    .get(self)
                    .items
                    .get(&widget.items_source[index].id)
                    .copied(),
            };
            if let Some(control) = control
                && control.widget(app).enabled
            {
                control.request_focus(app, FocusState::Keyboard);
                return true;
            }
            next += if forward { 1 } else { -1 };
        }
        false
    }

    /// Source CloneEllipsisItemSource reverses hidden items and copies their collection indices.
    fn open(self: Handle<Self>, app: &mut App, target: BuildContext) {
        let count = app.get(self).layout.first.saturating_sub(1);
        let widget = self.widget(app).clone();
        let children: Vec<_> = widget
            .items_source
            .iter()
            .take(count)
            .enumerate()
            .rev()
            .map(|(index, item)| {
                BreadcrumbItemControl {
                    owner: self,
                    item: Some(item.clone()),
                    index: Some(index),
                    current: false,
                    dropdown: true,
                    enabled: widget.is_enabled && item.is_enabled,
                    key: Rc::new(ValueKey::new(item.id.clone())),
                }
                .into_widget()
            })
            .collect();
        let flyout = app.get(self).flyout.unwrap();
        flyout.content(
            app,
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .children(children),
        );
        app.get(self).scroll.unwrap().change_view(app, 0.0, true);
        flyout.show_at(app, target);
    }

    /// ItemClicked does not alter ItemsSource; the owner decides where navigation goes.
    fn invoke(
        self: Handle<Self>,
        app: &mut App,
        item: BreadcrumbBarItem,
        index: usize,
        dropdown: bool,
    ) {
        if dropdown {
            app.get(self).flyout.unwrap().hide(app);
        }
        let callback = self.widget(app).item_clicked.clone();
        callback(app, BreadcrumbBarItemClickedEventArgs { index, item });
    }
}

impl State for BreadcrumbBarState {
    type Widget = BreadcrumbBar;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        InputDevice::initialize(app);
        let scroll = ScrollViewportController::new(app);
        app.get_mut(self).scroll = Some(scroll);
        let flyout = Flyout::new(app, SizedBox::new());
        app.get_mut(self).flyout = Some(flyout);
        app.get_mut(flyout).placement = FlyoutPlacementMode::Bottom;
        app.get_mut(flyout).minimum_size = Some(Size::new(0.0, 40.0));
        app.get_mut(flyout).flyout_presenter_style = Some(Rc::new(move |app, context, content| {
            let r = ThemeResources::of(app, context);
            IntrinsicWidth::new()
                .child(
                    ControlBorder::new(
                        Brush::Acrylic(r.breadcrumb_bar_ellipsis_flyout_presenter_background),
                        Brush::Solid(r.breadcrumb_bar_ellipsis_flyout_presenter_border_brush),
                    )
                    .border_thickness_ltrb(
                        r.breadcrumb_bar_ellipsis_flyout_presenter_border_theme_thickness,
                    )
                    .corner_radius_corners(r.overlay_corner_radius)
                    .child(
                        Padding::new(EdgeInsetsGeometry::from_ltrb(0.0, 2.0, 0.0, 2.0))
                            .child(ScrollBarViewport::new(scroll, content)),
                    ),
                )
                .into_widget()
        }));
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).flyout.unwrap().dispose(app);
        app.get(self).scroll.unwrap().dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let mut ids = HashSet::new();
        assert!(
            widget.items_source.iter().all(|item| ids.insert(&item.id)),
            "BreadcrumbBar item ids must be unique"
        );
        let mut children = vec![
            BreadcrumbItemControl {
                owner: self,
                item: None,
                index: None,
                current: false,
                dropdown: false,
                enabled: widget.is_enabled,
                key: Rc::new(ValueKey::new((false, String::new()))),
            }
            .into_widget(),
        ];
        let count = widget.items_source.len();
        for (index, item) in widget.items_source.into_iter().enumerate() {
            let key = Rc::new(ValueKey::new((true, item.id.clone())));
            children.push(
                BreadcrumbItemControl {
                    owner: self,
                    enabled: widget.is_enabled && item.is_enabled,
                    item: Some(item),
                    index: Some(index),
                    current: index + 1 == count,
                    dropdown: false,
                    key,
                }
                .into_widget(),
            );
        }
        BreadcrumbLayout {
            children,
            direction: Directionality::of(app, context),
            on_layout: Rc::new(move |app, info| self.arranged(app, info)),
        }
        .into_widget()
    }
}

/// Source BreadcrumbBarItem's inline, current, ellipsis and dropdown templates.
#[derive(Clone, Debug)]
struct BreadcrumbItemControl {
    /// Containing bar and its event source.
    owner: Handle<BreadcrumbBarState>,

    /// None denotes the synthetic ellipsis item.
    item: Option<BreadcrumbBarItem>,

    /// Original zero-based collection index.
    index: Option<usize>,

    /// LastItem visual state.
    current: bool,

    /// EllipsisDropDown visual state.
    dropdown: bool,

    /// Effective IsEnabled, including the bar.
    enabled: bool,

    /// Stable native widget identity.
    key: KeyRef,
}

/// Owns native focus and its WinUI visual-source state.
struct BreadcrumbItemState {
    /// Native widget state.
    state: StateData<BreadcrumbItemControl>,

    /// Outer control focus; the inline button is not a Tab stop.
    focus: Option<AnyFocusNode>,

    /// Whether the item has the focus.
    focused: bool,

    /// Context below FlyoutTarget for the ellipsis.
    target: Option<BuildContext>,
}

impl StatefulWidget for BreadcrumbItemControl {
    type State = BreadcrumbItemState;

    fn key(&self) -> Option<&KeyRef> {
        Some(&self.key)
    }

    fn create_state(&self) -> Self::State {
        BreadcrumbItemState {
            state: StateData::new(),
            focus: None,
            focused: false,
            target: None,
        }
    }
}

impl BreadcrumbItemState {
    /// Native focus identity.
    fn node(self: Handle<Self>, app: &App) -> AnyFocusNode {
        app.get(self).focus.unwrap()
    }

    /// Same-element focus requests also update the WinUI focus state.
    fn request_focus(self: Handle<Self>, app: &mut App, source: FocusState) {
        FocusState::name(app, self.node(app), source);
        self.set_state(app, |_| {});
        self.node(app).request_focus(app, None);
    }

    /// The source preview handler activates even the current item on Enter/Space.
    fn activate(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        if !widget.enabled {
            return;
        }
        if let Some(item) = widget.item {
            widget
                .owner
                .invoke(app, item, widget.index.unwrap(), widget.dropdown);
        } else if let Some(target) = app.get(self).target {
            widget.owner.open(app, target);
        }
    }

    /// Source preview keyboard activation and inline directional movement.
    fn key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if !self.widget(app).enabled || !matches!(event, KeyEvent::Down(_) | KeyEvent::Repeat(_)) {
            return KeyEventResult::Ignored;
        }
        let widget = self.widget(app).clone();
        match event.logical_key() {
            LogicalKeyboardKey::ENTER | LogicalKeyboardKey::SPACE => {
                self.activate(app);
                KeyEventResult::Handled
            }
            LogicalKeyboardKey::ARROW_LEFT | LogicalKeyboardKey::ARROW_RIGHT
                if !widget.dropdown =>
            {
                let rtl = Directionality::of(app, self.context(app)) == TextDirection::Rtl;
                if widget.owner.move_focus(
                    app,
                    widget.index,
                    (event.logical_key() == LogicalKeyboardKey::ARROW_RIGHT) != rtl,
                ) {
                    KeyEventResult::Handled
                } else {
                    KeyEventResult::Ignored
                }
            }
            _ => KeyEventResult::Ignored,
        }
    }

    /// Applies the XAML foreground/background setters; the last item has no pointer button.
    fn content(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let theme = ThemeResources::of(app, context);
        let r = theme.clone();
        let focused = app.get(self).focused && FocusState::current(app).shows_focus_visual();
        let body = widget
            .item
            .as_ref()
            .map(|item| item.content.clone())
            .unwrap_or_else(|| {
                Padding::new(EdgeInsetsGeometry::all(3.0))
                    .child(
                        FontIcon::symbol(FluentSymbol::More)
                            .font_size(r.control_content_theme_font_size),
                    )
                    .into_widget()
            });
        if widget.current {
            return FocusVisual::new(
                Padding::new(EdgeInsetsGeometry::from_ltrb(1.0, 3.0, 1.0, 3.0)).child(
                    DefaultTextStyle::new(
                        control_text_style(
                            &theme,
                            r.control_content_theme_font_size,
                            FontWeight::NORMAL,
                            r.breadcrumb_bar_current_normal_foreground_brush,
                        )
                        .height(20.0 / r.control_content_theme_font_size),
                        body,
                    ),
                ),
                theme.theme,
            )
            .visible(focused)
            .margin([-3.0; 4])
            .corner_radius(r.control_corner_radius[0])
            .into_widget();
        }
        let dropdown = widget.dropdown;
        let states_r = r.clone();
        let control = CommonStates::new(
            Listener::new(move |app| self.activate(app)),
            move |_, _, state| {
                let r = &states_r;
                let (foreground, background) = if dropdown {
                    match state.common {
                        CommonState::Disabled => (
                            r.breadcrumb_bar_ellipsis_drop_down_item_foreground_disabled,
                            r.breadcrumb_bar_ellipsis_drop_down_item_background_disabled,
                        ),
                        CommonState::Pressed => (
                            r.breadcrumb_bar_ellipsis_drop_down_item_foreground_pressed,
                            r.breadcrumb_bar_ellipsis_drop_down_item_background_pressed,
                        ),
                        CommonState::PointerOver => (
                            r.breadcrumb_bar_ellipsis_drop_down_item_foreground_pointer_over,
                            r.breadcrumb_bar_ellipsis_drop_down_item_background_pointer_over,
                        ),
                        CommonState::Normal => (
                            r.breadcrumb_bar_foreground_brush,
                            r.breadcrumb_bar_ellipsis_drop_down_item_background,
                        ),
                    }
                } else {
                    let fg = match state.common {
                        CommonState::Disabled => r.breadcrumb_bar_disabled_foreground_brush,
                        CommonState::Pressed => r.breadcrumb_bar_pressed_foreground_brush,
                        CommonState::PointerOver => r.breadcrumb_bar_hover_foreground_brush,
                        CommonState::Normal => r.breadcrumb_bar_normal_foreground_brush,
                    };
                    (fg, r.breadcrumb_bar_background_brush)
                };
                FocusVisual::new(
                    ControlBorder::new(
                        Brush::Solid(background),
                        Brush::Solid(r.breadcrumb_bar_border_brush),
                    )
                    .border_thickness(0.0)
                    .corner_radius_corners(r.control_corner_radius)
                    .padding(if dropdown {
                        [11.0, 7.0, 11.0, 9.0]
                    } else {
                        [1.0, 3.0, 1.0, 3.0]
                    })
                    .child(DefaultTextStyle::new(
                        if dropdown {
                            control_text_style(
                                &theme,
                                r.control_content_theme_font_size,
                                FontWeight::NORMAL,
                                foreground,
                            )
                        } else {
                            control_text_style(
                                &theme,
                                r.control_content_theme_font_size,
                                FontWeight::NORMAL,
                                foreground,
                            )
                            .height(20.0 / r.control_content_theme_font_size)
                        },
                        body.clone(),
                    )),
                    theme.theme,
                )
                .visible(focused)
                .margin([-3.0; 4])
                .corner_radius(r.control_corner_radius[0])
                .into_widget()
            },
        )
        .is_enabled(widget.enabled)
        .is_tab_stop(false);
        if dropdown {
            return Margin::new([5.0, 3.0, 5.0, 3.0], control).into_widget();
        }
        let rtl = Directionality::of(app, context) == TextDirection::Rtl;
        Row::new()
            .main_axis_size(MainAxisSize::Min)
            .children([
                control.into_widget(),
                Padding::new(EdgeInsetsGeometry::from_ltrb(2.0, 0.0, 2.0, 0.0))
                    .child(
                        FontIcon::symbol(if rtl {
                            FluentSymbol::ChevronLeft
                        } else {
                            FluentSymbol::ChevronRight
                        })
                        .font_size(r.breadcrumb_bar_chevron_font_size)
                        .foreground(r.breadcrumb_bar_normal_foreground_brush),
                    )
                    .into_widget(),
            ])
            .into_widget()
    }
}

impl State for BreadcrumbItemState {
    type Widget = BreadcrumbItemControl;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let node = FocusNode::new(app).as_node();
        app.get_mut(self).focus = Some(node);
        node.set_on_key_event(
            app,
            Some(Rc::new(move |app, _, event| self.key(app, event))),
        );
        let widget = self.widget(app).clone();
        if !widget.dropdown {
            node.set_skip_traversal(app, true);
            if let Some(item) = widget.item {
                app.get_mut(widget.owner).items.insert(item.id, self);
            } else {
                app.get_mut(widget.owner).ellipsis = Some(self);
            }
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        if !widget.dropdown && app.contains(widget.owner) {
            if let Some(item) = widget.item {
                app.get_mut(widget.owner).items.remove(&item.id);
            } else {
                app.get_mut(widget.owner).ellipsis = None;
            }
        }
        let node = self.node(app);
        node.dispose(app);
        app.destroy(node.id());
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let content = self.content(app, context);
        FocusableActionDetector::new(FlyoutTarget::new(Builder::new(move |app, context| {
            app.get_mut(self).target = Some(context);
            content.clone()
        })))
        .focus_node(self.node(app))
        .enabled(self.widget(app).enabled)
        .on_focus_change(move |app, focused| self.set_state(app, |state| state.focused = focused))
        .into_widget()
    }
}
