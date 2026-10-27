//! XAML `RangeBase` (`dxaml/xcp/core/core/elements/RangeBase.cpp`, `dxaml/xcp/dxaml/lib/RangeBase_Partial.cpp`): `Minimum`, `Maximum` and `Value`, and the coercion `CRangeBase::SetValue` applies whenever one of them is set. The comparisons the range controls make are `DoubleUtil`'s (`dxaml/xcp/components/base/DoubleUtil.cpp`).

/// `RangeBase::GetDefaultValue2`: `Maximum` defaults to 1; `Minimum` and `Value` to 0.
pub const RANGE_BASE_DEFAULT_MAXIMUM: f64 = 1.0;
/// `RangeBase::GetDefaultValue2`: `SmallChange` defaults to 0.1.
pub const RANGE_BASE_DEFAULT_SMALL_CHANGE: f64 = 0.1;
/// `RangeBase::GetDefaultValue2`: `LargeChange` defaults to 1.
pub const RANGE_BASE_DEFAULT_LARGE_CHANGE: f64 = 1.0;

/// `DoubleUtil::Epsilon`.
const EPSILON: f64 = 1.1102230246251567e-016;

/// `DoubleUtil::AreClose`: within an epsilon proportional to the values.
pub fn are_close(value1: f64, value2: f64) -> bool {
    if value1 == value2 {
        return true;
    }
    let epsilon = (value1.abs() + value2.abs() + 10.0) * EPSILON;
    let delta = value1 - value2;
    -epsilon < delta && epsilon > delta
}

/// `DoubleUtil::LessThan`: strictly less and not close.
pub fn less_than(value1: f64, value2: f64) -> bool {
    value1 < value2 && !are_close(value1, value2)
}

/// `DoubleUtil::LessThanOrClose`.
pub fn less_than_or_close(value1: f64, value2: f64) -> bool {
    value1 < value2 || are_close(value1, value2)
}

/// `DoubleUtil::Fractional`: the part after the decimal point, toward zero.
pub fn fractional(value: f64) -> f64 {
    if value > 0.0 {
        value - value.floor()
    } else {
        value - value.ceil()
    }
}

/// `RangeBase::EnsureValidDoubleValue`: a range property rejects NaN and the infinities (`E_INVALIDARG`, `ERROR_INVALID_DOUBLE_VALUE`).
pub fn ensure_valid_double(value: f64, property: &str) {
    assert!(
        value.is_finite(),
        "RangeBase.{property}: {value} is not a valid double value"
    );
}

/// `Minimum`, `Maximum` and `Value` of a `RangeBase` control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RangeBase {
    pub minimum: f64,
    pub maximum: f64,
    pub value: f64,
}

impl RangeBase {
    pub fn new(minimum: f64, maximum: f64, value: f64) -> RangeBase {
        ensure_valid_double(minimum, "Minimum");
        ensure_valid_double(maximum, "Maximum");
        ensure_valid_double(value, "Value");
        RangeBase {
            minimum,
            maximum,
            value,
        }
    }

    /// `CRangeBase::SetValue` for the three properties together: `Maximum` is kept at or above `Minimum`, and `Value` is clamped between them (`CoerceAndSetValue`).
    pub fn coerced(self) -> RangeBase {
        let maximum = self.maximum.max(self.minimum);
        RangeBase {
            minimum: self.minimum,
            maximum,
            value: self.value.min(maximum).max(self.minimum),
        }
    }

    /// `CRangeBase::CoerceAndSetValue`: what `put_Value(value)` stores.
    pub fn coerce_value(&self, value: f64) -> f64 {
        value.min(self.maximum).max(self.minimum)
    }

    /// `Maximum − Minimum`.
    pub fn span(&self) -> f64 {
        self.maximum - self.minimum
    }

    /// `Slider::UpdateTrackLayout`'s `multiplier`: the share of the track before `value`, 0 for an empty range.
    pub fn fraction_of(&self, value: f64) -> f64 {
        let range = self.span();
        if range <= 0.0 {
            0.0
        } else {
            1.0 - (self.maximum - value) / range
        }
    }
}
