//! `ItemContainer` (`controls/dev/ItemContainer`: `ItemContainer.xaml`, `ItemContainer_themeresources.xaml` and `ItemContainer.cpp`): the item of an `ItemsView`, which lights under the pointer, shows pressed and selected with its selection visual, shows a check box in `Multiple` mode, takes the focus, and tells its `ItemsView` of a click, a tap, a double tap, Enter and Space.
//!
//! Inside an `ItemsView` the container reads its index and its view from the scope the view puts around it, and listens to the view's selection for its own index, rebuilding only when its own selected state or tab stop changes. A later editor who has the view rebuild containers instead must keep that listener, or a container shows the selection it was built with.

use crate::FocusState;
use crate::*;
use std::{fmt, rc::Rc, time::Duration, time::Instant};
use waft_animation::{Cubic, Curve};
use waft_embedder::{Color, Offset, PointerDeviceKind};
use waft_foundation::{App, Handle, ListenableObject, Listener};
use waft_gestures::{K_DOUBLE_TAP_SLOP, K_DOUBLE_TAP_TIMEOUT, TapDownDetails};
use waft_painting::AlignmentGeometry;
use waft_rendering::HitTestBehavior;
use waft_services::{KeyEvent, LogicalKeyboardKey};
use waft_widgets::*;

/// `ItemContainerUserSelectMode`: whether the user can select the item; `Auto` takes it from the `ItemsView`'s selection mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ItemContainerUserSelectMode {
    #[default]
    Auto,
    UserCanSelect,
    UserCannotSelect,
}

/// `ItemContainerUserInvokeMode`: whether the user can invoke the item; `Auto` takes it from `ItemsView::is_item_invoked_enabled`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ItemContainerUserInvokeMode {
    #[default]
    Auto,
    UserCanInvoke,
    UserCannotInvoke,
}

/// `ItemContainerMultiSelectMode`: whether the item shows the check box of `Multiple` mode; `Auto` takes it from the `ItemsView`'s selection mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ItemContainerMultiSelectMode {
    #[default]
    Auto,
    Single,
    Extended,
    Multiple,
}

/// `ItemContainerInteractionTrigger`: what the user did to the item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ItemContainerInteractionTrigger {
    PointerReleased,
    Tap,
    DoubleTap,
    EnterKey,
    SpaceKey,
}

/// What a container's template is built for: the `CombinedStates`, `MultiSelectStates` and `DisabledStates` the container is in, and whether the system focus visual shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ItemContainerStates {
    pub(crate) is_selected: bool,
    /// `Normal`, `PointerOver`, `Pressed` or `Disabled`.
    pub(crate) common: CommonState,
    pub(crate) focused: bool,
    /// `MultiSelectStates`' `Multiple`.
    pub(crate) multiple: bool,
}

/// Builds a container's template for its states; `SelectorBarItem`'s style supplies one of its own.
pub(crate) type ItemContainerTemplate =
    Rc<dyn Fn(&ThemeResources, &ItemContainer, ItemContainerStates) -> WidgetRef>;

/// XAML `ItemContainer`.
#[derive(Clone)]
pub struct ItemContainer {
    /// `Child`: what the item shows.
    pub child: WidgetRef,
    /// `IsSelected`, for a container outside an `ItemsView`; inside one the view's selection decides.
    pub is_selected: bool,
    /// `IsEnabled`.
    pub is_enabled: bool,
    /// `CanUserSelect`.
    pub can_user_select: ItemContainerUserSelectMode,
    /// `CanUserInvoke`.
    pub can_user_invoke: ItemContainerUserInvokeMode,
    /// `MultiSelectMode`.
    pub multi_select_mode: ItemContainerMultiSelectMode,
    /// `Background`; `None` is the style's `ItemContainerBackground`.
    pub background: Option<Brush>,
    /// `CornerRadius`; `None` is the style's `ControlCornerRadius`.
    pub corner_radius: Option<[f64; 4]>,
    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
    /// The template; `None` is `ItemContainer.xaml`'s.
    pub(crate) template: Option<ItemContainerTemplate>,
}

impl ItemContainer {
    /// Creates a container showing `child`.
    pub fn new<K>(child: impl IntoWidget<K>) -> Self {
        Self {
            child: child.into_widget(),
            is_selected: false,
            is_enabled: true,
            can_user_select: ItemContainerUserSelectMode::Auto,
            can_user_invoke: ItemContainerUserInvokeMode::Auto,
            multi_select_mode: ItemContainerMultiSelectMode::Auto,
            background: None,
            corner_radius: None,
            key: None,
            template: None,
        }
    }

    /// Sets the XAML `IsSelected`.
    pub fn is_selected(mut self, value: bool) -> Self {
        self.is_selected = value;
        self
    }

    /// Sets the XAML `IsEnabled`.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Sets the XAML `CanUserSelect`.
    pub fn can_user_select(mut self, value: ItemContainerUserSelectMode) -> Self {
        self.can_user_select = value;
        self
    }

    /// Sets the XAML `CanUserInvoke`.
    pub fn can_user_invoke(mut self, value: ItemContainerUserInvokeMode) -> Self {
        self.can_user_invoke = value;
        self
    }

    /// Sets the XAML `MultiSelectMode`.
    pub fn multi_select_mode(mut self, value: ItemContainerMultiSelectMode) -> Self {
        self.multi_select_mode = value;
        self
    }

    /// Sets the XAML `Background`.
    pub fn background(mut self, value: Brush) -> Self {
        self.background = Some(value);
        self
    }

    /// Sets the XAML `CornerRadius`.
    pub fn corner_radius(mut self, value: [f64; 4]) -> Self {
        self.corner_radius = Some(value);
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }

    /// Gives the container another style's template.
    pub(crate) fn template(mut self, template: ItemContainerTemplate) -> Self {
        self.template = Some(template);
        self
    }
}

impl fmt::Debug for ItemContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemContainer")
            .field("is_selected", &self.is_selected)
            .field("is_enabled", &self.is_enabled)
            .field("can_user_select", &self.can_user_select)
            .field("can_user_invoke", &self.can_user_invoke)
            .field("multi_select_mode", &self.multi_select_mode)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ItemContainer {
    type State = ItemContainerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ItemContainerState {
        ItemContainerState {
            state: StateData::new(),
            focus_node: None,
            pointer: PointerInfo::default(),
            focused: false,
            last_tap: None,
            item: None,
            selection_listener: None,
            shown: ShownItem::default(),
        }
    }
}

/// `PointerInfo<ItemContainer>`: what the pointer is doing to the container.
#[derive(Clone, Copy, Debug, Default)]
struct PointerInfo {
    /// A mouse is over the container.
    mouse_over: bool,
    /// A touch or pen press is over it, from its press to its release.
    touch_over: bool,
    /// A press is down on it.
    pressed: bool,
}

impl PointerInfo {
    fn is_pointer_over(self) -> bool {
        self.mouse_over || self.touch_over
    }
}

/// What the container last showed of its view's state, so that a selection change rebuilds only the containers it concerns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ShownItem {
    selected: bool,
    tab_stop: bool,
}

/// The container's pointer, focus and place in its view.
pub struct ItemContainerState {
    state: StateData<ItemContainer>,
    focus_node: Option<Handle<FocusNode>>,
    pointer: PointerInfo,
    focused: bool,
    /// When and where the last tap was, which a second tap near it soon after makes a double tap.
    last_tap: Option<(Instant, Offset)>,
    /// The item's index and its view, from the scope the view puts around it.
    item: Option<ItemsViewItem>,
    selection_listener: Option<(Handle<ItemsViewSelection>, Listener)>,
    shown: ShownItem,
}

impl ItemContainerState {
    fn focus_node(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self).focus_node.expect("made in `init_state`")
    }

    /// What the view says of the item now; outside a view, the container's own `IsSelected`.
    fn view_state(self: Handle<Self>, app: &App) -> ShownItem {
        match app.get(self).item {
            Some(item) => ShownItem {
                selected: item.view.is_selected(app, item.index),
                tab_stop: item.view.is_tab_stop(app, item.index),
            },
            None => ShownItem {
                selected: self.widget(app).is_selected,
                tab_stop: true,
            },
        }
    }

    /// The selection changed: the container rebuilds when what it shows of it did.
    fn selection_changed(self: Handle<Self>, app: &mut App) {
        let now = self.view_state(app);
        if now != app.get(self).shown {
            self.set_state(app, |state| state.shown = now);
        }
    }

    /// Finds the view the container is an item of and listens to its selection.
    fn attach_to_view(self: Handle<Self>, app: &mut App) {
        let item = ItemsViewItem::of(app, self.context(app));
        let previous = app.get(self).item;
        if previous == item {
            return;
        }
        if let Some(previous) = previous {
            previous.view.unregister(app, previous.index, self);
        }
        self.stop_listening(app);
        app.get_mut(self).item = item;
        if let Some(item) = item {
            item.view
                .register(app, item.index, self, self.focus_node(app));
            let selection = item.view.selection(app);
            let listener = Listener::new(move |app| self.selection_changed(app));
            selection.add_listener(app, listener.clone());
            app.get_mut(self).selection_listener = Some((selection, listener));
        }
        let shown = self.view_state(app);
        app.get_mut(self).shown = shown;
    }

    fn stop_listening(self: Handle<Self>, app: &mut App) {
        if let Some((selection, listener)) = app.get_mut(self).selection_listener.take()
            && app.contains(selection)
        {
            selection.remove_listener(app, &listener);
        }
    }

    /// The container's `MultiSelectMode`, `Auto` resolved by its view as `ItemsView::OnItemsRepeaterElementPrepared` does.
    fn multi_select_mode(self: Handle<Self>, app: &App) -> ItemContainerMultiSelectMode {
        let mode = self.widget(app).multi_select_mode;
        match (mode, app.get(self).item) {
            (ItemContainerMultiSelectMode::Auto, Some(item)) => item.view.multi_select_mode(app),
            _ => mode,
        }
    }

    /// `ItemContainer::CanRaiseItemInvoked`: the user can invoke the item, or can select it.
    fn can_raise_item_invoked(self: Handle<Self>, app: &App) -> bool {
        let Some(item) = app.get(self).item else {
            return false;
        };
        let widget = self.widget(app);
        item.view.can_user_invoke(app, widget.can_user_invoke)
            || item.view.can_user_select(app, widget.can_user_select)
    }

    /// `RaiseItemInvoked`: tells the view, which answers whether it handled it.
    fn raise_item_invoked(
        self: Handle<Self>,
        app: &mut App,
        trigger: ItemContainerInteractionTrigger,
    ) -> bool {
        if !self.can_raise_item_invoked(app) {
            return false;
        }
        let item = app
            .get(self)
            .item
            .expect("checked by `can_raise_item_invoked`");
        let widget = self.widget(app).clone();
        item.view
            .item_container_invoked(app, item.index, &widget, trigger)
    }

    fn update_pointer(self: Handle<Self>, app: &mut App, change: impl FnOnce(&mut PointerInfo)) {
        let mut pointer = app.get(self).pointer;
        change(&mut pointer);
        let before = app.get(self).pointer;
        if pointer.is_pointer_over() != before.is_pointer_over()
            || pointer.pressed != before.pressed
        {
            self.set_state(app, |state| state.pointer = pointer);
        } else {
            app.get_mut(self).pointer = pointer;
        }
    }

    /// `OnPointerPressed`.
    fn pointer_pressed(self: Handle<Self>, app: &mut App, details: &TapDownDetails) {
        let touch = !matches!(details.kind, Some(PointerDeviceKind::Mouse) | None);
        self.update_pointer(app, |pointer| {
            pointer.pressed = true;
            if touch {
                pointer.touch_over = true;
            } else {
                pointer.mouse_over = true;
            }
        });
    }

    /// `OnPointerReleased`: the press ends and, a left button or a touch, the view hears of it.
    fn pointer_released(self: Handle<Self>, app: &mut App) {
        if !app.get(self).pointer.pressed {
            return;
        }
        self.update_pointer(app, |pointer| {
            pointer.pressed = false;
            pointer.touch_over = false;
        });
        self.raise_item_invoked(app, ItemContainerInteractionTrigger::PointerReleased);
    }

    /// `ProcessPointerCanceled`.
    fn pointer_canceled(self: Handle<Self>, app: &mut App) {
        self.update_pointer(app, |pointer| {
            pointer.pressed = false;
            pointer.touch_over = false;
        });
    }

    /// `OnTapped` and `OnDoubleTapped`: a second tap soon after the first and near it is a double tap.
    fn tapped(self: Handle<Self>, app: &mut App, position: Offset) {
        let now = Instant::now();
        let double = app.get(self).last_tap.is_some_and(|(at, from)| {
            now.duration_since(at) <= K_DOUBLE_TAP_TIMEOUT
                && (position - from).distance() <= K_DOUBLE_TAP_SLOP
        });
        app.get_mut(self).last_tap = if double { None } else { Some((now, position)) };
        let trigger = if double {
            ItemContainerInteractionTrigger::DoubleTap
        } else {
            ItemContainerInteractionTrigger::Tap
        };
        self.raise_item_invoked(app, trigger);
    }

    /// `OnKeyDown`: Enter and Space.
    fn key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if matches!(event, KeyEvent::Up(_)) {
            return KeyEventResult::Ignored;
        }
        let trigger = match event.logical_key() {
            LogicalKeyboardKey::ENTER => ItemContainerInteractionTrigger::EnterKey,
            LogicalKeyboardKey::SPACE => ItemContainerInteractionTrigger::SpaceKey,
            _ => return KeyEventResult::Ignored,
        };
        if self.raise_item_invoked(app, trigger) {
            KeyEventResult::Handled
        } else {
            KeyEventResult::Ignored
        }
    }

    fn focus_changed(self: Handle<Self>, app: &mut App, focused: bool) {
        self.set_state(app, |state| state.focused = focused);
        if focused && let Some(item) = app.get(self).item {
            item.view.item_got_focus(app, item.index);
        }
    }

    /// `UpdateVisualState` and `UpdateMultiSelectState`.
    fn states(self: Handle<Self>, app: &mut App) -> ItemContainerStates {
        let shows_focus_visual = FocusState::current(app).shows_focus_visual();
        let widget = self.widget(app);
        let state = app.get(self);
        let common = if !widget.is_enabled {
            CommonState::Disabled
        } else if state.pointer.pressed {
            CommonState::Pressed
        } else if state.pointer.is_pointer_over() {
            CommonState::PointerOver
        } else {
            CommonState::Normal
        };
        ItemContainerStates {
            is_selected: state.shown.selected,
            common,
            focused: state.focused && shows_focus_visual,
            multiple: self.multi_select_mode(app) == ItemContainerMultiSelectMode::Multiple,
        }
    }
}

impl State for ItemContainerState {
    type Widget = ItemContainer;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        InputDevice::initialize(app);
        app.get_mut(self).focus_node = Some(FocusNode::new(app));
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.attach_to_view(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &ItemContainer) {
        self.attach_to_view(app);
        if !self.widget(app).is_enabled {
            // `OnIsEnabledChanged`: a disabled container drops the pointer.
            app.get_mut(self).pointer = PointerInfo::default();
        }
        let shown = self.view_state(app);
        app.get_mut(self).shown = shown;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.stop_listening(app);
        if let Some(item) = app.get(self).item
            && app.contains(item.view)
        {
            item.view.unregister(app, item.index, self);
        }
        if let Some(node) = app.get_mut(self).focus_node.take() {
            node.dispose(app);
            app.destroy(node);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let states = self.states(app);
        let template = widget
            .template
            .clone()
            .unwrap_or_else(|| Rc::new(item_container_template));
        let visual = template(&resources, &widget, states);
        let mut gesture = GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            .child(visual);
        if widget.is_enabled {
            gesture = gesture
                .on_tap_down(Rc::new(move |app, details| {
                    self.pointer_pressed(app, &details)
                }))
                .on_tap_up(Rc::new(move |app, details| {
                    self.pointer_released(app);
                    self.tapped(app, details.global_position);
                }))
                .on_tap_cancel(Listener::new(move |app| self.pointer_canceled(app)));
        }
        let mouse = MouseRegion::new()
            .on_enter(Rc::new(move |app, _| {
                self.update_pointer(app, |pointer| pointer.mouse_over = true)
            }))
            .on_exit(Rc::new(move |app, _| {
                // A button held down while the mouse leaves no longer presses the container.
                self.update_pointer(app, |pointer| {
                    pointer.mouse_over = false;
                    pointer.pressed = false;
                })
            }))
            .child(gesture);
        Focus::new(mouse)
            .focus_node(self.focus_node(app).as_node())
            .can_request_focus(widget.is_enabled)
            .skip_traversal(!app.get(self).shown.tab_stop)
            .on_focus_change(move |app, focused| self.focus_changed(app, focused))
            .on_key_event(Rc::new(move |app, _, event| self.key(app, event)))
            .into_widget()
    }
}

/// `ItemContainer.xaml`'s template: `PART_ContainerRoot` with the child, `PART_SelectionVisual`, `PART_CommonVisual` and `PART_SelectionCheckbox` over it, in that order.
fn item_container_template(
    r: &ThemeResources,
    container: &ItemContainer,
    states: ItemContainerStates,
) -> WidgetRef {
    let corner = container.corner_radius.unwrap_or(r.control_corner_radius);
    let background = container
        .background
        .unwrap_or(Brush::Solid(r.item_container_background));
    let visuals = CombinedVisuals::of(r, states);
    let fast = r.control_fast_animation_duration;
    let spline: Rc<dyn Curve> = Rc::new(Cubic::new(0.0, 0.0, 0.0, 1.0));
    let disabled = states.common == CommonState::Disabled;
    let mut children = vec![container.child.clone()];
    if !disabled {
        children.push(selection_visual(&visuals, corner, fast, spline.clone()));
    }
    children.push(common_visual(r, &visuals, corner));
    children.push(selection_checkbox(r, states, fast, spline.clone()));
    let root = Grid::new()
        .background(background)
        .corner_radius_corners(corner)
        .children(
            children
                .into_iter()
                .map(|child| GridCell::new(child).into_widget()),
        );
    // `DisabledStates`: the root fades to `ItemContainerDisabledOpacity`, and back at once.
    let root = AnimatedOpacity::new(
        if disabled {
            r.item_container_disabled_opacity
        } else {
            1.0
        },
        if disabled { fast } else { Duration::ZERO },
    )
    .curve(spline)
    .child(root);
    FocusVisual::new(root, r.theme)
        .visible(states.focused)
        .corner_radius(corner[0])
        .into_widget()
}

/// The `CombinedStates` storyboards' values for `PART_SelectionVisual` and `PART_CommonVisual`.
struct CombinedVisuals {
    /// `PART_SelectionVisual.Opacity`, and whether it fades in over `ControlFastAnimationDuration` rather than at once.
    selection_opacity: (f64, bool),
    selection_brush: Color,
    common_fill: Option<Color>,
    common_stroke: Color,
    common_margin: [f64; 4],
}

impl CombinedVisuals {
    /// The state `UpdateVisualState` chooses: pressed before pointer over before normal, each selected or not; a disabled container keeps its selected or unselected normal state.
    fn of(r: &ThemeResources, states: ItemContainerStates) -> Self {
        let selected = states.is_selected;
        let unselected = |fill, stroke| CombinedVisuals {
            selection_opacity: (0.0, false),
            selection_brush: Color::new(0),
            common_fill: fill,
            common_stroke: stroke,
            common_margin: [0.0; 4],
        };
        let selected_with = |fades, brush, fill| CombinedVisuals {
            selection_opacity: (1.0, fades),
            selection_brush: brush,
            common_fill: Some(fill),
            common_stroke: r.item_container_selected_inner_border_brush,
            common_margin: r.item_container_selected_inner_margin,
        };
        match (selected, states.common) {
            (false, CommonState::PointerOver) => unselected(
                Some(r.item_container_pointer_over_background),
                r.item_container_pointer_over_border_brush,
            ),
            (false, CommonState::Pressed) => unselected(
                Some(r.item_container_pressed_background),
                r.item_container_pressed_border_brush,
            ),
            (false, _) => unselected(None, r.item_container_border_brush),
            (true, CommonState::PointerOver) => selected_with(
                true,
                r.item_container_selection_visual_pointer_over_background,
                r.item_container_selected_pointer_over_background,
            ),
            (true, CommonState::Pressed) => selected_with(
                true,
                r.item_container_selection_visual_pressed_background,
                r.item_container_selected_pressed_background,
            ),
            (true, _) => selected_with(
                false,
                r.item_container_selection_visual_background,
                r.item_container_selected_background,
            ),
        }
    }
}

/// `PART_SelectionVisual`: a 3 pixel border in the selection brush, taking no input.
fn selection_visual(
    visuals: &CombinedVisuals,
    corner: [f64; 4],
    fast: Duration,
    spline: Rc<dyn Curve>,
) -> WidgetRef {
    let (opacity, fades) = visuals.selection_opacity;
    IgnorePointer::new()
        .child(
            AnimatedOpacity::new(opacity, if fades { fast } else { Duration::ZERO })
                .curve(spline)
                .child(
                    ControlBorder::new(
                        Brush::Solid(Color::new(0)),
                        Brush::Solid(visuals.selection_brush),
                    )
                    .border_thickness(3.0)
                    .corner_radius_corners(corner),
                ),
        )
        .into_widget()
}

/// `PART_CommonVisual`: a `Rectangle` filled and stroked by the state, its radii the corner radius's top left and bottom right, taking no input.
fn common_visual(r: &ThemeResources, visuals: &CombinedVisuals, corner: [f64; 4]) -> WidgetRef {
    let rectangle = ControlBorder::new_optional(
        visuals.common_fill.map(Brush::Solid),
        Some(Brush::Solid(visuals.common_stroke)),
    )
    .border_thickness_ltrb(r.item_container_selected_inner_thickness)
    .corner_radius_corners([corner[0], corner[0], corner[2], corner[2]])
    .background_sizing(BackgroundSizing::OuterBorderEdge);
    IgnorePointer::new()
        .child(Margin::new(visuals.common_margin, rectangle))
        .into_widget()
}

/// `PART_SelectionCheckbox` in `ItemContainerSelectionCheckboxStyle`: a check box at the top right that shows the selection, fading in for `Multiple` and taking no input.
fn selection_checkbox(
    r: &ThemeResources,
    states: ItemContainerStates,
    fast: Duration,
    spline: Rc<dyn Curve>,
) -> WidgetRef {
    let disabled = states.common == CommonState::Disabled;
    // `CheckedDisabled` sets nothing, so a disabled selected box shows the template's own values.
    let (fill, stroke, glyph, check) = match (states.is_selected, disabled) {
        (true, false) => (
            r.check_box_check_background_fill_checked,
            r.check_box_check_background_stroke_checked,
            r.check_box_check_glyph_foreground_checked,
            CheckState::Checked,
        ),
        (false, true) => (
            r.check_box_check_background_fill_unchecked_disabled,
            r.check_box_check_background_stroke_unchecked_disabled,
            r.check_box_check_glyph_foreground_unchecked_disabled,
            CheckState::Unchecked,
        ),
        _ => (
            r.item_container_checkbox_background_unchecked,
            r.check_box_check_background_stroke_unchecked,
            r.check_box_check_glyph_foreground_unchecked,
            CheckState::Unchecked,
        ),
    };
    let root_border = ControlBorder::new(Brush::Solid(fill), Brush::Solid(stroke))
        .border_thickness(r.check_box_border_thickness)
        .corner_radius_corners(r.control_corner_radius)
        .child(Transform::translate(CHECK_GLYPH_OFFSET).child(CheckGlyph {
            check,
            color: glyph,
        }));
    let box_ = SizedBox::new()
        .width(r.check_box_size)
        .height(r.check_box_size)
        .child(root_border);
    let placed = Margin::new(
        r.item_container_checkbox_margin,
        Align::new()
            .alignment(AlignmentGeometry::TOP_RIGHT)
            .child(box_),
    );
    IgnorePointer::new()
        .child(
            AnimatedOpacity::new(
                if states.multiple { 1.0 } else { 0.0 },
                if states.multiple {
                    fast
                } else {
                    Duration::ZERO
                },
            )
            .curve(spline)
            .child(placed),
        )
        .into_widget()
}
