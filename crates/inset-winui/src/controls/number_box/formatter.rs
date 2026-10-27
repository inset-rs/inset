//! The number formatting NumberBox leaves to `Windows.Globalization.NumberFormatting`: the
//! interfaces of its `NumberFormatter` property, the `DecimalFormatter` it creates by default,
//! and the `SignificantDigitsNumberRounder` it rounds a value with before formatting it.

use std::any::Any;

/// `NumberFormatter`: what NumberBox's `NumberFormatter` property holds, an object that is both
/// an `INumberFormatter2` and an `INumberParser` (`NumberBox::ValidateNumberFormatter`).
/// NumberBox formats its value with `format_double` and reads what is typed with
/// `parse_double`.
pub trait NumberFormatter {
    /// `INumberFormatter2::FormatDouble`: the text shown for `value`.
    fn format_double(&self, value: f64) -> String;

    /// `INumberParser::ParseDouble`: the number `text` spells, or `None` when it spells none.
    fn parse_double(&self, text: &str) -> Option<f64>;
}

/// A [`NumberFormatter`] NumberBox can tell from another: every `NumberFormatter` with
/// `PartialEq` is one, so an equal formatter built again leaves the box's text alone.
pub trait ComparableNumberFormatter: NumberFormatter {
    /// The formatter as `Any`, to compare it with one of the same type.
    fn as_any(&self) -> &dyn Any;

    /// Whether `other` is a formatter of the same type and value.
    fn same_as(&self, other: &dyn ComparableNumberFormatter) -> bool;
}

impl<F: NumberFormatter + PartialEq + 'static> ComparableNumberFormatter for F {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn same_as(&self, other: &dyn ComparableNumberFormatter) -> bool {
        other.as_any().downcast_ref::<F>() == Some(self)
    }
}

/// `DecimalFormatter` as `NumberBox::GetRegionalSettingsAwareDecimalFormatter` makes it, in the
/// en-US conventions: a value in its shortest exact decimal form, with at least
/// `integer_digits` digits before a `.` and `fraction_digits` after it, without grouping, and a
/// `-` before a negative value but not before zero. It reads an optional `-`, digits and at
/// most one `.`; grouping separators, exponents and other signs are not numbers to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecimalFormatter {
    /// `IntegerDigits`: the fewest digits before the decimal point, padded with zeros.
    pub integer_digits: usize,

    /// `FractionDigits`: the fewest digits after the decimal point, padded with zeros.
    pub fraction_digits: usize,
}

impl Default for DecimalFormatter {
    /// NumberBox's: `IntegerDigits(1)` and `FractionDigits(0)`.
    fn default() -> Self {
        Self {
            integer_digits: 1,
            fraction_digits: 0,
        }
    }
}

impl NumberFormatter for DecimalFormatter {
    fn format_double(&self, value: f64) -> String {
        if value.is_nan() {
            return "NaN".to_owned();
        }
        if value.is_infinite() {
            return if value > 0.0 { "∞" } else { "-∞" }.to_owned();
        }
        // Rust's `Display` writes the shortest decimal that reads back as `value`, never with an
        // exponent.
        let digits = value.abs().to_string();
        let (integer, fraction) = digits.split_once('.').unwrap_or((&digits, ""));
        let integer = integer.trim_start_matches('0');
        let mut text = String::new();
        if value < 0.0 {
            text.push('-');
        }
        let integer_digits = if integer.is_empty() && fraction.is_empty() {
            self.integer_digits.max(1)
        } else {
            self.integer_digits
        };
        text.extend(std::iter::repeat_n(
            '0',
            integer_digits.saturating_sub(integer.len()),
        ));
        text.push_str(integer);
        let padding = self.fraction_digits.saturating_sub(fraction.len());
        if !fraction.is_empty() || padding > 0 {
            text.push('.');
            text.push_str(fraction);
            text.extend(std::iter::repeat_n('0', padding));
        }
        text
    }

    fn parse_double(&self, text: &str) -> Option<f64> {
        let unsigned = text.strip_prefix('-').unwrap_or(text);
        let mut digits = 0;
        let mut points = 0;
        for character in unsigned.chars() {
            match character {
                '0'..='9' => digits += 1,
                '.' => points += 1,
                _ => return None,
            }
        }
        if digits == 0 || points > 1 {
            return None;
        }
        text.parse().ok()
    }
}

/// `m_displayRounder.SignificantDigits(10)` in `NumberBox::OnApplyTemplate`.
const DISPLAY_SIGNIFICANT_DIGITS: usize = 10;

/// `m_displayRounder.RoundDouble`: NumberBox's `SignificantDigitsNumberRounder`, which keeps 10
/// significant digits so that floating-point noise is not shown. A value that lies halfway
/// rounds toward positive infinity, as `RoundHalfUp` does.
pub(super) fn round_for_display(value: f64) -> f64 {
    round_to_significant_digits(value, DISPLAY_SIGNIFICANT_DIGITS)
}

/// `value` with `digits` significant digits, halfway values rounding toward positive infinity.
fn round_to_significant_digits(value: f64, digits: usize) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    // One digit more than kept, in Rust's correctly rounded formatting, shows whether `value`
    // is the decimal halfway between two kept values.
    let one_more = format!("{value:.digits$e}");
    let halfway = one_more
        .split_once('e')
        .is_some_and(|(mantissa, _)| mantissa.ends_with('5'))
        && one_more.parse::<f64>() == Ok(value);
    // Formatting rounds halfway values to even; the next value up rounds them up instead.
    let value = if halfway { value.next_up() } else { value };
    let kept = digits - 1;
    format!("{value:.kept$e}")
        .parse()
        .expect("Rust reads back the numbers it formats")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_formatter_writes_the_shortest_decimal_without_grouping() {
        let formatter = DecimalFormatter::default();
        assert_eq!(formatter.format_double(0.0), "0");
        assert_eq!(formatter.format_double(-0.0), "0");
        assert_eq!(formatter.format_double(3.0), "3");
        assert_eq!(formatter.format_double(-12.5), "-12.5");
        assert_eq!(formatter.format_double(1234567.0), "1234567");
        assert_eq!(formatter.format_double(0.25), "0.25");
        assert_eq!(formatter.format_double(1e-7), "0.0000001");
    }

    #[test]
    fn integer_and_fraction_digits_pad_with_zeros() {
        let formatter = DecimalFormatter {
            integer_digits: 3,
            fraction_digits: 2,
        };
        assert_eq!(formatter.format_double(5.0), "005.00");
        assert_eq!(formatter.format_double(-0.125), "-000.125");
        let bare = DecimalFormatter {
            integer_digits: 0,
            fraction_digits: 0,
        };
        assert_eq!(bare.format_double(0.5), ".5");
        assert_eq!(bare.format_double(0.0), "0");
    }

    #[test]
    fn parsing_takes_a_minus_digits_and_one_point() {
        let formatter = DecimalFormatter::default();
        assert_eq!(formatter.parse_double("42"), Some(42.0));
        assert_eq!(formatter.parse_double("-3.5"), Some(-3.5));
        assert_eq!(formatter.parse_double(".5"), Some(0.5));
        assert_eq!(formatter.parse_double("5."), Some(5.0));
        for text in [
            "", "-", ".", "1.2.3", "1,000", "1e3", "+1", " 1", "abc", "inf", "NaN",
        ] {
            assert_eq!(formatter.parse_double(text), None, "{text:?}");
        }
    }

    #[test]
    fn the_display_rounder_hides_floating_point_noise() {
        assert_eq!(round_for_display(0.1 + 0.2), 0.3);
        assert_eq!(round_for_display(1.0 / 3.0), 0.3333333333);
        assert_eq!(round_for_display(12345678901234.0), 12345678900000.0);
        assert_eq!(round_for_display(-2.0 / 3.0), -0.6666666667);
        assert!(round_for_display(f64::NAN).is_nan());
        assert_eq!(round_for_display(0.0), 0.0);
    }

    #[test]
    fn a_halfway_value_rounds_toward_positive_infinity() {
        assert_eq!(round_for_display(1234567890.5), 1234567891.0);
        assert_eq!(round_for_display(-1234567890.5), -1234567890.0);
        assert_eq!(round_for_display(1234567891.5), 1234567892.0);
    }
}
