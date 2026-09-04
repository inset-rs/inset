# reveal-cupertino/src
Syntax (constructors, setters, `with_*` on data classes) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/cupertino
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## colors.rs → colors.dart

- Change: `CupertinoDynamicColor` is a `ColorExtension` carried by `AnyColor`; every `CupertinoColors` entry is an `AnyColor`, and `CupertinoDynamicColor::resolve(&AnyColor, app, context)` returns an `AnyColor` (a resolved instance is `Rc`-shared; a table entry is `&'static`).
  Reason: language — see reveal-painting `## colors.rs`: dart:ui `Color` is a Copy value and cannot be subclassed.
  Affect: `CupertinoDynamicColor::resolve(&color, app, context)` where Dart writes `CupertinoDynamicColor.resolve(color, context)`; `color.extension::<CupertinoDynamicColor>()` is `color is CupertinoDynamicColor`; a `CupertinoDynamicColor` value becomes a color with `.into_any()` / `.to_any()` / `AnyColor::from`.

- Change: `Debug` drops Dart's `resolved by: <widget>` suffix; `_debugResolveContext` is not stored.
  Reason: language — `Debug` has no `App` to reach the resolving element's widget; the field is diagnostics only.
  Affect: `{:?}` prints `label(*color = ..*, darkColor = .., …)` without the suffix.

## constants.rs → constants.dart

- Change: `kCupertinoButtonPadding`, `kCupertinoButtonSizeBorderRadius`, `kCupertinoButtonMinSize` are `const fn k_cupertino_button_*(size: CupertinoButtonSize)` lookups.
  Reason: language — no `const` map; the enum is exhaustive so the lookup is total.
  Affect: write `k_cupertino_button_padding(size)` where Dart writes `kCupertinoButtonPadding[size]!`; the `?? …` / `!` on the Dart call sites are no-ops.

## interface_level.rs → interface_level.dart

- Identical.

## theme.rs → theme.dart

- Change: `CupertinoThemeData` holds its construction-time specifications as a `NoDefaultCupertinoThemeData` (Dart's superclass) and adds the defaults in its getters.
  Reason: language — no inheritance.
  Affect: `CupertinoThemeData::raw(NoDefaultCupertinoThemeData { .. })` or `new()` + `with_*`.

- Change: `InheritedCupertinoTheme` is a plain inherited widget with `wrap` as an inherent method.
  Reason: platform — widgets' `InheritedTheme` (`captureAll`) waits.
  Affect: none until `InheritedTheme` lands.

- Identical: `CupertinoTheme` (`of`, `brightness_of`, `maybe_brightness_of`, the implied `IconTheme`), `resolve_from`, `no_default`, both `==` (including `NoDefaultCupertinoThemeData.==` leaving out `selectionHandleColor`).

## text_theme.rs → text_theme.dart

- Change: `_DefaultCupertinoTextThemeData` is `CupertinoTextThemeData::with_defaults(primary, label, inactive_gray)`, which stores the theme's label colors in the defaults builder instead of overriding the getters.
  Reason: language — no subclass to override getters in; the builder already applies those colors.
  Affect: the derived text theme of a resolved `CupertinoThemeData` compares unequal to a fresh one only through those stored colors, as Dart's `runtimeType` check makes it.

## icon_theme_data.rs → icon_theme_data.dart

- Change: `CupertinoIconThemeData::new()` returns a widgets `IconThemeData` whose `resolve` override is `CupertinoIconThemeData::resolve`.
  Reason: language — `IconTheme.data` is a value; see the widgets entry on `IconThemeData::resolver`.
  Affect: `CupertinoIconThemeData::new().color(..)`.

## button.rs → button.dart

- Change: `VoidCallback` is foundation's `Listener`; the deprecated `minSize` is omitted.
  Reason: language — one callback type per signature.
  Affect: `CupertinoButton::filled(child, Some(Listener::new(|app| ..)))`.

- Change: the button wraps `RawGestureDetector` directly; `FocusableActionDetector` and `Semantics(button: true)` are not in the tree, so `focus_color`, `focus_node`, `on_focus_change`, and `autofocus` have no effect (`focus_node` / `on_focus_change` / `autofocus` are not fields yet) and the focus outline never paints.
  Reason: platform — the focus system, actions, and accessibility wait.
  Affect: keyboard activation and the focus highlight are absent; press, fade, tap-move slop, long press, and the theme colours work.

- Change: `_opacityTween` is created in `init_state` (a state is created without the `App`), and `dispose` destroys the controller and tweens after disposing them.
  Reason: language — arena objects need the `App`; no collector.
  Affect: none.

- Identical: `tap_move_slop`, `_handleTapDown` / `Up` / `Cancel` / `Move` (with `RenderBox.globalToLocal` and the inflated paint bounds), `_animate` (`when_complete` is `TickerFuture.then`), the build's colour, text style, icon theme, cursor, and gesture wiring.

## Deferred
- theme.rs / text_theme.rs / icon_theme_data.rs: `debugFillProperties`, `createCupertinoColorProperty`, `hashCode`. Trigger: diagnostics.
- colors.rs / interface_level.rs: `createCupertinoColorProperty`, `debugFillProperties`. Trigger: `diagnostics.dart`.
- button.rs: `FocusableActionDetector` (focus node, autofocus, `onFocusChange`, `onShowFocusHighlight`, the `ActivateIntent` action map), `Semantics(button: true)` and `TapSemanticEvent`, `debugFillProperties`, the deprecated `minSize`. Trigger: the focus system and actions; accessibility; diagnostics.
