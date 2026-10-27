//! ColorPicker.xaml, ColorPicker_themeresources.xaml and ColorPicker.cpp: a colour chosen on a
//! spectrum, with a slider for the spectrum's third component, an opacity slider, a preview, and
//! text boxes for the RGB or HSV channels, the opacity and the hex value.
//!
//! Template, as the `Vertical` orientation lays it out: `RootGrid` (312 to 392 wide, padded 4
//! above and below) holds a grid of four Auto rows and four Auto columns. Row 0 is
//! `ColorSpectrumGrid` (columns *, Auto): the `ColorSpectrum` (256 to 336 square) and
//! `ColorPreviewRectangleGrid` (44 wide, 12 after the spectrum), whose checkered background,
//! `ColorPreviewRectangle` and `PreviousColorRectangle` (the lower half while there is a previous
//! colour) and `BorderRectangle` fill it. Row 1 is `ThirdDimensionSliderGrid`, a 12 tall gradient
//! of the third component (`ThirdDimensionBackgroundRectangle`) under `ThirdDimensionSlider`.
//! Row 2 is `AlphaSliderGrid`: a checkered background, the opacity gradient and `AlphaSlider`.
//! Row 3 is `MoreEntriesPanel`: `MoreButton`, a borderless toggle button, over `TextEntryGrid`
//! (columns *, 5, *): `ColorRepresentationComboBox` (RGB or HSV), `HexTextBox` beside it, and
//! below them `RgbPanel` or `HsvPanel`, three text boxes each with a label, and `AlphaPanel`.
//! The `Horizontal` orientation puts the spectrum, the two sliders (turned vertical) and the
//! entries side by side in row 0.
//!
//! The application owns the colour: `color` is what it passes, and `color_changed` asks for
//! another, with the colour before. The colour the picker asks for is what the picker reads as
//! its own until the application next builds it; if the application passed another by then, the
//! picker shows that one. The picker keeps the hue and saturation it was given while the colour
//! has none, as WinUI's does, for as long as the application passes back the colours it asks
//! for.

use super::toggle_button::{ToggleButtonSetters, toggle_button_presenter};
use crate::after_next_frame;
use crate::{
    Brush, COLOR_PICKER_BORDER_THICKNESS, ColorChangedEventArgs, ColorPickerHsvChannel,
    ColorPickerSlider, ColorSpectrum, ColorSpectrumColorChangedEventArgs, ColorSpectrumComponents,
    ColorSpectrumShape, ColumnDefinition, ComboBox, ComboBoxItem, CommonState, FluentSymbol,
    FontIcon, Grid, GridCell, GridLength, Hsv, HsvColor, Margin, Orientation, Rgb, RowDefinition,
    SizeObserver, TextBox, ThemeResources, ToggleButton, ToggleButtonBrushes, argb,
    checkered_background_pixels, color_from_rgba, control_text_style, hex_to_rgb, hex_to_rgba,
    hsv_to_rgb, image_from_pixel_data, rgb_from_color, rgb_to_hex, rgb_to_hsv, rgba_to_hex,
    same_color, try_parse_int, wtoi,
};
use inset_embedder::valo::{
    GradientStop, Matrix, Paint, PaintStyle, Point, Shader, SpreadMode, Stroke,
};
use inset_embedder::{Canvas, Color, FontWeight, Image, Rect, Size, TextAffinity, TextSelection};
use inset_foundation::{App, Handle, Listenable, Listener};
use inset_painting::AlignmentGeometry;
use inset_rendering::{BoxConstraints, CustomPainter};
use inset_scheduler::FrameCallback;
use inset_services::TextEditingValue;
use inset_widgets::*;
use std::{fmt, rc::Rc};

/// Receives XAML `ColorPicker.ColorChanged`.
pub type ColorPickerColorChangedHandler = Rc<dyn Fn(&mut App, ColorChangedEventArgs)>;

/// `ColorSpectrum`'s `MinWidth` and `MinHeight` in the template.
const SPECTRUM_MIN_SIZE: f64 = 256.0;
/// `ColorSpectrum`'s `MaxWidth` and `MaxHeight` in the template.
const SPECTRUM_MAX_SIZE: f64 = 336.0;
/// `ColorPreviewRectangleGrid`'s `Width`, and its `Height` while the spectrum is collapsed.
const PREVIEW_SIZE: f64 = 44.0;
/// The `Height` of the sliders' background rectangles, and their `Width` when vertical.
const SLIDER_BACKGROUND_SIZE: f64 = 12.0;
/// `MoreButton`'s `MinHeight`.
const MORE_BUTTON_MIN_HEIGHT: f64 = 32.0;
/// `MoreButton`'s `MinWidth`.
const MORE_BUTTON_MIN_WIDTH: f64 = 120.0;
/// `MoreGlyph`'s `FontSize`.
const MORE_GLYPH_FONT_SIZE: f64 = 12.0;
/// `Width` of the channel text boxes and the colour model box.
const TEXT_BOX_WIDTH: f64 = 120.0;
/// `HexTextBox`'s `Width` in the `Vertical` orientation.
const HEX_TEXT_BOX_WIDTH: f64 = 132.0;
/// `MaxLength` of a colour channel's text box.
const CHANNEL_MAX_LENGTH: i32 = 3;
/// `AlphaTextBox`'s `MaxLength`.
const ALPHA_MAX_LENGTH: i32 = 4;
/// `HexTextBox`'s `MaxLength`, and with alpha enabled (`AlphaEnabled`).
const HEX_MAX_LENGTH: [i32; 2] = [7, 9];
/// `TabIndex` an element without one has: `Int32.MaxValue`.
const DEFAULT_TAB_INDEX: i32 = i32::MAX;

/// XAML `ColorPicker` with the template of `ColorPicker.xaml`.
#[derive(Clone)]
pub struct ColorPicker {
    /// `Color`: white by default.
    pub color: Color,

    /// `ColorChanged`: asks the application for another colour; the application owns `color`.
    pub color_changed: ColorPickerColorChangedHandler,

    /// `PreviousColor`: shown beside the colour while set.
    pub previous_color: Option<Color>,

    /// `IsAlphaEnabled`: whether opacity can be chosen.
    pub is_alpha_enabled: bool,

    /// `IsColorSpectrumVisible`; true by default.
    pub is_color_spectrum_visible: bool,
    /// `IsColorPreviewVisible`; true by default.
    pub is_color_preview_visible: bool,
    /// `IsColorSliderVisible`: the third component's slider; true by default.
    pub is_color_slider_visible: bool,
    /// `IsAlphaSliderVisible`, while alpha is enabled; true by default.
    pub is_alpha_slider_visible: bool,
    /// `IsMoreButtonVisible`: the text entries hide behind a More button, in the vertical
    /// orientation only.
    pub is_more_button_visible: bool,
    /// `IsColorChannelTextInputVisible`; true by default.
    pub is_color_channel_text_input_visible: bool,
    /// `IsAlphaTextInputVisible`, while alpha is enabled; true by default.
    pub is_alpha_text_input_visible: bool,
    /// `IsHexInputVisible`; true by default.
    pub is_hex_input_visible: bool,

    /// `MinHue`, from 0 to 359; 0 by default.
    pub min_hue: i32,
    /// `MaxHue`, from 0 to 359; 359 by default.
    pub max_hue: i32,
    /// `MinSaturation`, from 0 to 100; 0 by default.
    pub min_saturation: i32,
    /// `MaxSaturation`, from 0 to 100; 100 by default.
    pub max_saturation: i32,
    /// `MinValue`, from 0 to 100; 0 by default.
    pub min_value: i32,
    /// `MaxValue`, from 0 to 100; 100 by default.
    pub max_value: i32,

    /// `ColorSpectrumShape`.
    pub color_spectrum_shape: ColorSpectrumShape,

    /// `ColorSpectrumComponents`.
    pub color_spectrum_components: ColorSpectrumComponents,

    /// `Orientation`: `Vertical` by default.
    pub orientation: Orientation,

    /// `CornerRadius` of the spectrum, the preview and the opacity slider's background;
    /// `ControlCornerRadius` by default; the theme's `ControlCornerRadius` when `None`.
    pub corner_radius: Option<f64>,

    /// `IsEnabled`.
    pub is_enabled: bool,

    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

/// Panics unless `value` is within `0..=max`, as the source throws for the property `name`.
fn assert_range(name: &str, value: i32, max: i32) {
    assert!(
        (0..=max).contains(&value),
        "{name} must be between 0 and {max}."
    );
}

impl ColorPicker {
    /// Creates a picker showing `color`, which asks for others through `color_changed`.
    pub fn new(
        color: Color,
        color_changed: impl Fn(&mut App, ColorChangedEventArgs) + 'static,
    ) -> ColorPicker {
        ColorPicker {
            color,
            color_changed: Rc::new(color_changed),
            previous_color: None,
            is_alpha_enabled: false,
            is_color_spectrum_visible: true,
            is_color_preview_visible: true,
            is_color_slider_visible: true,
            is_alpha_slider_visible: true,
            is_more_button_visible: false,
            is_color_channel_text_input_visible: true,
            is_alpha_text_input_visible: true,
            is_hex_input_visible: true,
            min_hue: 0,
            max_hue: 359,
            min_saturation: 0,
            max_saturation: 100,
            min_value: 0,
            max_value: 100,
            color_spectrum_shape: ColorSpectrumShape::Box,
            color_spectrum_components: ColorSpectrumComponents::HueSaturation,
            orientation: Orientation::Vertical,
            corner_radius: None,
            is_enabled: true,
            key: None,
        }
    }

    /// Sets PreviousColor.
    pub fn previous_color(mut self, value: Color) -> Self {
        self.previous_color = Some(value);
        self
    }

    /// Sets IsAlphaEnabled.
    pub fn is_alpha_enabled(mut self, value: bool) -> Self {
        self.is_alpha_enabled = value;
        self
    }

    /// Sets IsColorSpectrumVisible.
    pub fn is_color_spectrum_visible(mut self, value: bool) -> Self {
        self.is_color_spectrum_visible = value;
        self
    }

    /// Sets IsColorPreviewVisible.
    pub fn is_color_preview_visible(mut self, value: bool) -> Self {
        self.is_color_preview_visible = value;
        self
    }

    /// Sets IsColorSliderVisible.
    pub fn is_color_slider_visible(mut self, value: bool) -> Self {
        self.is_color_slider_visible = value;
        self
    }

    /// Sets IsAlphaSliderVisible.
    pub fn is_alpha_slider_visible(mut self, value: bool) -> Self {
        self.is_alpha_slider_visible = value;
        self
    }

    /// Sets IsMoreButtonVisible.
    pub fn is_more_button_visible(mut self, value: bool) -> Self {
        self.is_more_button_visible = value;
        self
    }

    /// Sets IsColorChannelTextInputVisible.
    pub fn is_color_channel_text_input_visible(mut self, value: bool) -> Self {
        self.is_color_channel_text_input_visible = value;
        self
    }

    /// Sets IsAlphaTextInputVisible.
    pub fn is_alpha_text_input_visible(mut self, value: bool) -> Self {
        self.is_alpha_text_input_visible = value;
        self
    }

    /// Sets IsHexInputVisible.
    pub fn is_hex_input_visible(mut self, value: bool) -> Self {
        self.is_hex_input_visible = value;
        self
    }

    /// Sets MinHue.
    pub fn min_hue(mut self, value: i32) -> Self {
        assert_range("MinHue", value, 359);
        self.min_hue = value;
        self
    }

    /// Sets MaxHue.
    pub fn max_hue(mut self, value: i32) -> Self {
        assert_range("MaxHue", value, 359);
        self.max_hue = value;
        self
    }

    /// Sets MinSaturation.
    pub fn min_saturation(mut self, value: i32) -> Self {
        assert_range("MinSaturation", value, 100);
        self.min_saturation = value;
        self
    }

    /// Sets MaxSaturation.
    pub fn max_saturation(mut self, value: i32) -> Self {
        assert_range("MaxSaturation", value, 100);
        self.max_saturation = value;
        self
    }

    /// Sets MinValue.
    pub fn min_value(mut self, value: i32) -> Self {
        assert_range("MinValue", value, 100);
        self.min_value = value;
        self
    }

    /// Sets MaxValue.
    pub fn max_value(mut self, value: i32) -> Self {
        assert_range("MaxValue", value, 100);
        self.max_value = value;
        self
    }

    /// Sets ColorSpectrumShape.
    pub fn color_spectrum_shape(mut self, value: ColorSpectrumShape) -> Self {
        self.color_spectrum_shape = value;
        self
    }

    /// Sets ColorSpectrumComponents.
    pub fn color_spectrum_components(mut self, value: ColorSpectrumComponents) -> Self {
        self.color_spectrum_components = value;
        self
    }

    /// Sets Orientation.
    pub fn orientation(mut self, value: Orientation) -> Self {
        self.orientation = value;
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

    /// `ApplyConstraintsToRgbColor`: `rgb` with its hue, saturation and value clamped to the
    /// ranges.
    fn apply_constraints_to_rgb_color(&self, rgb: Rgb) -> Rgb {
        let mut hsv = rgb_to_hsv(rgb);
        hsv.h = hsv
            .h
            .max(f64::from(self.min_hue))
            .min(f64::from(self.max_hue));
        hsv.s = hsv
            .s
            .max(f64::from(self.min_saturation) / 100.0)
            .min(f64::from(self.max_saturation) / 100.0);
        hsv.v = hsv
            .v
            .max(f64::from(self.min_value) / 100.0)
            .min(f64::from(self.max_value) / 100.0);
        hsv_to_rgb(hsv)
    }

    /// The channel `SetThirdDimensionSliderChannel` gives the third dimension's slider: the
    /// component the spectrum does not lay out.
    fn third_dimension_channel(&self) -> ColorPickerHsvChannel {
        match self.color_spectrum_components {
            ColorSpectrumComponents::ValueSaturation | ColorSpectrumComponents::SaturationValue => {
                ColorPickerHsvChannel::Hue
            }
            ColorSpectrumComponents::HueValue | ColorSpectrumComponents::ValueHue => {
                ColorPickerHsvChannel::Saturation
            }
            ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue => {
                ColorPickerHsvChannel::Value
            }
        }
    }

    /// `Minimum` and `Maximum` of the third dimension's slider (`UpdateThirdDimensionSlider`).
    fn third_dimension_range(&self) -> (i32, i32) {
        match self.third_dimension_channel() {
            ColorPickerHsvChannel::Hue => (self.min_hue, self.max_hue),
            ColorPickerHsvChannel::Saturation => (self.min_saturation, self.max_saturation),
            _ => (self.min_value, self.max_value),
        }
    }
}

impl fmt::Debug for ColorPicker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColorPicker")
            .field("color", &self.color)
            .field("is_alpha_enabled", &self.is_alpha_enabled)
            .field("orientation", &self.orientation)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

/// `ColorUpdateReason`: what changed the colour, whose control is then not updated from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColorUpdateReason {
    InitializingColor,
    ColorPropertyChanged,
    ColorSpectrumColorChanged,
    ThirdDimensionSliderChanged,
    AlphaSliderChanged,
    RgbTextBoxChanged,
    HsvTextBoxChanged,
    AlphaTextBoxChanged,
    HexTextBoxChanged,
}

/// The template's text boxes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChannelTextBox {
    Red,
    Green,
    Blue,
    Hue,
    Saturation,
    Value,
    Alpha,
    Hex,
}

impl ChannelTextBox {
    /// Every text box, in the template's order.
    const ALL: [ChannelTextBox; 8] = [
        ChannelTextBox::Red,
        ChannelTextBox::Green,
        ChannelTextBox::Blue,
        ChannelTextBox::Hue,
        ChannelTextBox::Saturation,
        ChannelTextBox::Value,
        ChannelTextBox::Alpha,
        ChannelTextBox::Hex,
    ];

    fn index(self) -> usize {
        self as usize
    }

    /// The label beside the text box.
    fn label(self) -> &'static str {
        match self {
            ChannelTextBox::Red => "Red",
            ChannelTextBox::Green => "Green",
            ChannelTextBox::Blue => "Blue",
            ChannelTextBox::Hue => "Hue",
            ChannelTextBox::Saturation => "Saturation",
            ChannelTextBox::Value => "Value",
            ChannelTextBox::Alpha => "Opacity",
            ChannelTextBox::Hex => "",
        }
    }

    /// The text the template gives the box before the colour is set.
    fn initial_text(self) -> &'static str {
        match self {
            ChannelTextBox::Red | ChannelTextBox::Green | ChannelTextBox::Blue => "255",
            ChannelTextBox::Hue | ChannelTextBox::Saturation => "0",
            ChannelTextBox::Value => "100",
            ChannelTextBox::Alpha => "100%",
            ChannelTextBox::Hex => "#FFFFFF",
        }
    }
}

/// A text box of the template: its text, its focus and the listener on the focus.
struct TextBoxPart {
    controller: Handle<TextEditingController>,
    focus: Handle<FocusNode>,
    focus_listener: Listener,
}

/// The template's visual state groups, each the state `UpdateVisualState` goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VisualStates {
    /// `ColorSpectrumVisibility`: `ColorSpectrumVisible` or `ColorSpectrumCollapsed`.
    color_spectrum_visible: bool,
    /// `ColorPreviewVisibility`.
    color_preview_visible: bool,
    /// `PreviousColorVisibility`.
    previous_color: PreviousColorVisibility,
    /// `ThirdDimensionSliderVisibility`.
    third_dimension_slider_visible: bool,
    /// `AlphaSliderVisibility`.
    alpha_slider_visible: bool,
    /// `MoreButtonVisibility`.
    more_button_visible: bool,
    /// `TextEntryGridVisibility`: `TextEntryGridVisible` shows the entries and turns the More
    /// button's chevron up.
    text_entry_grid_visible: bool,
    /// `ColorChannelTextInputVisibility`.
    color_channel_text_input_visible: bool,
    /// `AlphaTextInputVisibility`.
    alpha_text_input_visible: bool,
    /// `ColorRepresentationSelected`.
    color_representation: ColorRepresentation,
    /// `HexInputVisibility`.
    hex_input_visible: bool,
    /// `AlphaEnabledState`: `AlphaEnabled` lets the hex box take 9 characters.
    alpha_enabled: bool,
    /// `Orientation`.
    orientation: Orientation,
}

/// `PreviousColorVisibility`: with the spectrum (`Vertical`) the previous colour is the lower
/// half of the preview, without it (`Horizontal`) the right half.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names, reason = "the template's state names")]
enum PreviousColorVisibility {
    PreviousColorCollapsedVertical,
    PreviousColorVisibleVertical,
    PreviousColorCollapsedHorizontal,
    PreviousColorVisibleHorizontal,
}

/// `ColorRepresentationSelected`, the item of `ColorRepresentationComboBox`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names, reason = "the template's state names")]
enum ColorRepresentation {
    RgbSelected,
    HsvSelected,
}

impl StatefulWidget for ColorPicker {
    type State = ColorPickerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ColorPickerState {
        ColorPickerState {
            state: StateData::new(),
            color: self.color,
            current_rgb: Rgb::new(1.0, 1.0, 1.0),
            current_hsv: Hsv::new(0.0, 1.0, 1.0),
            current_hex: "#FFFFFFFF".to_owned(),
            current_alpha: 1.0,
            previous_string: String::new(),
            is_focused_text_box_valid: false,
            text_entry_grid_opened: false,
            color_representation: 0,
            spectrum_hsv: HsvColor::WHITE,
            spectrum_height: None,
            third_dimension_value: 0.0,
            alpha_slider_value: 0.0,
            text_boxes: Vec::new(),
            traversal_policy: None,
            preview_size: Size::ZERO,
            alpha_background_size: Size::ZERO,
            checker_color: None,
            preview_checkers: None,
            alpha_checkers: None,
            updating_widget: false,
        }
    }
}

/// The parts ColorPicker.cpp keeps of its template, and its colour.
pub struct ColorPickerState {
    state: StateData<ColorPicker>,

    /// `Color` as the picker reads it: the colour it last asked for until the application next
    /// builds it, else the application's.
    color: Color,

    /// `m_currentRgb`.
    current_rgb: Rgb,
    /// `m_currentHsv`.
    current_hsv: Hsv,
    /// `m_currentHex`.
    current_hex: String,
    /// `m_currentAlpha`.
    current_alpha: f64,

    /// `m_previousString`: the focused text box's text when it took the focus.
    previous_string: String,
    /// `m_isFocusedTextBoxValid`.
    is_focused_text_box_valid: bool,

    /// `m_textEntryGridOpened`: `MoreButton` is checked.
    text_entry_grid_opened: bool,

    /// `ColorRepresentationComboBox.SelectedIndex`: 0 for RGB, 1 for HSV.
    color_representation: usize,

    /// `ColorSpectrum.HsvColor`, as the picker last set it or the spectrum asked for it.
    spectrum_hsv: HsvColor,

    /// `ColorSpectrum.Height`, which `OnColorSpectrumSizeChanged` keeps at its width.
    spectrum_height: Option<f64>,

    /// `ThirdDimensionSlider.Value`.
    third_dimension_value: f64,

    /// `AlphaSlider.Value`.
    alpha_slider_value: f64,

    /// The text boxes, by `ChannelTextBox::index`.
    text_boxes: Vec<TextBoxPart>,

    /// Orders the focus by the template's `TabIndex`.
    traversal_policy: Option<Handle<OrderedTraversalPolicy>>,

    /// `ColorPreviewRectangleGrid`'s size after the last layout.
    preview_size: Size,
    /// `AlphaSliderBackgroundRectangle`'s size after the last layout.
    alpha_background_size: Size,
    /// `CheckerColorBrush.Color` the checkered backgrounds were made with.
    checker_color: Option<Color>,
    /// `ColorPreviewRectangleCheckeredBackgroundImageBrush.ImageSource`.
    preview_checkers: Option<Image>,
    /// `AlphaSliderCheckeredBackgroundImageBrush.ImageSource`.
    alpha_checkers: Option<Image>,

    /// The application's properties are being applied, during which `ColorChanged` waits for the
    /// end of the frame.
    updating_widget: bool,
}

impl ColorPickerState {
    /// `GetCurrentHsv`.
    pub(crate) fn current_hsv(self: Handle<Self>, app: &App) -> Hsv {
        app.get(self).current_hsv
    }

    fn part(self: Handle<Self>, app: &App, text_box: ChannelTextBox) -> &TextBoxPart {
        &app.get(self).text_boxes[text_box.index()]
    }

    fn text(self: Handle<Self>, app: &App, text_box: ChannelTextBox) -> String {
        let controller = self.part(app, text_box).controller;
        controller.text_value(app).to_owned()
    }

    /// `TextBox.Text = text`: the box shows `text` with the caret at the start, when it shows
    /// other text.
    fn set_text(self: Handle<Self>, app: &mut App, text_box: ChannelTextBox, text: &str) {
        let controller = self.part(app, text_box).controller;
        if controller.text_value(app) != text {
            controller.set_value(
                app,
                TextEditingValue::new()
                    .text(text)
                    .selection(TextSelection::collapsed(0, TextAffinity::Downstream)),
            );
        }
    }

    /// `GetCurrentHexValue`.
    fn current_hex_value(self: Handle<Self>, app: &App) -> String {
        let state = app.get(self);
        if self.widget(app).is_alpha_enabled {
            rgba_to_hex(state.current_rgb, state.current_alpha)
        } else {
            rgb_to_hex(state.current_rgb)
        }
    }

    /// The current colour from `color`: `OnColorChanged` and `InitializeColor`.
    fn read_color(self: Handle<Self>, app: &mut App, color: Color) {
        let rgb = rgb_from_color(color);
        let state = app.get_mut(self);
        state.current_rgb = rgb;
        state.current_alpha = f64::from(argb(color).a) / 255.0;
        state.current_hsv = rgb_to_hsv(rgb);
        app.get_mut(self).current_hex = self.current_hex_value(app);
    }

    /// `UpdateColor` with an RGB colour.
    fn update_color_rgb(self: Handle<Self>, app: &mut App, rgb: Rgb, reason: ColorUpdateReason) {
        let state = app.get_mut(self);
        state.current_rgb = rgb;
        state.current_hsv = rgb_to_hsv(rgb);
        app.get_mut(self).current_hex = self.current_hex_value(app);
        self.set_color_and_update_controls(app, reason);
    }

    /// `UpdateColor` with an HSV colour.
    fn update_color_hsv(self: Handle<Self>, app: &mut App, hsv: Hsv, reason: ColorUpdateReason) {
        let state = app.get_mut(self);
        state.current_hsv = hsv;
        state.current_rgb = hsv_to_rgb(hsv);
        app.get_mut(self).current_hex = self.current_hex_value(app);
        self.set_color_and_update_controls(app, reason);
    }

    /// `UpdateColor` with an alpha.
    fn update_color_alpha(
        self: Handle<Self>,
        app: &mut App,
        alpha: f64,
        reason: ColorUpdateReason,
    ) {
        app.get_mut(self).current_alpha = alpha;
        app.get_mut(self).current_hex = self.current_hex_value(app);
        self.set_color_and_update_controls(app, reason);
    }

    /// `SetColorAndUpdateControls`: `Color` becomes the current colour, which raises
    /// `ColorChanged` when it changes, and then the controls show it.
    fn set_color_and_update_controls(self: Handle<Self>, app: &mut App, reason: ColorUpdateReason) {
        let state = app.get(self);
        let new = color_from_rgba(state.current_rgb, state.current_alpha);
        let old = state.color;
        app.get_mut(self).color = new;
        if !same_color(old, new) {
            self.raise_color_changed(app, old, new);
        }
        if !app.contains(self) || !self.mounted(app) {
            return;
        }
        self.update_color_controls(app, reason);
        if !app.get(self).updating_widget {
            self.set_state(app, |_| {});
        }
    }

    /// `OnColorChanged`'s event: `ColorChanged` asks the application for `new`, at once for a
    /// change the user made, and once the frame is over for one the picker made to what the
    /// application passed, if the picker still has that colour then.
    fn raise_color_changed(self: Handle<Self>, app: &mut App, old: Color, new: Color) {
        let args = ColorChangedEventArgs {
            old_color: old,
            new_color: new,
        };
        if !app.get(self).updating_widget {
            let handler = self.widget(app).color_changed.clone();
            handler(app, args);
            return;
        }
        let view = View::maybe_id_of(app, self.context(app));
        after_next_frame(
            app,
            view,
            FrameCallback::new(move |app, _| {
                if !app.contains(self)
                    || !self.mounted(app)
                    || !same_color(app.get(self).color, new)
                {
                    return;
                }
                let handler = self.widget(app).color_changed.clone();
                handler(app, args);
            }),
        );
    }

    /// `UpdateColorControls`: every control but the one that changed the colour shows it.
    fn update_color_controls(self: Handle<Self>, app: &mut App, reason: ColorUpdateReason) {
        let state = app.get(self);
        let (rgb, hsv, alpha) = (state.current_rgb, state.current_hsv, state.current_alpha);
        let hex = state.current_hex.clone();
        if reason != ColorUpdateReason::ColorSpectrumColorChanged {
            app.get_mut(self).spectrum_hsv = HsvColor::from_hsv(hsv, alpha as f32);
        }
        if reason != ColorUpdateReason::ThirdDimensionSliderChanged {
            self.update_third_dimension_slider(app);
        }
        if reason != ColorUpdateReason::AlphaSliderChanged {
            // `UpdateAlphaSlider`.
            app.get_mut(self).alpha_slider_value = alpha * 100.0;
        }
        if app.get(self).text_boxes.is_empty() {
            return;
        }
        let byte = |channel: f64| ((channel * 255.0).round() as u8).to_string();
        let percent = |channel: f64| ((channel * 100.0).round() as i32).to_string();
        if reason != ColorUpdateReason::RgbTextBoxChanged {
            self.set_text(app, ChannelTextBox::Red, &byte(rgb.r));
            self.set_text(app, ChannelTextBox::Green, &byte(rgb.g));
            self.set_text(app, ChannelTextBox::Blue, &byte(rgb.b));
        }
        if reason != ColorUpdateReason::HsvTextBoxChanged {
            self.set_text(
                app,
                ChannelTextBox::Hue,
                &(hsv.h.round() as i32).to_string(),
            );
            self.set_text(app, ChannelTextBox::Saturation, &percent(hsv.s));
            self.set_text(app, ChannelTextBox::Value, &percent(hsv.v));
        }
        if reason != ColorUpdateReason::AlphaTextBoxChanged {
            self.set_text(app, ChannelTextBox::Alpha, &format!("{}%", percent(alpha)));
        }
        if reason != ColorUpdateReason::HexTextBoxChanged {
            self.set_text(app, ChannelTextBox::Hex, &hex);
        }
    }

    /// `UpdateThirdDimensionSlider`'s value: the third component of the colour.
    fn update_third_dimension_slider(self: Handle<Self>, app: &mut App) {
        let hsv = app.get(self).current_hsv;
        let value = match self.widget(app).third_dimension_channel() {
            ColorPickerHsvChannel::Hue => hsv.h,
            ColorPickerHsvChannel::Saturation => hsv.s * 100.0,
            _ => hsv.v * 100.0,
        };
        app.get_mut(self).third_dimension_value = value;
    }

    /// `OnColorSpectrumColorChanged`: the colour the spectrum asks for, which the picker passes
    /// back to it.
    fn spectrum_color_changed(
        self: Handle<Self>,
        app: &mut App,
        args: ColorSpectrumColorChangedEventArgs,
    ) {
        app.get_mut(self).spectrum_hsv = args.hsv_color;
        self.update_color_hsv(
            app,
            args.hsv_color.hsv(),
            ColorUpdateReason::ColorSpectrumColorChanged,
        );
    }

    /// `OnThirdDimensionSliderValueChanged`: the slider's value is the third component.
    fn third_dimension_slider_changed(self: Handle<Self>, app: &mut App, value: f64) {
        app.get_mut(self).third_dimension_value = value;
        let mut hsv = app.get(self).current_hsv;
        match self.widget(app).third_dimension_channel() {
            ColorPickerHsvChannel::Hue => hsv.h = value,
            ColorPickerHsvChannel::Saturation => hsv.s = value / 100.0,
            _ => hsv.v = value / 100.0,
        }
        self.update_color_hsv(app, hsv, ColorUpdateReason::ThirdDimensionSliderChanged);
    }

    /// `OnAlphaSliderValueChanged`.
    fn alpha_slider_changed(self: Handle<Self>, app: &mut App, value: f64) {
        app.get_mut(self).alpha_slider_value = value;
        self.update_color_alpha(app, value / 100.0, ColorUpdateReason::AlphaSliderChanged);
    }

    /// `OnMoreButtonChecked` and `OnMoreButtonUnchecked`.
    fn more_button_checked(self: Handle<Self>, app: &mut App, checked: bool) {
        self.set_state(app, |state| state.text_entry_grid_opened = checked);
    }

    /// `OnColorRepresentationComboBoxSelectionChanged`.
    fn color_representation_changed(self: Handle<Self>, app: &mut App, index: Option<usize>) {
        if let Some(index) = index {
            self.set_state(app, |state| state.color_representation = index);
        }
    }

    /// `OnTextBoxGotFocus` and `OnTextBoxLostFocus`: arriving focus records the text; leaving
    /// puts back that text when what was typed is not valid, and then every text box shows the
    /// colour, clearing what was not applied.
    fn text_box_focus_changed(self: Handle<Self>, app: &mut App, text_box: ChannelTextBox) {
        if !app.contains(self) || !self.mounted(app) {
            return;
        }
        if self.part(app, text_box).focus.has_focus(app) {
            let text = self.text(app, text_box);
            let state = app.get_mut(self);
            state.is_focused_text_box_valid = true;
            state.previous_string = text;
            return;
        }
        if !app.get(self).is_focused_text_box_valid {
            let previous = app.get(self).previous_string.clone();
            self.set_text(app, text_box, &previous);
        }
        self.update_color_controls(app, ColorUpdateReason::ColorPropertyChanged);
        self.set_state(app, |_| {});
    }

    /// The `TextChanging` handlers: valid text changes the colour, other text only marks the
    /// box invalid.
    fn text_changed(self: Handle<Self>, app: &mut App, text_box: ChannelTextBox) {
        match text_box {
            ChannelTextBox::Red | ChannelTextBox::Green | ChannelTextBox::Blue => {
                self.rgb_text_changed(app, text_box)
            }
            ChannelTextBox::Hue | ChannelTextBox::Saturation | ChannelTextBox::Value => {
                self.hsv_text_changed(app, text_box)
            }
            ChannelTextBox::Alpha => self.alpha_text_changed(app),
            ChannelTextBox::Hex => self.hex_text_changed(app),
        }
    }

    /// `OnRgbTextChanging`: a channel from 0 to 255 makes the colour of the three boxes,
    /// within the ranges.
    fn rgb_text_changed(self: Handle<Self>, app: &mut App, text_box: ChannelTextBox) {
        let valid = try_parse_int(&self.text(app, text_box)).is_some_and(|value| value <= 255);
        app.get_mut(self).is_focused_text_box_valid = valid;
        if valid {
            // `GetRgbColorFromTextBoxes`.
            let channel = |text_box| f64::from(wtoi(&self.text(app, text_box))) / 255.0;
            let rgb = Rgb::new(
                channel(ChannelTextBox::Red),
                channel(ChannelTextBox::Green),
                channel(ChannelTextBox::Blue),
            );
            let rgb = self.widget(app).apply_constraints_to_rgb_color(rgb);
            self.update_color_rgb(app, rgb, ColorUpdateReason::RgbTextBoxChanged);
        }
    }

    /// `OnHueTextChanging`, `OnSaturationTextChanging` and `OnValueTextChanging`: a component
    /// within its range makes the colour of the three boxes.
    fn hsv_text_changed(self: Handle<Self>, app: &mut App, text_box: ChannelTextBox) {
        let widget = self.widget(app);
        let (min, max) = match text_box {
            ChannelTextBox::Hue => (widget.min_hue, widget.max_hue),
            ChannelTextBox::Saturation => (widget.min_saturation, widget.max_saturation),
            _ => (widget.min_value, widget.max_value),
        };
        // The source compares as unsigned numbers.
        let valid = try_parse_int(&self.text(app, text_box))
            .is_some_and(|value| value >= min as u32 && value <= max as u32);
        app.get_mut(self).is_focused_text_box_valid = valid;
        if valid {
            // `GetHsvColorFromTextBoxes`.
            let component = |text_box| f64::from(wtoi(&self.text(app, text_box)));
            let hsv = Hsv::new(
                component(ChannelTextBox::Hue),
                component(ChannelTextBox::Saturation) / 100.0,
                component(ChannelTextBox::Value) / 100.0,
            );
            self.update_color_hsv(app, hsv, ColorUpdateReason::HsvTextBoxChanged);
        }
    }

    /// `OnAlphaTextChanging`: text without a final `%` gets one, the caret staying where it
    /// was; a percentage from 0 to 100 is the alpha.
    fn alpha_text_changed(self: Handle<Self>, app: &mut App) {
        let controller = self.part(app, ChannelTextBox::Alpha).controller;
        let text = controller.text_value(app).to_owned();
        if !text.ends_with('%') {
            let cursor = controller.value(app).selection.end();
            controller.set_value(
                app,
                TextEditingValue::new()
                    .text(format!("{text}%"))
                    .selection(TextSelection::collapsed(cursor, TextAffinity::Downstream)),
            );
        }
        let text = controller.text_value(app).to_owned();
        let alpha = try_parse_int(&text[..text.len() - 1]).filter(|value| *value <= 100);
        app.get_mut(self).is_focused_text_box_valid = alpha.is_some();
        if let Some(alpha) = alpha {
            self.update_color_alpha(
                app,
                f64::from(alpha) / 100.0,
                ColorUpdateReason::AlphaTextBoxChanged,
            );
        }
    }

    /// `OnHexTextChanging`: text without a leading `#` gets one, the caret going to the end;
    /// a hex colour, with its alpha while alpha is enabled, is the colour within the ranges.
    fn hex_text_changed(self: Handle<Self>, app: &mut App) {
        let controller = self.part(app, ChannelTextBox::Hex).controller;
        let text = controller.text_value(app).to_owned();
        if !text.starts_with('#') {
            let text = format!("#{text}");
            let end = text.encode_utf16().count() as i32;
            controller.set_value(
                app,
                TextEditingValue::new()
                    .text(text)
                    .selection(TextSelection::collapsed(end, TextAffinity::Downstream)),
            );
        }
        let text = controller.text_value(app).to_owned();
        let parsed = if self.widget(app).is_alpha_enabled {
            hex_to_rgba(&text)
        } else {
            hex_to_rgb(&text).map(|rgb| (rgb, 1.0))
        };
        app.get_mut(self).is_focused_text_box_valid = parsed.is_some();
        if let Some((rgb, alpha)) = parsed {
            let rgb = self.widget(app).apply_constraints_to_rgb_color(rgb);
            self.update_color_rgb(app, rgb, ColorUpdateReason::HexTextBoxChanged);
            self.update_color_alpha(app, alpha, ColorUpdateReason::HexTextBoxChanged);
        }
    }

    /// `OnColorSpectrumSizeChanged`: the spectrum is as tall as it is wide.
    fn spectrum_size_changed(self: Handle<Self>, app: &mut App, previous: Size, size: Size) {
        if app.contains(self) && self.mounted(app) && size.width() != previous.width() {
            self.set_state(app, |state| state.spectrum_height = Some(size.width()));
        }
    }

    /// `OnColorPreviewRectangleGridSizeChanged`.
    fn preview_size_changed(self: Handle<Self>, app: &mut App, size: Size) {
        if app.contains(self) && self.mounted(app) && app.get(self).preview_size != size {
            app.get_mut(self).preview_size = size;
            self.create_checkered_backgrounds(app);
            self.set_state(app, |_| {});
        }
    }

    /// `OnAlphaSliderBackgroundRectangleSizeChanged`.
    fn alpha_background_size_changed(self: Handle<Self>, app: &mut App, size: Size) {
        if app.contains(self) && self.mounted(app) && app.get(self).alpha_background_size != size {
            app.get_mut(self).alpha_background_size = size;
            self.create_checkered_backgrounds(app);
            self.set_state(app, |_| {});
        }
    }

    /// `CreateColorPreviewCheckeredBackground` and `CreateAlphaSliderCheckeredBackground`: the
    /// checkerboards of `CheckerColorBrush` at the size of what they lie behind.
    fn create_checkered_backgrounds(self: Handle<Self>, app: &mut App) {
        let Some(checker) = app.get(self).checker_color else {
            return;
        };
        let checkers = |app: &App, size: Size| {
            let (width, height) = (size.width().round() as u32, size.height().round() as u32);
            image_from_pixel_data(
                app,
                width,
                height,
                checkered_background_pixels(width, height, checker),
            )
        };
        let preview = checkers(app, app.get(self).preview_size);
        let alpha = checkers(app, app.get(self).alpha_background_size);
        let state = app.get_mut(self);
        state.preview_checkers = preview;
        state.alpha_checkers = alpha;
    }

    /// `UpdateVisualState`.
    fn visual_states(self: Handle<Self>, app: &App) -> VisualStates {
        let widget = self.widget(app);
        let state = app.get(self);
        let vertical = widget.orientation == Orientation::Vertical;
        let previous = widget.previous_color.is_some();
        let previous_color = match (widget.is_color_spectrum_visible, previous) {
            (true, true) => PreviousColorVisibility::PreviousColorVisibleVertical,
            (true, false) => PreviousColorVisibility::PreviousColorCollapsedVertical,
            (false, true) => PreviousColorVisibility::PreviousColorVisibleHorizontal,
            (false, false) => PreviousColorVisibility::PreviousColorCollapsedHorizontal,
        };
        VisualStates {
            color_spectrum_visible: widget.is_color_spectrum_visible,
            color_preview_visible: widget.is_color_preview_visible,
            previous_color,
            third_dimension_slider_visible: widget.is_color_slider_visible,
            alpha_slider_visible: widget.is_alpha_enabled && widget.is_alpha_slider_visible,
            // The More button is for the vertical orientation only.
            more_button_visible: widget.is_more_button_visible && vertical,
            text_entry_grid_visible: !widget.is_more_button_visible
                || state.text_entry_grid_opened
                || !vertical,
            color_channel_text_input_visible: widget.is_color_channel_text_input_visible,
            alpha_text_input_visible: widget.is_alpha_enabled && widget.is_alpha_text_input_visible,
            color_representation: if state.color_representation == 0 {
                ColorRepresentation::RgbSelected
            } else {
                ColorRepresentation::HsvSelected
            },
            hex_input_visible: widget.is_hex_input_visible,
            alpha_enabled: widget.is_alpha_enabled,
            orientation: widget.orientation,
        }
    }

    /// The property changes of `OnPropertyChanged`, from `old` to the widget: a colour the
    /// application passed that is not the picker's (`OnColorChanged`), `IsAlphaEnabled`, which
    /// rewrites the hex, a narrowed range, which brings the colour into it, and the components,
    /// which give the third slider another channel. `ColorChanged` of a change made here waits
    /// for the end of the frame.
    fn apply_widget(self: Handle<Self>, app: &mut App, old: &ColorPicker) {
        let widget = self.widget(app).clone();
        app.get_mut(self).updating_widget = true;
        if !same_color(widget.color, app.get(self).color) {
            app.get_mut(self).color = widget.color;
            self.read_color(app, widget.color);
            self.update_color_controls(app, ColorUpdateReason::ColorPropertyChanged);
        }
        if old.is_alpha_enabled != widget.is_alpha_enabled {
            let hex = self.current_hex_value(app);
            app.get_mut(self).current_hex = hex.clone();
            self.set_text(app, ChannelTextBox::Hex, &hex);
        }
        if (old.min_hue, old.max_hue) != (widget.min_hue, widget.max_hue) {
            let mut hsv = app.get(self).current_hsv;
            hsv.h = hsv
                .h
                .min(f64::from(widget.max_hue))
                .max(f64::from(widget.min_hue));
            self.update_color_hsv(app, hsv, ColorUpdateReason::ColorPropertyChanged);
        }
        if (old.min_saturation, old.max_saturation)
            != (widget.min_saturation, widget.max_saturation)
        {
            let mut hsv = app.get(self).current_hsv;
            hsv.s = hsv
                .s
                .min(f64::from(widget.max_saturation) / 100.0)
                .max(f64::from(widget.min_saturation) / 100.0);
            self.update_color_hsv(app, hsv, ColorUpdateReason::ColorPropertyChanged);
        }
        if (old.min_value, old.max_value) != (widget.min_value, widget.max_value) {
            let mut hsv = app.get(self).current_hsv;
            hsv.v = hsv
                .v
                .min(f64::from(widget.max_value) / 100.0)
                .max(f64::from(widget.min_value) / 100.0);
            self.update_color_hsv(app, hsv, ColorUpdateReason::ColorPropertyChanged);
        }
        if old.color_spectrum_components != widget.color_spectrum_components {
            self.update_third_dimension_slider(app);
        }
        app.get_mut(self).updating_widget = false;
    }
}

impl State for ColorPickerState {
    type Widget = ColorPicker;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        for text_box in ChannelTextBox::ALL {
            let controller = TextEditingController::new(app);
            controller.set_text(app, text_box.initial_text());
            let focus = FocusNode::new(app);
            let focus_listener =
                Listener::new(move |app| self.text_box_focus_changed(app, text_box));
            focus.add_listener(app, focus_listener.clone());
            app.get_mut(self).text_boxes.push(TextBoxPart {
                controller,
                focus,
                focus_listener,
            });
        }
        app.get_mut(self).traversal_policy = Some(OrderedTraversalPolicy::new(app));
        // `OnApplyTemplate`: `InitializeColor`, then the ranges as if set after `Color`.
        let widget = self.widget(app).clone();
        app.get_mut(self).updating_widget = true;
        self.read_color(app, widget.color);
        self.set_color_and_update_controls(app, ColorUpdateReason::InitializingColor);
        app.get_mut(self).updating_widget = false;
        self.apply_widget(app, &ColorPicker::new(widget.color, |_, _| {}));
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        // `OnCheckerColorChanged`: `CheckerColorBrush` is `SystemListLowColor` of the theme.
        let checker = ThemeResources::of(app, self.context(app)).system_list_low_color;
        if app.get(self).checker_color != Some(checker) {
            app.get_mut(self).checker_color = Some(checker);
            self.create_checkered_backgrounds(app);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &ColorPicker) {
        self.apply_widget(app, old);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for part in std::mem::take(&mut app.get_mut(self).text_boxes) {
            part.focus.remove_listener(app, &part.focus_listener);
            part.focus.dispose(app);
            app.destroy(part.focus);
            app.get_mut(part.controller).dispose();
            app.destroy(part.controller);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let states = self.visual_states(app);
        let vertical = states.orientation == Orientation::Vertical;
        let mut cells = Vec::new();
        // `ColorSpectrumGrid`.
        cells.push(
            GridCell::new(Margin::new(
                if vertical {
                    [0.0, 0.0, 0.0, 16.0]
                } else {
                    [0.0; 4]
                },
                self.color_spectrum_grid(app, &resources, states),
            ))
            .into_widget(),
        );
        if states.third_dimension_slider_visible {
            // `ThirdDimensionSliderGrid`.
            let (row, column) = if vertical { (1, 0) } else { (0, 1) };
            cells.push(
                GridCell::new(Margin::new(
                    if vertical {
                        [0.0, 0.0, 0.0, 6.0]
                    } else {
                        [0.0, 0.0, 6.0, 0.0]
                    },
                    self.third_dimension_slider_grid(app, &resources, states),
                ))
                .row(row)
                .column(column)
                .into_widget(),
            );
        }
        if states.alpha_slider_visible {
            // `AlphaSliderGrid`.
            let (row, column) = if vertical { (2, 0) } else { (0, 2) };
            cells.push(
                GridCell::new(Margin::new(
                    if vertical {
                        [0.0, 0.0, 0.0, 16.0]
                    } else {
                        [0.0, 0.0, 16.0, 0.0]
                    },
                    self.alpha_slider_grid(app, &resources, states),
                ))
                .row(row)
                .column(column)
                .into_widget(),
            );
        }
        // `MoreEntriesPanel`.
        let (row, column) = if vertical { (3, 0) } else { (0, 3) };
        cells.push(
            GridCell::new(Margin::new(
                if vertical {
                    [0.0, 0.0, 0.0, 12.0]
                } else {
                    [0.0; 4]
                },
                self.more_entries_panel(app, &resources, states),
            ))
            .row(row)
            .column(column)
            .into_widget(),
        );
        let grid = Grid::new()
            .row_definitions([RowDefinition::new(GridLength::AUTO); 4])
            .column_definitions([ColumnDefinition::new(GridLength::AUTO); 4])
            .children(cells);
        // `RootGrid`.
        let constraints = if vertical {
            BoxConstraints::new()
                .min_width(resources.color_picker_vertical_orientation_min_width)
                .max_width(resources.color_picker_vertical_orientation_max_width)
        } else {
            BoxConstraints::new()
                .min_height(resources.color_picker_vertical_orientation_min_height)
                .max_height(resources.color_picker_vertical_orientation_max_height)
                .max_width(10000.0)
        };
        let root = ConstrainedBox::new(constraints).child(Margin::new([0.0, 4.0, 0.0, 4.0], grid));
        let policy = app
            .get(self)
            .traversal_policy
            .expect("an initialized ColorPicker");
        FocusTraversalGroup::new(root)
            .policy(policy.as_policy())
            .into_widget()
    }
}

/// XAML alignment of an element sized to its content: `alignment` within its slot.
fn aligned<K>(alignment: AlignmentGeometry, child: impl IntoWidget<K>) -> WidgetRef {
    Align::new()
        .alignment(alignment)
        .width_factor(1.0)
        .height_factor(1.0)
        .child(child)
        .into_widget()
}

/// The template's `TabIndex` of an element, `document_index` its place among the elements
/// without one, as a focus order: `TabIndex` first, then the order in the template.
fn tab_order<K>(tab_index: i32, document_index: usize, child: impl IntoWidget<K>) -> WidgetRef {
    let order = f64::from(tab_index) * 64.0 + document_index as f64;
    FocusTraversalOrder::new(Rc::new(NumericFocusOrder::new(order)), child).into_widget()
}

impl ColorPickerState {
    /// `ColorSpectrumGrid` (columns *, Auto): the spectrum and the preview.
    fn color_spectrum_grid(
        self: Handle<Self>,
        app: &App,
        resources: &ThemeResources,
        states: VisualStates,
    ) -> WidgetRef {
        let mut cells = Vec::new();
        if states.color_spectrum_visible {
            cells.push(GridCell::new(self.color_spectrum(resources, app)).into_widget());
        }
        if states.color_preview_visible {
            let vertical = states.orientation == Orientation::Vertical;
            let preview = self.color_preview_rectangle_grid(app, resources, states);
            let cell = if states.color_spectrum_visible {
                // 44 wide, after the spectrum.
                let margin = if vertical {
                    [12.0, 0.0, 0.0, 0.0]
                } else {
                    [12.0, 0.0, 16.0, 0.0]
                };
                GridCell::new(Margin::new(
                    margin,
                    SizedBox::new().width(PREVIEW_SIZE).child(preview),
                ))
                .column(1)
            } else {
                // `ColorSpectrumCollapsed`: 44 tall, across both columns.
                GridCell::new(SizedBox::new().height(PREVIEW_SIZE).child(preview)).column_span(2)
            };
            cells.push(cell.into_widget());
        }
        Grid::new()
            .column_definitions([
                ColumnDefinition::new(GridLength::STAR),
                ColumnDefinition::new(GridLength::AUTO),
            ])
            .children(cells)
            .into_widget()
    }

    /// `ColorSpectrum`: 256 to 336 square, its height following its width.
    fn color_spectrum(self: Handle<Self>, r: &ThemeResources, app: &App) -> WidgetRef {
        let widget = self.widget(app);
        let height = app
            .get(self)
            .spectrum_height
            .map(|height| height.clamp(SPECTRUM_MIN_SIZE, SPECTRUM_MAX_SIZE));
        let constraints = BoxConstraints::new()
            .min_width(SPECTRUM_MIN_SIZE)
            .max_width(SPECTRUM_MAX_SIZE)
            .min_height(height.unwrap_or(SPECTRUM_MIN_SIZE))
            .max_height(height.unwrap_or(SPECTRUM_MAX_SIZE));
        let spectrum = ColorSpectrum::new(app.get(self).spectrum_hsv, move |app, args| {
            self.spectrum_color_changed(app, args)
        })
        .min_hue(widget.min_hue)
        .max_hue(widget.max_hue)
        .min_saturation(widget.min_saturation)
        .max_saturation(widget.max_saturation)
        .min_value(widget.min_value)
        .max_value(widget.max_value)
        .shape(widget.color_spectrum_shape)
        .components(widget.color_spectrum_components)
        .corner_radius(widget.corner_radius.unwrap_or(r.control_corner_radius[0]))
        .is_enabled(widget.is_enabled);
        tab_order(
            0,
            0,
            SizeObserver::new(
                ConstrainedBox::new(constraints).child(spectrum),
                Rc::new(move |app, previous, size| self.spectrum_size_changed(app, previous, size)),
            ),
        )
    }

    /// `ColorPreviewRectangleGrid` (two rows, two columns): the checkered background,
    /// `ColorPreviewRectangle`, `PreviousColorRectangle` and `BorderRectangle`.
    fn color_preview_rectangle_grid(
        self: Handle<Self>,
        app: &App,
        resources: &ThemeResources,
        states: VisualStates,
    ) -> WidgetRef {
        let state = app.get(self);
        let radius = self
            .widget(app)
            .corner_radius
            .unwrap_or(resources.control_corner_radius[0]);
        let rectangle = |fill: Option<ShapeFill>, stroke: Option<Color>| {
            CustomPaint::new()
                .painter(RectanglePainter::new(fill, stroke, radius))
                .into_widget()
        };
        let mut cells = vec![
            GridCell::new(rectangle(
                state.preview_checkers.clone().map(ShapeFill::Image),
                None,
            ))
            .row_span(2)
            .column_span(2)
            .into_widget(),
        ];
        // `ColorPreviewRectangle`, a half when the previous colour shows.
        let preview = GridCell::new(rectangle(Some(ShapeFill::Solid(state.color)), None));
        let preview = match states.previous_color {
            PreviousColorVisibility::PreviousColorVisibleVertical => {
                preview.row_span(1).column_span(2)
            }
            PreviousColorVisibility::PreviousColorVisibleHorizontal => {
                preview.row_span(2).column_span(1)
            }
            _ => preview.row_span(2).column_span(2),
        };
        cells.push(preview.into_widget());
        if let Some(previous) = self.widget(app).previous_color {
            let previous_cell = GridCell::new(rectangle(Some(ShapeFill::Solid(previous)), None));
            let previous_cell = match states.previous_color {
                PreviousColorVisibility::PreviousColorVisibleVertical => {
                    previous_cell.row(1).column_span(2)
                }
                _ => previous_cell.row(0).column(1).row_span(2),
            };
            cells.push(previous_cell.into_widget());
        }
        cells.push(
            GridCell::new(rectangle(None, Some(resources.color_picker_border_brush)))
                .row_span(2)
                .column_span(2)
                .into_widget(),
        );
        SizeObserver::new(
            Grid::new()
                .row_definitions([RowDefinition::new(GridLength::STAR); 2])
                .column_definitions([ColumnDefinition::new(GridLength::STAR); 2])
                .children(cells),
            Rc::new(move |app, _, size| self.preview_size_changed(app, size)),
        )
        .into_widget()
    }

    /// `ThirdDimensionSliderGrid`: `ThirdDimensionBackgroundRectangle` under
    /// `ThirdDimensionSlider`.
    fn third_dimension_slider_grid(
        self: Handle<Self>,
        app: &App,
        _resources: &ThemeResources,
        states: VisualStates,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let vertical_slider = states.orientation == Orientation::Horizontal;
        let (minimum, maximum) = widget.third_dimension_range();
        let background = CustomPaint::new()
            .painter(
                RectanglePainter::new(
                    Some(ShapeFill::Gradient {
                        stops: self.third_dimension_gradient(app),
                        rotated: vertical_slider,
                    }),
                    None,
                    _resources.color_picker_slider_corner_radius[0],
                )
                .slider_background(vertical_slider),
            )
            .size(slider_background_size(vertical_slider));
        let slider =
            ColorPickerSlider::new(app.get(self).third_dimension_value, move |app, value| {
                self.third_dimension_slider_changed(app, value)
            })
            .minimum(f64::from(minimum))
            .maximum(f64::from(maximum))
            .color_channel(widget.third_dimension_channel())
            .orientation(slider_orientation(vertical_slider))
            .is_thumb_tool_tip_enabled(false)
            .is_enabled(widget.is_enabled);
        Grid::new()
            .children([
                GridCell::new(background).into_widget(),
                GridCell::new(tab_order(1, 0, slider)).into_widget(),
            ])
            .into_widget()
    }

    /// The stops of `ThirdDimensionSliderGradientBrush`: the colour from the range's minimum to
    /// its maximum of the third component, at full value for saturation; for hue, the hues of
    /// the range's ends and the primary hues between them.
    fn third_dimension_gradient(self: Handle<Self>, app: &App) -> Vec<(f64, Color)> {
        let widget = self.widget(app);
        let hsv = app.get(self).current_hsv;
        let stop = |offset: f64, hsv: Hsv| (offset, color_from_rgba(hsv_to_rgb(hsv), 1.0));
        match widget.third_dimension_channel() {
            ColorPickerHsvChannel::Saturation => {
                let min = widget.min_saturation;
                let max = widget.max_saturation.max(min);
                vec![
                    stop(0.0, Hsv::new(hsv.h, f64::from(min) / 100.0, 1.0)),
                    stop(1.0, Hsv::new(hsv.h, f64::from(max) / 100.0, 1.0)),
                ]
            }
            ColorPickerHsvChannel::Hue => {
                let min = widget.min_hue;
                let max = widget.max_hue.max(min);
                let (min_offset, max_offset) = (f64::from(min) / 359.0, f64::from(max) / 359.0);
                let mut stops = vec![stop(0.0, Hsv::new(f64::from(min), 1.0, 1.0))];
                for sextant in 1..=5 {
                    let offset = f64::from(sextant) / 6.0;
                    if min_offset < offset && max_offset > offset {
                        stops.push(stop(
                            (offset - min_offset) / (max_offset - min_offset),
                            Hsv::new(60.0 * f64::from(sextant), 1.0, 1.0),
                        ));
                    }
                }
                stops.push(stop(1.0, Hsv::new(f64::from(max), 1.0, 1.0)));
                stops
            }
            _ => {
                let min = widget.min_value;
                let max = widget.max_value.max(min);
                vec![
                    stop(0.0, Hsv::new(hsv.h, hsv.s, f64::from(min) / 100.0)),
                    stop(1.0, Hsv::new(hsv.h, hsv.s, f64::from(max) / 100.0)),
                ]
            }
        }
    }

    /// `AlphaSliderGrid`: `AlphaSliderCheckeredBackgroundRectangle` and
    /// `AlphaSliderBackgroundRectangle`, the colour from transparent to opaque, under
    /// `AlphaSlider`.
    fn alpha_slider_grid(
        self: Handle<Self>,
        app: &App,
        _resources: &ThemeResources,
        states: VisualStates,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let state = app.get(self);
        let vertical_slider = states.orientation == Orientation::Horizontal;
        let size = slider_background_size(vertical_slider);
        let checkers = CustomPaint::new()
            .painter(
                RectanglePainter::new(
                    state.alpha_checkers.clone().map(ShapeFill::Image),
                    None,
                    widget
                        .corner_radius
                        .unwrap_or(_resources.control_corner_radius[0]),
                )
                .slider_background(vertical_slider),
            )
            .size(size);
        let color = color_from_rgba(hsv_to_rgb(state.current_hsv), 1.0);
        let gradient = CustomPaint::new()
            .painter(
                RectanglePainter::new(
                    Some(ShapeFill::Gradient {
                        stops: vec![(0.0, color.with_alpha(0)), (1.0, color)],
                        rotated: vertical_slider,
                    }),
                    None,
                    _resources.color_picker_slider_corner_radius[0],
                )
                .slider_background(vertical_slider),
            )
            .size(size);
        let gradient = SizeObserver::new(
            gradient,
            Rc::new(move |app, _, size| self.alpha_background_size_changed(app, size)),
        );
        let slider = ColorPickerSlider::new(state.alpha_slider_value, move |app, value| {
            self.alpha_slider_changed(app, value)
        })
        .minimum(0.0)
        .maximum(100.0)
        .color_channel(ColorPickerHsvChannel::Alpha)
        .orientation(slider_orientation(vertical_slider))
        .is_thumb_tool_tip_enabled(false)
        .is_enabled(widget.is_enabled);
        Grid::new()
            .children([
                GridCell::new(checkers).into_widget(),
                GridCell::new(gradient).into_widget(),
                GridCell::new(tab_order(2, 0, slider)).into_widget(),
            ])
            .into_widget()
    }

    /// `MoreEntriesPanel`, a vertical `StackPanel`: `MoreButton` at the right over
    /// `TextEntryGrid`.
    fn more_entries_panel(
        self: Handle<Self>,
        app: &App,
        resources: &ThemeResources,
        states: VisualStates,
    ) -> WidgetRef {
        let mut cells = Vec::new();
        if states.more_button_visible {
            cells.push(
                GridCell::new(aligned(
                    AlignmentGeometry::TOP_RIGHT,
                    tab_order(3, 0, self.more_button(app, states)),
                ))
                .into_widget(),
            );
        }
        if states.text_entry_grid_visible {
            cells.push(
                GridCell::new(self.text_entry_grid(app, resources, states))
                    .row(cells.len())
                    .into_widget(),
            );
        }
        stack_panel(cells)
    }

    /// `MoreButton`: a toggle button without fill or border, its label and chevron at the
    /// right, the chevron up while the entries show.
    fn more_button(self: Handle<Self>, app: &App, states: VisualStates) -> WidgetRef {
        let opened = app.get(self).text_entry_grid_opened;
        let label = if opened { "Less" } else { "More" };
        let glyph = if states.text_entry_grid_visible {
            FluentSymbol::ChevronUp
        } else {
            FluentSymbol::ChevronDown
        };
        let content = Margin::new(
            [0.0, 5.0, 0.0, 7.0],
            Row::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .children([
                    // `MoreButtonLabel`.
                    Text::new(label).into_widget(),
                    // `MoreGlyph`.
                    Margin::new(
                        [8.0, 0.0, 0.0, 0.0],
                        FontIcon::symbol(glyph).font_size(MORE_GLYPH_FONT_SIZE),
                    )
                    .into_widget(),
                ]),
        );
        let button = ToggleButton::new(Some(opened), move |app, checked| {
            self.more_button_checked(app, checked == Some(true))
        })
        .content(content)
        .is_enabled(self.widget(app).is_enabled)
        .template(|app, context, is_checked, content, states| {
            let resources = ThemeResources::of(app, context);
            toggle_button_presenter(
                &resources,
                more_button_brushes(&resources, is_checked == Some(true), states.common),
                ToggleButtonSetters {
                    padding: Some([0.0; 4]),
                    horizontal_content_alignment: AlignmentGeometry::CENTER_RIGHT,
                },
                content.unwrap_or_else(|| SizedBox::shrink().into_widget()),
                states,
            )
        });
        ConstrainedBox::new(
            BoxConstraints::new()
                .min_width(MORE_BUTTON_MIN_WIDTH)
                .min_height(MORE_BUTTON_MIN_HEIGHT),
        )
        .child(button)
        .into_widget()
    }

    /// `TextEntryGrid` (columns *, 5, *; three Auto rows): `ColorRepresentationComboBox`,
    /// `HexTextBox`, and `ColorTextInputPanels` below them.
    fn text_entry_grid(
        self: Handle<Self>,
        app: &App,
        resources: &ThemeResources,
        states: VisualStates,
    ) -> WidgetRef {
        let vertical = states.orientation == Orientation::Vertical;
        let mut cells = Vec::new();
        if states.color_channel_text_input_visible {
            // `ColorRepresentationComboBox`: `ComboBox` is left-aligned by its style.
            let index = app.get(self).color_representation;
            let combo_box = ComboBox::new(
                vec![
                    ComboBoxItem::text("rgb", "RGB"),
                    ComboBoxItem::text("hsv", "HSV"),
                ],
                Some(index),
                move |app, args| self.color_representation_changed(app, args.selected_index()),
            )
            .is_enabled(self.widget(app).is_enabled);
            cells.push(
                GridCell::new(aligned(
                    AlignmentGeometry::TOP_LEFT,
                    tab_order(4, 0, SizedBox::new().width(TEXT_BOX_WIDTH).child(combo_box)),
                ))
                .row(if vertical { 0 } else { 1 })
                .into_widget(),
            );
        }
        // `ColorTextInputPanels`.
        cells.push(
            GridCell::new(self.color_text_input_panels(app, resources, states))
                .row(if vertical { 1 } else { 2 })
                .column_span(3)
                .into_widget(),
        );
        if states.hex_input_visible {
            // `HexTextBox`: at the right of column 2; at the left of column 0 without the
            // channel entries (`ColorChannelTextInputCollapsed`); and in the `Horizontal`
            // orientation 120 wide, at the left above the entries, across the columns.
            let cell = if !vertical {
                GridCell::new(Margin::new(
                    [0.0, 0.0, 0.0, 12.0],
                    self.hex_text_box(app, states, TEXT_BOX_WIDTH, AlignmentGeometry::TOP_LEFT),
                ))
                .column_span(3)
            } else if states.color_channel_text_input_visible {
                GridCell::new(self.hex_text_box(
                    app,
                    states,
                    HEX_TEXT_BOX_WIDTH,
                    AlignmentGeometry::TOP_RIGHT,
                ))
                .column(2)
            } else {
                GridCell::new(self.hex_text_box(
                    app,
                    states,
                    HEX_TEXT_BOX_WIDTH,
                    AlignmentGeometry::TOP_LEFT,
                ))
            };
            cells.push(cell.into_widget());
        }
        Grid::new()
            .column_definitions([
                ColumnDefinition::new(GridLength::STAR),
                ColumnDefinition::new(GridLength::pixel(5.0)),
                ColumnDefinition::new(GridLength::STAR),
            ])
            .row_definitions([RowDefinition::new(GridLength::AUTO); 3])
            .children(cells)
            .into_widget()
    }

    /// `HexTextBox`, `width` wide at `alignment`; 9 characters long with alpha enabled.
    fn hex_text_box(
        self: Handle<Self>,
        app: &App,
        states: VisualStates,
        width: f64,
        alignment: AlignmentGeometry,
    ) -> WidgetRef {
        let max_length = HEX_MAX_LENGTH[usize::from(states.alpha_enabled)];
        let tab_index = if states.orientation == Orientation::Vertical {
            DEFAULT_TAB_INDEX
        } else {
            3
        };
        aligned(
            alignment,
            tab_order(
                tab_index,
                ChannelTextBox::Hex.index(),
                SizedBox::new().width(width).child(self.text_box(
                    app,
                    ChannelTextBox::Hex,
                    max_length,
                )),
            ),
        )
    }

    /// The template's `TextBox` for `text_box`.
    fn text_box(
        self: Handle<Self>,
        app: &App,
        text_box: ChannelTextBox,
        max_length: i32,
    ) -> TextBox {
        let part = self.part(app, text_box);
        TextBox::new(part.controller.as_controller())
            .focus_node(part.focus)
            .max_length(max_length)
            .is_enabled(self.widget(app).is_enabled)
            .text_changed(move |app, _| self.text_changed(app, text_box))
    }

    /// `ColorTextInputPanels`, a vertical `StackPanel`: `ColorChannelTextInputPanel`, with
    /// `RgbPanel` or `HsvPanel`, and `AlphaPanel`.
    fn color_text_input_panels(
        self: Handle<Self>,
        app: &App,
        resources: &ThemeResources,
        states: VisualStates,
    ) -> WidgetRef {
        let mut cells = Vec::new();
        if states.color_channel_text_input_visible {
            let channels = match states.color_representation {
                ColorRepresentation::RgbSelected => [
                    ChannelTextBox::Red,
                    ChannelTextBox::Green,
                    ChannelTextBox::Blue,
                ],
                ColorRepresentation::HsvSelected => [
                    ChannelTextBox::Hue,
                    ChannelTextBox::Saturation,
                    ChannelTextBox::Value,
                ],
            };
            let rows = channels
                .iter()
                .enumerate()
                .flat_map(|(row, &text_box)| {
                    self.channel_row(app, resources, states, text_box, CHANNEL_MAX_LENGTH, row)
                })
                .collect::<Vec<_>>();
            cells.push(
                GridCell::new(channel_panel(
                    resources,
                    states,
                    [RowDefinition::new(GridLength::STAR); 3],
                    rows,
                ))
                .into_widget(),
            );
        }
        if states.alpha_text_input_visible {
            // `AlphaPanel`.
            let row = self.channel_row(
                app,
                resources,
                states,
                ChannelTextBox::Alpha,
                ALPHA_MAX_LENGTH,
                0,
            );
            cells.push(
                GridCell::new(Margin::new(
                    [0.0, 12.0, 0.0, 0.0],
                    channel_panel(
                        resources,
                        states,
                        [RowDefinition::new(GridLength::STAR)],
                        row,
                    ),
                ))
                .row(cells.len())
                .into_widget(),
            );
        }
        stack_panel(cells)
    }

    /// One row of a channel panel: the text box, 120 wide, and its label, each 12 below the row
    /// above but for opacity, whose panel has that margin.
    fn channel_row(
        self: Handle<Self>,
        app: &App,
        resources: &ThemeResources,
        _states: VisualStates,
        text_box: ChannelTextBox,
        max_length: i32,
        row: usize,
    ) -> Vec<WidgetRef> {
        let margin = if text_box == ChannelTextBox::Alpha {
            [0.0; 4]
        } else {
            [0.0, 12.0, 0.0, 0.0]
        };
        let text = control_text_style(
            resources,
            resources.control_content_theme_font_size,
            FontWeight::NORMAL,
            resources.text_fill_color_primary,
        );
        vec![
            GridCell::new(Margin::new(
                margin,
                tab_order(
                    DEFAULT_TAB_INDEX,
                    text_box.index(),
                    SizedBox::new()
                        .width(TEXT_BOX_WIDTH)
                        .child(self.text_box(app, text_box, max_length)),
                ),
            ))
            .row(row)
            .into_widget(),
            GridCell::new(Margin::new(
                margin,
                aligned(
                    AlignmentGeometry::CENTER_LEFT,
                    DefaultTextStyle::new(text, Text::new(text_box.label())),
                ),
            ))
            .row(row)
            .column(2)
            .into_widget(),
        ]
    }
}

/// `RgbPanel`, `HsvPanel` or `AlphaPanel`: columns Auto, 8 and the label column, which the
/// `Horizontal` orientation makes 122 wide.
fn channel_panel<const N: usize>(
    r: &ThemeResources,
    states: VisualStates,
    rows: [RowDefinition; N],
    children: Vec<WidgetRef>,
) -> WidgetRef {
    let label_column = if states.orientation == Orientation::Vertical {
        GridLength::STAR
    } else {
        GridLength::pixel(r.color_picker_text_input_horizontal_orientation_margin)
    };
    Grid::new()
        .column_definitions([
            ColumnDefinition::new(GridLength::AUTO),
            ColumnDefinition::new(GridLength::pixel(8.0)),
            ColumnDefinition::new(label_column),
        ])
        .row_definitions(rows)
        .children(children)
        .into_widget()
}

/// A vertical `StackPanel` of `cells`, one in each row: a grid of Auto rows in one star column,
/// which stretches each child across as the panel does.
fn stack_panel(cells: Vec<WidgetRef>) -> WidgetRef {
    let rows = cells.len();
    Grid::new()
        .row_definitions(vec![RowDefinition::new(GridLength::AUTO); rows])
        .children(cells)
        .into_widget()
}

/// The orientation of the sliders: vertical in the picker's `Horizontal` orientation.
fn slider_orientation(vertical: bool) -> Orientation {
    if vertical {
        Orientation::Vertical
    } else {
        Orientation::Horizontal
    }
}

/// The size a slider background rectangle asks for: 12 across the slider.
fn slider_background_size(vertical: bool) -> Size {
    if vertical {
        Size::new(SLIDER_BACKGROUND_SIZE, 0.0)
    } else {
        Size::new(0.0, SLIDER_BACKGROUND_SIZE)
    }
}

/// The `ToggleButton.Resources` of `MoreButton`: no fill or border in any state, and the
/// foreground `SystemBaseHighColor` at rest, `SystemBaseMediumColor` under the pointer and
/// `SystemBaseMediumLowColor` pressed or disabled, checked or not.
fn more_button_brushes(
    resources: &ThemeResources,
    is_checked: bool,
    state: CommonState,
) -> ToggleButtonBrushes {
    let r = &resources;
    let transparent = Color::from_argb(0, 0, 0, 0);
    let foreground = match state {
        CommonState::Normal => r.system_base_high_color,
        CommonState::PointerOver => r.system_base_medium_color,
        CommonState::Pressed | CommonState::Disabled => r.system_base_medium_low_color,
    };
    let base = crate::toggle_button_brushes(resources, Some(is_checked), state);
    ToggleButtonBrushes {
        background: transparent,
        border: Brush::Solid(transparent),
        foreground,
        background_sizing: base.background_sizing,
    }
}

/// What a XAML `Rectangle` of the template is filled with.
#[derive(Clone, Debug, PartialEq)]
enum ShapeFill {
    /// `SolidColorBrush`.
    Solid(Color),
    /// `LinearGradientBrush` with its default `StartPoint` (0, 0) and `EndPoint` (1, 1) of the
    /// bounding box, `rotated` by 180 degrees (`RotateTransform Angle="180"`) for a vertical
    /// slider.
    Gradient {
        stops: Vec<(f64, Color)>,
        rotated: bool,
    },
    /// `ImageBrush`, stretched over the rectangle (`Stretch="Fill"`).
    Image(Image),
}

/// A XAML `Rectangle` filling its cell, or 12 across a slider and centred: an optional `Fill`,
/// an optional `ColorPickerBorderStyle` stroke inside the bounds, and `RadiusX` and `RadiusY`.
#[derive(Clone, Debug, PartialEq)]
struct RectanglePainter {
    fill: Option<ShapeFill>,
    stroke: Option<Color>,
    radius: f64,
    /// `Some(vertical)` for a slider's background: 12 tall and centred (or 12 wide when
    /// vertical).
    slider_background: Option<bool>,
}

impl RectanglePainter {
    fn new(fill: Option<ShapeFill>, stroke: Option<Color>, radius: f64) -> RectanglePainter {
        RectanglePainter {
            fill,
            stroke,
            radius,
            slider_background: None,
        }
    }

    /// The rectangle is a slider's background, 12 across a slider that is `vertical` or not.
    fn slider_background(mut self, vertical: bool) -> RectanglePainter {
        self.slider_background = Some(vertical);
        self
    }

    /// The rectangle's bounds in a cell of `size`.
    fn bounds(&self, size: Size) -> Rect {
        match self.slider_background {
            None => Rect::from_ltwh(0.0, 0.0, size.width(), size.height()),
            Some(false) => Rect::from_ltwh(
                0.0,
                (size.height() - SLIDER_BACKGROUND_SIZE) / 2.0,
                size.width(),
                SLIDER_BACKGROUND_SIZE,
            ),
            Some(true) => Rect::from_ltwh(
                (size.width() - SLIDER_BACKGROUND_SIZE) / 2.0,
                0.0,
                SLIDER_BACKGROUND_SIZE,
                size.height(),
            ),
        }
    }

    /// The paint of `fill` over `bounds`.
    fn fill_paint(fill: &ShapeFill, bounds: Rect) -> Paint {
        match fill {
            ShapeFill::Solid(color) => Paint::from_color((*color).into()),
            ShapeFill::Gradient { stops, rotated } => {
                let (start, end) = if *rotated {
                    (bounds.bottom_right(), bounds.top_left())
                } else {
                    (bounds.top_left(), bounds.bottom_right())
                };
                Paint::from_shader(Shader::Linear {
                    start: Point::new(start.dx() as f32, start.dy() as f32),
                    end: Point::new(end.dx() as f32, end.dy() as f32),
                    stops: stops
                        .iter()
                        .map(|(offset, color)| GradientStop {
                            offset: *offset as f32,
                            color: (*color).into(),
                        })
                        .collect(),
                    spread: SpreadMode::Pad,
                    local: Matrix::IDENTITY,
                })
            }
            ShapeFill::Image(image) => {
                let [width, height] = image.size();
                let local = Matrix::from_affine(
                    (bounds.width() / f64::from(width)) as f32,
                    0.0,
                    0.0,
                    (bounds.height() / f64::from(height)) as f32,
                    bounds.left as f32,
                    bounds.top as f32,
                );
                Paint::from_shader(Shader::Image {
                    image: image.clone(),
                    sampling: Default::default(),
                    local,
                })
            }
        }
    }
}

/// `CreateRoundedRect`'s limit: a radius is no more than half a side.
fn rectangle_radius(radius: f64, bounds: Rect) -> f32 {
    radius
        .abs()
        .min(bounds.width() / 2.0)
        .min(bounds.height() / 2.0) as f32
}

impl CustomPainter for RectanglePainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let bounds = self.bounds(size);
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        // A stroked shape fills and strokes its outline inset by half the stroke.
        let outline = if self.stroke.is_some() {
            bounds.deflate(COLOR_PICKER_BORDER_THICKNESS / 2.0)
        } else {
            bounds
        };
        if let Some(fill) = &self.fill {
            let paint = RectanglePainter::fill_paint(fill, outline);
            canvas.draw_rrect(outline, rectangle_radius(self.radius, outline), &paint);
        }
        if let Some(stroke) = self.stroke {
            let mut paint = Paint::from_color(stroke.into());
            paint.style = PaintStyle::Stroke(Stroke::new(COLOR_PICKER_BORDER_THICKNESS as f32));
            canvas.draw_rrect(outline, rectangle_radius(self.radius, outline), &paint);
        }
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
