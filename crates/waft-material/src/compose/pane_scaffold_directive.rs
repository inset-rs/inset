//! How many panes a window has room for, and how they are spaced.

use crate::compose::{WindowHeightSizeClass, WindowSizeClass, WindowWidthSizeClass};

/// How many panes the window has room for, and how they are spaced.
///
/// The values are Compose's `PaneScaffoldDirective` without the posture-derived bounds: a hinge
/// waits for the embedders to report one.
#[derive(Clone, Debug, PartialEq)]
pub struct PaneScaffoldDirective {
    /// The maximum number of panes side by side.
    pub max_horizontal_partitions: usize,
    /// The spacer between two panes side by side, in logical pixels.
    pub horizontal_partition_spacer_size: f64,
    /// The maximum number of panes stacked.
    pub max_vertical_partitions: usize,
    /// The spacer between two stacked panes, in logical pixels.
    pub vertical_partition_spacer_size: f64,
    /// The width a pane asks for when it names none.
    pub default_pane_preferred_width: f64,
    /// The height a reflowed pane asks for when it names none.
    pub default_pane_preferred_height: f64,
}

impl PaneScaffoldDirective {
    /// Compose's default preferred pane width, in logical pixels.
    pub const DEFAULT_PANE_PREFERRED_WIDTH: f64 = 360.0;
    /// Compose's default preferred pane width on a large window, in logical pixels.
    pub const DEFAULT_PANE_PREFERRED_WIDTH_XL: f64 = 412.0;
    /// Compose's default preferred pane height, in logical pixels.
    pub const DEFAULT_PANE_PREFERRED_HEIGHT: f64 = 420.0;

    /// One partition below Expanded, two at Expanded and three from Large; 24-pixel spacers.
    pub fn from_window_size_class(size_class: WindowSizeClass) -> PaneScaffoldDirective {
        let (max_horizontal_partitions, horizontal_partition_spacer_size, width) =
            match size_class.width {
                WindowWidthSizeClass::Compact | WindowWidthSizeClass::Medium => {
                    (1, 0.0, Self::DEFAULT_PANE_PREFERRED_WIDTH)
                }
                WindowWidthSizeClass::Expanded => (2, 24.0, Self::DEFAULT_PANE_PREFERRED_WIDTH),
                WindowWidthSizeClass::Large | WindowWidthSizeClass::ExtraLarge => {
                    (3, 24.0, Self::DEFAULT_PANE_PREFERRED_WIDTH_XL)
                }
            };
        // A single column on a tall window stacks a reflowed pane under its anchor.
        let (max_vertical_partitions, vertical_partition_spacer_size) = if max_horizontal_partitions
            == 1
            && size_class.height == WindowHeightSizeClass::Expanded
        {
            (2, 24.0)
        } else {
            (1, 0.0)
        };
        PaneScaffoldDirective {
            max_horizontal_partitions,
            horizontal_partition_spacer_size,
            max_vertical_partitions,
            vertical_partition_spacer_size,
            default_pane_preferred_width: width,
            default_pane_preferred_height: Self::DEFAULT_PANE_PREFERRED_HEIGHT,
        }
    }

    /// The same, with two panes already at Medium width.
    pub fn with_two_panes_on_medium_width(size_class: WindowSizeClass) -> PaneScaffoldDirective {
        let mut directive = PaneScaffoldDirective::from_window_size_class(size_class);
        if size_class.width == WindowWidthSizeClass::Medium {
            directive.max_horizontal_partitions = 2;
            directive.horizontal_partition_spacer_size = 24.0;
            directive.max_vertical_partitions = 1;
            directive.vertical_partition_spacer_size = 0.0;
        }
        directive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(width: WindowWidthSizeClass, height: WindowHeightSizeClass) -> WindowSizeClass {
        WindowSizeClass { width, height }
    }

    #[test]
    fn a_compact_window_has_one_pane_and_no_spacers() {
        let directive = PaneScaffoldDirective::from_window_size_class(size(
            WindowWidthSizeClass::Compact,
            WindowHeightSizeClass::Medium,
        ));
        assert_eq!(directive.max_horizontal_partitions, 1);
        assert_eq!(directive.max_vertical_partitions, 1);
        assert_eq!(directive.horizontal_partition_spacer_size, 0.0);
        assert_eq!(directive.vertical_partition_spacer_size, 0.0);
        assert_eq!(directive.default_pane_preferred_width, 360.0);
        assert_eq!(directive.default_pane_preferred_height, 420.0);
    }

    #[test]
    fn a_tall_single_column_stacks_reflowed_panes() {
        let directive = PaneScaffoldDirective::from_window_size_class(size(
            WindowWidthSizeClass::Medium,
            WindowHeightSizeClass::Expanded,
        ));
        assert_eq!(directive.max_horizontal_partitions, 1);
        assert_eq!(directive.max_vertical_partitions, 2);
        assert_eq!(directive.vertical_partition_spacer_size, 24.0);
    }

    #[test]
    fn an_expanded_window_has_two_panes_with_a_24_pixel_spacer() {
        let directive = PaneScaffoldDirective::from_window_size_class(size(
            WindowWidthSizeClass::Expanded,
            WindowHeightSizeClass::Expanded,
        ));
        assert_eq!(directive.max_horizontal_partitions, 2);
        assert_eq!(directive.max_vertical_partitions, 1);
        assert_eq!(directive.horizontal_partition_spacer_size, 24.0);
        assert_eq!(directive.default_pane_preferred_width, 360.0);
    }

    #[test]
    fn an_extra_large_window_has_three_panes_and_the_wide_default() {
        let directive = PaneScaffoldDirective::from_window_size_class(size(
            WindowWidthSizeClass::ExtraLarge,
            WindowHeightSizeClass::Expanded,
        ));
        assert_eq!(directive.max_horizontal_partitions, 3);
        assert_eq!(directive.horizontal_partition_spacer_size, 24.0);
        assert_eq!(directive.default_pane_preferred_width, 412.0);
    }

    #[test]
    fn a_medium_window_can_take_two_panes_when_asked() {
        let directive = PaneScaffoldDirective::with_two_panes_on_medium_width(size(
            WindowWidthSizeClass::Medium,
            WindowHeightSizeClass::Expanded,
        ));
        assert_eq!(directive.max_horizontal_partitions, 2);
        assert_eq!(directive.horizontal_partition_spacer_size, 24.0);
        assert_eq!(directive.max_vertical_partitions, 1);
        assert_eq!(directive.vertical_partition_spacer_size, 0.0);
    }

    #[test]
    fn two_panes_on_medium_leaves_other_classes_alone() {
        let directive = PaneScaffoldDirective::with_two_panes_on_medium_width(size(
            WindowWidthSizeClass::Expanded,
            WindowHeightSizeClass::Expanded,
        ));
        assert_eq!(
            directive,
            PaneScaffoldDirective::from_window_size_class(size(
                WindowWidthSizeClass::Expanded,
                WindowHeightSizeClass::Expanded,
            ))
        );
    }
}
