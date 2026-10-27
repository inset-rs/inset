//! ComboBoxItem_Partial.cpp and `DefaultComboBoxItemStyle`: one item of the drop-down, its
//! visual states and its pointer input.

use crate::*;
use inset_animation::{Cubic, Curve};
use inset_embedder::{Color, FontWeight};
use inset_foundation::{App, Handle};
use inset_painting::{Alignment, EdgeInsetsGeometry};
use inset_rendering::HitTestBehavior;
use inset_scheduler::{FrameCallback, Ticker};
use inset_widgets::*;
use std::{fmt, rc::Rc, time::Duration};

/// One item of a `ComboBox`: a XAML `ComboBoxItem`, or an item of `Items` the box makes one for.
#[derive(Clone)]
pub struct ComboBoxItem {
    /// Identity across rebuilds and moves; unique within the box.
    pub id: String,

    /// `Content`: shown in the drop-down and, while the item is selected and the drop-down
    /// closed, in the box.
    pub content: WidgetRef,

    /// The item's text, as `TryGetStringValue` reads an item: what type-ahead search and an
    /// editable box's matching compare, and what an editable box shows for the item. An item
    /// without text ends a search, as an item the source cannot get a string from does.
    pub plain_text: Option<String>,

    /// `IsEnabled`: a disabled item is drawn disabled and takes no pointer input or focus.
    pub is_enabled: bool,
}

impl ComboBoxItem {
    /// Creates an item showing `content`.
    pub fn new<K>(id: impl Into<String>, content: impl IntoWidget<K>) -> Self {
        Self {
            id: id.into(),
            content: content.into_widget(),
            plain_text: None,
            is_enabled: true,
        }
    }

    /// Creates an item that is `text`, as a string item of `Items` is.
    pub fn text(id: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            plain_text: Some(text.clone()),
            ..Self::new(id, Text::new(text).soft_wrap(false))
        }
    }

    /// Sets the item's text.
    pub fn plain_text(mut self, value: impl Into<String>) -> Self {
        self.plain_text = Some(value.into());
        self
    }

    /// Sets IsEnabled.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }
}

impl fmt::Debug for ComboBoxItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComboBoxItem")
            .field("id", &self.id)
            .field("plain_text", &self.plain_text)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

/// `ComboBoxItem`'s `CommonStates`, as `ComboBoxItem::ChangeVisualState` chooses among them.
/// The template's `SelectedDisabled` is never chosen: a disabled item is `Disabled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ComboBoxItemCommonState {
    Normal,
    PointerOver,
    Disabled,
    Pressed,
    Selected,
    SelectedUnfocused,
    SelectedPointerOver,
    SelectedPressed,
}

impl ComboBoxItemCommonState {
    /// Whether the state shows `Pill`.
    fn shows_pill(self) -> bool {
        matches!(
            self,
            Self::Selected
                | Self::SelectedUnfocused
                | Self::SelectedPointerOver
                | Self::SelectedPressed
        )
    }
}

/// `ComboBoxItem::ChangeVisualState`'s choice for an item of an open drop-down: a disabled item
/// whose content is not a control is `Disabled`; an item that appears selected is `Selected`
/// while the box has the focus and `SelectedUnfocused` otherwise, unless it is pressed or under
/// the pointer.
pub(super) fn common_state(
    is_enabled: bool,
    appears_selected: bool,
    selection_active: bool,
    pressed: bool,
    pointer_over: bool,
) -> ComboBoxItemCommonState {
    use ComboBoxItemCommonState::*;
    if !is_enabled {
        Disabled
    } else if appears_selected {
        if pressed {
            SelectedPressed
        } else if pointer_over {
            SelectedPointerOver
        } else if selection_active {
            Selected
        } else {
            SelectedUnfocused
        }
    } else if pressed {
        Pressed
    } else if pointer_over {
        PointerOver
    } else {
        Normal
    }
}

/// `LayoutRoot`'s `Margin`: the gap around each item.
pub(super) const ITEM_MARGIN: [f64; 4] = [5.0, 2.0, 5.0, 2.0];

/// `DefaultComboBoxItemStyle`'s `FocusVisualMargin`.
const ITEM_FOCUS_VISUAL_MARGIN: f64 = -3.0;

/// Receives a click on the item at an index.
pub(super) type ItemClicked = Rc<dyn Fn(&mut App, usize)>;

/// Receives the item at an index gaining or losing the focus.
pub(super) type ItemFocusChanged = Rc<dyn Fn(&mut App, usize, bool)>;

/// The container the box makes for an item: `DefaultComboBoxItemStyle`'s template around the
/// item's content, for an open drop-down.
#[derive(Clone)]
pub(super) struct ComboBoxItemContainer {
    /// The item.
    pub(super) item: ComboBoxItem,

    /// The item's index in `Items`.
    pub(super) index: usize,

    /// Whether the item appears selected: it is selected, or the box draws another item as
    /// selected in its place.
    pub(super) appears_selected: bool,

    /// `IsSelectionActive` of the box: whether the box has the focus.
    pub(super) selection_active: bool,

    /// Whether the focus visual shows while the item has the focus: the focus came from the
    /// keyboard.
    pub(super) shows_focus_visual: bool,

    /// The item's focus, owned by the box so that it can focus items it scrolls to.
    pub(super) focus_node: Handle<FocusNode>,

    /// `OnSelectorItemClicked`, then closing the drop-down.
    pub(super) clicked: ItemClicked,

    /// `ItemFocused` and `ItemUnfocused`.
    pub(super) focus_changed: ItemFocusChanged,
}

impl fmt::Debug for ComboBoxItemContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComboBoxItemContainer")
            .field("item", &self.item)
            .field("index", &self.index)
            .field("appears_selected", &self.appears_selected)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ComboBoxItemContainer {
    type State = ComboBoxItemContainerState;

    fn create_state(&self) -> Self::State {
        ComboBoxItemContainerState {
            state: StateData::new(),
            single_ticker_provider: Default::default(),
            ticker: None,
            pressed_for: Duration::ZERO,
            pointer_over: false,
            pressed: false,
            should_perform_actions: false,
            focused: false,
        }
    }
}

/// ComboBoxItem's pointer flags and the clock of its `SelectedPressed` storyboard.
pub(super) struct ComboBoxItemContainerState {
    state: StateData<ComboBoxItemContainer>,

    /// TickerMode subscription.
    single_ticker_provider: SingleTickerProviderStateMixinData,

    /// Clock of `PillTransform`'s `ScaleY` animation.
    ticker: Option<Handle<Ticker>>,

    /// How long the item has been in `SelectedPressed`.
    pressed_for: Duration,

    /// `IsPointerOver`.
    pointer_over: bool,

    /// `m_bIsPressed`.
    pressed: bool,

    /// `m_shouldPerformActions`.
    should_perform_actions: bool,

    /// Whether the item has the focus.
    focused: bool,
}

impl SingleTickerProviderStateMixin for ComboBoxItemContainerState {
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

impl ComboBoxItemContainerState {
    /// The current `CommonStates` state.
    fn common_state(self: Handle<Self>, app: &App) -> ComboBoxItemCommonState {
        let widget = self.widget(app);
        let state = app.get(self);
        common_state(
            widget.item.is_enabled,
            widget.appears_selected,
            widget.selection_active,
            state.pressed,
            state.pointer_over,
        )
    }

    /// `ClearVisualStateFlagsOnExit`.
    fn clear_flags(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| {
            state.pressed = false;
            state.pointer_over = false;
            state.should_perform_actions = false;
        });
        self.sync_storyboard(app);
    }

    /// Starts `SelectedPressed`'s storyboard on entering the state and stops it on leaving,
    /// which returns `PillTransform` to its base value at once.
    fn sync_storyboard(self: Handle<Self>, app: &mut App) {
        let Some(ticker) = app.get(self).ticker else {
            return;
        };
        let pressed = self.common_state(app) == ComboBoxItemCommonState::SelectedPressed;
        if pressed && !ticker.is_active(app) && app.get(self).pressed_for.is_zero() {
            ticker.start(app);
        } else if !pressed {
            ticker.stop(app, false);
            app.get_mut(self).pressed_for = Duration::ZERO;
        }
    }

    /// One frame of the storyboard.
    fn tick(self: Handle<Self>, app: &mut App, elapsed: Duration) {
        self.set_state(app, |state| state.pressed_for = elapsed);
        let duration =
            ThemeResources::of(app, self.context(app)).combo_box_item_scale_animation_duration;
        if elapsed >= duration {
            app.get(self).ticker.unwrap().stop(app, false);
        }
    }

    /// `PillTransform`'s `ScaleY`: 1, or on its way to `ComboBoxItemPillMinScale` in
    /// `SelectedPressed`.
    fn pill_scale(self: Handle<Self>, r: &ThemeResources, app: &App) -> f64 {
        if self.common_state(app) != ComboBoxItemCommonState::SelectedPressed {
            return 1.0;
        }
        let progress = (app.get(self).pressed_for.as_secs_f64()
            / r.combo_box_item_scale_animation_duration.as_secs_f64())
        .min(1.0);
        let eased = Cubic::new(0.0, 0.0, 0.0, 1.0).transform(progress);
        1.0 + (r.combo_box_item_pill_min_scale - 1.0) * eased
    }

    /// The template, `LayoutRoot` with `Pill` and `ContentPresenter`, in the current state.
    fn template(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let r = &resources;
        let widget = self.widget(app).clone();
        let common = self.common_state(app);
        use ComboBoxItemCommonState::*;
        let (background, foreground) = match common {
            Normal => (r.combo_box_item_background, r.combo_box_item_foreground),
            PointerOver => (
                r.combo_box_item_background_pointer_over,
                r.combo_box_item_foreground_pointer_over,
            ),
            Disabled => (
                r.combo_box_item_background_disabled,
                r.combo_box_item_foreground_disabled,
            ),
            Pressed => (
                r.combo_box_item_background_pressed,
                r.combo_box_item_foreground_pressed,
            ),
            Selected => (
                r.combo_box_item_background_selected,
                r.combo_box_item_foreground_selected,
            ),
            SelectedUnfocused => (
                r.combo_box_item_background_selected_unfocused,
                r.combo_box_item_foreground_selected_unfocused,
            ),
            SelectedPointerOver => (
                r.combo_box_item_background_selected_pointer_over,
                r.combo_box_item_foreground_selected_pointer_over,
            ),
            SelectedPressed => (
                r.combo_box_item_background_selected_pressed,
                r.combo_box_item_foreground_selected_pressed,
            ),
        };
        let pill_opacity = if common.shows_pill() { 1.0 } else { 0.0 };
        let pill_scale = self.pill_scale(&resources, app);
        // `Pill`, in `ComboBoxItemPill`: 3 by 16 at the left, centered, scaled about its center.
        let pill = Align::new().alignment(Alignment::CENTER_LEFT.into()).child(
            Opacity::new(pill_opacity).child(
                Transform::scale(None, None, Some(pill_scale)).child(
                    SizedBox::new()
                        .width(r.combo_box_item_pill_width)
                        .height(r.combo_box_item_pill_height)
                        .child(
                            ControlBorder::new(
                                Brush::Solid(r.combo_box_item_pill_fill_brush),
                                Brush::Solid(Color::new(0)),
                            )
                            .border_thickness(0.0)
                            .corner_radius_corners(r.combo_box_item_pill_corner_radius),
                        ),
                ),
            ),
        );
        // `ContentPresenter`: the content at `Padding`, stretched, in the state's foreground.
        let content =
            Padding::new(insets(r.combo_box_item_theme_padding)).child(DefaultTextStyle::new(
                control_text_style(
                    &resources,
                    r.control_content_theme_font_size,
                    FontWeight::NORMAL,
                    foreground,
                ),
                widget.item.content.clone(),
            ));
        // `LayoutRoot`; the style sets no `BorderThickness`, so its border brushes never show.
        let layout_root = Grid::new()
            .background(Brush::Solid(background))
            .corner_radius_corners(r.combo_box_item_corner_radius)
            .children([
                GridCell::new(pill).into_widget(),
                GridCell::new(content).into_widget(),
            ]);
        let focused = app.get(self).focused && widget.shows_focus_visual;
        FocusVisual::new(layout_root, resources.theme)
            .visible(focused)
            .margin([ITEM_FOCUS_VISUAL_MARGIN; 4])
            .corner_radius(r.combo_box_item_corner_radius[0])
            .into_widget()
    }
}

impl State for ComboBoxItemContainerState {
    type Widget = ComboBoxItemContainer;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let ticker = SingleTickerProviderStateMixin::create_ticker(
            self,
            app,
            FrameCallback::new(move |app, elapsed| self.tick(app, elapsed)),
        );
        app.get_mut(self).ticker = Some(ticker);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old: &ComboBoxItemContainer) {
        // `OnIsEnabledChanged`.
        if !self.widget(app).item.is_enabled {
            let state = app.get_mut(self);
            state.pressed = false;
            state.pointer_over = false;
            state.should_perform_actions = false;
        }
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
        let widget = self.widget(app).clone();
        let template = self.template(app, context);
        let mut gesture = GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            .child(template);
        if widget.item.is_enabled {
            // `OnPointerPressed` and `OnPointerReleased`: a press shows `Pressed`, and its
            // release performs the click.
            gesture = gesture
                .on_tap_down(Rc::new(move |app, _| {
                    self.set_state(app, |state| state.pressed = true);
                    self.sync_storyboard(app);
                }))
                .on_tap_up(Rc::new(move |app, _| {
                    self.set_state(app, |state| {
                        state.should_perform_actions = state.pressed;
                        state.pressed = false;
                    });
                    self.sync_storyboard(app);
                }))
                .on_tap_cancel(inset_foundation::Listener::new(move |app| {
                    self.set_state(app, |state| state.pressed = false);
                    self.sync_storyboard(app);
                }))
                .on_tap(inset_foundation::Listener::new(move |app| {
                    // `PerformPointerUpAction`.
                    if std::mem::take(&mut app.get_mut(self).should_perform_actions) {
                        let widget = self.widget(app).clone();
                        (widget.clicked)(app, widget.index);
                    }
                }));
        }
        let index = widget.index;
        let focus_changed = widget.focus_changed.clone();
        // A disabled item takes no focus; the items are reached by the arrow keys, never by Tab.
        let control = Focus::new(gesture)
            .focus_node(widget.focus_node.as_node())
            .can_request_focus(widget.item.is_enabled)
            .skip_traversal(true)
            .on_focus_change(move |app, focused| {
                self.set_state(app, |state| state.focused = focused);
                focus_changed(app, index, focused);
            });
        let control = MouseRegion::new()
            .on_enter(Rc::new(move |app, _| {
                // `OnPointerEntered`: only an enabled item takes `IsPointerOver`.
                if self.widget(app).item.is_enabled {
                    self.set_state(app, |state| state.pointer_over = true);
                    self.sync_storyboard(app);
                }
            }))
            .on_exit(Rc::new(move |app, _| self.clear_flags(app)))
            .child(control);
        // The margin is outside `LayoutRoot`, where the item takes no pointer input.
        Margin::new(ITEM_MARGIN, control).into_widget()
    }
}

/// Converts XAML thickness order into padding.
fn insets(value: [f64; 4]) -> EdgeInsetsGeometry {
    EdgeInsetsGeometry::from_ltrb(value[0], value[1], value[2], value[3])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ComboBoxItemCommonState::*;

    #[test]
    fn an_item_that_appears_selected_keeps_its_pill_in_every_state() {
        assert_eq!(common_state(true, true, true, false, false), Selected);
        assert_eq!(
            common_state(true, true, false, false, false),
            SelectedUnfocused
        );
        assert_eq!(
            common_state(true, true, true, false, true),
            SelectedPointerOver
        );
        assert_eq!(common_state(true, true, true, true, true), SelectedPressed);
        assert!(SelectedPressed.shows_pill());
        assert!(!PointerOver.shows_pill());
    }

    #[test]
    fn a_disabled_item_is_disabled_even_when_selected() {
        assert_eq!(common_state(false, true, true, false, false), Disabled);
        assert_eq!(common_state(true, false, true, true, false), Pressed);
        assert_eq!(common_state(true, false, true, false, false), Normal);
    }
}
