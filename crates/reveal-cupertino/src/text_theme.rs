//! Flutter counterpart: `cupertino/text_theme.dart`.

use reveal_embedder::{FontWeight, TextDecoration};
use reveal_foundation::App;
use reveal_painting::{AnyColor, TextStyle};
use reveal_widgets::BuildContext;

use crate::colors::{CupertinoColors, CupertinoDynamicColor};

// Dart's `const TextStyle`s are functions: a `TextStyle` owns its font family string.

// Please update _TextThemeDefaultsBuilder accordingly after changing the default
// color here, as their implementation depends on the default value of the color
// field.
//
// Values derived from https://developer.apple.com/design/resources/.
fn default_text_style() -> TextStyle {
    TextStyle::new()
        .inherit(false)
        .font_family("CupertinoSystemText")
        .font_size(17.0)
        .letter_spacing(-0.41)
        .color(CupertinoColors::LABEL)
        .decoration(TextDecoration::NONE)
}

// Please update _TextThemeDefaultsBuilder accordingly after changing the default
// color here, as their implementation depends on the default value of the color
// field.
//
// Values derived from https://developer.apple.com/design/resources/.
// See [iOS 17 + iPadOS 17 UI Kit](https://www.figma.com/community/file/1248375255495415511) for details.
fn default_action_text_style() -> TextStyle {
    TextStyle::new()
        .inherit(false)
        .font_family("CupertinoSystemText")
        .font_size(17.0)
        .letter_spacing(-0.41)
        .color(CupertinoColors::ACTIVE_BLUE)
        .decoration(TextDecoration::NONE)
}

// Please update _TextThemeDefaultsBuilder accordingly after changing the default
// color here, as their implementation depends on the default value of the color
// field.
//
// Values derived from https://developer.apple.com/design/resources/.
// See [iOS 17 + iPadOS 17 UI Kit](https://www.figma.com/community/file/1248375255495415511) for details.
fn default_action_small_text_style() -> TextStyle {
    TextStyle::new()
        .inherit(false)
        .font_family("CupertinoSystemText")
        .font_size(15.0)
        .letter_spacing(-0.23)
        .color(CupertinoColors::ACTIVE_BLUE)
        .decoration(TextDecoration::NONE)
}

// Please update _TextThemeDefaultsBuilder accordingly after changing the default
// color here, as their implementation depends on the default value of the color
// field.
//
// Values derived from https://developer.apple.com/design/resources/.
fn default_tab_label_text_style() -> TextStyle {
    TextStyle::new()
        .inherit(false)
        .font_family("CupertinoSystemText")
        .font_size(10.0)
        .font_weight(FontWeight::W500)
        .letter_spacing(-0.24)
        .color(CupertinoColors::INACTIVE_GRAY)
}

fn default_middle_title_text_style() -> TextStyle {
    TextStyle::new()
        .inherit(false)
        .font_family("CupertinoSystemText")
        .font_size(17.0)
        .font_weight(FontWeight::W600)
        .letter_spacing(-0.41)
        .color(CupertinoColors::LABEL)
}

fn default_large_title_text_style() -> TextStyle {
    TextStyle::new()
        .inherit(false)
        .font_family("CupertinoSystemDisplay")
        .font_size(34.0)
        .font_weight(FontWeight::W700)
        .letter_spacing(0.38)
        .color(CupertinoColors::LABEL)
}

// Please update _TextThemeDefaultsBuilder accordingly after changing the default
// color here, as their implementation depends on the default value of the color
// field.
//
// Inspected on iOS 13 simulator with "Debug View Hierarchy".
// Value extracted from off-center labels. Centered labels have a font size of 25pt.
//
// The letterSpacing sourced from iOS 14 simulator screenshots for comparison.
// See also:
//
// * https://github.com/flutter/flutter/pull/65501#discussion_r486557093
fn default_picker_text_style() -> TextStyle {
    TextStyle::new()
        .inherit(false)
        .font_family("CupertinoSystemDisplay")
        .font_size(21.0)
        .font_weight(FontWeight::W400)
        .letter_spacing(-0.6)
        .color(CupertinoColors::LABEL)
}

// Please update _TextThemeDefaultsBuilder accordingly after changing the default
// color here, as their implementation depends on the default value of the color
// field.
//
// Inspected on iOS 13 simulator with "Debug View Hierarchy".
// Value extracted from off-center labels. Centered labels have a font size of 25pt.
fn default_date_time_picker_text_style() -> TextStyle {
    TextStyle::new()
        .inherit(false)
        .font_family("CupertinoSystemDisplay")
        .font_size(21.0)
        .letter_spacing(0.4)
        .font_weight(FontWeight::NORMAL)
        .color(CupertinoColors::LABEL)
}

fn resolve_text_style(
    style: Option<&TextStyle>,
    app: &mut App,
    context: BuildContext,
) -> Option<TextStyle> {
    // This does not resolve the shadow color, foreground, background, etc.
    let style = style?;
    let mut resolved = style.copy_with();
    if let Some(color) = CupertinoDynamicColor::maybe_resolve(style.color.as_ref(), app, context) {
        resolved = resolved.color(color);
    }
    if let Some(background_color) =
        CupertinoDynamicColor::maybe_resolve(style.background_color.as_ref(), app, context)
    {
        resolved = resolved.background_color(background_color);
    }
    if let Some(decoration_color) =
        CupertinoDynamicColor::maybe_resolve(style.decoration_color.as_ref(), app, context)
    {
        resolved = resolved.decoration_color(decoration_color);
    }
    Some(resolved)
}

/// Cupertino typography theme in a `CupertinoThemeData`.
///
/// Dart's named constructor arguments are the `with_*` setters
/// (`CupertinoTextThemeData::new().with_text_style(..)`); the Dart getters keep their names.
#[derive(Clone, Debug, PartialEq)]
pub struct CupertinoTextThemeData {
    defaults: TextThemeDefaultsBuilder,
    primary_color: Option<AnyColor>,
    text_style: Option<TextStyle>,
    action_text_style: Option<TextStyle>,
    action_small_text_style: Option<TextStyle>,
    tab_label_text_style: Option<TextStyle>,
    nav_title_text_style: Option<TextStyle>,
    nav_large_title_text_style: Option<TextStyle>,
    nav_action_text_style: Option<TextStyle>,
    picker_text_style: Option<TextStyle>,
    date_time_picker_text_style: Option<TextStyle>,
}

impl Default for CupertinoTextThemeData {
    fn default() -> CupertinoTextThemeData {
        CupertinoTextThemeData::new()
    }
}

impl CupertinoTextThemeData {
    /// Create a [`CupertinoTextThemeData`].
    ///
    /// The `primary_color` is used to derive TextStyle defaults of other attributes such as
    /// [`nav_action_text_style`](Self::nav_action_text_style) and
    /// [`action_text_style`](Self::action_text_style). It must not be null when either
    /// [`nav_action_text_style`](Self::nav_action_text_style) or
    /// [`action_text_style`](Self::action_text_style) is null. Defaults to
    /// [`CupertinoColors::SYSTEM_BLUE`].
    ///
    /// Other [`TextStyle`] parameters default to default iOS text styles when unspecified.
    pub fn new() -> CupertinoTextThemeData {
        CupertinoTextThemeData::raw(
            TextThemeDefaultsBuilder::new(CupertinoColors::LABEL, CupertinoColors::INACTIVE_GRAY),
            Some(CupertinoColors::SYSTEM_BLUE),
        )
    }

    /// Dart's `_DefaultCupertinoTextThemeData`: a text theme with no text styles explicitly
    /// specified, deriving them from the given colors.
    pub(crate) fn with_defaults(
        primary_color: AnyColor,
        label_color: AnyColor,
        inactive_gray: AnyColor,
    ) -> CupertinoTextThemeData {
        CupertinoTextThemeData::raw(
            TextThemeDefaultsBuilder::new(label_color, inactive_gray),
            Some(primary_color),
        )
    }

    fn raw(
        defaults: TextThemeDefaultsBuilder,
        primary_color: Option<AnyColor>,
    ) -> CupertinoTextThemeData {
        CupertinoTextThemeData {
            defaults,
            primary_color,
            text_style: None,
            action_text_style: None,
            action_small_text_style: None,
            tab_label_text_style: None,
            nav_title_text_style: None,
            nav_large_title_text_style: None,
            nav_action_text_style: None,
            picker_text_style: None,
            date_time_picker_text_style: None,
        }
    }

    fn debug_assert_action_styles_derivable(&self) {
        debug_assert!(
            (self.nav_action_text_style.is_some() && self.action_text_style.is_some())
                || self.primary_color.is_some()
        );
    }

    /// Dart `CupertinoTextThemeData(primaryColor:)`.
    pub fn with_primary_color(
        mut self,
        primary_color: impl Into<AnyColor>,
    ) -> CupertinoTextThemeData {
        self.primary_color = Some(primary_color.into());
        self
    }

    /// Dart `CupertinoTextThemeData(textStyle:)`.
    pub fn with_text_style(mut self, text_style: TextStyle) -> CupertinoTextThemeData {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `CupertinoTextThemeData(actionTextStyle:)`.
    pub fn with_action_text_style(
        mut self,
        action_text_style: TextStyle,
    ) -> CupertinoTextThemeData {
        self.action_text_style = Some(action_text_style);
        self
    }

    /// Dart `CupertinoTextThemeData(actionSmallTextStyle:)`.
    pub fn with_action_small_text_style(
        mut self,
        action_small_text_style: TextStyle,
    ) -> CupertinoTextThemeData {
        self.action_small_text_style = Some(action_small_text_style);
        self
    }

    /// Dart `CupertinoTextThemeData(tabLabelTextStyle:)`.
    pub fn with_tab_label_text_style(
        mut self,
        tab_label_text_style: TextStyle,
    ) -> CupertinoTextThemeData {
        self.tab_label_text_style = Some(tab_label_text_style);
        self
    }

    /// Dart `CupertinoTextThemeData(navTitleTextStyle:)`.
    pub fn with_nav_title_text_style(
        mut self,
        nav_title_text_style: TextStyle,
    ) -> CupertinoTextThemeData {
        self.nav_title_text_style = Some(nav_title_text_style);
        self
    }

    /// Dart `CupertinoTextThemeData(navLargeTitleTextStyle:)`.
    pub fn with_nav_large_title_text_style(
        mut self,
        nav_large_title_text_style: TextStyle,
    ) -> CupertinoTextThemeData {
        self.nav_large_title_text_style = Some(nav_large_title_text_style);
        self
    }

    /// Dart `CupertinoTextThemeData(navActionTextStyle:)`.
    pub fn with_nav_action_text_style(
        mut self,
        nav_action_text_style: TextStyle,
    ) -> CupertinoTextThemeData {
        self.nav_action_text_style = Some(nav_action_text_style);
        self
    }

    /// Dart `CupertinoTextThemeData(pickerTextStyle:)`.
    pub fn with_picker_text_style(
        mut self,
        picker_text_style: TextStyle,
    ) -> CupertinoTextThemeData {
        self.picker_text_style = Some(picker_text_style);
        self
    }

    /// Dart `CupertinoTextThemeData(dateTimePickerTextStyle:)`.
    pub fn with_date_time_picker_text_style(
        mut self,
        date_time_picker_text_style: TextStyle,
    ) -> CupertinoTextThemeData {
        self.date_time_picker_text_style = Some(date_time_picker_text_style);
        self
    }

    /// The [`TextStyle`] of general text content for Cupertino widgets.
    pub fn text_style(&self) -> TextStyle {
        self.text_style
            .clone()
            .unwrap_or_else(|| self.defaults.text_style())
    }

    /// The [`TextStyle`] of interactive text content such as text in a button without
    /// background.
    pub fn action_text_style(&self) -> TextStyle {
        self.debug_assert_action_styles_derivable();
        self.action_text_style
            .clone()
            .unwrap_or_else(|| self.defaults.action_text_style(self.primary_color.as_ref()))
    }

    /// The [`TextStyle`] of interactive text content such as text in a small button.
    pub fn action_small_text_style(&self) -> TextStyle {
        self.action_small_text_style.clone().unwrap_or_else(|| {
            self.defaults
                .action_small_text_style(self.primary_color.as_ref())
        })
    }

    /// The [`TextStyle`] of unselected tabs.
    pub fn tab_label_text_style(&self) -> TextStyle {
        self.tab_label_text_style
            .clone()
            .unwrap_or_else(|| self.defaults.tab_label_text_style())
    }

    /// The [`TextStyle`] of titles in standard navigation bars.
    pub fn nav_title_text_style(&self) -> TextStyle {
        self.nav_title_text_style
            .clone()
            .unwrap_or_else(|| self.defaults.nav_title_text_style())
    }

    /// The [`TextStyle`] of large titles in sliver navigation bars.
    pub fn nav_large_title_text_style(&self) -> TextStyle {
        self.nav_large_title_text_style
            .clone()
            .unwrap_or_else(|| self.defaults.nav_large_title_text_style())
    }

    /// The [`TextStyle`] of interactive text content in navigation bars.
    pub fn nav_action_text_style(&self) -> TextStyle {
        self.debug_assert_action_styles_derivable();
        self.nav_action_text_style.clone().unwrap_or_else(|| {
            self.defaults
                .nav_action_text_style(self.primary_color.as_ref())
        })
    }

    /// The [`TextStyle`] of pickers.
    pub fn picker_text_style(&self) -> TextStyle {
        self.picker_text_style
            .clone()
            .unwrap_or_else(|| self.defaults.picker_text_style())
    }

    /// The [`TextStyle`] of date time pickers.
    pub fn date_time_picker_text_style(&self) -> TextStyle {
        self.date_time_picker_text_style
            .clone()
            .unwrap_or_else(|| self.defaults.date_time_picker_text_style())
    }

    /// Returns a copy of the current [`CupertinoTextThemeData`] with all the colors resolved
    /// against the given [`BuildContext`].
    ///
    /// If any of the `InheritedWidget`s required to resolve this [`CupertinoTextThemeData`]
    /// is not found in `context`, any unresolved `CupertinoDynamicColor`s will use the default
    /// trait value (`Brightness::Light` platform brightness, normal contrast,
    /// `CupertinoUserInterfaceLevelData::Base` elevation level).
    pub fn resolve_from(&self, app: &mut App, context: BuildContext) -> CupertinoTextThemeData {
        CupertinoTextThemeData {
            defaults: self.defaults.resolve_from(app, context),
            primary_color: CupertinoDynamicColor::maybe_resolve(
                self.primary_color.as_ref(),
                app,
                context,
            ),
            text_style: resolve_text_style(self.text_style.as_ref(), app, context),
            action_text_style: resolve_text_style(self.action_text_style.as_ref(), app, context),
            action_small_text_style: resolve_text_style(
                self.action_small_text_style.as_ref(),
                app,
                context,
            ),
            tab_label_text_style: resolve_text_style(
                self.tab_label_text_style.as_ref(),
                app,
                context,
            ),
            nav_title_text_style: resolve_text_style(
                self.nav_title_text_style.as_ref(),
                app,
                context,
            ),
            nav_large_title_text_style: resolve_text_style(
                self.nav_large_title_text_style.as_ref(),
                app,
                context,
            ),
            nav_action_text_style: resolve_text_style(
                self.nav_action_text_style.as_ref(),
                app,
                context,
            ),
            picker_text_style: resolve_text_style(self.picker_text_style.as_ref(), app, context),
            date_time_picker_text_style: resolve_text_style(
                self.date_time_picker_text_style.as_ref(),
                app,
                context,
            ),
        }
    }

    /// Returns a copy of the current [`CupertinoTextThemeData`] instance with specified
    /// overrides.
    ///
    /// Dart's named arguments are the `with_*` setters on the copy
    /// (`data.copy_with().with_text_style(..)`).
    pub fn copy_with(&self) -> CupertinoTextThemeData {
        self.clone()
    }
}

/// Dart's `_TextThemeDefaultsBuilder`: the label colors the default styles are built from.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextThemeDefaultsBuilder {
    label_color: AnyColor,
    inactive_gray_color: AnyColor,
}

impl TextThemeDefaultsBuilder {
    pub(crate) fn new(
        label_color: AnyColor,
        inactive_gray_color: AnyColor,
    ) -> TextThemeDefaultsBuilder {
        TextThemeDefaultsBuilder {
            label_color,
            inactive_gray_color,
        }
    }

    fn apply_label_color(original: TextStyle, color: &AnyColor) -> TextStyle {
        if original.color.as_ref() == Some(color) {
            original
        } else {
            original.copy_with().color(color.clone())
        }
    }

    fn text_style(&self) -> TextStyle {
        Self::apply_label_color(default_text_style(), &self.label_color)
    }

    fn tab_label_text_style(&self) -> TextStyle {
        Self::apply_label_color(default_tab_label_text_style(), &self.inactive_gray_color)
    }

    fn nav_title_text_style(&self) -> TextStyle {
        Self::apply_label_color(default_middle_title_text_style(), &self.label_color)
    }

    fn nav_large_title_text_style(&self) -> TextStyle {
        Self::apply_label_color(default_large_title_text_style(), &self.label_color)
    }

    fn picker_text_style(&self) -> TextStyle {
        Self::apply_label_color(default_picker_text_style(), &self.label_color)
    }

    fn date_time_picker_text_style(&self) -> TextStyle {
        Self::apply_label_color(default_date_time_picker_text_style(), &self.label_color)
    }

    fn action_text_style(&self, primary_color: Option<&AnyColor>) -> TextStyle {
        let style = default_action_text_style();
        match primary_color {
            Some(primary_color) => style.copy_with().color(primary_color.clone()),
            None => style,
        }
    }

    fn action_small_text_style(&self, primary_color: Option<&AnyColor>) -> TextStyle {
        let style = default_action_small_text_style();
        match primary_color {
            Some(primary_color) => style.copy_with().color(primary_color.clone()),
            None => style,
        }
    }

    fn nav_action_text_style(&self, primary_color: Option<&AnyColor>) -> TextStyle {
        self.action_text_style(primary_color)
    }

    fn resolve_from(&self, app: &mut App, context: BuildContext) -> TextThemeDefaultsBuilder {
        let resolved_label_color = CupertinoDynamicColor::resolve(&self.label_color, app, context);
        let resolved_inactive_gray =
            CupertinoDynamicColor::resolve(&self.inactive_gray_color, app, context);
        if resolved_label_color == self.label_color
            && resolved_inactive_gray == CupertinoColors::INACTIVE_GRAY
        {
            self.clone()
        } else {
            TextThemeDefaultsBuilder::new(resolved_label_color, resolved_inactive_gray)
        }
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::Color;

    use super::*;

    #[test]
    fn the_defaults_derive_from_the_label_and_primary_colors() {
        let data = CupertinoTextThemeData::new();
        assert_eq!(data.text_style().color, Some(CupertinoColors::LABEL));
        assert_eq!(data.text_style().font_size, Some(17.0));
        assert!(!data.text_style().inherit);
        assert_eq!(
            data.action_text_style().color,
            Some(CupertinoColors::SYSTEM_BLUE)
        );
        assert_eq!(
            data.tab_label_text_style().color,
            Some(CupertinoColors::INACTIVE_GRAY)
        );
        assert_eq!(data.nav_action_text_style(), data.action_text_style());
        assert_eq!(data, CupertinoTextThemeData::default());
    }

    #[test]
    fn an_explicit_style_wins_and_copy_with_keeps_the_rest() {
        let red = Color::from_argb(255, 255, 0, 0);
        let custom = TextStyle::new().font_size(30.0).color(red);
        let data = CupertinoTextThemeData::new()
            .with_primary_color(CupertinoColors::SYSTEM_GREEN)
            .with_text_style(custom.clone());
        assert_eq!(data.text_style(), custom);
        assert_eq!(
            data.action_text_style().color,
            Some(CupertinoColors::SYSTEM_GREEN)
        );
        let copy = data.copy_with().with_action_text_style(custom.clone());
        assert_eq!(copy.text_style(), custom);
        assert_eq!(copy.action_text_style(), custom);
        assert_ne!(copy, data);
    }

    #[test]
    fn with_defaults_uses_the_given_label_colors() {
        let red = AnyColor::new(Color::from_argb(255, 255, 0, 0));
        let gray = AnyColor::new(Color::from_argb(255, 128, 128, 128));
        let data = CupertinoTextThemeData::with_defaults(
            CupertinoColors::SYSTEM_BLUE,
            red.clone(),
            gray.clone(),
        );
        assert_eq!(data.text_style().color, Some(red.clone()));
        assert_eq!(data.nav_title_text_style().color, Some(red.clone()));
        assert_eq!(data.picker_text_style().color, Some(red));
        assert_eq!(data.tab_label_text_style().color, Some(gray));
    }
}
