# waft-winui/src
Audience: readers familiar with Flutter and Rust; the headings identify the corresponding WinUI code.

WinUI home: `controls/dev/` for the controls, `dxaml/phone/lib/ThemeTransitions.cpp` for the Frame's navigation transitions
Ported against: aa3207e6

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

- Change: `FluentApp.router` and the high-contrast and system-accent settings are not built; a system accent palette is not read.
  Reason: scope — the router branch waits with Flutter's `router.dart`, high contrast with a host palette, and WinUI's accent with a Windows-only API.
  Affect: an app routes with `home`, `routes`, `on_generate_route` and the other `WidgetsApp` routing fields; the accent stays the palette it was given.

## route.rs → WinUI `NavigationThemeTransition` / `EntranceNavigationTransitionInfo`

- Change: `FluentPageRoute::new` answers a `PageRouteBuilder` configured with the entrance, not a route type of its own as `MaterialPageRoute` is.
  Reason: framework — every part of the transition is a `PageRouteBuilder` callback, and its options stay at hand.
  Affect: a route is pushed as `FluentPageRoute::new(app, builder).settings(...).into_erased()`, and a kit app returns it from `WidgetsApp::page_route_builder`.

- Change: the leaving animation uses `BackNavigatingAway`'s 150 ms and `(0.7, 0.0, 1.0, 0.5)` curve, while Flutter reverses the entering animation over the reverse duration.
  Reason: platform — WinUI's Frame has a storyboard per navigation trigger, and its out storyboard is shorter than its in one.
  Affect: a page leaves in 150 ms, while it entered in 450 ms.

- Change: the page stays 140 effective pixels lower and invisible for 150 ms, then slides up with opacity snapped on at 150 ms, where the WinUI storyboard registers a discrete opacity keyframe at that time.
  Reason: platform — the transition is a XAML storyboard, whose discrete keyframes are steps.
  Affect: a page appears rather than fading in; the movement is what is seen.
