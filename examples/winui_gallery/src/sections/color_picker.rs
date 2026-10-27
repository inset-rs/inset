//! ColorPicker: a picker with its shape, opacity, More button and orientation as options, a
//! previous colour, and a ColorSpectrum of its own, with the colours they keep.

use crate::{column, example, example_row, label, section};
use inset_embedder::Color;
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
// `Orientation` is also an `inset_widgets` name (the media query's); the picker's is XAML's.
use inset_winui::Orientation;
use inset_winui::*;

/// Builds the ColorPicker feature page.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("ColorPicker", resources, ColorPickerDemo.into_widget())
}

/// The ColorPicker examples.
#[derive(Debug)]
struct ColorPickerDemo;

/// The colours the examples keep, and the picker's options.
struct ColorPickerDemoState {
    /// Native widget state.
    state: StateData<ColorPickerDemo>,

    /// The picker's colour.
    color: Color,

    /// The picker's previous colour, kept with the button.
    previous: Option<Color>,

    /// A ring spectrum instead of a box.
    ring: bool,

    /// Opacity can be chosen.
    alpha: bool,

    /// The entries hide behind the More button.
    more_button: bool,

    /// The `Horizontal` orientation.
    horizontal: bool,

    /// The last `ColorChanged`.
    event: String,

    /// The standalone spectrum's colour.
    spectrum: HsvColor,
}

impl StatefulWidget for ColorPickerDemo {
    type State = ColorPickerDemoState;

    fn create_state(&self) -> Self::State {
        ColorPickerDemoState {
            state: StateData::new(),
            color: Color::from_argb(255, 255, 38, 49),
            previous: None,
            ring: false,
            alpha: false,
            more_button: false,
            horizontal: false,
            event: "No color changed yet".to_owned(),
            spectrum: HsvColor::from_color(Color::from_argb(255, 0, 120, 212)),
        }
    }
}

/// `#AARRGGBB`.
fn hex(color: Color) -> String {
    format!("#{:08X}", color.to_argb32())
}

impl ColorPickerDemoState {
    /// A check box for an option, stored with `store`.
    fn option(
        self: Handle<Self>,
        text: &str,
        value: bool,
        store: fn(&mut ColorPickerDemoState, bool),
    ) -> WidgetRef {
        CheckBox::new(Some(value), move |app, checked| {
            self.set_state(app, |state| store(state, checked == Some(true)))
        })
        .content(Text::new(text))
        .into_widget()
    }
}

impl State for ColorPickerDemoState {
    type Widget = ColorPickerDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let secondary = r.text_fill_color_secondary;
        let s = app.get(self);
        let (color, previous, event) = (s.color, s.previous, s.event.clone());
        let (ring, alpha, more_button, horizontal) = (s.ring, s.alpha, s.more_button, s.horizontal);
        let spectrum = s.spectrum;
        let options = example_row(
            vec![
                self.option("Ring spectrum", ring, |s, v| s.ring = v),
                self.option("Opacity", alpha, |s, v| s.alpha = v),
                self.option("More button", more_button, |s, v| s.more_button = v),
                self.option("Horizontal", horizontal, |s, v| s.horizontal = v),
            ],
            12.0,
        );
        let mut picker = ColorPicker::new(color, move |app, args| {
            self.set_state(app, |state| {
                state.color = args.new_color;
                state.event = format!(
                    "ColorChanged: {} → {}",
                    hex(args.old_color),
                    hex(args.new_color)
                );
            })
        })
        .color_spectrum_shape(if ring {
            ColorSpectrumShape::Ring
        } else {
            ColorSpectrumShape::Box
        })
        .is_alpha_enabled(alpha)
        .is_more_button_visible(more_button)
        .orientation(if horizontal {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        });
        if let Some(previous) = previous {
            picker = picker.previous_color(previous);
        }
        let keep_previous = Listener::new(move |app| {
            self.set_state(app, |state| state.previous = Some(state.color))
        });
        let spectrum_widget = SizedBox::new().width(200.0).height(200.0).child(
            ColorSpectrum::new(spectrum, move |app, args| {
                self.set_state(app, |state| state.spectrum = args.hsv_color)
            })
            .shape(ColorSpectrumShape::Ring)
            .components(ColorSpectrumComponents::SaturationValue),
        );
        column(
            vec![
                example(
                    "A color picker",
                    "Pick on the spectrum or the sliders, or type the channels or the hex value; \
                     the arrow keys step a channel, and with Ctrl go to the next named color.",
                    &r,
                    column(
                        vec![
                            options,
                            picker.into_widget(),
                            Button::text("Keep as previous color", keep_previous).into_widget(),
                            label(event, TextBlockStyle::Body, secondary),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "A color spectrum",
                    "A ring of saturation around and value outwards, at the hue of the color.",
                    &r,
                    column(
                        vec![
                            spectrum_widget.into_widget(),
                            label(
                                format!("Color {}", hex(spectrum.to_color())),
                                TextBlockStyle::Body,
                                secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
            ],
            20.0,
        )
    }
}
