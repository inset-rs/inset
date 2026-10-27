//! `CGrid::MeasureOverride` and `CGrid::ArrangeOverride` (`dxaml/xcp/core/core/elements/grid.cpp`) as a pure algorithm over row and column definitions: cells are placements, children are measured through a callback, and arrange returns one rect per child.

use super::definition::{ColumnDefinition, Definition, GridUnitType, RowDefinition};
use inset_embedder::{Offset, Rect, Size};

/// `REAL_EPSILON` (`xcpmath.h`): `FLT_EPSILON`.
const REAL_EPSILON: f64 = f32::EPSILON as f64;

/// `GRID_STARVALUE_MAX` (`Grid.h`): `XFLOAT_MAX / XUINT32_MAX - 1`, the cap on one star weight so the weights can be summed.
const STAR_VALUE_MAX: f64 = f32::MAX as f64 / u32::MAX as f64 - 1.0;

/// The attached `Grid.Row`, `Grid.Column`, `Grid.RowSpan` and `Grid.ColumnSpan` of one child, before clamping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellPlacement {
    pub row: usize,
    pub column: usize,
    pub row_span: usize,
    pub column_span: usize,
}

impl Default for CellPlacement {
    fn default() -> CellPlacement {
        CellPlacement {
            row: 0,
            column: 0,
            row_span: 1,
            column_span: 1,
        }
    }
}

/// `CellUnitTypes`: the union of the unit types across the definitions a cell spans.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CellUnitTypes {
    auto: bool,
    star: bool,
}

/// `CellCache`: one child with its clamped placement and the unit types of its rows and columns.
#[derive(Clone, Copy, Debug)]
struct Cell {
    placement: CellPlacement,
    row_height_types: CellUnitTypes,
    column_width_types: CellUnitTypes,
}

/// `SpanStoreEntry`: the largest desired size seen for one span, distributed after the group is measured.
#[derive(Clone, Copy, Debug)]
struct SpanStoreEntry {
    span_start: usize,
    span_count: usize,
    desired_size: f64,
    is_column_definition: bool,
}

/// `GridFlags`.
#[derive(Clone, Copy, Debug, Default)]
struct GridFlags {
    has_star_rows: bool,
    has_star_columns: bool,
    has_auto_rows_and_star_column: bool,
}

/// The definitions of one axis: `m_pRows` or `m_pColumns`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    Rows,
    Columns,
}

/// The Grid's layout state: the effective definitions, spacing and flags `CGrid` keeps between the measure and arrange passes.
#[derive(Clone, Debug)]
pub struct GridLayout {
    pub row_definitions: Vec<RowDefinition>,
    pub column_definitions: Vec<ColumnDefinition>,
    pub row_spacing: f64,
    pub column_spacing: f64,
    rows: Vec<Definition>,
    columns: Vec<Definition>,
    cells: Vec<Cell>,
    desired_sizes: Vec<Size>,
    flags: GridFlags,
}

impl GridLayout {
    pub fn new() -> GridLayout {
        GridLayout {
            row_definitions: Vec::new(),
            column_definitions: Vec::new(),
            row_spacing: 0.0,
            column_spacing: 0.0,
            rows: Vec::new(),
            columns: Vec::new(),
            cells: Vec::new(),
            desired_sizes: Vec::new(),
            flags: GridFlags::default(),
        }
    }

    /// `IsWithoutRowAndColumnDefinitions`.
    fn is_without_row_and_column_definitions(&self) -> bool {
        self.row_definitions.is_empty() && self.column_definitions.is_empty()
    }

    /// `InitializeDefinitionStructure`: the user definitions, or a single default one per axis.
    fn initialize_definition_structure(&mut self) {
        self.rows = if self.row_definitions.is_empty() {
            vec![RowDefinition::default().definition()]
        } else {
            self.row_definitions
                .iter()
                .map(RowDefinition::definition)
                .collect()
        };
        self.columns = if self.column_definitions.is_empty() {
            vec![ColumnDefinition::default().definition()]
        } else {
            self.column_definitions
                .iter()
                .map(ColumnDefinition::definition)
                .collect()
        };
    }

    fn definitions_mut(&mut self, axis: Axis) -> &mut Vec<Definition> {
        match axis {
            Axis::Rows => &mut self.rows,
            Axis::Columns => &mut self.columns,
        }
    }

    fn spacing(&self, axis: Axis) -> f64 {
        match axis {
            Axis::Rows => self.row_spacing,
            Axis::Columns => self.column_spacing,
        }
    }

    /// `GetRowIndex`, `GetColumnIndex`, `GetRowSpan`, `GetColumnSpan`: clamped into the definitions.
    fn clamp_placement(&self, placement: CellPlacement) -> CellPlacement {
        let row = placement.row.min(self.rows.len() - 1);
        let column = placement.column.min(self.columns.len() - 1);
        CellPlacement {
            row,
            column,
            row_span: placement.row_span.max(1).min(self.rows.len() - row),
            column_span: placement
                .column_span
                .max(1)
                .min(self.columns.len() - column),
        }
    }

    /// `MeasureOverride`. `available` is the size the parent offers, infinite where unbounded; `measure_child` measures child `i` against an available size and returns its desired size.
    pub fn measure(
        &mut self,
        placements: &[CellPlacement],
        available: Size,
        mut measure_child: impl FnMut(usize, Size) -> Size,
    ) -> Size {
        if self.is_without_row_and_column_definitions() {
            // Without definitions every child is measured against the whole available size.
            self.desired_sizes = (0..placements.len())
                .map(|index| measure_child(index, available))
                .collect();
            return self
                .desired_sizes
                .iter()
                .fold(Size::ZERO, |desired, child| {
                    Size::new(
                        desired.width().max(child.width()),
                        desired.height().max(child.height()),
                    )
                });
        }

        self.initialize_definition_structure();
        validate_definitions(&mut self.rows, available.height().is_infinite());
        validate_definitions(&mut self.columns, available.width().is_infinite());

        let combined_row_spacing = self.row_spacing * (self.rows.len() - 1) as f64;
        let combined_column_spacing = self.column_spacing * (self.columns.len() - 1) as f64;
        let inner_available = Size::new(
            available.width() - combined_column_spacing,
            available.height() - combined_row_spacing,
        );

        let groups = self.validate_cells(placements);

        // Measure Group1. After Group1 is measured, only Group3 can have cells belonging to Auto rows.
        self.measure_cells_group(&groups[0], false, false, &mut measure_child);

        if !self.flags.has_auto_rows_and_star_column {
            // No cyclic dependency: resolve star rows, then auto columns, then star columns.
            if self.flags.has_star_rows {
                self.resolve_star(Axis::Rows, inner_available.height());
            }
            self.measure_cells_group(&groups[1], false, false, &mut measure_child);
            if self.flags.has_star_columns {
                self.resolve_star(Axis::Columns, inner_available.width());
            }
            self.measure_cells_group(&groups[2], false, false, &mut measure_child);
        } else if groups[1].is_empty() {
            // Nothing in Group2, so star columns can be resolved before Group3.
            if self.flags.has_star_columns {
                self.resolve_star(Axis::Columns, inner_available.width());
            }
            self.measure_cells_group(&groups[2], false, false, &mut measure_child);
            if self.flags.has_star_rows {
                self.resolve_star(Axis::Rows, inner_available.height());
            }
        } else {
            // A cyclic dependency: measure Group2 for widths with infinite row heights, resolve the
            // columns, measure Group3, resolve the rows, then measure Group2 again for heights only.
            self.measure_cells_group(&groups[1], false, true, &mut measure_child);
            if self.flags.has_star_columns {
                self.resolve_star(Axis::Columns, inner_available.width());
            }
            self.measure_cells_group(&groups[2], false, false, &mut measure_child);
            if self.flags.has_star_rows {
                self.resolve_star(Axis::Rows, inner_available.height());
            }
            self.measure_cells_group(&groups[1], true, false, &mut measure_child);
        }

        // Finally, measure Group4.
        self.measure_cells_group(&groups[3], false, false, &mut measure_child);

        Size::new(
            desired_inner_size(&self.columns) + combined_column_spacing,
            desired_inner_size(&self.rows) + combined_row_spacing,
        )
    }

    /// `ValidateCells`: caches each cell's clamped placement and unit types and sorts the cells into the four measure groups, in child order.
    fn validate_cells(&mut self, placements: &[CellPlacement]) -> [Vec<usize>; 4] {
        self.flags = GridFlags::default();
        let mut groups: [Vec<usize>; 4] = Default::default();
        self.cells.clear();

        for (index, &placement) in placements.iter().enumerate() {
            let placement = self.clamp_placement(placement);
            let row_height_types =
                length_type_for_range(&self.rows, placement.row, placement.row_span);
            let column_width_types =
                length_type_for_range(&self.columns, placement.column, placement.column_span);
            self.cells.push(Cell {
                placement,
                row_height_types,
                column_width_types,
            });

            // Px/Auto rows go to group 1 (Px/Auto columns) or 3 (Star columns); Star rows go to
            // group 2 (Auto columns without Star) or 4.
            if !row_height_types.star {
                if !column_width_types.star {
                    groups[0].push(index);
                } else {
                    groups[2].push(index);
                    if row_height_types.auto {
                        // At least one Auto row with a Star column: a possible cyclic dependency.
                        self.flags.has_auto_rows_and_star_column = true;
                    }
                }
            } else {
                self.flags.has_star_rows = true;
                if column_width_types.auto && !column_width_types.star {
                    groups[1].push(index);
                } else {
                    groups[3].push(index);
                }
            }

            if column_width_types.star {
                self.flags.has_star_columns = true;
            }
        }

        groups
    }

    /// `MeasureCellsGroup`.
    fn measure_cells_group(
        &mut self,
        group: &[usize],
        ignore_column_desired_size: bool,
        force_row_to_infinity: bool,
        measure_child: &mut impl FnMut(usize, Size) -> Size,
    ) {
        let mut span_store: Vec<SpanStoreEntry> = Vec::new();

        for &index in group {
            let cell = self.cells[index];
            let desired = self.measure_cell(index, cell, force_row_to_infinity, measure_child);
            let placement = cell.placement;

            // A span is stored for later; its size is distributed once every desired size for
            // that definition index and span is known.
            if !ignore_column_desired_size {
                if placement.column_span == 1 {
                    self.columns[placement.column].update_effective_min_size(desired.width());
                } else {
                    register_span(
                        &mut span_store,
                        placement.column,
                        placement.column_span,
                        desired.width(),
                        true,
                    );
                }
            }

            if !force_row_to_infinity {
                if placement.row_span == 1 {
                    self.rows[placement.row].update_effective_min_size(desired.height());
                } else {
                    register_span(
                        &mut span_store,
                        placement.row,
                        placement.row_span,
                        desired.height(),
                        false,
                    );
                }
            }
        }

        for entry in span_store {
            let axis = if entry.is_column_definition {
                Axis::Columns
            } else {
                Axis::Rows
            };
            let spacing = self.spacing(axis);
            ensure_min_size_in_definition_range(
                self.definitions_mut(axis),
                entry.span_start,
                entry.span_count,
                spacing,
                entry.desired_size,
            );
        }
    }

    /// `MeasureCell`: an Auto-only range is measured unbounded; any other range against the size its definitions hold so far.
    fn measure_cell(
        &self,
        index: usize,
        cell: Cell,
        force_row_to_infinity: bool,
        measure_child: &mut impl FnMut(usize, Size) -> Size,
    ) -> Size {
        let placement = cell.placement;
        let width = if cell.column_width_types.auto && !cell.column_width_types.star {
            f64::INFINITY
        } else {
            available_size_for_range(
                &self.columns,
                placement.column,
                placement.column_span,
                self.column_spacing,
            )
        };
        let height = if force_row_to_infinity
            || (cell.row_height_types.auto && !cell.row_height_types.star)
        {
            f64::INFINITY
        } else {
            available_size_for_range(
                &self.rows,
                placement.row,
                placement.row_span,
                self.row_spacing,
            )
        };
        measure_child(index, Size::new(width, height))
    }

    /// `ResolveStar`: resolves the star definitions of one axis against the space the other definitions leave.
    fn resolve_star(&mut self, axis: Axis, available_size: f64) {
        let definitions = self.definitions_mut(axis);
        let mut star_definitions = Vec::new();
        let mut taken_size = 0.0;

        for (index, definition) in definitions.iter_mut().enumerate() {
            match definition.effective_unit_type {
                GridUnitType::Star => {
                    star_definitions.push(index);
                    prepare_star(definition);
                }
                GridUnitType::Pixel => taken_size += definition.measure_arrange_size,
                GridUnitType::Auto => taken_size += definition.effective_min_size,
            }
        }

        distribute_star_space(
            definitions,
            &mut star_definitions,
            available_size - taken_size,
            &mut taken_size,
        );
    }

    /// `ArrangeOverride`: the rect of every child for the final size, in child order.
    pub fn arrange(&mut self, placements: &[CellPlacement], final_size: Size) -> Vec<Rect> {
        if self.is_without_row_and_column_definitions() {
            // Without definitions the arrange rect grows to each child's desired size in turn, so
            // a large child enlarges the rect the children after it receive.
            let mut rect = final_size;
            return self
                .desired_sizes
                .iter()
                .map(|desired| {
                    rect = Size::new(
                        rect.width().max(desired.width()),
                        rect.height().max(desired.height()),
                    );
                    Offset::ZERO & rect
                })
                .collect();
        }

        let combined_row_spacing = self.row_spacing * (self.rows.len() - 1) as f64;
        let combined_column_spacing = self.column_spacing * (self.columns.len() - 1) as f64;
        set_final_size(&mut self.rows, final_size.height() - combined_row_spacing);
        set_final_size(
            &mut self.columns,
            final_size.width() - combined_column_spacing,
        );

        placements
            .iter()
            .map(|&placement| {
                let placement = self.clamp_placement(placement);
                let x = self.columns[placement.column].final_offset
                    + self.column_spacing * placement.column as f64;
                let y =
                    self.rows[placement.row].final_offset + self.row_spacing * placement.row as f64;
                let width = final_size_for_range(
                    &self.columns,
                    placement.column,
                    placement.column_span,
                    self.column_spacing,
                );
                let height = final_size_for_range(
                    &self.rows,
                    placement.row,
                    placement.row_span,
                    self.row_spacing,
                );
                Rect::from_ltwh(x, y, width, height)
            })
            .collect()
    }
}

impl Default for GridLayout {
    fn default() -> GridLayout {
        GridLayout::new()
    }
}

/// `ValidateDefinitions`: the effective unit type, min size and initial measure size of each definition.
fn validate_definitions(definitions: &mut [Definition], treat_star_as_auto: bool) {
    for definition in definitions {
        let mut user_size = f64::INFINITY;
        let mut user_min_size = definition.user_min_size;
        let user_max_size = definition.user_max_size;

        match definition.user_size_type() {
            GridUnitType::Pixel => {
                user_size = definition.user_size.value;
                user_min_size = user_min_size.max(user_size.min(user_max_size));
                definition.effective_unit_type = GridUnitType::Pixel;
            }
            GridUnitType::Auto => definition.effective_unit_type = GridUnitType::Auto,
            GridUnitType::Star => {
                definition.effective_unit_type = if treat_star_as_auto {
                    GridUnitType::Auto
                } else {
                    GridUnitType::Star
                };
            }
        }

        definition.effective_min_size = user_min_size;
        definition.measure_arrange_size = user_min_size.max(user_size.min(user_max_size));
    }
}

/// `GetLengthTypeForRange`.
fn length_type_for_range(definitions: &[Definition], start: usize, count: usize) -> CellUnitTypes {
    let mut types = CellUnitTypes::default();
    for definition in &definitions[start..start + count] {
        match definition.effective_unit_type {
            GridUnitType::Auto => types.auto = true,
            GridUnitType::Star => types.star = true,
            GridUnitType::Pixel => {}
        }
    }
    types
}

/// `GetAvailableSizeForRange`: what the definitions of a range offer a child, Auto ones by their minimum so far.
fn available_size_for_range(
    definitions: &[Definition],
    start: usize,
    count: usize,
    spacing: f64,
) -> f64 {
    definitions[start..start + count]
        .iter()
        .map(|definition| {
            if definition.effective_unit_type == GridUnitType::Auto {
                definition.effective_min_size
            } else {
                definition.measure_arrange_size
            }
        })
        .sum::<f64>()
        + spacing * (count - 1) as f64
}

/// `GetFinalSizeForRange`.
fn final_size_for_range(
    definitions: &[Definition],
    start: usize,
    count: usize,
    spacing: f64,
) -> f64 {
    definitions[start..start + count]
        .iter()
        .map(|definition| definition.measure_arrange_size)
        .sum::<f64>()
        + spacing * (count - 1) as f64
}

/// `GetDesiredInnerSize`.
fn desired_inner_size(definitions: &[Definition]) -> f64 {
    definitions
        .iter()
        .map(|definition| definition.effective_min_size)
        .sum()
}

/// `RegisterSpan`: one entry per (start, count, axis) holding the largest desired size.
fn register_span(
    span_store: &mut Vec<SpanStoreEntry>,
    span_start: usize,
    span_count: usize,
    desired_size: f64,
    is_column_definition: bool,
) {
    if let Some(entry) = span_store.iter_mut().find(|entry| {
        entry.is_column_definition == is_column_definition
            && entry.span_start == span_start
            && entry.span_count == span_count
    }) {
        if entry.desired_size < desired_size {
            entry.desired_size = desired_size;
        }
    } else {
        span_store.push(SpanStoreEntry {
            span_start,
            span_count,
            desired_size,
            is_column_definition,
        });
    }
}

/// `EnsureMinSizeInDefinitionRange`: distributes a spanning child's desired size into the minimum sizes of the definitions it spans.
fn ensure_min_size_in_definition_range(
    definitions: &mut [Definition],
    span_start: usize,
    span_count: usize,
    spacing: f64,
    child_desired_size: f64,
) {
    debug_assert!(span_count > 1 && span_start + span_count <= definitions.len());

    // The spacing between the spanned definitions is not distributed.
    let requested_size = (child_desired_size - spacing * (span_count - 1) as f64).max(0.0);
    if requested_size <= REAL_EPSILON {
        return;
    }

    let span_end = span_start + span_count;
    let mut auto_definitions_count = 0;
    let mut range_min_size = 0.0;
    let mut range_max_size = 0.0;
    let mut range_preferred_size = 0.0;
    let mut max_max_size: f64 = 0.0;
    let mut temp: Vec<usize> = Vec::with_capacity(span_count);

    // Sum the sizes in the range, cache each max size, find the largest max size and count the
    // Auto definitions.
    for (index, definition) in definitions[span_start..span_end].iter_mut().enumerate() {
        let effective_min_size = definition.effective_min_size;
        let preferred_size = definition.preferred_size();
        let max_size = definition.user_max_size.max(effective_min_size);
        range_min_size += effective_min_size;
        range_preferred_size += preferred_size;
        range_max_size += max_size;

        debug_assert!(
            effective_min_size <= preferred_size
                && preferred_size <= max_size
                && range_min_size <= range_preferred_size
                && range_preferred_size <= range_max_size
        );

        definition.size_cache = max_size;
        max_max_size = max_max_size.max(max_size);
        if definition.is_auto() {
            auto_definitions_count += 1;
        }
        temp.push(span_start + index);
    }

    if requested_size <= range_min_size {
        // The range is already big enough.
    } else if requested_size <= range_preferred_size {
        // Within the preferred size of the range: Auto definitions stay tight; the others grow
        // to equal minimum sizes without exceeding their preferred size. Sorted Auto first, then
        // the others by ascending preferred size.
        let mut size_to_distribute = requested_size;
        sort_for_span_preferred_distribution(definitions, &mut temp);

        for &index in &temp[..auto_definitions_count] {
            debug_assert!(definitions[index].is_auto());
            size_to_distribute -= definitions[index].effective_min_size;
        }

        for (position, &index) in temp.iter().enumerate().skip(auto_definitions_count) {
            let definition = &mut definitions[index];
            debug_assert!(!definition.is_auto());
            let new_min_size = (size_to_distribute / (span_count - position) as f64)
                .min(definition.preferred_size());
            definition.update_effective_min_size(new_min_size);
            size_to_distribute -= new_min_size;
            if size_to_distribute < REAL_EPSILON {
                break;
            }
        }
    } else if requested_size <= range_max_size {
        // Beyond the preferred size but within the max size of the range: Auto definitions stay
        // tight if possible; the others grow to equal minimum sizes without exceeding their max
        // size. Sorted non-Auto first, then Auto, each by ascending max size (the size cache).
        let mut size_to_distribute = requested_size - range_preferred_size;
        sort_for_span_max_size_distribution(definitions, &mut temp);

        let non_auto_definitions_count = span_count - auto_definitions_count;
        for (position, &index) in temp.iter().enumerate() {
            let definition = &mut definitions[index];
            let preferred_size = definition.preferred_size();
            let mut new_min_size = preferred_size;
            if position < non_auto_definitions_count {
                debug_assert!(!definition.is_auto());
                new_min_size += size_to_distribute / (non_auto_definitions_count - position) as f64;
            } else {
                debug_assert!(definition.is_auto());
                new_min_size += size_to_distribute / (span_count - position) as f64;
            }

            new_min_size = new_min_size.min(definition.size_cache);
            definition.update_effective_min_size(new_min_size);
            size_to_distribute -= definition.effective_min_size - preferred_size;
            if size_to_distribute < REAL_EPSILON {
                break;
            }
        }
    } else {
        // Beyond the max size of the range: every definition grows towards an equal share.
        let equally_distributed_size = requested_size / span_count as f64;

        if equally_distributed_size < max_max_size
            && max_max_size - equally_distributed_size > REAL_EPSILON
        {
            // Below the largest max size, smaller definitions grow faster than larger ones.
            let total_remaining_size = max_max_size * span_count as f64 - range_max_size;
            let size_to_distribute = requested_size - range_max_size;
            debug_assert!(
                total_remaining_size.is_finite()
                    && total_remaining_size > 0.0
                    && size_to_distribute.is_finite()
                    && size_to_distribute > 0.0
            );
            for &index in &temp {
                let definition = &mut definitions[index];
                let delta_size = (max_max_size - definition.size_cache) * size_to_distribute
                    / total_remaining_size;
                definition.update_effective_min_size(definition.size_cache + delta_size);
            }
        } else {
            for &index in &temp {
                definitions[index].update_effective_min_size(equally_distributed_size);
            }
        }
    }
}

/// The star setup shared by `ResolveStar` and `SetFinalSize`: the weight goes into the measure size, `max size / weight` into the size cache.
fn prepare_star(definition: &mut Definition) {
    // The user value is in star units, not pixels.
    let star_value = definition.user_size.value;
    if star_value < REAL_EPSILON {
        definition.measure_arrange_size = 0.0;
        definition.size_cache = 0.0;
    } else {
        // Clipped by a max to avoid overflow when all the star values are added up.
        let star_value = star_value.min(STAR_VALUE_MAX);
        definition.measure_arrange_size = star_value;
        let max_size =
            STAR_VALUE_MAX.min(definition.effective_min_size.max(definition.user_max_size));
        definition.size_cache = max_size / star_value;
    }
}

/// `DistributeStarSpace`: the definition with the lowest max-size-to-weight ratio is sized first.
fn distribute_star_space(
    definitions: &mut [Definition],
    star_definitions: &mut [usize],
    available_size: f64,
    total_resolved_size: &mut f64,
) {
    sort_for_star_size_distribution(definitions, star_definitions);

    // Partial sums of the weights, from the last definition back to the first.
    let mut all_star_weights = 0.0;
    for &index in star_definitions.iter().rev() {
        all_star_weights += definitions[index].measure_arrange_size;
        definitions[index].size_cache = all_star_weights;
    }

    let mut total_star_resolved_size = 0.0;
    for &index in star_definitions.iter() {
        let definition = &mut definitions[index];
        let star_value = definition.measure_arrange_size;
        let resolved_size = if star_value == 0.0 {
            definition.effective_min_size
        } else {
            let resolved_size = (available_size - total_star_resolved_size).max(0.0)
                * (star_value / definition.size_cache);
            definition
                .effective_min_size
                .max(resolved_size.min(definition.user_max_size))
        };
        definition.measure_arrange_size = resolved_size;
        total_star_resolved_size += resolved_size;
    }
    *total_resolved_size += total_star_resolved_size;
}

/// `SetFinalSize`: the final size and offset of every definition of one axis.
fn set_final_size(definitions: &mut [Definition], final_size: f64) {
    let mut all_preferred_arrange_size = 0.0;
    let mut star_definitions = Vec::new();
    let mut non_star_definitions = Vec::new();

    for (index, definition) in definitions.iter_mut().enumerate() {
        if definition.user_size_type() == GridUnitType::Star {
            star_definitions.push(index);
            prepare_star(definition);
        } else {
            // The non-star definitions, in reverse order as `m_ppTempDefinitions` fills them
            // from the back.
            non_star_definitions.insert(0, index);
            let user_size = match definition.user_size_type() {
                GridUnitType::Pixel => definition.user_size.value,
                GridUnitType::Auto => definition.effective_min_size,
                GridUnitType::Star => unreachable!(),
            };
            definition.measure_arrange_size = definition
                .effective_min_size
                .max(user_size.min(definition.user_max_size));
            all_preferred_arrange_size += definition.measure_arrange_size;
        }
    }

    distribute_star_space(
        definitions,
        &mut star_definitions,
        final_size - all_preferred_arrange_size,
        &mut all_preferred_arrange_size,
    );

    // The combined size exceeds the final size: take the difference away, from the definitions
    // with the least room above their minimum first.
    if all_preferred_arrange_size > final_size
        && (all_preferred_arrange_size - final_size).abs() > REAL_EPSILON
    {
        let mut temp: Vec<usize> = star_definitions
            .iter()
            .chain(&non_star_definitions)
            .copied()
            .collect();
        sort_for_overflow_size_distribution(definitions, &mut temp);
        let mut size_to_distribute = final_size - all_preferred_arrange_size;
        let count = definitions.len();
        for (position, &index) in temp.iter().enumerate() {
            let definition = &mut definitions[index];
            let mut final_measure_arrange_size =
                definition.measure_arrange_size + size_to_distribute / (count - position) as f64;
            final_measure_arrange_size =
                final_measure_arrange_size.max(definition.effective_min_size);
            final_measure_arrange_size =
                final_measure_arrange_size.min(definition.measure_arrange_size);
            size_to_distribute -= final_measure_arrange_size - definition.measure_arrange_size;
            definition.measure_arrange_size = final_measure_arrange_size;
        }
    }

    // Offsets, in the original order.
    let mut offset = 0.0;
    for definition in definitions.iter_mut() {
        definition.final_offset = offset;
        offset += definition.measure_arrange_size;
    }
}

fn ascending(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}

/// `SortDefinitionsForSpanPreferredDistribution`: Auto definitions first by effective min size, then the others by preferred size. The source's stable insertion sort in the same order.
fn sort_for_span_preferred_distribution(definitions: &[Definition], indices: &mut [usize]) {
    indices.sort_by(|&a, &b| {
        let (a, b) = (&definitions[a], &definitions[b]);
        match (a.is_auto(), b.is_auto()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (true, true) => ascending(a.effective_min_size, b.effective_min_size),
            (false, false) => ascending(a.preferred_size(), b.preferred_size()),
        }
    });
}

/// `SortDefinitionsForSpanMaxSizeDistribution`: non-Auto definitions first, then Auto, each by ascending size cache.
fn sort_for_span_max_size_distribution(definitions: &[Definition], indices: &mut [usize]) {
    indices.sort_by(|&a, &b| {
        let (a, b) = (&definitions[a], &definitions[b]);
        match (a.is_auto(), b.is_auto()) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => ascending(a.size_cache, b.size_cache),
        }
    });
}

/// `SortDefinitionsForOverflowSizeDistribution`: by the room between the arrange size and the effective min size.
fn sort_for_overflow_size_distribution(definitions: &[Definition], indices: &mut [usize]) {
    indices.sort_by(|&a, &b| {
        let (a, b) = (&definitions[a], &definitions[b]);
        ascending(
            a.measure_arrange_size - a.effective_min_size,
            b.measure_arrange_size - b.effective_min_size,
        )
    });
}

/// `SortDefinitionsForStarSizeDistribution`: by the size cache (max size over weight), stable as WPF's.
fn sort_for_star_size_distribution(definitions: &[Definition], indices: &mut [usize]) {
    indices.sort_by(|&a, &b| ascending(definitions[a].size_cache, definitions[b].size_cache));
}

#[cfg(test)]
mod tests {
    use super::super::definition::GridLength;
    use super::*;
    use inset_embedder::Rect;

    fn columns(lengths: &[GridLength]) -> Vec<ColumnDefinition> {
        lengths
            .iter()
            .map(|&width| ColumnDefinition::new(width))
            .collect()
    }

    fn rows(lengths: &[GridLength]) -> Vec<RowDefinition> {
        lengths
            .iter()
            .map(|&height| RowDefinition::new(height))
            .collect()
    }

    fn cell(row: usize, column: usize) -> CellPlacement {
        CellPlacement {
            row,
            column,
            ..CellPlacement::default()
        }
    }

    /// Measures children that, like a `Rectangle` with `Width` and `Height`, want a fixed size whatever they are offered, then arranges at `final_size` (or the desired size when `None`).
    fn lay_out(
        layout: &mut GridLayout,
        placements: &[CellPlacement],
        sizes: &[Size],
        available: Size,
        final_size: Option<Size>,
    ) -> (Size, Vec<Rect>) {
        let desired = layout.measure(placements, available, |index, _| sizes[index]);
        let rects = layout.arrange(placements, final_size.unwrap_or(desired));
        (desired, rects)
    }

    fn assert_rect(rect: Rect, expected: (f64, f64, f64, f64)) {
        let actual = (rect.left, rect.top, rect.width(), rect.height());
        let close = |a: f64, b: f64| (a - b).abs() < 1e-6;
        assert!(
            close(actual.0, expected.0)
                && close(actual.1, expected.1)
                && close(actual.2, expected.2)
                && close(actual.3, expected.3),
            "expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn pixel_auto_and_star_columns_share_the_width() {
        let mut layout = GridLayout::new();
        layout.column_definitions =
            columns(&[GridLength::pixel(50.0), GridLength::AUTO, GridLength::STAR]);
        let placements = [cell(0, 0), cell(0, 1), cell(0, 2)];
        let sizes = [
            Size::new(30.0, 10.0),
            Size::new(70.0, 10.0),
            Size::new(20.0, 10.0),
        ];
        let final_size = Size::new(400.0, 400.0);
        let (desired, rects) = lay_out(
            &mut layout,
            &placements,
            &sizes,
            final_size,
            Some(final_size),
        );
        assert_eq!(desired, Size::new(140.0, 10.0));
        assert_rect(rects[0], (0.0, 0.0, 50.0, 400.0));
        assert_rect(rects[1], (50.0, 0.0, 70.0, 400.0));
        assert_rect(rects[2], (120.0, 0.0, 280.0, 400.0));
    }

    #[test]
    fn star_weights_split_the_remaining_space() {
        let mut layout = GridLayout::new();
        layout.column_definitions = columns(&[GridLength::STAR, GridLength::star(2.0)]);
        let placements = [cell(0, 0), cell(0, 1)];
        let sizes = [Size::new(10.0, 10.0); 2];
        let final_size = Size::new(300.0, 100.0);
        let (_, rects) = lay_out(
            &mut layout,
            &placements,
            &sizes,
            final_size,
            Some(final_size),
        );
        assert_rect(rects[0], (0.0, 0.0, 100.0, 100.0));
        assert_rect(rects[1], (100.0, 0.0, 200.0, 100.0));
    }

    #[test]
    fn row_spacing_is_taken_before_star_rows_are_resolved() {
        let mut layout = GridLayout::new();
        layout.row_definitions = rows(&[GridLength::STAR; 3]);
        layout.row_spacing = 10.0;
        let placements = [cell(0, 0), cell(1, 0), cell(2, 0)];
        let sizes = [Size::new(10.0, 10.0); 3];
        let final_size = Size::new(100.0, 400.0);
        let (desired, rects) = lay_out(
            &mut layout,
            &placements,
            &sizes,
            final_size,
            Some(final_size),
        );
        assert_eq!(desired, Size::new(10.0, 50.0));
        let row = 380.0 / 3.0;
        assert_rect(rects[0], (0.0, 0.0, 100.0, row));
        assert_rect(rects[1], (0.0, row + 10.0, 100.0, row));
        assert_rect(rects[2], (0.0, 2.0 * (row + 10.0), 100.0, row));
    }

    #[test]
    fn a_span_grows_auto_columns_towards_equal_shares() {
        let mut layout = GridLayout::new();
        layout.column_definitions = columns(&[GridLength::AUTO, GridLength::AUTO]);
        let placements = [
            cell(0, 0),
            CellPlacement {
                column_span: 2,
                ..cell(0, 0)
            },
        ];
        let sizes = [Size::new(30.0, 10.0), Size::new(100.0, 10.0)];
        let (desired, rects) = lay_out(
            &mut layout,
            &placements,
            &sizes,
            Size::new(400.0, 400.0),
            None,
        );
        // `EnsureMinSizeInDefinitionRange`: 70 beyond the preferred 30 goes 35 to each column.
        assert_eq!(desired, Size::new(100.0, 10.0));
        assert_rect(rects[0], (0.0, 0.0, 65.0, 10.0));
        assert_rect(rects[1], (0.0, 0.0, 100.0, 10.0));
    }

    #[test]
    fn an_auto_row_takes_its_content_and_the_star_row_the_rest() {
        let mut layout = GridLayout::new();
        layout.row_definitions = rows(&[GridLength::AUTO, GridLength::STAR]);
        let placements = [cell(0, 0), cell(1, 0)];
        let sizes = [Size::new(20.0, 40.0), Size::new(20.0, 20.0)];
        let final_size = Size::new(400.0, 400.0);
        let (desired, rects) = lay_out(
            &mut layout,
            &placements,
            &sizes,
            final_size,
            Some(final_size),
        );
        assert_eq!(desired, Size::new(20.0, 60.0));
        assert_rect(rects[0], (0.0, 0.0, 400.0, 40.0));
        assert_rect(rects[1], (0.0, 40.0, 400.0, 360.0));
    }

    #[test]
    fn without_definitions_every_child_gets_the_largest_desired_size() {
        let mut layout = GridLayout::new();
        let placements = [cell(0, 0), cell(0, 0)];
        let sizes = [Size::new(50.0, 50.0), Size::new(30.0, 80.0)];
        let (desired, rects) = lay_out(
            &mut layout,
            &placements,
            &sizes,
            Size::new(400.0, 400.0),
            None,
        );
        assert_eq!(desired, Size::new(50.0, 80.0));
        assert_rect(rects[0], (0.0, 0.0, 50.0, 80.0));
        assert_rect(rects[1], (0.0, 0.0, 50.0, 80.0));
    }

    #[test]
    fn pixel_columns_do_not_shrink_below_their_length() {
        let mut layout = GridLayout::new();
        layout.column_definitions = columns(&[GridLength::pixel(200.0), GridLength::pixel(300.0)]);
        let placements = [cell(0, 0), cell(0, 1)];
        let sizes = [Size::new(10.0, 10.0); 2];
        let final_size = Size::new(400.0, 100.0);
        let (_, rects) = lay_out(
            &mut layout,
            &placements,
            &sizes,
            final_size,
            Some(final_size),
        );
        assert_rect(rects[0], (0.0, 0.0, 200.0, 100.0));
        assert_rect(rects[1], (200.0, 0.0, 300.0, 100.0));
    }

    #[test]
    fn a_star_column_measured_unbounded_behaves_as_auto() {
        let mut layout = GridLayout::new();
        layout.column_definitions = columns(&[GridLength::STAR]);
        let placements = [cell(0, 0)];
        let sizes = [Size::new(30.0, 10.0)];
        let (desired, rects) = lay_out(
            &mut layout,
            &placements,
            &sizes,
            Size::new(f64::INFINITY, f64::INFINITY),
            None,
        );
        assert_eq!(desired, Size::new(30.0, 10.0));
        assert_rect(rects[0], (0.0, 0.0, 30.0, 10.0));
    }

    #[test]
    fn placements_outside_the_definitions_are_clamped() {
        let mut layout = GridLayout::new();
        layout.column_definitions = columns(&[GridLength::pixel(10.0), GridLength::pixel(20.0)]);
        let placements = [CellPlacement {
            column: 5,
            column_span: 9,
            ..cell(0, 0)
        }];
        let sizes = [Size::new(1.0, 1.0)];
        let final_size = Size::new(30.0, 10.0);
        let (_, rects) = lay_out(
            &mut layout,
            &placements,
            &sizes,
            final_size,
            Some(final_size),
        );
        assert_rect(rects[0], (10.0, 0.0, 20.0, 10.0));
    }
}
