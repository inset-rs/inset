//! `RowDefinition`, `ColumnDefinition` and `GridLength` (`dxaml/xcp/core/inc/GridDefinitions.h`, `GridLength.cpp`): what a caller declares, and the per-definition working state `CDefinitionBase` keeps between measure and arrange.

/// XAML `GridUnitType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridUnitType {
    /// Sized to the content.
    Auto,
    /// A fixed length.
    Pixel,
    /// A weighted share of the remaining space.
    Star,
}

/// XAML `GridLength`: `Auto`, a pixel value, or `n*`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridLength {
    pub value: f64,
    pub unit: GridUnitType,
}

impl GridLength {
    /// XAML `Auto`. The value is `XGRIDLENGTH::Default()`, 1.
    pub const AUTO: GridLength = GridLength {
        value: 1.0,
        unit: GridUnitType::Auto,
    };

    /// XAML `*`.
    pub const STAR: GridLength = GridLength::star(1.0);

    /// XAML `Width="40"`.
    pub const fn pixel(value: f64) -> GridLength {
        GridLength {
            value,
            unit: GridUnitType::Pixel,
        }
    }

    /// XAML `Width="2*"`.
    pub const fn star(value: f64) -> GridLength {
        GridLength {
            value,
            unit: GridUnitType::Star,
        }
    }
}

/// XAML `RowDefinition`: `Height` (default `*`), `MinHeight`, `MaxHeight`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowDefinition {
    pub height: GridLength,
    pub min_height: f64,
    pub max_height: f64,
}

impl RowDefinition {
    pub const fn new(height: GridLength) -> RowDefinition {
        RowDefinition {
            height,
            min_height: 0.0,
            max_height: f64::INFINITY,
        }
    }

    /// XAML `MinHeight`.
    pub const fn min_height(mut self, min_height: f64) -> RowDefinition {
        self.min_height = min_height;
        self
    }

    /// XAML `MaxHeight`.
    pub const fn max_height(mut self, max_height: f64) -> RowDefinition {
        self.max_height = max_height;
        self
    }

    pub(crate) fn definition(&self) -> Definition {
        Definition::new(self.height, self.min_height, self.max_height)
    }
}

impl Default for RowDefinition {
    fn default() -> RowDefinition {
        RowDefinition::new(GridLength::STAR)
    }
}

/// XAML `ColumnDefinition`: `Width` (default `*`), `MinWidth`, `MaxWidth`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnDefinition {
    pub width: GridLength,
    pub min_width: f64,
    pub max_width: f64,
}

impl ColumnDefinition {
    pub const fn new(width: GridLength) -> ColumnDefinition {
        ColumnDefinition {
            width,
            min_width: 0.0,
            max_width: f64::INFINITY,
        }
    }

    /// XAML `MinWidth`.
    pub const fn min_width(mut self, min_width: f64) -> ColumnDefinition {
        self.min_width = min_width;
        self
    }

    /// XAML `MaxWidth`.
    pub const fn max_width(mut self, max_width: f64) -> ColumnDefinition {
        self.max_width = max_width;
        self
    }

    pub(crate) fn definition(&self) -> Definition {
        Definition::new(self.width, self.min_width, self.max_width)
    }
}

impl Default for ColumnDefinition {
    fn default() -> ColumnDefinition {
        ColumnDefinition::new(GridLength::STAR)
    }
}

/// `CDefinitionBase`: the user values of a row or column and the sizes the layout passes compute for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Definition {
    pub user_size: GridLength,
    pub user_min_size: f64,
    pub user_max_size: f64,
    pub effective_min_size: f64,
    pub measure_arrange_size: f64,
    pub size_cache: f64,
    pub final_offset: f64,
    pub effective_unit_type: GridUnitType,
}

impl Definition {
    pub fn new(user_size: GridLength, user_min_size: f64, user_max_size: f64) -> Definition {
        Definition {
            user_size,
            user_min_size,
            user_max_size,
            effective_min_size: 0.0,
            measure_arrange_size: 0.0,
            size_cache: 0.0,
            final_offset: 0.0,
            effective_unit_type: GridUnitType::Auto,
        }
    }

    pub fn user_size_type(&self) -> GridUnitType {
        self.user_size.unit
    }

    pub fn is_auto(&self) -> bool {
        self.user_size.unit == GridUnitType::Auto
    }

    /// `GetPreferredSize`.
    pub fn preferred_size(&self) -> f64 {
        if self.effective_unit_type != GridUnitType::Auto
            && self.effective_min_size < self.measure_arrange_size
        {
            self.measure_arrange_size
        } else {
            self.effective_min_size
        }
    }

    /// `UpdateEffectiveMinSize`.
    pub fn update_effective_min_size(&mut self, new_value: f64) {
        self.effective_min_size = self.effective_min_size.max(new_value);
    }
}
