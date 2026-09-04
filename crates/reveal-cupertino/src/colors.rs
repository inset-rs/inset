//! Flutter counterpart: `cupertino/colors.dart`.
//!
//! Dart's `CupertinoDynamicColor implements Color`; here it is a [`ColorExtension`] carried
//! by an [`AnyColor`] whose value is the effective color (see reveal-painting's
//! `PORTING.md`). `createCupertinoColorProperty` and `debugFillProperties` are diagnostics.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{Brightness, Color};
use reveal_foundation::App;
use reveal_painting::{AnyColor, ColorExtension};
use reveal_widgets::{BuildContext, MediaQuery};

use crate::interface_level::{CupertinoUserInterfaceLevel, CupertinoUserInterfaceLevelData};
use crate::theme::CupertinoTheme;

/// A palette of [`Color`] constants that describe colors commonly used when
/// matching the iOS platform aesthetics.
///
/// ## Color palettes
///
/// ### Basic Colors
/// ![](https://flutter.github.io/assets-for-api-docs/assets/cupertino/cupertino_basic_colors.png)
///
/// ### Active Colors
/// ![](https://flutter.github.io/assets-for-api-docs/assets/cupertino/cupertino_active_colors.png)
///
/// ### System Colors
/// ![](https://flutter.github.io/assets-for-api-docs/assets/cupertino/cupertino_system_colors_1.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/cupertino/cupertino_system_colors_2.png)
/// ![](https://flutter.github.io/assets-for-api-docs/assets/cupertino/cupertino_system_colors_3.png)
///
/// ### Label Colors
/// ![](https://flutter.github.io/assets-for-api-docs/assets/cupertino/cupertino_label_colors.png)
///
/// ### Background Colors
/// ![](https://flutter.github.io/assets-for-api-docs/assets/cupertino/cupertino_background_colors.png)
///
/// Every entry is an [`AnyColor`]: a plain color, or a [`CupertinoDynamicColor`] recovered
/// with `extension::<CupertinoDynamicColor>()`.
pub struct CupertinoColors;

impl CupertinoColors {
    /// iOS 13's default blue color. Used to indicate active elements such as
    /// buttons, selected tabs and your own chat bubbles.
    ///
    /// This is SystemBlue in the iOS palette.
    pub const ACTIVE_BLUE: AnyColor = Self::SYSTEM_BLUE;

    /// iOS 13's default green color. Used to indicate active accents such as
    /// the switch in its on state and some accent buttons such as the call button
    /// and Apple Map's 'Go' button.
    ///
    /// This is SystemGreen in the iOS palette.
    pub const ACTIVE_GREEN: AnyColor = Self::SYSTEM_GREEN;

    /// iOS 13's orange color.
    ///
    /// This is SystemOrange in the iOS palette.
    pub const ACTIVE_ORANGE: AnyColor = Self::SYSTEM_ORANGE;

    /// Opaque white color. Used for backgrounds and fonts against dark backgrounds.
    ///
    /// This is SystemWhiteColor in the iOS palette.
    ///
    /// See also:
    ///
    ///  * `Colors.white`, the same color, in the Material Design palette.
    ///  * [`BLACK`](Self::BLACK), opaque black in the [`CupertinoColors`] palette.
    pub const WHITE: AnyColor = AnyColor::new(Color::new(0xFFFFFFFF));

    /// Opaque black color. Used for texts against light backgrounds.
    ///
    /// This is SystemBlackColor in the iOS palette.
    ///
    /// See also:
    ///
    ///  * `Colors.black`, the same color, in the Material Design palette.
    ///  * [`WHITE`](Self::WHITE), opaque white in the [`CupertinoColors`] palette.
    pub const BLACK: AnyColor = AnyColor::new(Color::new(0xFF000000));

    /// A fully-transparent color, completely invisible.
    ///
    /// See also:
    ///
    ///  * `Colors.transparent`, the same color, in the Material Design palette.
    pub const TRANSPARENT: AnyColor = AnyColor::new(Color::new(0x00000000));

    /// Used in iOS 10 for light background fills such as the chat bubble background.
    ///
    /// This is SystemLightGrayColor in the iOS palette.
    pub const LIGHT_BACKGROUND_GRAY: AnyColor = AnyColor::new(Color::new(0xFFE5E5EA));

    /// Used in iOS 12 for very light background fills in tables between cell groups.
    ///
    /// This is SystemExtraLightGrayColor in the iOS palette.
    pub const EXTRA_LIGHT_BACKGROUND_GRAY: AnyColor = AnyColor::new(Color::new(0xFFEFEFF4));

    /// Used in iOS 12 for very dark background fills in tables between cell groups
    /// in dark mode.
    // Value derived from screenshot from the dark themed Apple Watch app.
    pub const DARK_BACKGROUND_GRAY: AnyColor = AnyColor::new(Color::new(0xFF171717));

    const INACTIVE_GRAY_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness(Color::new(0xFF999999), Color::new(0xFF757575))
            .debug_label("inactiveGray");
    /// Used in iOS 13 for unselected selectables such as tab bar items in their
    /// inactive state or de-emphasized subtitles and details text.
    ///
    /// Not the same grey as disabled buttons etc.
    ///
    /// This is the disabled color in the iOS palette.
    pub const INACTIVE_GRAY: AnyColor = Self::INACTIVE_GRAY_DYNAMIC.to_any();

    /// Used for iOS 13 for destructive actions such as the delete actions in
    /// table view cells and dialogs.
    ///
    /// Not the same red as the camera shutter or springboard icon notifications
    /// or the foreground red theme in various native apps such as HealthKit.
    ///
    /// This is SystemRed in the iOS palette.
    pub const DESTRUCTIVE_RED: AnyColor = Self::SYSTEM_RED;

    const SYSTEM_BLUE_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 0, 122, 255),
            Color::from_argb(255, 10, 132, 255),
            Color::from_argb(255, 0, 64, 221),
            Color::from_argb(255, 64, 156, 255),
        )
        .debug_label("systemBlue");
    /// A blue color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemBlue](https://developer.apple.com/documentation/uikit/uicolor/3173141-systemblue),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_BLUE: AnyColor = Self::SYSTEM_BLUE_DYNAMIC.to_any();

    const SYSTEM_GREEN_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 52, 199, 89),
            Color::from_argb(255, 48, 209, 88),
            Color::from_argb(255, 36, 138, 61),
            Color::from_argb(255, 48, 219, 91),
        )
        .debug_label("systemGreen");
    /// A green color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemGreen](https://developer.apple.com/documentation/uikit/uicolor/3173144-systemgreen),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_GREEN: AnyColor = Self::SYSTEM_GREEN_DYNAMIC.to_any();

    const SYSTEM_MINT_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 0, 199, 190),
            Color::from_argb(255, 99, 230, 226),
            Color::from_argb(255, 12, 129, 123),
            Color::from_argb(255, 102, 212, 207),
        )
        .debug_label("systemMint");
    /// A mint color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemMint](https://developer.apple.com/documentation/uikit/uicolor/3852741-systemmint),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_MINT: AnyColor = Self::SYSTEM_MINT_DYNAMIC.to_any();

    const SYSTEM_INDIGO_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 88, 86, 214),
            Color::from_argb(255, 94, 92, 230),
            Color::from_argb(255, 54, 52, 163),
            Color::from_argb(255, 125, 122, 255),
        )
        .debug_label("systemIndigo");
    /// An indigo color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemIndigo](https://developer.apple.com/documentation/uikit/uicolor/3173146-systemindigo),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_INDIGO: AnyColor = Self::SYSTEM_INDIGO_DYNAMIC.to_any();

    const SYSTEM_ORANGE_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 255, 149, 0),
            Color::from_argb(255, 255, 159, 10),
            Color::from_argb(255, 201, 52, 0),
            Color::from_argb(255, 255, 179, 64),
        )
        .debug_label("systemOrange");
    /// An orange color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemOrange](https://developer.apple.com/documentation/uikit/uicolor/3173147-systemorange),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_ORANGE: AnyColor = Self::SYSTEM_ORANGE_DYNAMIC.to_any();

    const SYSTEM_PINK_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 255, 45, 85),
            Color::from_argb(255, 255, 55, 95),
            Color::from_argb(255, 211, 15, 69),
            Color::from_argb(255, 255, 100, 130),
        )
        .debug_label("systemPink");
    /// A pink color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemPink](https://developer.apple.com/documentation/uikit/uicolor/3173148-systempink),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_PINK: AnyColor = Self::SYSTEM_PINK_DYNAMIC.to_any();

    const SYSTEM_BROWN_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 162, 132, 94),
            Color::from_argb(255, 172, 142, 104),
            Color::from_argb(255, 127, 101, 69),
            Color::from_argb(255, 181, 148, 105),
        )
        .debug_label("systemBrown");
    /// A brown color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemBrown](https://developer.apple.com/documentation/uikit/uicolor/3173142-systembrown),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_BROWN: AnyColor = Self::SYSTEM_BROWN_DYNAMIC.to_any();

    const SYSTEM_PURPLE_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 175, 82, 222),
            Color::from_argb(255, 191, 90, 242),
            Color::from_argb(255, 137, 68, 171),
            Color::from_argb(255, 218, 143, 255),
        )
        .debug_label("systemPurple");
    /// A purple color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemPurple](https://developer.apple.com/documentation/uikit/uicolor/3173149-systempurple),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_PURPLE: AnyColor = Self::SYSTEM_PURPLE_DYNAMIC.to_any();

    const SYSTEM_RED_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 255, 59, 48),
            Color::from_argb(255, 255, 69, 58),
            Color::from_argb(255, 215, 0, 21),
            Color::from_argb(255, 255, 105, 97),
        )
        .debug_label("systemRed");
    /// A red color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemRed](https://developer.apple.com/documentation/uikit/uicolor/3173150-systemred),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_RED: AnyColor = Self::SYSTEM_RED_DYNAMIC.to_any();

    const SYSTEM_TEAL_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 90, 200, 250),
            Color::from_argb(255, 100, 210, 255),
            Color::from_argb(255, 0, 113, 164),
            Color::from_argb(255, 112, 215, 255),
        )
        .debug_label("systemTeal");
    /// A teal color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemTeal](https://developer.apple.com/documentation/uikit/uicolor/3173151-systemteal),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_TEAL: AnyColor = Self::SYSTEM_TEAL_DYNAMIC.to_any();

    const SYSTEM_CYAN_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 50, 173, 230),
            Color::from_argb(255, 100, 210, 255),
            Color::from_argb(255, 0, 113, 164),
            Color::from_argb(255, 112, 215, 255),
        )
        .debug_label("systemCyan");
    /// A cyan color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemCyan](https://developer.apple.com/documentation/uikit/uicolor/3852740-systemcyan),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_CYAN: AnyColor = Self::SYSTEM_CYAN_DYNAMIC.to_any();

    const SYSTEM_YELLOW_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 255, 204, 0),
            Color::from_argb(255, 255, 214, 10),
            Color::from_argb(255, 160, 90, 0),
            Color::from_argb(255, 255, 212, 38),
        )
        .debug_label("systemYellow");
    /// A yellow color that can adapt to the given [`BuildContext`].
    ///
    /// See also:
    ///
    ///  * [UIColor.systemYellow](https://developer.apple.com/documentation/uikit/uicolor/3173152-systemyellow),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_YELLOW: AnyColor = Self::SYSTEM_YELLOW_DYNAMIC.to_any();

    const SYSTEM_GREY_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 142, 142, 147),
            Color::from_argb(255, 142, 142, 147),
            Color::from_argb(255, 108, 108, 112),
            Color::from_argb(255, 174, 174, 178),
        )
        .debug_label("systemGrey");
    /// The base grey color.
    ///
    /// See also:
    ///
    ///  * [UIColor.systemGray](https://developer.apple.com/documentation/uikit/uicolor/3173143-systemgray),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_GREY: AnyColor = Self::SYSTEM_GREY_DYNAMIC.to_any();

    const SYSTEM_GREY2_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 174, 174, 178),
            Color::from_argb(255, 99, 99, 102),
            Color::from_argb(255, 142, 142, 147),
            Color::from_argb(255, 124, 124, 128),
        )
        .debug_label("systemGrey2");
    /// A second-level shade of grey.
    ///
    /// See also:
    ///
    ///  * [UIColor.systemGray2](https://developer.apple.com/documentation/uikit/uicolor/3255071-systemgray2),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_GREY2: AnyColor = Self::SYSTEM_GREY2_DYNAMIC.to_any();

    const SYSTEM_GREY3_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 199, 199, 204),
            Color::from_argb(255, 72, 72, 74),
            Color::from_argb(255, 174, 174, 178),
            Color::from_argb(255, 84, 84, 86),
        )
        .debug_label("systemGrey3");
    /// A third-level shade of grey.
    ///
    /// See also:
    ///
    ///  * [UIColor.systemGray3](https://developer.apple.com/documentation/uikit/uicolor/3255072-systemgray3),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_GREY3: AnyColor = Self::SYSTEM_GREY3_DYNAMIC.to_any();

    const SYSTEM_GREY4_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 209, 209, 214),
            Color::from_argb(255, 58, 58, 60),
            Color::from_argb(255, 188, 188, 192),
            Color::from_argb(255, 68, 68, 70),
        )
        .debug_label("systemGrey4");
    /// A fourth-level shade of grey.
    ///
    /// See also:
    ///
    ///  * [UIColor.systemGray4](https://developer.apple.com/documentation/uikit/uicolor/3255073-systemgray4),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_GREY4: AnyColor = Self::SYSTEM_GREY4_DYNAMIC.to_any();

    const SYSTEM_GREY5_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 229, 229, 234),
            Color::from_argb(255, 44, 44, 46),
            Color::from_argb(255, 216, 216, 220),
            Color::from_argb(255, 54, 54, 56),
        )
        .debug_label("systemGrey5");
    /// A fifth-level shade of grey.
    ///
    /// See also:
    ///
    ///  * [UIColor.systemGray5](https://developer.apple.com/documentation/uikit/uicolor/3255074-systemgray5),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_GREY5: AnyColor = Self::SYSTEM_GREY5_DYNAMIC.to_any();

    const SYSTEM_GREY6_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 242, 242, 247),
            Color::from_argb(255, 28, 28, 30),
            Color::from_argb(255, 235, 235, 240),
            Color::from_argb(255, 36, 36, 38),
        )
        .debug_label("systemGrey6");
    /// A sixth-level shade of grey.
    ///
    /// See also:
    ///
    ///  * [UIColor.systemGray6](https://developer.apple.com/documentation/uikit/uicolor/3255075-systemgray6),
    ///    the `UIKit` equivalent.
    pub const SYSTEM_GREY6: AnyColor = Self::SYSTEM_GREY6_DYNAMIC.to_any();

    const LABEL_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(255, 0, 0, 0),
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 0, 0, 0),
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 0, 0, 0),
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 0, 0, 0),
        Color::from_argb(255, 255, 255, 255),
    )
    .debug_label("label");
    /// The color for text labels containing primary content, equivalent to
    /// [UIColor.label](https://developer.apple.com/documentation/uikit/uicolor/3173131-label).
    pub const LABEL: AnyColor = Self::LABEL_DYNAMIC.to_any();

    const SECONDARY_LABEL_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(153, 60, 60, 67),
        Color::from_argb(153, 235, 235, 245),
        Color::from_argb(173, 60, 60, 67),
        Color::from_argb(173, 235, 235, 245),
        Color::from_argb(153, 60, 60, 67),
        Color::from_argb(153, 235, 235, 245),
        Color::from_argb(173, 60, 60, 67),
        Color::from_argb(173, 235, 235, 245),
    )
    .debug_label("secondaryLabel");
    /// The color for text labels containing secondary content, equivalent to
    /// [UIColor.secondaryLabel](https://developer.apple.com/documentation/uikit/uicolor/3173136-secondarylabel).
    pub const SECONDARY_LABEL: AnyColor = Self::SECONDARY_LABEL_DYNAMIC.to_any();

    const TERTIARY_LABEL_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(76, 60, 60, 67),
        Color::from_argb(76, 235, 235, 245),
        Color::from_argb(96, 60, 60, 67),
        Color::from_argb(96, 235, 235, 245),
        Color::from_argb(76, 60, 60, 67),
        Color::from_argb(76, 235, 235, 245),
        Color::from_argb(96, 60, 60, 67),
        Color::from_argb(96, 235, 235, 245),
    )
    .debug_label("tertiaryLabel");
    /// The color for text labels containing tertiary content, equivalent to
    /// [UIColor.tertiaryLabel](https://developer.apple.com/documentation/uikit/uicolor/3173153-tertiarylabel).
    pub const TERTIARY_LABEL: AnyColor = Self::TERTIARY_LABEL_DYNAMIC.to_any();

    const QUATERNARY_LABEL_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(45, 60, 60, 67),
        Color::from_argb(40, 235, 235, 245),
        Color::from_argb(66, 60, 60, 67),
        Color::from_argb(61, 235, 235, 245),
        Color::from_argb(45, 60, 60, 67),
        Color::from_argb(40, 235, 235, 245),
        Color::from_argb(66, 60, 60, 67),
        Color::from_argb(61, 235, 235, 245),
    )
    .debug_label("quaternaryLabel");
    /// The color for text labels containing quaternary content, equivalent to
    /// [UIColor.quaternaryLabel](https://developer.apple.com/documentation/uikit/uicolor/3173135-quaternarylabel).
    pub const QUATERNARY_LABEL: AnyColor = Self::QUATERNARY_LABEL_DYNAMIC.to_any();

    const SYSTEM_FILL_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(51, 120, 120, 128),
        Color::from_argb(91, 120, 120, 128),
        Color::from_argb(71, 120, 120, 128),
        Color::from_argb(112, 120, 120, 128),
        Color::from_argb(51, 120, 120, 128),
        Color::from_argb(91, 120, 120, 128),
        Color::from_argb(71, 120, 120, 128),
        Color::from_argb(112, 120, 120, 128),
    )
    .debug_label("systemFill");
    /// An overlay fill color for thin and small shapes, equivalent to
    /// [UIColor.systemFill](https://developer.apple.com/documentation/uikit/uicolor/3255070-systemfill).
    pub const SYSTEM_FILL: AnyColor = Self::SYSTEM_FILL_DYNAMIC.to_any();

    const SECONDARY_SYSTEM_FILL_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(40, 120, 120, 128),
        Color::from_argb(81, 120, 120, 128),
        Color::from_argb(61, 120, 120, 128),
        Color::from_argb(102, 120, 120, 128),
        Color::from_argb(40, 120, 120, 128),
        Color::from_argb(81, 120, 120, 128),
        Color::from_argb(61, 120, 120, 128),
        Color::from_argb(102, 120, 120, 128),
    )
    .debug_label("secondarySystemFill");
    /// An overlay fill color for medium-size shapes, equivalent to
    /// [UIColor.secondarySystemFill](https://developer.apple.com/documentation/uikit/uicolor/3255069-secondarysystemfill).
    pub const SECONDARY_SYSTEM_FILL: AnyColor = Self::SECONDARY_SYSTEM_FILL_DYNAMIC.to_any();

    const TERTIARY_SYSTEM_FILL_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(30, 118, 118, 128),
        Color::from_argb(61, 118, 118, 128),
        Color::from_argb(51, 118, 118, 128),
        Color::from_argb(81, 118, 118, 128),
        Color::from_argb(30, 118, 118, 128),
        Color::from_argb(61, 118, 118, 128),
        Color::from_argb(51, 118, 118, 128),
        Color::from_argb(81, 118, 118, 128),
    )
    .debug_label("tertiarySystemFill");
    /// An overlay fill color for large shapes, equivalent to
    /// [UIColor.tertiarySystemFill](https://developer.apple.com/documentation/uikit/uicolor/3255076-tertiarysystemfill).
    pub const TERTIARY_SYSTEM_FILL: AnyColor = Self::TERTIARY_SYSTEM_FILL_DYNAMIC.to_any();

    const QUATERNARY_SYSTEM_FILL_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(20, 116, 116, 128),
        Color::from_argb(45, 118, 118, 128),
        Color::from_argb(40, 116, 116, 128),
        Color::from_argb(66, 118, 118, 128),
        Color::from_argb(20, 116, 116, 128),
        Color::from_argb(45, 118, 118, 128),
        Color::from_argb(40, 116, 116, 128),
        Color::from_argb(66, 118, 118, 128),
    )
    .debug_label("quaternarySystemFill");
    /// An overlay fill color for large areas containing complex content, equivalent
    /// to [UIColor.quaternarySystemFill](https://developer.apple.com/documentation/uikit/uicolor/3255068-quaternarysystemfill).
    pub const QUATERNARY_SYSTEM_FILL: AnyColor = Self::QUATERNARY_SYSTEM_FILL_DYNAMIC.to_any();

    const PLACEHOLDER_TEXT_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(76, 60, 60, 67),
        Color::from_argb(76, 235, 235, 245),
        Color::from_argb(96, 60, 60, 67),
        Color::from_argb(96, 235, 235, 245),
        Color::from_argb(76, 60, 60, 67),
        Color::from_argb(76, 235, 235, 245),
        Color::from_argb(96, 60, 60, 67),
        Color::from_argb(96, 235, 235, 245),
    )
    .debug_label("placeholderText");
    /// The color for placeholder text in controls or text views, equivalent to
    /// [UIColor.placeholderText](https://developer.apple.com/documentation/uikit/uicolor/3173134-placeholdertext).
    pub const PLACEHOLDER_TEXT: AnyColor = Self::PLACEHOLDER_TEXT_DYNAMIC.to_any();

    const SYSTEM_BACKGROUND_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 0, 0, 0),
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 0, 0, 0),
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 28, 28, 30),
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 36, 36, 38),
    )
    .debug_label("systemBackground");
    /// The color for the main background of your interface, equivalent to
    /// [UIColor.systemBackground](https://developer.apple.com/documentation/uikit/uicolor/3173140-systembackground).
    ///
    /// Typically used for designs that have a white primary background in a light environment.
    pub const SYSTEM_BACKGROUND: AnyColor = Self::SYSTEM_BACKGROUND_DYNAMIC.to_any();

    const SECONDARY_SYSTEM_BACKGROUND_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(255, 242, 242, 247),
        Color::from_argb(255, 28, 28, 30),
        Color::from_argb(255, 235, 235, 240),
        Color::from_argb(255, 36, 36, 38),
        Color::from_argb(255, 242, 242, 247),
        Color::from_argb(255, 44, 44, 46),
        Color::from_argb(255, 235, 235, 240),
        Color::from_argb(255, 54, 54, 56),
    )
    .debug_label("secondarySystemBackground");
    /// The color for content layered on top of the main background, equivalent to
    /// [UIColor.secondarySystemBackground](https://developer.apple.com/documentation/uikit/uicolor/3173137-secondarysystembackground).
    ///
    /// Typically used for designs that have a white primary background in a light environment.
    pub const SECONDARY_SYSTEM_BACKGROUND: AnyColor =
        Self::SECONDARY_SYSTEM_BACKGROUND_DYNAMIC.to_any();

    const TERTIARY_SYSTEM_BACKGROUND_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 44, 44, 46),
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 54, 54, 56),
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 58, 58, 60),
        Color::from_argb(255, 255, 255, 255),
        Color::from_argb(255, 68, 68, 70),
    )
    .debug_label("tertiarySystemBackground");
    /// The color for content layered on top of secondary backgrounds, equivalent
    /// to [UIColor.tertiarySystemBackground](https://developer.apple.com/documentation/uikit/uicolor/3173154-tertiarysystembackground).
    ///
    /// Typically used for designs that have a white primary background in a light environment.
    pub const TERTIARY_SYSTEM_BACKGROUND: AnyColor =
        Self::TERTIARY_SYSTEM_BACKGROUND_DYNAMIC.to_any();

    const SYSTEM_GROUPED_BACKGROUND_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(255, 242, 242, 247),
        Color::from_argb(255, 0, 0, 0),
        Color::from_argb(255, 235, 235, 240),
        Color::from_argb(255, 0, 0, 0),
        Color::from_argb(255, 242, 242, 247),
        Color::from_argb(255, 28, 28, 30),
        Color::from_argb(255, 235, 235, 240),
        Color::from_argb(255, 36, 36, 38),
    )
    .debug_label("systemGroupedBackground");
    /// The color for the main background of your grouped interface, equivalent to
    /// [UIColor.systemGroupedBackground](https://developer.apple.com/documentation/uikit/uicolor/3173145-systemgroupedbackground).
    ///
    /// Typically used for grouped content, including table views and platter-based designs.
    pub const SYSTEM_GROUPED_BACKGROUND: AnyColor =
        Self::SYSTEM_GROUPED_BACKGROUND_DYNAMIC.to_any();

    const SECONDARY_SYSTEM_GROUPED_BACKGROUND_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::new(
            Color::from_argb(255, 255, 255, 255),
            Color::from_argb(255, 28, 28, 30),
            Color::from_argb(255, 255, 255, 255),
            Color::from_argb(255, 36, 36, 38),
            Color::from_argb(255, 255, 255, 255),
            Color::from_argb(255, 44, 44, 46),
            Color::from_argb(255, 255, 255, 255),
            Color::from_argb(255, 54, 54, 56),
        )
        .debug_label("secondarySystemGroupedBackground");
    /// The color for content layered on top of the main background of your grouped interface,
    /// equivalent to [UIColor.secondarySystemGroupedBackground](https://developer.apple.com/documentation/uikit/uicolor/3173138-secondarysystemgroupedbackground).
    ///
    /// Typically used for grouped content, including table views and platter-based designs.
    pub const SECONDARY_SYSTEM_GROUPED_BACKGROUND: AnyColor =
        Self::SECONDARY_SYSTEM_GROUPED_BACKGROUND_DYNAMIC.to_any();

    const TERTIARY_SYSTEM_GROUPED_BACKGROUND_DYNAMIC: CupertinoDynamicColor =
        CupertinoDynamicColor::new(
            Color::from_argb(255, 242, 242, 247),
            Color::from_argb(255, 44, 44, 46),
            Color::from_argb(255, 235, 235, 240),
            Color::from_argb(255, 54, 54, 56),
            Color::from_argb(255, 242, 242, 247),
            Color::from_argb(255, 58, 58, 60),
            Color::from_argb(255, 235, 235, 240),
            Color::from_argb(255, 68, 68, 70),
        )
        .debug_label("tertiarySystemGroupedBackground");
    /// The color for content layered on top of secondary backgrounds of your grouped interface,
    /// equivalent to [UIColor.tertiarySystemGroupedBackground](https://developer.apple.com/documentation/uikit/uicolor/3173155-tertiarysystemgroupedbackground).
    ///
    /// Typically used for grouped content, including table views and platter-based designs.
    pub const TERTIARY_SYSTEM_GROUPED_BACKGROUND: AnyColor =
        Self::TERTIARY_SYSTEM_GROUPED_BACKGROUND_DYNAMIC.to_any();

    const SEPARATOR_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(73, 60, 60, 67),
        Color::from_argb(153, 84, 84, 88),
        Color::from_argb(94, 60, 60, 67),
        Color::from_argb(173, 84, 84, 88),
        Color::from_argb(73, 60, 60, 67),
        Color::from_argb(153, 210, 210, 210),
        Color::from_argb(94, 60, 60, 67),
        Color::from_argb(173, 84, 84, 88),
    )
    .debug_label("separator");
    /// The color for thin borders or divider lines that allows some underlying content to be visible,
    /// equivalent to [UIColor.separator](https://developer.apple.com/documentation/uikit/uicolor/3173139-separator).
    pub const SEPARATOR: AnyColor = Self::SEPARATOR_DYNAMIC.to_any();

    const OPAQUE_SEPARATOR_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(255, 198, 198, 200),
        Color::from_argb(255, 56, 56, 58),
        Color::from_argb(255, 198, 198, 200),
        Color::from_argb(255, 56, 56, 58),
        Color::from_argb(255, 198, 198, 200),
        Color::from_argb(255, 56, 56, 58),
        Color::from_argb(255, 198, 198, 200),
        Color::from_argb(255, 56, 56, 58),
    )
    .debug_label("opaqueSeparator");
    /// The color for borders or divider lines that hide any underlying content,
    /// equivalent to [UIColor.opaqueSeparator](https://developer.apple.com/documentation/uikit/uicolor/3173133-opaqueseparator).
    pub const OPAQUE_SEPARATOR: AnyColor = Self::OPAQUE_SEPARATOR_DYNAMIC.to_any();

    const LINK_DYNAMIC: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::from_argb(255, 0, 122, 255),
        Color::from_argb(255, 9, 132, 255),
        Color::from_argb(255, 0, 122, 255),
        Color::from_argb(255, 9, 132, 255),
        Color::from_argb(255, 0, 122, 255),
        Color::from_argb(255, 9, 132, 255),
        Color::from_argb(255, 0, 122, 255),
        Color::from_argb(255, 9, 132, 255),
    )
    .debug_label("link");
    /// The color for links, equivalent to
    /// [UIColor.link](https://developer.apple.com/documentation/uikit/uicolor/3173132-link).
    pub const LINK: AnyColor = Self::LINK_DYNAMIC.to_any();
}

/// A [`Color`] subclass that represents a family of colors, and the correct effective
/// color in the color family.
///
/// When used as a regular color, [`CupertinoDynamicColor`] is equivalent to the
/// effective color (i.e. the [`AnyColor`] carrying it holds the effective color, and its
/// `value` comes from there), which is determined by the [`BuildContext`] it is last resolved
/// against. If it has never been resolved, the light, normal contrast, base elevation variant
/// [`CupertinoDynamicColor::color`] will be the default effective color.
///
/// Sometimes manually resolving a [`CupertinoDynamicColor`] is not necessary, because
/// the Cupertino Library provides built-in support for it.
///
/// ### Using [`CupertinoDynamicColor`] in a Cupertino widget
///
/// When a Cupertino widget is provided with a [`CupertinoDynamicColor`], either
/// directly in its constructor, or from an `InheritedWidget` it depends on (for example,
/// `DefaultTextStyle`), the widget will automatically resolve the color using
/// [`CupertinoDynamicColor::resolve`] against its own [`BuildContext`], on a best-effort
/// basis.
///
/// By default a `CupertinoButton` has no background color. The following sample
/// code shows how to build a `CupertinoButton` that appears white in light mode,
/// and changes automatically to black in dark mode.
///
/// ```text
/// CupertinoButton::new(child, on_pressed)
///     // CupertinoDynamicColor works out of box in a CupertinoButton.
///     .color(CupertinoDynamicColor::with_brightness(
///         CupertinoColors::WHITE.color(),
///         CupertinoColors::BLACK.color(),
///     ))
/// ```
///
/// ### Using a [`CupertinoDynamicColor`] from a `CupertinoTheme`
///
/// When referring to a `CupertinoTheme` color, generally the color will already
/// have adapted to the ambient [`BuildContext`], because `CupertinoTheme.of`
/// implicitly resolves all the colors used in the retrieved `CupertinoThemeData`,
/// before returning it.
///
/// The following code sample creates a `Container` with the `primary_color` of the
/// current theme. If `primary_color` is a [`CupertinoDynamicColor`], the container
/// will be adaptive, thanks to `CupertinoTheme.of`: it will switch to `primary_color`'s
/// dark variant once dark mode is turned on, and turns to `primary_color`'s high
/// contrast variant when `MediaQueryData.high_contrast` is requested in the ambient
/// `MediaQuery`, etc.
///
/// ```text
/// Container::new()
///     // Container is not a Cupertino widget, but CupertinoTheme.of implicitly
///     // resolves colors used in the retrieved CupertinoThemeData.
///     .color(CupertinoTheme::of(app, context).primary_color)
/// ```
///
/// ### Manually Resolving a [`CupertinoDynamicColor`]
///
/// When used to configure a non-Cupertino widget, or wrapped in an object opaque
/// to the receiving Cupertino component, a [`CupertinoDynamicColor`] may need to be
/// manually resolved using [`CupertinoDynamicColor::resolve`], before it can used
/// to paint. For example, to use a custom `Border` in a `CupertinoNavigationBar`,
/// the colors used in the `Border` have to be resolved manually before being passed
/// to `CupertinoNavigationBar`'s constructor.
///
/// The following code samples demonstrate two cases where you have to manually
/// resolve a [`CupertinoDynamicColor`].
///
/// ```text
/// CupertinoNavigationBar::new()
///     // CupertinoNavigationBar does not know how to resolve colors used in
///     // a Border class.
///     .border(Border::new().bottom(
///         BorderSide::new().color(
///             CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_BLUE, app, context).color(),
///         ),
///     ))
/// ```
///
/// ```text
/// Container::new()
///     // Container is not a Cupertino widget.
///     .color(CupertinoDynamicColor::resolve(&CupertinoColors::SYSTEM_BLUE, app, context))
/// ```
///
/// See also:
///
///  * [`CupertinoUserInterfaceLevel`], an `InheritedWidget` that may affect color
///    resolution of a [`CupertinoDynamicColor`].
///  * `CupertinoTheme.of`, a static method that retrieves the ambient `CupertinoThemeData`,
///    and then resolves [`CupertinoDynamicColor`]s used in the retrieved data.
#[derive(Clone, Copy)]
pub struct CupertinoDynamicColor {
    /// The current effective color.
    ///
    /// Defaults to [`color`](Self::color) if this [`CupertinoDynamicColor`] has never been
    /// resolved.
    effective_color: Color,

    debug_label: Option<&'static str>,

    /// The color to use when the [`BuildContext`] implies a combination of light mode,
    /// normal contrast, and base interface elevation.
    ///
    /// In other words, this color will be the effective color of the [`CupertinoDynamicColor`]
    /// after it is resolved against a [`BuildContext`] that:
    /// - has a `CupertinoTheme` whose `CupertinoThemeData.brightness` is [`Brightness::Light`],
    ///   or a `MediaQuery` whose `MediaQueryData.platform_brightness` is [`Brightness::Light`].
    /// - has a `MediaQuery` whose `MediaQueryData.high_contrast` is `false`.
    /// - has a [`CupertinoUserInterfaceLevel`] that indicates [`CupertinoUserInterfaceLevelData::Base`].
    pub color: Color,

    /// The color to use when the [`BuildContext`] implies a combination of dark mode,
    /// normal contrast, and base interface elevation.
    ///
    /// In other words, this color will be the effective color of the [`CupertinoDynamicColor`]
    /// after it is resolved against a [`BuildContext`] that:
    /// - has a `CupertinoTheme` whose `CupertinoThemeData.brightness` is [`Brightness::Dark`],
    ///   or a `MediaQuery` whose `MediaQueryData.platform_brightness` is [`Brightness::Dark`].
    /// - has a `MediaQuery` whose `MediaQueryData.high_contrast` is `false`.
    /// - has a [`CupertinoUserInterfaceLevel`] that indicates [`CupertinoUserInterfaceLevelData::Base`].
    pub dark_color: Color,

    /// The color to use when the [`BuildContext`] implies a combination of light mode,
    /// high contrast, and base interface elevation.
    ///
    /// In other words, this color will be the effective color of the [`CupertinoDynamicColor`]
    /// after it is resolved against a [`BuildContext`] that:
    /// - has a `CupertinoTheme` whose `CupertinoThemeData.brightness` is [`Brightness::Light`],
    ///   or a `MediaQuery` whose `MediaQueryData.platform_brightness` is [`Brightness::Light`].
    /// - has a `MediaQuery` whose `MediaQueryData.high_contrast` is `true`.
    /// - has a [`CupertinoUserInterfaceLevel`] that indicates [`CupertinoUserInterfaceLevelData::Base`].
    pub high_contrast_color: Color,

    /// The color to use when the [`BuildContext`] implies a combination of dark mode,
    /// high contrast, and base interface elevation.
    ///
    /// In other words, this color will be the effective color of the [`CupertinoDynamicColor`]
    /// after it is resolved against a [`BuildContext`] that:
    /// - has a `CupertinoTheme` whose `CupertinoThemeData.brightness` is [`Brightness::Dark`],
    ///   or a `MediaQuery` whose `MediaQueryData.platform_brightness` is [`Brightness::Dark`].
    /// - has a `MediaQuery` whose `MediaQueryData.high_contrast` is `true`.
    /// - has a [`CupertinoUserInterfaceLevel`] that indicates [`CupertinoUserInterfaceLevelData::Base`].
    pub dark_high_contrast_color: Color,

    /// The color to use when the [`BuildContext`] implies a combination of light mode,
    /// normal contrast, and elevated interface elevation.
    ///
    /// In other words, this color will be the effective color of the [`CupertinoDynamicColor`]
    /// after it is resolved against a [`BuildContext`] that:
    /// - has a `CupertinoTheme` whose `CupertinoThemeData.brightness` is [`Brightness::Light`],
    ///   or a `MediaQuery` whose `MediaQueryData.platform_brightness` is [`Brightness::Light`].
    /// - has a `MediaQuery` whose `MediaQueryData.high_contrast` is `false`.
    /// - has a [`CupertinoUserInterfaceLevel`] that indicates [`CupertinoUserInterfaceLevelData::Elevated`].
    pub elevated_color: Color,

    /// The color to use when the [`BuildContext`] implies a combination of dark mode,
    /// normal contrast, and elevated interface elevation.
    ///
    /// In other words, this color will be the effective color of the [`CupertinoDynamicColor`]
    /// after it is resolved against a [`BuildContext`] that:
    /// - has a `CupertinoTheme` whose `CupertinoThemeData.brightness` is [`Brightness::Dark`],
    ///   or a `MediaQuery` whose `MediaQueryData.platform_brightness` is [`Brightness::Dark`].
    /// - has a `MediaQuery` whose `MediaQueryData.high_contrast` is `false`.
    /// - has a [`CupertinoUserInterfaceLevel`] that indicates [`CupertinoUserInterfaceLevelData::Elevated`].
    pub dark_elevated_color: Color,

    /// The color to use when the [`BuildContext`] implies a combination of light mode,
    /// high contrast, and elevated interface elevation.
    ///
    /// In other words, this color will be the effective color of the [`CupertinoDynamicColor`]
    /// after it is resolved against a [`BuildContext`] that:
    /// - has a `CupertinoTheme` whose `CupertinoThemeData.brightness` is [`Brightness::Light`],
    ///   or a `MediaQuery` whose `MediaQueryData.platform_brightness` is [`Brightness::Light`].
    /// - has a `MediaQuery` whose `MediaQueryData.high_contrast` is `true`.
    /// - has a [`CupertinoUserInterfaceLevel`] that indicates [`CupertinoUserInterfaceLevelData::Elevated`].
    pub high_contrast_elevated_color: Color,

    /// The color to use when the [`BuildContext`] implies a combination of dark mode,
    /// high contrast, and elevated interface elevation.
    ///
    /// In other words, this color will be the effective color of the [`CupertinoDynamicColor`]
    /// after it is resolved against a [`BuildContext`] that:
    /// - has a `CupertinoTheme` whose `CupertinoThemeData.brightness` is [`Brightness::Dark`],
    ///   or a `MediaQuery` whose `MediaQueryData.platform_brightness` is [`Brightness::Dark`].
    /// - has a `MediaQuery` whose `MediaQueryData.high_contrast` is `true`.
    /// - has a [`CupertinoUserInterfaceLevel`] that indicates [`CupertinoUserInterfaceLevelData::Elevated`].
    pub dark_high_contrast_elevated_color: Color,
}

impl CupertinoDynamicColor {
    /// Creates an adaptive [`Color`] that changes its effective color based on the
    /// [`BuildContext`] given. The default effective color is `color`.
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's eight required named colors, in its order"
    )]
    pub const fn new(
        color: Color,
        dark_color: Color,
        high_contrast_color: Color,
        dark_high_contrast_color: Color,
        elevated_color: Color,
        dark_elevated_color: Color,
        high_contrast_elevated_color: Color,
        dark_high_contrast_elevated_color: Color,
    ) -> CupertinoDynamicColor {
        CupertinoDynamicColor {
            effective_color: color,
            debug_label: None,
            color,
            dark_color,
            high_contrast_color,
            dark_high_contrast_color,
            elevated_color,
            dark_elevated_color,
            high_contrast_elevated_color,
            dark_high_contrast_elevated_color,
        }
    }

    /// Creates an adaptive [`Color`] that changes its effective color based on the
    /// given [`BuildContext`]'s brightness (from `MediaQueryData.platform_brightness`
    /// or `CupertinoThemeData.brightness`) and accessibility contrast setting
    /// (`MediaQueryData.high_contrast`). The default effective color is `color`.
    pub const fn with_brightness_and_contrast(
        color: Color,
        dark_color: Color,
        high_contrast_color: Color,
        dark_high_contrast_color: Color,
    ) -> CupertinoDynamicColor {
        CupertinoDynamicColor::new(
            color,
            dark_color,
            high_contrast_color,
            dark_high_contrast_color,
            color,
            dark_color,
            high_contrast_color,
            dark_high_contrast_color,
        )
    }

    /// Creates an adaptive [`Color`] that changes its effective color based on the given
    /// [`BuildContext`]'s brightness (from `MediaQueryData.platform_brightness` or
    /// `CupertinoThemeData.brightness`). The default effective color is `color`.
    pub const fn with_brightness(color: Color, dark_color: Color) -> CupertinoDynamicColor {
        CupertinoDynamicColor::new(
            color, dark_color, color, dark_color, color, dark_color, color, dark_color,
        )
    }

    /// Dart `CupertinoDynamicColor(debugLabel:)`: the name diagnostics print for this color.
    pub const fn debug_label(mut self, debug_label: &'static str) -> CupertinoDynamicColor {
        self.debug_label = Some(debug_label);
        self
    }

    /// The current effective color: what Dart's `Color` members (`value`, `a`, `r`, …)
    /// read.
    ///
    /// Defaults to [`color`](Self::color) if this [`CupertinoDynamicColor`] has never been
    /// resolved.
    pub const fn effective_color(&self) -> Color {
        self.effective_color
    }

    /// Resolves the given [`Color`] by calling [`resolve_from`](Self::resolve_from).
    ///
    /// If the given color is already a concrete [`Color`], it will be returned as is.
    /// If the given color is a [`CupertinoDynamicColor`], but the given [`BuildContext`]
    /// lacks the dependencies required to the color resolution, the default trait
    /// value will be used ([`Brightness::Light`] platform brightness, normal contrast,
    /// [`CupertinoUserInterfaceLevelData::Base`] elevation level).
    ///
    /// See also:
    ///
    ///  * [`maybe_resolve`](Self::maybe_resolve), which is similar to this function, but
    ///    will allow a `None` `resolvable` color.
    pub fn resolve(resolvable: &AnyColor, app: &mut App, context: BuildContext) -> AnyColor {
        match resolvable.extension::<CupertinoDynamicColor>() {
            Some(dynamic) => dynamic.resolve_from(app, context).into_any(),
            None => resolvable.clone(),
        }
    }

    /// Resolves the given [`Color`] by calling [`resolve_from`](Self::resolve_from).
    ///
    /// If the given color is already a concrete [`Color`], it will be returned as is.
    /// If the given color is `None`, returns `None`.
    /// If the given color is a [`CupertinoDynamicColor`], but the given [`BuildContext`]
    /// lacks the dependencies required to the color resolution, the default trait
    /// value will be used ([`Brightness::Light`] platform brightness, normal contrast,
    /// [`CupertinoUserInterfaceLevelData::Base`] elevation level).
    ///
    /// See also:
    ///
    ///  * [`resolve`](Self::resolve), which is similar to this function, but returns a
    ///    non-optional value, and does not allow a `None` `resolvable` color.
    pub fn maybe_resolve(
        resolvable: Option<&AnyColor>,
        app: &mut App,
        context: BuildContext,
    ) -> Option<AnyColor> {
        resolvable.map(|resolvable| Self::resolve(resolvable, app, context))
    }

    fn is_platform_brightness_dependent(&self) -> bool {
        self.color != self.dark_color
            || self.elevated_color != self.dark_elevated_color
            || self.high_contrast_color != self.dark_high_contrast_color
            || self.high_contrast_elevated_color != self.dark_high_contrast_elevated_color
    }

    fn is_high_contrast_dependent(&self) -> bool {
        self.color != self.high_contrast_color
            || self.dark_color != self.dark_high_contrast_color
            || self.elevated_color != self.high_contrast_elevated_color
            || self.dark_elevated_color != self.dark_high_contrast_elevated_color
    }

    fn is_interface_elevation_dependent(&self) -> bool {
        self.color != self.elevated_color
            || self.dark_color != self.dark_elevated_color
            || self.high_contrast_color != self.high_contrast_elevated_color
            || self.dark_high_contrast_color != self.dark_high_contrast_elevated_color
    }

    /// Resolves this [`CupertinoDynamicColor`] using the provided [`BuildContext`].
    ///
    /// Calling this method will create a new [`CupertinoDynamicColor`] that is
    /// almost identical to this [`CupertinoDynamicColor`], except the effective
    /// color is changed to adapt to the given [`BuildContext`].
    ///
    /// For example, if the given [`BuildContext`] indicates the widgets in the
    /// subtree should be displayed in dark mode (the surrounding
    /// `CupertinoTheme`'s `CupertinoThemeData.brightness` or `MediaQuery`'s
    /// `MediaQueryData.platform_brightness` is [`Brightness::Dark`]), with a high
    /// accessibility contrast (the surrounding `MediaQuery`'s
    /// `MediaQueryData.high_contrast` is `true`), and an elevated interface
    /// elevation (the surrounding [`CupertinoUserInterfaceLevel`]'s `data` is
    /// [`CupertinoUserInterfaceLevelData::Elevated`]), the resolved
    /// [`CupertinoDynamicColor`] will be the same as this [`CupertinoDynamicColor`],
    /// except its effective color will be the `dark_high_contrast_elevated_color`
    /// variant from the original [`CupertinoDynamicColor`].
    ///
    /// Calling this function may create dependencies on the closest instance of
    /// some `InheritedWidget`s that enclose the given [`BuildContext`]. E.g., if
    /// [`dark_color`](Self::dark_color) is different from [`color`](Self::color), this
    /// method will call `CupertinoTheme::maybe_brightness_of` in an effort to determine the
    /// brightness. If [`color`](Self::color) is different from
    /// [`high_contrast_color`](Self::high_contrast_color), this method will call
    /// `MediaQuery::maybe_high_contrast_of` in an effort to determine the high contrast
    /// setting.
    ///
    /// If any of the required dependencies are missing from the given context,
    /// the default value of that trait will be used ([`Brightness::Light`] platform
    /// brightness, normal contrast, [`CupertinoUserInterfaceLevelData::Base`]
    /// elevation level).
    pub fn resolve_from(&self, app: &mut App, context: BuildContext) -> CupertinoDynamicColor {
        let brightness = if self.is_platform_brightness_dependent() {
            CupertinoTheme::maybe_brightness_of(app, context).unwrap_or(Brightness::Light)
        } else {
            Brightness::Light
        };

        let level = if self.is_interface_elevation_dependent() {
            CupertinoUserInterfaceLevel::maybe_of(app, context)
                .unwrap_or(CupertinoUserInterfaceLevelData::Base)
        } else {
            CupertinoUserInterfaceLevelData::Base
        };

        let high_contrast = self.is_high_contrast_dependent()
            && MediaQuery::maybe_high_contrast_of(app, context).unwrap_or(false);

        let resolved = match (brightness, level, high_contrast) {
            (Brightness::Light, CupertinoUserInterfaceLevelData::Base, false) => self.color,
            (Brightness::Light, CupertinoUserInterfaceLevelData::Base, true) => {
                self.high_contrast_color
            }
            (Brightness::Light, CupertinoUserInterfaceLevelData::Elevated, false) => {
                self.elevated_color
            }
            (Brightness::Light, CupertinoUserInterfaceLevelData::Elevated, true) => {
                self.high_contrast_elevated_color
            }
            (Brightness::Dark, CupertinoUserInterfaceLevelData::Base, false) => self.dark_color,
            (Brightness::Dark, CupertinoUserInterfaceLevelData::Base, true) => {
                self.dark_high_contrast_color
            }
            (Brightness::Dark, CupertinoUserInterfaceLevelData::Elevated, false) => {
                self.dark_elevated_color
            }
            (Brightness::Dark, CupertinoUserInterfaceLevelData::Elevated, true) => {
                self.dark_high_contrast_elevated_color
            }
        };

        CupertinoDynamicColor {
            effective_color: resolved,
            ..*self
        }
    }

    /// The dynamic color as the `Color` it is in Dart, built at run time (a resolved
    /// instance).
    pub fn into_any(self) -> AnyColor {
        AnyColor::with_shared(self.effective_color, Rc::new(self))
    }

    /// The dynamic color as the `Color` it is in Dart, from a `const` table entry.
    pub const fn to_any(&'static self) -> AnyColor {
        AnyColor::with_static(self.effective_color, self)
    }
}

impl ColorExtension for CupertinoDynamicColor {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_extension(&self, other: &dyn ColorExtension) -> bool {
        other
            .as_any()
            .downcast_ref::<CupertinoDynamicColor>()
            .is_some_and(|other| other == self)
    }
}

impl PartialEq for CupertinoDynamicColor {
    fn eq(&self, other: &CupertinoDynamicColor) -> bool {
        other.effective_color == self.effective_color
            && other.color == self.color
            && other.dark_color == self.dark_color
            && other.high_contrast_color == self.high_contrast_color
            && other.dark_high_contrast_color == self.dark_high_contrast_color
            && other.elevated_color == self.elevated_color
            && other.dark_elevated_color == self.dark_elevated_color
            && other.high_contrast_elevated_color == self.high_contrast_elevated_color
            && other.dark_high_contrast_elevated_color == self.dark_high_contrast_elevated_color
    }
}

impl Debug for CupertinoDynamicColor {
    /// Dart's `toString`, without the `resolved by:` suffix that names the resolving
    /// element (diagnostics).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let to_string = |name: &str, color: Color| {
            let marker = if color == self.effective_color {
                "*"
            } else {
                ""
            };
            format!("{marker}{name} = {color:?}{marker}")
        };

        let brightness = self.is_platform_brightness_dependent();
        let contrast = self.is_high_contrast_dependent();
        let elevation = self.is_interface_elevation_dependent();
        let mut xs = vec![to_string("color", self.color)];
        if brightness {
            xs.push(to_string("darkColor", self.dark_color));
        }
        if contrast {
            xs.push(to_string("highContrastColor", self.high_contrast_color));
        }
        if brightness && contrast {
            xs.push(to_string(
                "darkHighContrastColor",
                self.dark_high_contrast_color,
            ));
        }
        if elevation {
            xs.push(to_string("elevatedColor", self.elevated_color));
        }
        if brightness && elevation {
            xs.push(to_string("darkElevatedColor", self.dark_elevated_color));
        }
        if contrast && elevation {
            xs.push(to_string(
                "highContrastElevatedColor",
                self.high_contrast_elevated_color,
            ));
        }
        if brightness && contrast && elevation {
            xs.push(to_string(
                "darkHighContrastElevatedColor",
                self.dark_high_contrast_elevated_color,
            ));
        }

        write!(
            f,
            "{}({})",
            self.debug_label.unwrap_or("CupertinoDynamicColor"),
            xs.join(", ")
        )
    }
}

impl From<CupertinoDynamicColor> for AnyColor {
    fn from(color: CupertinoDynamicColor) -> AnyColor {
        color.into_any()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use reveal_widgets::{Builder, IntoWidget, MediaQuery, MediaQueryData, SizedBox, WidgetRef};

    use super::*;
    use crate::test_support::build;
    use crate::theme::CupertinoThemeData;

    /// `CupertinoDynamicColor::resolve(color)` against a context built under `wrap`.
    fn resolve_under(wrap: impl FnOnce(WidgetRef) -> WidgetRef, color: AnyColor) -> AnyColor {
        let mut app = crate::test_support::app();
        let seen = Rc::new(RefCell::new(None));
        let probe = Builder::new({
            let seen = Rc::clone(&seen);
            move |app, context| {
                *seen.borrow_mut() = Some(CupertinoDynamicColor::resolve(&color, app, context));
                SizedBox::shrink().into_widget()
            }
        })
        .into_widget();
        build(&mut app, wrap(probe));
        let resolved = seen.borrow_mut().take();
        resolved.expect("the probe built")
    }

    /// The Dart-level traits a context can carry, as widgets around a child.
    fn traits(
        brightness: Brightness,
        level: CupertinoUserInterfaceLevelData,
        high_contrast: bool,
    ) -> impl FnOnce(WidgetRef) -> WidgetRef {
        move |child| {
            MediaQuery::new(
                MediaQueryData::new()
                    .platform_brightness(brightness)
                    .high_contrast(high_contrast),
                CupertinoUserInterfaceLevel::new(level, child).into_widget(),
            )
            .into_widget()
        }
    }

    /// A color with eight distinct variants, so every branch of the resolution is visible.
    const EIGHT: CupertinoDynamicColor = CupertinoDynamicColor::new(
        Color::new(0xFF000001),
        Color::new(0xFF000002),
        Color::new(0xFF000003),
        Color::new(0xFF000004),
        Color::new(0xFF000005),
        Color::new(0xFF000006),
        Color::new(0xFF000007),
        Color::new(0xFF000008),
    );

    fn dynamic(color: &AnyColor) -> &CupertinoDynamicColor {
        color
            .extension::<CupertinoDynamicColor>()
            .expect("a dynamic color")
    }

    #[test]
    fn a_table_color_resolves_to_its_light_variant_by_default() {
        let resolved = resolve_under(|child| child, CupertinoColors::SYSTEM_BLUE);
        assert_eq!(resolved.color(), Color::from_argb(255, 0, 122, 255));
        assert_eq!(resolved, CupertinoColors::SYSTEM_BLUE);
    }

    #[test]
    fn a_table_color_resolves_to_its_dark_variant_under_a_dark_platform_brightness() {
        let dark = traits(
            Brightness::Dark,
            CupertinoUserInterfaceLevelData::Base,
            false,
        );
        let resolved = resolve_under(dark, CupertinoColors::SYSTEM_BLUE);
        assert_eq!(resolved.color(), Color::from_argb(255, 10, 132, 255));
        assert_eq!(
            dynamic(&resolved).effective_color(),
            dynamic(&CupertinoColors::SYSTEM_BLUE).dark_color
        );
        assert_ne!(resolved, CupertinoColors::SYSTEM_BLUE);
    }

    #[test]
    fn a_table_color_resolves_to_its_dark_variant_under_a_dark_cupertino_theme() {
        let dark_theme = |child: WidgetRef| {
            CupertinoTheme::new(
                CupertinoThemeData::new().with_brightness(Brightness::Dark),
                child,
            )
            .into_widget()
        };
        let resolved = resolve_under(dark_theme, CupertinoColors::SYSTEM_BLUE);
        assert_eq!(resolved.color(), Color::from_argb(255, 10, 132, 255));
        assert_eq!(
            dynamic(&resolved).effective_color(),
            dynamic(&CupertinoColors::SYSTEM_BLUE).dark_color
        );
    }

    #[test]
    fn a_table_color_resolves_to_its_high_contrast_variant_under_a_high_contrast_media_query() {
        let high_contrast = traits(
            Brightness::Light,
            CupertinoUserInterfaceLevelData::Base,
            true,
        );
        let resolved = resolve_under(high_contrast, CupertinoColors::SYSTEM_BLUE);
        assert_eq!(resolved.color(), Color::from_argb(255, 0, 64, 221));
    }

    #[test]
    fn a_table_color_resolves_to_its_elevated_variant_under_an_elevated_interface_level() {
        let elevated = traits(
            Brightness::Dark,
            CupertinoUserInterfaceLevelData::Elevated,
            false,
        );
        let resolved = resolve_under(elevated, CupertinoColors::SYSTEM_BACKGROUND);
        assert_eq!(resolved.color(), Color::from_argb(255, 28, 28, 30));
    }

    #[test]
    fn every_trait_combination_picks_its_variant() {
        let cases = [
            (
                Brightness::Light,
                CupertinoUserInterfaceLevelData::Base,
                false,
                EIGHT.color,
            ),
            (
                Brightness::Light,
                CupertinoUserInterfaceLevelData::Base,
                true,
                EIGHT.high_contrast_color,
            ),
            (
                Brightness::Light,
                CupertinoUserInterfaceLevelData::Elevated,
                false,
                EIGHT.elevated_color,
            ),
            (
                Brightness::Light,
                CupertinoUserInterfaceLevelData::Elevated,
                true,
                EIGHT.high_contrast_elevated_color,
            ),
            (
                Brightness::Dark,
                CupertinoUserInterfaceLevelData::Base,
                false,
                EIGHT.dark_color,
            ),
            (
                Brightness::Dark,
                CupertinoUserInterfaceLevelData::Base,
                true,
                EIGHT.dark_high_contrast_color,
            ),
            (
                Brightness::Dark,
                CupertinoUserInterfaceLevelData::Elevated,
                false,
                EIGHT.dark_elevated_color,
            ),
            (
                Brightness::Dark,
                CupertinoUserInterfaceLevelData::Elevated,
                true,
                EIGHT.dark_high_contrast_elevated_color,
            ),
        ];
        for (brightness, level, high_contrast, expected) in cases {
            let resolved =
                resolve_under(traits(brightness, level, high_contrast), EIGHT.into_any());
            assert_eq!(
                resolved.color(),
                expected,
                "{brightness:?}, {level:?}, high contrast {high_contrast}"
            );
            assert_eq!(
                dynamic(&resolved).color,
                EIGHT.color,
                "the variants survive"
            );
        }
    }

    #[test]
    fn a_plain_color_passes_through_resolve_unchanged() {
        let plain = AnyColor::new(Color::new(0xFF123456));
        let dark = traits(
            Brightness::Dark,
            CupertinoUserInterfaceLevelData::Elevated,
            true,
        );
        let resolved = resolve_under(dark, plain.clone());
        assert_eq!(resolved, plain);
        assert!(!resolved.has_extension());
    }

    #[test]
    fn maybe_resolve_passes_none_through() {
        let mut app = crate::test_support::app();
        let seen = Rc::new(RefCell::new(None));
        let probe = Builder::new({
            let seen = Rc::clone(&seen);
            move |app, context| {
                *seen.borrow_mut() = Some((
                    CupertinoDynamicColor::maybe_resolve(None, app, context),
                    CupertinoDynamicColor::maybe_resolve(
                        Some(&CupertinoColors::SYSTEM_RED),
                        app,
                        context,
                    ),
                ));
                SizedBox::shrink().into_widget()
            }
        })
        .into_widget();
        build(&mut app, probe);
        let (none, red) = seen.borrow_mut().take().expect("the probe built");
        assert_eq!(none, None);
        assert_eq!(red, Some(CupertinoColors::SYSTEM_RED));
    }

    #[test]
    fn a_dynamic_color_equals_itself_and_not_its_plain_value() {
        assert_eq!(CupertinoColors::SYSTEM_BLUE, CupertinoColors::SYSTEM_BLUE);
        assert_eq!(
            CupertinoColors::SYSTEM_BLUE,
            CupertinoColors::SYSTEM_BLUE_DYNAMIC.into_any()
        );
        assert_eq!(CupertinoColors::ACTIVE_BLUE, CupertinoColors::SYSTEM_BLUE);
        assert_ne!(
            CupertinoColors::SYSTEM_BLUE,
            AnyColor::new(Color::from_argb(255, 0, 122, 255))
        );
        assert_ne!(CupertinoColors::SYSTEM_BLUE, CupertinoColors::SYSTEM_GREEN);
        assert_eq!(
            CupertinoColors::SYSTEM_BLUE,
            CupertinoColors::SYSTEM_BLUE_DYNAMIC
                .debug_label("other label")
                .into_any(),
            "the label is not part of the value"
        );
        let same_without_label = CupertinoDynamicColor::with_brightness_and_contrast(
            Color::from_argb(255, 0, 122, 255),
            Color::from_argb(255, 10, 132, 255),
            Color::from_argb(255, 0, 64, 221),
            Color::from_argb(255, 64, 156, 255),
        );
        assert_eq!(CupertinoColors::SYSTEM_BLUE, same_without_label.into_any());
    }

    #[test]
    fn the_value_is_the_effective_color_and_color_methods_come_from_it() {
        let light = Color::from_argb(255, 0, 122, 255);
        assert_eq!(CupertinoColors::SYSTEM_BLUE.color(), light);
        assert_eq!(CupertinoColors::SYSTEM_BLUE.to_argb32(), light.to_argb32());
        assert_eq!(
            CupertinoColors::SYSTEM_BLUE.with_alpha(128),
            light.with_alpha(128)
        );
        assert_eq!(dynamic(&CupertinoColors::SYSTEM_BLUE).color, light);
        assert_eq!(CupertinoColors::WHITE.color(), Color::new(0xFFFFFFFF));
        assert!(!CupertinoColors::WHITE.has_extension());
    }

    #[test]
    fn debug_prints_the_dependent_variants_and_stars_the_effective_one() {
        let light = Color::from_argb(255, 0, 122, 255);
        let dark = Color::from_argb(255, 10, 132, 255);
        let high_contrast = Color::from_argb(255, 0, 64, 221);
        let dark_high_contrast = Color::from_argb(255, 64, 156, 255);
        assert_eq!(
            format!("{:?}", CupertinoColors::SYSTEM_BLUE),
            format!(
                "systemBlue(*color = {light:?}*, darkColor = {dark:?}, \
                 highContrastColor = {high_contrast:?}, \
                 darkHighContrastColor = {dark_high_contrast:?})"
            )
        );

        let resolved = resolve_under(
            traits(
                Brightness::Dark,
                CupertinoUserInterfaceLevelData::Base,
                false,
            ),
            CupertinoColors::SYSTEM_BLUE,
        );
        assert_eq!(
            format!("{resolved:?}"),
            format!(
                "systemBlue(color = {light:?}, *darkColor = {dark:?}*, \
                 highContrastColor = {high_contrast:?}, \
                 darkHighContrastColor = {dark_high_contrast:?})"
            )
        );

        let white = Color::new(0xFFFFFFFF);
        let black = Color::new(0xFF000000);
        let unlabeled = CupertinoDynamicColor::with_brightness(white, black);
        assert_eq!(
            format!("{unlabeled:?}"),
            format!("CupertinoDynamicColor(*color = {white:?}*, darkColor = {black:?})")
        );

        let constant = CupertinoDynamicColor::with_brightness(white, white);
        assert_eq!(
            format!("{constant:?}"),
            format!("CupertinoDynamicColor(*color = {white:?}*)")
        );
    }
}
