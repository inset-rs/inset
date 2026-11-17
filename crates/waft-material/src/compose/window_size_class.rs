//! The window's width and height size classes, from androidx.window's V2 breakpoints.

use waft_embedder::Size;
use waft_foundation::App;
use waft_widgets::{BuildContext, MediaQuery};

/// Material's width breakpoints, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WindowWidthSizeClass {
    /// Below 600 logical pixels.
    Compact,
    /// From 600 to 840.
    Medium,
    /// From 840 to 1200.
    Expanded,
    /// From 1200 to 1600.
    Large,
    /// From 1600.
    ExtraLarge,
}

/// Material's height breakpoints, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WindowHeightSizeClass {
    /// Below 480 logical pixels.
    Compact,
    /// From 480 to 900.
    Medium,
    /// From 900.
    Expanded,
}

/// The classes of a window's width and height.
///
/// The classes say how much room a layout has, not how the window is used; posture and hinges,
/// which would refine that, wait for the embedders to report them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowSizeClass {
    /// The width class.
    pub width: WindowWidthSizeClass,
    /// The height class.
    pub height: WindowHeightSizeClass,
}

impl WindowSizeClass {
    /// The lower bound of [`WindowWidthSizeClass::Medium`].
    pub const WIDTH_MEDIUM_LOWER_BOUND: f64 = 600.0;
    /// The lower bound of [`WindowWidthSizeClass::Expanded`].
    pub const WIDTH_EXPANDED_LOWER_BOUND: f64 = 840.0;
    /// The lower bound of [`WindowWidthSizeClass::Large`].
    pub const WIDTH_LARGE_LOWER_BOUND: f64 = 1200.0;
    /// The lower bound of [`WindowWidthSizeClass::ExtraLarge`].
    pub const WIDTH_EXTRA_LARGE_LOWER_BOUND: f64 = 1600.0;
    /// The lower bound of [`WindowHeightSizeClass::Medium`].
    pub const HEIGHT_MEDIUM_LOWER_BOUND: f64 = 480.0;
    /// The lower bound of [`WindowHeightSizeClass::Expanded`].
    pub const HEIGHT_EXPANDED_LOWER_BOUND: f64 = 900.0;

    /// The class of a window of this logical size.
    pub fn from_size(size: Size) -> WindowSizeClass {
        WindowSizeClass {
            width: width_class(size.width()),
            height: height_class(size.height()),
        }
    }

    /// The class of the nearest [`MediaQuery`]'s size.
    ///
    /// Reading it makes the caller rebuild when the window size changes.
    pub fn of(app: &mut App, context: BuildContext) -> WindowSizeClass {
        WindowSizeClass::from_size(MediaQuery::size_of(app, context))
    }

    /// The class of the nearest [`MediaQuery`]'s size, or `None` without one.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<WindowSizeClass> {
        MediaQuery::maybe_size_of(app, context).map(WindowSizeClass::from_size)
    }
}

fn width_class(width: f64) -> WindowWidthSizeClass {
    if width < WindowSizeClass::WIDTH_MEDIUM_LOWER_BOUND {
        WindowWidthSizeClass::Compact
    } else if width < WindowSizeClass::WIDTH_EXPANDED_LOWER_BOUND {
        WindowWidthSizeClass::Medium
    } else if width < WindowSizeClass::WIDTH_LARGE_LOWER_BOUND {
        WindowWidthSizeClass::Expanded
    } else if width < WindowSizeClass::WIDTH_EXTRA_LARGE_LOWER_BOUND {
        WindowWidthSizeClass::Large
    } else {
        WindowWidthSizeClass::ExtraLarge
    }
}

fn height_class(height: f64) -> WindowHeightSizeClass {
    if height < WindowSizeClass::HEIGHT_MEDIUM_LOWER_BOUND {
        WindowHeightSizeClass::Compact
    } else if height < WindowSizeClass::HEIGHT_EXPANDED_LOWER_BOUND {
        WindowHeightSizeClass::Medium
    } else {
        WindowHeightSizeClass::Expanded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_width_breakpoint_starts_its_class() {
        for (width, class) in [
            (0.0, WindowWidthSizeClass::Compact),
            (599.0, WindowWidthSizeClass::Compact),
            (600.0, WindowWidthSizeClass::Medium),
            (839.0, WindowWidthSizeClass::Medium),
            (840.0, WindowWidthSizeClass::Expanded),
            (1199.0, WindowWidthSizeClass::Expanded),
            (1200.0, WindowWidthSizeClass::Large),
            (1599.0, WindowWidthSizeClass::Large),
            (1600.0, WindowWidthSizeClass::ExtraLarge),
            (2000.0, WindowWidthSizeClass::ExtraLarge),
        ] {
            assert_eq!(
                WindowSizeClass::from_size(Size::new(width, 0.0)).width,
                class,
                "width {width}"
            );
        }
    }

    #[test]
    fn each_height_breakpoint_starts_its_class() {
        for (height, class) in [
            (0.0, WindowHeightSizeClass::Compact),
            (479.0, WindowHeightSizeClass::Compact),
            (480.0, WindowHeightSizeClass::Medium),
            (899.0, WindowHeightSizeClass::Medium),
            (900.0, WindowHeightSizeClass::Expanded),
            (2000.0, WindowHeightSizeClass::Expanded),
        ] {
            assert_eq!(
                WindowSizeClass::from_size(Size::new(0.0, height)).height,
                class,
                "height {height}"
            );
        }
    }
}
