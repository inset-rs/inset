//! Flutter counterpart: `cupertino/theme.dart`.
//!
//! `InheritedTheme` (`wrap` / `captureAll`) waits in widgets; `InheritedCupertinoTheme` is a
//! plain inherited widget with `wrap` as an inherent method until then.

use reveal_embedder::{Brightness, Color};
use reveal_foundation::App;
use reveal_painting::AnyColor;
use reveal_widgets::{
    BuildContext, IconTheme, InheritedWidget, IntoWidget, KeyRef, MediaQuery, StatelessWidget,
    WidgetRef,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::icon_theme_data::CupertinoIconThemeData;
use crate::text_theme::CupertinoTextThemeData;

// Values extracted from navigation bar. For toolbar or tabbar the dark color is 0xF0161616.
const DEFAULT_BAR_BACKGROUND_COLOR: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xF0F9F9F9), Color::new(0xF01D1D1D));

// Values derived from https://developer.apple.com/design/resources/.
const K_DEFAULT_THEME: CupertinoThemeDefaults = CupertinoThemeDefaults {
    brightness: None,
    primary_color: CupertinoColors::SYSTEM_BLUE,
    primary_contrasting_color: CupertinoColors::WHITE,
    bar_background_color: AnyColor::with_static(
        DEFAULT_BAR_BACKGROUND_COLOR.effective_color(),
        &DEFAULT_BAR_BACKGROUND_COLOR,
    ),
    scaffold_background_color: CupertinoColors::SYSTEM_BACKGROUND,
    selection_handle_color: CupertinoColors::SYSTEM_BLUE,
    apply_theme_to_all: false,
    text_theme_defaults: CupertinoTextThemeDefaults {
        label_color: CupertinoColors::LABEL,
        inactive_gray: CupertinoColors::INACTIVE_GRAY,
    },
};

/// Applies a visual styling theme to descendant Cupertino widgets.
///
/// Affects the color and text styles of Cupertino widgets whose styling are not overridden
/// when constructing the respective widgets instances.
///
/// Descendant widgets can retrieve the current [`CupertinoThemeData`] by calling
/// [`CupertinoTheme::of`]. An `InheritedWidget` dependency is created when an ancestor
/// [`CupertinoThemeData`] is retrieved via [`CupertinoTheme::of`].
///
/// The [`CupertinoTheme`] widget implies an `IconTheme` widget, whose `IconTheme.data` has
/// the same color as [`CupertinoThemeData::primary_color`].
///
/// See also:
///
///  * [`CupertinoThemeData`], specifies the theme's visual styling.
///  * `CupertinoApp`, which will automatically add a [`CupertinoTheme`].
///  * `Theme`, a Material theme which will automatically add a [`CupertinoTheme`] with a
///    [`CupertinoThemeData`] derived from the Material `ThemeData`.
#[derive(Clone, Debug)]
pub struct CupertinoTheme {
    pub key: Option<KeyRef>,
    /// The [`CupertinoThemeData`] styling for this theme.
    pub data: CupertinoThemeData,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl CupertinoTheme {
    /// Creates a [`CupertinoTheme`] to change descendant Cupertino widgets' styling.
    pub fn new<K>(data: CupertinoThemeData, child: impl IntoWidget<K>) -> CupertinoTheme {
        CupertinoTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `CupertinoTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`CupertinoThemeData`] from the closest ancestor [`CupertinoTheme`]
    /// widget, or a default [`CupertinoThemeData`] if no [`CupertinoTheme`] ancestor exists.
    ///
    /// Resolves all the colors defined in that [`CupertinoThemeData`] against the given
    /// [`BuildContext`] on a best-effort basis.
    pub fn of(app: &mut App, context: BuildContext) -> CupertinoThemeData {
        let data = context
            .depend_on_inherited_widget_of_exact_type::<InheritedCupertinoTheme>(app)
            .map(|inherited_theme| inherited_theme.theme.data.clone())
            .unwrap_or_default();
        data.resolve_from(app, context)
    }

    /// Retrieves the [`Brightness`] to use for descendant Cupertino widgets, based on the
    /// value of [`CupertinoThemeData::brightness`] in the given `context`.
    ///
    /// If no [`CupertinoTheme`] can be found in the given `context`, or its `brightness` is
    /// null, it will fall back to `MediaQueryData.platformBrightness`.
    ///
    /// Throws an exception if no valid [`CupertinoTheme`] or `MediaQuery` widgets exist in
    /// the ancestry tree.
    ///
    /// See also:
    ///
    ///  * [`maybe_brightness_of`](Self::maybe_brightness_of), which returns null if no valid
    ///    [`CupertinoTheme`] or `MediaQuery` exists, instead of throwing.
    ///  * [`CupertinoThemeData::brightness`], the property takes precedence over
    ///    `MediaQueryData.platformBrightness` for descendant Cupertino widgets.
    pub fn brightness_of(app: &mut App, context: BuildContext) -> Brightness {
        let brightness = context
            .depend_on_inherited_widget_of_exact_type::<InheritedCupertinoTheme>(app)
            .and_then(|inherited_theme| inherited_theme.theme.data.brightness());
        brightness.unwrap_or_else(|| MediaQuery::platform_brightness_of(app, context))
    }

    /// Retrieves the [`Brightness`] to use for descendant Cupertino widgets, based on the
    /// value of [`CupertinoThemeData::brightness`] in the given `context`.
    ///
    /// If no [`CupertinoTheme`] can be found in the given `context`, it will fall back to
    /// `MediaQueryData.platformBrightness`.
    ///
    /// Returns null if no valid [`CupertinoTheme`] or `MediaQuery` widgets exist in the
    /// ancestry tree.
    ///
    /// See also:
    ///
    ///  * [`CupertinoThemeData::brightness`], the property takes precedence over
    ///    `MediaQueryData.platformBrightness` for descendant Cupertino widgets.
    ///  * [`brightness_of`](Self::brightness_of), which throws if no valid [`CupertinoTheme`]
    ///    or `MediaQuery` exists, instead of returning null.
    pub fn maybe_brightness_of(app: &mut App, context: BuildContext) -> Option<Brightness> {
        let brightness = context
            .depend_on_inherited_widget_of_exact_type::<InheritedCupertinoTheme>(app)
            .and_then(|inherited_theme| inherited_theme.theme.data.brightness());
        brightness.or_else(|| MediaQuery::maybe_platform_brightness_of(app, context))
    }
}

impl StatelessWidget for CupertinoTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        InheritedCupertinoTheme {
            key: None,
            theme: self.clone(),
            child: IconTheme {
                key: None,
                data: CupertinoIconThemeData::new().color(self.data.primary_color()),
                child: self.child.clone(),
            }
            .into_widget(),
        }
        .into_widget()
    }
}

/// Provides a [`CupertinoTheme`] to all descendants.
#[derive(Debug)]
pub struct InheritedCupertinoTheme {
    pub key: Option<KeyRef>,
    /// The [`CupertinoTheme`] that is provided to widgets lower in the tree.
    pub theme: CupertinoTheme,
    pub child: WidgetRef,
}

impl InheritedCupertinoTheme {
    /// `InheritedTheme.wrap`: a new [`CupertinoTheme`] with this theme's data around `child`.
    pub fn wrap(&self, child: WidgetRef) -> WidgetRef {
        CupertinoTheme {
            key: None,
            data: self.theme.data.clone(),
            child,
        }
        .into_widget()
    }
}

impl InheritedWidget for InheritedCupertinoTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &InheritedCupertinoTheme) -> bool {
        self.theme.data != old_widget.theme.data
    }
}

/// Styling specifications for a [`CupertinoTheme`].
///
/// All constructor parameters can be null, in which case a [`CupertinoColors::ACTIVE_BLUE`]
/// based default iOS theme styling is used.
///
/// Parameters can also be partially specified, in which case some parameters will cascade
/// down to other dependent parameters to create a cohesive visual effect. For instance, if a
/// [`primary_color`](Self::primary_color) is specified, it would cascade down to affect some
/// fonts in [`text_theme`](Self::text_theme) if [`text_theme`](Self::text_theme) is not
/// specified.
///
/// Dart's `class CupertinoThemeData extends NoDefaultCupertinoThemeData`: the construction
/// time specifications are the [`NoDefaultCupertinoThemeData`] this holds, and the getters add
/// the defaults. Dart's named constructor arguments are the `with_*` setters
/// (`CupertinoThemeData::new().with_brightness(Brightness::Dark)`), or
/// [`raw`](Self::raw) over a [`NoDefaultCupertinoThemeData`] literal.
///
/// See also:
///
///  * [`CupertinoTheme`], in which this [`CupertinoThemeData`] is inserted.
///  * `ThemeData`, a Material equivalent that also configures Cupertino styling via a
///    [`CupertinoThemeData`] subclass `MaterialBasedCupertinoThemeData`.
#[derive(Clone, Debug)]
pub struct CupertinoThemeData {
    raw: NoDefaultCupertinoThemeData,
    defaults: CupertinoThemeDefaults,
}

impl Default for CupertinoThemeData {
    fn default() -> CupertinoThemeData {
        CupertinoThemeData::new()
    }
}

impl CupertinoThemeData {
    /// Creates a [`CupertinoTheme`] styling specification.
    ///
    /// Unspecified parameters default to a reasonable iOS default style.
    pub fn new() -> CupertinoThemeData {
        CupertinoThemeData::raw(NoDefaultCupertinoThemeData::default())
    }

    /// Same as the default constructor but over the construction time specifications, to
    /// avoid forgetting any and to specify all arguments.
    ///
    /// Used by subclasses to get the superclass's defaulting behaviors.
    pub fn raw(raw: NoDefaultCupertinoThemeData) -> CupertinoThemeData {
        CupertinoThemeData::raw_with_defaults(raw, K_DEFAULT_THEME)
    }

    fn raw_with_defaults(
        raw: NoDefaultCupertinoThemeData,
        defaults: CupertinoThemeDefaults,
    ) -> CupertinoThemeData {
        CupertinoThemeData { raw, defaults }
    }

    /// Dart `CupertinoThemeData(brightness:)`.
    pub fn with_brightness(mut self, brightness: Brightness) -> CupertinoThemeData {
        self.raw.brightness = Some(brightness);
        self
    }

    /// Dart `CupertinoThemeData(primaryColor:)`.
    pub fn with_primary_color(mut self, primary_color: impl Into<AnyColor>) -> CupertinoThemeData {
        self.raw.primary_color = Some(primary_color.into());
        self
    }

    /// Dart `CupertinoThemeData(primaryContrastingColor:)`.
    pub fn with_primary_contrasting_color(
        mut self,
        primary_contrasting_color: impl Into<AnyColor>,
    ) -> CupertinoThemeData {
        self.raw.primary_contrasting_color = Some(primary_contrasting_color.into());
        self
    }

    /// Dart `CupertinoThemeData(textTheme:)`.
    pub fn with_text_theme(mut self, text_theme: CupertinoTextThemeData) -> CupertinoThemeData {
        self.raw.text_theme = Some(text_theme);
        self
    }

    /// Dart `CupertinoThemeData(barBackgroundColor:)`.
    pub fn with_bar_background_color(
        mut self,
        bar_background_color: impl Into<AnyColor>,
    ) -> CupertinoThemeData {
        self.raw.bar_background_color = Some(bar_background_color.into());
        self
    }

    /// Dart `CupertinoThemeData(scaffoldBackgroundColor:)`.
    pub fn with_scaffold_background_color(
        mut self,
        scaffold_background_color: impl Into<AnyColor>,
    ) -> CupertinoThemeData {
        self.raw.scaffold_background_color = Some(scaffold_background_color.into());
        self
    }

    /// Dart `CupertinoThemeData(selectionHandleColor:)`.
    pub fn with_selection_handle_color(
        mut self,
        selection_handle_color: impl Into<AnyColor>,
    ) -> CupertinoThemeData {
        self.raw.selection_handle_color = Some(selection_handle_color.into());
        self
    }

    /// Dart `CupertinoThemeData(applyThemeToAll:)`.
    pub fn with_apply_theme_to_all(mut self, apply_theme_to_all: bool) -> CupertinoThemeData {
        self.raw.apply_theme_to_all = Some(apply_theme_to_all);
        self
    }

    /// The brightness override for Cupertino descendants.
    ///
    /// Defaults to null. If a non-null [`Brightness`] is specified, the value will take
    /// precedence over the ambient `MediaQueryData.platformBrightness`, when determining the
    /// brightness of descendant Cupertino widgets.
    ///
    /// See also:
    ///
    ///  * [`CupertinoTheme::brightness_of`], a method used to retrieve the overall
    ///    [`Brightness`] from a [`BuildContext`], for Cupertino widgets.
    pub fn brightness(&self) -> Option<Brightness> {
        self.raw.brightness.or(self.defaults.brightness)
    }

    /// A color used on interactive elements of the theme.
    ///
    /// This color is generally used on text and icons in buttons and tappable elements.
    /// Defaults to [`CupertinoColors::ACTIVE_BLUE`].
    pub fn primary_color(&self) -> AnyColor {
        self.raw
            .primary_color
            .clone()
            .unwrap_or_else(|| self.defaults.primary_color.clone())
    }

    /// A color that must be easy to see when rendered on a
    /// [`primary_color`](Self::primary_color) background.
    ///
    /// For example, this color is used for a `CupertinoButton`'s text and icons when the
    /// button's background is [`primary_color`](Self::primary_color).
    pub fn primary_contrasting_color(&self) -> AnyColor {
        self.raw
            .primary_contrasting_color
            .clone()
            .unwrap_or_else(|| self.defaults.primary_contrasting_color.clone())
    }

    /// Text styles used by Cupertino widgets.
    ///
    /// Derived from [`primary_color`](Self::primary_color) if unspecified.
    pub fn text_theme(&self) -> CupertinoTextThemeData {
        self.raw.text_theme.clone().unwrap_or_else(|| {
            self.defaults
                .text_theme_defaults
                .create_defaults(self.primary_color())
        })
    }

    /// Background color of the top nav bar and bottom tab bar.
    ///
    /// Defaults to a light gray in light mode, or a dark translucent gray color in dark mode.
    pub fn bar_background_color(&self) -> AnyColor {
        self.raw
            .bar_background_color
            .clone()
            .unwrap_or_else(|| self.defaults.bar_background_color.clone())
    }

    /// Background color of the scaffold.
    ///
    /// Defaults to [`CupertinoColors::SYSTEM_BACKGROUND`].
    pub fn scaffold_background_color(&self) -> AnyColor {
        self.raw
            .scaffold_background_color
            .clone()
            .unwrap_or_else(|| self.defaults.scaffold_background_color.clone())
    }

    /// The color of the selection handles on the text field.
    ///
    /// Defaults to [`CupertinoColors::SYSTEM_BLUE`].
    pub fn selection_handle_color(&self) -> AnyColor {
        self.raw
            .selection_handle_color
            .clone()
            .unwrap_or_else(|| self.defaults.selection_handle_color.clone())
    }

    /// Flag to apply this theme to all descendant Cupertino widgets.
    ///
    /// Certain Cupertino widgets previously didn't use theming, matching past versions of
    /// iOS. For example, `CupertinoSwitch`s always used [`CupertinoColors::SYSTEM_GREEN`]
    /// when active.
    ///
    /// Today, however, these widgets can indeed be themed on iOS. Moreover on macOS, the
    /// accent color is reflected in these widgets. Turning this flag on ensures that
    /// descendant Cupertino widgets will be themed accordingly.
    ///
    /// This flag currently applies to the following widgets:
    /// - `CupertinoSwitch` & `Switch.adaptive`
    ///
    /// Defaults to false.
    pub fn apply_theme_to_all(&self) -> bool {
        self.raw
            .apply_theme_to_all
            .unwrap_or(self.defaults.apply_theme_to_all)
    }

    /// Returns an instance of the theme data whose property getters only return the
    /// construction time specifications with no derived values.
    ///
    /// Used in Material themes to let unspecified properties fallback to Material theme
    /// properties instead of iOS defaults.
    pub fn no_default(&self) -> NoDefaultCupertinoThemeData {
        self.raw.clone()
    }

    /// Returns a new theme data with all its colors resolved against the given
    /// [`BuildContext`].
    ///
    /// Called by [`CupertinoTheme::of`] to resolve colors defined in the retrieved
    /// [`CupertinoThemeData`].
    pub fn resolve_from(&self, app: &mut App, context: BuildContext) -> CupertinoThemeData {
        let resolve_text_theme = self.raw.text_theme.is_none();
        CupertinoThemeData::raw_with_defaults(
            self.raw.resolve_from(app, context),
            self.defaults.resolve_from(app, context, resolve_text_theme),
        )
    }

    /// Creates a copy of the theme data with specified attributes overridden.
    ///
    /// Only the current instance's specified attributes are copied instead of derived values.
    /// For instance, if the current [`text_theme`](Self::text_theme) is implied from the
    /// current [`primary_color`](Self::primary_color) because it was not specified, copying
    /// with a different [`primary_color`](Self::primary_color) will also change the copy's
    /// implied [`text_theme`](Self::text_theme).
    ///
    /// Dart's named arguments are the `with_*` setters on the copy.
    pub fn copy_with(&self) -> CupertinoThemeData {
        self.clone()
    }
}

/// Dart's `==` compares the derived values, not the construction time specifications.
impl PartialEq for CupertinoThemeData {
    fn eq(&self, other: &CupertinoThemeData) -> bool {
        self.brightness() == other.brightness()
            && self.primary_color() == other.primary_color()
            && self.primary_contrasting_color() == other.primary_contrasting_color()
            && self.text_theme() == other.text_theme()
            && self.bar_background_color() == other.bar_background_color()
            && self.scaffold_background_color() == other.scaffold_background_color()
            && self.selection_handle_color() == other.selection_handle_color()
            && self.apply_theme_to_all() == other.apply_theme_to_all()
    }
}

/// Styling specifications for a cupertino theme without default values for unspecified
/// properties.
///
/// Unlike [`CupertinoThemeData`] instances of this class do not return default values for
/// properties that have been left unspecified in the constructor. Instead, unspecified
/// properties will return null. This is used by Material's `ThemeData.cupertinoOverrideTheme`.
///
/// See also:
///
///  * [`CupertinoThemeData`], which uses reasonable default values for unspecified theme
///    properties.
#[derive(Clone, Debug, Default)]
pub struct NoDefaultCupertinoThemeData {
    /// The brightness override for Cupertino descendants.
    ///
    /// Defaults to null. If a non-null [`Brightness`] is specified, the value will take
    /// precedence over the ambient `MediaQueryData.platformBrightness`, when determining the
    /// brightness of descendant Cupertino widgets.
    pub brightness: Option<Brightness>,
    /// A color used on interactive elements of the theme.
    ///
    /// This color is generally used on text and icons in buttons and tappable elements.
    /// Defaults to [`CupertinoColors::ACTIVE_BLUE`].
    pub primary_color: Option<AnyColor>,
    /// A color that must be easy to see when rendered on a
    /// [`primary_color`](Self::primary_color) background.
    pub primary_contrasting_color: Option<AnyColor>,
    /// Text styles used by Cupertino widgets.
    ///
    /// Derived from [`primary_color`](Self::primary_color) if unspecified.
    pub text_theme: Option<CupertinoTextThemeData>,
    /// Background color of the top nav bar and bottom tab bar.
    pub bar_background_color: Option<AnyColor>,
    /// Background color of the scaffold.
    pub scaffold_background_color: Option<AnyColor>,
    /// The color of the selection handles on the text field.
    pub selection_handle_color: Option<AnyColor>,
    /// Flag to apply this theme to all descendant Cupertino widgets.
    pub apply_theme_to_all: Option<bool>,
}

impl NoDefaultCupertinoThemeData {
    /// Creates a [`NoDefaultCupertinoThemeData`] styling specification.
    ///
    /// Unspecified properties default to null.
    pub fn new() -> NoDefaultCupertinoThemeData {
        NoDefaultCupertinoThemeData::default()
    }

    /// Returns an instance of the theme data whose property getters only return the
    /// construction time specifications with no derived values.
    pub fn no_default(&self) -> NoDefaultCupertinoThemeData {
        self.clone()
    }

    /// Returns a new theme data with all its colors resolved against the given
    /// [`BuildContext`].
    pub(crate) fn resolve_from(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> NoDefaultCupertinoThemeData {
        NoDefaultCupertinoThemeData {
            brightness: self.brightness,
            primary_color: CupertinoDynamicColor::maybe_resolve(
                self.primary_color.as_ref(),
                app,
                context,
            ),
            primary_contrasting_color: CupertinoDynamicColor::maybe_resolve(
                self.primary_contrasting_color.as_ref(),
                app,
                context,
            ),
            text_theme: self
                .text_theme
                .as_ref()
                .map(|text_theme| text_theme.resolve_from(app, context)),
            bar_background_color: CupertinoDynamicColor::maybe_resolve(
                self.bar_background_color.as_ref(),
                app,
                context,
            ),
            scaffold_background_color: CupertinoDynamicColor::maybe_resolve(
                self.scaffold_background_color.as_ref(),
                app,
                context,
            ),
            selection_handle_color: CupertinoDynamicColor::maybe_resolve(
                self.selection_handle_color.as_ref(),
                app,
                context,
            ),
            apply_theme_to_all: self.apply_theme_to_all,
        }
    }

    /// Creates a copy of the theme data with specified attributes overridden.
    ///
    /// Only the current instance's specified attributes are copied instead of derived values.
    /// Dart's named arguments are field assignments on the copy.
    pub fn copy_with(&self) -> NoDefaultCupertinoThemeData {
        self.clone()
    }
}

/// Dart's `==` leaves `selectionHandleColor` out; kept as it is.
impl PartialEq for NoDefaultCupertinoThemeData {
    fn eq(&self, other: &NoDefaultCupertinoThemeData) -> bool {
        self.brightness == other.brightness
            && self.primary_color == other.primary_color
            && self.primary_contrasting_color == other.primary_contrasting_color
            && self.text_theme == other.text_theme
            && self.bar_background_color == other.bar_background_color
            && self.scaffold_background_color == other.scaffold_background_color
            && self.apply_theme_to_all == other.apply_theme_to_all
    }
}

/// Dart's `_CupertinoThemeDefaults`.
#[derive(Clone, Debug)]
struct CupertinoThemeDefaults {
    brightness: Option<Brightness>,
    primary_color: AnyColor,
    primary_contrasting_color: AnyColor,
    bar_background_color: AnyColor,
    scaffold_background_color: AnyColor,
    selection_handle_color: AnyColor,
    apply_theme_to_all: bool,
    text_theme_defaults: CupertinoTextThemeDefaults,
}

impl CupertinoThemeDefaults {
    fn resolve_from(
        &self,
        app: &mut App,
        context: BuildContext,
        resolve_text_theme: bool,
    ) -> CupertinoThemeDefaults {
        CupertinoThemeDefaults {
            brightness: self.brightness,
            primary_color: CupertinoDynamicColor::resolve(&self.primary_color, app, context),
            primary_contrasting_color: CupertinoDynamicColor::resolve(
                &self.primary_contrasting_color,
                app,
                context,
            ),
            bar_background_color: CupertinoDynamicColor::resolve(
                &self.bar_background_color,
                app,
                context,
            ),
            scaffold_background_color: CupertinoDynamicColor::resolve(
                &self.scaffold_background_color,
                app,
                context,
            ),
            selection_handle_color: CupertinoDynamicColor::resolve(
                &self.selection_handle_color,
                app,
                context,
            ),
            apply_theme_to_all: self.apply_theme_to_all,
            text_theme_defaults: if resolve_text_theme {
                self.text_theme_defaults.resolve_from(app, context)
            } else {
                self.text_theme_defaults.clone()
            },
        }
    }
}

/// Dart's `_CupertinoTextThemeDefaults`.
#[derive(Clone, Debug)]
struct CupertinoTextThemeDefaults {
    label_color: AnyColor,
    inactive_gray: AnyColor,
}

impl CupertinoTextThemeDefaults {
    fn resolve_from(&self, app: &mut App, context: BuildContext) -> CupertinoTextThemeDefaults {
        CupertinoTextThemeDefaults {
            label_color: CupertinoDynamicColor::resolve(&self.label_color, app, context),
            inactive_gray: CupertinoDynamicColor::resolve(&self.inactive_gray, app, context),
        }
    }

    fn create_defaults(&self, primary_color: AnyColor) -> CupertinoTextThemeData {
        CupertinoTextThemeData::with_defaults(
            primary_color,
            self.label_color.clone(),
            self.inactive_gray.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use reveal_embedder::Color;
    use reveal_widgets::{Builder, SizedBox};

    use super::*;
    use crate::test_support::build;

    /// Builds `wrap(probe)` and returns what `read` saw at the probe's context.
    fn probe<T: 'static>(
        wrap: impl FnOnce(WidgetRef) -> WidgetRef,
        read: impl Fn(&mut App, BuildContext) -> T + 'static,
    ) -> T {
        let mut app = crate::test_support::app();
        let seen = Rc::new(RefCell::new(None));
        let probe = Builder::new({
            let seen = Rc::clone(&seen);
            move |app, context| {
                *seen.borrow_mut() = Some(read(app, context));
                SizedBox::shrink().into_widget()
            }
        })
        .into_widget();
        build(&mut app, wrap(probe));
        let value = seen.borrow_mut().take();
        value.expect("the probe built")
    }

    fn themed(data: CupertinoThemeData) -> impl FnOnce(WidgetRef) -> WidgetRef {
        move |child| CupertinoTheme::new(data, child).into_widget()
    }

    fn dynamic(color: &AnyColor) -> &CupertinoDynamicColor {
        color
            .extension::<CupertinoDynamicColor>()
            .expect("a dynamic color")
    }

    #[test]
    fn without_a_theme_of_returns_the_resolved_defaults() {
        let data = probe(|child| child, CupertinoTheme::of);
        assert_eq!(data.primary_color(), CupertinoColors::SYSTEM_BLUE);
        assert_eq!(
            data.primary_color().color(),
            dynamic(&CupertinoColors::SYSTEM_BLUE).color
        );
        assert_eq!(data.brightness(), None);
        assert!(!data.apply_theme_to_all());
        assert_eq!(
            data.text_theme().action_text_style().color,
            Some(data.primary_color())
        );
    }

    #[test]
    fn a_dark_theme_resolves_its_colors_to_the_dark_variants() {
        let dark = CupertinoThemeData::new().with_brightness(Brightness::Dark);
        let (brightness, data) = probe(themed(dark), |app, context| {
            (
                CupertinoTheme::brightness_of(app, context),
                CupertinoTheme::of(app, context),
            )
        });
        assert_eq!(brightness, Brightness::Dark);
        assert_eq!(
            data.primary_color().color(),
            dynamic(&CupertinoColors::SYSTEM_BLUE).dark_color
        );
        assert_eq!(
            data.scaffold_background_color().color(),
            dynamic(&CupertinoColors::SYSTEM_BACKGROUND).dark_color
        );
        assert_eq!(
            data.text_theme()
                .text_style()
                .color
                .map(|color| color.color()),
            Some(dynamic(&CupertinoColors::LABEL).dark_color)
        );
    }

    #[test]
    fn a_primary_color_cascades_into_the_text_theme_and_the_icon_theme() {
        let red = Color::from_argb(255, 255, 0, 0);
        let themed = themed(CupertinoThemeData::new().with_primary_color(red));
        let (data, icon_color) = probe(themed, |app, context| {
            (
                CupertinoTheme::of(app, context),
                reveal_widgets::IconTheme::of(app, context).color,
            )
        });
        assert_eq!(data.primary_color(), AnyColor::new(red));
        assert_eq!(
            data.text_theme().action_text_style().color,
            Some(AnyColor::new(red))
        );
        assert_eq!(
            data.text_theme().nav_action_text_style().color,
            Some(AnyColor::new(red))
        );
        assert_eq!(icon_color, Some(AnyColor::new(red)));
    }

    #[test]
    fn the_implied_icon_theme_resolves_a_dynamic_primary_color() {
        let dark = CupertinoThemeData::new().with_brightness(Brightness::Dark);
        let icon_color = probe(themed(dark), |app, context| {
            reveal_widgets::IconTheme::of(app, context).color
        });
        assert_eq!(
            icon_color.map(|color| color.color()),
            Some(dynamic(&CupertinoColors::SYSTEM_BLUE).dark_color)
        );
    }

    #[test]
    fn copy_with_keeps_only_the_specified_attributes() {
        let data = CupertinoThemeData::new().with_primary_color(CupertinoColors::SYSTEM_RED);
        let copy = data
            .copy_with()
            .with_primary_color(CupertinoColors::SYSTEM_GREEN);
        assert_eq!(copy.primary_color(), CupertinoColors::SYSTEM_GREEN);
        assert_eq!(
            copy.text_theme().action_text_style().color,
            Some(CupertinoColors::SYSTEM_GREEN)
        );
        assert_eq!(copy.no_default().text_theme, None);
        assert_ne!(copy, data);
        assert_eq!(CupertinoThemeData::new(), CupertinoThemeData::default());
    }
}
