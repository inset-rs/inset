# waft-winui/src
Audience: readers familiar with Flutter and Rust; the headings identify the corresponding WinUI code.

WinUI home: `controls/dev/` for the controls, `dxaml/phone/lib/ThemeTransitions.cpp` for the Frame's navigation transitions
Ported against: aa3207e6

## Names that share the app's namespace with `waft::*`

An app globs the facade and a kit together (`use waft::*; use waft_winui::*;`), so a name both
export would be ambiguous. A kit reuses Flutter's type where WinUI's is the same thing, and
prefixes a type of its own, as decided for WaOS's docs:

- `Page` is `FluentPage`, as `FluentApp` and `FluentPageRoute` are; Flutter's widgets library
  already exports `Page`, a route's settings. An app writes `FluentPage::new()`.
- `Slider`, `ColorPicker` and `ColorPickerSlider`'s `Orientation` is Flutter's [`Axis`]:
  horizontal and vertical are exactly it, and `Orientation` is Flutter's landscape/portrait
  media-query type. An app writes `.orientation(Axis::Vertical)`.
- `TableViewColumn`'s `Visibility` is `visible(app)` / `set_visible(app, bool)`: Flutter's
  `Visibility` is a widget, and WinUI's two states are a boolean.
- the checkbox's `CheckState` is internal, so an app never sees it; its `is_checked` is already
  `Option<bool>`, as Flutter's `Checkbox(value:)` is `bool?`, and `None` is indeterminate.

## app.rs → Flutter `material/app.dart` and `cupertino/app.dart`, over WinUI's theme resources

- Change: `FluentApp` takes [`FluentTheme`](crate::FluentTheme) values, each a WinUI [`Theme`](crate::Theme) with an [`AccentPalette`](crate::AccentPalette), where `MaterialApp` takes `ThemeData` and `CupertinoApp` `CupertinoThemeData`.
  Reason: platform — the kit's theme is WinUI's pair of theme dictionaries and the OS accent palette.
  Affect: a theme names light or dark and an accent, rather than a colour scheme; the app puts its pages in a `ThemeScope` with both.

- Change: `FluentApp` installs the Fluent icon font when it mounts, where a WinUI app gets it from its resources and the gallery used to call `install_icon_font` separately.
  Reason: framework — the kit's `FontIcon` and controls draw glyphs from that family, and an app should not have to know it.
  Affect: an app that mounts a `FluentApp` needs no font installation call; text drawn with a bare `WidgetsApp` still needs one.

- Change: a nested `FluentApp`'s theme is chosen from `theme`, `dark_theme` and `theme_mode` and the platform's brightness, and applied by the app's own `ThemeScope`, where Flutter animates between two `ThemeData` values.
  Reason: framework — theme changes are not animated here, as the kit's `ThemeScope` resolves values at build.
  Affect: switching theme mode changes every themed control at once, with no transition.

- Change: `FluentApp` gives its text a default style — the kit's content-control family, WinUI body text's 14 points, and the theme's `TextFillColorPrimary` — where `MaterialApp` gives its own body style and `CupertinoApp` the theme's text theme.
  Reason: platform — WinUI's body text is `BodyTextBlockStyle`, whose family is `ContentControlThemeFontFamily`.
  Affect: a `Text` with no style draws in Selawik at 14 points in the theme's primary text colour, changing with the theme.

- Change: `FluentApp` wraps its pages in a `ScrollConfiguration` with [`FluentScrollBehavior`](crate::FluentScrollBehavior): a two-point `ScrollBarForeground` bar, shown while scrolling, and clamping with no glow or stretch.
  Reason: platform — WinUI's `ScrollViewer` has auto-hiding overlay bars and no overscroll indicator.
  Affect: scrollables show a thin bar in the theme's colour and stop at the edge on every system, where the default behaviour glows on Android and Fuchsia.

- Change: `FluentApp` wraps its pages in `DefaultSelectionStyle` with the accent's `base` at 40% for a selection and the accent for a caret, where `MaterialApp` uses its primary colour.
  Reason: platform — WinUI's selection highlight is the system accent.
  Affect: selecting text or placing a caret uses the app's accent.

- Change: `FluentApp.router` and the high-contrast and system-accent settings are not built; a system accent palette is not read.
  Reason: scope — the router branch waits with Flutter's `router.dart`, high contrast with a host palette, and WinUI's accent with a Windows-only API.
  Affect: an app routes with `home`, `routes`, `on_generate_route` and the other `WidgetsApp` routing fields; the accent stays the palette it was given.

## route.rs → WinUI `NavigationThemeTransition` / `EntranceNavigationTransitionInfo`

- Change: `FluentPageRoute` is a `PageRoute` with its own route data, as `MaterialPageRoute` and `CupertinoPageRoute` are, where Dart's `MaterialPageRoute` extends `PageRouteBuilder`.
  Reason: framework — the kit has no `PageRouteBuilder` subclass; the route holds the data and plays the transition itself.
  Affect: it is pushed as `FluentPageRoute::new(app, builder).settings(...).into_erased()`, with setters for the `PageRoute` options.

- Change: the leaving animation uses `BackNavigatingAway`'s 150 ms and `(0.7, 0.0, 1.0, 0.5)` curve, and the back as a whole takes 450 ms so the revealed page can fade in after it.
  Reason: platform — WinUI's Frame has a storyboard per navigation trigger, and its back runs the leaving and revealed pages' storyboards one after the other.
  Affect: a page leaves in 150 ms, while it entered in 450 ms, and the page below returns over the remaining 300 ms.

- Change: the covered page is its own `build_transitions` half: it fades out over 150 ms as a page enters, and from 150 ms into a back fades in over 300 ms, where Flutter's routes animate the page below through the secondary animation.
  Reason: platform — `NavigationThemeTransition` plays `NavigatingAway` on the page an unload covers, and `BackNavigatingTo` on the one a back reveals.
  Affect: a page below dims as another enters and returns as the top page leaves.

- Change: the page stays 140 effective pixels lower and invisible for 150 ms, then slides up with opacity snapped on at 150 ms, where the WinUI storyboard registers a discrete opacity keyframe at that time.
  Reason: platform — the transition is a XAML storyboard, whose discrete keyframes are steps.
  Affect: a page appears rather than fading in; the movement is what is seen.
