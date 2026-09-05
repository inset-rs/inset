//! Flutter counterpart: `cupertino/segmented_control.dart`.
//!
//! The `Semantics` wrappers around each segment wait with accessibility.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

use indexmap::IndexMap;
use reveal_animation::{Animatable, Animation, AnimationBehavior, AnimationController, ColorTween};
use reveal_embedder::{
    Color, Offset, Paint, PaintStyle, RSuperellipse, Radius, Rect, Size, Stroke,
};
use reveal_embedder::{TextBaseline, TextDirection};
use reveal_foundation::{App, Handle, K_IS_WEB, Listener, ValueChanged};
use reveal_painting::{AnyColor, Axis, ClipContext, EdgeInsetsGeometry, draw_rsuperellipse};
use reveal_rendering::{
    AnyRenderBox, AnyRenderObject, BaselineOffset, BoxConstraints, BoxHitTestResult, BoxParentData,
    ContainerBoxParentData, ContainerParentData, ContainerParentDataMixin,
    ContainerRenderObjectData, ContainerRenderObjectMixin, HitTestBehavior, PaintingContext,
    ParentData, PipelineOwner, RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData,
    RenderHandle, RenderObject, RenderObjectData,
};
use reveal_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use reveal_services::{MouseCursor, SystemMouseCursors};
use reveal_widgets::{
    Action, Actions, AnyAction, AnyFocusNode, AnyRadioGroupRegistry, BuildContext, Center,
    DefaultTextStyle, Directionality, Focus, FocusNode, FocusNodeLeaf, GestureDetector, GlobalKey,
    IconTheme, IconThemeData, IntoWidget, KeyEventResult, KeyRef, MouseRegion,
    MultiChildRenderObjectWidget, Padding, RadioClient, RadioClientData, RadioGroup,
    RenderObjectWidget, State, StateData, StatefulWidget, TickerProviderStateMixin,
    TickerProviderStateMixinData, UnconstrainedBox, VoidCallbackAction, VoidCallbackIntent,
    WidgetRef,
};

use crate::theme::CupertinoTheme;

/// Minimum padding from the edges of the segmented control to the edges of the encompassing
/// widget.
const K_HORIZONTAL_ITEM_PADDING: EdgeInsetsGeometry = EdgeInsetsGeometry::symmetric(0.0, 16.0);

/// Minimum height of the segmented control.
const K_MIN_SEGMENTED_CONTROL_HEIGHT: f64 = 28.0;

/// The default color used for the text of the disabled segment.
const K_DISABLE_TEXT_COLOR: Color = Color::from_argb(115, 122, 122, 122);

/// The duration of the fade animation used to transition when a new widget is selected.
const K_FADE_DURATION: Duration = Duration::from_millis(165);

/// An iOS-style segmented control.
///
/// Displays the widgets provided in the map of [`children`](Self::children) in a horizontal
/// list. Used to select between a number of mutually exclusive options. When one option in the
/// segmented control is selected, the other options in the segmented control cease to be
/// selected.
///
/// A segmented control can feature any widget as one of the values in its map of
/// [`children`](Self::children). The type `T` is the type of the keys used to identify each
/// widget and determine which widget is selected. Keys must be of consistent types and must be
/// comparable. The ordering of the keys determines the order of the widgets in the segmented
/// control.
///
/// When the state of the segmented control changes, the widget calls the
/// [`on_value_changed`](Self::on_value_changed) callback. The map key associated with the newly
/// selected widget is returned in the [`on_value_changed`](Self::on_value_changed) callback.
/// Typically, widgets that use a segmented control will listen for the
/// [`on_value_changed`](Self::on_value_changed) callback and rebuild the segmented control with
/// a new [`group_value`](Self::group_value) to update which option is currently selected.
///
/// The [`children`](Self::children) will be displayed in the order of the keys in the map. The
/// height of the segmented control is determined by the height of the tallest widget provided
/// as a value in the map of [`children`](Self::children). The width of each child in the
/// segmented control will be equal to the width of the widest child, unless the combined width
/// of the children is wider than the available horizontal space. In this case, the available
/// horizontal space is divided by the number of provided [`children`](Self::children) to
/// determine the width of each widget. The selection area for each of the widgets in the map of
/// [`children`](Self::children) will then be expanded to fill the calculated space, so each
/// widget will appear to have the same dimensions.
///
/// A segmented control may optionally be created with custom colors. The
/// [`unselected_color`](Self::unselected_color), [`selected_color`](Self::selected_color),
/// [`border_color`](Self::border_color), and [`pressed_color`](Self::pressed_color) arguments
/// can be used to override the segmented control's colors from [`CupertinoTheme`] defaults. The
/// [`disabled_color`](Self::disabled_color) and
/// [`disabled_text_color`](Self::disabled_text_color) set the background and text colors of the
/// segment when it is disabled.
///
/// The segmented control can be disabled by adding children to the set of
/// [`disabled_children`](Self::disabled_children). If the child is not present in the set, it is
/// enabled by default.
///
/// See also:
///
///  * <https://developer.apple.com/design/human-interface-guidelines/ios/controls/segmented-controls/>
pub struct CupertinoSegmentedControl<T: Copy + Eq + Hash + Debug + 'static> {
    /// See [`Widget::key`](reveal_widgets::Widget::key).
    pub key: Option<KeyRef>,

    /// The identifying keys and corresponding widget values in the segmented control.
    ///
    /// The map must have more than one entry, and it keeps its insertion order.
    pub children: IndexMap<T, WidgetRef>,

    /// The identifier of the widget that is currently selected.
    ///
    /// This must be one of the keys in the map of [`children`](Self::children). If this
    /// attribute is `None`, no widget will be initially selected.
    pub group_value: Option<T>,

    /// The callback that is called when a new option is tapped.
    ///
    /// The segmented control passes the newly selected widget's associated key to the callback
    /// but does not actually change state until the parent widget rebuilds the segmented
    /// control with the new [`group_value`](Self::group_value).
    pub on_value_changed: ValueChanged<T>,

    /// The color used to fill the backgrounds of unselected widgets and as the text color of
    /// the selected widget.
    ///
    /// Defaults to [`CupertinoTheme`]'s `primary_contrasting_color` if `None`.
    pub unselected_color: Option<AnyColor>,

    /// The color used to fill the background of the selected widget and as the text color of
    /// unselected widgets.
    ///
    /// Defaults to [`CupertinoTheme`]'s `primary_color` if `None`.
    pub selected_color: Option<AnyColor>,

    /// The color used as the border around each widget.
    ///
    /// Defaults to [`CupertinoTheme`]'s `primary_color` if `None`.
    pub border_color: Option<AnyColor>,

    /// The color used to fill the background of the widget the user is temporarily interacting
    /// with through a long press or drag.
    ///
    /// Defaults to the [`selected_color`](Self::selected_color) at 20% opacity if `None`.
    pub pressed_color: Option<AnyColor>,

    /// The color used to fill the background of the segment when it is disabled.
    ///
    /// If `None`, this color will be 50% opacity of the
    /// [`selected_color`](Self::selected_color) when the segment is selected. If the segment is
    /// unselected, this color will be set to [`unselected_color`](Self::unselected_color).
    pub disabled_color: Option<AnyColor>,

    /// The color used for the text of the segment when it is disabled.
    pub disabled_text_color: Option<AnyColor>,

    /// The segmented control will be placed inside this padding.
    ///
    /// Defaults to `EdgeInsetsGeometry::symmetric(0.0, 16.0)`.
    pub padding: Option<EdgeInsetsGeometry>,

    /// The set of identifying keys that correspond to the segments that should be disabled.
    ///
    /// All segments are enabled by default.
    pub disabled_children: Vec<T>,
}

impl<T: Copy + Eq + Hash + Debug + 'static> CupertinoSegmentedControl<T> {
    /// Creates an iOS-style segmented control bar; Dart's optional named arguments are the
    /// setters.
    ///
    /// The `children` argument keeps its insertion order, and its length must be greater than
    /// one.
    ///
    /// Each widget value in the map of `children` must have an associated key that uniquely
    /// identifies this widget. This key is what will be returned in the `on_value_changed`
    /// callback when a new value from the `children` map is selected.
    pub fn new(
        children: IndexMap<T, WidgetRef>,
        on_value_changed: ValueChanged<T>,
    ) -> CupertinoSegmentedControl<T> {
        debug_assert!(children.len() >= 2);
        CupertinoSegmentedControl {
            key: None,
            children,
            group_value: None,
            on_value_changed,
            unselected_color: None,
            selected_color: None,
            border_color: None,
            pressed_color: None,
            disabled_color: None,
            disabled_text_color: None,
            padding: None,
            disabled_children: Vec::new(),
        }
    }

    /// Dart `CupertinoSegmentedControl(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoSegmentedControl<T> {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoSegmentedControl(groupValue:)`.
    pub fn group_value(mut self, group_value: T) -> CupertinoSegmentedControl<T> {
        debug_assert!(
            self.children.contains_key(&group_value),
            "The groupValue must be either null or one of the keys in the children map."
        );
        self.group_value = Some(group_value);
        self
    }

    /// Dart `CupertinoSegmentedControl(unselectedColor:)`.
    pub fn unselected_color(mut self, color: AnyColor) -> CupertinoSegmentedControl<T> {
        self.unselected_color = Some(color);
        self
    }

    /// Dart `CupertinoSegmentedControl(selectedColor:)`.
    pub fn selected_color(mut self, color: AnyColor) -> CupertinoSegmentedControl<T> {
        self.selected_color = Some(color);
        self
    }

    /// Dart `CupertinoSegmentedControl(borderColor:)`.
    pub fn border_color(mut self, color: AnyColor) -> CupertinoSegmentedControl<T> {
        self.border_color = Some(color);
        self
    }

    /// Dart `CupertinoSegmentedControl(pressedColor:)`.
    pub fn pressed_color(mut self, color: AnyColor) -> CupertinoSegmentedControl<T> {
        self.pressed_color = Some(color);
        self
    }

    /// Dart `CupertinoSegmentedControl(disabledColor:)`.
    pub fn disabled_color(mut self, color: AnyColor) -> CupertinoSegmentedControl<T> {
        self.disabled_color = Some(color);
        self
    }

    /// Dart `CupertinoSegmentedControl(disabledTextColor:)`.
    pub fn disabled_text_color(mut self, color: AnyColor) -> CupertinoSegmentedControl<T> {
        self.disabled_text_color = Some(color);
        self
    }

    /// Dart `CupertinoSegmentedControl(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> CupertinoSegmentedControl<T> {
        self.padding = Some(padding);
        self
    }

    /// Dart `CupertinoSegmentedControl(disabledChildren:)`.
    pub fn disabled_children(
        mut self,
        disabled_children: impl IntoIterator<Item = T>,
    ) -> CupertinoSegmentedControl<T> {
        self.disabled_children = disabled_children.into_iter().collect();
        self
    }
}

impl<T: Copy + Eq + Hash + Debug + 'static> Debug for CupertinoSegmentedControl<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoSegmentedControl")
            .field("children", &self.children.len())
            .finish_non_exhaustive()
    }
}

impl<T: Copy + Eq + Hash + Debug + 'static> StatefulWidget for CupertinoSegmentedControl<T> {
    type State = SegmentedControlState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SegmentedControlState<T> {
        SegmentedControlState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::default(),
            pressed_key: None,
            action_map: HashMap::new(),
            selection_controllers: Vec::new(),
            child_tweens: Vec::new(),
            segment_keys: HashMap::new(),
            forward_background_color_tween: None,
            reverse_background_color_tween: None,
            text_color_tween: None,
            selected_color: None,
            unselected_color: None,
            border_color: None,
            pressed_color: None,
            selected_disabled_color: None,
            unselected_disabled_color: None,
            disabled_text_color: None,
        }
    }
}

/// A wrapper widget that implements [`RadioClient`] for each segment button.
///
/// Dart's `_SegmentButton`; public because its state is, and a `State` names its widget.
pub struct SegmentButton<T: Copy + Eq + Hash + Debug + 'static> {
    key: Option<KeyRef>,
    /// The value this segment stands for.
    value: T,
    child: WidgetRef,
    enabled: bool,
}

impl<T: Copy + Eq + Hash + Debug + 'static> Debug for SegmentButton<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SegmentButton")
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
}

impl<T: Copy + Eq + Hash + Debug + 'static> StatefulWidget for SegmentButton<T> {
    type State = SegmentButtonState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SegmentButtonState<T> {
        SegmentButtonState {
            state: StateData::new(),
            radio_client: RadioClientData::new(),
            focus_node: None,
        }
    }
}

/// Dart's `_SegmentButtonState`.
pub struct SegmentButtonState<T: Copy + Eq + Hash + Debug + 'static> {
    state: StateData<SegmentButton<T>>,
    radio_client: RadioClientData<T>,
    focus_node: Option<Handle<FocusNode>>,
}

impl<T: Copy + Eq + Hash + Debug + 'static> SegmentButtonState<T> {
    fn node(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self).focus_node.expect("created in init_state")
    }

    /// Dart's `_SegmentButtonState.requestFocus`.
    pub fn request_focus(self: Handle<Self>, app: &mut App) {
        if self.widget(app).enabled {
            self.node(app).as_node().request_focus(app, None);
        }
    }

    fn registry_for_widget(self: Handle<Self>, app: &mut App) -> Option<AnyRadioGroupRegistry<T>> {
        let context = self.context(app);
        if self.widget(app).enabled {
            RadioGroup::<T>::maybe_of(app, context)
        } else {
            None
        }
    }
}

impl<T: Copy + Eq + Hash + Debug + 'static> RadioClient<T> for SegmentButtonState<T> {
    reveal_widgets::radio_client_accessors!(T);

    fn tristate(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    fn radio_value(self: Handle<Self>, app: &App) -> T {
        self.widget(app).value
    }

    fn enabled(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).enabled
    }

    fn focus_node(self: Handle<Self>, app: &App) -> AnyFocusNode {
        self.node(app).as_node()
    }
}

impl<T: Copy + Eq + Hash + Debug + 'static> State for SegmentButtonState<T> {
    type Widget = SegmentButton<T>;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let node = FocusNode::new(app);
        let label = format!("CupertinoSegmentedControl[{:?}]", self.widget(app).value);
        node.as_node().set_debug_label(app, Some(label));
        app.get_mut(self).focus_node = Some(node);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let registry = self.registry_for_widget(app);
        self.set_registry(app, registry);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SegmentButton<T>) {
        if old_widget.enabled != self.widget(app).enabled {
            let registry = self.registry_for_widget(app);
            self.set_registry(app, registry);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.set_registry(app, None);
        self.node(app).as_node().dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let node = self.node(app).as_node();
        let enabled = self.widget(app).enabled;
        let child = self.widget(app).child.clone();
        Focus::new(child)
            .focus_node(node)
            .can_request_focus(enabled)
            .on_key_event(Rc::new(|_app, _node, _event| KeyEventResult::Ignored))
            .into_widget()
    }
}

/// Dart's `_SegmentedControlState`.
pub struct SegmentedControlState<T: Copy + Eq + Hash + Debug + 'static> {
    state: StateData<CupertinoSegmentedControl<T>>,
    ticker_provider: TickerProviderStateMixinData,
    pressed_key: Option<T>,
    /// Dart builds a `VoidCallbackAction` inline in `build`; an action is an arena object, so
    /// this one is created once (see `PORTING.md`).
    action_map: HashMap<std::any::TypeId, AnyAction>,
    selection_controllers: Vec<Handle<AnimationController>>,
    child_tweens: Vec<Handle<ColorTween>>,
    segment_keys: HashMap<T, GlobalKey>,
    forward_background_color_tween: Option<Handle<ColorTween>>,
    reverse_background_color_tween: Option<Handle<ColorTween>>,
    text_color_tween: Option<Handle<ColorTween>>,
    selected_color: Option<AnyColor>,
    unselected_color: Option<AnyColor>,
    border_color: Option<AnyColor>,
    pressed_color: Option<AnyColor>,
    selected_disabled_color: Option<AnyColor>,
    unselected_disabled_color: Option<AnyColor>,
    disabled_text_color: Option<AnyColor>,
}

impl<T: Copy + Eq + Hash + Debug + 'static> SegmentedControlState<T> {
    /// Dart's `createAnimationController`.
    fn create_animation_controller(
        self: Handle<Self>,
        app: &mut App,
    ) -> Handle<AnimationController> {
        let controller = AnimationController::create(
            app,
            Some(0.0),
            Some(K_FADE_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        controller.add_listener(
            app,
            Listener::handle_method(self, SegmentedControlState::<T>::handle_animation_tick),
        );
        controller
    }

    /// The `setState` Dart's controller listener performs: the background and text colors have
    /// changed.
    fn handle_animation_tick(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {});
    }

    /// Dart's `_updateColors`.
    fn update_colors(self: Handle<Self>, app: &mut App) -> bool {
        debug_assert!(
            self.mounted(app),
            "This should only be called after didUpdateDependencies"
        );
        let context = self.context(app);
        let mut changed = false;
        let disabled_text_color = self
            .widget(app)
            .disabled_text_color
            .clone()
            .unwrap_or(AnyColor::new(K_DISABLE_TEXT_COLOR));
        if app.get(self).disabled_text_color != Some(disabled_text_color.clone()) {
            changed = true;
            app.get_mut(self).disabled_text_color = Some(disabled_text_color);
        }
        let theme = CupertinoTheme::of(app, context);
        let selected_color = self
            .widget(app)
            .selected_color
            .clone()
            .unwrap_or_else(|| theme.primary_color());
        if app.get(self).selected_color != Some(selected_color.clone()) {
            changed = true;
            app.get_mut(self).selected_color = Some(selected_color.clone());
        }
        let unselected_color = self
            .widget(app)
            .unselected_color
            .clone()
            .unwrap_or_else(|| theme.primary_contrasting_color());
        if app.get(self).unselected_color != Some(unselected_color.clone()) {
            changed = true;
            app.get_mut(self).unselected_color = Some(unselected_color.clone());
        }
        let selected_disabled_color = self
            .widget(app)
            .disabled_color
            .clone()
            .unwrap_or_else(|| with_opacity(&selected_color, 0.5));
        let unselected_disabled_color = self
            .widget(app)
            .disabled_color
            .clone()
            .unwrap_or_else(|| unselected_color.clone());
        if app.get(self).selected_disabled_color != Some(selected_disabled_color.clone())
            || app.get(self).unselected_disabled_color != Some(unselected_disabled_color.clone())
        {
            changed = true;
            let this = app.get_mut(self);
            this.selected_disabled_color = Some(selected_disabled_color);
            this.unselected_disabled_color = Some(unselected_disabled_color);
        }
        let border_color = self
            .widget(app)
            .border_color
            .clone()
            .unwrap_or_else(|| theme.primary_color());
        if app.get(self).border_color != Some(border_color.clone()) {
            changed = true;
            app.get_mut(self).border_color = Some(border_color);
        }
        let pressed_color = self
            .widget(app)
            .pressed_color
            .clone()
            .unwrap_or_else(|| with_opacity(&theme.primary_color(), 0.2));
        if app.get(self).pressed_color != Some(pressed_color.clone()) {
            changed = true;
            app.get_mut(self).pressed_color = Some(pressed_color);
        }

        let (pressed, selected, unselected) = {
            let this = app.get(self);
            (
                color_of(&this.pressed_color),
                color_of(&this.selected_color),
                color_of(&this.unselected_color),
            )
        };
        // Dart replaces the three tweens with fresh objects here; a `_childTweens` entry that
        // still points at an older one only survives while every colour is unchanged, in which
        // case the replacement holds the same end points. Writing the end points into the same
        // three arena objects is that, without a slot per rebuild.
        let forward = self.tween(app, |state| &mut state.forward_background_color_tween);
        forward.set_begin(app, pressed);
        forward.set_end(app, selected);
        let reverse = self.tween(app, |state| &mut state.reverse_background_color_tween);
        reverse.set_begin(app, unselected);
        reverse.set_end(app, selected);
        let text = self.tween(app, |state| &mut state.text_color_tween);
        text.set_begin(app, selected);
        text.set_end(app, unselected);
        changed
    }

    /// The tween in the named slot, created on first use.
    fn tween(
        self: Handle<Self>,
        app: &mut App,
        slot: fn(&mut SegmentedControlState<T>) -> &mut Option<Handle<ColorTween>>,
    ) -> Handle<ColorTween> {
        if let Some(tween) = *slot(app.get_mut(self)) {
            return tween;
        }
        let tween = ColorTween::new(app, None, None);
        *slot(app.get_mut(self)) = Some(tween);
        tween
    }

    /// Dart's `_updateAnimationControllers`.
    fn update_animation_controllers(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            self.mounted(app),
            "This should only be called after didUpdateDependencies"
        );
        for controller in std::mem::take(&mut app.get_mut(self).selection_controllers) {
            controller.dispose(app);
            app.destroy(controller);
        }
        app.get_mut(self).child_tweens.clear();

        let keys: Vec<T> = self.widget(app).children.keys().copied().collect();
        let group_value = self.widget(app).group_value;
        for key in keys {
            let controller = self.create_animation_controller(app);
            let tween = if group_value == Some(key) {
                controller.set_value(app, 1.0);
                app.get(self)
                    .reverse_background_color_tween
                    .expect("update_colors ran first")
            } else {
                app.get(self)
                    .forward_background_color_tween
                    .expect("update_colors ran first")
            };
            let this = app.get_mut(self);
            this.child_tweens.push(tween);
            this.selection_controllers.push(controller);
        }
    }

    fn on_tap_down(self: Handle<Self>, app: &mut App, current_key: T) {
        if app.get(self).pressed_key.is_none() && Some(current_key) != self.widget(app).group_value
        {
            self.set_state(app, |state| state.pressed_key = Some(current_key));
        }
    }

    fn on_tap_cancel(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| state.pressed_key = None);
    }

    fn on_tap(self: Handle<Self>, app: &mut App, current_key: T) {
        if Some(current_key) != app.get(self).pressed_key {
            return;
        }
        if !self.widget(app).disabled_children.contains(&current_key) {
            self.request_segment_focus(app, current_key);

            if Some(current_key) != self.widget(app).group_value {
                let on_value_changed = Rc::clone(&self.widget(app).on_value_changed);
                on_value_changed(app, current_key);
            }
        }
        self.set_state(app, |state| state.pressed_key = None);
    }

    /// Dart's `_segmentKeys[currentKey]?.currentState?.requestFocus()`.
    fn request_segment_focus(self: Handle<Self>, app: &mut App, current_key: T) {
        let Some(key) = app.get(self).segment_keys.get(&current_key).cloned() else {
            return;
        };
        if let Some(state) = key.current_state::<SegmentButtonState<T>>(app) {
            state.request_focus(app);
        }
    }

    /// Dart's `getTextColor`.
    fn text_color(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        current_key: T,
    ) -> Option<AnyColor> {
        if self.widget(app).disabled_children.contains(&current_key) {
            return app.get(self).disabled_text_color.clone();
        }
        let controller = app.get(self).selection_controllers[index];
        if controller.is_animating(app) {
            let tween = app.get(self).text_color_tween.expect("update_colors ran");
            return tween
                .transform(app, controller.value(app))
                .map(AnyColor::new);
        }
        if self.widget(app).group_value == Some(current_key) {
            return app.get(self).unselected_color.clone();
        }
        app.get(self).selected_color.clone()
    }

    /// Dart's `getBackgroundColor`.
    fn background_color(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        current_key: T,
    ) -> Option<AnyColor> {
        if self.widget(app).disabled_children.contains(&current_key) {
            return if self.widget(app).group_value == Some(current_key) {
                app.get(self).selected_disabled_color.clone()
            } else {
                app.get(self).unselected_disabled_color.clone()
            };
        }
        let controller = app.get(self).selection_controllers[index];
        if controller.is_animating(app) {
            let tween = app.get(self).child_tweens[index];
            return tween
                .transform(app, controller.value(app))
                .map(AnyColor::new);
        }
        if self.widget(app).group_value == Some(current_key) {
            return app.get(self).selected_color.clone();
        }
        if app.get(self).pressed_key == Some(current_key) {
            return app.get(self).pressed_color.clone();
        }
        app.get(self).unselected_color.clone()
    }
}

impl<T: Copy + Eq + Hash + Debug + 'static> TickerProviderStateMixin for SegmentedControlState<T> {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

impl<T: Copy + Eq + Hash + Debug + 'static> TickerProviderObject for SegmentedControlState<T> {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl<T: Copy + Eq + Hash + Debug + 'static> State for SegmentedControlState<T> {
    type Widget = CupertinoSegmentedControl<T>;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let void_callback = VoidCallbackAction::new(app);
        app.get_mut(self).action_map.insert(
            std::any::TypeId::of::<VoidCallbackIntent>(),
            Action::as_action(void_callback),
        );
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        if self.update_colors(app) {
            self.update_animation_controllers(app);
        }
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &CupertinoSegmentedControl<T>,
    ) {
        let old_length = old_widget.children.len();
        let old_group_value = old_widget.group_value;
        if self.update_colors(app) || old_length != self.widget(app).children.len() {
            self.update_animation_controllers(app);
        }

        if old_group_value != self.widget(app).group_value {
            let keys: Vec<T> = self.widget(app).children.keys().copied().collect();
            for (index, key) in keys.into_iter().enumerate() {
                let controller = app.get(self).selection_controllers[index];
                if self.widget(app).group_value == Some(key) {
                    let tween = app
                        .get(self)
                        .forward_background_color_tween
                        .expect("update_colors ran");
                    app.get_mut(self).child_tweens[index] = tween;
                    controller.forward(app, None);
                } else {
                    let tween = app
                        .get(self)
                        .reverse_background_color_tween
                        .expect("update_colors ran");
                    app.get_mut(self).child_tweens[index] = tween;
                    controller.reverse(app, None);
                }
            }
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for action in std::mem::take(&mut app.get_mut(self).action_map).into_values() {
            app.destroy(action.id());
        }
        for controller in std::mem::take(&mut app.get_mut(self).selection_controllers) {
            controller.dispose(app);
            app.destroy(controller);
        }
        TickerProviderStateMixin::dispose(self, app);
        let this = app.get_mut(self);
        this.child_tweens.clear();
        let tweens = [
            this.forward_background_color_tween.take(),
            this.reverse_background_color_tween.take(),
            this.text_color_tween.take(),
        ];
        for tween in tweens.into_iter().flatten() {
            app.destroy(tween);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut gesture_children: Vec<WidgetRef> = Vec::new();
        let mut background_colors: Vec<Color> = Vec::new();
        let mut selected_index: Option<usize> = None;
        let mut pressed_index: Option<usize> = None;
        let keys: Vec<T> = self.widget(app).children.keys().copied().collect();
        for (index, current_key) in keys.into_iter().enumerate() {
            if self.widget(app).group_value == Some(current_key) {
                selected_index = Some(index);
            }
            if app.get(self).pressed_key == Some(current_key) {
                pressed_index = Some(index);
            }

            let text_color = self.text_color(app, index, current_key);
            let mut text_style = DefaultTextStyle::of(app, context).style.copy_with();
            let mut icon_theme = IconThemeData::new();
            if let Some(color) = text_color {
                text_style = text_style.color(color.clone());
                icon_theme = icon_theme.color(color);
            }

            let child = Center::new().child(
                self.widget(app)
                    .children
                    .get(&current_key)
                    .expect("the key comes from the map")
                    .clone(),
            );

            let is_enabled = !self.widget(app).disabled_children.contains(&current_key);

            let segment_key = app
                .get_mut(self)
                .segment_keys
                .entry(current_key)
                .or_insert_with(GlobalKey::new)
                .clone();

            let mut detector = GestureDetector::new().behavior(HitTestBehavior::Opaque);
            if is_enabled {
                detector = detector
                    .on_tap_down(Rc::new(move |app: &mut App, _details| {
                        self.on_tap_down(app, current_key);
                    }))
                    .on_tap_cancel(Listener::new(move |app: &mut App| {
                        self.on_tap_cancel(app);
                    }));
            }
            let detector = detector
                .on_tap(Listener::new(move |app: &mut App| {
                    if is_enabled {
                        self.request_segment_focus(app, current_key);
                    }
                    self.on_tap(app, current_key);
                }))
                .child(IconTheme::new(
                    icon_theme,
                    DefaultTextStyle::new(text_style, child),
                ));

            let segment = SegmentButton {
                key: Some(Rc::new(segment_key)),
                value: current_key,
                enabled: is_enabled,
                child: MouseRegion::new()
                    .cursor(if K_IS_WEB {
                        SystemMouseCursors::CLICK.into()
                    } else {
                        <dyn MouseCursor>::defer()
                    })
                    .child(detector)
                    .into_widget(),
            };

            let background_color = self
                .background_color(app, index, current_key)
                .expect("every segment has a background color");
            background_colors.push(background_color.color());
            gesture_children.push(segment.into_widget());
        }

        let border_color = app
            .get(self)
            .border_color
            .clone()
            .expect("update_colors ran")
            .color();
        let segmented_box = SegmentedControlRenderWidget {
            key: None,
            children: gesture_children,
            selected_index,
            pressed_index,
            background_colors,
            border_color,
        };

        let padding = self
            .widget(app)
            .padding
            .unwrap_or(K_HORIZONTAL_ITEM_PADDING);
        let actions = app.get(self).action_map.clone();
        Actions::new(actions, {
            let mut group = RadioGroup::<T>::new(
                Rc::new(move |app: &mut App, value: Option<T>| {
                    if let Some(value) = value
                        && !self.widget(app).disabled_children.contains(&value)
                    {
                        let on_value_changed = Rc::clone(&self.widget(app).on_value_changed);
                        on_value_changed(app, value);
                    }
                }),
                Padding::new(padding).child(
                    UnconstrainedBox::new()
                        .constrained_axis(Axis::Horizontal)
                        .child(segmented_box),
                ),
            );
            if let Some(group_value) = self.widget(app).group_value {
                group = group.group_value(group_value);
            }
            group
        })
        .into_widget()
    }
}

/// Dart's `Color.withOpacity`, whose replacement is `withValues(alpha:)`.
fn with_opacity(color: &AnyColor, opacity: f64) -> AnyColor {
    AnyColor::new(
        color
            .color()
            .with_values(Some(opacity), None, None, None, None),
    )
}

/// The plain [`Color`] of an optional [`AnyColor`], for a tween end point.
fn color_of(color: &Option<AnyColor>) -> Option<Color> {
    color.as_ref().map(AnyColor::color)
}

/// Dart's `_SegmentedControlRenderWidget`.
#[derive(Debug)]
struct SegmentedControlRenderWidget {
    key: Option<KeyRef>,
    children: Vec<WidgetRef>,
    selected_index: Option<usize>,
    pressed_index: Option<usize>,
    background_colors: Vec<Color>,
    border_color: Color,
}

impl RenderObjectWidget for SegmentedControlRenderWidget {
    type RenderObject = RenderSegmentedControl;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::of(app, context);
        RenderSegmentedControl::new(
            app,
            self.selected_index,
            self.pressed_index,
            text_direction,
            self.background_colors.clone(),
            self.border_color,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderSegmentedControl>,
    ) {
        let text_direction = Directionality::of(app, context);
        render_object.set_text_direction(app, text_direction);
        render_object.set_selected_index(app, self.selected_index);
        render_object.set_pressed_index(app, self.pressed_index);
        render_object.set_background_colors(app, self.background_colors.clone());
        render_object.set_border_color(app, self.border_color);
    }
}

impl MultiChildRenderObjectWidget for SegmentedControlRenderWidget {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// Dart's `_SegmentedControlContainerBoxParentData`.
#[derive(Debug, Default)]
pub struct SegmentedControlContainerBoxParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,
    /// The rounded superellipse painted behind and around this segment.
    pub surrounding_rect: Option<RSuperellipse>,
}

impl SegmentedControlContainerBoxParentData {
    /// Parent data for a segment that has not been laid out yet.
    pub const fn new() -> SegmentedControlContainerBoxParentData {
        SegmentedControlContainerBoxParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
            surrounding_rect: None,
        }
    }
}

impl fmt::Display for SegmentedControlContainerBoxParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.box_parent_data)
    }
}

impl ParentData for SegmentedControlContainerBoxParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<SegmentedControlContainerBoxParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<SegmentedControlContainerBoxParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for SegmentedControlContainerBoxParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for SegmentedControlContainerBoxParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

/// Dart's `_NextChild`: the walk direction `_layoutRects` follows.
type NextChild =
    fn(RenderHandle<RenderSegmentedControl>, &App, AnyRenderBox) -> Option<AnyRenderBox>;

/// Dart's `_RenderSegmentedControl`.
///
/// Dart's unused `T` type parameter is dropped: nothing in the render object reads it.
pub struct RenderSegmentedControl {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    selected_index: Option<usize>,
    pressed_index: Option<usize>,
    text_direction: TextDirection,
    background_colors: Vec<Color>,
    border_color: Color,
}

impl RenderSegmentedControl {
    /// Creates the render object that lays out and paints the segments.
    pub fn new(
        app: &mut App,
        selected_index: Option<usize>,
        pressed_index: Option<usize>,
        text_direction: TextDirection,
        background_colors: Vec<Color>,
        border_color: Color,
    ) -> RenderHandle<RenderSegmentedControl> {
        RenderHandle::new_box(
            app,
            RenderSegmentedControl {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                selected_index,
                pressed_index,
                text_direction,
                background_colors,
                border_color,
            },
        )
    }

    /// The index of the selected segment, if any.
    pub fn selected_index(self: RenderHandle<Self>, app: &App) -> Option<usize> {
        self.get(app).selected_index
    }

    /// Sets [`selected_index`](Self::selected_index).
    pub fn set_selected_index(self: RenderHandle<Self>, app: &mut App, value: Option<usize>) {
        if self.get(app).selected_index == value {
            return;
        }
        self.get_mut(app).selected_index = value;
        self.mark_needs_paint(app);
    }

    /// The index of the segment the user is pressing, if any.
    pub fn pressed_index(self: RenderHandle<Self>, app: &App) -> Option<usize> {
        self.get(app).pressed_index
    }

    /// Sets [`pressed_index`](Self::pressed_index).
    pub fn set_pressed_index(self: RenderHandle<Self>, app: &mut App, value: Option<usize>) {
        if self.get(app).pressed_index == value {
            return;
        }
        self.get_mut(app).pressed_index = value;
        self.mark_needs_paint(app);
    }

    /// The direction the segments are laid out in.
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> TextDirection {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: TextDirection) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_layout(app);
    }

    /// The background colors of the segments, in child order.
    pub fn background_colors(self: RenderHandle<Self>, app: &App) -> Vec<Color> {
        self.get(app).background_colors.clone()
    }

    /// Sets [`background_colors`](Self::background_colors).
    pub fn set_background_colors(self: RenderHandle<Self>, app: &mut App, value: Vec<Color>) {
        if self.get(app).background_colors == value {
            return;
        }
        self.get_mut(app).background_colors = value;
        self.mark_needs_paint(app);
    }

    /// The color of the border drawn around every segment.
    pub fn border_color(self: RenderHandle<Self>, app: &App) -> Color {
        self.get(app).border_color
    }

    /// Sets [`border_color`](Self::border_color).
    pub fn set_border_color(self: RenderHandle<Self>, app: &mut App, value: Color) {
        if self.get(app).border_color == value {
            return;
        }
        self.get_mut(app).border_color = value;
        self.mark_needs_paint(app);
    }

    /// Dart's `_layoutRects`.
    fn layout_rects(
        self: RenderHandle<Self>,
        app: &mut App,
        next_child: NextChild,
        left_child: Option<AnyRenderBox>,
        right_child: Option<AnyRenderBox>,
    ) {
        let mut child = left_child;
        let mut start = 0.0;
        while let Some(current) = child {
            let child_offset = Offset::new(start, 0.0);
            let size = current.size(app);
            let child_rect = Rect::from_ltwh(start, 0.0, size.width(), size.height());
            let r_child_rect = if Some(current) == left_child {
                RSuperellipse::from_rect_and_corners(
                    child_rect,
                    Radius::circular(3.0),
                    Radius::ZERO,
                    Radius::ZERO,
                    Radius::circular(3.0),
                )
            } else if Some(current) == right_child {
                RSuperellipse::from_rect_and_corners(
                    child_rect,
                    Radius::ZERO,
                    Radius::circular(3.0),
                    Radius::circular(3.0),
                    Radius::ZERO,
                )
            } else {
                RSuperellipse::from_rect_and_corners(
                    child_rect,
                    Radius::ZERO,
                    Radius::ZERO,
                    Radius::ZERO,
                    Radius::ZERO,
                )
            };
            let parent_data = current
                .as_object()
                .parent_data_of_mut::<SegmentedControlContainerBoxParentData>(app);
            parent_data.set_offset(child_offset);
            parent_data.surrounding_rect = Some(r_child_rect);
            start += size.width();
            child = next_child(self, app, current);
        }
    }

    /// Dart's `_calculateChildSize`.
    fn calculate_child_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let child_count = self.child_count(app) as f64;
        let mut max_height = K_MIN_SEGMENTED_CONTROL_HEIGHT;
        let mut child_width = constraints.min_width / child_count;
        let mut child = self.first_child(app);
        while let Some(current) = child {
            child_width = child_width.max(current.get_max_intrinsic_width(app, f64::INFINITY));
            child = self.child_after(app, current);
        }
        child_width = child_width.min(constraints.max_width / child_count);
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let box_height = current.get_max_intrinsic_height(app, child_width);
            max_height = max_height.max(box_height);
            child = self.child_after(app, current);
        }
        Size::new(child_width, max_height)
    }

    /// Dart's `_computeOverallSizeFromChildSize`.
    fn compute_overall_size_from_child_size(
        self: RenderHandle<Self>,
        app: &mut App,
        child_size: Size,
    ) -> Size {
        let child_count = self.child_count(app) as f64;
        let constraints = RenderBox::constraints(self, app);
        constraints.constrain(Size::new(
            child_size.width() * child_count,
            child_size.height(),
        ))
    }

    /// Dart's `_paintChild`.
    fn paint_child(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        child: AnyRenderBox,
        child_index: usize,
    ) {
        let (surrounding_rect, child_offset) = {
            let parent_data = child
                .as_object()
                .parent_data_of::<SegmentedControlContainerBoxParentData>(app);
            (
                parent_data
                    .surrounding_rect
                    .expect("laid out before painting"),
                parent_data.offset(),
            )
        };
        let background_color = self.get(app).background_colors[child_index];
        let border_color = self.get(app).border_color;

        let fill = Paint {
            color: background_color.into(),
            style: PaintStyle::Fill,
            ..Paint::default()
        };
        draw_rsuperellipse(context.canvas(), surrounding_rect.shift(offset), &fill);
        let stroke = Paint {
            color: border_color.into(),
            style: PaintStyle::Stroke(Stroke::new(1.0)),
            ..Paint::default()
        };
        draw_rsuperellipse(context.canvas(), surrounding_rect.shift(offset), &stroke);

        context.paint_child(app, child.as_object(), child_offset + offset);
    }
}

impl ContainerRenderObjectMixin for RenderSegmentedControl {
    type ChildType = AnyRenderBox;
    type ParentDataType = SegmentedControlContainerBoxParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }
}

impl RenderBoxContainerDefaultsMixin for RenderSegmentedControl {}

impl RenderObject for RenderSegmentedControl {
    reveal_rendering::render_object_accessors!();

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = RenderBox::constraints(self, app);
        let child_size = self.calculate_child_size(app, constraints);

        let child_constraints =
            BoxConstraints::tight_for(Some(child_size.width()), Some(child_size.height()));

        let mut child = self.first_child(app);
        while let Some(current) = child {
            current.layout(app, child_constraints, true);
            child = self.child_after(app, current);
        }

        let (next_child, left_child, right_child): (NextChild, _, _) =
            match self.text_direction(app) {
                TextDirection::Rtl => (
                    ContainerRenderObjectMixin::child_before,
                    self.last_child(app),
                    self.first_child(app),
                ),
                TextDirection::Ltr => (
                    ContainerRenderObjectMixin::child_after,
                    self.first_child(app),
                    self.last_child(app),
                ),
            };
        self.layout_rects(app, next_child, left_child, right_child);

        let size = self.compute_overall_size_from_child_size(app, child_size);
        self.set_size(app, size);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let mut child = self.first_child(app);
        let mut index = 0;
        while let Some(current) = child {
            self.paint_child(app, context, offset, current, index);
            child = self.child_after(app, current);
            index += 1;
        }
    }
}

impl RenderBox for RenderSegmentedControl {
    reveal_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<SegmentedControlContainerBoxParentData>(app) {
            child.set_parent_data(app, SegmentedControlContainerBoxParentData::new());
        }
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let mut child = self.first_child(app);
        let mut min_width: f64 = 0.0;
        while let Some(current) = child {
            let child_width = current.get_min_intrinsic_width(app, height);
            min_width = min_width.max(child_width);
            child = self.child_after(app, current);
        }
        min_width * self.child_count(app) as f64
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let mut child = self.first_child(app);
        let mut max_width: f64 = 0.0;
        while let Some(current) = child {
            let child_width = current.get_max_intrinsic_width(app, height);
            max_width = max_width.max(child_width);
            child = self.child_after(app, current);
        }
        max_width * self.child_count(app) as f64
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let mut child = self.first_child(app);
        let mut min_height: f64 = 0.0;
        while let Some(current) = child {
            let child_height = current.get_min_intrinsic_height(app, width);
            min_height = min_height.max(child_height);
            child = self.child_after(app, current);
        }
        min_height
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let mut child = self.first_child(app);
        let mut max_height: f64 = 0.0;
        while let Some(current) = child {
            let child_height = current.get_max_intrinsic_height(app, width);
            max_height = max_height.max(child_height);
            child = self.child_after(app, current);
        }
        max_height
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.default_compute_distance_to_highest_actual_baseline(app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child_size = self.calculate_child_size(app, constraints);
        let child_constraints = BoxConstraints::tight(child_size);

        let mut baseline_offset = BaselineOffset::NO_BASELINE;
        let mut child = self.first_child(app);
        while let Some(current) = child {
            baseline_offset = baseline_offset.min_of(BaselineOffset(current.get_dry_baseline(
                app,
                child_constraints,
                baseline,
            )));
            child = self.child_after(app, current);
        }
        baseline_offset.offset()
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let child_size = self.calculate_child_size(app, constraints);
        self.compute_overall_size_from_child_size(app, child_size)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let mut child = self.last_child(app);
        while let Some(current) = child {
            let (surrounding_rect, offset, previous_sibling) = {
                let parent_data = current
                    .as_object()
                    .parent_data_of::<SegmentedControlContainerBoxParentData>(app);
                (
                    parent_data.surrounding_rect.expect("laid out"),
                    parent_data.offset(),
                    parent_data.previous_sibling(),
                )
            };
            if surrounding_rect.outer_rect().contains(position) {
                return result.add_with_paint_offset(
                    Some(offset),
                    position,
                    |result, local_offset| {
                        debug_assert_eq!(local_offset, position - offset);
                        current.hit_test(app, result, local_offset)
                    },
                );
            }
            child = previous_sibling;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::RefCell;

    use reveal_embedder::{PointerChange, PointerData, PointerDataPacket, PointerDeviceKind};
    use reveal_gestures::GestureBinding;
    use reveal_services::{
        HardwareKeyboard, KeyDownEvent, KeyEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
    };
    use reveal_widgets::{GlobalKey, SizedBox, WidgetsBinding};

    use super::*;
    use crate::test_support::{build, pump};

    const SELECTED: Color = Color::from_argb(255, 0, 255, 0);
    const UNSELECTED: Color = Color::from_argb(255, 0, 0, 255);
    const PRESSED: Color = Color::from_argb(255, 255, 0, 0);
    const BORDER: Color = Color::from_argb(255, 0, 0, 0);

    /// The test view is 2x: logical coordinates double into the packet.
    fn send(app: &mut App, change: PointerChange, at: Offset, time: Duration) {
        GestureBinding::instance(app).handle_pointer_data_packet(
            app,
            PointerDataPacket::new(vec![PointerData {
                change,
                kind: PointerDeviceKind::Touch,
                time_stamp: time,
                pointer_identifier: 1,
                physical_x: at.dx() * 2.0,
                physical_y: at.dy() * 2.0,
                ..PointerData::default()
            }]),
        );
        app.drain_microtasks();
    }

    /// A host that owns the group value, as a Dart caller's `StatefulWidget` does.
    #[derive(Debug)]
    struct Host {
        key: Option<KeyRef>,
        changes: Rc<RefCell<Vec<u32>>>,
        disabled: Vec<u32>,
        text_direction: TextDirection,
    }

    impl StatefulWidget for Host {
        type State = HostState;

        fn key(&self) -> Option<&KeyRef> {
            self.key.as_ref()
        }

        fn create_state(&self) -> HostState {
            HostState {
                state: StateData::new(),
                group_value: 0,
                control_key: GlobalKey::new(),
            }
        }
    }

    struct HostState {
        state: StateData<Host>,
        group_value: u32,
        control_key: GlobalKey,
    }

    impl State for HostState {
        type Widget = Host;
        reveal_widgets::state_accessors!();

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let changes = Rc::clone(&self.widget(app).changes);
            let mut children: IndexMap<u32, WidgetRef> = IndexMap::new();
            for value in 0..3u32 {
                children.insert(
                    value,
                    SizedBox::new().width(40.0).height(20.0).into_widget(),
                );
            }
            let control = CupertinoSegmentedControl::new(
                children,
                Rc::new(move |app: &mut App, value: u32| {
                    changes.borrow_mut().push(value);
                    self.set_state(app, |state| state.group_value = value);
                }),
            )
            .group_value(app.get(self).group_value)
            .selected_color(AnyColor::new(SELECTED))
            .unselected_color(AnyColor::new(UNSELECTED))
            .pressed_color(AnyColor::new(PRESSED))
            .border_color(AnyColor::new(BORDER))
            .disabled_children(self.widget(app).disabled.clone())
            .key(Rc::new(app.get(self).control_key.clone()));
            reveal_widgets::Directionality::new(self.widget(app).text_direction, control)
                .into_widget()
        }
    }

    struct Mounted {
        cell: Rc<AppCell>,
        binding: Handle<WidgetsBinding>,
        host: Handle<HostState>,
        control: Handle<SegmentedControlState<u32>>,
        changes: Rc<RefCell<Vec<u32>>>,
    }

    fn find<T: RenderObject>(app: &App, node: AnyRenderObject) -> Option<RenderHandle<T>> {
        if let Some(found) = node.downcast::<T>(app) {
            return Some(found);
        }
        let mut found = None;
        node.visit_children(app, &mut |child| {
            if found.is_none() {
                found = find::<T>(app, child);
            }
        });
        found
    }

    impl Mounted {
        fn render(&self) -> RenderHandle<RenderSegmentedControl> {
            let root = self
                .binding
                .root_element(&self.cell.borrow())
                .expect("a mounted root element")
                .find_render_object(&self.cell.borrow())
                .expect("a mounted view has a render object");
            find::<RenderSegmentedControl>(&self.cell.borrow(), root)
                .expect("the control's render object")
        }

        fn segments(&self) -> Vec<AnyRenderBox> {
            self.render().children_as_list(&self.cell.borrow())
        }

        fn segment_offset(&self, index: usize) -> Offset {
            self.segments()[index]
                .as_object()
                .parent_data_of::<SegmentedControlContainerBoxParentData>(&self.cell.borrow())
                .offset()
        }

        /// The centre of a segment in the view's coordinate system.
        fn segment_center(&self, index: usize) -> Offset {
            let child = self.segments()[index];
            let size = child.size(&self.cell.borrow());
            let local =
                self.segment_offset(index) + Offset::new(size.width() / 2.0, size.height() / 2.0);
            self.render()
                .as_box()
                .local_to_global(&self.cell.borrow(), local, None)
        }

        fn backgrounds(&self) -> Vec<Color> {
            self.render().background_colors(&self.cell.borrow())
        }

        fn settle(&mut self, at: Duration) {
            pump(&mut self.cell.borrow_mut(), at);
        }
    }

    fn mount(disabled: Vec<u32>, text_direction: TextDirection) -> Mounted {
        let cell = crate::test_support::test_cell();
        let changes: Rc<RefCell<Vec<u32>>> = Rc::default();
        let host_key = GlobalKey::new();
        build(
            &cell,
            Host {
                key: Some(Rc::new(host_key.clone())),
                changes: Rc::clone(&changes),
                disabled,
                text_direction,
            }
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let host = host_key
            .current_state::<HostState>(&mut app)
            .expect("the host mounted");
        let control = app
            .get(host)
            .control_key
            .clone()
            .current_state::<SegmentedControlState<u32>>(&mut app)
            .expect("the control mounted");
        let binding = WidgetsBinding::instance(&mut app);
        drop(app);
        Mounted {
            cell,
            binding,
            host,
            control,
            changes,
        }
    }

    #[test]
    fn every_segment_takes_the_same_width_under_a_tight_parent() {
        let mounted = mount(Vec::new(), TextDirection::Ltr);
        let widths: Vec<f64> = mounted
            .segments()
            .iter()
            .map(|child| child.size(&mounted.cell.borrow()).width())
            .collect();
        let expected = (400.0 - 32.0) / 3.0;
        assert_eq!(widths, vec![expected; 3]);
        assert_eq!(
            mounted.render().size(&mounted.cell.borrow()),
            Size::new(400.0 - 32.0, K_MIN_SEGMENTED_CONTROL_HEIGHT),
            "the control is as tall as the minimum and as wide as the padded view"
        );
        assert_eq!(
            mounted.segment_offset(1),
            Offset::new(expected, 0.0),
            "segments sit next to each other"
        );
    }

    #[test]
    fn the_selected_segment_takes_the_selected_colour() {
        let mut mounted = mount(Vec::new(), TextDirection::Ltr);
        assert_eq!(
            mounted.backgrounds(),
            vec![SELECTED, UNSELECTED, UNSELECTED]
        );
        assert_eq!(
            mounted.render().border_color(&mounted.cell.borrow()),
            BORDER
        );

        let tap = mounted.segment_center(1);
        send(
            &mut mounted.cell.borrow_mut(),
            PointerChange::Down,
            tap,
            Duration::ZERO,
        );
        send(
            &mut mounted.cell.borrow_mut(),
            PointerChange::Up,
            tap,
            Duration::from_millis(10),
        );
        assert_eq!(*mounted.changes.borrow(), [1], "the second segment's key");

        mounted.settle(Duration::from_millis(20));
        mounted.settle(Duration::from_millis(400));
        assert_eq!(mounted.cell.borrow().get(mounted.host).group_value, 1);
        assert_eq!(
            mounted.backgrounds(),
            vec![UNSELECTED, SELECTED, UNSELECTED],
            "the colours follow the selection once the fade ends"
        );
    }

    #[test]
    fn a_disabled_segment_ignores_a_tap() {
        let mounted = mount(vec![2], TextDirection::Ltr);
        let tap = mounted.segment_center(2);
        send(
            &mut mounted.cell.borrow_mut(),
            PointerChange::Down,
            tap,
            Duration::ZERO,
        );
        send(
            &mut mounted.cell.borrow_mut(),
            PointerChange::Up,
            tap,
            Duration::from_millis(10),
        );
        assert!(
            mounted.changes.borrow().is_empty(),
            "a disabled segment reports nothing"
        );
        assert_eq!(mounted.cell.borrow().get(mounted.host).group_value, 0);
    }

    #[test]
    fn a_held_segment_takes_the_pressed_colour() {
        let mut mounted = mount(Vec::new(), TextDirection::Ltr);
        let tap = mounted.segment_center(1);
        send(
            &mut mounted.cell.borrow_mut(),
            PointerChange::Down,
            tap,
            Duration::ZERO,
        );
        mounted.settle(Duration::from_millis(10));
        assert_eq!(
            mounted.cell.borrow().get(mounted.control).pressed_key,
            Some(1),
            "the segment under the finger is the pressed key"
        );
        assert_eq!(mounted.backgrounds(), vec![SELECTED, PRESSED, UNSELECTED]);

        send(
            &mut mounted.cell.borrow_mut(),
            PointerChange::Cancel,
            tap,
            Duration::from_millis(20),
        );
        mounted.settle(Duration::from_millis(30));
        assert_eq!(mounted.cell.borrow().get(mounted.control).pressed_key, None);
        assert_eq!(
            mounted.backgrounds(),
            vec![SELECTED, UNSELECTED, UNSELECTED]
        );
    }

    #[test]
    fn a_right_to_left_control_rounds_the_rightmost_segment_first() {
        let mounted = mount(Vec::new(), TextDirection::Rtl);
        let width = (400.0 - 32.0) / 3.0;
        assert_eq!(
            mounted.segment_offset(0),
            Offset::new(2.0 * width, 0.0),
            "the first child is laid out on the right"
        );
        assert_eq!(mounted.segment_offset(2), Offset::new(0.0, 0.0));
        let leftmost = mounted.segments()[2]
            .as_object()
            .parent_data_of::<SegmentedControlContainerBoxParentData>(&mounted.cell.borrow())
            .surrounding_rect
            .expect("laid out");
        assert_eq!(leftmost.tl_radius_x, 3.0, "the leftmost corner is rounded");
        assert_eq!(leftmost.tr_radius_x, 0.0);
    }

    #[test]
    fn an_arrow_key_selects_the_next_segment_through_the_radio_group() {
        let mounted = mount(Vec::new(), TextDirection::Ltr);
        let key = mounted
            .cell
            .borrow()
            .get(mounted.control)
            .segment_keys
            .get(&0)
            .cloned()
            .expect("a key for the first segment");
        let first = key
            .current_state::<SegmentButtonState<u32>>(&mut mounted.cell.borrow_mut())
            .expect("the segment mounted");
        first.request_focus(&mut mounted.cell.borrow_mut());
        mounted.cell.borrow_mut().drain_microtasks();

        let keyboard = HardwareKeyboard::instance(&mut mounted.cell.borrow_mut());
        let arrow_right = KeyEvent::Down(KeyDownEvent::new(
            PhysicalKeyboardKey::ARROW_RIGHT,
            LogicalKeyboardKey::ARROW_RIGHT,
            Duration::ZERO,
        ));
        assert!(
            keyboard.handle_key_event(&mut mounted.cell.borrow_mut(), &arrow_right),
            "the radio group handled the arrow key"
        );
        mounted.cell.borrow_mut().drain_microtasks();
        assert_eq!(*mounted.changes.borrow(), [1]);
    }
}
