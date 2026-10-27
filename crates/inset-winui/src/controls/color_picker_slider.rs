//! ColorPickerSlider.cpp and `ColorPickerSliderStyle` of ColorPicker.xaml: a Slider for one
//! channel of a colour, whose arrow keys step the channel as ColorPicker's spectrum does and
//! whose thumb tooltip names the value and the colour.
//!
//! Template, top to bottom: a `Grid` (rows Auto, *) with `HeaderContentPresenter` in row 0 and
//! `SliderContainer` (transparent, the focus visual's target) in row 1, holding
//! `HorizontalTemplate` (at least 32 tall; columns Auto, Auto, *; rows
//! `SliderPreContentMargin`, Auto, `SliderPostContentMargin`) or `VerticalTemplate` (the same
//! turned). Each has its track and decrease rectangles, transparent at opacity 0, so that only
//! the thumb shows over whatever is behind the slider; and its thumb (`HorizontalThumb`,
//! `VerticalThumb`), 18 square, a `Border` of `SliderThumbBorderBrush` and
//! `SliderOuterThumbBackground` around a 10 pixel `SliderInnerThumb` ellipse filled with the
//! thumb's `Background`, which the `CommonStates` storyboards set on `HorizontalThumb` only.
//! Each thumb owns a tooltip: `ToolTip` the horizontal one, whose content and opening this
//! control manages, `VerticalToolTip` the vertical one, which it leaves alone.
//!
//! The application owns the value, as for `Slider`.

use crate::{
    Brush, ColorPickerHsvChannel, ColorPickerState, ColumnDefinition, CommonState, ControlBorder,
    FocusVisual, Grid, GridCell, GridLength, Hsv, IncrementAmount, IncrementDirection, Orientation,
    RangeBase, RowDefinition, SLIDER_DEFAULT_MAXIMUM, SLIDER_DEFAULT_STEP_FREQUENCY,
    SLIDER_FOCUS_VISUAL_MARGIN, Slider, SliderTemplate, SliderTemplateParts, SliderTemplateStates,
    ThemeResources, ToolTip, ToolTipService, ValueChangedHandler, color_display_name,
    color_from_rgba, control_text_style, decrease_definitions, fill_bounded, hsv_to_rgb,
    increment_alpha_channel, increment_color_channel,
};
use inset_embedder::{Canvas, Color, Rect, Size};
use inset_foundation::{App, Handle};
use inset_painting::{EdgeInsetsGeometry, draw_oval};
use inset_rendering::{BoxConstraints, CustomPainter};
use inset_services::{HardwareKeyboard, LogicalKeyboardKey};
use inset_widgets::*;
use std::{fmt, rc::Rc};

/// `HorizontalTemplate`'s `MinHeight` and `VerticalTemplate`'s `MinWidth`.
const TEMPLATE_MIN_SIZE: f64 = 32.0;
/// `BorderThickness` of the template's `SliderThumbStyle`.
const THUMB_BORDER_THICKNESS: f64 = 1.0;
/// `VerticalOffset` of the thumbs' tooltips.
const THUMB_TOOL_TIP_OFFSET: f64 = 20.0;

/// XAML `ColorPickerSlider` (a `Slider`) in `ColorPickerSliderStyle`.
#[derive(Clone)]
pub struct ColorPickerSlider {
    /// `Value`.
    pub value: f64,
    /// `ValueChanged`, with the value the slider wants; the caller owns `value`.
    pub value_changed: ValueChangedHandler,
    /// `ColorChannel`: what the value is, for the arrow keys and the tooltip; `Value` by default.
    pub color_channel: ColorPickerHsvChannel,
    /// `Minimum`.
    pub minimum: f64,
    /// `Maximum`.
    pub maximum: f64,
    /// `StepFrequency`.
    pub step_frequency: f64,
    /// `Orientation`.
    pub orientation: Orientation,
    /// `IsDirectionReversed`.
    pub is_direction_reversed: bool,
    /// `Header`.
    pub header: Option<WidgetRef>,
    /// `IsThumbToolTipEnabled`: whether the slider itself shows the thumb tooltips, and whether
    /// they are enabled before the slider first takes the focus or changes value.
    pub is_thumb_tool_tip_enabled: bool,
    /// `CornerRadius`: of `SliderContainer`, which the focus visual follows; `ControlCornerRadius`
    /// by default; the theme's `ControlCornerRadius` when `None`.
    pub corner_radius: Option<f64>,
    /// `IsEnabled`.
    pub is_enabled: bool,
    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl ColorPickerSlider {
    /// Creates a slider at `value`, which asks for others through `value_changed`.
    pub fn new(value: f64, value_changed: impl Fn(&mut App, f64) + 'static) -> ColorPickerSlider {
        // `Slider`'s defaults: the control's style is Slider's.
        ColorPickerSlider {
            value,
            value_changed: Rc::new(value_changed),
            color_channel: ColorPickerHsvChannel::Value,
            minimum: 0.0,
            maximum: SLIDER_DEFAULT_MAXIMUM,
            step_frequency: SLIDER_DEFAULT_STEP_FREQUENCY,
            orientation: Orientation::Horizontal,
            is_direction_reversed: false,
            header: None,
            is_thumb_tool_tip_enabled: true,
            corner_radius: None,
            is_enabled: true,
            key: None,
        }
    }

    /// Sets ColorChannel.
    pub fn color_channel(mut self, value: ColorPickerHsvChannel) -> Self {
        self.color_channel = value;
        self
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

    /// Sets StepFrequency.
    pub fn step_frequency(mut self, value: f64) -> Self {
        self.step_frequency = value;
        self
    }

    /// Sets Orientation.
    pub fn orientation(mut self, value: Orientation) -> Self {
        self.orientation = value;
        self
    }

    /// Sets IsDirectionReversed.
    pub fn is_direction_reversed(mut self, value: bool) -> Self {
        self.is_direction_reversed = value;
        self
    }

    /// Sets Header.
    pub fn header<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.header = Some(value.into_widget());
        self
    }

    /// Sets IsThumbToolTipEnabled.
    pub fn is_thumb_tool_tip_enabled(mut self, value: bool) -> Self {
        self.is_thumb_tool_tip_enabled = value;
        self
    }

    /// Sets CornerRadius.
    pub fn corner_radius(mut self, value: f64) -> Self {
        self.corner_radius = Some(value);
        self
    }

    /// Sets IsEnabled.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Sets widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for ColorPickerSlider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColorPickerSlider")
            .field("value", &self.value)
            .field("color_channel", &self.color_channel)
            .field("minimum", &self.minimum)
            .field("maximum", &self.maximum)
            .field("orientation", &self.orientation)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ColorPickerSlider {
    type State = ColorPickerSliderState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ColorPickerSliderState {
        ColorPickerSliderState {
            state: StateData::new(),
            tool_tip_text: String::new(),
            tool_tip_enabled: self.is_thumb_tool_tip_enabled,
            focused: false,
        }
    }
}

/// The slider's own state: its `ToolTip`.
pub struct ColorPickerSliderState {
    state: StateData<ColorPickerSlider>,
    /// `ToolTip.Content`.
    tool_tip_text: String,
    /// `ToolTip.IsEnabled`.
    tool_tip_enabled: bool,
    /// The slider has the focus, so that `OnGotFocus` opened `ToolTip`.
    focused: bool,
}

impl ColorPickerSliderState {
    /// `GetParentColorPicker`: the nearest enclosing ColorPicker.
    fn parent_color_picker(self: Handle<Self>, app: &App) -> Option<Handle<ColorPickerState>> {
        self.context(app)
            .find_ancestor_state_of_type::<ColorPickerState>(app)
    }

    /// `GetToolTipString`: the value, and with an enclosing ColorPicker the name of its colour
    /// with this channel at the value.
    fn tool_tip_string(self: Handle<Self>, app: &App) -> String {
        let widget = self.widget(app);
        let value = widget.value.round() as u32;
        let channel = widget.color_channel;
        if channel == ColorPickerHsvChannel::Alpha {
            return format!("{value}% opacity");
        }
        let label = match channel {
            ColorPickerHsvChannel::Hue => "Hue",
            ColorPickerHsvChannel::Saturation => "Saturation",
            _ => "Value",
        };
        let Some(picker) = self.parent_color_picker(app) else {
            return format!("{label} {value}");
        };
        let mut hsv = picker.current_hsv(app);
        match channel {
            ColorPickerHsvChannel::Hue => hsv.h = widget.value,
            ColorPickerHsvChannel::Saturation => hsv.s = widget.value / 100.0,
            _ => hsv.v = widget.value / 100.0,
        }
        let name = color_display_name(color_from_rgba(hsv_to_rgb(hsv), 1.0));
        format!("{label} {value} ({name})")
    }

    /// `OnKeyDown`: the arrows step the channel as the spectrum's keys do, by one or with Ctrl
    /// to the next named colour (alpha by 1 or to the next 10 percent), within the enclosing
    /// ColorPicker's range and without wrapping; Left and Down lower it. Without an enclosing
    /// ColorPicker the arrows do nothing. Other keys are the slider's.
    fn key_down(
        self: Handle<Self>,
        app: &mut App,
        key: LogicalKeyboardKey,
    ) -> Option<KeyEventResult> {
        let direction = match key {
            LogicalKeyboardKey::ARROW_LEFT | LogicalKeyboardKey::ARROW_DOWN => {
                IncrementDirection::Lower
            }
            LogicalKeyboardKey::ARROW_RIGHT | LogicalKeyboardKey::ARROW_UP => {
                IncrementDirection::Higher
            }
            _ => return None,
        };
        let Some(picker) = self.parent_color_picker(app) else {
            return Some(KeyEventResult::Ignored);
        };
        let amount = if HardwareKeyboard::instance(app).is_control_pressed(app) {
            IncrementAmount::Large
        } else {
            IncrementAmount::Small
        };
        let widget = self.widget(app).clone();
        let bounds = picker.widget(app).clone();
        let mut hsv: Hsv = picker.current_hsv(app);
        let value = widget.value;
        let new_value = match widget.color_channel {
            ColorPickerHsvChannel::Alpha => {
                increment_alpha_channel(value / 100.0, direction, amount, false, 0.0, 100.0) * 100.0
            }
            channel => {
                let (min_bound, max_bound) = match channel {
                    ColorPickerHsvChannel::Hue => {
                        hsv.h = value;
                        (bounds.min_hue, bounds.max_hue)
                    }
                    ColorPickerHsvChannel::Saturation => {
                        hsv.s = value / 100.0;
                        (bounds.min_saturation, bounds.max_saturation)
                    }
                    _ => {
                        hsv.v = value / 100.0;
                        (bounds.min_value, bounds.max_value)
                    }
                };
                let hsv = increment_color_channel(
                    hsv,
                    channel,
                    direction,
                    amount,
                    false,
                    f64::from(min_bound),
                    f64::from(max_bound),
                );
                match channel {
                    ColorPickerHsvChannel::Hue => hsv.h,
                    ColorPickerHsvChannel::Saturation => hsv.s * 100.0,
                    _ => hsv.v * 100.0,
                }
            }
        };
        // `put_Value`: coerced into the range; `ValueChanged` only when it differs.
        let coerced = RangeBase::new(widget.minimum, widget.maximum, new_value)
            .coerced()
            .value;
        let current = RangeBase::new(widget.minimum, widget.maximum, value)
            .coerced()
            .value;
        if coerced != current {
            (widget.value_changed)(app, coerced);
        }
        Some(KeyEventResult::Handled)
    }

    /// `OnGotFocus` and `OnLostFocus`: arriving focus refreshes and enables `ToolTip` and opens
    /// it; leaving closes it.
    fn focus_changed(self: Handle<Self>, app: &mut App, focused: bool) {
        let text = focused.then(|| self.tool_tip_string(app));
        self.set_state(app, |state| {
            state.focused = focused;
            if let Some(text) = text {
                state.tool_tip_text = text;
                state.tool_tip_enabled = true;
            }
        });
    }
}

impl State for ColorPickerSliderState {
    type Widget = ColorPickerSlider;
    inset_widgets::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        // `OnApplyTemplate` sets `ToolTip`'s content once the slider is in the tree.
        if app.get(self).tool_tip_text.is_empty() {
            app.get_mut(self).tool_tip_text = self.tool_tip_string(app);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &ColorPickerSlider) {
        let widget = self.widget(app).clone();
        if old.is_thumb_tool_tip_enabled != widget.is_thumb_tool_tip_enabled {
            // `Slider::OnIsThumbToolTipEnabledChanged`.
            app.get_mut(self).tool_tip_enabled = widget.is_thumb_tool_tip_enabled;
        }
        if old.value != widget.value {
            // `OnValueChangedEvent`: `ToolTip` shows the new value and is enabled.
            let text = self.tool_tip_string(app);
            let state = app.get_mut(self);
            state.tool_tip_text = text;
            state.tool_tip_enabled = true;
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let mut slider = Slider::new(widget.value, {
            let changed = widget.value_changed.clone();
            move |app, value| changed(app, value)
        })
        .minimum(widget.minimum)
        .maximum(widget.maximum)
        .step_frequency(widget.step_frequency)
        .orientation(widget.orientation)
        .is_direction_reversed(widget.is_direction_reversed)
        .is_thumb_tool_tip_enabled(widget.is_thumb_tool_tip_enabled)
        .is_enabled(widget.is_enabled)
        .template(SliderTemplate::new(
            0.0,
            move |app, context, slider, states, parts| {
                self.template(app, context, slider, states, parts)
            },
        ))
        .key_down(move |app, key| self.key_down(app, key));
        if let Some(header) = widget.header.clone() {
            slider = slider.header(header);
        }
        Focus::new(slider)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_focus_change(move |app, focused| self.focus_changed(app, focused))
            .into_widget()
    }
}

impl ColorPickerSliderState {
    /// The template of `ColorPickerSliderStyle`.
    fn template(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        slider: &Slider,
        states: SliderTemplateStates,
        parts: &SliderTemplateParts,
    ) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let r = &resources;
        let widget = self.widget(app).clone();
        let thumbs = ThumbVisuals::of(r, states.slider);
        let content = match slider.orientation {
            Orientation::Horizontal => self.horizontal_template(app, r, slider, states, thumbs),
            Orientation::Vertical => vertical_template(r, slider, states, thumbs),
        };
        // `SliderContainer`: transparent, and the target of the focus visual.
        let container =
            parts.slider_container(ColoredBox::new(Color::from_argb(0, 0, 0, 0)).child(content));
        let mut children = Vec::new();
        if let Some(header) = &slider.header {
            // `HeaderContentPresenter`.
            let [left, top, right, bottom] = r.slider_header_theme_margin;
            let text = control_text_style(
                &resources,
                r.control_content_theme_font_size,
                r.slider_header_theme_font_weight,
                thumbs.header,
            );
            children.push(
                GridCell::new(
                    Padding::new(EdgeInsetsGeometry::from_ltrb(left, top, right, bottom))
                        .child(DefaultTextStyle::new(text, header.clone()).soft_wrap(true)),
                )
                .row(0)
                .into_widget(),
            );
        }
        children.push(
            GridCell::new(
                FocusVisual::new(container, resources.theme)
                    .visible(states.focused)
                    .margin(SLIDER_FOCUS_VISUAL_MARGIN)
                    .corner_radius(widget.corner_radius.unwrap_or(r.control_corner_radius[0])),
            )
            .row(1)
            .into_widget(),
        );
        Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::STAR),
            ])
            .children(children)
            .into_widget()
    }

    /// `HorizontalTemplate`: the thumb in column 1, after the decrease column, across all rows.
    fn horizontal_template(
        self: Handle<Self>,
        app: &App,
        r: &ThemeResources,
        slider: &Slider,
        states: SliderTemplateStates,
        thumbs: ThumbVisuals,
    ) -> WidgetRef {
        let state = app.get(self);
        // `ToolTip`: opened by the slider while it shows thumb tooltips, or by `OnGotFocus`.
        let mut tool_tip = ToolTipService::new(
            thumb(r, thumbs.horizontal),
            ToolTip::text(state.tool_tip_text.clone())
                .vertical_offset(THUMB_TOOL_TIP_OFFSET)
                .placement_rect(thumb_slot(r)),
        )
        .is_enabled(state.tool_tip_enabled);
        if states.tooltip_open || state.focused {
            tool_tip = tool_tip.is_open(true);
        }
        // `HorizontalTrackRect` and `HorizontalDecreaseRect` are transparent at opacity 0; the
        // track only stretches the slider to its bounded width, as `Slider`'s does.
        let track = GridCell::new(fill_bounded(Orientation::Horizontal, SizedBox::shrink()))
            .row(1)
            .column_span(3)
            .into_widget();
        let thumb = GridCell::new(
            Center::new()
                .width_factor(1.0)
                .height_factor(1.0)
                .child(tool_tip),
        )
        .row(0)
        .row_span(3)
        .column(1)
        .into_widget();
        let fraction = slider.range().fraction_of(states.intermediate_value);
        let (first, last) = decrease_definitions(fraction, slider.is_direction_reversed);
        let grid = Grid::new()
            .column_definitions([
                ColumnDefinition::new(first),
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(last),
            ])
            .row_definitions([
                RowDefinition::new(GridLength::pixel(r.slider_pre_content_margin)),
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::pixel(r.slider_post_content_margin)),
            ])
            .children([track, thumb]);
        ConstrainedBox::new(BoxConstraints::new().min_height(TEMPLATE_MIN_SIZE))
            .child(grid)
            .into_widget()
    }
}

/// `VerticalTemplate`: the thumb in row 1, across all columns, with `VerticalToolTip`, which
/// only the slider's own thumb tooltip logic opens and which has no content.
fn vertical_template(
    r: &ThemeResources,
    slider: &Slider,
    states: SliderTemplateStates,
    thumbs: ThumbVisuals,
) -> WidgetRef {
    let mut tool_tip = ToolTipService::new(
        thumb(r, thumbs.vertical),
        ToolTip::new(SizedBox::shrink())
            .vertical_offset(THUMB_TOOL_TIP_OFFSET)
            .placement_rect(thumb_slot(r)),
    )
    .is_enabled(slider.is_thumb_tool_tip_enabled);
    if states.tooltip_open {
        tool_tip = tool_tip.is_open(true);
    }
    // `VerticalTrackRect`, transparent: it only stretches the slider to its bounded height.
    let track = GridCell::new(fill_bounded(Orientation::Vertical, SizedBox::shrink()))
        .column(1)
        .row_span(3)
        .into_widget();
    let thumb = GridCell::new(
        Center::new()
            .width_factor(1.0)
            .height_factor(1.0)
            .child(tool_tip),
    )
    .row(1)
    .column(0)
    .column_span(3)
    .into_widget();
    let fraction = slider.range().fraction_of(states.intermediate_value);
    // The decrease rect is in the last row unless reversed.
    let (last, first) = decrease_definitions(fraction, slider.is_direction_reversed);
    let grid = Grid::new()
        .row_definitions([
            RowDefinition::new(first),
            RowDefinition::new(GridLength::AUTO),
            RowDefinition::new(last),
        ])
        .column_definitions([
            ColumnDefinition::new(GridLength::pixel(r.slider_pre_content_margin)),
            ColumnDefinition::new(GridLength::AUTO),
            ColumnDefinition::new(GridLength::pixel(r.slider_post_content_margin)),
        ])
        .children([track, thumb]);
    ConstrainedBox::new(BoxConstraints::new().min_width(TEMPLATE_MIN_SIZE))
        .child(grid)
        .into_widget()
}

/// A thumb's slot, which its tooltip docks to as a slider thumb's does.
fn thumb_slot(r: &ThemeResources) -> Rect {
    Rect::from_ltwh(
        0.0,
        0.0,
        r.slider_horizontal_thumb_width,
        r.slider_horizontal_thumb_height,
    )
}

/// The brushes the template's `CommonStates` set: the thumbs' `Background` and the header's
/// `Foreground`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ThumbVisuals {
    /// `HorizontalThumb.Background`, which every state sets.
    horizontal: Color,
    /// `VerticalThumb.Background`, which no state sets.
    vertical: Color,
    /// `HeaderContentPresenter.Foreground`.
    header: Color,
}

impl ThumbVisuals {
    fn of(r: &ThemeResources, state: CommonState) -> ThumbVisuals {
        let horizontal = match state {
            CommonState::Normal => r.color_picker_slider_thumb_background,
            CommonState::PointerOver => r.color_picker_slider_thumb_background_pointer_over,
            CommonState::Pressed => r.color_picker_slider_thumb_background_pressed,
            CommonState::Disabled => r.color_picker_slider_thumb_background_disabled,
        };
        ThumbVisuals {
            horizontal,
            vertical: r.color_picker_slider_thumb_background,
            header: if state == CommonState::Disabled {
                r.color_picker_header_content_disabled
            } else {
                r.slider_header_foreground
            },
        }
    }
}

/// A thumb in the template's `SliderThumbStyle`: an 18 square `Border` with
/// `SliderThumbCornerRadius`, `SliderOuterThumbBackground` inside `SliderThumbBorderBrush`, and
/// the 10 pixel `SliderInnerThumb` filled with `fill`.
fn thumb(r: &ThemeResources, fill: Color) -> WidgetRef {
    SizedBox::new()
        .width(r.slider_horizontal_thumb_width)
        .height(r.slider_horizontal_thumb_height)
        .child(
            ControlBorder::new(
                Brush::Solid(r.slider_outer_thumb_background),
                Brush::ControlElevation(r.slider_thumb_border_brush),
            )
            .border_thickness(THUMB_BORDER_THICKNESS)
            .corner_radius(r.slider_thumb_corner_radius[0])
            .child(
                Center::new().child(
                    SizedBox::new()
                        .width(r.color_picker_slider_inner_thumb_width)
                        .height(r.color_picker_slider_inner_thumb_height)
                        .child(CustomPaint::new().painter(InnerThumbPainter { fill })),
                ),
            ),
        )
        .into_widget()
}

/// `SliderInnerThumb`: an ellipse filling its box.
#[derive(Clone, Debug, PartialEq)]
struct InnerThumbPainter {
    fill: Color,
}

impl CustomPainter for InnerThumbPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let bounds = Rect::from_ltwh(0.0, 0.0, size.width(), size.height());
        draw_oval(
            canvas,
            bounds,
            &inset_embedder::Paint::from_color(self.fill.into()),
        );
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
