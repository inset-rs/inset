//! Flutter counterpart: `cupertino/constants.dart`.
//!
//! Dart's `const Map<CupertinoButtonSize, V>` tables are `const fn` lookups over the enum:
//! Rust has no `const` map, and the enum is exhaustive, so the lookup is total.

use inset_embedder::Radius;
use inset_painting::{BorderRadius, EdgeInsetsGeometry};

use crate::button::CupertinoButtonSize;

/// The minimum dimension of any interactive region according to the iOS Human
/// Interface Guidelines.
///
/// This is used to avoid small regions that are hard for the user to interact
/// with. It applies to both dimensions of a region, so a square of size
/// kMinInteractiveDimension x kMinInteractiveDimension is the smallest
/// acceptable region that should respond to gestures.
///
/// See also:
///
///  * `kMinInteractiveDimension`
///  * <https://developer.apple.com/ios/human-interface-guidelines/visual-design/adaptivity-and-layout/>
pub const K_MIN_INTERACTIVE_DIMENSION_CUPERTINO: f64 = 44.0;

/// The relative values needed to transform a color to it's equivalent focus
/// outline color.
///
/// These are used to draw a focus ring around `CupertinoSwitch`,
/// `CupertinoCheckbox`, `CupertinoRadio` and `CupertinoButton`.
///
/// See also:
///
/// * <https://developer.apple.com/design/human-interface-guidelines/focus-and-selection/>
pub const K_CUPERTINO_FOCUS_COLOR_OPACITY: f64 = 0.80;
/// See [`K_CUPERTINO_FOCUS_COLOR_OPACITY`].
pub const K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS: f64 = 0.69;
/// See [`K_CUPERTINO_FOCUS_COLOR_OPACITY`].
pub const K_CUPERTINO_FOCUS_COLOR_SATURATION: f64 = 0.835;

/// Opacity values for the background of a `CupertinoButton.tinted`.
///
/// See also:
///
/// * <https://developer.apple.com/design/human-interface-guidelines/buttons#iOS-iPadOS>
pub const K_CUPERTINO_BUTTON_TINTED_OPACITY_LIGHT: f64 = 0.12;
/// See [`K_CUPERTINO_BUTTON_TINTED_OPACITY_LIGHT`].
pub const K_CUPERTINO_BUTTON_TINTED_OPACITY_DARK: f64 = 0.26;

/// The default value for `IconThemeData.size` of `CupertinoButton.child`.
///
/// Set to match the most-frequent size of icons in iOS (matches md/lg).
///
/// Used only when the `CupertinoTextThemeData.actionTextStyle` or
/// `CupertinoTextThemeData.actionSmallTextStyle` has a null `TextStyle.fontSize`.
pub const K_CUPERTINO_BUTTON_DEFAULT_ICON_SIZE: f64 = 20.0;

/// The padding values for the different [`CupertinoButtonSize`]s.
///
/// Based on the iOS (17) [Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/buttons#iOS-iPadOS).
pub const fn k_cupertino_button_padding(size: CupertinoButtonSize) -> EdgeInsetsGeometry {
    match size {
        CupertinoButtonSize::Small => EdgeInsetsGeometry::symmetric(6.0, 12.0),
        CupertinoButtonSize::Medium => EdgeInsetsGeometry::symmetric(10.0, 15.0),
        CupertinoButtonSize::Large => EdgeInsetsGeometry::symmetric(16.0, 20.0),
    }
}

/// The border radius values for the different [`CupertinoButtonSize`]s.
///
/// Based on the iOS (17) [Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/buttons#iOS-iPadOS).
pub const fn k_cupertino_button_size_border_radius(size: CupertinoButtonSize) -> BorderRadius {
    match size {
        CupertinoButtonSize::Small => BorderRadius::all(Radius::circular(40.0)),
        CupertinoButtonSize::Medium => BorderRadius::all(Radius::circular(40.0)),
        CupertinoButtonSize::Large => BorderRadius::all(Radius::circular(12.0)),
    }
}

/// The minimum size of a `CupertinoButton` based on the [`CupertinoButtonSize`].
///
/// Based on the iOS (17) [Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/buttons#iOS-iPadOS).
pub const fn k_cupertino_button_min_size(size: CupertinoButtonSize) -> f64 {
    match size {
        CupertinoButtonSize::Small => 28.0,
        CupertinoButtonSize::Medium => 32.0,
        CupertinoButtonSize::Large => 44.0,
    }
}

/// The distance a button needs to be moved after being pressed for its opacity to change.
///
/// The opacity changes when the position moved is this distance away from the button.
/// This variable is effective on mobile platforms. For desktop platforms, a distance of 0 is used.
///
/// This value was obtained through actual testing on an iOS 18.1 simulator.
pub const K_CUPERTINO_BUTTON_TAP_MOVE_SLOP: f64 = 70.0;
