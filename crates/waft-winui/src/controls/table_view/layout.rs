//! `TableView::ResolveColumnWidths` (`controls/dev/TableView/TableView_Layout.cpp`), from the widths the cell panels measured: every visible column's width from its `Width`, the widest realized cell of an `Auto` column, and the viewport a star column shares.
//!
//! Fixed columns (pixel and `Auto`) are sized first; star columns then split what the viewport has left, a column that would pass its `MinWidth` or `MaxWidth` fixed at that bound and the rest dividing again. Widths are snapped to device pixels as XAML's layout rounding does.

use super::column::DEFAULT_WIDTH;
use super::*;
use crate::{GridLength, GridUnitType};

/// What the resolution reads of one column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ColumnSizing {
    pub(crate) width: GridLength,
    pub(crate) min_width: f64,
    /// Whether `MinWidth` was set, which a `0*` column honours.
    pub(crate) min_width_set: bool,
    pub(crate) max_width: f64,
    pub(crate) visible: bool,
    /// The column's width before this resolution, which a column the resolution skips keeps.
    pub(crate) actual_width: f64,
    /// The widest measured cell of the column, header included when shown; 0 before any is measured.
    pub(crate) measured: f64,
}

/// `ResolveColumnWidths`: each column's resolved width. `viewport` is the body's width, `None` before it is known; `scale` the device pixel ratio widths are rounded to.
pub(crate) fn resolve_column_widths(
    columns: &[ColumnSizing],
    viewport: Option<f64>,
    scale: f64,
) -> Vec<f64> {
    let round = |value: f64| {
        if scale > 0.0 {
            (value * scale).round() / scale
        } else {
            value
        }
    };
    let ceil = |value: f64| {
        if scale > 0.0 {
            (value * scale).ceil() / scale
        } else {
            value
        }
    };
    let mut widths: Vec<f64> = columns.iter().map(|column| column.actual_width).collect();
    let mut fixed_total = 0.0;
    let mut star_columns = Vec::new();
    for (index, column) in columns.iter().enumerate() {
        if !column.visible {
            continue;
        }
        let lo = column.min_width;
        let hi = lo.max(column.max_width);
        match column.width.unit {
            GridUnitType::Star => star_columns.push(index),
            GridUnitType::Auto => {
                let desired = if column.measured > 0.0 {
                    column.measured
                } else {
                    DEFAULT_WIDTH
                };
                // Up rather than to the nearest pixel: a cell narrower than its content by a
                // fraction of a pixel ellipsizes it, which XAML's whole-pixel text measure never does.
                let resolved = ceil(desired).clamp(lo, hi);
                widths[index] = resolved;
                fixed_total += resolved;
            }
            GridUnitType::Pixel => {
                let resolved = round(column.width.value.clamp(lo, hi));
                widths[index] = resolved;
                fixed_total += resolved;
            }
        }
    }
    // Star needs a finite viewport to divide; before one is known they keep their provisional width.
    let Some(viewport) = viewport.filter(|viewport| *viewport > 0.0 && viewport.is_finite()) else {
        return widths;
    };
    let factor = |index: usize| columns[index].width.value.max(0.0);
    let bounds = |index: usize| {
        let column = &columns[index];
        // `MinWidthForStarFactor`: a `0*` column gets no share unless its MinWidth was set.
        let lo = if factor(index) > 0.0 || column.min_width_set {
            column.min_width
        } else {
            0.0
        };
        (lo, lo.max(column.max_width))
    };
    let mut available = (round(viewport) - fixed_total).max(0.0);
    let mut pool = star_columns;
    let mut adjusted = true;
    while adjusted && !pool.is_empty() {
        adjusted = false;
        let total: f64 = pool.iter().map(|&index| factor(index)).sum();
        let unit = if total > 0.0 { available / total } else { 0.0 };
        for position in 0..pool.len() {
            let index = pool[position];
            let desired = unit * factor(index);
            let (lo, hi) = bounds(index);
            let clamped = desired.clamp(lo, hi);
            if clamped != desired {
                widths[index] = round(clamped);
                available -= clamped;
                pool.remove(position);
                adjusted = true;
                break;
            }
        }
    }
    let total: f64 = pool.iter().map(|&index| factor(index)).sum();
    let unit = if total > 0.0 {
        available.max(0.0) / total
    } else {
        0.0
    };
    for index in pool {
        let (lo, hi) = bounds(index);
        widths[index] = round((unit * factor(index)).clamp(lo, hi));
    }
    widths
}

// MARK: The table's column widths

impl<T: 'static> TableViewState<T> {
    pub(super) fn measurements(self: Handle<Self>, app: &App) -> Rc<ColumnMeasurements> {
        app.get(self)
            .measurements
            .clone()
            .expect("made in `init_state`")
    }

    pub(super) fn sizing(
        self: Handle<Self>,
        app: &App,
        columns: &[Handle<TableViewColumn<T>>],
    ) -> Vec<ColumnSizing> {
        let measurements = self.measurements(app);
        columns
            .iter()
            .map(|&column| ColumnSizing {
                width: column.width(app),
                min_width: column.min_width(app),
                min_width_set: column.min_width_set(app),
                max_width: column.max_width(app),
                visible: column.visible(app),
                actual_width: column.actual_width(app),
                measured: measurements.measured(column.id()),
            })
            .collect()
    }

    /// `ResolveColumnWidths`, writing each column's `ActualWidth`. Answers whether any changed.
    pub(super) fn resolve_column_widths(
        self: Handle<Self>,
        app: &mut App,
        columns: &[Handle<TableViewColumn<T>>],
    ) -> bool {
        let sizing = self.sizing(app, columns);
        let state = app.get(self);
        let widths = resolve_column_widths(&sizing, state.viewport, state.scale);
        let mut changed = false;
        for (column, width) in columns.iter().zip(widths) {
            if (column.actual_width(app) - width).abs() > 0.0001 {
                column.set_actual_width(app, width);
                changed = true;
            }
        }
        changed
    }

    /// `RequestColumnWidthResolve`: a panel's measured widths changed; the widths are resolved again once the frame is over.
    pub(super) fn request_resolve(self: Handle<Self>, app: &mut App) {
        if !self.alive(app) || app.get(self).resolve_queued {
            return;
        }
        app.get_mut(self).resolve_queued = true;
        self.after_frame(app, move |app| {
            app.get_mut(self).resolve_queued = false;
            let columns = self.owned_columns(app);
            if self.resolve_column_widths(app, &columns) {
                self.set_state(app, |_| {});
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(width: GridLength) -> ColumnSizing {
        ColumnSizing {
            width,
            min_width: 20.0,
            min_width_set: false,
            max_width: f64::INFINITY,
            visible: true,
            actual_width: DEFAULT_WIDTH,
            measured: 0.0,
        }
    }

    #[test]
    fn pixel_auto_and_star_resolve_against_the_viewport() {
        let mut auto = column(GridLength::AUTO);
        auto.measured = 64.4;
        let columns = [
            column(GridLength::pixel(100.0)),
            auto,
            column(GridLength::star(1.0)),
            column(GridLength::star(3.0)),
        ];
        let widths = resolve_column_widths(&columns, Some(565.0), 1.0);
        assert_eq!(widths, [100.0, 65.0, 100.0, 300.0]);
    }

    #[test]
    fn an_auto_column_with_nothing_measured_takes_the_default_width() {
        let widths = resolve_column_widths(&[column(GridLength::AUTO)], None, 1.0);
        assert_eq!(widths, [DEFAULT_WIDTH]);
    }

    // WPF's `ComputeStarColumnWidths` shape: a column clamped by its bounds is fixed there, and the
    // others divide what is left.
    #[test]
    fn a_star_column_held_at_its_max_width_leaves_the_rest_to_the_others() {
        let mut narrow = column(GridLength::star(1.0));
        narrow.max_width = 50.0;
        let columns = [narrow, column(GridLength::star(1.0))];
        assert_eq!(
            resolve_column_widths(&columns, Some(400.0), 1.0),
            [50.0, 350.0]
        );
    }

    #[test]
    fn star_columns_keep_their_width_until_the_viewport_is_known_and_hidden_columns_are_skipped() {
        let mut hidden = column(GridLength::pixel(500.0));
        hidden.visible = false;
        hidden.actual_width = 7.0;
        let columns = [column(GridLength::star(1.0)), hidden];
        assert_eq!(
            resolve_column_widths(&columns, None, 1.0),
            [DEFAULT_WIDTH, 7.0]
        );
        assert_eq!(
            resolve_column_widths(&columns, Some(300.0), 1.0),
            [300.0, 7.0]
        );
    }

    #[test]
    fn a_zero_star_column_gets_no_share_unless_its_min_width_was_set() {
        let mut set = column(GridLength::star(0.0));
        set.min_width_set = true;
        let columns = [
            column(GridLength::star(0.0)),
            set,
            column(GridLength::star(1.0)),
        ];
        assert_eq!(
            resolve_column_widths(&columns, Some(200.0), 1.0),
            [0.0, 20.0, 180.0]
        );
    }

    #[test]
    fn widths_snap_to_device_pixels() {
        let columns = [column(GridLength::pixel(100.3))];
        assert_eq!(resolve_column_widths(&columns, None, 2.0), [100.5]);
    }
}
