//! XAML `ToggleSwitch` with the Fluent style of `ToggleSwitch_themeresources.xaml`.
//!
//! Template, element by element: the root `Grid` (rows Auto and *) holds `HeaderContentPresenter` in row 0 and, in row 1 aligned top-left, the inner `Grid` whose rows are `ToggleSwitchPreContentMargin`, Auto and `ToggleSwitchPostContentMargin` and whose columns are Auto, 12 and Auto. Its children, in order: `SwitchAreaGrid` (spanning everything, `Margin="0,5"`, the container background, the focus target), `OffContentPresenter` and `OnContentPresenter` (column 2; the `ContentStates` group makes one opaque), `OuterBorder` and `SwitchKnobBounds` (row 1, 40 × 20 rounded rectangles, the off and the on track, crossfaded by `ToggleStates`), `SwitchKnob` (row 1, a 20 × 20 grid aligned left and moved by `KnobTranslateTransform`, holding `SwitchKnobOn` and `SwitchKnobOff`) and `SwitchThumb` (spanning everything, the drag surface).
//!
//! Behaviour is `ToggleSwitch_Partial.cpp`: a tap on the thumb toggles (`TapHandler`); a drag moves the knob with the pointer (`DragStartedHandler`, `DragDeltaHandler` → `MoveDelta`) and toggles at release when the knob passed half the track (`DragCompletedHandler` → `MoveCompleted`).

use crate::{
    BackgroundSizing, Brush, ColumnDefinition, CommonState, ControlBorder, ControlStates,
    FocusVisual, Grid, GridCell, GridLength, RowDefinition, Theme, ThemeResources,
    control_text_style,
};
use inset_animation::{Cubic, Curve};
use inset_embedder::{Clip, Color, FontWeight};
use inset_foundation::{App, Handle, Listener};
use inset_gestures::{
    DragDownDetails, DragEndDetails, DragGestureRecognizer, DragStartBehavior, DragUpdateDetails,
    HorizontalDragGestureRecognizer,
};
use inset_painting::{AlignmentGeometry, EdgeInsetsGeometry, TextStyle};
use inset_rendering::{BoxConstraints, HitTestBehavior};
use inset_services::{KeyEvent, LogicalKeyboardKey};
use inset_widgets::*;
use std::{any::TypeId, fmt, rc::Rc, time::Duration};

/// `OuterBorder` and `SwitchKnobBounds`: the track.
pub const TRACK_WIDTH: f64 = 40.0;
pub const TRACK_HEIGHT: f64 = 20.0;
/// `SwitchKnob`: the grid the knob shapes are arranged in.
pub const KNOB_AREA: f64 = 20.0;
/// The gap column between the switch and its content.
pub const CONTENT_GAP: f64 = 12.0;
/// `FocusVisualMargin` of the style.
pub const FOCUS_VISUAL_MARGIN: [f64; 4] = [-7.0, -3.0, -7.0, -3.0];
/// `SwitchAreaGrid`'s `Margin="0,5"`: left, top, right, bottom.
pub const SWITCH_AREA_MARGIN: [f64; 4] = [0.0, 5.0, 0.0, 5.0];
/// `SwitchKnobOn`'s `CornerRadius` and `SwitchKnobOff`'s `RadiusX` / `RadiusY`.
pub const KNOB_CORNER_RADIUS: f64 = 7.0;
/// `KnobTranslateTransform.X` in the `On` state (`To="20"`): `m_maxKnobTranslation` of `SizeChangedHandler`, the knob bounds' width less the knob's; `m_minKnobTranslation` is 0.
pub const KNOB_TRANSLATION_RANGE: f64 = TRACK_WIDTH - KNOB_AREA;

/// The knob's size per common state: the storyboards animate `SwitchKnobOn` and `SwitchKnobOff` to 12 × 12 (`Normal`, `Disabled`), 14 × 14 (`PointerOver`) and 17 × 14 (`Pressed`) with `ControlFastOutSlowInKeySpline`.
pub fn knob_size(state: CommonState) -> (f64, f64) {
    match state {
        CommonState::Normal | CommonState::Disabled => (12.0, 12.0),
        CommonState::PointerOver => (14.0, 14.0),
        CommonState::Pressed => (17.0, 14.0),
    }
}

/// How long the knob takes to reach `knob_size`: `ControlNormalAnimationDuration` into `Disabled`, `ControlFasterAnimationDuration` into the other states.
pub fn knob_size_duration(r: &ThemeResources, state: CommonState) -> Duration {
    match state {
        CommonState::Disabled => r.control_normal_animation_duration,
        _ => r.control_faster_animation_duration,
    }
}

/// XAML `ControlFastOutSlowInKeySpline` as a curve.
pub fn fast_out_slow_in(r: &ThemeResources) -> Rc<dyn Curve> {
    let [x1, y1, x2, y2] = r.control_fast_out_slow_in_key_spline;
    Rc::new(Cubic::new(x1, y1, x2, y2))
}

/// XAML `Toggled` handler, given the value the switch wants.
pub type ToggledHandler = Rc<dyn Fn(&mut App, bool)>;

/// XAML `ToggleSwitch`: `IsOn` with `Toggled`, `Header`, `OnContent` and `OffContent`.
#[derive(Clone)]
pub struct ToggleSwitch {
    pub is_on: bool,
    /// XAML `Toggled`, with the value the switch wants.
    pub toggled: ToggledHandler,
    pub header: Option<WidgetRef>,
    /// XAML `OnContent`; the control's default is the text "On".
    pub on_content: Option<WidgetRef>,
    /// XAML `OffContent`; the control's default is the text "Off".
    pub off_content: Option<WidgetRef>,
    pub is_enabled: bool,
    pub key: Option<KeyRef>,
}

impl ToggleSwitch {
    pub fn new(is_on: bool, toggled: impl Fn(&mut App, bool) + 'static) -> ToggleSwitch {
        ToggleSwitch {
            is_on,
            toggled: Rc::new(toggled),
            header: None,
            on_content: None,
            off_content: None,
            is_enabled: true,
            key: None,
        }
    }

    pub fn header<K>(mut self, header: impl IntoWidget<K>) -> ToggleSwitch {
        self.header = Some(header.into_widget());
        self
    }

    pub fn on_content<K>(mut self, content: impl IntoWidget<K>) -> ToggleSwitch {
        self.on_content = Some(content.into_widget());
        self
    }

    pub fn off_content<K>(mut self, content: impl IntoWidget<K>) -> ToggleSwitch {
        self.off_content = Some(content.into_widget());
        self
    }

    pub fn is_enabled(mut self, enabled: bool) -> ToggleSwitch {
        self.is_enabled = enabled;
        self
    }

    pub fn key(mut self, key: KeyRef) -> ToggleSwitch {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for ToggleSwitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToggleSwitch")
            .field("is_on", &self.is_on)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

/// The drag of `ToggleSwitch_Partial.cpp`: `m_isDragging`, `m_wasDragged` and `m_knobTranslation`.
pub struct ToggleSwitchState {
    state: StateData<ToggleSwitch>,
    /// `m_isDragging`: the thumb holds the pointer, so `CommonStates` shows `Pressed` and `ToggleStates` `Dragging`.
    is_dragging: bool,
    /// `m_wasDragged`: a `DragDelta` moved horizontally, so the release decides by position instead of leaving the toggle to the tap.
    was_dragged: bool,
    /// `m_knobTranslation`: the horizontal change since the drag started, accumulated unclamped; `SetTranslations` clamps what the knob shows.
    knob_translation: f64,
    focus_node: Option<Handle<FocusNode>>,
    hovered: bool,
    focused: bool,
    handled_key_down: bool,
}

impl StatefulWidget for ToggleSwitch {
    type State = ToggleSwitchState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ToggleSwitchState {
        ToggleSwitchState {
            state: StateData::new(),
            is_dragging: false,
            was_dragged: false,
            knob_translation: 0.0,
            focus_node: None,
            hovered: false,
            focused: false,
            handled_key_down: false,
        }
    }
}

impl State for ToggleSwitchState {
    type Widget = ToggleSwitch;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        crate::InputDevice::initialize(app);
        app.get_mut(self).focus_node = Some(FocusNode::new(app));
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(node) = app.get_mut(self).focus_node.take() {
            node.dispose(app);
            app.destroy(node);
        }
    }

    /// `OnIsEnabledChanged`: disabling ends a drag.
    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &ToggleSwitch) {
        if !self.widget(app).is_enabled {
            let state = app.get_mut(self);
            state.is_dragging = false;
            state.hovered = false;
            state.handled_key_down = false;
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let is_enabled = self.widget(app).is_enabled;
        let state = app.get(self);
        let states = ControlStates {
            common: if !is_enabled {
                CommonState::Disabled
            } else if state.hovered {
                CommonState::PointerOver
            } else {
                CommonState::Normal
            },
            focused: is_enabled && state.focused,
        };
        let node = state.focus_node.unwrap();
        let body = self.template(app, context, states);
        let mut tapped = GestureDetector::new()
            .behavior(HitTestBehavior::Opaque)
            .child(body);
        if is_enabled {
            tapped = tapped.on_tap(Listener::new(move |app| {
                if !app.get(self).is_dragging {
                    self.toggle(app);
                }
            }));
        }
        let detector = FocusableActionDetector::new(tapped)
            .focus_node(node.as_node())
            .enabled(is_enabled)
            .on_focus_change(move |app, focused| {
                let focused = focused && crate::FocusState::current(app).shows_focus_visual();
                self.set_state(app, |state| state.focused = focused);
            })
            .on_show_hover_highlight(move |app, hovered| {
                self.set_state(app, |state| state.hovered = hovered);
            });
        Focus::new(detector)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _, event| self.handle_key(app, event)))
            .into_widget()
    }
}

impl ToggleSwitchState {
    // `HandlesKey` accepts Space and GamepadA.
    fn handle_key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if !self.widget(app).is_enabled {
            return KeyEventResult::Ignored;
        }
        let gamepad = event.logical_key() == LogicalKeyboardKey::GAME_BUTTON_A;
        let handles_key = event.logical_key() == LogicalKeyboardKey::SPACE || gamepad;
        match event {
            KeyEvent::Down(_) | KeyEvent::Repeat(_) if !app.get(self).is_dragging => {
                app.get_mut(self).handled_key_down = handles_key;
                if handles_key {
                    return KeyEventResult::Handled;
                }
            }
            KeyEvent::Up(_) if handles_key => {
                let handled = app.get(self).handled_key_down;
                app.get_mut(self).handled_key_down = false;
                if gamepad || (handled && !app.get(self).is_dragging) {
                    self.toggle(app);
                    return KeyEventResult::Handled;
                }
            }
            _ => {}
        }
        KeyEventResult::Ignored
    }

    /// `Toggle`: `IsOn` flips; here the `Toggled` handler is asked for the flipped value.
    fn toggle(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        if widget.is_enabled {
            (widget.toggled)(app, !widget.is_on);
        }
    }

    /// `DragStartedHandler`: the drag begins where the state left the knob (`GetTranslations`), shown pressed (`UpdateVisualState`).
    fn drag_started(self: Handle<Self>, app: &mut App) {
        let is_on = self.widget(app).is_on;
        self.set_state(app, |state| {
            state.is_dragging = true;
            state.was_dragged = false;
            state.knob_translation = if is_on { KNOB_TRANSLATION_RANGE } else { 0.0 };
        });
    }

    /// `DragDeltaHandler` then `MoveDelta`: a horizontal change moves the knob and marks the drag as moved; vertical movement is allowed and ignored.
    fn drag_delta(self: Handle<Self>, app: &mut App, horizontal_change: f64) {
        if horizontal_change == 0.0 {
            return;
        }
        self.set_state(app, |state| {
            state.was_dragged = true;
            state.knob_translation += horizontal_change;
        });
    }

    /// `DragCompletedHandler` then `MoveCompleted`: the switch toggles when the knob was dragged past half the track towards the other state; `ClearTranslations` returns the knob to its state's end either way.
    fn drag_completed(self: Handle<Self>, app: &mut App) {
        let is_on = self.widget(app).is_on;
        let (was_dragged, knob_translation) = {
            let state = app.get(self);
            (state.was_dragged, state.knob_translation)
        };
        // `(m_maxKnobTranslation - m_minKnobTranslation) / 2`, with `m_minKnobTranslation` 0.
        let half_of_translation_range = KNOB_TRANSLATION_RANGE / 2.0;
        let was_toggled = was_dragged
            && if is_on {
                knob_translation <= half_of_translation_range
            } else {
                knob_translation >= half_of_translation_range
            };
        self.set_state(app, |state| state.is_dragging = false);
        if was_toggled {
            self.toggle(app);
        }
    }

    /// The recogniser gives the drag up: the pointer was lost, or it was released inside the slop and the tap takes it. XAML raises `DragCompleted` for both; without movement `MoveCompleted` only restores the knob, and `Tapped` toggles.
    fn drag_cancelled(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| state.is_dragging = false);
    }

    /// `SwitchThumb`'s `DragStarted` / `DragDelta` / `DragCompleted` as a `HorizontalDragGestureRecognizer`.
    fn drag_factory(self: Handle<Self>) -> GestureRecognizerFactoryRef {
        GestureRecognizerFactoryWithHandlers::<HorizontalDragGestureRecognizer>::new(
            HorizontalDragGestureRecognizer::new,
            move |app, recognizer| {
                // `Thumb` measures every `DragDelta` from the press point, so the movement that
                // made the recogniser win is not discarded.
                recognizer.set_drag_start_behavior(app, DragStartBehavior::Down);
                recognizer.set_on_down(
                    app,
                    Some(Rc::new(move |app: &mut App, _: DragDownDetails| {
                        self.drag_started(app)
                    })),
                );
                recognizer.set_on_update(
                    app,
                    Some(Rc::new(move |app: &mut App, details: DragUpdateDetails| {
                        self.drag_delta(app, details.primary_delta.unwrap_or(0.0))
                    })),
                );
                recognizer.set_on_end(
                    app,
                    Some(Rc::new(move |app: &mut App, _: DragEndDetails| {
                        self.drag_completed(app)
                    })),
                );
                recognizer.set_on_cancel(
                    app,
                    Some(Listener::new(move |app| self.drag_cancelled(app))),
                );
            },
        )
        .into_factory()
    }

    /// The `ControlTemplate` for the current states.
    fn template(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        states: ControlStates,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let is_dragging = app.get(self).is_dragging;
        let resources = ThemeResources::of(app, context);
        // `ChangeVisualState`: a drag holds `CommonStates` in `Pressed`.
        let common = if is_dragging {
            CommonState::Pressed
        } else {
            states.common
        };
        let brushes = brushes(&resources, common);
        let header = widget
            .header
            .clone()
            .map(|header| header_content_presenter(&resources, header, brushes.header));
        let inner = self.inner_grid(app, &widget, &resources, &brushes, common, states.focused);
        // `HorizontalAlignment="Left"` `VerticalAlignment="Top"`: sized to the content and placed top-left in the star row.
        let inner = GridCell::new(
            Align::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .width_factor(1.0)
                .height_factor(1.0)
                .child(inner),
        )
        .row(1)
        .into_widget();
        // The root `Grid`: `Background`, `BorderBrush`, `BorderThickness` and `CornerRadius` are
        // template bindings the style leaves unset, so it paints nothing.
        let root = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::STAR),
            ])
            .children(header.into_iter().chain([inner]));
        // The style's `MinWidth`.
        ConstrainedBox::new(
            BoxConstraints::new().min_width(resources.toggle_switch_theme_min_width),
        )
        .child(root)
        .into_widget()
    }

    /// The inner `Grid`: the switch, the gap and the content, in rows of `ToggleSwitchPreContentMargin`, Auto and `ToggleSwitchPostContentMargin`.
    fn inner_grid(
        self: Handle<Self>,
        app: &mut App,
        widget: &ToggleSwitch,
        resources: &ThemeResources,
        brushes: &SwitchBrushes,
        common: CommonState,
        focused: bool,
    ) -> Grid {
        let is_on = widget.is_on;
        let text = control_text_style(
            resources,
            resources.control_content_theme_font_size,
            FontWeight::W400,
            brushes.foreground,
        );
        let off_content = widget
            .off_content
            .clone()
            .unwrap_or_else(|| Text::new("Off").into_widget());
        let on_content = widget
            .on_content
            .clone()
            .unwrap_or_else(|| Text::new("On").into_widget());
        Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::pixel(
                    resources.toggle_switch_pre_content_margin,
                )),
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::pixel(
                    resources.toggle_switch_post_content_margin,
                )),
            ])
            .column_definitions([
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::pixel(CONTENT_GAP)).max_width(CONTENT_GAP),
                ColumnDefinition::new(GridLength::AUTO),
            ])
            .children([
                GridCell::new(switch_area_grid(
                    resources,
                    resources.theme,
                    brushes.container,
                    focused,
                ))
                .row_span(3)
                .column_span(3)
                .into_widget(),
                GridCell::new(content_presenter(text.clone(), off_content, !is_on))
                    .row_span(3)
                    .column(2)
                    .into_widget(),
                GridCell::new(content_presenter(text, on_content, is_on))
                    .row_span(3)
                    .column(2)
                    .into_widget(),
                GridCell::new(outer_border(resources, brushes, is_on))
                    .row(1)
                    .into_widget(),
                GridCell::new(switch_knob_bounds(resources, brushes, is_on))
                    .row(1)
                    .into_widget(),
                GridCell::new(self.switch_knob(app, resources, brushes, common, is_on))
                    .row(1)
                    .into_widget(),
                GridCell::new(self.switch_thumb(widget.is_enabled))
                    .row_span(3)
                    .column_span(3)
                    .into_widget(),
            ])
    }

    /// `SwitchKnob`: `HorizontalAlignment="Left"`, 20 × 20, moved by `KnobTranslateTransform`. `X` is 20 in the `On` state, the clamped `m_knobTranslation` while dragging (`SetTranslations`) and 0 otherwise; the `ToggleStates` transitions move it with `RepositionThemeAnimation`, here `control_fast_animation_duration` with `ControlFastOutSlowInKeySpline`.
    fn switch_knob(
        self: Handle<Self>,
        app: &App,
        resources: &ThemeResources,
        brushes: &SwitchBrushes,
        common: CommonState,
        is_on: bool,
    ) -> WidgetRef {
        let state = app.get(self);
        let curve = fast_out_slow_in(resources);
        let (translation, duration) = if state.is_dragging {
            let translation = state.knob_translation.clamp(0.0, KNOB_TRANSLATION_RANGE);
            (translation, Duration::ZERO)
        } else if is_on {
            (
                KNOB_TRANSLATION_RANGE,
                resources.control_fast_animation_duration,
            )
        } else {
            (0.0, resources.control_fast_animation_duration)
        };
        let (knob_width, knob_height) = knob_size(common);
        // `VerticalAlignment` `Stretch` with an explicit `Height`: centred in the 20 pt grid.
        let top = (KNOB_AREA - knob_height) / 2.0;
        let shape = |child: WidgetRef, left: f64| {
            AnimatedPositioned::new(child, knob_size_duration(resources, common))
                .curve(curve.clone())
                .left(left)
                .top(top)
                .width(knob_width)
                .height(knob_height)
                .into_widget()
        };
        let knob = Stack::new().children(vec![
            shape(
                switch_knob_on(resources, brushes, is_on),
                knob_on_left(common, knob_width),
            ),
            shape(
                switch_knob_off(resources, brushes, is_on),
                knob_off_left(common, knob_width),
            ),
        ]);
        // `Width="20" Height="20"` at the left of the cell; the transform moves the knob beyond
        // that box, so the stack does not clip.
        let translated = Stack::new().clip_behavior(Clip::None).children(vec![
            AnimatedPositioned::new(knob, duration)
                .curve(curve)
                .left(translation)
                .top(0.0)
                .width(KNOB_AREA)
                .height(KNOB_AREA)
                .into_widget(),
        ]);
        Align::new()
            .alignment(AlignmentGeometry::CENTER_LEFT)
            .width_factor(1.0)
            .height_factor(1.0)
            .child(
                SizedBox::new()
                    .width(KNOB_AREA)
                    .height(KNOB_AREA)
                    .child(translated),
            )
            .into_widget()
    }

    /// `SwitchThumb`: a `Thumb` templated as a transparent rectangle over the whole switch. Its `Tapped` is the tap of the `GestureDetector` around the template; its `DragStarted` / `DragDelta` / `DragCompleted` are a `HorizontalDragGestureRecognizer` here, in the same arena, so a press that moves past the slop becomes the drag and cancels the tap, and one that does not stays the tap — as `Thumb`'s drag and `Tapped` compose in XAML.
    fn switch_thumb(self: Handle<Self>, is_enabled: bool) -> WidgetRef {
        let gestures = if is_enabled {
            vec![(
                TypeId::of::<HorizontalDragGestureRecognizer>(),
                self.drag_factory(),
            )]
        } else {
            Vec::new()
        };
        RawGestureDetector::new()
            .gestures(gestures)
            .behavior(HitTestBehavior::Opaque)
            .child(SizedBox::new())
            .into_widget()
    }
}

/// The track, knob and content brushes for one state; the `{ThemeResource …}` values the `CommonStates` storyboards name.
struct SwitchBrushes {
    fill_off: Color,
    stroke_off: Color,
    fill_on: Color,
    stroke_on: Color,
    knob_off: Color,
    knob_on: Color,
    container: Color,
    foreground: Color,
    header: Color,
}

fn brushes(r: &ThemeResources, state: CommonState) -> SwitchBrushes {
    match state {
        CommonState::Normal => SwitchBrushes {
            fill_off: r.toggle_switch_fill_off,
            stroke_off: r.toggle_switch_stroke_off,
            fill_on: r.toggle_switch_fill_on,
            stroke_on: r.toggle_switch_stroke_on,
            knob_off: r.toggle_switch_knob_fill_off,
            knob_on: r.toggle_switch_knob_fill_on,
            container: r.toggle_switch_container_background,
            foreground: r.toggle_switch_content_foreground,
            header: r.toggle_switch_header_foreground,
        },
        CommonState::PointerOver => SwitchBrushes {
            fill_off: r.toggle_switch_fill_off_pointer_over,
            stroke_off: r.toggle_switch_stroke_off_pointer_over,
            fill_on: r.toggle_switch_fill_on_pointer_over,
            stroke_on: r.toggle_switch_stroke_on_pointer_over,
            knob_off: r.toggle_switch_knob_fill_off_pointer_over,
            knob_on: r.toggle_switch_knob_fill_on_pointer_over,
            container: r.toggle_switch_container_background_pointer_over,
            foreground: r.toggle_switch_content_foreground,
            header: r.toggle_switch_header_foreground,
        },
        CommonState::Pressed => SwitchBrushes {
            fill_off: r.toggle_switch_fill_off_pressed,
            stroke_off: r.toggle_switch_stroke_off_pressed,
            fill_on: r.toggle_switch_fill_on_pressed,
            stroke_on: r.toggle_switch_stroke_on_pressed,
            knob_off: r.toggle_switch_knob_fill_off_pressed,
            knob_on: r.toggle_switch_knob_fill_on_pressed,
            container: r.toggle_switch_container_background_pressed,
            foreground: r.toggle_switch_content_foreground,
            header: r.toggle_switch_header_foreground,
        },
        CommonState::Disabled => SwitchBrushes {
            fill_off: r.toggle_switch_fill_off_disabled,
            stroke_off: r.toggle_switch_stroke_off_disabled,
            fill_on: r.toggle_switch_fill_on_disabled,
            stroke_on: r.toggle_switch_stroke_on_disabled,
            knob_off: r.toggle_switch_knob_fill_off_disabled,
            knob_on: r.toggle_switch_knob_fill_on_disabled,
            container: r.toggle_switch_container_background_disabled,
            foreground: r.toggle_switch_content_foreground_disabled,
            header: r.toggle_switch_header_foreground_disabled,
        },
    }
}

/// `HeaderContentPresenter`: `x:DeferLoadStrategy="Lazy"` and `Visibility="Collapsed"` until a `Header` is set, so it exists only with one; `Margin` `ToggleSwitchTopHeaderMargin`, `Foreground` per state, `TextWrapping="Wrap"`.
fn header_content_presenter(r: &ThemeResources, header: WidgetRef, foreground: Color) -> WidgetRef {
    let [left, top, right, bottom] = r.toggle_switch_top_header_margin;
    let text = control_text_style(
        r,
        r.control_content_theme_font_size,
        FontWeight::W400,
        foreground,
    );
    GridCell::new(
        Padding::new(EdgeInsetsGeometry::from_ltrb(left, top, right, bottom))
            .child(DefaultTextStyle::new(text, header)),
    )
    .row(0)
    .into_widget()
}

/// `SwitchAreaGrid`: spans the inner grid with `Margin="0,5"` and the container background (transparent in every state) under the style's `CornerRadius`; `Control.IsTemplateFocusTarget`, so the system focus visual draws around it with the style's `FocusVisualMargin`.
fn switch_area_grid(
    r: &ThemeResources,
    theme: Theme,
    container: Color,
    focused: bool,
) -> WidgetRef {
    let [left, top, right, bottom] = SWITCH_AREA_MARGIN;
    let area = Grid::new()
        .background(Brush::Solid(container))
        .corner_radius(r.control_corner_radius[0]);
    Padding::new(EdgeInsetsGeometry::from_ltrb(left, top, right, bottom))
        .child(
            FocusVisual::new(area, theme)
                .visible(focused)
                .margin(FOCUS_VISUAL_MARGIN)
                .corner_radius(r.control_corner_radius[0]),
        )
        .into_widget()
}

/// `OffContentPresenter` / `OnContentPresenter`: `Foreground` per state, `HorizontalContentAlignment` `Left` (the style), `VerticalContentAlignment` `Center` (`CControl`'s default), and `Opacity` from the `ContentStates` group, whose `Duration="0"` animations switch it discretely.
fn content_presenter(text: TextStyle, content: WidgetRef, visible: bool) -> WidgetRef {
    Opacity::new(if visible { 1.0 } else { 0.0 })
        .child(
            Align::new()
                .alignment(AlignmentGeometry::CENTER_LEFT)
                .width_factor(1.0)
                .height_factor(1.0)
                .child(DefaultTextStyle::new(text, content)),
        )
        .into_widget()
}

/// A 40 × 20 `Rectangle` with `RadiusX` / `RadiusY` 10, faded by the `ToggleStates` storyboards over `ControlFasterAnimationDuration`.
fn track(
    r: &ThemeResources,
    fill: Color,
    stroke: Color,
    stroke_thickness: f64,
    visible: bool,
) -> WidgetRef {
    AnimatedOpacity::new(
        if visible { 1.0 } else { 0.0 },
        r.control_faster_animation_duration,
    )
    .child(
        SizedBox::new()
            .width(TRACK_WIDTH)
            .height(TRACK_HEIGHT)
            .child(
                ControlBorder::new(Brush::Solid(fill), Brush::Solid(stroke))
                    .border_thickness(stroke_thickness)
                    .corner_radius(TRACK_HEIGHT / 2.0),
            ),
    )
    .into_widget()
}

/// `OuterBorder`: the off track, `StrokeThickness` `ToggleSwitchOuterBorderStrokeThickness`, faded out by the `On` state.
fn outer_border(r: &ThemeResources, brushes: &SwitchBrushes, is_on: bool) -> WidgetRef {
    track(
        r,
        brushes.fill_off,
        brushes.stroke_off,
        r.toggle_switch_outer_border_stroke_thickness,
        !is_on,
    )
}

/// `SwitchKnobBounds`: the on track, `StrokeThickness` `ToggleSwitchOnStrokeThickness`, faded in by the `On` state.
fn switch_knob_bounds(r: &ThemeResources, brushes: &SwitchBrushes, is_on: bool) -> WidgetRef {
    track(
        r,
        brushes.fill_on,
        brushes.stroke_on,
        r.toggle_switch_on_stroke_thickness,
        is_on,
    )
}

/// `SwitchKnobOn`: a `Border` with `ToggleSwitchKnobFillOn` and the circle elevation `BorderBrush` (its `BorderThickness` is unset, so 0), `BackgroundSizing="OuterBorderEdge"`, `CornerRadius` 7, faded in by the `On` state.
fn switch_knob_on(resources: &ThemeResources, brushes: &SwitchBrushes, is_on: bool) -> WidgetRef {
    AnimatedOpacity::new(
        if is_on { 1.0 } else { 0.0 },
        resources.control_faster_animation_duration,
    )
    .child(
        ControlBorder::new(
            Brush::Solid(brushes.knob_on),
            Brush::CircleElevation(resources.toggle_switch_knob_stroke_on),
        )
        .border_thickness(0.0)
        .background_sizing(BackgroundSizing::OuterBorderEdge)
        .corner_radius(KNOB_CORNER_RADIUS),
    )
    .into_widget()
}

/// `SwitchKnobOff`: a `Rectangle` filled with `ToggleSwitchKnobFillOff`, `RadiusX` / `RadiusY` 7, faded out by the `On` state.
fn switch_knob_off(r: &ThemeResources, brushes: &SwitchBrushes, is_on: bool) -> WidgetRef {
    AnimatedOpacity::new(
        if is_on { 0.0 } else { 1.0 },
        r.control_faster_animation_duration,
    )
    .child(
        ControlBorder::new(
            Brush::Solid(brushes.knob_off),
            Brush::Solid(Color::from_argb(0, 0, 0, 0)),
        )
        .border_thickness(0.0)
        .corner_radius(KNOB_CORNER_RADIUS),
    )
    .into_widget()
}

/// XAML `HorizontalAlignment` of an element with an explicit `Width`.
#[derive(Clone, Copy)]
enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

/// `FrameworkElement::ArrangeCore`: where an element `width` wide, with `alignment` and a `margin` (left, right), lands in a slot `slot` wide.
fn arranged_left(slot: f64, width: f64, alignment: HorizontalAlignment, margin: (f64, f64)) -> f64 {
    let (left, right) = margin;
    match alignment {
        HorizontalAlignment::Left => left,
        HorizontalAlignment::Center => left + (slot - left - right - width) / 2.0,
        HorizontalAlignment::Right => slot - right - width,
    }
}

/// `SwitchKnobOn` in `SwitchKnob`: `HorizontalAlignment="Center"` with `Margin="0,0,1,0"`; the `Pressed` setters make it `Right` with `Margin="0,0,3,0"`, so it stretches towards the off end.
fn knob_on_left(state: CommonState, width: f64) -> f64 {
    match state {
        CommonState::Pressed => {
            arranged_left(KNOB_AREA, width, HorizontalAlignment::Right, (0.0, 3.0))
        }
        _ => arranged_left(KNOB_AREA, width, HorizontalAlignment::Center, (0.0, 1.0)),
    }
}

/// `SwitchKnobOff` in `SwitchKnob`: `HorizontalAlignment="Center"` with `Margin="-1,0,0,0"`; the `Pressed` setters make it `Left` with `Margin="3,0,0,0"`, so it stretches towards the on end.
fn knob_off_left(state: CommonState, width: f64) -> f64 {
    match state {
        CommonState::Pressed => {
            arranged_left(KNOB_AREA, width, HorizontalAlignment::Left, (3.0, 0.0))
        }
        _ => arranged_left(KNOB_AREA, width, HorizontalAlignment::Center, (-1.0, 0.0)),
    }
}
