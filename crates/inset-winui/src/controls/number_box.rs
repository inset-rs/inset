//! NumberBox.xaml, NumberBox_themeresources.xaml and NumberBox.cpp: a text box for one number.
//! It shows the value formatted, reads what is typed (as arithmetic with `accepts_expression`),
//! keeps the value between `minimum` and `maximum`, and steps it by `small_change` and
//! `large_change` from the arrow and page keys, the mouse wheel and its spin buttons, inline or
//! in a popup.
//!
//! The application owns the value: `value` is what it passes, and `value_changed` asks for
//! another. A value the box asks for is what the box itself reads as its value, and what it
//! shows, until the application next builds it; if the application passed another value by
//! then, the box shows that one. A change the box makes to a value the application passes,
//! coercing it into range, is reported after the frame.

mod formatter;
mod parser;

pub use formatter::{ComparableNumberFormatter, DecimalFormatter, NumberFormatter};

use super::flyout::shadow::PopupShadow;
use super::repeat_button::{RepeatButtonSetters, repeat_button_presenter};
use super::text_control::TextControl;
use crate::*;
use formatter::round_for_display;
use inset_embedder::{
    Color, FontWeight, Offset, Rect, Size, TextAffinity, TextAlign, TextSelection,
};
use inset_foundation::{App, Handle, Listenable, Listener};
use inset_gestures::{GestureBinding, PointerEvent};
use inset_painting::{AlignmentGeometry, transform_rect};
use inset_rendering::{BoxConstraints, SingleChildLayoutDelegate};
use inset_scheduler::FrameCallback;
use inset_services::{KeyEvent, LogicalKeyboardKey, TextEditingValue};
use inset_widgets::Listener as PointerListener;
use inset_widgets::*;
use std::{any::Any, fmt, rc::Rc};

/// XAML `NumberBoxSpinButtonPlacementMode`: where the spin buttons are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NumberBoxSpinButtonPlacementMode {
    /// No spin buttons.
    #[default]
    Hidden,
    /// In a popup beside the box while it has the focus, with an indicator in the box.
    Compact,
    /// Inside the box, after the text.
    Inline,
}

/// XAML `NumberBoxValidationMode`: what becomes of text that is not a number, and of values out
/// of range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NumberBoxValidationMode {
    /// Text that is not a number is replaced by the value, and a value out of range is moved to
    /// the bound it passed.
    #[default]
    InvalidInputOverwritten,
    /// Text that is not a number stays, and a value out of range is kept.
    Disabled,
}

/// XAML `NumberBoxValueChangedEventArgs`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberBoxValueChangedEventArgs {
    /// `OldValue`: the value before the change; NaN for none.
    pub old_value: f64,
    /// `NewValue`: the value the box asks for; NaN for none.
    pub new_value: f64,
}

/// Receives XAML `ValueChanged`.
pub type NumberBoxValueChangedHandler = Rc<dyn Fn(&mut App, NumberBoxValueChangedEventArgs)>;

/// XAML `NumberBox` with the template of `NumberBox.xaml`.
#[derive(Clone)]
pub struct NumberBox {
    /// `Value`: the number shown, NaN for none.
    pub value: f64,

    /// `ValueChanged`: asks the application for another value; the application owns `value`.
    pub value_changed: NumberBoxValueChangedHandler,

    /// `Minimum`: the lowest value; the lowest finite double by default.
    pub minimum: f64,

    /// `Maximum`: the highest value; the highest finite double by default.
    pub maximum: f64,

    /// `SmallChange`: the step of the arrow keys, the wheel and the spin buttons; 1 by default.
    pub small_change: f64,

    /// `LargeChange`: the step of Page Up and Page Down; 10 by default.
    pub large_change: f64,

    /// `Text`: text for the box to read as if it were typed and committed, applied when it
    /// changes; while `value` is NaN it gives the first value.
    pub text: Option<String>,

    /// `Header`: shown above the box.
    pub header: Option<WidgetRef>,

    /// `PlaceholderText`: shown while the box is empty.
    pub placeholder_text: String,

    /// `Description`: shown below the box.
    pub description: Option<WidgetRef>,

    /// `ValidationMode`.
    pub validation_mode: NumberBoxValidationMode,

    /// `SpinButtonPlacementMode`.
    pub spin_button_placement_mode: NumberBoxSpinButtonPlacementMode,

    /// `IsWrapEnabled`: whether a step past one bound goes to the other.
    pub is_wrap_enabled: bool,

    /// `AcceptsExpression`: whether typed arithmetic is computed.
    pub accepts_expression: bool,

    /// `NumberFormatter`: formats the value and reads typed numbers; `None` is NumberBox's own,
    /// a `DecimalFormatter` with one integer digit and no fraction digits. The text is
    /// formatted again when a formatter of another type or value is passed.
    pub number_formatter: Option<Rc<dyn ComparableNumberFormatter>>,

    /// `IsEnabled`.
    pub is_enabled: bool,

    /// `FontSize`: of the text, the header and the inline spin buttons' glyphs; the theme's `ControlContentThemeFontSize` when `None`.
    pub font_size: Option<f64>,

    /// `CornerRadius`: of the box and its inline spin buttons; the theme's `ControlCornerRadius`
    /// when `None`.
    pub corner_radius: Option<f64>,

    /// `SelectionHighlightColor`: overrides the selection fill.
    pub selection_highlight_color: Option<Color>,

    /// `TextAlignment`.
    pub text_alignment: TextAlign,

    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl NumberBox {
    /// Creates a box showing `value`, which asks for others through `value_changed`.
    pub fn new(
        value: f64,
        value_changed: impl Fn(&mut App, NumberBoxValueChangedEventArgs) + 'static,
    ) -> NumberBox {
        NumberBox {
            value,
            value_changed: Rc::new(value_changed),
            minimum: -f64::MAX,
            maximum: f64::MAX,
            small_change: 1.0,
            large_change: 10.0,
            text: None,
            header: None,
            placeholder_text: String::new(),
            description: None,
            validation_mode: NumberBoxValidationMode::InvalidInputOverwritten,
            spin_button_placement_mode: NumberBoxSpinButtonPlacementMode::Hidden,
            is_wrap_enabled: false,
            accepts_expression: false,
            number_formatter: None,
            is_enabled: true,
            font_size: None,
            corner_radius: None,
            selection_highlight_color: None,
            text_alignment: TextAlign::Start,
            key: None,
        }
    }

    /// Sets Minimum.
    pub fn minimum(mut self, value: f64) -> Self {
        self.minimum = value;
        self
    }

    /// Sets Maximum.
    pub fn maximum(mut self, value: f64) -> Self {
        self.maximum = value;
        self
    }

    /// Sets SmallChange.
    pub fn small_change(mut self, value: f64) -> Self {
        self.small_change = value;
        self
    }

    /// Sets LargeChange.
    pub fn large_change(mut self, value: f64) -> Self {
        self.large_change = value;
        self
    }

    /// Sets Text.
    pub fn text(mut self, value: impl Into<String>) -> Self {
        self.text = Some(value.into());
        self
    }

    /// Sets Header.
    pub fn header<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.header = Some(value.into_widget());
        self
    }

    /// Sets PlaceholderText.
    pub fn placeholder_text(mut self, value: impl Into<String>) -> Self {
        self.placeholder_text = value.into();
        self
    }

    /// Sets Description.
    pub fn description<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.description = Some(value.into_widget());
        self
    }

    /// Sets ValidationMode.
    pub fn validation_mode(mut self, value: NumberBoxValidationMode) -> Self {
        self.validation_mode = value;
        self
    }

    /// Sets SpinButtonPlacementMode.
    pub fn spin_button_placement_mode(mut self, value: NumberBoxSpinButtonPlacementMode) -> Self {
        self.spin_button_placement_mode = value;
        self
    }

    /// Sets IsWrapEnabled.
    pub fn is_wrap_enabled(mut self, value: bool) -> Self {
        self.is_wrap_enabled = value;
        self
    }

    /// Sets AcceptsExpression.
    pub fn accepts_expression(mut self, value: bool) -> Self {
        self.accepts_expression = value;
        self
    }

    /// Sets NumberFormatter.
    pub fn number_formatter(mut self, value: impl NumberFormatter + PartialEq + 'static) -> Self {
        self.number_formatter = Some(Rc::new(value));
        self
    }

    /// The formatter in use: `number_formatter`, or NumberBox's own.
    fn formatter(&self) -> Rc<dyn NumberFormatter> {
        match &self.number_formatter {
            Some(formatter) => Rc::clone(formatter) as Rc<dyn NumberFormatter>,
            None => Rc::new(DecimalFormatter::default()),
        }
    }

    /// Sets IsEnabled.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Sets FontSize.
    pub fn font_size(mut self, value: f64) -> Self {
        self.font_size = Some(value);
        self
    }

    /// Sets CornerRadius.
    pub fn corner_radius(mut self, value: f64) -> Self {
        self.corner_radius = Some(value);
        self
    }

    /// Sets SelectionHighlightColor.
    pub fn selection_highlight_color(mut self, value: Color) -> Self {
        self.selection_highlight_color = Some(value);
        self
    }

    /// Sets TextAlignment.
    pub fn text_alignment(mut self, value: TextAlign) -> Self {
        self.text_alignment = value;
        self
    }

    /// Sets widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for NumberBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NumberBox")
            .field("value", &self.value)
            .field("minimum", &self.minimum)
            .field("maximum", &self.maximum)
            .field(
                "spin_button_placement_mode",
                &self.spin_button_placement_mode,
            )
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

/// `CommonStates`: the header's foreground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumberBoxCommonState {
    Normal,
    Disabled,
}

/// `SpinButtonStates`, which `UpdateSpinButtonPlacement` sets on the box and on `InputBox`
/// alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names, reason = "the template's state names")]
pub(super) enum SpinButtonsState {
    SpinButtonsCollapsed,
    /// `UpSpinButton`, `DownSpinButton` and `InputEater` show, and `InputBox` is at least
    /// `NumberBoxMinWidth` wide.
    SpinButtonsVisible,
    /// `InputBox` shows its `PopupIndicator`.
    SpinButtonsPopup,
}

impl SpinButtonsState {
    /// The state `UpdateSpinButtonPlacement` goes to for `mode`.
    fn of(mode: NumberBoxSpinButtonPlacementMode) -> SpinButtonsState {
        match mode {
            NumberBoxSpinButtonPlacementMode::Hidden => SpinButtonsState::SpinButtonsCollapsed,
            NumberBoxSpinButtonPlacementMode::Compact => SpinButtonsState::SpinButtonsPopup,
            NumberBoxSpinButtonPlacementMode::Inline => SpinButtonsState::SpinButtonsVisible,
        }
    }
}

/// `UpSpinButtonEnabledStates` and `DownSpinButtonEnabledStates`: whether the inline and popup
/// button of one direction can be pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpinButtonEnabledState {
    Enabled,
    Disabled,
}

/// `NumberBoxTextBoxStyle`: the TextBox template of NumberBox's `InputBox`, in the
/// `SpinButtonStates` state the box sends it to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NumberBoxTextBoxStyle {
    /// The state `UpdateSpinButtonPlacement` sets on `InputBox`.
    pub(super) spin_buttons: SpinButtonsState,
    /// `InputBox.MinWidth`: `TextControlThemeMinWidth`, or `NumberBoxMinWidth` while the spin
    /// buttons are inline.
    pub(super) min_width: f64,
}

/// `MinWidth` of the template's `DeleteButton`.
const DELETE_BUTTON_MIN_WIDTH: f64 = 40.0;

/// `SpinButtonsColumn.Width` in the `SpinButtonsVisible` state: the room the inline spin buttons
/// cover.
const SPIN_BUTTONS_COLUMN_WIDTH: f64 = 72.0;

/// `NumberBoxSpinButtonStyle`'s `MinWidth`.
const SPIN_BUTTON_MIN_WIDTH: f64 = 32.0;

/// `NumberBoxPopupSpinButtonStyle`'s `Width` and `Height`.
const POPUP_SPIN_BUTTON_SIZE: f64 = 36.0;

/// `NumberBoxPopupSpinButtonStyle`'s `FontSize`.
const POPUP_SPIN_BUTTON_FONT_SIZE: f64 = 16.0;

/// `PopupContentRoot`'s `Padding`.
const POPUP_CONTENT_PADDING: f64 = 6.0;

/// `PopupIndicator`'s `FontSize`.
const POPUP_INDICATOR_FONT_SIZE: f64 = 12.0;

impl NumberBoxTextBoxStyle {
    /// `SpinButtonsColumn`: 72 wide while the spin buttons are inline, else as wide as what it
    /// holds.
    pub(super) fn spin_buttons_column(self) -> ColumnDefinition {
        ColumnDefinition::new(match self.spin_buttons {
            SpinButtonsState::SpinButtonsVisible => GridLength::pixel(SPIN_BUTTONS_COLUMN_WIDTH),
            _ => GridLength::AUTO,
        })
    }

    /// `PopupIndicator`, which the `SpinButtonsPopup` state shows in `SpinButtonsColumn`.
    pub(super) fn popup_indicator(self, resources: &ThemeResources) -> Option<WidgetRef> {
        if self.spin_buttons != SpinButtonsState::SpinButtonsPopup {
            return None;
        }
        let foreground = resources.number_box_popup_indicator_foreground;
        Some(
            Margin::new(
                resources.number_box_popup_indicator_margin,
                Center::new().width_factor(1.0).height_factor(1.0).child(
                    FontIcon::symbol(FluentSymbol::ChevronUpDown)
                        .font_size(POPUP_INDICATOR_FONT_SIZE)
                        .foreground(foreground),
                ),
            )
            .into_widget(),
        )
    }

    /// `DeleteButton`'s width: its height, as `TextBox::OnDeleteButtonSizeChanged` keeps it,
    /// but not under its `MinWidth`.
    pub(super) fn delete_button_width(self, height: f64) -> f64 {
        height.max(DELETE_BUTTON_MIN_WIDTH)
    }
}

/// `Minimum` and `Maximum`, as `CoerceMinimum` and `CoerceMaximum` leave them.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    minimum: f64,
    maximum: f64,
}

impl Bounds {
    /// The bounds after `minimum` or `maximum` changed. A change of `Maximum` alone runs
    /// `CoerceMinimum`, lowering a `Minimum` above it to it; any other runs `CoerceMaximum`,
    /// raising a `Maximum` below `Minimum` to it.
    fn new(minimum: f64, maximum: f64, maximum_changed_alone: bool) -> Bounds {
        if maximum_changed_alone && minimum > maximum {
            Bounds {
                minimum: maximum,
                maximum,
            }
        } else if maximum < minimum {
            Bounds {
                minimum,
                maximum: minimum,
            }
        } else {
            Bounds { minimum, maximum }
        }
    }

    /// `IsInBounds`.
    fn contain(self, value: f64) -> bool {
        value >= self.minimum && value <= self.maximum
    }
}

/// `CoerceValue`: a value out of bounds goes to the bound it passed when invalid input is
/// overwritten; NaN stays.
fn coerce_value(value: f64, bounds: Bounds, mode: NumberBoxValidationMode) -> f64 {
    if value.is_nan()
        || bounds.contain(value)
        || mode != NumberBoxValidationMode::InvalidInputOverwritten
    {
        value
    } else if value > bounds.maximum {
        bounds.maximum
    } else {
        bounds.minimum
    }
}

/// `StepValue`'s arithmetic: `value` moved by `change`, and with wrapping a value past one bound
/// goes to the other.
fn stepped(value: f64, change: f64, bounds: Bounds, wrap: bool) -> f64 {
    let value = value + change;
    if !wrap {
        value
    } else if value > bounds.maximum {
        bounds.minimum
    } else if value < bounds.minimum {
        bounds.maximum
    } else {
        value
    }
}

/// `UpdateSpinButtonEnabled`: with a value, both directions can step when wrapping or when
/// invalid input is kept, and otherwise each until its bound; without one neither can.
fn spin_button_enabled_states(
    value: f64,
    bounds: Bounds,
    wrap: bool,
    mode: NumberBoxValidationMode,
) -> (SpinButtonEnabledState, SpinButtonEnabledState) {
    let state = |enabled: bool| {
        if enabled {
            SpinButtonEnabledState::Enabled
        } else {
            SpinButtonEnabledState::Disabled
        }
    };
    if value.is_nan() {
        return (state(false), state(false));
    }
    if wrap || mode != NumberBoxValidationMode::InvalidInputOverwritten {
        return (state(true), state(true));
    }
    (state(value < bounds.maximum), state(value > bounds.minimum))
}

/// Whether `new` is another value than `old`, NaN being one value: `OnValuePropertyChanged`'s
/// test before it raises `ValueChanged`.
fn differs(old: f64, new: f64) -> bool {
    new != old && !(new.is_nan() && old.is_nan())
}

/// What `UpdateTextToValue` shows for `value`: nothing for NaN, else the value rounded by the
/// display rounder and formatted.
fn displayed_text(value: f64, formatter: &dyn NumberFormatter) -> String {
    if value.is_nan() {
        String::new()
    } else {
        formatter.format_double(round_for_display(value))
    }
}

/// `trim` of NumberBox.cpp: without leading and trailing spaces, line breaks, tabs, form feeds
/// and vertical tabs.
fn trim(text: &str) -> &str {
    text.trim_matches([' ', '\n', '\r', '\t', '\x0c', '\x0b'])
}

impl StatefulWidget for NumberBox {
    type State = NumberBoxState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> NumberBoxState {
        NumberBoxState {
            state: StateData::new(),
            controller: None,
            focus: None,
            popup: None,
            value: f64::NAN,
            bounds: Bounds {
                minimum: self.minimum,
                maximum: self.maximum,
            },
            requested: None,
            focused: false,
        }
    }
}

/// The parts NumberBox.cpp keeps of its template, and its value.
pub struct NumberBoxState {
    state: StateData<NumberBox>,

    /// `InputBox`'s text.
    controller: Option<Handle<TextEditingController>>,

    /// `InputBox`'s focus, which takes the arrow, page, Enter and Escape keys before the text
    /// box does, as `PreviewKeyDown` and `KeyUp` do.
    focus: Option<Handle<FocusNode>>,

    /// Shows `UpDownPopup`.
    popup: Option<Handle<OverlayPortalController>>,

    /// `Value`: the application's value, as `CoerceValue` left it.
    value: f64,

    /// `Minimum` and `Maximum`, as their coercion left them.
    bounds: Bounds,

    /// The value the box last asked for, until the application next builds it.
    requested: Option<f64>,

    /// Whether `InputBox` has the focus, as `GotFocus` and `LostFocus` last said.
    focused: bool,
}

impl NumberBoxState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<TextEditingController> {
        app.get(self).controller.expect("an initialized NumberBox")
    }

    fn focus(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self).focus.expect("an initialized NumberBox")
    }

    fn popup(self: Handle<Self>, app: &App) -> Handle<OverlayPortalController> {
        app.get(self).popup.expect("an initialized NumberBox")
    }

    /// `Value()` as the box's own logic reads it: the value it last asked for, until the
    /// application answers, else the application's.
    fn current_value(self: Handle<Self>, app: &App) -> f64 {
        let state = app.get(self);
        state.requested.unwrap_or(state.value)
    }

    /// The `Value` setter, for a value the box sets, and `OnValuePropertyChanged`: setting NaN
    /// over NaN, or the value there already is, changes nothing; otherwise `CoerceValue` keeps
    /// it in range, `ValueChanged` asks the application for it when it is another value, and the
    /// text shows it.
    fn put_value(self: Handle<Self>, app: &mut App, value: f64) {
        let old = self.current_value(app);
        if !differs(old, value) {
            return;
        }
        let widget = self.widget(app);
        let (handler, mode) = (widget.value_changed.clone(), widget.validation_mode);
        let new = coerce_value(value, app.get(self).bounds, mode);
        if differs(old, new) {
            self.set_state(app, |state| state.requested = Some(new));
            handler(
                app,
                NumberBoxValueChangedEventArgs {
                    old_value: old,
                    new_value: new,
                },
            );
        }
        if app.contains(self) && self.mounted(app) {
            self.update_text_to_value(app, new);
        }
    }

    /// `UpdateTextToValue`: `InputBox` shows `value`. Replacing its text puts the caret at the
    /// start, as setting `TextBox.Text` does.
    fn update_text_to_value(self: Handle<Self>, app: &mut App, value: f64) {
        let text = displayed_text(value, &*self.widget(app).formatter());
        self.set_text(app, text);
    }

    /// Puts `text` in `InputBox` when it holds other text, with the caret at the start.
    fn set_text(self: Handle<Self>, app: &mut App, text: String) {
        let controller = self.controller(app);
        if controller.text_value(app) != text {
            controller.set_value(
                app,
                TextEditingValue::new()
                    .text(text)
                    .selection(TextSelection::collapsed(0, TextAffinity::Downstream)),
            );
        }
    }

    /// `ValidateInput`: commits `InputBox`'s text. Empty text clears the value. Text that reads
    /// as a number, or computes to one with `accepts_expression`, becomes the value, and is
    /// formatted even when the value stays. Other text is replaced by the value unless
    /// validation is disabled.
    fn validate_input(self: Handle<Self>, app: &mut App) {
        let text = trim(self.controller(app).text_value(app)).to_owned();
        if text.is_empty() {
            self.put_value(app, f64::NAN);
            return;
        }
        let widget = self.widget(app);
        let (formatter, mode) = (widget.formatter(), widget.validation_mode);
        let value = if widget.accepts_expression {
            parser::compute(&text, &*formatter)
        } else {
            formatter.parse_double(&text)
        };
        let current = self.current_value(app);
        match value {
            None if mode == NumberBoxValidationMode::InvalidInputOverwritten => {
                self.update_text_to_value(app, current)
            }
            None => {}
            // `3` after `1 + 2` with a value of 3 still becomes `3`.
            Some(value) if value == current => self.update_text_to_value(app, current),
            Some(value) => self.put_value(app, value),
        }
    }

    /// `StepValue`: commits the text first so that it is not lost, then moves the value by
    /// `change` and puts the caret at the end. Nothing moves while there is no value.
    fn step_value(self: Handle<Self>, app: &mut App, change: f64) {
        self.validate_input(app);
        let value = self.current_value(app);
        if value.is_nan() {
            return;
        }
        let wrap = self.widget(app).is_wrap_enabled;
        let bounds = app.get(self).bounds;
        self.put_value(app, stepped(value, change, bounds, wrap));
        self.move_caret_to_text_end(app);
    }

    /// `MoveCaretToTextEnd`.
    fn move_caret_to_text_end(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        let end = controller.text_value(app).encode_utf16().count() as i32;
        controller.set_selection(app, TextSelection::collapsed(end, TextAffinity::Downstream));
    }

    /// `TextBox::SelectAll`.
    fn select_all(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        let end = controller.text_value(app).encode_utf16().count() as i32;
        controller.set_selection(app, TextSelection::new(0, end));
    }

    /// `OnNumberBoxKeyDown`, which WinUI hooks to `InputBox`'s `PreviewKeyDown` because the text
    /// box takes some of these keys: the arrows step by `small_change` and the page keys by
    /// `large_change`, on each press and repeat. And `OnNumberBoxKeyUp`: Enter commits the text
    /// and Escape puts the value back in it.
    fn key_event(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        let key = event.logical_key();
        match event {
            KeyEvent::Down(_) | KeyEvent::Repeat(_) => {
                let widget = self.widget(app);
                let change = match key {
                    LogicalKeyboardKey::ARROW_UP => widget.small_change,
                    LogicalKeyboardKey::ARROW_DOWN => -widget.small_change,
                    LogicalKeyboardKey::PAGE_UP => widget.large_change,
                    LogicalKeyboardKey::PAGE_DOWN => -widget.large_change,
                    _ => return KeyEventResult::Ignored,
                };
                self.step_value(app, change);
                KeyEventResult::Handled
            }
            KeyEvent::Up(_) => match key {
                LogicalKeyboardKey::ENTER
                | LogicalKeyboardKey::NUMPAD_ENTER
                | LogicalKeyboardKey::GAME_BUTTON_A => {
                    self.validate_input(app);
                    KeyEventResult::Handled
                }
                LogicalKeyboardKey::ESCAPE | LogicalKeyboardKey::GAME_BUTTON_B => {
                    let value = self.current_value(app);
                    self.update_text_to_value(app, value);
                    KeyEventResult::Handled
                }
                _ => KeyEventResult::Ignored,
            },
        }
    }

    /// `OnNumberBoxScroll`: while `InputBox` has the focus, the box takes each wheel event, so
    /// that the page does not scroll, and steps by `small_change` in the wheel's direction.
    fn pointer_signal(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        if !matches!(event, PointerEvent::Scroll(_)) || !self.focus(app).has_focus(app) {
            return;
        }
        let resolver = GestureBinding::instance(app).pointer_signal_resolver(app);
        resolver.register(
            app,
            event,
            Rc::new(move |app, event| self.wheel(app, event)),
        );
    }

    /// One wheel event the box took. `MouseWheelDelta` is positive for a wheel turned away from
    /// the user, where Inset's vertical scroll delta is negative, and for a wheel tilted right.
    fn wheel(self: Handle<Self>, app: &mut App, event: &PointerEvent) {
        let PointerEvent::Scroll(scroll) = event else {
            return;
        };
        if !app.contains(self) || !self.mounted(app) {
            return;
        }
        let delta = if scroll.scroll_delta.dy() != 0.0 {
            -scroll.scroll_delta.dy()
        } else {
            scroll.scroll_delta.dx()
        };
        let small_change = self.widget(app).small_change;
        if delta > 0.0 {
            self.step_value(app, small_change);
        } else if delta < 0.0 {
            self.step_value(app, -small_change);
        }
    }

    /// `OnNumberBoxGotFocus` and `OnNumberBoxLostFocus`: taking the focus selects the text and
    /// opens the popup of compact spin buttons; losing it commits the text and closes the
    /// popup.
    fn focus_changed(self: Handle<Self>, app: &mut App) {
        let focused = self.focus(app).has_focus(app);
        if focused == app.get(self).focused {
            return;
        }
        app.get_mut(self).focused = focused;
        let popup = self.popup(app);
        if focused {
            self.select_all(app);
            if self.widget(app).spin_button_placement_mode
                == NumberBoxSpinButtonPlacementMode::Compact
            {
                popup.show(app);
            }
        } else {
            self.validate_input(app);
            if app.contains(self) && self.mounted(app) {
                popup.hide(app);
            }
        }
    }

    /// The application has answered the value the box asked for, by building the box again: if
    /// `InputBox` still shows the asked-for value, it shows the application's.
    fn settle_request(self: Handle<Self>, app: &mut App) {
        let Some(requested) = app.get_mut(self).requested.take() else {
            return;
        };
        let formatter = self.widget(app).formatter();
        if self.controller(app).text_value(app) == displayed_text(requested, &*formatter) {
            let value = app.get(self).value;
            self.update_text_to_value(app, value);
        }
    }

    /// The property changes of `OnValuePropertyChanged`, `OnMinimumPropertyChanged`,
    /// `OnMaximumPropertyChanged`, `OnNumberFormatterPropertyChanged`, `OnTextPropertyChanged`
    /// and `OnValidationModePropertyChanged`, from `old` (none on the first build) to the
    /// widget, and `OnApplyTemplate`'s first text. What would raise `ValueChanged` during the
    /// build runs after the frame.
    fn apply_widget(self: Handle<Self>, app: &mut App, old: Option<&NumberBox>) {
        let widget = self.widget(app).clone();
        let bounds_changed =
            old.is_none_or(|old| old.minimum != widget.minimum || old.maximum != widget.maximum);
        let bounds = if bounds_changed {
            let maximum_changed_alone = old.is_some_and(|old| old.minimum == widget.minimum);
            Bounds::new(widget.minimum, widget.maximum, maximum_changed_alone)
        } else {
            app.get(self).bounds
        };
        let value_set = old.is_none_or(|old| differs(old.value, widget.value));
        let previous = app.get(self).value;
        let value = if value_set || bounds_changed {
            coerce_value(widget.value, bounds, widget.validation_mode)
        } else {
            previous
        };
        let state = app.get_mut(self);
        state.bounds = bounds;
        state.value = value;
        self.settle_request(app);
        let formatter_changed =
            old.is_some_and(
                |old| match (&old.number_formatter, &widget.number_formatter) {
                    (None, None) => false,
                    (Some(old), Some(new)) => !old.same_as(&**new),
                    _ => true,
                },
            );
        if value_set || differs(previous, value) || formatter_changed {
            self.update_text_to_value(app, value);
        }
        if differs(previous, value) && differs(widget.value, value) {
            self.report_coercion_after_frame(app, previous, value);
        }
        let text_set = match old {
            // `OnApplyTemplate` reads a `Text` set without a `Value`.
            None => widget.value.is_nan() && widget.text.is_some(),
            Some(old) => widget.text.is_some() && old.text != widget.text,
        };
        if let Some(text) = widget.text.clone().filter(|_| text_set) {
            // `UpdateValueToText`.
            self.set_text(app, text);
            self.validate_input_after_frame(app);
        } else if old.is_some_and(|old| old.validation_mode != widget.validation_mode) {
            self.validate_input_after_frame(app);
        }
    }

    /// Raises the `ValueChanged` that coercing the application's value from `old` to `new`
    /// raises in WinUI, once the frame is over, if the box still has that value then.
    fn report_coercion_after_frame(self: Handle<Self>, app: &mut App, old: f64, new: f64) {
        let view = View::maybe_id_of(app, self.context(app));
        after_next_frame(
            app,
            view,
            FrameCallback::new(move |app, _| {
                if !app.contains(self) || !self.mounted(app) || differs(app.get(self).value, new) {
                    return;
                }
                let handler = self.widget(app).value_changed.clone();
                handler(
                    app,
                    NumberBoxValueChangedEventArgs {
                        old_value: old,
                        new_value: new,
                    },
                );
            }),
        );
    }

    /// Runs `ValidateInput` once the frame is over, where it may ask the application for a
    /// value.
    fn validate_input_after_frame(self: Handle<Self>, app: &mut App) {
        let view = View::maybe_id_of(app, self.context(app));
        after_next_frame(
            app,
            view,
            FrameCallback::new(move |app, _| {
                if app.contains(self) && self.mounted(app) {
                    self.validate_input(app);
                }
            }),
        );
    }
}

impl State for NumberBoxState {
    type Widget = NumberBox;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = TextEditingController::new(app);
        let focus = FocusNode::new(app);
        focus.as_node().set_on_key_event(
            app,
            Some(Rc::new(move |app, _, event| self.key_event(app, event))),
        );
        focus.add_listener(app, Listener::handle_method(self, Self::focus_changed));
        let popup = OverlayPortalController::new(app, Some("NumberBox UpDownPopup".into()));
        let state = app.get_mut(self);
        state.controller = Some(controller);
        state.focus = Some(focus);
        state.popup = Some(popup);
        self.apply_widget(app, None);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &NumberBox) {
        self.apply_widget(app, Some(old));
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        let (controller, focus, popup) = (
            state.controller.take().expect("an initialized NumberBox"),
            state.focus.take().expect("an initialized NumberBox"),
            state.popup.take().expect("an initialized NumberBox"),
        );
        focus.remove_listener(app, &Listener::handle_method(self, Self::focus_changed));
        focus.dispose(app);
        app.destroy(focus);
        app.get_mut(controller).dispose();
        app.destroy(controller);
        app.destroy(popup);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // A value asked for since the last build that the application did not pass is undone.
        self.settle_request(app);
        let widget = self.widget(app).clone();
        let resources = ThemeResources::of(app, context);
        let r = &resources;
        let common = if widget.is_enabled {
            NumberBoxCommonState::Normal
        } else {
            NumberBoxCommonState::Disabled
        };
        let spin_buttons = SpinButtonsState::of(widget.spin_button_placement_mode);
        let mut cells = Vec::new();
        if let Some(header) = widget.header.clone() {
            // `HeaderContentPresenter`.
            let foreground = match common {
                NumberBoxCommonState::Normal => r.text_control_header_foreground,
                NumberBoxCommonState::Disabled => r.text_control_header_foreground_disabled,
            };
            cells.push(
                GridCell::new(Margin::new(
                    r.text_box_top_header_margin,
                    DefaultTextStyle::new(
                        control_text_style(
                            &resources,
                            widget
                                .font_size
                                .unwrap_or(r.control_content_theme_font_size),
                            FontWeight::NORMAL,
                            foreground,
                        ),
                        header,
                    ),
                ))
                .column_span(3)
                .into_widget(),
            );
        }
        // `InputBox`.
        cells.push(
            GridCell::new(self.input_box(&resources, app, spin_buttons))
                .row(1)
                .column_span(3)
                .into_widget(),
        );
        // `UpDownPopup`, at the start of its cell.
        cells.push(
            GridCell::new(self.up_down_popup(app))
                .row(1)
                .column(1)
                .into_widget(),
        );
        if spin_buttons == SpinButtonsState::SpinButtonsVisible {
            // `InputEater`: a button with an empty template, which takes presses between and
            // around the spin buttons so that they never reach `InputBox` beneath.
            cells.push(
                GridCell::new(Margin::new(
                    [4.0, 0.0, 0.0, 0.0],
                    // It measures as nothing in the Auto columns and takes its cell.
                    PointerListener::new()
                        .behavior(inset_rendering::HitTestBehavior::Opaque)
                        .child(SizedBox::shrink()),
                ))
                .row(1)
                .column(1)
                .column_span(2)
                .into_widget(),
            );
            let (up, down) = self.spin_button_enabled_states(app);
            // `UpSpinButton` and `DownSpinButton`.
            cells.push(
                GridCell::new(Margin::new(
                    [4.0; 4],
                    self.inline_spin_button(&resources, app, FluentSymbol::ChevronUp, 1.0, up),
                ))
                .row(1)
                .column(1)
                .into_widget(),
            );
            cells.push(
                GridCell::new(Margin::new(
                    [0.0, 4.0, 4.0, 4.0],
                    self.inline_spin_button(&resources, app, FluentSymbol::ChevronDown, -1.0, down),
                ))
                .row(1)
                .column(2)
                .into_widget(),
            );
        }
        if let Some(description) = widget.description.clone() {
            // `DescriptionPresenter`.
            cells.push(
                GridCell::new(DefaultTextStyle::new(
                    control_text_style(
                        &resources,
                        widget
                            .font_size
                            .unwrap_or(r.control_content_theme_font_size),
                        FontWeight::NORMAL,
                        r.system_control_description_text_foreground_brush,
                    ),
                    description,
                ))
                .row(2)
                .column_span(3)
                .into_widget(),
            );
        }
        let template = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::STAR),
                RowDefinition::new(GridLength::AUTO),
            ])
            .column_definitions([
                ColumnDefinition::new(GridLength::STAR),
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::AUTO),
            ])
            .children(cells);
        // The box is one text field to the platform: pressing a spin button, the header or the
        // popup leaves `InputBox` focused.
        TextFieldTapRegion::new(
            PointerListener::new()
                .on_pointer_signal(Rc::new(move |app, event| self.pointer_signal(app, &event)))
                .child(template),
        )
        .into_widget()
    }
}

impl NumberBoxState {
    /// `InputBox`: a TextBox in `NumberBoxTextBoxStyle`, with the box's text properties.
    fn input_box(
        self: Handle<Self>,
        r: &ThemeResources,
        app: &App,
        spin_buttons: SpinButtonsState,
    ) -> WidgetRef {
        let widget = self.widget(app);
        let mut config = TextBox::new(self.controller(app).as_controller())
            .focus_node(self.focus(app))
            .placeholder_text(widget.placeholder_text.clone())
            .is_enabled(widget.is_enabled)
            .font_size(
                widget
                    .font_size
                    .unwrap_or(r.control_content_theme_font_size),
            )
            .corner_radius(widget.corner_radius.unwrap_or(r.control_corner_radius[0]))
            .text_alignment(widget.text_alignment);
        if let Some(color) = widget.selection_highlight_color {
            config = config.selection_highlight_color(color);
        }
        let min_width = if spin_buttons == SpinButtonsState::SpinButtonsVisible {
            r.number_box_min_width
        } else {
            r.text_control_theme_min_width
        };
        TextControl::new(config, None)
            .number_box_style(NumberBoxTextBoxStyle {
                spin_buttons,
                min_width,
            })
            .into_widget()
    }

    /// The up and down states of `UpdateSpinButtonEnabled`.
    fn spin_button_enabled_states(
        self: Handle<Self>,
        app: &App,
    ) -> (SpinButtonEnabledState, SpinButtonEnabledState) {
        let widget = self.widget(app);
        spin_button_enabled_states(
            self.current_value(app),
            app.get(self).bounds,
            widget.is_wrap_enabled,
            widget.validation_mode,
        )
    }

    /// A spin button, stepping by `direction` times `small_change` on each `Click`
    /// (`OnSpinUpClick` and `OnSpinDownClick`).
    fn spin_button(
        self: Handle<Self>,
        app: &App,
        glyph: FluentSymbol,
        direction: f64,
        state: SpinButtonEnabledState,
        setters: RepeatButtonSetters,
    ) -> RepeatButton {
        let enabled = self.widget(app).is_enabled && state == SpinButtonEnabledState::Enabled;
        let click = Listener::new(move |app| {
            let change = direction * self.widget(app).small_change;
            self.step_value(app, change);
        });
        RepeatButton::new(FontIcon::symbol(glyph).font_size(setters.font_size), click)
            .is_enabled(enabled)
            .is_tab_stop(false)
            .template(move |app, context, states, content| {
                let resources = ThemeResources::of(app, context);
                repeat_button_presenter(
                    &resources,
                    spin_button_brushes(&resources, states.common),
                    setters,
                    content,
                )
            })
    }

    /// `UpSpinButton` or `DownSpinButton`, in `NumberBoxSpinButtonStyle`: at least 32 wide, as
    /// tall as the box less their margins, with the box's font size and corner radius.
    fn inline_spin_button(
        self: Handle<Self>,
        r: &ThemeResources,
        app: &App,
        glyph: FluentSymbol,
        direction: f64,
        state: SpinButtonEnabledState,
    ) -> WidgetRef {
        let widget = self.widget(app);
        let setters = RepeatButtonSetters {
            border_thickness: r.number_box_spin_button_border_thickness,
            corner_radius: widget.corner_radius.unwrap_or(r.control_corner_radius[0]),
            padding: [0.0; 4],
            font_size: widget
                .font_size
                .unwrap_or(r.control_content_theme_font_size),
        };
        ConstrainedBox::new(BoxConstraints::new().min_width(SPIN_BUTTON_MIN_WIDTH))
            .child(self.spin_button(app, glyph, direction, state, setters))
            .into_widget()
    }

    /// `PopupUpSpinButton` or `PopupDownSpinButton`, in `NumberBoxPopupSpinButtonStyle`: 36 by
    /// 36 at the left of its row and centered in it, overflowing evenly a row that is shorter,
    /// as XAML arranges an element of fixed size.
    fn popup_spin_button(
        self: Handle<Self>,
        r: &ThemeResources,
        app: &App,
        glyph: FluentSymbol,
        direction: f64,
        state: SpinButtonEnabledState,
    ) -> WidgetRef {
        let setters = RepeatButtonSetters {
            border_thickness: r.number_box_popup_spin_button_border_thickness,
            corner_radius: r.control_corner_radius[0],
            padding: [0.0; 4],
            font_size: POPUP_SPIN_BUTTON_FONT_SIZE,
        };
        SizedBox::new()
            .width(POPUP_SPIN_BUTTON_SIZE)
            .height(POPUP_SPIN_BUTTON_SIZE)
            .child(
                OverflowBox::new()
                    .alignment(AlignmentGeometry::CENTER_LEFT)
                    .min_width(POPUP_SPIN_BUTTON_SIZE)
                    .max_width(POPUP_SPIN_BUTTON_SIZE)
                    .min_height(POPUP_SPIN_BUTTON_SIZE)
                    .max_height(POPUP_SPIN_BUTTON_SIZE)
                    .child(self.spin_button(app, glyph, direction, state, setters)),
            )
            .into_widget()
    }

    /// `UpDownPopup`: no light dismiss, opened and closed by the focus; its content is placed
    /// from where the popup is in the template.
    fn up_down_popup(self: Handle<Self>, app: &App) -> WidgetRef {
        OverlayPortal::overlay_child_layout_builder(self.popup(app), move |app, context, info| {
            let r = ThemeResources::of(app, context);
            let slot = transform_rect(
                &info.child_paint_transform(),
                Offset::ZERO & info.child_size(),
            );
            CustomSingleChildLayout::new(Rc::new(PopupLayout {
                slot,
                offset: Offset::new(
                    r.number_box_popup_horizonal_offset,
                    r.number_box_popup_vertical_offset,
                ),
            }))
            .child(self.popup_content_root(app, context))
            .into_widget()
        })
        .overlay_location(OverlayChildLocation::RootOverlay)
        .child(SizedBox::shrink())
        .into_widget()
    }

    /// `PopupContentRoot`: the popup spin buttons on acrylic, raised by
    /// `NumberBoxPopupShadowDepth`.
    fn popup_content_root(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let r = &resources;
        let (up, down) = self.spin_button_enabled_states(app);
        let root = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::STAR),
                RowDefinition::new(GridLength::STAR),
            ])
            .padding([POPUP_CONTENT_PADDING; 4])
            .background(Brush::Acrylic(r.number_box_popup_background))
            .border_brush(Brush::Solid(r.number_box_popup_border_brush))
            .border_thickness_ltrb(r.number_box_popup_border_thickness)
            .corner_radius_corners(r.overlay_corner_radius)
            .children([
                GridCell::new(Margin::new(
                    [0.0, 0.0, 0.0, 4.0],
                    self.popup_spin_button(&resources, app, FluentSymbol::ChevronUp, 1.0, up),
                ))
                .into_widget(),
                GridCell::new(self.popup_spin_button(
                    &resources,
                    app,
                    FluentSymbol::ChevronDown,
                    -1.0,
                    down,
                ))
                .row(1)
                .into_widget(),
            ]);
        TextFieldTapRegion::new(
            CustomPaint::new()
                .painter(PopupShadow::for_number_box_popup(
                    &resources,
                    resources.theme,
                ))
                .child(root),
        )
        .into_widget()
    }
}

/// The brushes `DefaultRepeatButtonStyle`'s `CommonStates` give a spin button, where the
/// template's `Grid.Resources` set the `RepeatButton` brushes of the normal, pointer-over and
/// pressed states, and the disabled border, to their `TextControlButton` counterparts.
fn spin_button_brushes(resources: &ThemeResources, state: CommonState) -> RepeatButtonBrushes {
    let r = &resources;
    let (background, border, foreground) = match state {
        CommonState::Normal => (
            r.text_control_button_background,
            r.text_control_button_border_brush,
            r.text_control_button_foreground,
        ),
        CommonState::PointerOver => (
            r.text_control_button_background_pointer_over,
            r.text_control_button_border_brush_pointer_over,
            r.text_control_button_foreground_pointer_over,
        ),
        CommonState::Pressed => (
            r.text_control_button_background_pressed,
            r.text_control_button_border_brush_pressed,
            r.text_control_button_foreground_pressed,
        ),
        CommonState::Disabled => {
            let repeat_button = &resources;
            (
                repeat_button.repeat_button_background_disabled,
                r.text_control_button_border_brush,
                repeat_button.repeat_button_foreground_disabled,
            )
        }
    };
    RepeatButtonBrushes {
        background,
        border: Brush::Solid(border),
        foreground,
    }
}

/// Places `PopupContentRoot` where `UpDownPopup` opens it: at the top left of the popup's slot
/// (row 1, column 1 of the template), moved by `NumberBoxPopupHorizonalOffset` and
/// `NumberBoxPopupVerticalOffset`.
#[derive(Debug, PartialEq)]
struct PopupLayout {
    /// The popup's slot in the overlay.
    slot: Rect,
    /// `NumberBoxPopupHorizonalOffset` and `NumberBoxPopupVerticalOffset`.
    offset: Offset,
}

impl SingleChildLayoutDelegate for PopupLayout {
    /// A popup measures its child against an unbounded size.
    fn get_constraints_for_child(&self, _constraints: BoxConstraints) -> BoxConstraints {
        BoxConstraints::new()
    }

    fn get_position_for_child(&self, _size: Size, _child_size: Size) -> Offset {
        Offset::new(
            self.slot.left + self.offset.dx(),
            self.slot.top + self.offset.dy(),
        )
    }

    fn should_relayout(&self, old_delegate: &dyn SingleChildLayoutDelegate) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OVERWRITTEN: NumberBoxValidationMode = NumberBoxValidationMode::InvalidInputOverwritten;

    fn bounds(minimum: f64, maximum: f64) -> Bounds {
        Bounds { minimum, maximum }
    }

    #[test]
    fn crossed_bounds_keep_the_one_that_changed() {
        assert_eq!(Bounds::new(10.0, 5.0, false), bounds(10.0, 10.0));
        assert_eq!(Bounds::new(10.0, 5.0, true), bounds(5.0, 5.0));
        assert_eq!(Bounds::new(0.0, 5.0, true), bounds(0.0, 5.0));
    }

    #[test]
    fn coercion_moves_a_value_to_the_bound_it_passed_unless_validation_is_disabled() {
        let range = bounds(0.0, 100.0);
        assert_eq!(coerce_value(150.0, range, OVERWRITTEN), 100.0);
        assert_eq!(coerce_value(-5.0, range, OVERWRITTEN), 0.0);
        assert_eq!(coerce_value(50.0, range, OVERWRITTEN), 50.0);
        assert!(coerce_value(f64::NAN, range, OVERWRITTEN).is_nan());
        assert_eq!(
            coerce_value(150.0, range, NumberBoxValidationMode::Disabled),
            150.0
        );
    }

    #[test]
    fn steps_wrap_past_a_bound_to_the_other() {
        let range = bounds(0.0, 100.0);
        assert_eq!(stepped(95.0, 10.0, range, false), 105.0);
        assert_eq!(stepped(95.0, 10.0, range, true), 0.0);
        assert_eq!(stepped(5.0, -10.0, range, true), 100.0);
        assert_eq!(stepped(50.0, 10.0, range, true), 60.0);
    }

    #[test]
    fn spin_buttons_stop_at_their_bound_unless_wrapping_or_keeping_invalid_input() {
        use SpinButtonEnabledState::{Disabled, Enabled};
        let range = bounds(0.0, 100.0);
        assert_eq!(
            spin_button_enabled_states(100.0, range, false, OVERWRITTEN),
            (Disabled, Enabled)
        );
        assert_eq!(
            spin_button_enabled_states(0.0, range, false, OVERWRITTEN),
            (Enabled, Disabled)
        );
        assert_eq!(
            spin_button_enabled_states(100.0, range, true, OVERWRITTEN),
            (Enabled, Enabled)
        );
        assert_eq!(
            spin_button_enabled_states(100.0, range, false, NumberBoxValidationMode::Disabled),
            (Enabled, Enabled)
        );
        assert_eq!(
            spin_button_enabled_states(f64::NAN, range, true, OVERWRITTEN),
            (Disabled, Disabled)
        );
    }

    #[test]
    fn nan_to_nan_is_no_change() {
        assert!(!differs(f64::NAN, f64::NAN));
        assert!(differs(f64::NAN, 1.0));
        assert!(differs(1.0, f64::NAN));
        assert!(!differs(1.0, 1.0));
    }

    #[test]
    fn the_shown_text_is_the_rounded_formatted_value() {
        let formatter = DecimalFormatter::default();
        assert_eq!(displayed_text(f64::NAN, &formatter), "");
        assert_eq!(displayed_text(0.1 + 0.2, &formatter), "0.3");
        assert_eq!(trim(" \t12\n"), "12");
    }
}
