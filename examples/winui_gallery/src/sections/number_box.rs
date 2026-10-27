//! NumberBox: computed expressions, inline and compact spin buttons, a range with its
//! validation, a formatter of its own and a disabled box, with the values they keep.

use crate::{column, example, example_row, label, section};
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;

/// Builds the NumberBox feature page.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("NumberBox", resources, NumberBoxDemo.into_widget())
}

/// Formats a value to two decimals after rounding it to the nearest quarter, as WinUI Gallery's
/// `IncrementNumberRounder` sample does; the value itself keeps what was typed.
#[derive(PartialEq)]
struct QuarterFormatter;

impl NumberFormatter for QuarterFormatter {
    fn format_double(&self, value: f64) -> String {
        format!("{:.2}", (value * 4.0).round() / 4.0)
    }

    fn parse_double(&self, text: &str) -> Option<f64> {
        DecimalFormatter::default().parse_double(text)
    }
}

/// The NumberBox examples.
#[derive(Debug)]
struct NumberBoxDemo;

/// The values the examples keep, and their options.
struct NumberBoxDemoState {
    /// Native widget state.
    state: StateData<NumberBoxDemo>,

    /// The computed expression's value.
    expression: f64,

    /// The spin buttons example's value.
    spin: f64,

    /// Where that example's spin buttons are.
    placement: NumberBoxSpinButtonPlacementMode,

    /// Whether that example's steps wrap at its bounds.
    wrap: bool,

    /// The percentage's value.
    percentage: f64,

    /// Whether the percentage overwrites invalid input.
    overwrite: bool,

    /// The amount formatted to quarters.
    amount: f64,

    /// The last `ValueChanged`.
    event: String,
}

impl StatefulWidget for NumberBoxDemo {
    type State = NumberBoxDemoState;

    fn create_state(&self) -> Self::State {
        NumberBoxDemoState {
            state: StateData::new(),
            expression: f64::NAN,
            spin: 0.0,
            placement: NumberBoxSpinButtonPlacementMode::Inline,
            wrap: false,
            percentage: 50.0,
            overwrite: true,
            amount: 1.5,
            event: "No value changed yet".to_owned(),
        }
    }
}

/// The value as the labels show it.
fn shown(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_owned()
    } else {
        value.to_string()
    }
}

impl NumberBoxDemoState {
    /// A `ValueChanged` handler that stores the new value with `store` and records the event.
    fn changed(
        self: Handle<Self>,
        store: fn(&mut NumberBoxDemoState, f64),
    ) -> impl Fn(&mut App, NumberBoxValueChangedEventArgs) + 'static {
        move |app, args| {
            self.set_state(app, |state| {
                store(state, args.new_value);
                state.event = format!(
                    "ValueChanged: {} → {}",
                    shown(args.old_value),
                    shown(args.new_value)
                );
            })
        }
    }
}

impl State for NumberBoxDemoState {
    type Widget = NumberBoxDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let secondary = r.text_fill_color_secondary;
        let s = app.get(self);
        let (expression, spin, placement, wrap) = (s.expression, s.spin, s.placement, s.wrap);
        let (percentage, overwrite, amount) = (s.percentage, s.overwrite, s.amount);
        let event = s.event.clone();
        let next_placement = Listener::new(move |app| {
            self.set_state(app, |state| {
                state.placement = match state.placement {
                    NumberBoxSpinButtonPlacementMode::Inline => {
                        NumberBoxSpinButtonPlacementMode::Compact
                    }
                    NumberBoxSpinButtonPlacementMode::Compact => {
                        NumberBoxSpinButtonPlacementMode::Hidden
                    }
                    NumberBoxSpinButtonPlacementMode::Hidden => {
                        NumberBoxSpinButtonPlacementMode::Inline
                    }
                }
            })
        });
        let width = |child: NumberBox| SizedBox::new().width(240.0).child(child).into_widget();
        column(
            vec![
                example(
                    "Expressions",
                    "Type arithmetic such as 1 + 2^2 and press Enter or leave the box.",
                    &r,
                    column(
                        vec![
                            width(
                                NumberBox::new(expression, self.changed(|s, v| s.expression = v))
                                    .header(Text::new("Enter an expression:"))
                                    .placeholder_text("1 + 2^2")
                                    .accepts_expression(true),
                            ),
                            label(
                                format!("Expression value: {}", shown(expression)),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "Spin buttons",
                    "The arrows, the wheel and the spin buttons step by 10; Page Up and Page Down by 100.",
                    &r,
                    column(
                        vec![
                            example_row(
                                vec![
                                    Button::text(
                                        format!("Spin buttons: {placement:?}"),
                                        next_placement,
                                    )
                                    .into_widget(),
                                    CheckBox::new(Some(wrap), move |app, value| {
                                        self.set_state(app, |s| s.wrap = value == Some(true))
                                    })
                                    .content(Text::new("Wrap"))
                                    .into_widget(),
                                ],
                                16.0,
                            ),
                            width(
                                NumberBox::new(spin, self.changed(|s, v| s.spin = v))
                                    .header(Text::new("Enter an integer:"))
                                    .placeholder_text("0 to 500")
                                    .minimum(0.0)
                                    .maximum(500.0)
                                    .small_change(10.0)
                                    .large_change(100.0)
                                    .spin_button_placement_mode(placement)
                                    .is_wrap_enabled(wrap),
                            ),
                            label(
                                format!("Integer value: {}", shown(spin)),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "Range and validation",
                    "Values beyond 0 or 100 go to the bound; text that is not a number is replaced unless overwriting is off.",
                    &r,
                    column(
                        vec![
                            CheckBox::new(Some(overwrite), move |app, value| {
                                self.set_state(app, |s| s.overwrite = value == Some(true))
                            })
                            .content(Text::new("Overwrite invalid input"))
                            .into_widget(),
                            width(
                                NumberBox::new(percentage, self.changed(|s, v| s.percentage = v))
                                    .header(Text::new("Enter a percentage:"))
                                    .placeholder_text("0 to 100")
                                    .description(Text::new("Between 0 and 100"))
                                    .minimum(0.0)
                                    .maximum(100.0)
                                    .validation_mode(if overwrite {
                                        NumberBoxValidationMode::InvalidInputOverwritten
                                    } else {
                                        NumberBoxValidationMode::Disabled
                                    }),
                            ),
                            label(
                                format!("Percentage value: {}", shown(percentage)),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "Formatting",
                    "The amount shows two decimals, rounded to the nearest quarter, and steps by 0.25.",
                    &r,
                    column(
                        vec![
                            width(
                                NumberBox::new(amount, self.changed(|s, v| s.amount = v))
                                    .header(Text::new("Enter a dollar amount:"))
                                    .placeholder_text("0.00")
                                    .small_change(0.25)
                                    .spin_button_placement_mode(
                                        NumberBoxSpinButtonPlacementMode::Compact,
                                    )
                                    .number_formatter(QuarterFormatter),
                            ),
                            label(
                                format!("Amount value: {}", shown(amount)),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "Disabled",
                    "",
                    &r,
                    width(
                        NumberBox::new(42.0, |_, _| {})
                            .header(Text::new("Disabled"))
                            .placeholder_text("Disabled")
                            .spin_button_placement_mode(NumberBoxSpinButtonPlacementMode::Inline)
                            .is_enabled(false),
                    ),
                ),
                label(event, TextBlockStyle::Caption, secondary),
            ],
            20.0,
        )
    }
}
