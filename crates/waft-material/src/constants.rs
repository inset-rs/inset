//! Port of material_ui's `constants.dart`.

use std::time::Duration;

use waft_embedder::Color;
use waft_painting::EdgeInsets;

/// The minimum dimension of any interactive region according to Material
/// guidelines.
///
/// This is used to avoid small regions that are hard for the user to interact
/// with. It applies to both dimensions of a region, so a square of size
/// kMinInteractiveDimension x kMinInteractiveDimension is the smallest
/// acceptable region that should respond to gestures.
///
/// See also:
///
///  * `kMinInteractiveDimensionCupertino`
///  * The Material spec on touch targets at <https://material.io/design/usability/accessibility.html#layout-typography>.
pub const K_MIN_INTERACTIVE_DIMENSION: f64 = 48.0;

/// The height of the toolbar component of the `AppBar`.
pub const K_TOOLBAR_HEIGHT: f64 = 56.0;

/// The height of the bottom navigation bar.
pub const K_BOTTOM_NAVIGATION_BAR_HEIGHT: f64 = 56.0;

/// The height of a tab bar containing text.
pub const K_TEXT_TAB_BAR_HEIGHT: f64 = K_MIN_INTERACTIVE_DIMENSION;

/// The amount of time theme change animations should last.
pub const K_THEME_CHANGE_DURATION: Duration = Duration::from_millis(200);

/// The default radius of a circular material ink response in logical pixels.
pub const K_RADIAL_REACTION_RADIUS: f64 = 20.0;

/// The amount of time a circular material ink response should take to expand to its full size.
pub const K_RADIAL_REACTION_DURATION: Duration = Duration::from_millis(100);

/// The value of the alpha channel to use when drawing a circular material ink response.
pub const K_RADIAL_REACTION_ALPHA: i32 = 0x1F;

/// The duration of the horizontal scroll animation that occurs when a tab is tapped.
pub const K_TAB_SCROLL_DURATION: Duration = Duration::from_millis(300);

/// The horizontal padding included by `Tab`s.
pub const K_TAB_LABEL_PADDING: EdgeInsets = EdgeInsets::symmetric(0.0, 16.0);

/// The padding added around material list items.
pub const K_MATERIAL_LIST_PADDING: EdgeInsets = EdgeInsets::symmetric(8.0, 0.0);

/// The default color for `ThemeData.iconTheme` when `ThemeData.brightness` is
/// `Brightness.dark`. This color is used in `IconButton` to detect whether
/// `IconTheme.of(context).color` is the same as the default color of `ThemeData.iconTheme`.
pub const K_DEFAULT_ICON_LIGHT_COLOR: Color = Color::new(0xFFFFFFFF);

/// The default color for `ThemeData.iconTheme` when `ThemeData.brightness` is
/// `Brightness.light`. This color is used in `IconButton` to detect whether
/// `IconTheme.of(context).color` is the same as the default color of `ThemeData.iconTheme`.
pub const K_DEFAULT_ICON_DARK_COLOR: Color = Color::new(0xDD000000);
