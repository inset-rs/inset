//! ColorSpectrum.xaml, ColorSpectrum.cpp and SpectrumBrush.cpp: a square or circle of colours
//! spanning two HSV components, with a selection ellipse on the selected colour.
//!
//! Template, back to front: `LayoutRoot` (a `Grid` filling the control) centres `SizingGrid`, a
//! square as wide as the smaller side of `LayoutRoot`, clipped to that square. It holds
//! `SpectrumRectangle` or `SpectrumEllipse` (by `ShapeSelected`), filled with the spectrum
//! images, and their `SpectrumOverlayRectangle` and `SpectrumOverlayEllipse`, which the source
//! never fills; `InputTarget`, a transparent canvas that takes the pointer and holds
//! `SelectionEllipsePanel` (16 square, 48 in `PressedLarge`) centred on the selected colour, with
//! `FocusEllipse` (2 larger on every side, shown in `Focused`) and `SelectionEllipse`, which owns
//! `ColorNameToolTip`; and `RectangleBorder` or `EllipseBorder`, stroked with
//! `ColorPickerBorderBrush`.
//!
//! The images (`CreateBitmapsAndColorMap`) are made when the control's size, `Shape`,
//! `Components` or the range of a component it lays out changes: two images, at the minimum and
//! maximum of the third component, or six, at the six primary hues when hue is the third. The
//! spectrum shows the two the third component lies between, the second over the first at the
//! opacity of how far between (`SpectrumBrush`'s cross-fade).
//!
//! The application owns the colour: `hsv_color` is what it passes, and `color_changed` asks for
//! another. The colour the spectrum asks for is what it reads as its own until the application
//! next builds it; if the application passed another by then, the spectrum shows that one.

use crate::{
    ColorPickerHsvChannel, ColorSpectrumComponents, ColorSpectrumShape, FocusState, Hsv,
    IncrementAmount, IncrementDirection, InputDevice, SizeObserver, ThemeResources, ToolTip,
    ToolTipService, argb, color_display_name, color_from_rgba, hsv_to_rgb, image_from_pixel_data,
    increment_color_channel, rgb_from_color, rgb_to_hsv, same_color,
};
use inset_embedder::valo::{Matrix, Paint, PaintStyle, Shader, Stroke};
use inset_embedder::{Canvas, Color, Image, Offset, PointerDeviceKind, Rect, Size};
use inset_foundation::{App, Handle, Listenable, Listener};
use inset_gestures::{EagerGestureRecognizer, PointerDownEvent};
use inset_painting::draw_oval;
use inset_rendering::{CustomPainter, HitTestBehavior};
use inset_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey};
use inset_widgets::Listener as PointerListener;
use inset_widgets::*;
use std::{any::TypeId, f64::consts::PI, fmt, rc::Rc};

/// XAML `Vector4` of `ColorSpectrum.HsvColor`: hue from 0 to 360, and saturation, value and
/// alpha from 0 to 1, as `x`, `y`, `z` and `w`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HsvColor {
    /// `x`: the hue in degrees.
    pub hue: f32,
    /// `y`.
    pub saturation: f32,
    /// `z`.
    pub value: f32,
    /// `w`.
    pub alpha: f32,
}

impl HsvColor {
    /// `HsvColor`'s default, `{ 0, 0, 1, 1 }`: white.
    pub const WHITE: HsvColor = HsvColor {
        hue: 0.0,
        saturation: 0.0,
        value: 1.0,
        alpha: 1.0,
    };

    /// The `HsvColor` of `color`, as `OnColorChanged` sets it when `Color` is set.
    pub fn from_color(color: Color) -> HsvColor {
        let hsv = rgb_to_hsv(rgb_from_color(color));
        HsvColor {
            hue: hsv.h as f32,
            saturation: hsv.s as f32,
            value: hsv.v as f32,
            alpha: (f64::from(argb(color).a) / 255.0) as f32,
        }
    }

    /// The `Color` of this colour, as `SetColor` sets it when `HsvColor` is set.
    pub fn to_color(self) -> Color {
        color_from_rgba(hsv_to_rgb(self.hsv()), f64::from(self.alpha))
    }

    /// Hue, saturation and value as `Hsv`.
    pub(crate) fn hsv(self) -> Hsv {
        Hsv::new(
            f64::from(self.hue),
            f64::from(self.saturation),
            f64::from(self.value),
        )
    }

    /// `hsv` with `alpha`.
    pub(crate) fn from_hsv(hsv: Hsv, alpha: f32) -> HsvColor {
        HsvColor {
            hue: hsv.h as f32,
            saturation: hsv.s as f32,
            value: hsv.v as f32,
            alpha,
        }
    }
}

impl Default for HsvColor {
    fn default() -> Self {
        HsvColor::WHITE
    }
}

/// XAML `ColorChangedEventArgs` of `ColorSpectrum.ColorChanged`, with the `HsvColor` the
/// spectrum asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorSpectrumColorChangedEventArgs {
    /// `OldColor`.
    pub old_color: Color,
    /// `NewColor`.
    pub new_color: Color,
    /// `HsvColor` after the change, which the application stores and passes back.
    pub hsv_color: HsvColor,
}

/// Receives XAML `ColorSpectrum.ColorChanged`.
pub type ColorSpectrumColorChangedHandler =
    Rc<dyn Fn(&mut App, ColorSpectrumColorChangedEventArgs)>;

/// The `CommonStates` default `Width` and `Height` of `SelectionEllipsePanel`.
pub const SELECTION_ELLIPSE_SIZE: f64 = 16.0;
/// `SelectionEllipsePanel`'s `Width` and `Height` in `PressedLarge`.
pub const SELECTION_ELLIPSE_LARGE_SIZE: f64 = 48.0;
/// `FocusEllipse`'s `Margin="-2"` outside the panel.
const FOCUS_ELLIPSE_OUTSET: f64 = 2.0;
/// `StrokeThickness` of `FocusEllipse` and `SelectionEllipse`.
const ELLIPSE_STROKE_THICKNESS: f64 = 2.0;
/// `ColorPickerBorderStyle`'s `StrokeThickness`, of `RectangleBorder` and `EllipseBorder`.
pub const COLOR_PICKER_BORDER_THICKNESS: f64 = 2.0;
/// `SelectionEllipse`'s `Opacity` in `PointerOver`.
const POINTER_OVER_SELECTION_OPACITY: f64 = 0.8;
/// `ColorNameToolTip`'s `VerticalOffset`.
const COLOR_NAME_TOOL_TIP_OFFSET: f64 = 20.0;

/// XAML `ColorSpectrum` with the template of `ColorSpectrum.xaml`.
#[derive(Clone)]
pub struct ColorSpectrum {
    /// `HsvColor`: the selected colour; `Color` is `HsvColor::to_color` of it.
    pub hsv_color: HsvColor,

    /// `ColorChanged`: asks the application for another `HsvColor`; the application owns
    /// `hsv_color`.
    pub color_changed: ColorSpectrumColorChangedHandler,

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

    /// `Shape`.
    pub shape: ColorSpectrumShape,

    /// `Components`.
    pub components: ColorSpectrumComponents,

    /// `CornerRadius` of the box: its top left is the rectangles' `RadiusX`, its bottom right
    /// their `RadiusY`, the same here; 0 by default, as the style sets none.
    pub corner_radius: f64,

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

impl ColorSpectrum {
    /// Creates a spectrum selecting `hsv_color`, which asks for others through `color_changed`.
    pub fn new(
        hsv_color: HsvColor,
        color_changed: impl Fn(&mut App, ColorSpectrumColorChangedEventArgs) + 'static,
    ) -> ColorSpectrum {
        ColorSpectrum {
            hsv_color,
            color_changed: Rc::new(color_changed),
            min_hue: 0,
            max_hue: 359,
            min_saturation: 0,
            max_saturation: 100,
            min_value: 0,
            max_value: 100,
            shape: ColorSpectrumShape::Box,
            components: ColorSpectrumComponents::HueSaturation,
            corner_radius: 0.0,
            is_enabled: true,
            key: None,
        }
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

    /// Sets Shape.
    pub fn shape(mut self, value: ColorSpectrumShape) -> Self {
        self.shape = value;
        self
    }

    /// Sets Components.
    pub fn components(mut self, value: ColorSpectrumComponents) -> Self {
        self.components = value;
        self
    }

    /// Sets CornerRadius.
    pub fn corner_radius(mut self, value: f64) -> Self {
        self.corner_radius = value;
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

    /// The component ranges, as `CreateBitmapsAndColorMap` reads them.
    fn ranges(&self) -> Ranges {
        Ranges {
            min_hue: self.min_hue,
            max_hue: self.max_hue,
            min_saturation: self.min_saturation,
            max_saturation: self.max_saturation,
            min_value: self.min_value,
            max_value: self.max_value,
        }
    }
}

impl fmt::Debug for ColorSpectrum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColorSpectrum")
            .field("hsv_color", &self.hsv_color)
            .field("shape", &self.shape)
            .field("components", &self.components)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

/// `MinHue` to `MaxValue`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ranges {
    min_hue: i32,
    max_hue: i32,
    min_saturation: i32,
    max_saturation: i32,
    min_value: i32,
    max_value: i32,
}

/// The images `CreateBitmapsAndColorMap` made, by the component they are for.
#[derive(Clone, Debug, PartialEq)]
enum SpectrumSurfaces {
    /// `m_saturationMinimumSurface` and `m_saturationMaximumSurface`, for `HueValue` and
    /// `ValueHue`.
    Saturation {
        minimum: Option<Image>,
        maximum: Option<Image>,
    },
    /// `m_valueSurface`, for `HueSaturation` and `SaturationHue`: the maximum value only.
    Value(Option<Image>),
    /// `m_hueRedSurface` to `m_huePurpleSurface`, for `ValueSaturation` and `SaturationValue`.
    Hue([Option<Image>; 6]),
}

/// What the last `CreateBitmapsAndColorMap` made, and the `*FromLastBitmapCreation` fields it
/// made them for, which place the selection ellipse and map the pointer.
struct BitmapCreation {
    shape: ColorSpectrumShape,
    components: ColorSpectrumComponents,
    /// `m_imageWidthFromLastBitmapCreation` and `m_imageHeightFromLastBitmapCreation`, which are
    /// equal.
    size: f64,
    ranges: Ranges,
    /// `m_hsvValues`: the colour at each pixel, row by row.
    hsv_values: Vec<Hsv>,
    surfaces: SpectrumSurfaces,
}

/// `CommonStates` of the template.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpectrumCommonState {
    Normal,
    /// `SelectionEllipse` at 0.8 opacity.
    PointerOver,
    Pressed,
    /// `SelectionEllipsePanel` 48 square, for a pen or touch.
    PressedLarge,
}

/// `SelectionEllipseColor`: a white selection ellipse with a black focus ellipse over dark
/// colours, and the reverse over light ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names, reason = "the template's state names")]
enum SelectionEllipseColor {
    SelectionEllipseLight,
    SelectionEllipseDark,
}

/// `FocusStates`: `FocusEllipse` shows in `Focused`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpectrumFocusState {
    Unfocused,
    Focused,
    PointerFocused,
}

impl StatefulWidget for ColorSpectrum {
    type State = ColorSpectrumState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ColorSpectrumState {
        let color = self.hsv_color.to_color();
        ColorSpectrumState {
            state: StateData::new(),
            focus: None,
            hsv_color: self.hsv_color,
            old_color: Color::from_argb(255, 255, 255, 255),
            color_name: color_display_name(color),
            is_pointer_over: false,
            is_pointer_pressed: false,
            should_show_large_selection: false,
            focused: false,
            layout_root_size: Size::ZERO,
            creation: None,
        }
    }
}

/// The spectrum's own state (`ColorSpectrum.h` members).
pub struct ColorSpectrumState {
    state: StateData<ColorSpectrum>,

    /// The control's focus.
    focus: Option<Handle<FocusNode>>,

    /// `HsvColor` as the spectrum reads it: the colour it last asked for until the application
    /// next builds it, else the application's.
    hsv_color: HsvColor,

    /// `m_oldColor`: `Color` before it last changed.
    old_color: Color,

    /// `ColorNameToolTip.Content`.
    color_name: &'static str,

    /// `m_isPointerOver`.
    is_pointer_over: bool,

    /// `m_isPointerPressed`.
    is_pointer_pressed: bool,

    /// `m_shouldShowLargeSelection`: the press is a pen's or a touch.
    should_show_large_selection: bool,

    /// Whether the control has the focus.
    focused: bool,

    /// `LayoutRoot`'s size after the last layout.
    layout_root_size: Size,

    /// The last `CreateBitmapsAndColorMap`.
    creation: Option<BitmapCreation>,
}

impl ColorSpectrumState {
    fn focus(self: Handle<Self>, app: &App) -> Handle<FocusNode> {
        app.get(self).focus.expect("an initialized ColorSpectrum")
    }

    /// `Color`.
    fn color(self: Handle<Self>, app: &App) -> Color {
        app.get(self).hsv_color.to_color()
    }

    /// `HsvColor` and `Color` become `hsv_color`; `m_oldColor` keeps the colour before when it
    /// changes. Then `RaiseColorChanged`'s test: when the colour changed, or a channel is 0
    /// before and after, the name tooltip shows the new colour's name, and the arguments of
    /// `ColorChanged` come back.
    fn set_hsv_color(
        self: Handle<Self>,
        app: &mut App,
        hsv_color: HsvColor,
    ) -> Option<ColorSpectrumColorChangedEventArgs> {
        let old = self.color(app);
        let new = hsv_color.to_color();
        let state = app.get_mut(self);
        if !same_color(old, new) {
            state.old_color = old;
        }
        state.hsv_color = hsv_color;
        let old_color = state.old_color;
        let changed = !same_color(old_color, new);
        let (before, after) = (argb(old_color), argb(new));
        let channel_zero_in_both = (before.r == after.r && after.r == 0)
            || (before.g == after.g && after.g == 0)
            || (before.b == after.b && after.b == 0);
        if !(changed || channel_zero_in_both) {
            return None;
        }
        state.color_name = color_display_name(new);
        Some(ColorSpectrumColorChangedEventArgs {
            old_color,
            new_color: new,
            hsv_color,
        })
    }

    /// `UpdateColor`: the user chose `hsv`, with the current alpha, and `ColorChanged` asks the
    /// application for it.
    fn update_color(self: Handle<Self>, app: &mut App, hsv: Hsv) {
        let alpha = app.get(self).hsv_color.alpha;
        let args = self.set_hsv_color(app, HsvColor::from_hsv(hsv, alpha));
        self.set_state(app, |_| {});
        if let Some(args) = args {
            let handler = self.widget(app).color_changed.clone();
            handler(app, args);
        }
    }

    /// `UpdateColorFromPoint`: the colour of the pixel under `point` of `InputTarget`, pulled into
    /// the circle for a ring, with the current third component. Nothing happens before the images
    /// are made.
    fn update_color_from_point(self: Handle<Self>, app: &mut App, point: Offset) {
        let shape = self.widget(app).shape;
        let current = app.get(self).hsv_color;
        let Some(creation) = &app.get(self).creation else {
            return;
        };
        if creation.hsv_values.is_empty() {
            return;
        }
        let size = creation.size;
        let mut x_position = point.dx();
        let mut y_position = point.dy();
        let radius = size / 2.0;
        let distance = ((x_position - radius).powi(2) + (y_position - radius).powi(2)).sqrt();
        if distance > radius && shape == ColorSpectrumShape::Ring {
            x_position = (radius / distance) * (x_position - radius) + radius;
            y_position = (radius / distance) * (y_position - radius) + radius;
        }
        let width = size.round() as i64;
        let clamp_index = |position: f64| {
            let index = position.round() as i64;
            if index < 0 {
                0
            } else if position.round() >= size {
                width - 1
            } else {
                index
            }
        };
        let (x, y) = (clamp_index(x_position), clamp_index(y_position));
        let mut hsv = creation.hsv_values[(y * width + x) as usize];
        // The images hold two components; the third stays where it is.
        match self.widget(app).components {
            ColorSpectrumComponents::HueValue | ColorSpectrumComponents::ValueHue => {
                hsv.s = f64::from(current.saturation)
            }
            ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue => {
                hsv.v = f64::from(current.value)
            }
            ColorSpectrumComponents::ValueSaturation | ColorSpectrumComponents::SaturationValue => {
                hsv.h = f64::from(current.hue)
            }
        }
        self.update_color(app, hsv);
    }

    /// `OnKeyDown`: the arrows step the component across (Left, Right) or down (Up, Down) the
    /// spectrum by one, or with Ctrl to the next named colour, wrapping at the bounds. Each arrow
    /// moves the selection its way: left and up lower hue but raise saturation and value, whose
    /// axes run the other way, except across a `SaturationValue` spectrum.
    fn key_down(self: Handle<Self>, app: &mut App, key: LogicalKeyboardKey) -> KeyEventResult {
        let horizontal = match key {
            LogicalKeyboardKey::ARROW_LEFT | LogicalKeyboardKey::ARROW_RIGHT => true,
            LogicalKeyboardKey::ARROW_UP | LogicalKeyboardKey::ARROW_DOWN => false,
            _ => return KeyEventResult::Ignored,
        };
        let widget = self.widget(app).clone();
        let control_down = HardwareKeyboard::instance(app).is_control_pressed(app);
        let channel = arrow_channel(widget.components, horizontal);
        let is_saturation_value = widget.components == ColorSpectrumComponents::SaturationValue;
        let (min_bound, max_bound) = match channel {
            ColorPickerHsvChannel::Hue => (widget.min_hue, widget.max_hue),
            ColorPickerHsvChannel::Saturation => (widget.min_saturation, widget.max_saturation),
            _ => (widget.min_value, widget.max_value),
        };
        let lowers = if channel == ColorPickerHsvChannel::Hue {
            matches!(
                key,
                LogicalKeyboardKey::ARROW_LEFT | LogicalKeyboardKey::ARROW_UP
            )
        } else {
            matches!(
                key,
                LogicalKeyboardKey::ARROW_RIGHT | LogicalKeyboardKey::ARROW_DOWN
            )
        };
        let mut direction = if lowers {
            IncrementDirection::Lower
        } else {
            IncrementDirection::Higher
        };
        // A `SaturationValue` spectrum runs the other way across.
        if is_saturation_value && horizontal {
            direction = direction.reversed();
        }
        let amount = if control_down {
            IncrementAmount::Large
        } else {
            IncrementAmount::Small
        };
        let hsv = app.get(self).hsv_color.hsv();
        let new_hsv = increment_color_channel(
            hsv,
            channel,
            direction,
            amount,
            true,
            f64::from(min_bound),
            f64::from(max_bound),
        );
        self.update_color(app, new_hsv);
        KeyEventResult::Handled
    }

    /// `OnInputTargetPointerPressed`: the control takes the focus, the pointer is pressed (large
    /// for a pen or touch) and the colour under it is chosen.
    fn pointer_pressed(self: Handle<Self>, app: &mut App, event: &PointerDownEvent) {
        let focus = self.focus(app);
        FocusState::name(app, focus.as_node(), FocusState::Pointer);
        focus.request_focus(app, None);
        let large = matches!(
            event.kind,
            PointerDeviceKind::Touch
                | PointerDeviceKind::Stylus
                | PointerDeviceKind::InvertedStylus
        );
        self.set_state(app, |state| {
            state.is_pointer_pressed = true;
            state.should_show_large_selection = large;
        });
        self.update_color_from_point(app, event.local_position());
    }

    /// `OnInputTargetPointerMoved`: while pressed, the colour under the pointer is chosen.
    fn pointer_moved(self: Handle<Self>, app: &mut App, point: Offset) {
        if app.get(self).is_pointer_pressed {
            self.update_color_from_point(app, point);
        }
    }

    /// `OnInputTargetPointerReleased`.
    fn pointer_released(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| {
            state.is_pointer_pressed = false;
            state.should_show_large_selection = false;
        });
    }

    /// `OnInputTargetPointerEntered` and `OnInputTargetPointerExited`.
    fn pointer_over(self: Handle<Self>, app: &mut App, over: bool) {
        self.set_state(app, |state| state.is_pointer_over = over);
    }

    /// `OnGotFocus` and `OnLostFocus`: arriving focus opens the name tooltip; leaving closes it.
    fn focus_changed(self: Handle<Self>, app: &mut App) {
        let focused = self.focus(app).has_primary_focus(app);
        if focused != app.get(self).focused {
            self.set_state(app, |state| state.focused = focused);
        }
    }

    /// `OnLayoutRootSizeChanged`.
    fn layout_root_size_changed(self: Handle<Self>, app: &mut App, size: Size) {
        if !app.contains(self) || !self.mounted(app) || app.get(self).layout_root_size == size {
            return;
        }
        app.get_mut(self).layout_root_size = size;
        self.create_bitmaps_and_color_map(app);
        self.set_state(app, |_| {});
    }

    /// `CreateBitmapsAndColorMap`: the images and the colour of each pixel for the current size,
    /// shape, components and ranges. Nothing is made while `LayoutRoot` has no size.
    fn create_bitmaps_and_color_map(self: Handle<Self>, app: &mut App) {
        let size = app.get(self).layout_root_size;
        let min_dimension = size.width().min(size.height());
        if min_dimension == 0.0 {
            return;
        }
        let widget = self.widget(app).clone();
        let ranges = widget.ranges();
        let pixels = spectrum_pixels(
            min_dimension.round() as usize,
            widget.shape,
            widget.components,
            ranges,
        );
        let side = min_dimension.round() as u32;
        let image = |data: Vec<u8>| image_from_pixel_data(app, side, side, data);
        let surfaces = match pixels.images {
            SpectrumImages::Two { minimum, maximum } => match widget.components {
                ColorSpectrumComponents::HueValue | ColorSpectrumComponents::ValueHue => {
                    SpectrumSurfaces::Saturation {
                        minimum: image(minimum),
                        maximum: image(maximum),
                    }
                }
                _ => SpectrumSurfaces::Value(image(maximum)),
            },
            SpectrumImages::Six(hues) => SpectrumSurfaces::Hue(hues.map(image)),
        };
        app.get_mut(self).creation = Some(BitmapCreation {
            shape: widget.shape,
            components: widget.components,
            size: min_dimension,
            ranges,
            hsv_values: pixels.hsv_values,
            surfaces,
        });
    }

    /// `UpdateBitmapSources`: the images to fill with and the second's opacity. For saturation
    /// or value as the third component, the images at its minimum and maximum (one image for
    /// value, at its maximum) at the opacity of the colour's saturation (or 1); for hue, the
    /// primary hues on either side of the colour's hue, at how far past the first it is.
    fn spectrum_fill(self: Handle<Self>, app: &App) -> Option<SpectrumFill> {
        let hsv = app.get(self).hsv_color;
        let creation = app.get(self).creation.as_ref()?;
        match &creation.surfaces {
            SpectrumSurfaces::Saturation { minimum, maximum } => Some(SpectrumFill {
                minimum: minimum.clone()?,
                maximum: maximum.clone()?,
                weight: f64::from(hsv.saturation),
            }),
            SpectrumSurfaces::Value(value) => Some(SpectrumFill {
                minimum: value.clone()?,
                maximum: value.clone()?,
                weight: 1.0,
            }),
            SpectrumSurfaces::Hue(hues) => {
                let sextant = f64::from(hsv.hue) / 60.0;
                let first = if sextant < 1.0 {
                    0
                } else if sextant < 2.0 {
                    1
                } else if sextant < 3.0 {
                    2
                } else if sextant < 4.0 {
                    3
                } else if sextant < 5.0 {
                    4
                } else {
                    5
                };
                Some(SpectrumFill {
                    minimum: hues[first].clone()?,
                    maximum: hues[(first + 1) % 6].clone()?,
                    weight: sextant - f64::from(sextant as i32),
                })
            }
        }
    }

    /// `UpdateEllipse`: the centre of the selection ellipse in `InputTarget`, from the images
    /// last made; `None`, collapsing it, before there are any.
    fn ellipse_center(self: Handle<Self>, app: &App) -> Option<Offset> {
        let hsv = app.get(self).hsv_color;
        let creation = app.get(self).creation.as_ref()?;
        Some(ellipse_center(hsv, creation))
    }

    /// `SelectionEllipseShouldBeLight`: a light selection ellipse when the colour under it
    /// contrasts more with black than with white, by relative luminance. With value as the third
    /// component the spectrum shows full value, so that colour is the one judged.
    fn selection_ellipse_color(self: Handle<Self>, app: &App) -> SelectionEllipseColor {
        let hsv = app.get(self).hsv_color;
        let displayed = match self.widget(app).components {
            ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue => {
                let full_value = Hsv::new(f64::from(hsv.hue), f64::from(hsv.saturation), 1.0);
                color_from_rgba(hsv_to_rgb(full_value), f64::from(hsv.alpha))
            }
            _ => hsv.to_color(),
        };
        let linear = |channel: u8| {
            let channel = f64::from(channel);
            if channel <= 10.0 {
                channel / 3294.0
            } else {
                (channel / 269.0 + 0.0513).powf(2.4)
            }
        };
        let displayed = argb(displayed);
        let luminance = 0.2126 * linear(displayed.r)
            + 0.7152 * linear(displayed.g)
            + 0.0722 * linear(displayed.b);
        if luminance <= 0.5 {
            SelectionEllipseColor::SelectionEllipseLight
        } else {
            SelectionEllipseColor::SelectionEllipseDark
        }
    }

    /// `UpdateVisualState`'s `CommonStates`.
    fn common_state(self: Handle<Self>, app: &App) -> SpectrumCommonState {
        let state = app.get(self);
        if state.is_pointer_pressed {
            if state.should_show_large_selection {
                SpectrumCommonState::PressedLarge
            } else {
                SpectrumCommonState::Pressed
            }
        } else if state.is_pointer_over {
            SpectrumCommonState::PointerOver
        } else {
            SpectrumCommonState::Normal
        }
    }

    /// `UpdateVisualState`'s `FocusStates`, by how the focus arrived.
    fn focus_visual_state(self: Handle<Self>, app: &mut App) -> SpectrumFocusState {
        if !app.get(self).focused || !self.widget(app).is_enabled {
            return SpectrumFocusState::Unfocused;
        }
        match FocusState::current(app) {
            FocusState::Pointer => SpectrumFocusState::PointerFocused,
            FocusState::Keyboard => SpectrumFocusState::Focused,
        }
    }

    /// `OnPropertyChanged` for the properties the images depend on: `Shape` and `Components`
    /// always remake them, and a component's range when that component is an axis.
    fn images_outdated(old: &ColorSpectrum, new: &ColorSpectrum) -> bool {
        use ColorSpectrumComponents::*;
        let hue_range = (old.min_hue, old.max_hue) != (new.min_hue, new.max_hue);
        let saturation_range =
            (old.min_saturation, old.max_saturation) != (new.min_saturation, new.max_saturation);
        let value_range = (old.min_value, old.max_value) != (new.min_value, new.max_value);
        old.shape != new.shape
            || old.components != new.components
            || (hue_range && !matches!(new.components, SaturationValue | ValueSaturation))
            || (saturation_range && !matches!(new.components, HueValue | ValueHue))
            || (value_range && !matches!(new.components, HueSaturation | SaturationHue))
    }
}

/// The component an arrow steps: across (Left and Right) the first of `components`, down (Up
/// and Down) the second.
fn arrow_channel(components: ColorSpectrumComponents, horizontal: bool) -> ColorPickerHsvChannel {
    use ColorPickerHsvChannel::{Hue, Saturation, Value};
    use ColorSpectrumComponents::*;
    match (components, horizontal) {
        (HueSaturation | HueValue, true) | (SaturationHue | ValueHue, false) => Hue,
        (SaturationValue | SaturationHue, true) | (HueSaturation | ValueSaturation, false) => {
            Saturation
        }
        (ValueHue | ValueSaturation, true) | (SaturationValue | HueValue, false) => Value,
    }
}

/// `UpdateEllipse`'s position: the colour, clamped to the ranges, as a share of each axis of a
/// box, or as an angle and a distance from the centre of a ring. Saturation with hue, and value
/// in any spectrum, run from their maximum at the top or the outside.
fn ellipse_center(hsv: HsvColor, creation: &BitmapCreation) -> Offset {
    let ranges = creation.ranges;
    let (min_hue, max_hue) = (f64::from(ranges.min_hue), f64::from(ranges.max_hue));
    let (min_saturation, max_saturation) = (
        f64::from(ranges.min_saturation),
        f64::from(ranges.max_saturation),
    );
    let (min_value, max_value) = (f64::from(ranges.min_value), f64::from(ranges.max_value));
    let hue = f64::from(clamp_f32(hsv.hue, min_hue as f32, max_hue as f32));
    let saturation = f64::from(clamp_f32(
        hsv.saturation,
        (min_saturation / 100.0) as f32,
        (max_saturation / 100.0) as f32,
    ));
    let value = f64::from(clamp_f32(
        hsv.value,
        (min_value / 100.0) as f32,
        (max_value / 100.0) as f32,
    ));
    let saturation_with_hue = matches!(
        creation.components,
        ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue
    );
    let size = creation.size;
    let (x, y) = if creation.shape == ColorSpectrumShape::Box {
        let h = (hue - min_hue) / (max_hue - min_hue);
        let mut s = (saturation * 100.0 - min_saturation) / (max_saturation - min_saturation);
        let mut v = (value * 100.0 - min_value) / (max_value - min_value);
        if saturation_with_hue {
            s = 1.0 - s;
        } else {
            v = 1.0 - v;
        }
        let (x_percent, y_percent) = match creation.components {
            ColorSpectrumComponents::HueValue => (h, v),
            ColorSpectrumComponents::HueSaturation => (h, s),
            ColorSpectrumComponents::ValueHue => (v, h),
            ColorSpectrumComponents::ValueSaturation => (v, s),
            ColorSpectrumComponents::SaturationHue => (s, h),
            ColorSpectrumComponents::SaturationValue => (s, v),
        };
        (size * x_percent, size * y_percent)
    } else {
        let share = |value: f64, min: f64, max: f64| {
            if max != min {
                (value - min) / (max - min)
            } else {
                0.0
            }
        };
        let theta_of = |value: f64, min: f64, max: f64| {
            if max != min {
                360.0 * share(value, min, max)
            } else {
                0.0
            }
        };
        let r_of = |value: f64, min: f64, max: f64| {
            if max != min {
                share(value, min, max) - 1.0
            } else {
                0.0
            }
        };
        let h_theta = theta_of(hue, min_hue, max_hue);
        let mut s_theta = theta_of(saturation * 100.0, min_saturation, max_saturation);
        let mut v_theta = theta_of(value * 100.0, min_value, max_value);
        let h_r = r_of(hue, min_hue, max_hue);
        let mut s_r = r_of(saturation * 100.0, min_saturation, max_saturation);
        let mut v_r = r_of(value * 100.0, min_value, max_value);
        if saturation_with_hue {
            s_theta = 360.0 - s_theta;
            s_r = -s_r - 1.0;
        } else {
            v_theta = 360.0 - v_theta;
            v_r = -v_r - 1.0;
        }
        let (theta, r) = match creation.components {
            ColorSpectrumComponents::HueValue => (h_theta, v_r),
            ColorSpectrumComponents::HueSaturation => (h_theta, s_r),
            ColorSpectrumComponents::ValueHue => (v_theta, h_r),
            ColorSpectrumComponents::ValueSaturation => (v_theta, s_r),
            ColorSpectrumComponents::SaturationHue => (s_theta, h_r),
            ColorSpectrumComponents::SaturationValue => (s_theta, v_r),
        };
        let radius = size / 2.0;
        let angle = theta * PI / 180.0 + PI;
        (
            angle.cos() * radius * r + radius,
            angle.sin() * radius * r + radius,
        )
    };
    // A range of one value divides by zero; `Canvas.Left` and `Canvas.Top` of NaN place at 0.
    let finite = |position: f64| if position.is_finite() { position } else { 0.0 };
    Offset::new(finite(x), finite(y))
}

/// `clamp` of floats, as `UpdateEllipse` clamps the colour; `min` wins over `max`.
fn clamp_f32(value: f32, min: f32, max: f32) -> f32 {
    if value < min {
        min
    } else if max < value {
        max
    } else {
        value
    }
}

/// The pixel data `CreateBitmapsAndColorMap` computes for one size, shape, components and
/// ranges.
struct SpectrumPixels {
    /// The colour of each pixel, row by row; the third component is the minimum's.
    hsv_values: Vec<Hsv>,
    images: SpectrumImages,
}

/// The BGRA pixels of the images: the third component at its minimum and maximum, or at the six
/// primary hues.
enum SpectrumImages {
    Two { minimum: Vec<u8>, maximum: Vec<u8> },
    Six([Vec<u8>; 6]),
}

/// Whether hue is the third component of `components`, so that six images are made.
fn hue_is_third(components: ColorSpectrumComponents) -> bool {
    matches!(
        components,
        ColorSpectrumComponents::ValueSaturation | ColorSpectrumComponents::SaturationValue
    )
}

/// The pixel loops of `CreateBitmapsAndColorMap`: a box is filled column by column from its
/// bottom right, which with the axes `FillPixelForBox` gives puts the first component across
/// and the second down; a ring row by row. When a range's minimum is not below its maximum, the
/// minimum is the only value.
fn spectrum_pixels(
    side: usize,
    shape: ColorSpectrumShape,
    components: ColorSpectrumComponents,
    ranges: Ranges,
) -> SpectrumPixels {
    let ranges = Ranges {
        max_hue: ranges.max_hue.max(ranges.min_hue),
        max_saturation: ranges.max_saturation.max(ranges.min_saturation),
        max_value: ranges.max_value.max(ranges.min_value),
        ..ranges
    };
    let count = side * side;
    let images = if hue_is_third(components) { 6 } else { 2 };
    let mut data: Vec<Vec<u8>> = (0..images).map(|_| Vec::with_capacity(count * 4)).collect();
    let mut hsv_values = Vec::with_capacity(count);
    let mut fill = |hsvs: [Hsv; 6]| {
        hsv_values.push(hsvs[0]);
        let used: &[Hsv] = if images == 6 {
            &hsvs
        } else {
            &[hsvs[0], hsvs[5]]
        };
        for (pixels, hsv) in data.iter_mut().zip(used) {
            let rgb = hsv_to_rgb(*hsv);
            let byte = |channel: f64| (channel * 255.0).round() as u8;
            // The alpha byte is ignored.
            pixels.extend([byte(rgb.b), byte(rgb.g), byte(rgb.r), 255]);
        }
    };
    let n = side as f64;
    match shape {
        ColorSpectrumShape::Box => {
            for x in (0..side).rev() {
                for y in (0..side).rev() {
                    fill(pixel_for_box(x as f64, y as f64, n, components, ranges));
                }
            }
        }
        ColorSpectrumShape::Ring => {
            for y in 0..side {
                for x in 0..side {
                    fill(pixel_for_ring(
                        x as f64,
                        y as f64,
                        n / 2.0,
                        components,
                        ranges,
                    ));
                }
            }
        }
    }
    let images = if images == 6 {
        let mut six = data.into_iter();
        SpectrumImages::Six(std::array::from_fn(|_| six.next().expect("six images")))
    } else {
        let maximum = data.pop().expect("a maximum image");
        let minimum = data.pop().expect("a minimum image");
        SpectrumImages::Two { minimum, maximum }
    };
    SpectrumPixels { hsv_values, images }
}

/// The ranges in HSV units.
fn hsv_ranges(ranges: Ranges) -> [f64; 6] {
    [
        f64::from(ranges.min_hue),
        f64::from(ranges.max_hue),
        f64::from(ranges.min_saturation) / 100.0,
        f64::from(ranges.max_saturation) / 100.0,
        f64::from(ranges.min_value) / 100.0,
        f64::from(ranges.max_value) / 100.0,
    ]
}

/// `FillPixelForBox`: the colours of pixel `(x, y)` of the six images, from the shares `x` and
/// `y` are of the way back from the far corner.
fn pixel_for_box(
    x: f64,
    y: f64,
    side: f64,
    components: ColorSpectrumComponents,
    ranges: Ranges,
) -> [Hsv; 6] {
    let x_percent = (side - 1.0 - x) / (side - 1.0);
    let y_percent = (side - 1.0 - y) / (side - 1.0);
    pixel_colors(x_percent, y_percent, components, ranges)
}

/// `FillPixelForRing`: the colours of pixel `(x, y)` from its angle around the centre and its
/// distance from the edge. A pixel outside the circle takes the colour at the edge, so that
/// hit-testing the edge always finds one.
fn pixel_for_ring(
    x: f64,
    y: f64,
    radius: f64,
    components: ColorSpectrumComponents,
    ranges: Ranges,
) -> [Hsv; 6] {
    let mut distance = ((x - radius).powi(2) + (y - radius).powi(2)).sqrt();
    let (mut x_to_use, mut y_to_use) = (x, y);
    if distance > radius {
        x_to_use = (radius / distance) * (x - radius) + radius;
        y_to_use = (radius / distance) * (y - radius) + radius;
        distance = radius;
    }
    let r = 1.0 - distance / radius;
    let mut theta = ((radius - y_to_use).atan2(radius - x_to_use) * 180.0 / PI + 180.0).floor();
    while theta > 360.0 {
        theta -= 360.0;
    }
    pixel_colors(r, theta / 360.0, components, ranges)
}

/// The shared tail of `FillPixelForBox` and `FillPixelForRing`, for `first`, a box's `xPercent`
/// or a ring's `r`, and `second`, a box's `yPercent` or a ring's `thetaPercent`: the components
/// at those shares of their ranges and the third at its minimum and maximum (or the six primary
/// hues), with saturation beside hue, and value anywhere, inverted so that they run from their
/// maximum at the top or the outside.
fn pixel_colors(
    first: f64,
    second: f64,
    components: ColorSpectrumComponents,
    ranges: Ranges,
) -> [Hsv; 6] {
    let [h_min, h_max, s_min, s_max, v_min, v_max] = hsv_ranges(ranges);
    let hue = |share: f64| h_min + share * (h_max - h_min);
    let saturation = |share: f64| s_min + share * (s_max - s_min);
    let value = |share: f64| v_min + share * (v_max - v_min);
    let with_third = |h: Option<f64>, s: Option<f64>, v: Option<f64>| -> [Hsv; 6] {
        std::array::from_fn(|i| {
            let third = match (h, s, v) {
                (None, _, _) => 60.0 * i as f64,
                _ if i == 5 => 1.0,
                _ => 0.0,
            };
            Hsv::new(h.unwrap_or(third), s.unwrap_or(third), v.unwrap_or(third))
        })
    };
    let mut colors = match components {
        ColorSpectrumComponents::HueValue => {
            with_third(Some(hue(second)), None, Some(value(first)))
        }
        ColorSpectrumComponents::HueSaturation => {
            with_third(Some(hue(second)), Some(saturation(first)), None)
        }
        ColorSpectrumComponents::ValueHue => {
            with_third(Some(hue(first)), None, Some(value(second)))
        }
        ColorSpectrumComponents::ValueSaturation => {
            with_third(None, Some(saturation(first)), Some(value(second)))
        }
        ColorSpectrumComponents::SaturationHue => {
            with_third(Some(hue(first)), Some(saturation(second)), None)
        }
        ColorSpectrumComponents::SaturationValue => {
            with_third(None, Some(saturation(second)), Some(value(first)))
        }
    };
    let saturation_with_hue = matches!(
        components,
        ColorSpectrumComponents::HueSaturation | ColorSpectrumComponents::SaturationHue
    );
    for hsv in &mut colors {
        if saturation_with_hue {
            hsv.s = s_max - hsv.s + s_min;
        } else {
            hsv.v = v_max - hsv.v + v_min;
        }
    }
    colors
}

impl State for ColorSpectrumState {
    type Widget = ColorSpectrum;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        InputDevice::initialize(app);
        let focus = FocusNode::new(app);
        focus.as_node().set_on_key_event(
            app,
            Some(Rc::new(move |app, _, event: &KeyEvent| match event {
                KeyEvent::Down(down) => self.key_down(app, down.logical_key),
                KeyEvent::Repeat(repeat) => self.key_down(app, repeat.logical_key),
                KeyEvent::Up(_) => KeyEventResult::Ignored,
            })),
        );
        focus.add_listener(app, Listener::handle_method(self, Self::focus_changed));
        app.get_mut(self).focus = Some(focus);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &ColorSpectrum) {
        let widget = self.widget(app).clone();
        if ColorSpectrumState::images_outdated(old, &widget) {
            self.create_bitmaps_and_color_map(app);
        }
        // `OnHsvColorChanged`: the application's colour, unless it is the one the spectrum has.
        if widget.hsv_color != app.get(self).hsv_color {
            self.set_hsv_color(app, widget.hsv_color);
        }
        if !widget.is_enabled {
            let state = app.get_mut(self);
            state.is_pointer_over = false;
            state.is_pointer_pressed = false;
            state.should_show_large_selection = false;
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let focus = app
            .get_mut(self)
            .focus
            .take()
            .expect("an initialized ColorSpectrum");
        focus.remove_listener(app, &Listener::handle_method(self, Self::focus_changed));
        focus.dispose(app);
        app.destroy(focus);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let r = ThemeResources::of(app, context);
        let shape = app
            .get(self)
            .creation
            .as_ref()
            .map_or(widget.shape, |c| c.shape);
        let common = self.common_state(app);
        let ellipse_color = self.selection_ellipse_color(app);
        let focus_state = self.focus_visual_state(app);
        let (selection_stroke, focus_stroke) = match ellipse_color {
            SelectionEllipseColor::SelectionEllipseLight => (
                r.system_control_background_chrome_white_brush,
                r.system_control_background_chrome_black_high_brush,
            ),
            SelectionEllipseColor::SelectionEllipseDark => (
                r.system_control_background_chrome_black_high_brush,
                r.system_control_background_chrome_white_brush,
            ),
        };
        let panel_size = if common == SpectrumCommonState::PressedLarge {
            SELECTION_ELLIPSE_LARGE_SIZE
        } else {
            SELECTION_ELLIPSE_SIZE
        };
        // `SelectionEllipse`, owning `ColorNameToolTip`, which shows while the control has the
        // focus.
        let selection_ellipse = ToolTipService::new(
            Opacity::new(if common == SpectrumCommonState::PointerOver {
                POINTER_OVER_SELECTION_OPACITY
            } else {
                1.0
            })
            .child(CustomPaint::new().painter(EllipseStrokePainter {
                stroke: selection_stroke,
                thickness: ELLIPSE_STROKE_THICKNESS,
            })),
            ToolTip::text(app.get(self).color_name).vertical_offset(COLOR_NAME_TOOL_TIP_OFFSET),
        )
        .is_open(app.get(self).focused && widget.is_enabled);
        // `SelectionEllipsePanel`, with `FocusEllipse` 2 larger on every side.
        let mut panel_children = Vec::new();
        if focus_state == SpectrumFocusState::Focused {
            panel_children.push(
                Positioned::from_rect(
                    Rect::from_ltwh(
                        -FOCUS_ELLIPSE_OUTSET,
                        -FOCUS_ELLIPSE_OUTSET,
                        panel_size + 2.0 * FOCUS_ELLIPSE_OUTSET,
                        panel_size + 2.0 * FOCUS_ELLIPSE_OUTSET,
                    ),
                    CustomPaint::new().painter(EllipseStrokePainter {
                        stroke: focus_stroke,
                        thickness: ELLIPSE_STROKE_THICKNESS,
                    }),
                )
                .into_widget(),
            );
        }
        panel_children.push(Positioned::fill(selection_ellipse).into_widget());
        let mut input_children = Vec::new();
        if let Some(center) = self.ellipse_center(app) {
            input_children.push(
                Positioned::from_rect(
                    Rect::from_ltwh(
                        center.dx() - panel_size / 2.0,
                        center.dy() - panel_size / 2.0,
                        panel_size,
                        panel_size,
                    ),
                    IgnorePointer::new().child(
                        Stack::new()
                            .clip_behavior(Clip::None)
                            .children(panel_children),
                    ),
                )
                .into_widget(),
            );
        }
        let input_target = self.input_target(
            app,
            widget.is_enabled,
            Stack::new()
                .fit(StackFit::Expand)
                .clip_behavior(Clip::None)
                .children(input_children)
                .into_widget(),
        );
        let fill = self.spectrum_fill(app);
        // `SizingGrid`: its children in order, clipped to the square.
        let sizing_grid_children = vec![
            IgnorePointer::new()
                .child(CustomPaint::new().painter(SpectrumPainter {
                    shape,
                    corner_radius: widget.corner_radius,
                    fill,
                }))
                .into_widget(),
            input_target,
            IgnorePointer::new()
                .child(CustomPaint::new().painter(SpectrumBorderPainter {
                    shape,
                    corner_radius: widget.corner_radius,
                    stroke: r.color_picker_border_brush,
                }))
                .into_widget(),
        ];
        let side = app.get(self).layout_root_size;
        let side = side.width().min(side.height());
        let sizing_grid = if side > 0.0 {
            SizedBox::new()
                .width(side)
                .height(side)
                .child(
                    ClipRect::new().child(
                        Stack::new()
                            .fit(StackFit::Expand)
                            .children(sizing_grid_children),
                    ),
                )
                .into_widget()
        } else {
            SizedBox::shrink().into_widget()
        };
        // `LayoutRoot`: centres `SizingGrid` and reports its own size.
        let layout_root = SizeObserver::new(
            Center::new()
                .width_factor(1.0)
                .height_factor(1.0)
                .child(sizing_grid),
            Rc::new(move |app, _, size| self.layout_root_size_changed(app, size)),
        );
        Focus::new(layout_root)
            .focus_node(self.focus(app).as_node())
            .can_request_focus(widget.is_enabled)
            .skip_traversal(!widget.is_enabled)
            .into_widget()
    }
}

impl ColorSpectrumState {
    /// `InputTarget`: takes the pointer over the whole square while enabled
    /// (`ManipulationMode="None"`, so that no scrolling takes a touch from it), and reports its
    /// pointer entering and leaving.
    fn input_target(self: Handle<Self>, _app: &App, enabled: bool, child: WidgetRef) -> WidgetRef {
        if !enabled {
            return child;
        }
        let listener = PointerListener::new()
            .behavior(HitTestBehavior::Opaque)
            .on_pointer_down(Rc::new(move |app: &mut App, event| {
                self.pointer_pressed(app, &event);
            }))
            .on_pointer_move(Rc::new(move |app: &mut App, event| {
                self.pointer_moved(app, event.local_position());
            }))
            .on_pointer_up(Rc::new(move |app: &mut App, _event| {
                self.pointer_released(app);
            }))
            .child(child);
        let region = MouseRegion::new()
            .on_enter(Rc::new(move |app, _| self.pointer_over(app, true)))
            .on_exit(Rc::new(move |app, _| self.pointer_over(app, false)))
            .child(listener);
        RawGestureDetector::new()
            .gestures(vec![(
                TypeId::of::<EagerGestureRecognizer>(),
                GestureRecognizerFactoryWithHandlers::new(EagerGestureRecognizer::new, |_, _| {})
                    .into_factory(),
            )])
            .child(region)
            .into_widget()
    }
}

/// The `SpectrumBrush` of the fill: `maximum` over `minimum` at `weight`'s opacity.
#[derive(Clone, Debug, PartialEq)]
struct SpectrumFill {
    minimum: Image,
    maximum: Image,
    weight: f64,
}

/// `SpectrumRectangle` (the control's `CornerRadius` as `RadiusX` and `RadiusY`) or
/// `SpectrumEllipse`, filled with the spectrum images stretched over it.
#[derive(Clone, Debug, PartialEq)]
struct SpectrumPainter {
    shape: ColorSpectrumShape,
    corner_radius: f64,
    fill: Option<SpectrumFill>,
}

impl SpectrumPainter {
    /// Fills the shape with `image` stretched over `size`, at `opacity`.
    fn fill_shape(&self, canvas: &mut Canvas, size: Size, image: &Image, opacity: f64) {
        let [width, height] = image.size();
        let local = Matrix::scale(
            (size.width() / f64::from(width)) as f32,
            (size.height() / f64::from(height)) as f32,
        );
        let mut paint = Paint::from_shader(Shader::Image {
            image: image.clone(),
            sampling: Default::default(),
            local,
        });
        paint.color = paint.color.with_alpha(opacity as f32);
        paint_shape(canvas, self.shape, size, self.corner_radius, &paint);
    }
}

impl CustomPainter for SpectrumPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let Some(fill) = &self.fill else {
            return;
        };
        self.fill_shape(canvas, size, &fill.minimum, 1.0);
        if fill.weight > 0.0 && fill.maximum != fill.minimum {
            self.fill_shape(canvas, size, &fill.maximum, fill.weight);
        }
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Fills `shape` over `size`: a rounded rectangle or the ellipse in it.
fn paint_shape(
    canvas: &mut Canvas,
    shape: ColorSpectrumShape,
    size: Size,
    corner_radius: f64,
    paint: &Paint,
) {
    let bounds = Rect::from_ltwh(0.0, 0.0, size.width(), size.height());
    match shape {
        ColorSpectrumShape::Box => {
            canvas.draw_rrect(bounds, shape_radius(corner_radius, bounds) as f32, paint)
        }
        ColorSpectrumShape::Ring => draw_oval(canvas, bounds, paint),
    }
}

/// `RadiusX` and `RadiusY` as `CreateRoundedRect` limits them: no more than half the side.
fn shape_radius(radius: f64, bounds: Rect) -> f64 {
    radius
        .abs()
        .min(bounds.width() / 2.0)
        .min(bounds.height() / 2.0)
}

/// `RectangleBorder` or `EllipseBorder` in `ColorPickerBorderStyle`: a 2-pixel stroke of `stroke`
/// inside the bounds, as a shape strokes the outline inset by half its thickness.
#[derive(Clone, Debug, PartialEq)]
struct SpectrumBorderPainter {
    shape: ColorSpectrumShape,
    corner_radius: f64,
    stroke: Color,
}

impl CustomPainter for SpectrumBorderPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let outline = Rect::from_ltwh(0.0, 0.0, size.width(), size.height())
            .deflate(COLOR_PICKER_BORDER_THICKNESS / 2.0);
        let mut paint = Paint::from_color(self.stroke.into());
        paint.style = PaintStyle::Stroke(Stroke::new(COLOR_PICKER_BORDER_THICKNESS as f32));
        match self.shape {
            ColorSpectrumShape::Box => canvas.draw_rrect(
                outline,
                shape_radius(self.corner_radius, outline) as f32,
                &paint,
            ),
            ColorSpectrumShape::Ring => draw_oval(canvas, outline, &paint),
        }
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// An `Ellipse` with a `Stroke` of `thickness` and no fill, inside its box.
#[derive(Clone, Debug, PartialEq)]
struct EllipseStrokePainter {
    stroke: Color,
    thickness: f64,
}

impl CustomPainter for EllipseStrokePainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let outline =
            Rect::from_ltwh(0.0, 0.0, size.width(), size.height()).deflate(self.thickness / 2.0);
        let mut paint = Paint::from_color(self.stroke.into());
        paint.style = PaintStyle::Stroke(Stroke::new(self.thickness as f32));
        draw_oval(canvas, outline, &paint);
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creation(
        shape: ColorSpectrumShape,
        components: ColorSpectrumComponents,
        size: f64,
    ) -> BitmapCreation {
        let ranges = Ranges {
            min_hue: 0,
            max_hue: 359,
            min_saturation: 0,
            max_saturation: 100,
            min_value: 0,
            max_value: 100,
        };
        let pixels = spectrum_pixels(size as usize, shape, components, ranges);
        BitmapCreation {
            shape,
            components,
            size,
            ranges,
            hsv_values: pixels.hsv_values,
            surfaces: SpectrumSurfaces::Value(None),
        }
    }

    fn red_at(creation: &BitmapCreation, x: usize, y: usize) -> Hsv {
        creation.hsv_values[y * creation.size as usize + x]
    }

    #[test]
    fn a_hue_saturation_box_runs_hue_across_and_saturation_down() {
        let c = creation(
            ColorSpectrumShape::Box,
            ColorSpectrumComponents::HueSaturation,
            256.0,
        );
        assert_eq!(red_at(&c, 0, 0), Hsv::new(0.0, 1.0, 0.0));
        let right = red_at(&c, 255, 0);
        assert_eq!((right.h, right.s), (359.0, 1.0));
        let bottom = red_at(&c, 0, 255);
        assert_eq!((bottom.h, bottom.s), (0.0, 0.0));
        // The ellipse of a pixel's colour is on that pixel.
        let hsv = HsvColor::from_hsv(Hsv::new(right.h, right.s, 1.0), 1.0);
        assert_eq!(ellipse_center(hsv, &c), Offset::new(256.0, 0.0));
    }

    #[test]
    fn a_ring_puts_hue_around_and_saturation_outwards() {
        let c = creation(
            ColorSpectrumShape::Ring,
            ColorSpectrumComponents::HueSaturation,
            256.0,
        );
        // Right of the centre is hue 0 (360) at full saturation; below it, a quarter turn.
        let right = red_at(&c, 255, 128);
        assert!(
            right.s > 0.99 && (right.h == 359.0 || right.h == 0.0),
            "{right:?}"
        );
        let below = red_at(&c, 128, 255);
        assert!((below.h - 89.75).abs() < 1.0, "{below:?}");
        // Hue 89.75 is a quarter of 359: straight below, halfway out.
        let centre = ellipse_center(HsvColor::from_hsv(Hsv::new(89.75, 0.5, 1.0), 1.0), &c);
        assert!((centre.dx() - 128.0).abs() < 1e-4, "{centre:?}");
        assert!((centre.dy() - 192.0).abs() < 1e-4, "{centre:?}");
    }

    #[test]
    fn arrows_step_the_component_their_axis_shows() {
        use ColorPickerHsvChannel::*;
        use ColorSpectrumComponents::*;
        assert_eq!(arrow_channel(HueSaturation, true), Hue);
        assert_eq!(arrow_channel(HueSaturation, false), Saturation);
        assert_eq!(arrow_channel(SaturationValue, true), Saturation);
        assert_eq!(arrow_channel(SaturationValue, false), Value);
        assert_eq!(arrow_channel(ValueHue, true), Value);
        assert_eq!(arrow_channel(ValueHue, false), Hue);
    }
}
