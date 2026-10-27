//! `Common.idl` of ColorPicker (the enumerations and event arguments ColorSpectrum,
//! ColorPickerSlider and ColorPicker share) and `ColorHelpers.cpp`: stepping a colour channel
//! from the keyboard, to the next named colour with Ctrl, and the pixels of the checkered
//! backgrounds behind translucent colours.

use crate::{Hsv, argb, color_display_name, color_from_rgba, hsv_to_rgb};
use inset_embedder::valo::{AlphaType, PixelBuffer, PixelFormat, PixelLayout};
use inset_embedder::{Color, Image};
use inset_foundation::App;

/// XAML `ColorSpectrumShape`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorSpectrumShape {
    /// A square: one component across, the other down.
    #[default]
    Box,
    /// A circle: one component around, the other from the centre out.
    Ring,
}

/// XAML `ColorSpectrumComponents`: the two HSV components a spectrum lays out, the first across
/// a box or around a ring, the second down a box or out from a ring's centre. The third is the
/// one the spectrum blends its images by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorSpectrumComponents {
    HueValue,
    ValueHue,
    #[default]
    HueSaturation,
    SaturationHue,
    SaturationValue,
    ValueSaturation,
}

/// XAML `ColorPickerHsvChannel`: the channel a `ColorPickerSlider` sets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorPickerHsvChannel {
    Hue,
    Saturation,
    #[default]
    Value,
    Alpha,
}

/// XAML `ColorChangedEventArgs`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorChangedEventArgs {
    /// `OldColor`.
    pub old_color: Color,
    /// `NewColor`.
    pub new_color: Color,
}

/// `IncrementDirection`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IncrementDirection {
    Lower,
    Higher,
}

impl IncrementDirection {
    /// The other direction.
    pub(crate) fn reversed(self) -> IncrementDirection {
        match self {
            IncrementDirection::Lower => IncrementDirection::Higher,
            IncrementDirection::Higher => IncrementDirection::Lower,
        }
    }

    /// -1 lower, 1 higher.
    fn sign(self) -> f64 {
        match self {
            IncrementDirection::Lower => -1.0,
            IncrementDirection::Higher => 1.0,
        }
    }
}

/// `IncrementAmount`: an arrow key alone, or with Ctrl.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IncrementAmount {
    Small,
    Large,
}

/// The component of `hsv` that `channel` names.
fn component(hsv: &mut Hsv, channel: ColorPickerHsvChannel) -> &mut f64 {
    match channel {
        ColorPickerHsvChannel::Hue => &mut hsv.h,
        ColorPickerHsvChannel::Saturation => &mut hsv.s,
        ColorPickerHsvChannel::Value => &mut hsv.v,
        ColorPickerHsvChannel::Alpha => unreachable!("alpha is stepped by increment_alpha_channel"),
    }
}

/// `IncrementColorChannel`: a small step moves `channel` by 1 (a degree of hue, a percent of
/// saturation or value) and stops at the bounds, or with `should_wrap` goes to the other bound
/// when it was already at one; a large step goes to the next named colour. The bounds are in
/// degrees for hue and in percent for saturation and value.
pub(crate) fn increment_color_channel(
    original: Hsv,
    channel: ColorPickerHsvChannel,
    direction: IncrementDirection,
    amount: IncrementAmount,
    should_wrap: bool,
    min_bound: f64,
    max_bound: f64,
) -> Hsv {
    if amount == IncrementAmount::Large {
        // The named colours are found in HSV units, so saturation and value bounds go to 0-1.
        let (min_bound, max_bound) = match channel {
            ColorPickerHsvChannel::Hue => (min_bound, max_bound),
            _ => (min_bound / 100.0, max_bound / 100.0),
        };
        return find_next_named_color(
            original,
            channel,
            direction,
            should_wrap,
            min_bound,
            max_bound,
        );
    }
    // Saturation and value go to 0-100 to keep away from rounding issues.
    let mut hsv = Hsv::new(original.h, original.s * 100.0, original.v * 100.0);
    let value = component(&mut hsv, channel);
    let previous = *value;
    *value += direction.sign();
    if *value < min_bound {
        *value = if should_wrap && previous == min_bound {
            max_bound
        } else {
            min_bound
        };
    }
    if *value > max_bound {
        *value = if should_wrap && previous == max_bound {
            min_bound
        } else {
            max_bound
        };
    }
    Hsv::new(hsv.h, hsv.s / 100.0, hsv.v / 100.0)
}

/// The sign of `value`: -1, 0 or 1.
fn sgn(value: f64) -> i32 {
    i32::from(value > 0.0) - i32::from(value < 0.0)
}

/// The name of the colour `hsv` shows.
fn hsv_name(hsv: Hsv) -> &'static str {
    color_display_name(color_from_rgba(hsv_to_rgb(hsv), 1.0))
}

/// `FindNextNamedColor`: steps `channel` in `direction` until the colour has another name, then
/// across that name's range to its far end, and returns the middle of the range. Without
/// `should_wrap` it stops at the bound it reaches; with it, a search that comes back to the
/// start finds no other name and stays where it is.
fn find_next_named_color(
    original: Hsv,
    channel: ColorPickerHsvChannel,
    direction: IncrementDirection,
    should_wrap: bool,
    min_bound: f64,
    max_bound: f64,
) -> Hsv {
    let mut new_hsv = original;
    let original_name = hsv_name(original);
    let mut new_name = original_name;
    let mut original_copy = original;
    let original_value = *component(&mut original_copy, channel);
    let increment = match channel {
        ColorPickerHsvChannel::Hue => 1.0,
        _ => 0.01,
    };
    let mut should_find_mid_point = true;
    while new_name == original_name {
        let value = component(&mut new_hsv, channel);
        let previous = *value;
        *value += direction.sign() * increment;
        let mut just_wrapped = false;
        if *value > max_bound {
            if should_wrap {
                *value = min_bound;
                just_wrapped = true;
            } else {
                *value = max_bound;
                should_find_mid_point = false;
                break;
            }
        } else if *value < min_bound {
            if should_wrap {
                *value = max_bound;
                just_wrapped = true;
            } else {
                *value = min_bound;
                should_find_mid_point = false;
                break;
            }
        }
        // Wrapped all the way back past the start without finding another name.
        if !just_wrapped
            && previous != original_value
            && sgn(*value - original_value) != sgn(previous - original_value)
        {
            should_find_mid_point = false;
            break;
        }
        new_name = hsv_name(new_hsv);
    }
    if should_find_mid_point {
        new_hsv = middle_of_named_range(
            new_hsv,
            new_name,
            channel,
            direction,
            should_wrap,
            min_bound,
            max_bound,
            increment,
        );
    }
    new_hsv
}

/// The second half of `FindNextNamedColor`: from `start`, the first colour named `name`, steps
/// on to where the name changes again and returns the middle of the two, on a whole step and
/// within the bounds.
#[allow(clippy::too_many_arguments)]
fn middle_of_named_range(
    start: Hsv,
    name: &str,
    channel: ColorPickerHsvChannel,
    direction: IncrementDirection,
    should_wrap: bool,
    min_bound: f64,
    max_bound: f64,
    increment: f64,
) -> Hsv {
    let mut start_hsv = start;
    let mut current = start;
    let mut start_end_offset = 0.0;
    let wrap_increment = match channel {
        ColorPickerHsvChannel::Hue => 360.0,
        _ => 1.0,
    };
    let mut current_name = name;
    while current_name == name {
        let value = component(&mut current, channel);
        *value += direction.sign() * increment;
        if *value > max_bound {
            if should_wrap {
                *value = min_bound;
                start_end_offset = max_bound - min_bound;
            } else {
                *value = max_bound;
                break;
            }
        } else if *value < min_bound {
            if should_wrap {
                *value = max_bound;
                start_end_offset = min_bound - max_bound;
            } else {
                *value = min_bound;
                break;
            }
        }
        current_name = hsv_name(current);
    }
    let start_value = *component(&mut start_hsv, channel);
    let current_value = *component(&mut current, channel);
    let mut new_hsv = start;
    let value = component(&mut new_hsv, channel);
    *value = (start_value + current_value + start_end_offset) / 2.0;
    // Halving may have landed halfway through a step; that leftover goes.
    let mut leftover = value.abs();
    while leftover > increment {
        leftover -= increment;
    }
    *value -= leftover;
    while *value < min_bound {
        *value += wrap_increment;
    }
    while *value > max_bound {
        *value -= wrap_increment;
    }
    new_hsv
}

/// `IncrementAlphaChannel`: a small step moves alpha by 1 percent; a large one goes to the next
/// multiple of 10 percent in `direction`. Past a bound it stops there, or with `should_wrap` goes
/// to the other one. The bounds are in percent; alpha is from 0 to 1.
pub(crate) fn increment_alpha_channel(
    original: f64,
    direction: IncrementDirection,
    amount: IncrementAmount,
    should_wrap: bool,
    min_bound: f64,
    max_bound: f64,
) -> f64 {
    const SMALL: f64 = 1.0;
    const LARGE: f64 = 10.0;
    let mut alpha = original * 100.0;
    alpha = match (amount, direction) {
        (IncrementAmount::Small, _) => alpha + direction.sign() * SMALL,
        (IncrementAmount::Large, IncrementDirection::Lower) => {
            ((alpha - LARGE) / LARGE).ceil() * LARGE
        }
        (IncrementAmount::Large, IncrementDirection::Higher) => {
            ((alpha + LARGE) / LARGE).floor() * LARGE
        }
    };
    if alpha < min_bound {
        alpha = if should_wrap { max_bound } else { min_bound };
    }
    if alpha > max_bound {
        alpha = if should_wrap { min_bound } else { max_bound };
    }
    alpha / 100.0
}

/// `CheckerSize`: the side of one square of the checkered background, in pixels.
const CHECKER_SIZE: u32 = 4;

/// The pixels of `CreateCheckeredBackgroundAsync`: `width` by `height` premultiplied BGRA,
/// transparent and `checker_color` squares alternating across and down.
pub(crate) fn checkered_background_pixels(
    width: u32,
    height: u32,
    checker_color: Color,
) -> Vec<u8> {
    let color = argb(checker_color);
    let premultiplied = |channel: u8| (u32::from(channel) * u32::from(color.a) / 255) as u8;
    let checker = [
        premultiplied(color.b),
        premultiplied(color.g),
        premultiplied(color.r),
        color.a,
    ];
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let blank = (x / CHECKER_SIZE + y / CHECKER_SIZE).is_multiple_of(2);
            pixels.extend(if blank { [0; 4] } else { checker });
        }
    }
    pixels
}

/// `CreateBitmapFromPixelData` and `CreateSurfaceFromPixelData`: an image of `width` by `height`
/// premultiplied BGRA pixels, from the platform; `None` while it has no renderer.
pub(crate) fn image_from_pixel_data(
    app: &App,
    width: u32,
    height: u32,
    bgra: Vec<u8>,
) -> Option<Image> {
    if width == 0 || height == 0 {
        return None;
    }
    let layout = PixelLayout::packed(
        [width, height],
        PixelFormat::Bgra8,
        AlphaType::Premultiplied,
    );
    let pixels = PixelBuffer::new(layout, bgra).ok()?;
    app.platform().import_pixels(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Rgb, rgb_to_hsv};

    fn bytes(hsv: Hsv) -> (u8, u8, u8) {
        let color = color_from_rgba(hsv_to_rgb(hsv), 1.0);
        let bytes = argb(color);
        (bytes.r, bytes.g, bytes.b)
    }

    /// `CanUseKeyboardToInteractWithColorSpectrum`: from red, the arrows and Ctrl+arrows step
    /// hue and saturation with wrapping to these colours.
    #[test]
    fn spectrum_keys_reach_the_interaction_test_colours() {
        use ColorPickerHsvChannel::{Hue, Saturation};
        use IncrementAmount::{Large, Small};
        use IncrementDirection::{Higher, Lower};
        let steps = [
            (Hue, Higher, Small, (255, 4, 0)),
            (Hue, Lower, Small, (255, 0, 0)),
            (Hue, Lower, Small, (255, 0, 4)),
            (Hue, Higher, Small, (255, 0, 0)),
            (Hue, Higher, Large, (255, 119, 0)),
            (Hue, Lower, Large, (255, 0, 13)),
            (Hue, Lower, Large, (255, 0, 170)),
            (Hue, Higher, Large, (255, 0, 13)),
            (Saturation, Lower, Small, (255, 3, 15)),
            (Saturation, Higher, Small, (255, 0, 13)),
            (Saturation, Higher, Small, (255, 255, 255)),
            (Saturation, Lower, Small, (255, 0, 13)),
            (Saturation, Lower, Large, (255, 153, 158)),
            (Saturation, Higher, Large, (255, 38, 49)),
            (Saturation, Higher, Large, (255, 242, 243)),
            (Saturation, Lower, Large, (255, 41, 52)),
            (Saturation, Lower, Large, (255, 153, 158)),
            (Saturation, Higher, Large, (255, 38, 49)),
        ];
        let mut hsv = Hsv::new(0.0, 1.0, 1.0);
        for (channel, direction, amount, expected) in steps {
            let max = if channel == Hue { 359.0 } else { 100.0 };
            hsv = increment_color_channel(hsv, channel, direction, amount, true, 0.0, max);
            assert_eq!(bytes(hsv), expected, "{channel:?} {direction:?} {amount:?}");
        }
    }

    /// `CanUseKeyboardToInteractWithSliders`: the value slider steps without wrapping.
    #[test]
    fn value_slider_keys_reach_the_interaction_test_colours() {
        use IncrementAmount::{Large, Small};
        use IncrementDirection::{Higher, Lower};
        let steps = [
            (Lower, Small, (252, 38, 49)),
            (Higher, Small, (255, 38, 49)),
            (Higher, Small, (255, 38, 49)),
            (Lower, Large, (102, 15, 20)),
            (Lower, Large, (15, 2, 3)),
            (Lower, Large, (0, 0, 0)),
            (Higher, Large, (105, 16, 20)),
            (Higher, Large, (214, 32, 41)),
            (Higher, Large, (255, 38, 49)),
        ];
        let mut hsv = rgb_to_hsv(Rgb::new(1.0, 38.0 / 255.0, 49.0 / 255.0));
        for (direction, amount, expected) in steps {
            hsv = increment_color_channel(
                hsv,
                ColorPickerHsvChannel::Value,
                direction,
                amount,
                false,
                0.0,
                100.0,
            );
            assert_eq!(bytes(hsv), expected, "{direction:?} {amount:?}");
        }
    }

    /// The alpha slider: 1 percent a step, and with Ctrl to multiples of 10.
    #[test]
    fn alpha_steps_snap_to_tens_with_ctrl() {
        use IncrementAmount::{Large, Small};
        use IncrementDirection::{Higher, Lower};
        let byte = |alpha: f64| (alpha * 255.0).round() as i32;
        let mut alpha = 1.0;
        for (direction, amount, expected) in [
            (Lower, Small, 252),
            (Higher, Small, 255),
            (Higher, Small, 255),
            (Lower, Large, 230),
            (Higher, Small, 232),
            (Lower, Large, 230),
            (Higher, Large, 255),
        ] {
            alpha = increment_alpha_channel(alpha, direction, amount, false, 0.0, 100.0);
            assert_eq!(byte(alpha), expected);
        }
    }

    #[test]
    fn the_checkerboard_alternates_every_four_pixels() {
        let pixels = checkered_background_pixels(8, 8, Color::from_argb(25, 0, 0, 0));
        let at = |x: usize, y: usize| &pixels[(y * 8 + x) * 4..(y * 8 + x) * 4 + 4];
        assert_eq!(at(0, 0), [0, 0, 0, 0]);
        assert_eq!(at(4, 0), [0, 0, 0, 25]);
        assert_eq!(at(4, 4), [0, 0, 0, 0]);
        let light = checkered_background_pixels(8, 8, Color::from_argb(25, 255, 255, 255));
        assert_eq!(&light[16..20], [25, 25, 25, 25]);
    }
}
