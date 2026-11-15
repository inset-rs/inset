//! Flutter counterpart: material_ui's `theme_data.dart`.
//!
//! [`ThemeData`] carries the fields whose types exist; each component theme joins it with its
//! component, in Dart's place in every list below (the struct, the factory, `copyWith`,
//! `lerp` and `==`), so keep the order when one lands.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use indexmap::IndexMap;
use waft_cupertino::{
    CupertinoThemeData, CupertinoThemeDataOverrides, NoDefaultCupertinoThemeData,
};
use waft_embedder::{Brightness, Color, Offset, TargetPlatform, clamp_double, lerp_double};
use waft_foundation::{App, K_IS_WEB, PRECISION_ERROR_TOLERANCE};
use waft_painting::AnyColor;
use waft_rendering::{BoxConstraints, Constraints};
use waft_widgets::{BuildContext, IconThemeData};

use crate::action_icons_theme::ActionIconThemeData;
use crate::app_bar_theme::AppBarThemeData;
use crate::badge_theme::BadgeThemeData;
use crate::banner_theme::MaterialBannerThemeData;
use crate::bottom_navigation_bar_theme::BottomNavigationBarThemeData;
use crate::bottom_sheet_theme::BottomSheetThemeData;
use crate::card_theme::CardThemeData;
use crate::checkbox_theme::CheckboxThemeData;
use crate::color_scheme::ColorScheme;
use crate::colors::{Colors, MaterialColor};
use crate::constants::{K_DEFAULT_ICON_DARK_COLOR, K_DEFAULT_ICON_LIGHT_COLOR};
use crate::dialog_theme::DialogThemeData;
use crate::divider_theme::DividerThemeData;
use crate::drawer_theme::DrawerThemeData;
use crate::elevated_button_theme::ElevatedButtonThemeData;
use crate::expansion_tile_theme::ExpansionTileThemeData;
use crate::filled_button_theme::FilledButtonThemeData;
use crate::floating_action_button_theme::FloatingActionButtonThemeData;
use crate::generated::color_scheme_defaults::{
    color_scheme_dark_high_contrast_m3, color_scheme_dark_m3, color_scheme_dark_medium_contrast_m3,
    color_scheme_light_high_contrast_m3, color_scheme_light_m3,
    color_scheme_light_medium_contrast_m3,
};
use crate::icon_button_theme::IconButtonThemeData;
use crate::ink_ripple::InkRipple;
use crate::ink_sparkle::InkSparkle;
use crate::ink_splash::InkSplash;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::input_decorator::InputDecorationThemeData;
use crate::list_tile_theme::ListTileThemeData;
use crate::menu_bar_theme::MenuBarThemeData;
use crate::menu_button_theme::MenuButtonThemeData;
use crate::menu_theme::MenuThemeData;
use crate::navigation_bar_theme::NavigationBarThemeData;
use crate::navigation_drawer_theme::NavigationDrawerThemeData;
use crate::navigation_rail_theme::NavigationRailThemeData;
use crate::outlined_button_theme::OutlinedButtonThemeData;
use crate::page_transitions_theme::PageTransitionsTheme;
use crate::popup_menu_theme::PopupMenuThemeData;
use crate::progress_indicator_theme::ProgressIndicatorThemeData;
use crate::radio_theme::RadioThemeData;
use crate::scrollbar_theme::ScrollbarThemeData;
use crate::slider_theme::SliderThemeData;
use crate::snack_bar_theme::SnackBarThemeData;
use crate::switch_theme::SwitchThemeData;
use crate::tab_bar_theme::TabBarThemeData;
use crate::text_button_theme::TextButtonThemeData;
use crate::text_selection_theme::TextSelectionThemeData;
use crate::text_theme::TextTheme;
use crate::tooltip_theme::TooltipThemeData;
use crate::typography::Typography;

/// Defines a customized theme for components with an `adaptive` factory constructor.
///
/// Currently, only `Switch.adaptive` supports this class.
///
/// `T` is the adaptation's type, the key [`ThemeData::adaptation_map`] stores it under
/// (an [`AdaptationRef`] erases it).
pub trait Adaptation<T: 'static>: Debug + 'static {
    /// Typically, this is overridden to return an instance of a custom component
    /// ThemeData class, like `SwitchThemeData`, instead of the default_value.
    ///
    /// Factory constructors that support adaptations - currently only
    /// `Switch.adaptive` - look for a type-specific adaptation in
    /// [`ThemeData::adaptation_map`] when computing their effective default component
    /// theme. If a matching adaptation is not found, the component may choose to
    /// use a default adaptation. For example, the `Switch.adaptive` component
    /// uses an empty `SwitchThemeData` if a matching adaptation is not found, for
    /// the sake of backwards compatibility.
    fn adapt(&self, theme: &ThemeData, default_value: T) -> T {
        let _ = theme;
        default_value
    }
}

/// An [`Adaptation`] of any type: what Dart's `Adaptation<Object>` in
/// [`ThemeData::adaptation_map`] becomes.
///
/// Two refs are equal when they hold the same adaptation, as Dart's `==` on an `Adaptation`
/// is identity.
#[derive(Clone)]
pub struct AdaptationRef {
    adaptation_type: TypeId,
    adaptation: Rc<dyn ErasedAdaptation>,
}

impl AdaptationRef {
    /// Erases `adaptation`, keyed by its type `T`.
    pub fn new<T: 'static>(adaptation: impl Adaptation<T>) -> AdaptationRef {
        AdaptationRef {
            adaptation_type: TypeId::of::<T>(),
            adaptation: Rc::new(TypedAdaptation::<T>(Box::new(adaptation))),
        }
    }

    /// The adaptation's type: Dart's `type`, the `T` of `Adaptation<T>`.
    pub fn adaptation_type(&self) -> TypeId {
        self.adaptation_type
    }

    /// The adaptation, if its type is `T`.
    pub fn downcast<T: 'static>(&self) -> Option<&dyn Adaptation<T>> {
        self.adaptation
            .as_any()
            .downcast_ref::<TypedAdaptation<T>>()
            .map(|typed| &*typed.0)
    }
}

impl PartialEq for AdaptationRef {
    fn eq(&self, other: &AdaptationRef) -> bool {
        Rc::ptr_eq(&self.adaptation, &other.adaptation)
    }
}

impl Debug for AdaptationRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&*self.adaptation, f)
    }
}

/// The typed adaptation behind an [`AdaptationRef`], recovered by its `T`.
trait ErasedAdaptation: Debug {
    fn as_any(&self) -> &dyn Any;
}

struct TypedAdaptation<T: 'static>(Box<dyn Adaptation<T>>);

impl<T: 'static> ErasedAdaptation for TypedAdaptation<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<T: 'static> Debug for TypedAdaptation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&*self.0, f)
    }
}

/// An interface that defines custom additions to a [`ThemeData`] object.
///
/// Learn more about [`ThemeExtension`] on the [Flutter YouTube channel](https://www.youtube.com/watch?v=8-szcYzFVao).
///
/// Typically used for custom colors. To use, implement [`ThemeExtension`] on a type,
/// define a number of fields (e.g. [`Color`]s), and implement the
/// [`copy_with`](Self::copy_with) and [`lerp`](Self::lerp) methods. The latter will ensure
/// smooth transitions of properties when switching themes.
///
/// Dart's `ThemeExtension<T extends ThemeExtension<T>>` is the implementing type itself:
/// [`extension_type`](Self::extension_type) is its `T`, the key
/// [`ThemeData::extensions`] stores it under, and [`ThemeData::extension`] downcasts to it.
pub trait ThemeExtension: Any + Debug {
    /// The extension as `Any`, for [`ThemeData::extension`] and for the downcast of
    /// [`lerp`](Self::lerp)'s `other`.
    fn as_any(&self) -> &dyn Any;

    /// The extension's type.
    fn extension_type(&self) -> TypeId {
        self.as_any().type_id()
    }

    /// Creates a copy of this theme extension with the given fields
    /// replaced by the non-null parameter values.
    ///
    /// Dart's named arguments are the implementing type's own.
    fn copy_with(&self) -> ThemeExtensionRef;

    /// Linearly interpolate with another [`ThemeExtension`] object.
    ///
    /// `other` is Dart's `covariant` parameter: an extension of this one's type, or `None`;
    /// downcast it through [`as_any`](Self::as_any). An implementation returns `self`
    /// where Dart returns `this`.
    ///
    /// {@macro dart.ui.shadow.lerp}
    fn lerp(self: Rc<Self>, other: Option<&dyn ThemeExtension>, t: f64) -> ThemeExtensionRef;

    /// Dart's `==`: identity, unless the implementing type overrides it.
    fn eq_extension(&self, other: &dyn ThemeExtension) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const dyn ThemeExtension)
    }
}

/// A shared [`ThemeExtension`]: what Dart's `ThemeExtension<dynamic>` values become.
pub type ThemeExtensionRef = Rc<dyn ThemeExtension>;

/// Configures the tap target and layout size of certain Material widgets.
///
/// Changing the value in [`ThemeData::material_tap_target_size`] will affect the
/// accessibility experience.
///
/// Some of the impacted widgets include:
///
///   * `FloatingActionButton`, only the mini tap target size is increased.
///   * `MaterialButton`
///   * `OutlinedButton`
///   * `TextButton`
///   * `ElevatedButton`
///   * `IconButton`
///   * The time picker widget (`showTimePicker`)
///   * `SnackBar`
///   * `Chip`
///   * `RawChip`
///   * `InputChip`
///   * `ChoiceChip`
///   * `FilterChip`
///   * `ActionChip`
///   * `Radio`
///   * `Switch`
///   * `Checkbox`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaterialTapTargetSize {
    /// Expands the minimum tap target size to 48px by 48px.
    ///
    /// This is the default value of [`ThemeData::material_tap_target_size`] and the
    /// recommended size to conform to Android accessibility scanner
    /// recommendations.
    Padded,

    /// Shrinks the tap target size to the minimum provided by the Material
    /// specification.
    ShrinkWrap,
}

/// Defines the Material Design style variant used by Material components.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StyleVariant {
    /// The Material Design 3 style variant.
    Material3,

    /// The Material Design 3 Expressive style variant.
    ///
    /// Material 3 Expressive support is under development. Apps or components
    /// that opt into this variant may see visual changes as more Expressive
    /// features get added.
    ///
    /// See also:
    ///
    ///  * [Material Design 3 Expressive updates](https://m3.material.io/#9f2db05b-ebbb-47c2-892d-1cca35afa386) for design guidelines.
    Material3Expressive,
}

/// Named contrast levels for Material 3 color schemes.
///
/// Material 3 baseline color schemes provide standard, medium, and high
/// contrast levels. When the `color_scheme_seed` parameter of the [`ThemeData`]
/// constructor is null, use [`STANDARD`](Self::STANDARD), [`MEDIUM`](Self::MEDIUM), or
/// [`HIGH`](Self::HIGH) to select a token-backed baseline color scheme with that contrast
/// level.
///
/// A custom `f64` between -1.0 and 1.0 can also be passed as the
/// `contrast_level` parameter of the [`ThemeData`] constructor when
/// `color_scheme_seed` is provided, in which case the value is passed to
/// [`ColorScheme::from_seed`].
///
/// See also:
///
///  * [`ColorScheme::from_seed`], which generates a color scheme from a seed color,
///    brightness, and contrast level.
///  * <https://m3.material.io/styles/color/system/overview>, the Material 3
///    color system overview.
pub struct ContrastLevel;

impl ContrastLevel {
    /// Standard contrast, corresponding to a contrast level of 0.0.
    pub const STANDARD: f64 = 0.0;

    /// Medium contrast, corresponding to a contrast level of 0.5.
    pub const MEDIUM: f64 = 0.5;

    /// High contrast, corresponding to a contrast level of 1.0.
    pub const HIGH: f64 = 1.0;
}

/// Defines the configuration of the overall visual `Theme` for a `MaterialApp`
/// or a widget subtree within the app.
///
/// The `MaterialApp` theme property can be used to configure the appearance
/// of the entire app. Widget subtrees within an app can override the app's
/// theme by including a `Theme` widget at the top of the subtree.
///
/// Widgets whose appearance should align with the overall theme can obtain the
/// current theme's configuration with `Theme::of`. Material components typically
/// depend exclusively on the [`color_scheme`](Self::color_scheme) and
/// [`text_theme`](Self::text_theme). These properties are guaranteed to have non-null
/// values.
///
/// The static `Theme::of` method finds the [`ThemeData`] value specified for the
/// nearest `BuildContext` ancestor. This lookup is inexpensive, essentially
/// just a single HashMap access. It can sometimes be a little confusing
/// because `Theme::of` can not see a `Theme` widget that is defined in the
/// current build method's context. To overcome that, create a new custom widget
/// for the subtree that appears below the new `Theme`, or insert a widget
/// that creates a new BuildContext, like `Builder`.
///
/// See <https://material.io/design/color/> for
/// more discussion on how to pick the right colors.
///
/// Dart's factory constructor is [`ThemeData::builder`]; `ThemeData.raw`, which takes every
/// value exactly, is the struct literal.
#[derive(Clone, Debug)]
pub struct ThemeData {
    // For the sanity of the reader, make sure these properties are in the same
    // order in every place that they are separated by section comments (e.g.
    // GENERAL CONFIGURATION). Each section except for deprecations should be
    // alphabetical by symbol name.

    // GENERAL CONFIGURATION
    /// Apply a semi-transparent overlay color on Material surfaces to indicate
    /// elevation for dark themes.
    ///
    /// If [`use_material3`](Self::use_material3) is true, then this flag is ignored as there
    /// is a new `Material.surfaceTintColor` used to create an overlay for Material 3.
    /// This flag is meant only for the Material 2 elevation overlay for dark
    /// themes.
    ///
    /// Material drop shadows can be difficult to see in a dark theme, so the
    /// elevation of a surface should be portrayed with an "overlay" in addition
    /// to the shadow. As the elevation of the component increases, the
    /// overlay increases in opacity. [`apply_elevation_overlay_color`](Self::apply_elevation_overlay_color)
    /// turns the application of this overlay on or off for dark themes.
    ///
    /// If true and [`brightness`](Self::brightness) is [`Brightness::Dark`], a
    /// semi-transparent version of `ColorScheme.on_surface` will be
    /// applied on top of `Material` widgets that have a `ColorScheme.surface`
    /// color. The level of transparency is based on `Material.elevation` as
    /// per the Material Dark theme specification.
    ///
    /// If false the surface color will be used unmodified.
    ///
    /// Defaults to false in order to maintain backwards compatibility with
    /// apps that were built before the Material Dark theme specification
    /// was published. New apps should set this to true for any themes
    /// where [`brightness`](Self::brightness) is [`Brightness::Dark`].
    ///
    /// See also:
    ///
    ///  * `Material.elevation`, which effects the level of transparency of the
    ///    overlay color.
    ///  * `ElevationOverlay.applyOverlay`, which is used by `Material` to apply
    ///    the overlay color to its surface color.
    ///  * <https://material.io/design/color/dark-theme.html>, which specifies how
    ///    the overlay should be applied.
    pub apply_elevation_overlay_color: bool,
    /// Components of the [`CupertinoThemeData`] to override from the Material
    /// [`ThemeData`] adaptation.
    ///
    /// By default, [`cupertino_override_theme`](Self::cupertino_override_theme) is null and
    /// Cupertino widgets descendant to the Material `Theme` will adhere to a `CupertinoTheme`
    /// derived from the Material [`ThemeData`]. e.g. [`ThemeData`]'s [`ColorScheme`]
    /// will also inform the [`CupertinoThemeData`]'s `primary_color` etc.
    ///
    /// This cascading effect for individual attributes of the [`CupertinoThemeData`]
    /// can be overridden using attributes of this
    /// [`cupertino_override_theme`](Self::cupertino_override_theme).
    pub cupertino_override_theme: Option<NoDefaultCupertinoThemeData>,
    /// Arbitrary additions to this theme.
    ///
    /// To define extensions, pass one or more [`ThemeExtension`]s to
    /// [`ThemeDataBuilder::extensions`] or [`ThemeDataCopyWith::extensions`].
    ///
    /// To obtain an extension, use [`extension`](Self::extension).
    ///
    /// See also:
    ///
    /// * [`extension`](Self::extension), a convenience function for obtaining a specific
    ///   extension.
    pub extensions: IndexMap<TypeId, ThemeExtensionRef>,
    /// A map which contains the adaptations for the theme. The entry's key is the
    /// type of the adaptation; the value is the adaptation itself.
    ///
    /// To obtain an adaptation, use [`get_adaptation`](Self::get_adaptation).
    pub adaptation_map: IndexMap<TypeId, AdaptationRef>,
    /// The default `InputDecoration` values for `InputDecorator`, `TextField`,
    /// and `TextFormField` are based on this theme.
    ///
    /// See `InputDecoration::apply_defaults`.
    pub input_decoration_theme: InputDecorationThemeData,
    /// Configures the hit test size of certain Material widgets.
    ///
    /// Defaults to a [`platform`](Self::platform)-appropriate size:
    /// [`MaterialTapTargetSize::Padded`] on mobile platforms,
    /// [`MaterialTapTargetSize::ShrinkWrap`] on desktop platforms.
    pub material_tap_target_size: MaterialTapTargetSize,
    /// Default `MaterialPageRoute` transitions per [`TargetPlatform`].
    ///
    /// `MaterialPageRoute::build_transitions` delegates to a [`platform`](Self::platform)
    /// specific `PageTransitionsBuilder`. If a matching builder is not found, a builder whose
    /// platform is null is used.
    pub page_transitions_theme: PageTransitionsTheme,
    /// The platform the material widgets should adapt to target.
    ///
    /// Defaults to the current platform, as exposed by `defaultTargetPlatform`
    /// (`app.platform().target_platform()`). This should be used in order to style UI
    /// elements according to platform conventions.
    ///
    /// Widgets from the material library should use this getter (via `Theme::of`)
    /// to determine the current platform for the purpose of emulating the
    /// platform behavior (e.g. scrolling or haptic effects). Widgets and render
    /// objects at lower layers that try to emulate the underlying platform
    /// can depend on `defaultTargetPlatform` directly, or may require
    /// that the target platform be provided as an argument.
    ///
    /// Determines the defaults for [`typography`](Self::typography) and
    /// [`material_tap_target_size`](Self::material_tap_target_size).
    pub platform: TargetPlatform,
    /// A theme for customizing the colors, thickness, and shape of `Scrollbar`s.
    pub scrollbar_theme: ScrollbarThemeData,
    /// Defines the appearance of ink splashes produces by [`InkWell`](crate::InkWell)
    /// and [`InkResponse`](crate::InkResponse).
    ///
    /// See also:
    ///
    ///  * [`InkSplash::splash_factory`](crate::InkSplash::splash_factory), which defines the
    ///    default splash.
    ///  * [`InkRipple::splash_factory`](crate::InkRipple::splash_factory), which defines a
    ///    splash that spreads out more aggressively than the default.
    ///  * [`InkSparkle::splash_factory`](crate::InkSparkle::splash_factory), which defines a
    ///    more aggressive and organic splash with sparkle effects.
    pub splash_factory: InteractiveInkFeatureFactoryRef,
    /// A temporary flag that can be used to opt-out of Material 3 features.
    ///
    /// This flag is _true_ by default. If false, then components will
    /// continue to use the colors, typography and other features of
    /// Material 2.
    ///
    /// In the long run this flag will be deprecated and eventually
    /// only Material 3 will be supported. We recommend that applications
    /// migrate to Material 3 as soon as that's practical. Until that migration
    /// is complete, this flag can be set to false.
    ///
    /// ## Defaults
    ///
    /// If a [`ThemeData`] is _constructed_ with [`use_material3`](Self::use_material3) set to
    /// true, then some properties will get updated defaults. However, the
    /// [`ThemeData::copy_with`] method with `use_material3` set to true will _not_
    /// change any of these properties in the resulting [`ThemeData`].
    ///
    /// | Property        | Material 3 default             | Material 2 default             |
    /// | :-------------- | :----------------------------- | :----------------------------- |
    /// | color_scheme    | M3 baseline light color scheme | M2 baseline light color scheme |
    /// | typography      | `Typography::material2021`     | `Typography::material2014`     |
    /// | splash_factory  | `InkSparkle`* or `InkRipple`   | `InkSplash`                    |
    ///
    /// \* if the target platform is Android and the app is not
    /// running on the web, otherwise it will fallback to `InkRipple`.
    ///
    /// If [`brightness`](Self::brightness) is [`Brightness::Dark`] then the default color
    /// scheme will be either the M3 baseline dark color scheme or the M2 baseline dark color
    /// scheme depending on [`use_material3`](Self::use_material3).
    ///
    /// See also:
    ///
    ///   * [Material 3 specification](https://m3.material.io/).
    pub use_material3: bool,
    /// The density value for specifying the compactness of various UI components.
    ///
    /// Density, in the context of a UI, is the vertical and horizontal
    /// "compactness" of the elements in the UI. It is unitless, since it means
    /// different things to different UI elements. For buttons, it affects the
    /// spacing around the centered label of the button. For lists, it affects the
    /// distance between baselines of entries in the list.
    ///
    /// Typically, density values are integral, but any value in range may be
    /// used. The range includes values from [`VisualDensity::MINIMUM_DENSITY`] (which
    /// is -4), to [`VisualDensity::MAXIMUM_DENSITY`] (which is 4), inclusive, where
    /// negative values indicate a denser, more compact, UI, and positive values
    /// indicate a less dense, more expanded, UI. If a component doesn't support
    /// the value given, it will clamp to the nearest supported value.
    ///
    /// The default for visual densities is zero for both vertical and horizontal
    /// densities, which corresponds to the default visual density of components
    /// in the Material Design specification.
    ///
    /// As a rule of thumb, a change of 1 or -1 in density corresponds to 4
    /// logical pixels. However, this is not a strict relationship since
    /// components interpret the density values appropriately for their needs.
    ///
    /// A larger value translates to a spacing increase (less dense), and a
    /// smaller value translates to a spacing decrease (more dense).
    ///
    /// In Material Design 3, the [`visual_density`](Self::visual_density) does not override
    /// the default visual for the following components which are set to
    /// [`VisualDensity::STANDARD`] for all platforms:
    ///
    ///  * `IconButton` - To override the default value of `IconButton.visualDensity`,
    ///    use `ThemeData.iconButtonTheme` instead.
    ///  * `Checkbox` - To override the default value of `Checkbox.visualDensity`,
    ///    use `ThemeData.checkboxTheme` instead.
    pub visual_density: VisualDensity,

    // COLOR
    /// The default color of `MaterialType.canvas` `Material`.
    pub canvas_color: Color,
    /// The color of `Material` when it is used as a `Card`.
    pub card_color: Color,
    /// A set of 45 colors based on the
    /// [Material spec](https://m3.material.io/styles/color/the-color-system/color-roles)
    /// that can be used to configure the color properties of most components.
    ///
    /// This property was added much later than the theme's set of highly specific
    /// colors, like [`card_color`](Self::card_color), [`canvas_color`](Self::canvas_color)
    /// etc. New components can be defined exclusively in terms of
    /// [`color_scheme`](Self::color_scheme). Existing components will gradually migrate to
    /// it, to the extent that is possible without significant backwards compatibility breaks.
    pub color_scheme: ColorScheme,
    /// The color used for widgets that are inoperative, regardless of
    /// their state. For example, a disabled checkbox (which may be
    /// checked or unchecked).
    pub disabled_color: Color,
    /// The color of `Divider`s and `PopupMenuDivider`s, also used
    /// between `ListTile`s, between rows in `DataTable`s, and so forth.
    ///
    /// To create an appropriate `BorderSide` that uses this color, consider
    /// `Divider.createBorderSide`.
    pub divider_color: Color,
    /// The focus color used indicate that a component has the input focus.
    pub focus_color: Color,
    /// The highlight color used during ink splash animations or to
    /// indicate an item in a menu is selected.
    pub highlight_color: Color,
    /// The color to use for hint text or placeholder text, e.g. in
    /// `TextField` fields.
    pub hint_color: Color,
    /// The hover color used to indicate when a pointer is hovering over a
    /// component.
    pub hover_color: Color,
    /// The background color for major parts of the app (toolbars, tab bars, etc)
    ///
    /// The theme's [`color_scheme`](Self::color_scheme) property contains
    /// `ColorScheme.primary`, as well as a color that contrasts well with the primary color
    /// called `ColorScheme.on_primary`. It might be simpler to just configure an app's
    /// visuals in terms of the theme's [`color_scheme`](Self::color_scheme).
    pub primary_color: Color,
    /// A darker version of the [`primary_color`](Self::primary_color).
    pub primary_color_dark: Color,
    /// A lighter version of the [`primary_color`](Self::primary_color).
    pub primary_color_light: Color,
    /// The default color of the `Material` that underlies the `Scaffold`. The
    /// background color for a typical material app or a page within the app.
    pub scaffold_background_color: Color,
    /// The color of the header of a `PaginatedDataTable` when there are selected rows.
    // According to the spec for data tables:
    // https://material.io/archive/guidelines/components/data-tables.html#data-tables-tables-within-cards
    // ...this should be the "50-value of secondary app color".
    pub secondary_header_color: Color,
    /// The color that the `Material` widget uses to draw elevation shadows.
    ///
    /// Defaults to fully opaque black.
    ///
    /// Shadows can be difficult to see in a dark theme, so the elevation of a
    /// surface should be rendered with an "overlay" in addition to the shadow.
    /// As the elevation of the component increases, the overlay increases in
    /// opacity. The [`apply_elevation_overlay_color`](Self::apply_elevation_overlay_color)
    /// property turns the elevation overlay on or off for dark themes.
    pub shadow_color: Color,
    /// The color of ink splashes.
    ///
    /// See also:
    ///  * `splashFactory`, which defines the appearance of the splash.
    pub splash_color: Color,
    /// The color used for widgets in their inactive (but enabled)
    /// state. For example, an unchecked checkbox. See also
    /// [`disabled_color`](Self::disabled_color).
    pub unselected_widget_color: Color,

    // TYPOGRAPHY & ICONOGRAPHY
    /// An icon theme that contrasts with the card and canvas colors.
    pub icon_theme: IconThemeData,
    /// An icon theme that contrasts with the primary color.
    pub primary_icon_theme: IconThemeData,
    /// A text theme that contrasts with the primary color.
    pub primary_text_theme: TextTheme,
    /// Text with a color that contrasts with the card and canvas colors.
    pub text_theme: TextTheme,
    /// The color and geometry [`TextTheme`] values used to configure
    /// [`text_theme`](Self::text_theme).
    ///
    /// Defaults to a [`platform`](Self::platform)-appropriate typography.
    pub typography: Typography,

    // COMPONENT THEMES
    /// A theme for customizing icons of `BackButton`, `CloseButton`, `DrawerButton`,
    /// and `EndDrawerButton`.
    pub action_icon_theme: Option<ActionIconThemeData>,
    /// A theme for customizing the color, elevation, brightness, iconTheme and
    /// textTheme of `AppBar`s.
    pub app_bar_theme: AppBarThemeData,
    /// A theme for customizing the color, shape, and size of a [`Badge`](crate::Badge).
    pub badge_theme: BadgeThemeData,
    /// A theme for customizing the color and text style of a `MaterialBanner`.
    pub banner_theme: MaterialBannerThemeData,
    /// A theme for customizing the appearance and layout of `BottomNavigationBar`
    /// widgets.
    pub bottom_navigation_bar_theme: BottomNavigationBarThemeData,
    /// A theme for customizing the color, elevation, and shape of a bottom sheet.
    pub bottom_sheet_theme: BottomSheetThemeData,
    /// A theme for customizing the color, elevation, and shape of a [`Card`](crate::Card).
    ///
    /// This is the value returned from `CardTheme::of`.
    pub card_theme: CardThemeData,
    /// A theme for customizing the appearance and layout of `Checkbox` widgets.
    pub checkbox_theme: CheckboxThemeData,
    /// A theme for customizing the shape of a dialog.
    pub dialog_theme: DialogThemeData,
    /// A theme for customizing the appearance of [`Divider`](crate::Divider)s,
    /// `VerticalDivider`s, dividers between `ListTile`s, and dividers between rows in
    /// `DataTable`s.
    pub divider_theme: DividerThemeData,
    /// A theme for customizing the appearance and layout of `Drawer` widgets.
    pub drawer_theme: DrawerThemeData,
    /// A theme for customizing the appearance and internal layout of
    /// `ElevatedButton`s.
    pub elevated_button_theme: ElevatedButtonThemeData,
    /// A theme for customizing the visual properties of `ExpansionTile`s.
    pub expansion_tile_theme: ExpansionTileThemeData,
    /// A theme for customizing the appearance and internal layout of
    /// `FilledButton`s.
    pub filled_button_theme: FilledButtonThemeData,
    /// A theme for customizing the shape, elevation, and color of a
    /// `FloatingActionButton`.
    pub floating_action_button_theme: FloatingActionButtonThemeData,
    /// A theme for customizing the appearance and internal layout of
    /// `IconButton`s.
    pub icon_button_theme: IconButtonThemeData,
    /// A theme for customizing the appearance of `ListTile` widgets.
    pub list_tile_theme: ListTileThemeData,
    /// A theme for customizing the color, shape, elevation, and other `MenuStyle`
    /// aspects of the menu bar created by the `MenuBar` widget.
    pub menu_bar_theme: MenuBarThemeData,
    /// A theme for customizing the color, shape, elevation, and text style of
    /// cascading menu buttons created by `SubmenuButton` or `MenuItemButton`.
    pub menu_button_theme: MenuButtonThemeData,
    /// A theme for customizing the color, shape, elevation, and other `MenuStyle`
    /// attributes of menus created by the `SubmenuButton` widget.
    pub menu_theme: MenuThemeData,
    /// A theme for customizing the background color, text style, and icon themes
    /// of a `NavigationBar`.
    pub navigation_bar_theme: NavigationBarThemeData,
    /// A theme for customizing the background color, text style, and icon themes
    /// of a `NavigationDrawer`.
    pub navigation_drawer_theme: NavigationDrawerThemeData,
    /// A theme for customizing the background color, elevation, text style, and
    /// icon themes of a `NavigationRail`.
    pub navigation_rail_theme: NavigationRailThemeData,
    /// A theme for customizing the appearance and internal layout of
    /// `OutlinedButton`s.
    pub outlined_button_theme: OutlinedButtonThemeData,
    /// A theme for customizing the color, shape, elevation, and text style of
    /// popup menus.
    pub popup_menu_theme: PopupMenuThemeData,
    /// A theme for customizing the appearance and layout of `ProgressIndicator` widgets.
    pub progress_indicator_theme: ProgressIndicatorThemeData,
    /// A theme for customizing the appearance and layout of `Radio` widgets.
    pub radio_theme: RadioThemeData,
    /// The colors and shapes used to render `Slider`.
    ///
    /// This is the value returned from `SliderTheme::of`.
    pub slider_theme: SliderThemeData,
    /// A theme for customizing colors, shape, elevation, and behavior of a `SnackBar`.
    pub snack_bar_theme: SnackBarThemeData,
    /// A theme for customizing the appearance and layout of `Switch` widgets.
    pub switch_theme: SwitchThemeData,
    /// A theme for customizing the size, shape, and color of the tab bar indicator.
    pub tab_bar_theme: TabBarThemeData,
    /// A theme for customizing the appearance and internal layout of
    /// `TextButton`s.
    pub text_button_theme: TextButtonThemeData,
    /// A theme for customizing the appearance for text selection in `TextField` and
    /// `SelectableText` widgets.
    pub text_selection_theme: TextSelectionThemeData,
    /// A theme for customizing the appearance and layout of `Tooltip` widgets.
    pub tooltip_theme: TooltipThemeData,

    // DEPRECATED (newest deprecations at the bottom)
    /// The background color of `Dialog` elements.
    #[deprecated(
        note = "Use DialogThemeData.backgroundColor instead. This feature was deprecated after v3.27.0-0.1.pre."
    )]
    pub dialog_background_color: Color,
    /// The color of the selected tab indicator in a tab bar.
    #[deprecated(
        note = "Use TabBarThemeData.indicatorColor instead. This feature was deprecated after v3.28.0-1.0.pre."
    )]
    pub indicator_color: Color,
}

impl ThemeData {
    /// Create a [`ThemeData`] that's used to configure a `Theme`.
    ///
    /// The [`color_scheme`](Self::color_scheme) and [`text_theme`](Self::text_theme) are used
    /// by the Material components to compute default values for visual properties. The API
    /// documentation for each component widget explains exactly how the defaults are
    /// computed.
    ///
    /// When providing a [`ColorScheme`], apps can either provide one directly with
    /// the `color_scheme` parameter, or have one generated for them. A provided
    /// `color_scheme` is used directly and is not affected by `brightness`,
    /// `color_scheme_seed`, or `contrast_level`.
    ///
    /// If a `color_scheme_seed` is provided, [`ColorScheme::from_seed`] generates the
    /// color scheme from `color_scheme_seed`, `brightness`, and `contrast_level`.
    /// In this case, `contrast_level` can be any value between -1.0 and 1.0
    /// inclusive. All of the color scheme's contrasting colors will meet
    /// accessibility guidelines for readability. (See [`ColorScheme::from_seed`] for
    /// more details.)
    ///
    /// If `color_scheme_seed` is null, use `brightness` and `contrast_level` to
    /// select a token-backed Material 3 baseline color scheme.
    /// Use [`ContrastLevel::STANDARD`], [`ContrastLevel::MEDIUM`], or
    /// [`ContrastLevel::HIGH`] to choose between the standard, medium, and high
    /// contrast baselines. Other custom `contrast_level` values, such as 0.6,
    /// require a `color_scheme_seed`.
    ///
    /// If the app wants to customize a generated color scheme, it can use
    /// [`ColorScheme::from_seed`] directly and then [`ColorScheme::copy_with`] on the
    /// result to override any colors that need to be replaced. The result of
    /// this can be used as the `color_scheme` directly.
    ///
    /// For historical reasons, instead of using a `color_scheme_seed` or
    /// `color_scheme`, you can provide either a `primary_color` or `primary_swatch`
    /// to construct the `color_scheme`, but the results will not be as complete
    /// as when using generation from a seed color.
    ///
    /// If `color_scheme_seed` is non-null then `color_scheme`, `primary_color` and
    /// `primary_swatch` must all be null.
    ///
    /// The [`text_theme`](Self::text_theme) [`TextStyle`](waft_painting::TextStyle) colors are
    /// black if the color scheme's brightness is [`Brightness::Light`], and white for
    /// [`Brightness::Dark`].
    ///
    /// See also:
    ///
    ///  * [`ThemeData::from`], which creates a ThemeData from a [`ColorScheme`].
    ///  * [`ThemeData::light`], which creates the default light theme.
    ///  * [`ThemeData::dark`], which creates the default dark theme.
    ///  * [`ColorScheme::from_seed`], which is used to create a [`ColorScheme`] from a seed
    ///    color.
    pub fn builder() -> ThemeDataBuilder {
        ThemeDataBuilder::default()
    }

    /// Dart's `ThemeData()`: [`builder`](Self::builder) with no arguments.
    pub fn new(app: &App) -> ThemeData {
        ThemeData::builder().build(app)
    }

    /// Create a [`ThemeData`] based on the colors in the given `color_scheme` and
    /// text styles of the optional `text_theme`.
    ///
    /// If `color_scheme`.brightness is [`Brightness::Dark`] then
    /// [`apply_elevation_overlay_color`](Self::apply_elevation_overlay_color) will be set to
    /// true to support the Material dark theme method for indicating elevation by applying
    /// a semi-transparent on_surface color on top of the surface color.
    ///
    /// This is the recommended method to theme your application. As we move
    /// forward we will be converting all the widget implementations to only use
    /// colors or colors derived from those in [`ColorScheme`].
    ///
    /// See <https://material.io/design/color/> for
    /// more discussion on how to pick the right colors.
    #[allow(clippy::should_implement_trait)]
    pub fn from(color_scheme: ColorScheme) -> ThemeDataFromBuilder {
        ThemeDataFromBuilder {
            color_scheme,
            text_theme: None,
            use_material3: None,
        }
    }

    /// A default light theme.
    ///
    /// This theme does not contain text geometry. Instead, it is expected that
    /// this theme is localized using text geometry using [`ThemeData::localize`].
    pub fn light(app: &App, use_material3: Option<bool>) -> ThemeData {
        let builder = ThemeData::builder().brightness(Brightness::Light);
        match use_material3 {
            Some(use_material3) => builder.use_material3(use_material3),
            None => builder,
        }
        .build(app)
    }

    /// A default dark theme.
    ///
    /// This theme does not contain text geometry. Instead, it is expected that
    /// this theme is localized using text geometry using [`ThemeData::localize`].
    pub fn dark(app: &App, use_material3: Option<bool>) -> ThemeData {
        let builder = ThemeData::builder().brightness(Brightness::Dark);
        match use_material3 {
            Some(use_material3) => builder.use_material3(use_material3),
            None => builder,
        }
        .build(app)
    }

    /// The default color theme. Same as [`ThemeData::light`].
    ///
    /// This is used by `Theme::of` when no theme has been specified.
    ///
    /// This theme does not contain text geometry. Instead, it is expected that
    /// this theme is localized using text geometry using [`ThemeData::localize`].
    ///
    /// Most applications would use `Theme::of`, which provides correct localized
    /// text geometry.
    pub fn fallback(app: &App, use_material3: Option<bool>) -> ThemeData {
        ThemeData::light(app, use_material3)
    }

    /// Used to obtain a particular [`Adaptation`] from [`adaptation_map`](Self::adaptation_map).
    ///
    /// To get an adaptation, use `Theme::of(app, context).get_adaptation::<MyAdaptation>()`.
    pub fn get_adaptation<T: 'static>(&self) -> Option<&dyn Adaptation<T>> {
        self.adaptation_map
            .get(&TypeId::of::<T>())
            .and_then(AdaptationRef::downcast::<T>)
    }

    fn create_adaptation_map(adaptations: Vec<AdaptationRef>) -> IndexMap<TypeId, AdaptationRef> {
        adaptations
            .into_iter()
            .map(|adaptation| (adaptation.adaptation_type(), adaptation))
            .collect()
    }

    /// The overall theme brightness.
    ///
    /// The default [`TextStyle`](waft_painting::TextStyle) color for the
    /// [`text_theme`](Self::text_theme) is black if the theme is constructed with
    /// [`Brightness::Light`] and white if the theme is constructed with
    /// [`Brightness::Dark`].
    pub fn brightness(&self) -> Brightness {
        self.color_scheme.brightness
    }

    /// Used to obtain a particular [`ThemeExtension`] from [`extensions`](Self::extensions).
    ///
    /// Obtain with `Theme::of(app, context).extension::<MyThemeExtension>()`.
    pub fn extension<T: ThemeExtension>(&self) -> Option<&T> {
        self.extensions
            .get(&TypeId::of::<T>())
            .and_then(|extension| extension.as_any().downcast_ref::<T>())
    }

    /// Creates a copy of this theme but with the given fields replaced with the new values.
    ///
    /// The `brightness` value is applied to the [`color_scheme`](Self::color_scheme).
    ///
    /// Dart's named arguments are the setters of the returned [`ThemeDataCopyWith`], then
    /// [`ThemeDataCopyWith::build`].
    pub fn copy_with(&self) -> ThemeDataCopyWith {
        ThemeDataCopyWith {
            theme: Box::new(self.clone()),
            color_scheme: None,
            brightness: None,
        }
    }

    // The number 5 was chosen without any real science or research behind it. It
    // just seemed like a number that's not too big (we should be able to fit 5
    // copies of ThemeData in memory comfortably) and not too small (most apps
    // shouldn't have more than 5 theme/localization pairs).
    const LOCALIZED_THEME_DATA_CACHE_SIZE: usize = 5;

    /// Returns a new theme built by merging the text geometry provided by the
    /// `local_text_geometry` theme with the `base_theme`.
    ///
    /// For those text styles in the `base_theme` whose [`TextStyle::inherit`](waft_painting::TextStyle::inherit)
    /// is set to true, the returned theme's text styles inherit the geometric properties
    /// of `local_text_geometry`. The resulting text styles' `inherit` is
    /// set to those provided by `local_text_geometry`.
    pub fn localize(
        app: &mut App,
        base_theme: &Rc<ThemeData>,
        local_text_geometry: &Rc<TextTheme>,
    ) -> Rc<ThemeData> {
        // WARNING: this method memoizes the result in a cache based on the
        // previously seen baseTheme and localTextGeometry. Memoization is safe
        // because all inputs and outputs of this function are deeply immutable, and
        // the computations are referentially transparent. It only short-circuits
        // the computation if the new inputs are identical() to the previous ones.
        // It does not use the == operator, which performs a costly deep comparison.
        //
        // When changing this method, make sure the memoization logic is correct.
        // Remember:
        //
        // There are only two hard things in Computer Science: cache invalidation
        // and naming things. -- Phil Karlton

        let localized_theme_data_cache = app.singleton::<LocalizedThemeDataCache>();
        app.get_mut(localized_theme_data_cache).0.put_if_absent(
            IdentityThemeDataCacheKey {
                base_theme: Rc::clone(base_theme),
                local_text_geometry: Rc::clone(local_text_geometry),
            },
            || {
                Rc::new(
                    base_theme
                        .copy_with()
                        .primary_text_theme(
                            local_text_geometry.merge(Some(&base_theme.primary_text_theme)),
                        )
                        .text_theme(local_text_geometry.merge(Some(&base_theme.text_theme)))
                        .build(),
                )
            },
        )
    }

    /// Determines whether the given [`Color`] is [`Brightness::Light`] or
    /// [`Brightness::Dark`].
    ///
    /// This compares the luminosity of the given color to a threshold value that
    /// matches the Material Design specification.
    pub fn estimate_brightness_for_color(color: Color) -> Brightness {
        let relative_luminance = color.compute_luminance();

        // See <https://www.w3.org/TR/WCAG20/#contrast-ratiodef>
        // The spec says to use kThreshold=0.0525, but Material Design appears to bias
        // more towards using light text than WCAG20 recommends. Material Design spec
        // doesn't say what value to use, but 0.15 seemed close to what the Material
        // Design spec shows for its color palette on
        // <https://material.io/go/design-theming#color-color-palette>.
        const K_THRESHOLD: f64 = 0.15;
        if (relative_luminance + 0.05) * (relative_luminance + 0.05) > K_THRESHOLD {
            return Brightness::Light;
        }
        Brightness::Dark
    }

    /// Linearly interpolate between two [`extensions`](Self::extensions).
    ///
    /// Includes all theme extensions in `a` and `b`.
    ///
    /// {@macro dart.ui.shadow.lerp}
    fn lerp_theme_extensions(
        a: &ThemeData,
        b: &ThemeData,
        t: f64,
    ) -> IndexMap<TypeId, ThemeExtensionRef> {
        // Lerp [a].
        let mut new_extensions: IndexMap<TypeId, ThemeExtensionRef> = a
            .extensions
            .iter()
            .map(|(&id, extension_a)| {
                let extension_b = b.extensions.get(&id);
                (
                    id,
                    Rc::clone(extension_a).lerp(extension_b.map(|extension| &**extension), t),
                )
            })
            .collect();
        // Add [b]-only extensions.
        new_extensions.extend(
            b.extensions
                .iter()
                .filter(|(id, _)| !a.extensions.contains_key(*id))
                .map(|(&id, extension)| (id, Rc::clone(extension))),
        );

        new_extensions
    }

    /// Convert the `extensions_iterable` passed to [`ThemeDataBuilder::extensions`] or
    /// [`ThemeDataCopyWith::extensions`] to the stored [`extensions`](Self::extensions) map,
    /// where each entry's key consists of the extension's type.
    fn theme_extension_iterable_to_map(
        extensions_iterable: Vec<ThemeExtensionRef>,
    ) -> IndexMap<TypeId, ThemeExtensionRef> {
        extensions_iterable
            .into_iter()
            .map(|extension| (extension.extension_type(), extension))
            .collect()
    }

    /// Linearly interpolate between two themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    #[allow(deprecated)]
    pub fn lerp(a: &ThemeData, b: &ThemeData, t: f64) -> ThemeData {
        if std::ptr::eq(a, b) {
            return a.clone();
        }
        let color = |a: Color, b: Color| Color::lerp(Some(a), Some(b), t).expect("both sides");
        ThemeData {
            // For the sanity of the reader, make sure these properties are in the same
            // order in every place that they are separated by section comments (e.g.
            // GENERAL CONFIGURATION). Each section except for deprecations should be
            // alphabetical by symbol name.

            // GENERAL CONFIGURATION
            adaptation_map: if t < 0.5 {
                a.adaptation_map.clone()
            } else {
                b.adaptation_map.clone()
            },
            apply_elevation_overlay_color: if t < 0.5 {
                a.apply_elevation_overlay_color
            } else {
                b.apply_elevation_overlay_color
            },
            cupertino_override_theme: if t < 0.5 {
                a.cupertino_override_theme.clone()
            } else {
                b.cupertino_override_theme.clone()
            },
            extensions: ThemeData::lerp_theme_extensions(a, b, t),
            input_decoration_theme: if t < 0.5 {
                a.input_decoration_theme.clone()
            } else {
                b.input_decoration_theme.clone()
            },
            material_tap_target_size: if t < 0.5 {
                a.material_tap_target_size
            } else {
                b.material_tap_target_size
            },
            page_transitions_theme: if t < 0.5 {
                a.page_transitions_theme.clone()
            } else {
                b.page_transitions_theme.clone()
            },
            platform: if t < 0.5 { a.platform } else { b.platform },
            scrollbar_theme: ScrollbarThemeData::lerp(
                Some(&a.scrollbar_theme),
                Some(&b.scrollbar_theme),
                t,
            ),
            splash_factory: if t < 0.5 {
                Rc::clone(&a.splash_factory)
            } else {
                Rc::clone(&b.splash_factory)
            },
            use_material3: if t < 0.5 {
                a.use_material3
            } else {
                b.use_material3
            },
            visual_density: VisualDensity::lerp(a.visual_density, b.visual_density, t),
            // COLOR
            canvas_color: color(a.canvas_color, b.canvas_color),
            card_color: color(a.card_color, b.card_color),
            color_scheme: ColorScheme::lerp(&a.color_scheme, &b.color_scheme, t),
            disabled_color: color(a.disabled_color, b.disabled_color),
            divider_color: color(a.divider_color, b.divider_color),
            focus_color: color(a.focus_color, b.focus_color),
            highlight_color: color(a.highlight_color, b.highlight_color),
            hint_color: color(a.hint_color, b.hint_color),
            hover_color: color(a.hover_color, b.hover_color),
            primary_color: color(a.primary_color, b.primary_color),
            primary_color_dark: color(a.primary_color_dark, b.primary_color_dark),
            primary_color_light: color(a.primary_color_light, b.primary_color_light),
            scaffold_background_color: color(
                a.scaffold_background_color,
                b.scaffold_background_color,
            ),
            secondary_header_color: color(a.secondary_header_color, b.secondary_header_color),
            shadow_color: color(a.shadow_color, b.shadow_color),
            splash_color: color(a.splash_color, b.splash_color),
            unselected_widget_color: color(a.unselected_widget_color, b.unselected_widget_color),
            // TYPOGRAPHY & ICONOGRAPHY
            icon_theme: IconThemeData::lerp(Some(&a.icon_theme), Some(&b.icon_theme), t),
            primary_icon_theme: IconThemeData::lerp(
                Some(&a.primary_icon_theme),
                Some(&b.primary_icon_theme),
                t,
            ),
            primary_text_theme: TextTheme::lerp(
                Some(&a.primary_text_theme),
                Some(&b.primary_text_theme),
                t,
            ),
            text_theme: TextTheme::lerp(Some(&a.text_theme), Some(&b.text_theme), t),
            typography: Typography::lerp(&a.typography, &b.typography, t),
            // COMPONENT THEMES
            action_icon_theme: ActionIconThemeData::lerp(
                a.action_icon_theme.as_ref(),
                b.action_icon_theme.as_ref(),
                t,
            ),
            app_bar_theme: AppBarThemeData::lerp(&a.app_bar_theme, &b.app_bar_theme, t),
            badge_theme: BadgeThemeData::lerp(Some(&a.badge_theme), Some(&b.badge_theme), t),
            banner_theme: MaterialBannerThemeData::lerp(
                Some(&a.banner_theme),
                Some(&b.banner_theme),
                t,
            ),
            bottom_navigation_bar_theme: BottomNavigationBarThemeData::lerp(
                Some(&a.bottom_navigation_bar_theme),
                Some(&b.bottom_navigation_bar_theme),
                t,
            ),
            bottom_sheet_theme: BottomSheetThemeData::lerp(
                Some(&a.bottom_sheet_theme),
                Some(&b.bottom_sheet_theme),
                t,
            )
            .expect("both sides"),
            card_theme: CardThemeData::lerp(Some(&a.card_theme), Some(&b.card_theme), t),
            checkbox_theme: CheckboxThemeData::lerp(
                Some(&a.checkbox_theme),
                Some(&b.checkbox_theme),
                t,
            ),
            dialog_theme: DialogThemeData::lerp(Some(&a.dialog_theme), Some(&b.dialog_theme), t),
            divider_theme: DividerThemeData::lerp(
                Some(&a.divider_theme),
                Some(&b.divider_theme),
                t,
            ),
            drawer_theme: DrawerThemeData::lerp(Some(&a.drawer_theme), Some(&b.drawer_theme), t)
                .expect("both sides"),
            elevated_button_theme: ElevatedButtonThemeData::lerp(
                Some(&a.elevated_button_theme),
                Some(&b.elevated_button_theme),
                t,
            )
            .expect("both sides"),
            expansion_tile_theme: ExpansionTileThemeData::lerp(
                Some(&a.expansion_tile_theme),
                Some(&b.expansion_tile_theme),
                t,
            )
            .expect("both sides"),
            filled_button_theme: FilledButtonThemeData::lerp(
                Some(&a.filled_button_theme),
                Some(&b.filled_button_theme),
                t,
            )
            .expect("both sides"),
            floating_action_button_theme: FloatingActionButtonThemeData::lerp(
                Some(&a.floating_action_button_theme),
                Some(&b.floating_action_button_theme),
                t,
            )
            .expect("both sides"),
            icon_button_theme: IconButtonThemeData::lerp(
                Some(&a.icon_button_theme),
                Some(&b.icon_button_theme),
                t,
            )
            .expect("both sides"),
            list_tile_theme: ListTileThemeData::lerp(
                Some(&a.list_tile_theme),
                Some(&b.list_tile_theme),
                t,
            )
            .expect("both sides"),
            menu_bar_theme: MenuBarThemeData::lerp(
                Some(&a.menu_bar_theme),
                Some(&b.menu_bar_theme),
                t,
            )
            .expect("both sides"),
            menu_button_theme: MenuButtonThemeData::lerp(
                Some(&a.menu_button_theme),
                Some(&b.menu_button_theme),
                t,
            )
            .expect("both sides"),
            menu_theme: MenuThemeData::lerp(Some(&a.menu_theme), Some(&b.menu_theme), t)
                .expect("both sides"),
            navigation_bar_theme: NavigationBarThemeData::lerp(
                Some(&a.navigation_bar_theme),
                Some(&b.navigation_bar_theme),
                t,
            )
            .expect("both sides"),
            navigation_drawer_theme: NavigationDrawerThemeData::lerp(
                Some(&a.navigation_drawer_theme),
                Some(&b.navigation_drawer_theme),
                t,
            )
            .expect("both sides"),
            navigation_rail_theme: NavigationRailThemeData::lerp(
                Some(&a.navigation_rail_theme),
                Some(&b.navigation_rail_theme),
                t,
            )
            .expect("both sides"),
            outlined_button_theme: OutlinedButtonThemeData::lerp(
                Some(&a.outlined_button_theme),
                Some(&b.outlined_button_theme),
                t,
            )
            .expect("both sides"),
            popup_menu_theme: PopupMenuThemeData::lerp(
                Some(&a.popup_menu_theme),
                Some(&b.popup_menu_theme),
                t,
            )
            .expect("both sides"),
            progress_indicator_theme: ProgressIndicatorThemeData::lerp(
                Some(&a.progress_indicator_theme),
                Some(&b.progress_indicator_theme),
                t,
            )
            .expect("both sides"),
            radio_theme: RadioThemeData::lerp(Some(&a.radio_theme), Some(&b.radio_theme), t),
            slider_theme: SliderThemeData::lerp(&a.slider_theme, &b.slider_theme, t),
            snack_bar_theme: SnackBarThemeData::lerp(
                Some(&a.snack_bar_theme),
                Some(&b.snack_bar_theme),
                t,
            ),
            switch_theme: SwitchThemeData::lerp(Some(&a.switch_theme), Some(&b.switch_theme), t),
            tab_bar_theme: TabBarThemeData::lerp(&a.tab_bar_theme, &b.tab_bar_theme, t),
            text_button_theme: TextButtonThemeData::lerp(
                Some(&a.text_button_theme),
                Some(&b.text_button_theme),
                t,
            )
            .expect("both sides"),
            text_selection_theme: TextSelectionThemeData::lerp(
                Some(&a.text_selection_theme),
                Some(&b.text_selection_theme),
                t,
            )
            .expect("both sides"),
            tooltip_theme: TooltipThemeData::lerp(
                Some(&a.tooltip_theme),
                Some(&b.tooltip_theme),
                t,
            )
            .expect("both sides"),
            // DEPRECATED (newest deprecations at the bottom)
            dialog_background_color: color(a.dialog_background_color, b.dialog_background_color),
            indicator_color: color(a.indicator_color, b.indicator_color),
        }
    }
}

/// Dart's `mapEquals` over two extension maps, with each extension's `==`.
fn extensions_equal(
    a: &IndexMap<TypeId, ThemeExtensionRef>,
    b: &IndexMap<TypeId, ThemeExtensionRef>,
) -> bool {
    a.len() == b.len()
        && a.iter().all(|(id, extension)| {
            b.get(id)
                .is_some_and(|other| extension.eq_extension(&**other))
        })
}

impl PartialEq for ThemeData {
    #[allow(deprecated)]
    fn eq(&self, other: &ThemeData) -> bool {
        // For the sanity of the reader, make sure these properties are in the same
        // order in every place that they are separated by section comments (e.g.
        // GENERAL CONFIGURATION). Each section except for deprecations should be
        // alphabetical by symbol name.
        other.adaptation_map == self.adaptation_map
            && other.apply_elevation_overlay_color == self.apply_elevation_overlay_color
            && other.cupertino_override_theme == self.cupertino_override_theme
            && extensions_equal(&other.extensions, &self.extensions)
            && other.input_decoration_theme == self.input_decoration_theme
            && other.material_tap_target_size == self.material_tap_target_size
            && other.page_transitions_theme == self.page_transitions_theme
            && other.platform == self.platform
            && other.scrollbar_theme == self.scrollbar_theme
            && Rc::ptr_eq(&other.splash_factory, &self.splash_factory)
            && other.use_material3 == self.use_material3
            && other.visual_density == self.visual_density
            // COLOR
            && other.canvas_color == self.canvas_color
            && other.card_color == self.card_color
            && other.color_scheme == self.color_scheme
            && other.disabled_color == self.disabled_color
            && other.divider_color == self.divider_color
            && other.focus_color == self.focus_color
            && other.highlight_color == self.highlight_color
            && other.hint_color == self.hint_color
            && other.hover_color == self.hover_color
            && other.primary_color == self.primary_color
            && other.primary_color_dark == self.primary_color_dark
            && other.primary_color_light == self.primary_color_light
            && other.scaffold_background_color == self.scaffold_background_color
            && other.secondary_header_color == self.secondary_header_color
            && other.shadow_color == self.shadow_color
            && other.splash_color == self.splash_color
            && other.unselected_widget_color == self.unselected_widget_color
            // TYPOGRAPHY & ICONOGRAPHY
            && other.icon_theme == self.icon_theme
            && other.primary_icon_theme == self.primary_icon_theme
            && other.primary_text_theme == self.primary_text_theme
            && other.text_theme == self.text_theme
            && other.typography == self.typography
            // COMPONENT THEMES
            && other.action_icon_theme == self.action_icon_theme
            && other.app_bar_theme == self.app_bar_theme
            && other.badge_theme == self.badge_theme
            && other.banner_theme == self.banner_theme
            && other.bottom_navigation_bar_theme == self.bottom_navigation_bar_theme
            && other.bottom_sheet_theme == self.bottom_sheet_theme
            && other.card_theme == self.card_theme
            && other.checkbox_theme == self.checkbox_theme
            && other.dialog_theme == self.dialog_theme
            && other.divider_theme == self.divider_theme
            && other.drawer_theme == self.drawer_theme
            && other.elevated_button_theme == self.elevated_button_theme
            && other.expansion_tile_theme == self.expansion_tile_theme
            && other.filled_button_theme == self.filled_button_theme
            && other.floating_action_button_theme == self.floating_action_button_theme
            && other.icon_button_theme == self.icon_button_theme
            && other.list_tile_theme == self.list_tile_theme
            && other.menu_bar_theme == self.menu_bar_theme
            && other.menu_button_theme == self.menu_button_theme
            && other.menu_theme == self.menu_theme
            && other.navigation_bar_theme == self.navigation_bar_theme
            && other.navigation_drawer_theme == self.navigation_drawer_theme
            && other.navigation_rail_theme == self.navigation_rail_theme
            && other.outlined_button_theme == self.outlined_button_theme
            && other.popup_menu_theme == self.popup_menu_theme
            && other.progress_indicator_theme == self.progress_indicator_theme
            && other.radio_theme == self.radio_theme
            && other.slider_theme == self.slider_theme
            && other.snack_bar_theme == self.snack_bar_theme
            && other.switch_theme == self.switch_theme
            && other.tab_bar_theme == self.tab_bar_theme
            && other.text_button_theme == self.text_button_theme
            && other.text_selection_theme == self.text_selection_theme
            && other.tooltip_theme == self.tooltip_theme
            // DEPRECATED (newest deprecations at the bottom)
            && other.dialog_background_color == self.dialog_background_color
            && other.indicator_color == self.indicator_color
    }
}

/// Dart's `ThemeData` factory constructor, collecting its named arguments; [`build`](Self::build)
/// fills every field the caller left out.
#[derive(Clone, Debug, Default)]
pub struct ThemeDataBuilder {
    // For the sanity of the reader, make sure these properties are in the same
    // order in every place that they are separated by section comments (e.g.
    // GENERAL CONFIGURATION). Each section except for deprecations should be
    // alphabetical by symbol name.

    // GENERAL CONFIGURATION
    adaptations: Option<Vec<AdaptationRef>>,
    apply_elevation_overlay_color: Option<bool>,
    cupertino_override_theme: Option<Box<NoDefaultCupertinoThemeData>>,
    extensions: Option<Vec<ThemeExtensionRef>>,
    input_decoration_theme: Option<Box<InputDecorationThemeData>>,
    material_tap_target_size: Option<MaterialTapTargetSize>,
    page_transitions_theme: Option<PageTransitionsTheme>,
    platform: Option<TargetPlatform>,
    scrollbar_theme: Option<ScrollbarThemeData>,
    splash_factory: Option<InteractiveInkFeatureFactoryRef>,
    use_material3: Option<bool>,
    visual_density: Option<VisualDensity>,
    // COLOR
    color_scheme: Option<ColorScheme>,
    brightness: Option<Brightness>,
    color_scheme_seed: Option<Color>,
    contrast_level: Option<f64>,
    // [colorScheme] is the preferred way to configure colors. The [Color] properties
    // listed below (as well as primarySwatch) will gradually be phased out, see
    // https://github.com/flutter/flutter/issues/91772.
    canvas_color: Option<Color>,
    card_color: Option<Color>,
    disabled_color: Option<Color>,
    divider_color: Option<Color>,
    focus_color: Option<Color>,
    highlight_color: Option<Color>,
    hint_color: Option<Color>,
    hover_color: Option<Color>,
    primary_color: Option<Color>,
    primary_color_dark: Option<Color>,
    primary_color_light: Option<Color>,
    primary_swatch: Option<MaterialColor>,
    scaffold_background_color: Option<Color>,
    secondary_header_color: Option<Color>,
    shadow_color: Option<Color>,
    splash_color: Option<Color>,
    unselected_widget_color: Option<Color>,
    // TYPOGRAPHY & ICONOGRAPHY
    font_family: Option<String>,
    font_family_fallback: Option<Vec<String>>,
    package: Option<String>,
    icon_theme: Option<IconThemeData>,
    primary_icon_theme: Option<IconThemeData>,
    primary_text_theme: Option<Box<TextTheme>>,
    text_theme: Option<Box<TextTheme>>,
    typography: Option<Box<Typography>>,
    // COMPONENT THEMES
    action_icon_theme: Option<ActionIconThemeData>,
    app_bar_theme: Option<Box<AppBarThemeData>>,
    badge_theme: Option<Box<BadgeThemeData>>,
    banner_theme: Option<Box<MaterialBannerThemeData>>,
    bottom_navigation_bar_theme: Option<Box<BottomNavigationBarThemeData>>,
    bottom_sheet_theme: Option<Box<BottomSheetThemeData>>,
    card_theme: Option<Box<CardThemeData>>,
    checkbox_theme: Option<Box<CheckboxThemeData>>,
    dialog_theme: Option<Box<DialogThemeData>>,
    divider_theme: Option<Box<DividerThemeData>>,
    drawer_theme: Option<Box<DrawerThemeData>>,
    elevated_button_theme: Option<Box<ElevatedButtonThemeData>>,
    expansion_tile_theme: Option<Box<ExpansionTileThemeData>>,
    filled_button_theme: Option<Box<FilledButtonThemeData>>,
    floating_action_button_theme: Option<Box<FloatingActionButtonThemeData>>,
    icon_button_theme: Option<Box<IconButtonThemeData>>,
    list_tile_theme: Option<Box<ListTileThemeData>>,
    menu_bar_theme: Option<Box<MenuBarThemeData>>,
    menu_button_theme: Option<Box<MenuButtonThemeData>>,
    menu_theme: Option<Box<MenuThemeData>>,
    navigation_bar_theme: Option<Box<NavigationBarThemeData>>,
    navigation_drawer_theme: Option<Box<NavigationDrawerThemeData>>,
    navigation_rail_theme: Option<Box<NavigationRailThemeData>>,
    outlined_button_theme: Option<Box<OutlinedButtonThemeData>>,
    popup_menu_theme: Option<Box<PopupMenuThemeData>>,
    progress_indicator_theme: Option<Box<ProgressIndicatorThemeData>>,
    radio_theme: Option<Box<RadioThemeData>>,
    slider_theme: Option<Box<SliderThemeData>>,
    snack_bar_theme: Option<Box<SnackBarThemeData>>,
    switch_theme: Option<Box<SwitchThemeData>>,
    tab_bar_theme: Option<Box<TabBarThemeData>>,
    text_button_theme: Option<Box<TextButtonThemeData>>,
    text_selection_theme: Option<Box<TextSelectionThemeData>>,
    tooltip_theme: Option<Box<TooltipThemeData>>,
    // DEPRECATED (newest deprecations at the bottom)
    dialog_background_color: Option<Color>,
    indicator_color: Option<Color>,
}

impl ThemeDataBuilder {
    /// Dart `ThemeData(adaptations:)`.
    pub fn adaptations(mut self, adaptations: Vec<AdaptationRef>) -> ThemeDataBuilder {
        self.adaptations = Some(adaptations);
        self
    }

    /// Dart `ThemeData(applyElevationOverlayColor:)`.
    pub fn apply_elevation_overlay_color(
        mut self,
        apply_elevation_overlay_color: bool,
    ) -> ThemeDataBuilder {
        self.apply_elevation_overlay_color = Some(apply_elevation_overlay_color);
        self
    }

    /// Dart `ThemeData(cupertinoOverrideTheme:)`. A [`CupertinoThemeData`] is passed as its
    /// `no_default()`, which is what Dart keeps of it.
    pub fn cupertino_override_theme(
        mut self,
        cupertino_override_theme: NoDefaultCupertinoThemeData,
    ) -> ThemeDataBuilder {
        self.cupertino_override_theme = Some(Box::new(cupertino_override_theme));
        self
    }

    /// Dart `ThemeData(extensions:)`.
    pub fn extensions(mut self, extensions: Vec<ThemeExtensionRef>) -> ThemeDataBuilder {
        self.extensions = Some(extensions);
        self
    }

    /// Dart `ThemeData(inputDecorationTheme:)`, an `InputDecorationThemeData` or an
    /// `InputDecorationTheme`, whose `data` is kept.
    pub fn input_decoration_theme(
        mut self,
        input_decoration_theme: impl Into<InputDecorationThemeData>,
    ) -> ThemeDataBuilder {
        self.input_decoration_theme = Some(Box::new(input_decoration_theme.into()));
        self
    }

    /// Dart `ThemeData(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> ThemeDataBuilder {
        self.material_tap_target_size = Some(material_tap_target_size);
        self
    }

    /// Dart `ThemeData(pageTransitionsTheme:)`.
    pub fn page_transitions_theme(
        mut self,
        page_transitions_theme: PageTransitionsTheme,
    ) -> ThemeDataBuilder {
        self.page_transitions_theme = Some(page_transitions_theme);
        self
    }

    /// Dart `ThemeData(platform:)`.
    pub fn platform(mut self, platform: TargetPlatform) -> ThemeDataBuilder {
        self.platform = Some(platform);
        self
    }

    /// Dart `ThemeData(scrollbarTheme:)`.
    pub fn scrollbar_theme(mut self, scrollbar_theme: ScrollbarThemeData) -> ThemeDataBuilder {
        self.scrollbar_theme = Some(scrollbar_theme);
        self
    }

    /// Dart `ThemeData(splashFactory:)`.
    pub fn splash_factory(
        mut self,
        splash_factory: InteractiveInkFeatureFactoryRef,
    ) -> ThemeDataBuilder {
        self.splash_factory = Some(splash_factory);
        self
    }

    /// Dart `ThemeData(useMaterial3:)`.
    pub fn use_material3(mut self, use_material3: bool) -> ThemeDataBuilder {
        self.use_material3 = Some(use_material3);
        self
    }

    /// Dart `ThemeData(visualDensity:)`.
    pub fn visual_density(mut self, visual_density: VisualDensity) -> ThemeDataBuilder {
        self.visual_density = Some(visual_density);
        self
    }

    /// Dart `ThemeData(colorScheme:)`.
    pub fn color_scheme(mut self, color_scheme: ColorScheme) -> ThemeDataBuilder {
        self.color_scheme = Some(color_scheme);
        self
    }

    /// Dart `ThemeData(brightness:)`.
    pub fn brightness(mut self, brightness: Brightness) -> ThemeDataBuilder {
        self.brightness = Some(brightness);
        self
    }

    /// Dart `ThemeData(colorSchemeSeed:)`.
    pub fn color_scheme_seed(mut self, color_scheme_seed: Color) -> ThemeDataBuilder {
        self.color_scheme_seed = Some(color_scheme_seed);
        self
    }

    /// Dart `ThemeData(contrastLevel:)`.
    pub fn contrast_level(mut self, contrast_level: f64) -> ThemeDataBuilder {
        self.contrast_level = Some(contrast_level);
        self
    }

    /// Dart `ThemeData(canvasColor:)`.
    pub fn canvas_color(mut self, canvas_color: Color) -> ThemeDataBuilder {
        self.canvas_color = Some(canvas_color);
        self
    }

    /// Dart `ThemeData(cardColor:)`.
    pub fn card_color(mut self, card_color: Color) -> ThemeDataBuilder {
        self.card_color = Some(card_color);
        self
    }

    /// Dart `ThemeData(disabledColor:)`.
    pub fn disabled_color(mut self, disabled_color: Color) -> ThemeDataBuilder {
        self.disabled_color = Some(disabled_color);
        self
    }

    /// Dart `ThemeData(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> ThemeDataBuilder {
        self.divider_color = Some(divider_color);
        self
    }

    /// Dart `ThemeData(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> ThemeDataBuilder {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `ThemeData(highlightColor:)`.
    pub fn highlight_color(mut self, highlight_color: Color) -> ThemeDataBuilder {
        self.highlight_color = Some(highlight_color);
        self
    }

    /// Dart `ThemeData(hintColor:)`.
    pub fn hint_color(mut self, hint_color: Color) -> ThemeDataBuilder {
        self.hint_color = Some(hint_color);
        self
    }

    /// Dart `ThemeData(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> ThemeDataBuilder {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `ThemeData(primaryColor:)`.
    pub fn primary_color(mut self, primary_color: Color) -> ThemeDataBuilder {
        self.primary_color = Some(primary_color);
        self
    }

    /// Dart `ThemeData(primaryColorDark:)`.
    pub fn primary_color_dark(mut self, primary_color_dark: Color) -> ThemeDataBuilder {
        self.primary_color_dark = Some(primary_color_dark);
        self
    }

    /// Dart `ThemeData(primaryColorLight:)`.
    pub fn primary_color_light(mut self, primary_color_light: Color) -> ThemeDataBuilder {
        self.primary_color_light = Some(primary_color_light);
        self
    }

    /// Dart `ThemeData(primarySwatch:)`.
    pub fn primary_swatch(mut self, primary_swatch: MaterialColor) -> ThemeDataBuilder {
        self.primary_swatch = Some(primary_swatch);
        self
    }

    /// Dart `ThemeData(scaffoldBackgroundColor:)`.
    pub fn scaffold_background_color(
        mut self,
        scaffold_background_color: Color,
    ) -> ThemeDataBuilder {
        self.scaffold_background_color = Some(scaffold_background_color);
        self
    }

    /// Dart `ThemeData(secondaryHeaderColor:)`.
    pub fn secondary_header_color(mut self, secondary_header_color: Color) -> ThemeDataBuilder {
        self.secondary_header_color = Some(secondary_header_color);
        self
    }

    /// Dart `ThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> ThemeDataBuilder {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `ThemeData(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> ThemeDataBuilder {
        self.splash_color = Some(splash_color);
        self
    }

    /// Dart `ThemeData(unselectedWidgetColor:)`.
    pub fn unselected_widget_color(mut self, unselected_widget_color: Color) -> ThemeDataBuilder {
        self.unselected_widget_color = Some(unselected_widget_color);
        self
    }

    /// Dart `ThemeData(fontFamily:)`.
    pub fn font_family(mut self, font_family: impl Into<String>) -> ThemeDataBuilder {
        self.font_family = Some(font_family.into());
        self
    }

    /// Dart `ThemeData(fontFamilyFallback:)`.
    pub fn font_family_fallback(mut self, font_family_fallback: Vec<String>) -> ThemeDataBuilder {
        self.font_family_fallback = Some(font_family_fallback);
        self
    }

    /// Dart `ThemeData(package:)`.
    pub fn package(mut self, package: impl Into<String>) -> ThemeDataBuilder {
        self.package = Some(package.into());
        self
    }

    /// Dart `ThemeData(iconTheme:)`.
    pub fn icon_theme(mut self, icon_theme: IconThemeData) -> ThemeDataBuilder {
        self.icon_theme = Some(icon_theme);
        self
    }

    /// Dart `ThemeData(primaryIconTheme:)`.
    pub fn primary_icon_theme(mut self, primary_icon_theme: IconThemeData) -> ThemeDataBuilder {
        self.primary_icon_theme = Some(primary_icon_theme);
        self
    }

    /// Dart `ThemeData(primaryTextTheme:)`.
    pub fn primary_text_theme(mut self, primary_text_theme: TextTheme) -> ThemeDataBuilder {
        self.primary_text_theme = Some(Box::new(primary_text_theme));
        self
    }

    /// Dart `ThemeData(textTheme:)`.
    pub fn text_theme(mut self, text_theme: TextTheme) -> ThemeDataBuilder {
        self.text_theme = Some(Box::new(text_theme));
        self
    }

    /// Dart `ThemeData(typography:)`.
    pub fn typography(mut self, typography: Typography) -> ThemeDataBuilder {
        self.typography = Some(Box::new(typography));
        self
    }

    /// Dart `ThemeData(actionIconTheme:)`.
    pub fn action_icon_theme(mut self, action_icon_theme: ActionIconThemeData) -> ThemeDataBuilder {
        self.action_icon_theme = Some(action_icon_theme);
        self
    }

    /// Dart `ThemeData(appBarTheme:)`.
    pub fn app_bar_theme(mut self, app_bar_theme: impl Into<AppBarThemeData>) -> ThemeDataBuilder {
        self.app_bar_theme = Some(Box::new(app_bar_theme.into()));
        self
    }

    /// Dart `ThemeData(badgeTheme:)`.
    pub fn badge_theme(mut self, badge_theme: BadgeThemeData) -> ThemeDataBuilder {
        self.badge_theme = Some(Box::new(badge_theme));
        self
    }

    /// Dart `ThemeData(bannerTheme:)`.
    pub fn banner_theme(mut self, banner_theme: MaterialBannerThemeData) -> ThemeDataBuilder {
        self.banner_theme = Some(Box::new(banner_theme));
        self
    }

    /// Dart `ThemeData(bottomNavigationBarTheme:)`.
    pub fn bottom_navigation_bar_theme(
        mut self,
        bottom_navigation_bar_theme: BottomNavigationBarThemeData,
    ) -> ThemeDataBuilder {
        self.bottom_navigation_bar_theme = Some(Box::new(bottom_navigation_bar_theme));
        self
    }

    /// Dart `ThemeData(bottomSheetTheme:)`.
    pub fn bottom_sheet_theme(
        mut self,
        bottom_sheet_theme: BottomSheetThemeData,
    ) -> ThemeDataBuilder {
        self.bottom_sheet_theme = Some(Box::new(bottom_sheet_theme));
        self
    }

    /// Dart `ThemeData(cardTheme:)`.
    pub fn card_theme(mut self, card_theme: CardThemeData) -> ThemeDataBuilder {
        self.card_theme = Some(Box::new(card_theme));
        self
    }

    /// Dart `ThemeData(checkboxTheme:)`.
    pub fn checkbox_theme(mut self, checkbox_theme: CheckboxThemeData) -> ThemeDataBuilder {
        self.checkbox_theme = Some(Box::new(checkbox_theme));
        self
    }

    /// Dart `ThemeData(dialogTheme:)`.
    pub fn dialog_theme(mut self, dialog_theme: DialogThemeData) -> ThemeDataBuilder {
        self.dialog_theme = Some(Box::new(dialog_theme));
        self
    }

    /// Dart `ThemeData(dividerTheme:)`.
    pub fn divider_theme(mut self, divider_theme: DividerThemeData) -> ThemeDataBuilder {
        self.divider_theme = Some(Box::new(divider_theme));
        self
    }

    /// Dart `ThemeData(drawerTheme:)`.
    pub fn drawer_theme(mut self, drawer_theme: DrawerThemeData) -> ThemeDataBuilder {
        self.drawer_theme = Some(Box::new(drawer_theme));
        self
    }

    /// Dart `ThemeData(elevatedButtonTheme:)`.
    pub fn elevated_button_theme(
        mut self,
        elevated_button_theme: ElevatedButtonThemeData,
    ) -> ThemeDataBuilder {
        self.elevated_button_theme = Some(Box::new(elevated_button_theme));
        self
    }

    /// Dart `ThemeData(expansionTileTheme:)`.
    pub fn expansion_tile_theme(
        mut self,
        expansion_tile_theme: ExpansionTileThemeData,
    ) -> ThemeDataBuilder {
        self.expansion_tile_theme = Some(Box::new(expansion_tile_theme));
        self
    }

    /// Dart `ThemeData(filledButtonTheme:)`.
    pub fn filled_button_theme(
        mut self,
        filled_button_theme: FilledButtonThemeData,
    ) -> ThemeDataBuilder {
        self.filled_button_theme = Some(Box::new(filled_button_theme));
        self
    }

    /// Dart `ThemeData(floatingActionButtonTheme:)`.
    pub fn floating_action_button_theme(
        mut self,
        floating_action_button_theme: FloatingActionButtonThemeData,
    ) -> ThemeDataBuilder {
        self.floating_action_button_theme = Some(Box::new(floating_action_button_theme));
        self
    }

    /// Dart `ThemeData(iconButtonTheme:)`.
    pub fn icon_button_theme(mut self, icon_button_theme: IconButtonThemeData) -> ThemeDataBuilder {
        self.icon_button_theme = Some(Box::new(icon_button_theme));
        self
    }

    /// Dart `ThemeData(listTileTheme:)`.
    pub fn list_tile_theme(mut self, list_tile_theme: ListTileThemeData) -> ThemeDataBuilder {
        self.list_tile_theme = Some(Box::new(list_tile_theme));
        self
    }

    /// Dart `ThemeData(menuBarTheme:)`.
    pub fn menu_bar_theme(mut self, menu_bar_theme: MenuBarThemeData) -> ThemeDataBuilder {
        self.menu_bar_theme = Some(Box::new(menu_bar_theme));
        self
    }

    /// Dart `ThemeData(menuButtonTheme:)`.
    pub fn menu_button_theme(mut self, menu_button_theme: MenuButtonThemeData) -> ThemeDataBuilder {
        self.menu_button_theme = Some(Box::new(menu_button_theme));
        self
    }

    /// Dart `ThemeData(menuTheme:)`.
    pub fn menu_theme(mut self, menu_theme: MenuThemeData) -> ThemeDataBuilder {
        self.menu_theme = Some(Box::new(menu_theme));
        self
    }

    /// Dart `ThemeData(navigationBarTheme:)`.
    pub fn navigation_bar_theme(
        mut self,
        navigation_bar_theme: NavigationBarThemeData,
    ) -> ThemeDataBuilder {
        self.navigation_bar_theme = Some(Box::new(navigation_bar_theme));
        self
    }

    /// Dart `ThemeData(navigationDrawerTheme:)`.
    pub fn navigation_drawer_theme(
        mut self,
        navigation_drawer_theme: NavigationDrawerThemeData,
    ) -> ThemeDataBuilder {
        self.navigation_drawer_theme = Some(Box::new(navigation_drawer_theme));
        self
    }

    /// Dart `ThemeData(navigationRailTheme:)`.
    pub fn navigation_rail_theme(
        mut self,
        navigation_rail_theme: NavigationRailThemeData,
    ) -> ThemeDataBuilder {
        self.navigation_rail_theme = Some(Box::new(navigation_rail_theme));
        self
    }

    /// Dart `ThemeData(outlinedButtonTheme:)`.
    pub fn outlined_button_theme(
        mut self,
        outlined_button_theme: OutlinedButtonThemeData,
    ) -> ThemeDataBuilder {
        self.outlined_button_theme = Some(Box::new(outlined_button_theme));
        self
    }

    /// Dart `ThemeData(popupMenuTheme:)`.
    pub fn popup_menu_theme(mut self, popup_menu_theme: PopupMenuThemeData) -> ThemeDataBuilder {
        self.popup_menu_theme = Some(Box::new(popup_menu_theme));
        self
    }

    /// Dart `ThemeData(progressIndicatorTheme:)`.
    pub fn progress_indicator_theme(
        mut self,
        progress_indicator_theme: ProgressIndicatorThemeData,
    ) -> ThemeDataBuilder {
        self.progress_indicator_theme = Some(Box::new(progress_indicator_theme));
        self
    }

    /// Dart `ThemeData(radioTheme:)`.
    pub fn radio_theme(mut self, radio_theme: RadioThemeData) -> ThemeDataBuilder {
        self.radio_theme = Some(Box::new(radio_theme));
        self
    }

    /// Dart `ThemeData(sliderTheme:)`.
    pub fn slider_theme(mut self, slider_theme: SliderThemeData) -> ThemeDataBuilder {
        self.slider_theme = Some(Box::new(slider_theme));
        self
    }

    /// Dart `ThemeData(snackBarTheme:)`.
    pub fn snack_bar_theme(mut self, snack_bar_theme: SnackBarThemeData) -> ThemeDataBuilder {
        self.snack_bar_theme = Some(Box::new(snack_bar_theme));
        self
    }

    /// Dart `ThemeData(switchTheme:)`.
    pub fn switch_theme(mut self, switch_theme: SwitchThemeData) -> ThemeDataBuilder {
        self.switch_theme = Some(Box::new(switch_theme));
        self
    }

    /// Dart `ThemeData(tabBarTheme:)`.
    pub fn tab_bar_theme(mut self, tab_bar_theme: TabBarThemeData) -> ThemeDataBuilder {
        self.tab_bar_theme = Some(Box::new(tab_bar_theme));
        self
    }

    /// Dart `ThemeData(textButtonTheme:)`.
    pub fn text_button_theme(mut self, text_button_theme: TextButtonThemeData) -> ThemeDataBuilder {
        self.text_button_theme = Some(Box::new(text_button_theme));
        self
    }

    /// Dart `ThemeData(textSelectionTheme:)`.
    pub fn text_selection_theme(
        mut self,
        text_selection_theme: TextSelectionThemeData,
    ) -> ThemeDataBuilder {
        self.text_selection_theme = Some(Box::new(text_selection_theme));
        self
    }

    /// Dart `ThemeData(tooltipTheme:)`.
    pub fn tooltip_theme(mut self, tooltip_theme: TooltipThemeData) -> ThemeDataBuilder {
        self.tooltip_theme = Some(Box::new(tooltip_theme));
        self
    }

    /// Dart `ThemeData(dialogBackgroundColor:)`.
    #[deprecated(
        note = "Use DialogThemeData.backgroundColor instead. This feature was deprecated after v3.27.0-0.1.pre."
    )]
    pub fn dialog_background_color(mut self, dialog_background_color: Color) -> ThemeDataBuilder {
        self.dialog_background_color = Some(dialog_background_color);
        self
    }

    /// Dart `ThemeData(indicatorColor:)`.
    #[deprecated(
        note = "Use TabBarThemeData.indicatorColor instead. This feature was deprecated after v3.28.0-1.0.pre."
    )]
    pub fn indicator_color(mut self, indicator_color: Color) -> ThemeDataBuilder {
        self.indicator_color = Some(indicator_color);
        self
    }

    /// The theme Dart's factory constructor returns. `app` gives `defaultTargetPlatform`,
    /// the default of `platform`.
    #[allow(deprecated)]
    pub fn build(self, app: &App) -> ThemeData {
        let ThemeDataBuilder {
            adaptations,
            mut apply_elevation_overlay_color,
            cupertino_override_theme,
            extensions,
            input_decoration_theme,
            mut material_tap_target_size,
            page_transitions_theme,
            platform,
            scrollbar_theme,
            splash_factory,
            use_material3,
            visual_density,
            mut color_scheme,
            brightness,
            color_scheme_seed,
            contrast_level,
            mut canvas_color,
            mut card_color,
            disabled_color,
            mut divider_color,
            focus_color,
            highlight_color,
            hint_color,
            hover_color,
            mut primary_color,
            primary_color_dark,
            primary_color_light,
            primary_swatch,
            mut scaffold_background_color,
            secondary_header_color,
            shadow_color,
            splash_color,
            unselected_widget_color,
            font_family,
            font_family_fallback,
            package,
            icon_theme,
            primary_icon_theme,
            primary_text_theme,
            text_theme,
            typography,
            action_icon_theme,
            app_bar_theme,
            badge_theme,
            banner_theme,
            bottom_navigation_bar_theme,
            bottom_sheet_theme,
            card_theme,
            checkbox_theme,
            dialog_theme,
            divider_theme,
            drawer_theme,
            elevated_button_theme,
            expansion_tile_theme,
            filled_button_theme,
            floating_action_button_theme,
            icon_button_theme,
            list_tile_theme,
            menu_bar_theme,
            menu_button_theme,
            menu_theme,
            navigation_bar_theme,
            navigation_drawer_theme,
            navigation_rail_theme,
            outlined_button_theme,
            popup_menu_theme,
            progress_indicator_theme,
            radio_theme,
            slider_theme,
            snack_bar_theme,
            switch_theme,
            tab_bar_theme,
            text_button_theme,
            text_selection_theme,
            tooltip_theme,
            mut dialog_background_color,
            mut indicator_color,
        } = self;

        // GENERAL CONFIGURATION
        let cupertino_override_theme =
            cupertino_override_theme.map(|override_theme| override_theme.no_default());
        let extensions = extensions.unwrap_or_default();
        let adaptations = adaptations.unwrap_or_default();
        let platform = platform.unwrap_or_else(|| app.platform().target_platform());
        match platform {
            TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => {
                material_tap_target_size.get_or_insert(MaterialTapTargetSize::Padded);
            }
            TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                material_tap_target_size.get_or_insert(MaterialTapTargetSize::ShrinkWrap);
            }
        }
        let page_transitions_theme = page_transitions_theme.unwrap_or_default();
        let scrollbar_theme = scrollbar_theme.unwrap_or_default();
        let visual_density =
            visual_density.unwrap_or_else(|| VisualDensity::default_density_for_platform(platform));
        let use_material3 = use_material3.unwrap_or(true);
        debug_assert!(
            contrast_level.is_none_or(|contrast_level| {
                contrast_level + PRECISION_ERROR_TOLERANCE >= -1.0
                    && contrast_level - PRECISION_ERROR_TOLERANCE <= 1.0
            }),
            "contrastLevel must be between -1.0 and 1.0 inclusive."
        );
        let contrast_level = contrast_level.unwrap_or(ContrastLevel::STANDARD);
        debug_assert!(
            color_scheme_seed.is_some() || debug_is_using_predefined_contrast_level(contrast_level),
            "colorSchemeSeed must be provided to use a custom contrastLevel. \
             Without a colorSchemeSeed, contrastLevel must be \
             ContrastLevel.standard (0.0), ContrastLevel.medium (0.5), or \
             ContrastLevel.high (1.0)."
        );
        let use_ink_sparkle = platform == TargetPlatform::Android && !K_IS_WEB;
        let splash_factory = splash_factory.unwrap_or_else(|| {
            if use_material3 {
                if use_ink_sparkle {
                    InkSparkle::splash_factory()
                } else {
                    InkRipple::splash_factory()
                }
            } else {
                InkSplash::splash_factory()
            }
        });

        // COLOR
        debug_assert!(
            color_scheme.is_none()
                || brightness.is_none()
                || color_scheme.as_ref().map(|scheme| scheme.brightness) == brightness,
            "ThemeData.brightness does not match ColorScheme.brightness. \
             Either override ColorScheme.brightness or ThemeData.brightness to \
             match the other."
        );
        debug_assert!(color_scheme_seed.is_none() || color_scheme.is_none());
        debug_assert!(color_scheme_seed.is_none() || primary_swatch.is_none());
        debug_assert!(color_scheme_seed.is_none() || primary_color.is_none());
        let effective_brightness = brightness
            .or(color_scheme.as_ref().map(|scheme| scheme.brightness))
            .unwrap_or(Brightness::Light);
        let is_dark = effective_brightness == Brightness::Dark;
        if color_scheme_seed.is_some() || use_material3 {
            if let Some(color_scheme_seed) = color_scheme_seed {
                color_scheme = Some(
                    ColorScheme::from_seed(color_scheme_seed)
                        .brightness(effective_brightness)
                        .contrast_level(contrast_level)
                        .build(),
                );
            }
            let color_scheme = color_scheme.get_or_insert_with(|| {
                default_m3_color_scheme(effective_brightness, contrast_level)
            });

            // For surfaces that use primary color in light themes and surface color in dark
            let primary_surface_color = if is_dark {
                color_scheme.surface
            } else {
                color_scheme.primary
            };
            let on_primary_surface_color = if is_dark {
                color_scheme.on_surface
            } else {
                color_scheme.on_primary
            };

            // Default some of the color settings to values from the color scheme
            primary_color.get_or_insert(primary_surface_color);
            canvas_color.get_or_insert(color_scheme.surface);
            scaffold_background_color.get_or_insert(color_scheme.surface);
            card_color.get_or_insert(color_scheme.surface);
            divider_color.get_or_insert(color_scheme.outline());
            dialog_background_color.get_or_insert(color_scheme.surface);
            indicator_color.get_or_insert(on_primary_surface_color);
            apply_elevation_overlay_color.get_or_insert(brightness == Some(Brightness::Dark));
        }
        let apply_elevation_overlay_color = apply_elevation_overlay_color.unwrap_or(false);
        let primary_swatch = primary_swatch.unwrap_or(Colors::BLUE);
        let primary_color = primary_color.unwrap_or_else(|| {
            if is_dark {
                Colors::GREY.shade900()
            } else {
                primary_swatch.primary()
            }
        });
        let estimated_primary_color_brightness =
            ThemeData::estimate_brightness_for_color(primary_color);
        let primary_color_light = primary_color_light.unwrap_or_else(|| {
            if is_dark {
                Colors::GREY.shade500()
            } else {
                primary_swatch.shade100()
            }
        });
        let primary_color_dark = primary_color_dark.unwrap_or_else(|| {
            if is_dark {
                Colors::BLACK
            } else {
                primary_swatch.shade700()
            }
        });
        let primary_is_dark = estimated_primary_color_brightness == Brightness::Dark;
        let focus_color = focus_color.unwrap_or_else(|| {
            if is_dark {
                Colors::WHITE.with_opacity(0.12)
            } else {
                Colors::BLACK.with_opacity(0.12)
            }
        });
        let hover_color = hover_color.unwrap_or_else(|| {
            if is_dark {
                Colors::WHITE.with_opacity(0.04)
            } else {
                Colors::BLACK.with_opacity(0.04)
            }
        });
        let shadow_color = shadow_color.unwrap_or(Colors::BLACK);
        let canvas_color = canvas_color.unwrap_or_else(|| {
            if is_dark {
                grey(850)
            } else {
                Colors::GREY.shade50()
            }
        });
        let scaffold_background_color = scaffold_background_color.unwrap_or(canvas_color);
        let card_color = card_color.unwrap_or_else(|| {
            if is_dark {
                Colors::GREY.shade800()
            } else {
                Colors::WHITE
            }
        });
        let divider_color = divider_color.unwrap_or(if is_dark {
            Color::new(0x1FFFFFFF)
        } else {
            Color::new(0x1F000000)
        });
        // Create a ColorScheme that is backwards compatible as possible
        // with the existing default ThemeData color values.
        let color_scheme = color_scheme.unwrap_or_else(|| {
            ColorScheme::from_swatch()
                .primary_swatch(primary_swatch.clone())
                .accent_color(if is_dark {
                    Colors::TEAL_ACCENT.shade200()
                } else {
                    primary_swatch.shade500()
                })
                .card_color(card_color)
                .background_color(if is_dark {
                    Colors::GREY.shade700()
                } else {
                    primary_swatch.shade200()
                })
                .error_color(Colors::RED.shade700())
                .brightness(effective_brightness)
                .build()
        });
        let unselected_widget_color = unselected_widget_color.unwrap_or(if is_dark {
            Colors::WHITE70
        } else {
            Colors::BLACK54
        });
        // Spec doesn't specify a dark theme secondaryHeaderColor, this is a guess.
        let secondary_header_color = secondary_header_color.unwrap_or_else(|| {
            if is_dark {
                Colors::GREY.shade700()
            } else {
                primary_swatch.shade50()
            }
        });
        let hint_color = hint_color.unwrap_or_else(|| {
            if is_dark {
                Colors::WHITE60
            } else {
                Colors::BLACK.with_opacity(0.6)
            }
        });
        let disabled_color = disabled_color.unwrap_or(if is_dark {
            Colors::WHITE38
        } else {
            Colors::BLACK38
        });
        let highlight_color = highlight_color.unwrap_or(if is_dark {
            Color::new(0x40CCCCCC)
        } else {
            Color::new(0x66BCBCBC)
        });
        let splash_color = splash_color.unwrap_or(if is_dark {
            Color::new(0x40CCCCCC)
        } else {
            Color::new(0x66C8C8C8)
        });

        // TYPOGRAPHY & ICONOGRAPHY
        let typography = typography.map(|typography| *typography).unwrap_or_else(|| {
            if use_material3 {
                Typography::material2021()
                    .platform(Some(platform))
                    .color_scheme(color_scheme.clone())
                    .build()
            } else {
                Typography::material2014().platform(Some(platform)).build()
            }
        });
        let mut default_text_theme = TextTheme::clone(if is_dark {
            &typography.white
        } else {
            &typography.black
        });
        let mut default_primary_text_theme = TextTheme::clone(if primary_is_dark {
            &typography.white
        } else {
            &typography.black
        });
        if let Some(font_family) = &font_family {
            default_text_theme = default_text_theme
                .apply()
                .font_family(font_family.clone())
                .into_text_theme();
            default_primary_text_theme = default_primary_text_theme
                .apply()
                .font_family(font_family.clone())
                .into_text_theme();
        }
        if let Some(font_family_fallback) = &font_family_fallback {
            default_text_theme = default_text_theme
                .apply()
                .font_family_fallback(font_family_fallback.clone())
                .into_text_theme();
            default_primary_text_theme = default_primary_text_theme
                .apply()
                .font_family_fallback(font_family_fallback.clone())
                .into_text_theme();
        }
        if let Some(package) = &package {
            default_text_theme = default_text_theme
                .apply()
                .package(package.clone())
                .into_text_theme();
            default_primary_text_theme = default_primary_text_theme
                .apply()
                .package(package.clone())
                .into_text_theme();
        }
        let text_theme = default_text_theme.merge(text_theme.as_deref());
        let primary_text_theme = default_primary_text_theme.merge(primary_text_theme.as_deref());
        let icon_theme = icon_theme.unwrap_or_else(|| {
            if is_dark {
                IconThemeData::new().color(K_DEFAULT_ICON_LIGHT_COLOR)
            } else {
                IconThemeData::new().color(K_DEFAULT_ICON_DARK_COLOR)
            }
        });
        let primary_icon_theme = primary_icon_theme.unwrap_or_else(|| {
            if primary_is_dark {
                IconThemeData::new().color(Colors::WHITE)
            } else {
                IconThemeData::new().color(Colors::BLACK)
            }
        });

        // DEPRECATED (newest deprecations at the bottom)
        let dialog_background_color = dialog_background_color.unwrap_or_else(|| {
            if is_dark {
                Colors::GREY.shade800()
            } else {
                Colors::WHITE
            }
        });
        let indicator_color =
            indicator_color.unwrap_or(if color_scheme.secondary == primary_color {
                Colors::WHITE
            } else {
                color_scheme.secondary
            });

        ThemeData {
            // For the sanity of the reader, make sure these properties are in the same
            // order in every place that they are separated by section comments (e.g.
            // GENERAL CONFIGURATION). Each section except for deprecations should be
            // alphabetical by symbol name.

            // GENERAL CONFIGURATION
            adaptation_map: ThemeData::create_adaptation_map(adaptations),
            apply_elevation_overlay_color,
            cupertino_override_theme,
            extensions: ThemeData::theme_extension_iterable_to_map(extensions),
            input_decoration_theme: unboxed_or_default(input_decoration_theme),
            material_tap_target_size: material_tap_target_size.expect("defaulted per platform"),
            page_transitions_theme,
            platform,
            scrollbar_theme,
            splash_factory,
            use_material3,
            visual_density,
            // COLOR
            canvas_color,
            card_color,
            color_scheme,
            disabled_color,
            divider_color,
            focus_color,
            highlight_color,
            hint_color,
            hover_color,
            primary_color,
            primary_color_dark,
            primary_color_light,
            scaffold_background_color,
            secondary_header_color,
            shadow_color,
            splash_color,
            unselected_widget_color,
            // TYPOGRAPHY & ICONOGRAPHY
            icon_theme,
            primary_text_theme,
            text_theme,
            typography,
            primary_icon_theme,
            // COMPONENT THEMES
            action_icon_theme,
            app_bar_theme: unboxed_or_default(app_bar_theme),
            badge_theme: unboxed_or_default(badge_theme),
            banner_theme: unboxed_or_default(banner_theme),
            bottom_navigation_bar_theme: unboxed_or_default(bottom_navigation_bar_theme),
            bottom_sheet_theme: unboxed_or_default(bottom_sheet_theme),
            card_theme: unboxed_or_default(card_theme),
            checkbox_theme: unboxed_or_default(checkbox_theme),
            dialog_theme: unboxed_or_default(dialog_theme),
            divider_theme: unboxed_or_default(divider_theme),
            drawer_theme: unboxed_or_default(drawer_theme),
            elevated_button_theme: unboxed_or_default(elevated_button_theme),
            expansion_tile_theme: unboxed_or_default(expansion_tile_theme),
            filled_button_theme: unboxed_or_default(filled_button_theme),
            floating_action_button_theme: unboxed_or_default(floating_action_button_theme),
            icon_button_theme: unboxed_or_default(icon_button_theme),
            list_tile_theme: unboxed_or_default(list_tile_theme),
            menu_bar_theme: unboxed_or_default(menu_bar_theme),
            menu_button_theme: unboxed_or_default(menu_button_theme),
            menu_theme: unboxed_or_default(menu_theme),
            navigation_bar_theme: unboxed_or_default(navigation_bar_theme),
            navigation_drawer_theme: unboxed_or_default(navigation_drawer_theme),
            navigation_rail_theme: unboxed_or_default(navigation_rail_theme),
            outlined_button_theme: unboxed_or_default(outlined_button_theme),
            popup_menu_theme: unboxed_or_default(popup_menu_theme),
            progress_indicator_theme: unboxed_or_default(progress_indicator_theme),
            radio_theme: unboxed_or_default(radio_theme),
            slider_theme: unboxed_or_default(slider_theme),
            snack_bar_theme: unboxed_or_default(snack_bar_theme),
            switch_theme: unboxed_or_default(switch_theme),
            tab_bar_theme: unboxed_or_default(tab_bar_theme),
            text_button_theme: unboxed_or_default(text_button_theme),
            text_selection_theme: unboxed_or_default(text_selection_theme),
            tooltip_theme: unboxed_or_default(tooltip_theme),
            // DEPRECATED (newest deprecations at the bottom)
            dialog_background_color,
            indicator_color,
        }
    }
}

/// A boxed builder argument, or the type's default when it was not given.
fn unboxed_or_default<T: Default>(value: Option<Box<T>>) -> T {
    value.map_or_else(T::default, |value| *value)
}

/// `Colors.grey[shade]` for a shade without its own accessor.
fn grey(shade: i32) -> Color {
    Colors::GREY
        .get(&shade)
        .expect("the grey swatch has this shade")
}

/// Dart's `ThemeData.from` named arguments.
#[derive(Clone, Debug)]
pub struct ThemeDataFromBuilder {
    color_scheme: ColorScheme,
    text_theme: Option<Box<TextTheme>>,
    use_material3: Option<bool>,
}

impl ThemeDataFromBuilder {
    /// Dart `ThemeData.from(textTheme:)`.
    pub fn text_theme(mut self, text_theme: TextTheme) -> ThemeDataFromBuilder {
        self.text_theme = Some(Box::new(text_theme));
        self
    }

    /// Dart `ThemeData.from(useMaterial3:)`.
    pub fn use_material3(mut self, use_material3: bool) -> ThemeDataFromBuilder {
        self.use_material3 = Some(use_material3);
        self
    }

    /// The theme `ThemeData.from` returns.
    #[allow(deprecated)]
    pub fn build(self, app: &App) -> ThemeData {
        let color_scheme = self.color_scheme;
        let is_dark = color_scheme.brightness == Brightness::Dark;

        // For surfaces that use primary color in light themes and surface color in dark
        let primary_surface_color = if is_dark {
            color_scheme.surface
        } else {
            color_scheme.primary
        };
        let on_primary_surface_color = if is_dark {
            color_scheme.on_surface
        } else {
            color_scheme.on_primary
        };

        let mut builder = ThemeData::builder()
            .brightness(color_scheme.brightness)
            .primary_color(primary_surface_color)
            .canvas_color(color_scheme.surface)
            .scaffold_background_color(color_scheme.surface)
            .card_color(color_scheme.surface)
            .divider_color(color_scheme.on_surface.with_opacity(0.12))
            .dialog_background_color(color_scheme.surface)
            .indicator_color(on_primary_surface_color)
            .apply_elevation_overlay_color(is_dark)
            .color_scheme(color_scheme);
        if let Some(text_theme) = self.text_theme {
            builder = builder.text_theme(*text_theme);
        }
        if let Some(use_material3) = self.use_material3 {
            builder = builder.use_material3(use_material3);
        }
        builder.build(app)
    }
}

/// Dart's `ThemeData.copyWith` named arguments over the theme they copy;
/// [`build`](Self::build) applies `brightness` to the colour scheme, whichever order the two
/// were given in.
#[derive(Clone, Debug)]
pub struct ThemeDataCopyWith {
    theme: Box<ThemeData>,
    color_scheme: Option<ColorScheme>,
    brightness: Option<Brightness>,
}

impl ThemeDataCopyWith {
    /// Dart `copyWith(adaptations:)`.
    pub fn adaptations(mut self, adaptations: Vec<AdaptationRef>) -> ThemeDataCopyWith {
        self.theme.adaptation_map = ThemeData::create_adaptation_map(adaptations);
        self
    }

    /// Dart `copyWith(applyElevationOverlayColor:)`.
    pub fn apply_elevation_overlay_color(
        mut self,
        apply_elevation_overlay_color: bool,
    ) -> ThemeDataCopyWith {
        self.theme.apply_elevation_overlay_color = apply_elevation_overlay_color;
        self
    }

    /// Dart `copyWith(cupertinoOverrideTheme:)`.
    pub fn cupertino_override_theme(
        mut self,
        cupertino_override_theme: NoDefaultCupertinoThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.cupertino_override_theme = Some(cupertino_override_theme.no_default());
        self
    }

    /// Dart `copyWith(extensions:)`.
    pub fn extensions(mut self, extensions: Vec<ThemeExtensionRef>) -> ThemeDataCopyWith {
        self.theme.extensions = ThemeData::theme_extension_iterable_to_map(extensions);
        self
    }

    /// Dart `copyWith(inputDecorationTheme:)`, an `InputDecorationThemeData` or an
    /// `InputDecorationTheme`, whose `data` is kept.
    pub fn input_decoration_theme(
        mut self,
        input_decoration_theme: impl Into<InputDecorationThemeData>,
    ) -> ThemeDataCopyWith {
        self.theme.input_decoration_theme = input_decoration_theme.into();
        self
    }

    /// Dart `copyWith(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> ThemeDataCopyWith {
        self.theme.material_tap_target_size = material_tap_target_size;
        self
    }

    /// Dart `copyWith(pageTransitionsTheme:)`.
    pub fn page_transitions_theme(
        mut self,
        page_transitions_theme: PageTransitionsTheme,
    ) -> ThemeDataCopyWith {
        self.theme.page_transitions_theme = page_transitions_theme;
        self
    }

    /// Dart `copyWith(platform:)`.
    pub fn platform(mut self, platform: TargetPlatform) -> ThemeDataCopyWith {
        self.theme.platform = platform;
        self
    }

    /// Dart `copyWith(scrollbarTheme:)`.
    pub fn scrollbar_theme(mut self, scrollbar_theme: ScrollbarThemeData) -> ThemeDataCopyWith {
        self.theme.scrollbar_theme = scrollbar_theme;
        self
    }

    /// Dart `copyWith(splashFactory:)`.
    pub fn splash_factory(
        mut self,
        splash_factory: InteractiveInkFeatureFactoryRef,
    ) -> ThemeDataCopyWith {
        self.theme.splash_factory = splash_factory;
        self
    }

    /// Dart `copyWith(visualDensity:)`.
    pub fn visual_density(mut self, visual_density: VisualDensity) -> ThemeDataCopyWith {
        self.theme.visual_density = visual_density;
        self
    }

    /// Dart `copyWith(colorScheme:)`.
    pub fn color_scheme(mut self, color_scheme: ColorScheme) -> ThemeDataCopyWith {
        self.color_scheme = Some(color_scheme);
        self
    }

    /// Dart `copyWith(brightness:)`.
    pub fn brightness(mut self, brightness: Brightness) -> ThemeDataCopyWith {
        self.brightness = Some(brightness);
        self
    }

    /// Dart `copyWith(canvasColor:)`.
    pub fn canvas_color(mut self, canvas_color: Color) -> ThemeDataCopyWith {
        self.theme.canvas_color = canvas_color;
        self
    }

    /// Dart `copyWith(cardColor:)`.
    pub fn card_color(mut self, card_color: Color) -> ThemeDataCopyWith {
        self.theme.card_color = card_color;
        self
    }

    /// Dart `copyWith(disabledColor:)`.
    pub fn disabled_color(mut self, disabled_color: Color) -> ThemeDataCopyWith {
        self.theme.disabled_color = disabled_color;
        self
    }

    /// Dart `copyWith(dividerColor:)`.
    pub fn divider_color(mut self, divider_color: Color) -> ThemeDataCopyWith {
        self.theme.divider_color = divider_color;
        self
    }

    /// Dart `copyWith(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> ThemeDataCopyWith {
        self.theme.focus_color = focus_color;
        self
    }

    /// Dart `copyWith(highlightColor:)`.
    pub fn highlight_color(mut self, highlight_color: Color) -> ThemeDataCopyWith {
        self.theme.highlight_color = highlight_color;
        self
    }

    /// Dart `copyWith(hintColor:)`.
    pub fn hint_color(mut self, hint_color: Color) -> ThemeDataCopyWith {
        self.theme.hint_color = hint_color;
        self
    }

    /// Dart `copyWith(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> ThemeDataCopyWith {
        self.theme.hover_color = hover_color;
        self
    }

    /// Dart `copyWith(primaryColor:)`.
    pub fn primary_color(mut self, primary_color: Color) -> ThemeDataCopyWith {
        self.theme.primary_color = primary_color;
        self
    }

    /// Dart `copyWith(primaryColorDark:)`.
    pub fn primary_color_dark(mut self, primary_color_dark: Color) -> ThemeDataCopyWith {
        self.theme.primary_color_dark = primary_color_dark;
        self
    }

    /// Dart `copyWith(primaryColorLight:)`.
    pub fn primary_color_light(mut self, primary_color_light: Color) -> ThemeDataCopyWith {
        self.theme.primary_color_light = primary_color_light;
        self
    }

    /// Dart `copyWith(scaffoldBackgroundColor:)`.
    pub fn scaffold_background_color(
        mut self,
        scaffold_background_color: Color,
    ) -> ThemeDataCopyWith {
        self.theme.scaffold_background_color = scaffold_background_color;
        self
    }

    /// Dart `copyWith(secondaryHeaderColor:)`.
    pub fn secondary_header_color(mut self, secondary_header_color: Color) -> ThemeDataCopyWith {
        self.theme.secondary_header_color = secondary_header_color;
        self
    }

    /// Dart `copyWith(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> ThemeDataCopyWith {
        self.theme.shadow_color = shadow_color;
        self
    }

    /// Dart `copyWith(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> ThemeDataCopyWith {
        self.theme.splash_color = splash_color;
        self
    }

    /// Dart `copyWith(unselectedWidgetColor:)`.
    pub fn unselected_widget_color(mut self, unselected_widget_color: Color) -> ThemeDataCopyWith {
        self.theme.unselected_widget_color = unselected_widget_color;
        self
    }

    /// Dart `copyWith(iconTheme:)`.
    pub fn icon_theme(mut self, icon_theme: IconThemeData) -> ThemeDataCopyWith {
        self.theme.icon_theme = icon_theme;
        self
    }

    /// Dart `copyWith(primaryIconTheme:)`.
    pub fn primary_icon_theme(mut self, primary_icon_theme: IconThemeData) -> ThemeDataCopyWith {
        self.theme.primary_icon_theme = primary_icon_theme;
        self
    }

    /// Dart `copyWith(primaryTextTheme:)`.
    pub fn primary_text_theme(mut self, primary_text_theme: TextTheme) -> ThemeDataCopyWith {
        self.theme.primary_text_theme = primary_text_theme;
        self
    }

    /// Dart `copyWith(textTheme:)`.
    pub fn text_theme(mut self, text_theme: TextTheme) -> ThemeDataCopyWith {
        self.theme.text_theme = text_theme;
        self
    }

    /// Dart `copyWith(typography:)`.
    pub fn typography(mut self, typography: Typography) -> ThemeDataCopyWith {
        self.theme.typography = typography;
        self
    }

    /// Dart `copyWith(actionIconTheme:)`.
    pub fn action_icon_theme(
        mut self,
        action_icon_theme: ActionIconThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.action_icon_theme = Some(action_icon_theme);
        self
    }

    /// Dart `copyWith(appBarTheme:)`.
    pub fn app_bar_theme(mut self, app_bar_theme: impl Into<AppBarThemeData>) -> ThemeDataCopyWith {
        self.theme.app_bar_theme = app_bar_theme.into();
        self
    }

    /// Dart `copyWith(badgeTheme:)`.
    pub fn badge_theme(mut self, badge_theme: BadgeThemeData) -> ThemeDataCopyWith {
        self.theme.badge_theme = badge_theme;
        self
    }

    /// Dart `copyWith(bannerTheme:)`.
    pub fn banner_theme(mut self, banner_theme: MaterialBannerThemeData) -> ThemeDataCopyWith {
        self.theme.banner_theme = banner_theme;
        self
    }

    /// Dart `copyWith(bottomNavigationBarTheme:)`.
    pub fn bottom_navigation_bar_theme(
        mut self,
        bottom_navigation_bar_theme: BottomNavigationBarThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.bottom_navigation_bar_theme = bottom_navigation_bar_theme;
        self
    }

    /// Dart `copyWith(bottomSheetTheme:)`.
    pub fn bottom_sheet_theme(
        mut self,
        bottom_sheet_theme: BottomSheetThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.bottom_sheet_theme = bottom_sheet_theme;
        self
    }

    /// Dart `copyWith(cardTheme:)`.
    pub fn card_theme(mut self, card_theme: CardThemeData) -> ThemeDataCopyWith {
        self.theme.card_theme = card_theme;
        self
    }

    /// Dart `copyWith(checkboxTheme:)`.
    pub fn checkbox_theme(mut self, checkbox_theme: CheckboxThemeData) -> ThemeDataCopyWith {
        self.theme.checkbox_theme = checkbox_theme;
        self
    }

    /// Dart `copyWith(dialogTheme:)`.
    pub fn dialog_theme(mut self, dialog_theme: DialogThemeData) -> ThemeDataCopyWith {
        self.theme.dialog_theme = dialog_theme;
        self
    }

    /// Dart `copyWith(dividerTheme:)`.
    pub fn divider_theme(mut self, divider_theme: DividerThemeData) -> ThemeDataCopyWith {
        self.theme.divider_theme = divider_theme;
        self
    }

    /// Dart `copyWith(drawerTheme:)`.
    pub fn drawer_theme(mut self, drawer_theme: DrawerThemeData) -> ThemeDataCopyWith {
        self.theme.drawer_theme = drawer_theme;
        self
    }

    /// Dart `copyWith(elevatedButtonTheme:)`.
    pub fn elevated_button_theme(
        mut self,
        elevated_button_theme: ElevatedButtonThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.elevated_button_theme = elevated_button_theme;
        self
    }

    /// Dart `copyWith(expansionTileTheme:)`.
    pub fn expansion_tile_theme(
        mut self,
        expansion_tile_theme: ExpansionTileThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.expansion_tile_theme = expansion_tile_theme;
        self
    }

    /// Dart `copyWith(filledButtonTheme:)`.
    pub fn filled_button_theme(
        mut self,
        filled_button_theme: FilledButtonThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.filled_button_theme = filled_button_theme;
        self
    }

    /// Dart `copyWith(floatingActionButtonTheme:)`.
    pub fn floating_action_button_theme(
        mut self,
        floating_action_button_theme: FloatingActionButtonThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.floating_action_button_theme = floating_action_button_theme;
        self
    }

    /// Dart `copyWith(iconButtonTheme:)`.
    pub fn icon_button_theme(
        mut self,
        icon_button_theme: IconButtonThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.icon_button_theme = icon_button_theme;
        self
    }

    /// Dart `copyWith(listTileTheme:)`.
    pub fn list_tile_theme(mut self, list_tile_theme: ListTileThemeData) -> ThemeDataCopyWith {
        self.theme.list_tile_theme = list_tile_theme;
        self
    }

    /// Dart `copyWith(menuBarTheme:)`.
    pub fn menu_bar_theme(mut self, menu_bar_theme: MenuBarThemeData) -> ThemeDataCopyWith {
        self.theme.menu_bar_theme = menu_bar_theme;
        self
    }

    /// Dart `copyWith(menuButtonTheme:)`.
    pub fn menu_button_theme(
        mut self,
        menu_button_theme: MenuButtonThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.menu_button_theme = menu_button_theme;
        self
    }

    /// Dart `copyWith(menuTheme:)`.
    pub fn menu_theme(mut self, menu_theme: MenuThemeData) -> ThemeDataCopyWith {
        self.theme.menu_theme = menu_theme;
        self
    }

    /// Dart `copyWith(navigationBarTheme:)`.
    pub fn navigation_bar_theme(
        mut self,
        navigation_bar_theme: NavigationBarThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.navigation_bar_theme = navigation_bar_theme;
        self
    }

    /// Dart `copyWith(navigationDrawerTheme:)`.
    pub fn navigation_drawer_theme(
        mut self,
        navigation_drawer_theme: NavigationDrawerThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.navigation_drawer_theme = navigation_drawer_theme;
        self
    }

    /// Dart `copyWith(navigationRailTheme:)`.
    pub fn navigation_rail_theme(
        mut self,
        navigation_rail_theme: NavigationRailThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.navigation_rail_theme = navigation_rail_theme;
        self
    }

    /// Dart `copyWith(outlinedButtonTheme:)`.
    pub fn outlined_button_theme(
        mut self,
        outlined_button_theme: OutlinedButtonThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.outlined_button_theme = outlined_button_theme;
        self
    }

    /// Dart `copyWith(popupMenuTheme:)`.
    pub fn popup_menu_theme(mut self, popup_menu_theme: PopupMenuThemeData) -> ThemeDataCopyWith {
        self.theme.popup_menu_theme = popup_menu_theme;
        self
    }

    /// Dart `copyWith(progressIndicatorTheme:)`.
    pub fn progress_indicator_theme(
        mut self,
        progress_indicator_theme: ProgressIndicatorThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.progress_indicator_theme = progress_indicator_theme;
        self
    }

    /// Dart `copyWith(radioTheme:)`.
    pub fn radio_theme(mut self, radio_theme: RadioThemeData) -> ThemeDataCopyWith {
        self.theme.radio_theme = radio_theme;
        self
    }

    /// Dart `copyWith(sliderTheme:)`.
    pub fn slider_theme(mut self, slider_theme: SliderThemeData) -> ThemeDataCopyWith {
        self.theme.slider_theme = slider_theme;
        self
    }

    /// Dart `copyWith(snackBarTheme:)`.
    pub fn snack_bar_theme(mut self, snack_bar_theme: SnackBarThemeData) -> ThemeDataCopyWith {
        self.theme.snack_bar_theme = snack_bar_theme;
        self
    }

    /// Dart `copyWith(switchTheme:)`.
    pub fn switch_theme(mut self, switch_theme: SwitchThemeData) -> ThemeDataCopyWith {
        self.theme.switch_theme = switch_theme;
        self
    }

    /// Dart `copyWith(tabBarTheme:)`.
    pub fn tab_bar_theme(mut self, tab_bar_theme: TabBarThemeData) -> ThemeDataCopyWith {
        self.theme.tab_bar_theme = tab_bar_theme;
        self
    }

    /// Dart `copyWith(textButtonTheme:)`.
    pub fn text_button_theme(
        mut self,
        text_button_theme: TextButtonThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.text_button_theme = text_button_theme;
        self
    }

    /// Dart `copyWith(textSelectionTheme:)`.
    pub fn text_selection_theme(
        mut self,
        text_selection_theme: TextSelectionThemeData,
    ) -> ThemeDataCopyWith {
        self.theme.text_selection_theme = text_selection_theme;
        self
    }

    /// Dart `copyWith(tooltipTheme:)`.
    pub fn tooltip_theme(mut self, tooltip_theme: TooltipThemeData) -> ThemeDataCopyWith {
        self.theme.tooltip_theme = tooltip_theme;
        self
    }

    /// Dart `copyWith(useMaterial3:)`.
    #[deprecated(
        note = "Use a ThemeData constructor (.from, .light, or .dark) instead. These constructors all have a useMaterial3 argument, and they set appropriate default values based on its value. See the useMaterial3 API documentation for full details. This feature was deprecated after v3.13.0-0.2.pre."
    )]
    pub fn use_material3(mut self, use_material3: bool) -> ThemeDataCopyWith {
        self.theme.use_material3 = use_material3;
        self
    }

    /// Dart `copyWith(dialogBackgroundColor:)`.
    #[deprecated(
        note = "Use DialogThemeData.backgroundColor instead. This feature was deprecated after v3.27.0-0.1.pre."
    )]
    #[allow(deprecated)]
    pub fn dialog_background_color(mut self, dialog_background_color: Color) -> ThemeDataCopyWith {
        self.theme.dialog_background_color = dialog_background_color;
        self
    }

    /// Dart `copyWith(indicatorColor:)`.
    #[deprecated(
        note = "Use TabBarThemeData.indicatorColor instead. This feature was deprecated after v3.28.0-1.0.pre."
    )]
    #[allow(deprecated)]
    pub fn indicator_color(mut self, indicator_color: Color) -> ThemeDataCopyWith {
        self.theme.indicator_color = indicator_color;
        self
    }

    /// The copy.
    pub fn build(self) -> ThemeData {
        let mut theme = *self.theme;
        let mut color_scheme = self.color_scheme.unwrap_or(theme.color_scheme).copy_with();
        if let Some(brightness) = self.brightness {
            color_scheme = color_scheme.with_brightness(brightness);
        }
        theme.color_scheme = color_scheme;
        theme
    }
}

/// Dart's static `ThemeData._localizedThemeDataCache`, one per app.
struct LocalizedThemeDataCache(FifoCache<IdentityThemeDataCacheKey, Rc<ThemeData>>);

impl Default for LocalizedThemeDataCache {
    fn default() -> LocalizedThemeDataCache {
        LocalizedThemeDataCache(FifoCache::new(ThemeData::LOCALIZED_THEME_DATA_CACHE_SIZE))
    }
}

/// Dart's `_IdentityThemeDataCacheKey`: equal only to a key holding the same two objects.
///
/// It holds both, so neither allocation is reused for another theme while the key is cached.
struct IdentityThemeDataCacheKey {
    base_theme: Rc<ThemeData>,
    local_text_geometry: Rc<TextTheme>,
}

impl Hash for IdentityThemeDataCacheKey {
    // Using XOR to make the hash function as fast as possible (e.g. Jenkins is
    // noticeably slower).
    fn hash<H: Hasher>(&self, state: &mut H) {
        (Rc::as_ptr(&self.base_theme).addr() ^ Rc::as_ptr(&self.local_text_geometry).addr())
            .hash(state);
    }
}

impl PartialEq for IdentityThemeDataCacheKey {
    fn eq(&self, other: &IdentityThemeDataCacheKey) -> bool {
        Rc::ptr_eq(&other.base_theme, &self.base_theme)
            && Rc::ptr_eq(&other.local_text_geometry, &self.local_text_geometry)
    }
}

impl Eq for IdentityThemeDataCacheKey {}

/// Cache of objects of limited size that uses the first in first out eviction
/// strategy (a.k.a least recently inserted).
///
/// The key that was inserted before all other keys is evicted first, i.e. the
/// one inserted least recently.
struct FifoCache<K, V> {
    /// An [`IndexMap`] keeps its keys in the order they were inserted, as Dart's map
    /// literal does.
    cache: IndexMap<K, V>,

    /// Maximum number of entries to store in the cache.
    ///
    /// Once this many entries have been cached, the entry inserted least recently
    /// is evicted when adding a new entry.
    maximum_size: usize,
}

impl<K: Hash + Eq, V: Clone> FifoCache<K, V> {
    fn new(maximum_size: usize) -> FifoCache<K, V> {
        debug_assert!(maximum_size > 0);
        FifoCache {
            cache: IndexMap::new(),
            maximum_size,
        }
    }

    /// Returns the previously cached value for the given key, if available;
    /// if not, calls the given callback to obtain it first.
    fn put_if_absent(&mut self, key: K, loader: impl FnOnce() -> V) -> V {
        if let Some(result) = self.cache.get(&key) {
            return result.clone();
        }
        if self.cache.len() == self.maximum_size {
            self.cache.shift_remove_index(0);
        }
        let result = loader();
        self.cache.insert(key, result.clone());
        result
    }
}

fn default_m3_color_scheme(brightness: Brightness, contrast_level: f64) -> ColorScheme {
    match (brightness, contrast_level) {
        (Brightness::Light, ContrastLevel::MEDIUM) => color_scheme_light_medium_contrast_m3(),
        (Brightness::Dark, ContrastLevel::MEDIUM) => color_scheme_dark_medium_contrast_m3(),
        (Brightness::Light, ContrastLevel::HIGH) => color_scheme_light_high_contrast_m3(),
        (Brightness::Dark, ContrastLevel::HIGH) => color_scheme_dark_high_contrast_m3(),
        (Brightness::Light, _) => color_scheme_light_m3(),
        (Brightness::Dark, _) => color_scheme_dark_m3(),
    }
}

fn debug_is_using_predefined_contrast_level(contrast_level: f64) -> bool {
    contrast_level == ContrastLevel::STANDARD
        || contrast_level == ContrastLevel::MEDIUM
        || contrast_level == ContrastLevel::HIGH
}

/// A [`CupertinoThemeData`] that defers unspecified theme attributes to an
/// upstream Material [`ThemeData`].
///
/// This type of [`CupertinoThemeData`] is used by the Material `Theme` to
/// harmonize the `CupertinoTheme` with the material theme's colors and text
/// styles.
///
/// In the most basic case, [`ThemeData`]'s `cupertino_override_theme` is null and
/// descendant Cupertino widgets' styling is derived from the Material theme.
///
/// To override individual parts of the Material-derived Cupertino styling,
/// `cupertino_override_theme`'s construction parameters can be used.
///
/// To completely decouple the Cupertino styling from Material theme derivation,
/// another `CupertinoTheme` widget can be inserted as a descendant of the
/// Material `Theme`. On a `MaterialApp`, this can be done using the `builder`
/// parameter on the constructor.
///
/// Dart's `extends CupertinoThemeData`: this is the [`CupertinoThemeDataOverrides`] a
/// [`CupertinoThemeData`] holds, and [`new`](Self::new) returns that theme data. Dart's
/// `_cupertinoOverrideTheme` is the theme data's construction time specifications, which equal
/// it but for the selection handle color the constructor fills in from the Material theme.
///
/// Its `copy_with` is [`CupertinoThemeData::copy_with`], which copies the [`ThemeData`]'s
/// `cupertino_override_theme`: only the specified override attributes and the newly specified
/// parameters are in the copy, which keeps this Material theme. No derived attributes from iOS
/// defaults or from cascaded Material theme attributes are copied. It cannot change the base
/// Material [`ThemeData`]; to change it, create a new Material `Theme` and use
/// [`ThemeData::copy_with`] on the Material [`ThemeData`] instead.
///
/// See also:
///
///  * [`CupertinoThemeData`], whose null constructor parameters default to
///    reasonable iOS styling defaults rather than harmonizing with a Material
///    theme.
///  * `Theme`, widget which inserts a `CupertinoTheme` with this
///    [`MaterialBasedCupertinoThemeData`].
// This class subclasses CupertinoThemeData rather than composes one because it
// _is_ a CupertinoThemeData with partially altered behavior. e.g. its textTheme
// is from the superclass and based on the primaryColor but the primaryColor
// comes from the Material theme unless overridden.
#[derive(Debug)]
pub struct MaterialBasedCupertinoThemeData {
    material_theme: Rc<ThemeData>,
}

impl MaterialBasedCupertinoThemeData {
    /// Create a [`MaterialBasedCupertinoThemeData`] based on a Material [`ThemeData`]
    /// and its `cupertino_override_theme`.
    #[expect(
        clippy::new_ret_no_self,
        reason = "Dart's subclass constructor; the value is the base type with the overrides set"
    )]
    pub fn new(material_theme: &Rc<ThemeData>) -> CupertinoThemeData {
        MaterialBasedCupertinoThemeData::raw(
            Rc::clone(material_theme),
            material_theme
                .cupertino_override_theme
                .clone()
                .unwrap_or_else(|| CupertinoThemeData::new().no_default()),
        )
    }

    fn raw(
        material_theme: Rc<ThemeData>,
        cupertino_override_theme: NoDefaultCupertinoThemeData,
    ) -> CupertinoThemeData {
        // Pass all values to the superclass so Material-agnostic properties
        // like barBackgroundColor can still behave like a normal
        // CupertinoThemeData.
        let selection_handle_color =
            cupertino_override_theme
                .selection_handle_color
                .clone()
                .or(material_theme
                    .text_selection_theme
                    .selection_handle_color
                    .map(AnyColor::new));
        CupertinoThemeData::raw_with_overrides(
            NoDefaultCupertinoThemeData {
                selection_handle_color,
                ..cupertino_override_theme
            },
            Rc::new(MaterialBasedCupertinoThemeData { material_theme }),
        )
    }
}

impl CupertinoThemeDataOverrides for MaterialBasedCupertinoThemeData {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn brightness(
        &self,
        cupertino_override_theme: &NoDefaultCupertinoThemeData,
    ) -> Option<Brightness> {
        Some(
            cupertino_override_theme
                .brightness
                .unwrap_or_else(|| self.material_theme.brightness()),
        )
    }

    fn primary_color(&self, cupertino_override_theme: &NoDefaultCupertinoThemeData) -> AnyColor {
        cupertino_override_theme
            .primary_color
            .clone()
            .unwrap_or_else(|| AnyColor::new(self.material_theme.color_scheme.primary))
    }

    fn primary_contrasting_color(
        &self,
        cupertino_override_theme: &NoDefaultCupertinoThemeData,
    ) -> AnyColor {
        cupertino_override_theme
            .primary_contrasting_color
            .clone()
            .unwrap_or_else(|| AnyColor::new(self.material_theme.color_scheme.on_primary))
    }

    fn scaffold_background_color(
        &self,
        cupertino_override_theme: &NoDefaultCupertinoThemeData,
    ) -> AnyColor {
        cupertino_override_theme
            .scaffold_background_color
            .clone()
            .unwrap_or_else(|| AnyColor::new(self.material_theme.scaffold_background_color))
    }

    fn resolve_from(
        &self,
        this: &CupertinoThemeData,
        app: &mut App,
        context: BuildContext,
    ) -> CupertinoThemeData {
        // Only the cupertino override theme part will be resolved, as well as the
        // default text theme.
        // If the color comes from the material theme it's not resolved.
        let mut cupertino_override_theme_with_text_theme = this.no_default().copy_with();
        cupertino_override_theme_with_text_theme.text_theme = Some(this.text_theme());
        MaterialBasedCupertinoThemeData::raw(
            Rc::clone(&self.material_theme),
            cupertino_override_theme_with_text_theme.resolve_from(app, context),
        )
    }
}

/// A class for creating a Material theme with a color scheme based off of the
/// colors from a [`CupertinoThemeData`]. This is intended to be used only in the
/// case when a Material widget is unable to find a Material theme in the tree,
/// but is able to find a Cupertino theme. Most often this will occur when a
/// Material widget is used inside of a `CupertinoApp`.
///
/// Besides the colors, this theme will use all the defaults from Material's
/// [`ThemeData`], so if further customization is needed, it is best to manually
/// add a Material `Theme` above the `CupertinoApp`.
#[derive(Clone, Debug)]
pub struct CupertinoBasedMaterialThemeData {
    /// The Material theme data with colors based on an existing [`CupertinoThemeData`].
    pub material_theme: Rc<ThemeData>,
}

impl CupertinoBasedMaterialThemeData {
    /// Creates a Material theme with a color scheme based off of the colors from
    /// a [`CupertinoThemeData`].
    pub fn new(app: &App, theme_data: &CupertinoThemeData) -> CupertinoBasedMaterialThemeData {
        CupertinoBasedMaterialThemeData {
            material_theme: Rc::new(
                ThemeData::builder()
                    .color_scheme(
                        ColorScheme::from_seed(theme_data.primary_color().color())
                            .brightness(theme_data.brightness().unwrap_or(Brightness::Light))
                            .build()
                            .with_primary(theme_data.primary_color().color())
                            .with_on_primary(theme_data.primary_contrasting_color().color()),
                    )
                    .build(app),
            ),
        }
    }
}

/// Defines the visual density of user interface components.
///
/// Density, in the context of a UI, is the vertical and horizontal
/// "compactness" of the components in the UI. It is unitless, since it means
/// different things to different UI components.
///
/// The default for visual densities is zero for both vertical and horizontal
/// densities, which corresponds to the default visual density of components in
/// the Material Design specification. It does not affect text sizes, icon
/// sizes, or padding values.
///
/// The default visual density varies by platform: mobile platforms (Android, iOS,
/// Fuchsia) use [`VisualDensity::STANDARD`], while desktop platforms (macOS, Windows,
/// Linux) use [`VisualDensity::COMPACT`]. See
/// [`default_density_for_platform`](Self::default_density_for_platform) for more details.
///
/// For example, for buttons, it affects the spacing around the child of the
/// button. For lists, it affects the distance between baselines of entries in
/// the list. For chips, it only affects the vertical size, not the horizontal
/// size.
///
/// Here are some examples of widgets that respond to density changes:
///
///  * `Checkbox`
///  * `Chip`
///  * `ElevatedButton`
///  * `FilledButton`
///  * `IconButton`
///  * `InputDecorator` (which gives density support to `TextField`, etc.)
///  * `ListTile`
///  * `MaterialButton`
///  * `OutlinedButton`
///  * `Radio`
///  * `RawMaterialButton`
///  * `TextButton`
///
/// See also:
///
///  * [`ThemeData::visual_density`], where this property is used to specify the base
///    horizontal density of Material components.
///  * [Material design guidance on density](https://material.io/design/layout/applying-density.html).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisualDensity {
    /// The horizontal visual density of UI components.
    ///
    /// This property affects only the horizontal spacing between and within
    /// components, to allow for different UI visual densities. It does not affect
    /// text sizes, icon sizes, or padding values. The default value is 0.0,
    /// corresponding to the metrics specified in the Material Design
    /// specification. The value can range from [`MINIMUM_DENSITY`](Self::MINIMUM_DENSITY)
    /// to [`MAXIMUM_DENSITY`](Self::MAXIMUM_DENSITY), inclusive.
    ///
    /// See also:
    ///
    ///  * [`ThemeData::visual_density`], where this property is used to specify the base
    ///    horizontal density of Material components.
    ///  * [Material design guidance on density](https://material.io/design/layout/applying-density.html).
    pub horizontal: f64,
    /// The vertical visual density of UI components.
    ///
    /// This property affects only the vertical spacing between and within
    /// components, to allow for different UI visual densities. It does not affect
    /// text sizes, icon sizes, or padding values. The default value is 0.0,
    /// corresponding to the metrics specified in the Material Design
    /// specification. The value can range from [`MINIMUM_DENSITY`](Self::MINIMUM_DENSITY)
    /// to [`MAXIMUM_DENSITY`](Self::MAXIMUM_DENSITY), inclusive.
    ///
    /// See also:
    ///
    ///  * [`ThemeData::visual_density`], where this property is used to specify the base
    ///    vertical density of Material components.
    ///  * [Material design guidance on density](https://material.io/design/layout/applying-density.html).
    pub vertical: f64,
}

impl VisualDensity {
    /// A const constructor for [`VisualDensity`]; Dart's two named arguments default to 0.0.
    ///
    /// The `horizontal` and `vertical` arguments must be in the interval between
    /// [`MINIMUM_DENSITY`](Self::MINIMUM_DENSITY) and [`MAXIMUM_DENSITY`](Self::MAXIMUM_DENSITY),
    /// inclusive.
    pub const fn new(horizontal: f64, vertical: f64) -> VisualDensity {
        debug_assert!(vertical <= VisualDensity::MAXIMUM_DENSITY);
        debug_assert!(vertical >= VisualDensity::MINIMUM_DENSITY);
        debug_assert!(horizontal <= VisualDensity::MAXIMUM_DENSITY);
        debug_assert!(horizontal >= VisualDensity::MINIMUM_DENSITY);
        VisualDensity {
            horizontal,
            vertical,
        }
    }

    /// The minimum allowed density.
    pub const MINIMUM_DENSITY: f64 = -4.0;

    /// The maximum allowed density.
    pub const MAXIMUM_DENSITY: f64 = 4.0;

    /// The default profile for [`VisualDensity`] in [`ThemeData`].
    ///
    /// This default value represents a visual density that is less dense than
    /// either [`COMFORTABLE`](Self::COMFORTABLE) or [`COMPACT`](Self::COMPACT), and
    /// corresponds to density values of zero in both axes.
    pub const STANDARD: VisualDensity = VisualDensity::new(0.0, 0.0);

    /// The profile for a "comfortable" interpretation of [`VisualDensity`].
    ///
    /// Individual components will interpret the density value independently,
    /// making themselves more visually dense than [`STANDARD`](Self::STANDARD) and less
    /// dense than [`COMPACT`](Self::COMPACT) to different degrees based on the Material
    /// Design specification of the "comfortable" setting for their particular use case.
    ///
    /// It corresponds to a density value of -1 in both axes.
    pub const COMFORTABLE: VisualDensity = VisualDensity::new(-1.0, -1.0);

    /// The profile for a "compact" interpretation of [`VisualDensity`].
    ///
    /// Individual components will interpret the density value independently,
    /// making themselves more visually dense than [`STANDARD`](Self::STANDARD) and
    /// [`COMFORTABLE`](Self::COMFORTABLE) to different degrees based on the Material
    /// Design specification of the "comfortable" setting for their particular use case.
    ///
    /// It corresponds to a density value of -2 in both axes.
    pub const COMPACT: VisualDensity = VisualDensity::new(-2.0, -2.0);

    /// Returns a [`VisualDensity`] that is adaptive based on the current platform
    /// on which the framework is executing, from `defaultTargetPlatform`.
    ///
    /// When `defaultTargetPlatform` is a desktop platform, this returns
    /// [`COMPACT`](Self::COMPACT), and for other platforms, it returns a
    /// default-constructed [`VisualDensity`].
    ///
    /// See also:
    ///
    /// * [`default_density_for_platform`](Self::default_density_for_platform) which returns
    ///   a [`VisualDensity`] that is adaptive based on the platform given to it.
    pub fn adaptive_platform_density(app: &App) -> VisualDensity {
        VisualDensity::default_density_for_platform(app.platform().target_platform())
    }

    /// Returns a [`VisualDensity`] that is adaptive based on the given `platform`.
    ///
    /// For mobile platforms (Android, iOS, Fuchsia), this returns
    /// [`VisualDensity::STANDARD`], and for desktop platforms (macOS, Windows, Linux), it
    /// returns [`VisualDensity::COMPACT`].
    ///
    /// See also:
    ///
    /// * [`adaptive_platform_density`](Self::adaptive_platform_density) which returns a
    ///   [`VisualDensity`] that is adaptive based on `defaultTargetPlatform`.
    pub fn default_density_for_platform(platform: TargetPlatform) -> VisualDensity {
        match platform {
            TargetPlatform::Android | TargetPlatform::IOS | TargetPlatform::Fuchsia => {
                VisualDensity::STANDARD
            }
            TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                VisualDensity::COMPACT
            }
        }
    }

    /// Copy the current [`VisualDensity`] with the given values replacing the
    /// current values.
    pub fn copy_with(self, horizontal: Option<f64>, vertical: Option<f64>) -> VisualDensity {
        VisualDensity::new(
            horizontal.unwrap_or(self.horizontal),
            vertical.unwrap_or(self.vertical),
        )
    }

    /// The base adjustment in logical pixels of the visual density of UI components.
    ///
    /// The input density values are multiplied by a constant to arrive at a base
    /// size adjustment that fits Material Design guidelines.
    ///
    /// Individual components may adjust this value based upon their own
    /// individual interpretation of density.
    pub fn base_size_adjustment(&self) -> Offset {
        // The number of logical pixels represented by an increase or decrease in
        // density by one. The Material Design guidelines say to increment/decrement
        // sizes in terms of four pixel increments.
        const INTERVAL: f64 = 4.0;

        Offset::new(self.horizontal, self.vertical) * INTERVAL
    }

    /// Linearly interpolate between two densities.
    pub fn lerp(a: VisualDensity, b: VisualDensity, t: f64) -> VisualDensity {
        VisualDensity::new(
            lerp_double(Some(a.horizontal), Some(b.horizontal), t).expect("both sides"),
            lerp_double(Some(a.vertical), Some(b.vertical), t).expect("both sides"),
        )
    }

    /// Return a copy of `constraints` whose minimum width and height have been
    /// updated with the [`base_size_adjustment`](Self::base_size_adjustment).
    ///
    /// The resulting min_width and min_height values are clamped to not exceed the
    /// max_width and max_height values, respectively.
    pub fn effective_constraints(&self, constraints: BoxConstraints) -> BoxConstraints {
        debug_assert!(constraints.debug_assert_is_valid(false));
        constraints
            .copy_with()
            .min_width(clamp_double(
                constraints.min_width + self.base_size_adjustment().dx(),
                0.0,
                constraints.max_width,
            ))
            .min_height(clamp_double(
                constraints.min_height + self.base_size_adjustment().dy(),
                0.0,
                constraints.max_height,
            ))
    }
}

impl Default for VisualDensity {
    fn default() -> VisualDensity {
        VisualDensity::STANDARD
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::cell::Ref;

    use waft_painting::TextStyle;

    use super::*;
    use crate::bottom_navigation_bar::BottomNavigationBarType;
    use crate::elevated_button::ElevatedButton;
    use crate::filled_button::FilledButton;
    use crate::icon_button::IconButton;
    use crate::input_border::{InputBorder, OutlineInputBorder};
    use crate::material_state::MaterialStatePropertyAll;
    use crate::menu_anchor::MenuItemButton;
    use crate::menu_style::MenuStyle;
    use crate::outlined_button::OutlinedButton;
    use crate::test_support::{test_cell, test_cell_on, with_main_thread_stack};
    use crate::text_button::TextButton;
    use waft_cupertino::CupertinoPageTransitionsBuilder;
    use waft_widgets::PageTransitionsBuilderRef;

    const ALL_PLATFORMS: [TargetPlatform; 6] = [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::IOS,
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];

    /// Runs `body` with an app whose `defaultTargetPlatform` is `platform`.
    fn on<T>(platform: TargetPlatform, body: impl FnOnce(&App) -> T + Send) {
        with_main_thread_stack(move || {
            let cell = test_cell_on(platform);
            let app: Ref<'_, App> = cell.borrow();
            body(&app);
        });
    }

    /// Runs `body` with an app on Android, the platform Flutter's tests run on.
    fn android<T>(body: impl FnOnce(&App) -> T + Send) {
        with_main_thread_stack(move || {
            let cell = test_cell();
            let app: Ref<'_, App> = cell.borrow();
            body(&app);
        });
    }

    fn title_large_color(theme: &TextTheme) -> Option<AnyColor> {
        theme
            .title_large
            .as_ref()
            .expect("a title style")
            .color
            .clone()
    }

    #[derive(Debug)]
    struct MyThemeExtensionA {
        color1: Option<Color>,
        color2: Option<Color>,
    }

    impl ThemeExtension for MyThemeExtensionA {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn copy_with(&self) -> ThemeExtensionRef {
            Rc::new(MyThemeExtensionA {
                color1: self.color1,
                color2: self.color2,
            })
        }

        fn lerp(self: Rc<Self>, other: Option<&dyn ThemeExtension>, t: f64) -> ThemeExtensionRef {
            let Some(other) = other.and_then(|other| other.as_any().downcast_ref::<Self>()) else {
                return self;
            };
            Rc::new(MyThemeExtensionA {
                color1: Color::lerp(self.color1, other.color1, t),
                color2: Color::lerp(self.color2, other.color2, t),
            })
        }
    }

    #[derive(Debug)]
    struct MyThemeExtensionB {
        text_style: Option<TextStyle>,
    }

    impl ThemeExtension for MyThemeExtensionB {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn copy_with(&self) -> ThemeExtensionRef {
            Rc::new(MyThemeExtensionB {
                text_style: self.text_style.clone(),
            })
        }

        fn lerp(self: Rc<Self>, other: Option<&dyn ThemeExtension>, t: f64) -> ThemeExtensionRef {
            let Some(other) = other.and_then(|other| other.as_any().downcast_ref::<Self>()) else {
                return self;
            };
            Rc::new(MyThemeExtensionB {
                text_style: TextStyle::lerp(self.text_style.as_ref(), other.text_style.as_ref(), t),
            })
        }
    }

    #[derive(Debug)]
    struct StringAdaptation;

    impl Adaptation<String> for StringAdaptation {
        fn adapt(&self, _theme: &ThemeData, _default_value: String) -> String {
            "Adaptive theme.".to_string()
        }
    }

    #[test]
    fn theme_data_control_test() {
        android(|app| {
            let dark = ThemeData::dark(app, None);
            assert_eq!(dark, dark.copy_with().build());

            let light = ThemeData::new(app);
            let dawn = ThemeData::lerp(&dark, &light, 0.25);

            assert_eq!(dawn.brightness(), Brightness::Dark);
            assert_eq!(
                dawn.primary_color,
                Color::lerp(Some(dark.primary_color), Some(light.primary_color), 0.25)
                    .expect("both sides")
            );
        });
    }

    #[test]
    fn defaults_to_the_default_typography_for_the_platform() {
        android(|app| {
            for platform in ALL_PLATFORMS {
                let theme = ThemeData::builder()
                    .platform(platform)
                    .use_material3(false)
                    .build(app);
                let typography = Typography::material2018().platform(Some(platform)).build();
                assert_eq!(
                    theme.text_theme,
                    typography
                        .black
                        .apply()
                        .decoration(waft_embedder::TextDecoration::NONE)
                        .into_text_theme(),
                    "Not using default typography for {platform:?}"
                );
            }
        });
    }

    #[test]
    fn default_text_and_icon_themes_contrast_with_brightness() {
        android(|app| {
            let light_theme = ThemeData::builder()
                .brightness(Brightness::Light)
                .use_material3(false)
                .build(app);
            let dark_theme = ThemeData::builder()
                .brightness(Brightness::Dark)
                .use_material3(false)
                .build(app);
            let typography = Typography::material2018()
                .platform(Some(light_theme.platform))
                .build();

            assert_eq!(
                title_large_color(&light_theme.text_theme),
                title_large_color(&typography.black)
            );
            assert_eq!(
                title_large_color(&dark_theme.text_theme),
                title_large_color(&typography.white)
            );
        });
    }

    #[test]
    fn default_primary_text_and_icon_themes_contrast_with_primary_brightness() {
        android(|app| {
            let light_theme = ThemeData::builder()
                .primary_color(Colors::WHITE)
                .use_material3(false)
                .build(app);
            let dark_theme = ThemeData::builder()
                .primary_color(Colors::BLACK)
                .use_material3(false)
                .build(app);
            let typography = Typography::material2018()
                .platform(Some(light_theme.platform))
                .build();

            assert_eq!(
                title_large_color(&light_theme.primary_text_theme),
                title_large_color(&typography.black)
            );
            assert_eq!(
                title_large_color(&dark_theme.primary_text_theme),
                title_large_color(&typography.white)
            );
        });
    }

    #[test]
    fn light_dark_and_fallback_constructors_support_use_material3() {
        android(|app| {
            let light_theme = ThemeData::new(app);
            assert!(light_theme.use_material3);
            assert_eq!(
                light_theme.typography,
                Typography::material2021()
                    .color_scheme(light_theme.color_scheme.clone())
                    .build()
            );

            let dark_theme = ThemeData::dark(app, None);
            assert!(dark_theme.use_material3);
            assert_eq!(
                dark_theme.typography,
                Typography::material2021()
                    .color_scheme(dark_theme.color_scheme.clone())
                    .build()
            );

            let fallback_theme = ThemeData::fallback(app, None);
            assert!(fallback_theme.use_material3);
            assert_eq!(
                fallback_theme.typography,
                Typography::material2021()
                    .color_scheme(fallback_theme.color_scheme.clone())
                    .build()
            );
        });
    }

    #[test]
    fn theme_data_supports_material_3_contrast_levels() {
        android(|app| {
            let theme = |brightness, contrast_level| {
                ThemeData::builder()
                    .brightness(brightness)
                    .contrast_level(contrast_level)
                    .build(app)
                    .color_scheme
            };
            let standard = ThemeData::new(app).color_scheme;
            let medium = theme(Brightness::Light, ContrastLevel::MEDIUM);
            let high = theme(Brightness::Light, ContrastLevel::HIGH);
            let dark_medium = theme(Brightness::Dark, ContrastLevel::MEDIUM);
            let dark_high = theme(Brightness::Dark, ContrastLevel::HIGH);

            assert_ne!(standard.primary, medium.primary);
            assert_eq!(medium.primary, Color::new(0xFF4F378B));
            assert_eq!(medium.on_surface, Color::new(0xFF000000));
            assert_eq!(high.primary, Color::new(0xFF381E72));
            assert_eq!(high.on_surface_variant(), Color::new(0xFF000000));
            assert_eq!(dark_medium.primary, Color::new(0xFFEADDFF));
            assert_eq!(dark_medium.on_surface, Color::new(0xFFFFFFFF));
            assert_eq!(dark_high.primary, Color::new(0xFFF6EDFF));
            assert_eq!(dark_high.on_surface_variant(), Color::new(0xFFFFFFFF));
        });
    }

    #[test]
    fn theme_data_supports_custom_material_3_contrast_levels_from_seed_colors() {
        android(|app| {
            let contrast_level = 0.25;
            let theme = ThemeData::builder()
                .color_scheme_seed(Colors::BLUE.primary())
                .contrast_level(contrast_level)
                .build(app);

            assert_eq!(
                theme.color_scheme,
                ColorScheme::from_seed(Colors::BLUE.primary())
                    .contrast_level(contrast_level)
                    .build()
            );
        });
    }

    #[test]
    #[should_panic(expected = "colorSchemeSeed must be provided to use a custom contrastLevel")]
    fn theme_data_requires_a_seed_color_for_custom_contrast_levels() {
        android(|app| ThemeData::builder().contrast_level(0.25).build(app));
    }

    #[test]
    #[should_panic(expected = "colorSchemeSeed must be provided to use a custom contrastLevel")]
    fn theme_data_requires_a_seed_color_for_custom_contrast_levels_with_a_color_scheme() {
        android(|app| {
            ThemeData::builder()
                .color_scheme(ColorScheme::light())
                .contrast_level(0.25)
                .build(app)
        });
    }

    #[test]
    #[should_panic(expected = "contrastLevel must be between -1.0 and 1.0 inclusive.")]
    fn theme_data_throws_for_contrast_levels_below_the_valid_range() {
        android(|app| ThemeData::builder().contrast_level(-1.5).build(app));
    }

    #[test]
    #[should_panic(expected = "contrastLevel must be between -1.0 and 1.0 inclusive.")]
    fn theme_data_throws_for_contrast_levels_above_the_valid_range() {
        android(|app| ThemeData::builder().contrast_level(1.5).build(app));
    }

    #[test]
    fn theme_data_allows_precision_errors_at_contrast_level_limits() {
        android(|app| {
            let precision_error = PRECISION_ERROR_TOLERANCE / 2.0;
            for contrast_level in [-1.0 - precision_error, 1.0 + precision_error] {
                ThemeData::builder()
                    .color_scheme_seed(Colors::BLUE.primary())
                    .contrast_level(contrast_level)
                    .build(app);
            }
        });
    }

    #[test]
    fn defaults_to_padded_tap_targets_on_mobile_platforms_and_shrink_wrap_on_desktop() {
        for platform in ALL_PLATFORMS {
            on(platform, |app| {
                let theme_data = ThemeData::builder()
                    .platform(app.platform().target_platform())
                    .build(app);
                let expected = match platform {
                    TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => {
                        MaterialTapTargetSize::Padded
                    }
                    TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                        MaterialTapTargetSize::ShrinkWrap
                    }
                };
                assert_eq!(
                    theme_data.material_tap_target_size, expected,
                    "{platform:?}"
                );
            });
        }
    }

    #[test]
    fn can_control_font_family_default() {
        android(|app| {
            let theme_data = ThemeData::builder()
                .font_family("FlutterTest")
                .text_theme(TextTheme::new().title_large(TextStyle::new().font_family("Roboto")))
                .build(app);

            let family = |style: &Option<TextStyle>| {
                style.as_ref().and_then(|style| style.font_family.clone())
            };
            assert_eq!(
                family(&theme_data.text_theme.body_large).as_deref(),
                Some("FlutterTest")
            );
            assert_eq!(
                family(&theme_data.primary_text_theme.display_small).as_deref(),
                Some("FlutterTest")
            );

            // Shouldn't override the specified style's family
            assert_eq!(
                family(&theme_data.text_theme.title_large).as_deref(),
                Some("Roboto")
            );
        });
    }

    #[test]
    fn can_estimate_brightness_directly() {
        let estimate = ThemeData::estimate_brightness_for_color;
        assert_eq!(estimate(Colors::WHITE), Brightness::Light);
        assert_eq!(estimate(Colors::BLACK), Brightness::Dark);
        assert_eq!(estimate(Colors::BLUE.primary()), Brightness::Dark);
        assert_eq!(estimate(Colors::YELLOW.primary()), Brightness::Light);
        assert_eq!(estimate(Colors::DEEP_ORANGE.primary()), Brightness::Dark);
        assert_eq!(estimate(Colors::ORANGE.primary()), Brightness::Light);
        assert_eq!(estimate(Colors::LIME.primary()), Brightness::Light);
        assert_eq!(estimate(Colors::GREY.primary()), Brightness::Light);
        assert_eq!(estimate(Colors::TEAL.primary()), Brightness::Dark);
        assert_eq!(estimate(Colors::INDIGO.primary()), Brightness::Dark);
    }

    #[test]
    #[should_panic]
    fn a_color_scheme_seed_rules_out_a_color_scheme() {
        android(|app| {
            ThemeData::builder()
                .color_scheme_seed(Colors::BLUE.primary())
                .color_scheme(ColorScheme::light())
                .build(app)
        });
    }

    #[test]
    #[should_panic]
    fn a_color_scheme_seed_rules_out_a_primary_color() {
        android(|app| {
            ThemeData::builder()
                .color_scheme_seed(Colors::BLUE.primary())
                .primary_color(Colors::GREEN.primary())
                .build(app)
        });
    }

    #[test]
    #[should_panic]
    fn a_color_scheme_seed_rules_out_a_primary_swatch() {
        android(|app| {
            ThemeData::builder()
                .color_scheme_seed(Colors::BLUE.primary())
                .primary_swatch(Colors::GREEN)
                .build(app)
        });
    }

    #[test]
    fn theme_data_can_generate_a_light_color_scheme_from_color_scheme_seed() {
        android(|app| {
            let theme = ThemeData::builder()
                .color_scheme_seed(Colors::BLUE.primary())
                .build(app);
            let scheme = &theme.color_scheme;
            let expected = [
                (scheme.primary, Color::new(0xFF36618E)),
                (scheme.on_primary, Color::new(0xFFFFFFFF)),
                (scheme.primary_container(), Color::new(0xFFD1E4FF)),
                (scheme.on_primary_container(), Color::new(0xFF194975)),
                (scheme.primary_fixed(), Color::new(0xFFD1E4FF)),
                (scheme.primary_fixed_dim(), Color::new(0xFFA0CAFD)),
                (scheme.on_primary_fixed(), Color::new(0xFF001D36)),
                (scheme.on_primary_fixed_variant(), Color::new(0xFF194975)),
                (scheme.secondary, Color::new(0xFF535F70)),
                (scheme.on_secondary, Color::new(0xFFFFFFFF)),
                (scheme.secondary_container(), Color::new(0xFFD7E3F7)),
                (scheme.on_secondary_container(), Color::new(0xFF3B4858)),
                (scheme.secondary_fixed(), Color::new(0xFFD7E3F7)),
                (scheme.secondary_fixed_dim(), Color::new(0xFFBBC7DB)),
                (scheme.on_secondary_fixed(), Color::new(0xFF101C2B)),
                (scheme.on_secondary_fixed_variant(), Color::new(0xFF3B4858)),
                (scheme.tertiary(), Color::new(0xFF6B5778)),
                (scheme.on_tertiary(), Color::new(0xFFFFFFFF)),
                (scheme.tertiary_container(), Color::new(0xFFF2DAFF)),
                (scheme.on_tertiary_container(), Color::new(0xFF523F5F)),
                (scheme.tertiary_fixed(), Color::new(0xFFF2DAFF)),
                (scheme.tertiary_fixed_dim(), Color::new(0xFFD6BEE4)),
                (scheme.on_tertiary_fixed(), Color::new(0xFF251431)),
                (scheme.on_tertiary_fixed_variant(), Color::new(0xFF523F5F)),
                (scheme.error, Color::new(0xFFBA1A1A)),
                (scheme.on_error, Color::new(0xFFFFFFFF)),
                (scheme.error_container(), Color::new(0xFFFFDAD6)),
                (scheme.on_error_container(), Color::new(0xFF93000A)),
                (scheme.outline(), Color::new(0xFF73777F)),
                (scheme.outline_variant(), Color::new(0xFFC3C7CF)),
                (scheme.background(), Color::new(0xFFF8F9FF)),
                (scheme.on_background(), Color::new(0xFF191C20)),
                (scheme.surface, Color::new(0xFFF8F9FF)),
                (scheme.surface_bright(), Color::new(0xFFF8F9FF)),
                (scheme.surface_dim(), Color::new(0xFFD8DAE0)),
                (scheme.surface_container_lowest(), Color::new(0xFFFFFFFF)),
                (scheme.surface_container_low(), Color::new(0xFFF2F3FA)),
                (scheme.surface_container(), Color::new(0xFFECEEF4)),
                (scheme.surface_container_high(), Color::new(0xFFE6E8EE)),
                (scheme.surface_container_highest(), Color::new(0xFFE1E2E8)),
                (scheme.on_surface, Color::new(0xFF191C20)),
                (scheme.surface_variant(), Color::new(0xFFDFE2EB)),
                (scheme.on_surface_variant(), Color::new(0xFF43474E)),
                (scheme.inverse_surface(), Color::new(0xFF2E3135)),
                (scheme.on_inverse_surface(), Color::new(0xFFEFF0F7)),
                (scheme.inverse_primary(), Color::new(0xFFA0CAFD)),
                (scheme.shadow(), Color::new(0xFF000000)),
                (scheme.scrim(), Color::new(0xFF000000)),
                (scheme.surface_tint(), Color::new(0xFF36618E)),
            ];
            for (index, (actual, expected)) in expected.into_iter().enumerate() {
                assert_eq!(actual, expected, "colour {index}");
            }
            assert_eq!(scheme.brightness, Brightness::Light);

            assert_eq!(theme.primary_color, scheme.primary);
            assert_eq!(theme.canvas_color, scheme.surface);
            assert_eq!(theme.scaffold_background_color, scheme.surface);
            assert_eq!(theme.card_color, scheme.surface);
            assert_eq!(theme.divider_color, scheme.outline());
            assert_eq!(theme.dialog_background_color, scheme.surface);
            assert_eq!(theme.indicator_color, scheme.on_primary);
            assert!(!theme.apply_elevation_overlay_color);
        });
    }

    #[test]
    fn theme_data_can_generate_a_dark_color_scheme_from_color_scheme_seed() {
        android(|app| {
            let theme = ThemeData::builder()
                .color_scheme_seed(Colors::BLUE.primary())
                .brightness(Brightness::Dark)
                .build(app);
            let scheme = &theme.color_scheme;
            let expected = [
                (scheme.primary, Color::new(0xFFA0CAFD)),
                (scheme.on_primary, Color::new(0xFF003258)),
                (scheme.primary_container(), Color::new(0xFF194975)),
                (scheme.on_primary_container(), Color::new(0xFFD1E4FF)),
                (scheme.primary_fixed(), Color::new(0xFFD1E4FF)),
                (scheme.primary_fixed_dim(), Color::new(0xFFA0CAFD)),
                (scheme.on_primary_fixed(), Color::new(0xFF001D36)),
                (scheme.on_primary_fixed_variant(), Color::new(0xFF194975)),
                (scheme.secondary, Color::new(0xFFBBC7DB)),
                (scheme.on_secondary, Color::new(0xFF253140)),
                (scheme.secondary_container(), Color::new(0xFF3B4858)),
                (scheme.on_secondary_container(), Color::new(0xFFD7E3F7)),
                (scheme.secondary_fixed(), Color::new(0xFFD7E3F7)),
                (scheme.secondary_fixed_dim(), Color::new(0xFFBBC7DB)),
                (scheme.on_secondary_fixed(), Color::new(0xFF101C2B)),
                (scheme.on_secondary_fixed_variant(), Color::new(0xFF3B4858)),
                (scheme.tertiary(), Color::new(0xFFD6BEE4)),
                (scheme.on_tertiary(), Color::new(0xFF3B2948)),
                (scheme.tertiary_container(), Color::new(0xFF523F5F)),
                (scheme.on_tertiary_container(), Color::new(0xFFF2DAFF)),
                (scheme.tertiary_fixed(), Color::new(0xFFF2DAFF)),
                (scheme.tertiary_fixed_dim(), Color::new(0xFFD6BEE4)),
                (scheme.on_tertiary_fixed(), Color::new(0xFF251431)),
                (scheme.on_tertiary_fixed_variant(), Color::new(0xFF523F5F)),
                (scheme.error, Color::new(0xFFFFB4AB)),
                (scheme.on_error, Color::new(0xFF690005)),
                (scheme.error_container(), Color::new(0xFF93000A)),
                (scheme.on_error_container(), Color::new(0xFFFFDAD6)),
                (scheme.outline(), Color::new(0xFF8D9199)),
                (scheme.outline_variant(), Color::new(0xFF43474E)),
                (scheme.background(), Color::new(0xFF111418)),
                (scheme.on_background(), Color::new(0xFFE1E2E8)),
                (scheme.surface, Color::new(0xFF111418)),
                (scheme.surface_dim(), Color::new(0xFF111418)),
                (scheme.surface_bright(), Color::new(0xFF36393E)),
                (scheme.surface_container_lowest(), Color::new(0xFF0B0E13)),
                (scheme.surface_container_low(), Color::new(0xFF191C20)),
                (scheme.surface_container(), Color::new(0xFF1D2024)),
                (scheme.surface_container_high(), Color::new(0xFF272A2F)),
                (scheme.surface_container_highest(), Color::new(0xFF32353A)),
                (scheme.on_surface, Color::new(0xFFE1E2E8)),
                (scheme.surface_variant(), Color::new(0xFF43474E)),
                (scheme.on_surface_variant(), Color::new(0xFFC3C7CF)),
                (scheme.inverse_surface(), Color::new(0xFFE1E2E8)),
                (scheme.on_inverse_surface(), Color::new(0xFF2E3135)),
                (scheme.inverse_primary(), Color::new(0xFF36618E)),
                (scheme.shadow(), Color::new(0xFF000000)),
                (scheme.scrim(), Color::new(0xFF000000)),
                (scheme.surface_tint(), Color::new(0xFFA0CAFD)),
            ];
            for (index, (actual, expected)) in expected.into_iter().enumerate() {
                assert_eq!(actual, expected, "colour {index}");
            }
            assert_eq!(scheme.brightness, Brightness::Dark);

            assert_eq!(theme.primary_color, scheme.surface);
            assert_eq!(theme.canvas_color, scheme.surface);
            assert_eq!(theme.scaffold_background_color, scheme.surface);
            assert_eq!(theme.card_color, scheme.surface);
            assert_eq!(theme.divider_color, scheme.outline());
            assert_eq!(theme.dialog_background_color, scheme.surface);
            assert_eq!(theme.indicator_color, scheme.on_surface);
            assert!(theme.apply_elevation_overlay_color);
        });
    }

    #[test]
    fn theme_data_can_generate_a_default_m3_light_color_scheme_when_use_material3_is_true() {
        android(|app| {
            let constructors: [fn(&App) -> ThemeData; 2] =
                [ThemeData::new, |app| ThemeData::light(app, None)];
            for constructor in constructors {
                let theme = Box::new(constructor(app));
                let scheme = &theme.color_scheme;
                let expected = [
                    (scheme.primary, Color::new(0xFF6750A4)),
                    (scheme.on_primary, Color::new(0xFFFFFFFF)),
                    (scheme.primary_container(), Color::new(0xFFEADDFF)),
                    (scheme.on_primary_container(), Color::new(0xFF4F378B)),
                    (scheme.primary_fixed(), Color::new(0xFFEADDFF)),
                    (scheme.primary_fixed_dim(), Color::new(0xFFD0BCFF)),
                    (scheme.on_primary_fixed(), Color::new(0xFF21005D)),
                    (scheme.on_primary_fixed_variant(), Color::new(0xFF4F378B)),
                    (scheme.secondary, Color::new(0xFF625B71)),
                    (scheme.on_secondary, Color::new(0xFFFFFFFF)),
                    (scheme.secondary_container(), Color::new(0xFFE8DEF8)),
                    (scheme.on_secondary_container(), Color::new(0xFF4A4458)),
                    (scheme.secondary_fixed(), Color::new(0xFFE8DEF8)),
                    (scheme.secondary_fixed_dim(), Color::new(0xFFCCC2DC)),
                    (scheme.on_secondary_fixed(), Color::new(0xFF1D192B)),
                    (scheme.on_secondary_fixed_variant(), Color::new(0xFF4A4458)),
                    (scheme.tertiary(), Color::new(0xFF7D5260)),
                    (scheme.on_tertiary(), Color::new(0xFFFFFFFF)),
                    (scheme.tertiary_container(), Color::new(0xFFFFD8E4)),
                    (scheme.on_tertiary_container(), Color::new(0xFF633B48)),
                    (scheme.tertiary_fixed(), Color::new(0xFFFFD8E4)),
                    (scheme.tertiary_fixed_dim(), Color::new(0xFFEFB8C8)),
                    (scheme.on_tertiary_fixed(), Color::new(0xFF31111D)),
                    (scheme.on_tertiary_fixed_variant(), Color::new(0xFF633B48)),
                    (scheme.error, Color::new(0xFFB3261E)),
                    (scheme.on_error, Color::new(0xFFFFFFFF)),
                    (scheme.error_container(), Color::new(0xFFF9DEDC)),
                    (scheme.on_error_container(), Color::new(0xFF8C1D18)),
                    (scheme.outline(), Color::new(0xFF79747E)),
                    (scheme.background(), Color::new(0xFFFEF7FF)),
                    (scheme.on_background(), Color::new(0xFF1D1B20)),
                    (scheme.surface, Color::new(0xFFFEF7FF)),
                    (scheme.on_surface, Color::new(0xFF1D1B20)),
                    (scheme.surface_variant(), Color::new(0xFFE7E0EC)),
                    (scheme.on_surface_variant(), Color::new(0xFF49454F)),
                    (scheme.surface_bright(), Color::new(0xFFFEF7FF)),
                    (scheme.surface_dim(), Color::new(0xFFDED8E1)),
                    (scheme.surface_container(), Color::new(0xFFF3EDF7)),
                    (scheme.surface_container_highest(), Color::new(0xFFE6E0E9)),
                    (scheme.surface_container_high(), Color::new(0xFFECE6F0)),
                    (scheme.surface_container_lowest(), Color::new(0xFFFFFFFF)),
                    (scheme.surface_container_low(), Color::new(0xFFF7F2FA)),
                    (scheme.inverse_surface(), Color::new(0xFF322F35)),
                    (scheme.on_inverse_surface(), Color::new(0xFFF5EFF7)),
                    (scheme.inverse_primary(), Color::new(0xFFD0BCFF)),
                    (scheme.shadow(), Color::new(0xFF000000)),
                ];
                for (index, (actual, expected)) in expected.into_iter().enumerate() {
                    assert_eq!(actual, expected, "colour {index}");
                }
                assert_eq!(scheme.brightness, Brightness::Light);

                assert_eq!(theme.primary_color, scheme.primary);
                assert_eq!(theme.canvas_color, scheme.surface);
                assert_eq!(theme.scaffold_background_color, scheme.surface);
                assert_eq!(theme.card_color, scheme.surface);
                assert_eq!(theme.divider_color, scheme.outline());
                assert_eq!(theme.dialog_background_color, scheme.surface);
                assert_eq!(theme.indicator_color, scheme.on_primary);
                assert!(!theme.apply_elevation_overlay_color);
            }
        });
    }

    #[test]
    fn theme_data_dark_can_generate_a_default_m3_dark_color_scheme_when_use_material3_is_true() {
        android(|app| {
            let theme = ThemeData::dark(app, None);
            let scheme = &theme.color_scheme;
            let expected = [
                (scheme.primary, Color::new(0xFFD0BCFF)),
                (scheme.on_primary, Color::new(0xFF381E72)),
                (scheme.primary_container(), Color::new(0xFF4F378B)),
                (scheme.on_primary_container(), Color::new(0xFFEADDFF)),
                (scheme.primary_fixed(), Color::new(0xFFEADDFF)),
                (scheme.primary_fixed_dim(), Color::new(0xFFD0BCFF)),
                (scheme.on_primary_fixed(), Color::new(0xFF21005D)),
                (scheme.on_primary_fixed_variant(), Color::new(0xFF4F378B)),
                (scheme.secondary, Color::new(0xFFCCC2DC)),
                (scheme.on_secondary, Color::new(0xFF332D41)),
                (scheme.secondary_container(), Color::new(0xFF4A4458)),
                (scheme.on_secondary_container(), Color::new(0xFFE8DEF8)),
                (scheme.secondary_fixed(), Color::new(0xFFE8DEF8)),
                (scheme.secondary_fixed_dim(), Color::new(0xFFCCC2DC)),
                (scheme.on_secondary_fixed(), Color::new(0xFF1D192B)),
                (scheme.on_secondary_fixed_variant(), Color::new(0xFF4A4458)),
                (scheme.tertiary(), Color::new(0xFFEFB8C8)),
                (scheme.on_tertiary(), Color::new(0xFF492532)),
                (scheme.tertiary_container(), Color::new(0xFF633B48)),
                (scheme.on_tertiary_container(), Color::new(0xFFFFD8E4)),
                (scheme.tertiary_fixed(), Color::new(0xFFFFD8E4)),
                (scheme.tertiary_fixed_dim(), Color::new(0xFFEFB8C8)),
                (scheme.on_tertiary_fixed(), Color::new(0xFF31111D)),
                (scheme.on_tertiary_fixed_variant(), Color::new(0xFF633B48)),
                (scheme.error, Color::new(0xFFF2B8B5)),
                (scheme.on_error, Color::new(0xFF601410)),
                (scheme.error_container(), Color::new(0xFF8C1D18)),
                (scheme.on_error_container(), Color::new(0xFFF9DEDC)),
                (scheme.outline(), Color::new(0xFF938F99)),
                (scheme.background(), Color::new(0xFF141218)),
                (scheme.on_background(), Color::new(0xFFE6E0E9)),
                (scheme.surface, Color::new(0xFF141218)),
                (scheme.on_surface, Color::new(0xFFE6E0E9)),
                (scheme.surface_variant(), Color::new(0xFF49454F)),
                (scheme.on_surface_variant(), Color::new(0xFFCAC4D0)),
                (scheme.surface_bright(), Color::new(0xFF3B383E)),
                (scheme.surface_dim(), Color::new(0xFF141218)),
                (scheme.surface_container(), Color::new(0xFF211F26)),
                (scheme.surface_container_highest(), Color::new(0xFF36343B)),
                (scheme.surface_container_high(), Color::new(0xFF2B2930)),
                (scheme.surface_container_lowest(), Color::new(0xFF0F0D13)),
                (scheme.surface_container_low(), Color::new(0xFF1D1B20)),
                (scheme.inverse_surface(), Color::new(0xFFE6E0E9)),
                (scheme.on_inverse_surface(), Color::new(0xFF322F35)),
                (scheme.inverse_primary(), Color::new(0xFF6750A4)),
                (scheme.shadow(), Color::new(0xFF000000)),
            ];
            for (index, (actual, expected)) in expected.into_iter().enumerate() {
                assert_eq!(actual, expected, "colour {index}");
            }
            assert_eq!(scheme.brightness, Brightness::Dark);

            assert_eq!(theme.primary_color, scheme.surface);
            assert_eq!(theme.canvas_color, scheme.surface);
            assert_eq!(theme.scaffold_background_color, scheme.surface);
            assert_eq!(theme.card_color, scheme.surface);
            assert_eq!(theme.divider_color, scheme.outline());
            assert_eq!(theme.dialog_background_color, scheme.surface);
            assert_eq!(theme.indicator_color, scheme.on_surface);
            assert!(theme.apply_elevation_overlay_color);
        });
    }

    #[test]
    fn theme_data_from_a_light_color_scheme_sets_appropriate_values() {
        android(|app| {
            let light_colors = ColorScheme::light();
            let theme = ThemeData::from(light_colors.clone()).build(app);

            assert_eq!(theme.brightness(), Brightness::Light);
            assert_eq!(theme.primary_color, light_colors.primary);
            assert_eq!(theme.card_color, light_colors.surface);
            assert_eq!(theme.canvas_color, light_colors.surface);
            assert_eq!(theme.scaffold_background_color, light_colors.surface);
            assert_eq!(theme.dialog_background_color, light_colors.surface);
            assert!(!theme.apply_elevation_overlay_color);
        });
    }

    #[test]
    fn theme_data_from_a_dark_color_scheme_sets_appropriate_values() {
        android(|app| {
            let dark_colors = ColorScheme::dark();
            let theme = ThemeData::from(dark_colors.clone()).build(app);

            assert_eq!(theme.brightness(), Brightness::Dark);
            // in dark theme's the color used for main components is surface instead of primary
            assert_eq!(theme.primary_color, dark_colors.surface);
            assert_eq!(theme.card_color, dark_colors.surface);
            assert_eq!(theme.canvas_color, dark_colors.surface);
            assert_eq!(theme.scaffold_background_color, dark_colors.surface);
            assert_eq!(theme.dialog_background_color, dark_colors.surface);
            assert!(theme.apply_elevation_overlay_color);
        });
    }

    #[test]
    fn splash_factory_is_ink_sparkle_only_for_android_non_web_when_use_material3_is_true() {
        for platform in ALL_PLATFORMS {
            on(platform, |app| {
                let theme = ThemeData::new(app);

                // Basic check that this theme is in fact using material 3.
                assert!(theme.use_material3);

                let expected = match platform {
                    TargetPlatform::Android if K_IS_WEB => InkRipple::splash_factory(),
                    TargetPlatform::Android => InkSparkle::splash_factory(),
                    _ => InkRipple::splash_factory(),
                };
                assert!(Rc::ptr_eq(&theme.splash_factory, &expected), "{platform:?}");
            });
        }
    }

    #[test]
    fn splash_factory_is_ink_splash_for_every_platform_when_use_material3_is_false() {
        for platform in ALL_PLATFORMS {
            on(platform, |app| {
                let theme = ThemeData::builder().use_material3(false).build(app);
                assert!(
                    Rc::ptr_eq(&theme.splash_factory, &InkSplash::splash_factory()),
                    "{platform:?}"
                );
            });
        }
    }

    #[test]
    fn visual_density_adaptive_platform_density_returns_adaptive_values() {
        for platform in ALL_PLATFORMS {
            on(platform, |app| {
                let expected = match platform {
                    TargetPlatform::Android | TargetPlatform::IOS | TargetPlatform::Fuchsia => {
                        VisualDensity::STANDARD
                    }
                    TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                        VisualDensity::COMPACT
                    }
                };
                assert_eq!(VisualDensity::adaptive_platform_density(app), expected);
                assert_eq!(
                    VisualDensity::default_density_for_platform(platform),
                    expected
                );
            });
        }
    }

    #[test]
    fn visual_density_in_theme_data_defaults_to_compact_on_desktop_and_standard_on_mobile() {
        for platform in ALL_PLATFORMS {
            on(platform, |app| {
                let theme_data = ThemeData::new(app);
                let expected = match platform {
                    TargetPlatform::Android | TargetPlatform::IOS | TargetPlatform::Fuchsia => {
                        VisualDensity::STANDARD
                    }
                    TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => {
                        VisualDensity::COMPACT
                    }
                };
                assert_eq!(theme_data.visual_density, expected, "{platform:?}");
            });
        }
    }

    #[test]
    fn visual_density_in_theme_data_defaults_to_the_right_thing_when_a_platform_is_supplied_to_it()
    {
        for platform in ALL_PLATFORMS {
            on(platform, |app| {
                let theme_data = ThemeData::builder()
                    .platform(if platform == TargetPlatform::Android {
                        TargetPlatform::Linux
                    } else {
                        TargetPlatform::Android
                    })
                    .build(app);
                let expected = match platform {
                    TargetPlatform::Android => VisualDensity::COMPACT,
                    _ => VisualDensity::STANDARD,
                };
                assert_eq!(theme_data.visual_density, expected, "{platform:?}");
            });
        }
    }

    #[test]
    fn visual_density_effective_constraints_are_clamped() {
        let expand = VisualDensity::new(4.0, 4.0);
        let contract = VisualDensity::new(-4.0, -4.0);

        let square = BoxConstraints::tight_for(Some(35.0), Some(35.0));
        let expanded = expand.effective_constraints(square);
        assert_eq!(expanded.min_width, 35.0);
        assert_eq!(expanded.min_height, 35.0);
        assert_eq!(expanded.max_width, 35.0);
        assert_eq!(expanded.max_height, 35.0);

        let contracted = contract.effective_constraints(square);
        assert_eq!(contracted.min_width, 19.0);
        assert_eq!(contracted.min_height, 19.0);
        assert_eq!(expanded.max_width, 35.0);
        assert_eq!(expanded.max_height, 35.0);

        let small = BoxConstraints::tight_for(Some(4.0), Some(4.0));
        let expanded = expand.effective_constraints(small);
        assert_eq!(expanded.min_width, 4.0);
        assert_eq!(expanded.min_height, 4.0);
        assert_eq!(expanded.max_width, 4.0);
        assert_eq!(expanded.max_height, 4.0);

        let contracted = contract.effective_constraints(small);
        assert_eq!(contracted.min_width, 0.0);
        assert_eq!(contracted.min_height, 0.0);
        assert_eq!(expanded.max_width, 4.0);
        assert_eq!(expanded.max_height, 4.0);
    }

    #[test]
    fn visual_density_effective_constraints_expand_and_contract() {
        let square = BoxConstraints::new();
        let expanded = VisualDensity::new(4.0, 4.0).effective_constraints(square);
        assert_eq!(expanded.min_width, 16.0);
        assert_eq!(expanded.min_height, 16.0);
        assert_eq!(expanded.max_width, f64::INFINITY);
        assert_eq!(expanded.max_height, f64::INFINITY);

        let contracted = VisualDensity::new(-4.0, -4.0).effective_constraints(square);
        assert_eq!(contracted.min_width, 0.0);
        assert_eq!(contracted.min_height, 0.0);
        assert_eq!(expanded.max_width, f64::INFINITY);
        assert_eq!(expanded.max_height, f64::INFINITY);
    }

    #[test]
    fn theme_extensions_can_be_obtained() {
        android(|app| {
            let theme = ThemeData::builder()
                .extensions(vec![
                    Rc::new(MyThemeExtensionA {
                        color1: Some(Colors::BLACK),
                        color2: Some(Colors::AMBER.primary()),
                    }),
                    Rc::new(MyThemeExtensionB {
                        text_style: Some(TextStyle::new().font_size(50.0)),
                    }),
                ])
                .build(app);

            let a = theme.extension::<MyThemeExtensionA>().expect("extension A");
            assert_eq!(a.color1, Some(Colors::BLACK));
            assert_eq!(a.color2, Some(Colors::AMBER.primary()));
            let b = theme.extension::<MyThemeExtensionB>().expect("extension B");
            assert_eq!(b.text_style, Some(TextStyle::new().font_size(50.0)));
        });
    }

    #[test]
    fn theme_extensions_can_use_copy_with() {
        android(|app| {
            let extension = MyThemeExtensionA {
                color1: Some(Colors::BLACK),
                color2: Some(Colors::AMBER.primary()),
            };
            let copied = MyThemeExtensionA {
                color1: Some(Colors::BLUE.primary()),
                ..extension
            };
            let theme = ThemeData::builder()
                .extensions(vec![copied.copy_with()])
                .build(app);

            let a = theme.extension::<MyThemeExtensionA>().expect("extension A");
            assert_eq!(a.color1, Some(Colors::BLUE.primary()));
            assert_eq!(a.color2, Some(Colors::AMBER.primary()));
        });
    }

    /// Flutter's `isSameColorAs`: equal to within a channel step, since `Color::lerp` works in
    /// floating point.
    fn assert_same_color(actual: Option<Color>, expected: Color) {
        let actual = actual.expect("a color");
        let step = 1.0 / 255.0;
        assert!(
            (actual.a - expected.a).abs() < step
                && (actual.r - expected.r).abs() < step
                && (actual.g - expected.g).abs() < step
                && (actual.b - expected.b).abs() < step,
            "{actual:?} is not {expected:?}"
        );
    }

    #[test]
    fn theme_extensions_can_lerp() {
        android(|app| {
            let extension_a1: ThemeExtensionRef = Rc::new(MyThemeExtensionA {
                color1: Some(Colors::BLACK),
                color2: Some(Colors::AMBER.primary()),
            });
            let extension_a2: ThemeExtensionRef = Rc::new(MyThemeExtensionA {
                color1: Some(Colors::WHITE),
                color2: Some(Colors::BLUE.primary()),
            });
            let extension_b1: ThemeExtensionRef = Rc::new(MyThemeExtensionB {
                text_style: Some(TextStyle::new().font_size(50.0)),
            });
            let extension_b2: ThemeExtensionRef = Rc::new(MyThemeExtensionB {
                text_style: Some(TextStyle::new().font_size(100.0)),
            });
            let themed = |extensions: &[&ThemeExtensionRef]| {
                ThemeData::builder()
                    .extensions(
                        extensions
                            .iter()
                            .map(|&extension| Rc::clone(extension))
                            .collect(),
                    )
                    .build(app)
            };
            let a = |theme: &ThemeData| {
                let a = theme.extension::<MyThemeExtensionA>().expect("extension A");
                (a.color1, a.color2)
            };
            let font_size = |theme: &ThemeData| {
                theme
                    .extension::<MyThemeExtensionB>()
                    .expect("extension B")
                    .text_style
                    .clone()
            };

            // Both ThemeData arguments include both extensions.
            let lerped = ThemeData::lerp(
                &themed(&[&extension_a1, &extension_b1]),
                &themed(&[&extension_a2, &extension_b2]),
                0.5,
            );
            let (color1, color2) = a(&lerped);
            assert_same_color(color1, Color::new(0xff7f7f7f));
            assert_same_color(color2, Color::new(0xff90ab7d));
            assert_eq!(font_size(&lerped), Some(TextStyle::new().font_size(75.0)));

            // Missing from 2nd ThemeData
            let lerped = ThemeData::lerp(
                &themed(&[&extension_a1, &extension_b1]),
                &themed(&[&extension_b2]),
                0.5,
            );
            // Not lerped
            let (color1, color2) = a(&lerped);
            assert_same_color(color1, Colors::BLACK);
            assert_same_color(color2, Colors::AMBER.primary());
            assert_eq!(font_size(&lerped), Some(TextStyle::new().font_size(75.0)));

            // Missing from 1st ThemeData
            let lerped = ThemeData::lerp(
                &themed(&[&extension_a1]),
                &themed(&[&extension_a2, &extension_b2]),
                0.5,
            );
            let (color1, color2) = a(&lerped);
            assert_same_color(color1, Color::new(0xff7f7f7f));
            assert_same_color(color2, Color::new(0xff90ab7d));
            // Not lerped
            assert_eq!(font_size(&lerped), Some(TextStyle::new().font_size(100.0)));
        });
    }

    #[test]
    fn theme_extensions_keep_the_order_they_were_given_in_and_lerp_keeps_the_first_themes() {
        android(|app| {
            let a: ThemeExtensionRef = Rc::new(MyThemeExtensionA {
                color1: None,
                color2: None,
            });
            let b: ThemeExtensionRef = Rc::new(MyThemeExtensionB { text_style: None });
            let a_type = TypeId::of::<MyThemeExtensionA>();
            let b_type = TypeId::of::<MyThemeExtensionB>();
            let types = |theme: &ThemeData| theme.extensions.keys().copied().collect::<Vec<_>>();
            let themed = |extensions: Vec<ThemeExtensionRef>| {
                ThemeData::builder().extensions(extensions).build(app)
            };

            assert_eq!(
                types(&themed(vec![Rc::clone(&a), Rc::clone(&b)])),
                [a_type, b_type]
            );
            assert_eq!(
                types(&themed(vec![Rc::clone(&b), Rc::clone(&a)])),
                [b_type, a_type]
            );

            // A repeated type keeps its first place and takes the last value.
            let last_a: ThemeExtensionRef = Rc::new(MyThemeExtensionA {
                color1: Some(Colors::BLACK),
                color2: None,
            });
            let repeated = themed(vec![Rc::clone(&a), Rc::clone(&b), last_a]);
            assert_eq!(types(&repeated), [a_type, b_type]);
            assert_eq!(
                repeated
                    .extension::<MyThemeExtensionA>()
                    .expect("extension A")
                    .color1,
                Some(Colors::BLACK)
            );

            // Lerp keeps the first theme's order, then adds the second's own.
            let lerped = ThemeData::lerp(
                &themed(vec![Rc::clone(&b)]),
                &themed(vec![Rc::clone(&a), Rc::clone(&b)]),
                0.5,
            );
            assert_eq!(types(&lerped), [b_type, a_type]);
        });
    }

    #[test]
    fn theme_extensions_return_none_when_not_found() {
        android(|app| {
            let theme = ThemeData::builder().extensions(Vec::new()).build(app);
            assert!(theme.extension::<MyThemeExtensionA>().is_none());
        });
    }

    #[test]
    fn copy_with_and_equality_basics() {
        android(|app| {
            assert_eq!(ThemeData::new(app), ThemeData::new(app).copy_with().build());
        });
    }

    #[test]
    fn equality_includes_focus_color_and_hover_color() {
        // regression test for https://github.com/flutter/flutter/issues/91587
        android(|app| {
            let focus_color_black = ThemeData::builder().focus_color(Colors::BLACK).build(app);
            let focus_color_white = ThemeData::builder().focus_color(Colors::WHITE).build(app);
            assert_ne!(focus_color_black, focus_color_white);

            let hover_color_black = ThemeData::builder().hover_color(Colors::BLACK).build(app);
            let hover_color_white = ThemeData::builder().hover_color(Colors::WHITE).build(app);
            assert_ne!(hover_color_black, hover_color_white);
        });
    }

    #[test]
    fn text_selection_theme_defaults_to_an_empty_theme_and_takes_part_in_equality() {
        android(|app| {
            let theme = ThemeData::new(app);
            assert_eq!(theme.text_selection_theme, TextSelectionThemeData::new());

            let red = ThemeData::builder()
                .text_selection_theme(
                    TextSelectionThemeData::new().cursor_color(Colors::RED.primary()),
                )
                .build(app);
            assert_eq!(
                red.text_selection_theme.cursor_color,
                Some(Colors::RED.primary())
            );
            assert_ne!(theme, red);
            assert_eq!(
                theme
                    .copy_with()
                    .text_selection_theme(red.text_selection_theme.clone())
                    .build(),
                red
            );
        });
    }

    #[test]
    fn text_selection_theme_lerps_with_the_theme() {
        android(|app| {
            let theme_with = |color: Color| {
                ThemeData::builder()
                    .text_selection_theme(TextSelectionThemeData::new().cursor_color(color))
                    .build(app)
            };
            let lerped =
                ThemeData::lerp(&theme_with(Colors::BLACK), &theme_with(Colors::WHITE), 0.5);
            assert_eq!(
                lerped.text_selection_theme.cursor_color,
                Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
            );
        });
    }

    /// Dart's `theme` in the copy_with case.
    fn black_raw_theme(dark: &ThemeData) -> Box<ThemeData> {
        Box::new(ThemeData {
            // GENERAL CONFIGURATION
            adaptation_map: IndexMap::new(),
            apply_elevation_overlay_color: false,
            cupertino_override_theme: None,
            extensions: IndexMap::new(),
            input_decoration_theme: dark
                .input_decoration_theme
                .copy_with()
                .border(OutlineInputBorder::new()),
            material_tap_target_size: MaterialTapTargetSize::ShrinkWrap,
            page_transitions_theme: PageTransitionsTheme::new().with_builders(IndexMap::from([
                (
                    TargetPlatform::IOS,
                    PageTransitionsBuilderRef::new(CupertinoPageTransitionsBuilder::new()),
                ),
                (
                    TargetPlatform::MacOS,
                    PageTransitionsBuilderRef::new(CupertinoPageTransitionsBuilder::new()),
                ),
            ])),
            platform: TargetPlatform::IOS,
            scrollbar_theme: ScrollbarThemeData::new()
                .radius(waft_embedder::Radius::circular(10.0)),
            splash_factory: InkRipple::splash_factory(),
            use_material3: false,
            visual_density: VisualDensity::STANDARD,
            // COLOR
            canvas_color: Colors::BLACK,
            card_color: Colors::BLACK,
            color_scheme: ColorScheme::light(),
            disabled_color: Colors::BLACK,
            divider_color: Colors::BLACK,
            focus_color: Colors::BLACK,
            highlight_color: Colors::BLACK,
            hint_color: Colors::BLACK,
            hover_color: Colors::BLACK,
            primary_color: Colors::BLACK,
            primary_color_dark: Colors::BLACK,
            primary_color_light: Colors::BLACK,
            scaffold_background_color: Colors::BLACK,
            secondary_header_color: Colors::BLACK,
            shadow_color: Colors::BLACK,
            splash_color: Colors::BLACK,
            unselected_widget_color: Colors::BLACK,
            // TYPOGRAPHY & ICONOGRAPHY
            icon_theme: dark.icon_theme.clone(),
            primary_icon_theme: dark.icon_theme.clone(),
            primary_text_theme: dark.text_theme.clone(),
            text_theme: dark.text_theme.clone(),
            typography: Typography::material2018().build(),
            // COMPONENT THEMES
            action_icon_theme: Some(ActionIconThemeData::new()),
            app_bar_theme: AppBarThemeData::new().elevation(1.0),
            badge_theme: BadgeThemeData::new().small_size(1.0),
            banner_theme: MaterialBannerThemeData::new().elevation(1.0),
            bottom_navigation_bar_theme: BottomNavigationBarThemeData::new()
                .r#type(BottomNavigationBarType::Fixed),
            bottom_sheet_theme: BottomSheetThemeData::new().elevation(1.0),
            card_theme: CardThemeData::new().elevation(1.0),
            checkbox_theme: CheckboxThemeData::new(),
            dialog_theme: DialogThemeData::new().background_color(Colors::BLACK),
            divider_theme: DividerThemeData::new().color(Colors::BLACK),
            drawer_theme: DrawerThemeData::new().elevation(1.0),
            elevated_button_theme: ElevatedButtonThemeData::new().style(
                ElevatedButton::style_from()
                    .background_color(Colors::GREEN.primary())
                    .build(),
            ),
            expansion_tile_theme: ExpansionTileThemeData::new().background_color(Colors::BLACK),
            filled_button_theme: FilledButtonThemeData::new().style(
                FilledButton::style_from()
                    .foreground_color(Colors::GREEN.primary())
                    .build(),
            ),
            floating_action_button_theme: FloatingActionButtonThemeData::new().elevation(1.0),
            icon_button_theme: IconButtonThemeData::new().style(
                IconButton::style_from()
                    .foreground_color(Colors::PINK.primary())
                    .build(),
            ),
            list_tile_theme: ListTileThemeData::new(),
            menu_bar_theme: MenuBarThemeData::new().style(
                MenuStyle::new()
                    .background_color(Rc::new(MaterialStatePropertyAll::new(Some(Colors::BLACK)))),
            ),
            menu_button_theme: MenuButtonThemeData::new().style(
                MenuItemButton::style_from()
                    .background_color(Colors::BLACK)
                    .build(),
            ),
            menu_theme: MenuThemeData::new().style(
                MenuStyle::new()
                    .background_color(Rc::new(MaterialStatePropertyAll::new(Some(Colors::BLACK)))),
            ),
            navigation_bar_theme: NavigationBarThemeData::new().background_color(Colors::BLACK),
            navigation_drawer_theme: NavigationDrawerThemeData::new()
                .background_color(Colors::BLACK),
            navigation_rail_theme: NavigationRailThemeData::new().background_color(Colors::BLACK),
            outlined_button_theme: OutlinedButtonThemeData::new().style(
                OutlinedButton::style_from()
                    .foreground_color(Colors::BLUE.primary())
                    .build(),
            ),
            popup_menu_theme: PopupMenuThemeData::new().color(Colors::BLACK),
            progress_indicator_theme: ProgressIndicatorThemeData::new(),
            radio_theme: RadioThemeData::new(),
            slider_theme: SliderThemeData::from_primary_colors(
                Colors::BLACK,
                Colors::BLACK,
                Colors::BLACK,
                TextStyle::new().color(Colors::BLACK),
            ),
            snack_bar_theme: SnackBarThemeData::new().elevation(1.0),
            switch_theme: SwitchThemeData::new(),
            tab_bar_theme: TabBarThemeData::new().label_color(Colors::BLACK),
            text_button_theme: TextButtonThemeData::new().style(
                TextButton::style_from()
                    .foreground_color(Colors::RED.primary())
                    .build(),
            ),
            text_selection_theme: TextSelectionThemeData::new().cursor_color(Colors::BLACK),
            tooltip_theme: TooltipThemeData::new().height(100.0),
            // DEPRECATED (newest deprecations at the bottom)
            dialog_background_color: Colors::BLACK,
            indicator_color: Colors::BLACK,
        })
    }

    /// Dart's `otherTheme` in the copy_with case.
    fn white_raw_theme(light: &ThemeData) -> Box<ThemeData> {
        Box::new(ThemeData {
            // GENERAL CONFIGURATION
            adaptation_map: ThemeData::create_adaptation_map(vec![AdaptationRef::new(
                StringAdaptation,
            )]),
            apply_elevation_overlay_color: true,
            cupertino_override_theme: light.cupertino_override_theme.clone(),
            extensions: ThemeData::theme_extension_iterable_to_map(vec![Rc::new(
                MyThemeExtensionB {
                    text_style: Some(TextStyle::new()),
                },
            )]),
            input_decoration_theme: light
                .input_decoration_theme
                .copy_with()
                .border(<dyn InputBorder>::none()),
            material_tap_target_size: MaterialTapTargetSize::Padded,
            page_transitions_theme: PageTransitionsTheme::new(),
            platform: TargetPlatform::Android,
            scrollbar_theme: ScrollbarThemeData::new()
                .radius(waft_embedder::Radius::circular(10.0)),
            splash_factory: InkRipple::splash_factory(),
            use_material3: true,
            visual_density: VisualDensity::STANDARD,
            // COLOR
            canvas_color: Colors::WHITE,
            card_color: Colors::WHITE,
            color_scheme: ColorScheme::light(),
            disabled_color: Colors::WHITE,
            divider_color: Colors::WHITE,
            focus_color: Colors::WHITE,
            highlight_color: Colors::WHITE,
            hint_color: Colors::WHITE,
            hover_color: Colors::WHITE,
            primary_color: Colors::WHITE,
            primary_color_dark: Colors::WHITE,
            primary_color_light: Colors::WHITE,
            scaffold_background_color: Colors::WHITE,
            secondary_header_color: Colors::WHITE,
            shadow_color: Colors::WHITE,
            splash_color: Colors::WHITE,
            unselected_widget_color: Colors::WHITE,
            // TYPOGRAPHY & ICONOGRAPHY
            icon_theme: light.icon_theme.clone(),
            primary_icon_theme: light.icon_theme.clone(),
            primary_text_theme: light.text_theme.clone(),
            text_theme: light.text_theme.clone(),
            typography: Typography::material2018()
                .platform(Some(TargetPlatform::IOS))
                .build(),
            // COMPONENT THEMES
            action_icon_theme: Some(ActionIconThemeData::new()),
            app_bar_theme: AppBarThemeData::new().elevation(2.0),
            badge_theme: BadgeThemeData::new().small_size(2.0),
            banner_theme: MaterialBannerThemeData::new().elevation(2.0),
            bottom_navigation_bar_theme: BottomNavigationBarThemeData::new()
                .r#type(BottomNavigationBarType::Shifting),
            bottom_sheet_theme: BottomSheetThemeData::new().elevation(2.0),
            card_theme: CardThemeData::new().elevation(2.0),
            checkbox_theme: CheckboxThemeData::new(),
            dialog_theme: DialogThemeData::new().background_color(Colors::WHITE),
            divider_theme: DividerThemeData::new().color(Colors::WHITE),
            drawer_theme: DrawerThemeData::new().elevation(2.0),
            elevated_button_theme: ElevatedButtonThemeData::new(),
            expansion_tile_theme: ExpansionTileThemeData::new().background_color(Colors::BLACK),
            filled_button_theme: FilledButtonThemeData::new(),
            floating_action_button_theme: FloatingActionButtonThemeData::new().elevation(2.0),
            icon_button_theme: IconButtonThemeData::new(),
            list_tile_theme: ListTileThemeData::new(),
            menu_bar_theme: MenuBarThemeData::new().style(
                MenuStyle::new()
                    .background_color(Rc::new(MaterialStatePropertyAll::new(Some(Colors::WHITE)))),
            ),
            menu_button_theme: MenuButtonThemeData::new().style(
                MenuItemButton::style_from()
                    .background_color(Colors::BLACK)
                    .build(),
            ),
            menu_theme: MenuThemeData::new().style(
                MenuStyle::new()
                    .background_color(Rc::new(MaterialStatePropertyAll::new(Some(Colors::WHITE)))),
            ),
            navigation_bar_theme: NavigationBarThemeData::new().background_color(Colors::WHITE),
            navigation_drawer_theme: NavigationDrawerThemeData::new()
                .background_color(Colors::WHITE),
            navigation_rail_theme: NavigationRailThemeData::new().background_color(Colors::WHITE),
            outlined_button_theme: OutlinedButtonThemeData::new(),
            popup_menu_theme: PopupMenuThemeData::new().color(Colors::WHITE),
            progress_indicator_theme: ProgressIndicatorThemeData::new(),
            radio_theme: RadioThemeData::new(),
            slider_theme: SliderThemeData::from_primary_colors(
                Colors::WHITE,
                Colors::WHITE,
                Colors::WHITE,
                TextStyle::new().color(Colors::WHITE),
            ),
            snack_bar_theme: SnackBarThemeData::new().elevation(2.0),
            switch_theme: SwitchThemeData::new(),
            tab_bar_theme: TabBarThemeData::new().label_color(Colors::WHITE),
            text_button_theme: TextButtonThemeData::new(),
            text_selection_theme: TextSelectionThemeData::new().cursor_color(Colors::WHITE),
            tooltip_theme: TooltipThemeData::new().height(100.0),
            // DEPRECATED (newest deprecations at the bottom)
            dialog_background_color: Colors::WHITE,
            indicator_color: Colors::WHITE,
        })
    }

    /// `theme.copy_with()` with every argument taken from `other_theme`.
    fn copy_every_argument(theme: &ThemeData, other_theme: &ThemeData) -> Box<ThemeData> {
        let theme_data_copy = theme
            .copy_with()
            // GENERAL CONFIGURATION
            .adaptations(other_theme.adaptation_map.values().cloned().collect())
            .apply_elevation_overlay_color(other_theme.apply_elevation_overlay_color)
            .extensions(other_theme.extensions.values().cloned().collect())
            .input_decoration_theme(other_theme.input_decoration_theme.clone())
            .material_tap_target_size(other_theme.material_tap_target_size)
            .page_transitions_theme(other_theme.page_transitions_theme.clone())
            .platform(other_theme.platform)
            .scrollbar_theme(other_theme.scrollbar_theme.clone())
            .splash_factory(Rc::clone(&other_theme.splash_factory))
            .use_material3(other_theme.use_material3)
            .visual_density(other_theme.visual_density)
            // COLOR
            .canvas_color(other_theme.canvas_color)
            .card_color(other_theme.card_color)
            .color_scheme(other_theme.color_scheme.clone())
            .disabled_color(other_theme.disabled_color)
            .divider_color(other_theme.divider_color)
            .focus_color(other_theme.focus_color)
            .highlight_color(other_theme.highlight_color)
            .hint_color(other_theme.hint_color)
            .hover_color(other_theme.hover_color)
            .primary_color(other_theme.primary_color)
            .primary_color_dark(other_theme.primary_color_dark)
            .primary_color_light(other_theme.primary_color_light)
            .scaffold_background_color(other_theme.scaffold_background_color)
            .secondary_header_color(other_theme.secondary_header_color)
            .shadow_color(other_theme.shadow_color)
            .splash_color(other_theme.splash_color)
            .unselected_widget_color(other_theme.unselected_widget_color)
            // TYPOGRAPHY & ICONOGRAPHY
            .icon_theme(other_theme.icon_theme.clone())
            .primary_icon_theme(other_theme.primary_icon_theme.clone())
            .primary_text_theme(other_theme.primary_text_theme.clone())
            .text_theme(other_theme.text_theme.clone())
            .typography(other_theme.typography.clone())
            // COMPONENT THEMES
            .action_icon_theme(
                other_theme
                    .action_icon_theme
                    .clone()
                    .expect("the other theme has an action icon theme"),
            )
            .app_bar_theme(other_theme.app_bar_theme.clone())
            .badge_theme(other_theme.badge_theme.clone())
            .banner_theme(other_theme.banner_theme.clone())
            .bottom_navigation_bar_theme(other_theme.bottom_navigation_bar_theme.clone())
            .bottom_sheet_theme(other_theme.bottom_sheet_theme.clone())
            .card_theme(other_theme.card_theme.clone())
            .checkbox_theme(other_theme.checkbox_theme.clone())
            .dialog_theme(other_theme.dialog_theme.clone())
            .divider_theme(other_theme.divider_theme.clone())
            .drawer_theme(other_theme.drawer_theme.clone())
            .elevated_button_theme(other_theme.elevated_button_theme.clone())
            .expansion_tile_theme(other_theme.expansion_tile_theme.clone())
            .filled_button_theme(other_theme.filled_button_theme.clone())
            .floating_action_button_theme(other_theme.floating_action_button_theme.clone())
            .icon_button_theme(other_theme.icon_button_theme.clone())
            .list_tile_theme(other_theme.list_tile_theme.clone())
            .menu_bar_theme(other_theme.menu_bar_theme.clone())
            .menu_button_theme(other_theme.menu_button_theme.clone())
            .menu_theme(other_theme.menu_theme.clone())
            .navigation_bar_theme(other_theme.navigation_bar_theme.clone())
            .navigation_drawer_theme(other_theme.navigation_drawer_theme.clone())
            .navigation_rail_theme(other_theme.navigation_rail_theme.clone())
            .outlined_button_theme(other_theme.outlined_button_theme.clone())
            .popup_menu_theme(other_theme.popup_menu_theme.clone())
            .progress_indicator_theme(other_theme.progress_indicator_theme)
            .radio_theme(other_theme.radio_theme.clone())
            .slider_theme(other_theme.slider_theme.clone())
            .snack_bar_theme(other_theme.snack_bar_theme.clone())
            .switch_theme(other_theme.switch_theme.clone())
            .tab_bar_theme(other_theme.tab_bar_theme.clone())
            .text_button_theme(other_theme.text_button_theme.clone())
            .text_selection_theme(other_theme.text_selection_theme.clone())
            .tooltip_theme(other_theme.tooltip_theme.clone())
            // DEPRECATED (newest deprecations at the bottom)
            .dialog_background_color(other_theme.dialog_background_color)
            .indicator_color(other_theme.indicator_color)
            .build();
        Box::new(theme_data_copy)
    }

    #[test]
    fn copy_with_correctly_creates_new_theme_data_with_all_copied_arguments() {
        android(|app| {
            // Each `ThemeData` is over 100 KB: the five here live on the heap so that the
            // test thread's stack holds them in a debug build.
            let dark = Box::new(ThemeData::dark(app, None));
            let light = Box::new(ThemeData::new(app));
            let theme = black_raw_theme(&dark);

            let other_theme = white_raw_theme(&light);

            let theme_data_copy = copy_every_argument(&theme, &other_theme);

            // GENERAL CONFIGURATION
            assert_eq!(theme_data_copy.adaptation_map, other_theme.adaptation_map);
            assert_eq!(
                theme_data_copy.apply_elevation_overlay_color,
                other_theme.apply_elevation_overlay_color
            );
            assert_eq!(
                theme_data_copy.cupertino_override_theme,
                other_theme.cupertino_override_theme
            );
            assert!(extensions_equal(
                &theme_data_copy.extensions,
                &other_theme.extensions
            ));
            assert_eq!(
                theme_data_copy.input_decoration_theme,
                other_theme.input_decoration_theme
            );
            assert_eq!(
                theme_data_copy.material_tap_target_size,
                other_theme.material_tap_target_size
            );
            assert_eq!(
                theme_data_copy.page_transitions_theme,
                other_theme.page_transitions_theme
            );
            assert_eq!(theme_data_copy.platform, other_theme.platform);
            assert_eq!(theme_data_copy.scrollbar_theme, other_theme.scrollbar_theme);
            assert!(Rc::ptr_eq(
                &theme_data_copy.splash_factory,
                &other_theme.splash_factory
            ));
            assert_eq!(theme_data_copy.use_material3, other_theme.use_material3);
            assert_eq!(theme_data_copy.visual_density, other_theme.visual_density);
            // COLOR
            assert_eq!(theme_data_copy.canvas_color, other_theme.canvas_color);
            assert_eq!(theme_data_copy.card_color, other_theme.card_color);
            assert_eq!(theme_data_copy.color_scheme, other_theme.color_scheme);
            assert_eq!(theme_data_copy.disabled_color, other_theme.disabled_color);
            assert_eq!(theme_data_copy.divider_color, other_theme.divider_color);
            assert_eq!(theme_data_copy.focus_color, other_theme.focus_color);
            assert_eq!(theme_data_copy.highlight_color, other_theme.highlight_color);
            assert_eq!(theme_data_copy.hint_color, other_theme.hint_color);
            assert_eq!(theme_data_copy.hover_color, other_theme.hover_color);
            assert_eq!(theme_data_copy.primary_color, other_theme.primary_color);
            assert_eq!(
                theme_data_copy.primary_color_dark,
                other_theme.primary_color_dark
            );
            assert_eq!(
                theme_data_copy.primary_color_light,
                other_theme.primary_color_light
            );
            assert_eq!(
                theme_data_copy.scaffold_background_color,
                other_theme.scaffold_background_color
            );
            assert_eq!(
                theme_data_copy.secondary_header_color,
                other_theme.secondary_header_color
            );
            assert_eq!(theme_data_copy.shadow_color, other_theme.shadow_color);
            assert_eq!(theme_data_copy.splash_color, other_theme.splash_color);
            assert_eq!(
                theme_data_copy.unselected_widget_color,
                other_theme.unselected_widget_color
            );
            // TYPOGRAPHY & ICONOGRAPHY
            assert_eq!(theme_data_copy.icon_theme, other_theme.icon_theme);
            assert_eq!(
                theme_data_copy.primary_icon_theme,
                other_theme.primary_icon_theme
            );
            assert_eq!(
                theme_data_copy.primary_text_theme,
                other_theme.primary_text_theme
            );
            assert_eq!(theme_data_copy.text_theme, other_theme.text_theme);
            assert_eq!(theme_data_copy.typography, other_theme.typography);
            // COMPONENT THEMES
            assert_eq!(
                theme_data_copy.action_icon_theme,
                other_theme.action_icon_theme
            );
            assert_eq!(theme_data_copy.app_bar_theme, other_theme.app_bar_theme);
            assert_eq!(theme_data_copy.badge_theme, other_theme.badge_theme);
            assert_eq!(theme_data_copy.banner_theme, other_theme.banner_theme);
            assert_eq!(
                theme_data_copy.bottom_navigation_bar_theme,
                other_theme.bottom_navigation_bar_theme
            );
            assert_eq!(
                theme_data_copy.bottom_sheet_theme,
                other_theme.bottom_sheet_theme
            );
            assert_eq!(theme_data_copy.card_theme, other_theme.card_theme);
            assert_eq!(theme_data_copy.checkbox_theme, other_theme.checkbox_theme);
            assert_eq!(theme_data_copy.dialog_theme, other_theme.dialog_theme);
            assert_eq!(theme_data_copy.divider_theme, other_theme.divider_theme);
            assert_eq!(theme_data_copy.drawer_theme, other_theme.drawer_theme);
            assert_eq!(
                theme_data_copy.elevated_button_theme,
                other_theme.elevated_button_theme
            );
            assert_eq!(
                theme_data_copy.expansion_tile_theme,
                other_theme.expansion_tile_theme
            );
            assert_eq!(
                theme_data_copy.filled_button_theme,
                other_theme.filled_button_theme
            );
            assert_eq!(
                theme_data_copy.floating_action_button_theme,
                other_theme.floating_action_button_theme
            );
            assert_eq!(
                theme_data_copy.icon_button_theme,
                other_theme.icon_button_theme
            );
            assert_eq!(theme_data_copy.list_tile_theme, other_theme.list_tile_theme);
            assert_eq!(theme_data_copy.menu_bar_theme, other_theme.menu_bar_theme);
            assert_eq!(
                theme_data_copy.menu_button_theme,
                other_theme.menu_button_theme
            );
            assert_eq!(theme_data_copy.menu_theme, other_theme.menu_theme);
            assert_eq!(
                theme_data_copy.navigation_bar_theme,
                other_theme.navigation_bar_theme
            );
            assert_eq!(
                theme_data_copy.navigation_drawer_theme,
                other_theme.navigation_drawer_theme
            );
            assert_eq!(
                theme_data_copy.navigation_rail_theme,
                other_theme.navigation_rail_theme
            );
            assert_eq!(
                theme_data_copy.outlined_button_theme,
                other_theme.outlined_button_theme
            );
            assert_eq!(
                theme_data_copy.popup_menu_theme,
                other_theme.popup_menu_theme
            );
            assert_eq!(
                theme_data_copy.progress_indicator_theme,
                other_theme.progress_indicator_theme
            );
            assert_eq!(theme_data_copy.radio_theme, other_theme.radio_theme);
            assert_eq!(theme_data_copy.slider_theme, other_theme.slider_theme);
            assert_eq!(theme_data_copy.snack_bar_theme, other_theme.snack_bar_theme);
            assert_eq!(theme_data_copy.switch_theme, other_theme.switch_theme);
            assert_eq!(theme_data_copy.tab_bar_theme, other_theme.tab_bar_theme);
            assert_eq!(
                theme_data_copy.text_button_theme,
                other_theme.text_button_theme
            );
            assert_eq!(
                theme_data_copy.text_selection_theme,
                other_theme.text_selection_theme
            );
            assert_eq!(
                theme_data_copy.text_selection_theme.selection_color,
                other_theme.text_selection_theme.selection_color
            );
            assert_eq!(
                theme_data_copy.text_selection_theme.cursor_color,
                other_theme.text_selection_theme.cursor_color
            );
            assert_eq!(
                theme_data_copy.text_selection_theme.selection_handle_color,
                other_theme.text_selection_theme.selection_handle_color
            );
            assert_eq!(theme_data_copy.tooltip_theme, other_theme.tooltip_theme);
            // DEPRECATED (newest deprecations at the bottom)
            assert_eq!(
                theme_data_copy.dialog_background_color,
                other_theme.dialog_background_color
            );
            assert_eq!(theme_data_copy.indicator_color, other_theme.indicator_color);
        });
    }

    #[test]
    #[should_panic(expected = "ThemeData.brightness does not match ColorScheme.brightness")]
    fn theme_data_brightness_must_match_the_color_scheme_brightness() {
        android(|app| {
            ThemeData::builder()
                .color_scheme(ColorScheme::light())
                .brightness(Brightness::Dark)
                .build(app)
        });
    }

    #[test]
    fn theme_data_copy_with_brightness_parameter_overrides_color_scheme_brightness() {
        android(|app| {
            let light_colors = ColorScheme::light();
            let theme = ThemeData::from(light_colors.clone())
                .build(app)
                .copy_with()
                .brightness(Brightness::Dark)
                .build();

            // The brightness parameter only overrides ColorScheme.brightness.
            assert_eq!(theme.brightness(), Brightness::Dark);
            assert_eq!(theme.color_scheme.brightness, Brightness::Dark);
            assert_eq!(theme.primary_color, light_colors.primary);
            assert_eq!(theme.card_color, light_colors.surface);
            assert_eq!(theme.canvas_color, light_colors.surface);
            assert_eq!(theme.scaffold_background_color, light_colors.surface);
            assert_eq!(theme.dialog_background_color, light_colors.surface);
            assert!(!theme.apply_elevation_overlay_color);
        });
    }

    #[test]
    fn copy_with_applies_brightness_whichever_order_it_is_given_in() {
        android(|app| {
            let theme = ThemeData::new(app);
            let brightness_first = theme
                .copy_with()
                .brightness(Brightness::Dark)
                .color_scheme(ColorScheme::light())
                .build();
            assert_eq!(brightness_first.color_scheme.brightness, Brightness::Dark);
        });
    }

    #[test]
    fn theme_adaptation_map_can_be_obtained() {
        android(|app| {
            let theme = ThemeData::builder()
                .adaptations(vec![AdaptationRef::new(StringAdaptation)])
                .build(app);
            let adaptive_string = theme
                .get_adaptation::<String>()
                .expect("a String adaptation")
                .adapt(&theme, "Default theme".to_string());

            assert_eq!(adaptive_string, "Adaptive theme.");
        });
    }

    #[test]
    fn theme_adaptation_map_returns_none_when_not_found() {
        android(|app| {
            let theme = ThemeData::builder()
                .adaptations(vec![AdaptationRef::new(StringAdaptation)])
                .build(app);

            assert!(theme.get_adaptation::<f64>().is_none());
            assert!(theme.extension::<MyThemeExtensionA>().is_none());
        });
    }

    /// Dart's third call, `ThemeData(inputDecorationTheme: Object())`, cannot be written:
    /// the setter takes `impl Into<InputDecorationThemeData>`.
    #[test]
    fn theme_data_input_decoration_theme_accepts_only_a_theme_or_theme_data() {
        android(|app| {
            let from_theme = ThemeData::builder()
                .input_decoration_theme(crate::input_decorator::InputDecorationTheme::new())
                .build(app);
            assert_eq!(
                from_theme.input_decoration_theme,
                crate::input_decorator::InputDecorationThemeData::new()
            );

            let from_data = ThemeData::builder()
                .input_decoration_theme(crate::input_decorator::InputDecorationThemeData::new())
                .build(app);
            assert_eq!(
                from_data.input_decoration_theme,
                crate::input_decorator::InputDecorationThemeData::new()
            );
        });
    }

    #[test]
    fn localize_merges_the_text_geometry_under_the_theme_text_styles() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let mut app = cell.borrow_mut();
            let theme = Rc::new(ThemeData::new(&app));
            let geometry = Rc::clone(&theme.typography.english_like);
            let localized = ThemeData::localize(&mut app, &theme, &geometry);
            assert_eq!(
                localized.text_theme,
                geometry.merge(Some(&theme.text_theme))
            );
            assert_eq!(
                localized.primary_text_theme,
                geometry.merge(Some(&theme.primary_text_theme))
            );
            let body = localized
                .text_theme
                .body_medium
                .clone()
                .expect("a body style");
            assert_eq!(body.font_size, Some(14.0));
            assert!(!body.inherit);
        });
    }
}
