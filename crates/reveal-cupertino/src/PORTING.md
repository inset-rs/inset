# reveal-cupertino/src
Syntax (constructors, setters, `with_*` on data classes) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/cupertino
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- interface_level.rs → interface_level.dart
- form_row.rs → form_row.dart
- activity_indicator.rs → activity_indicator.dart
- form_section.rs → form_section.dart
- focus_halo.rs → cupertino_focus_halo.dart
- app.rs → app.dart
- constants.rs → constants.dart
- theme.rs → theme.dart
- localizations.rs → localizations.dart
- text_field.rs → text_field.dart

## colors.rs → colors.dart

- Change: `CupertinoDynamicColor` is a colour extension carried by painting's `AnyColor`: every `CupertinoColors` entry is an `AnyColor`, and `CupertinoDynamicColor::resolve` takes and returns one.
  Reason: language — dart:ui `Color` is a Copy value and cannot be subclassed (reveal-painting's colors.rs entry).
  Affect: ask `color.extension::<CupertinoDynamicColor>()` where Dart asks `color is CupertinoDynamicColor`.

- Change: `Debug` output leaves off Dart's `resolved by: <widget>` suffix; the resolving context is not stored.
  Reason: language — `Debug` has no `App` to reach the resolving element's widget.
  Affect: `{:?}` prints the colour's label and variants without the suffix.

## text_theme.rs → text_theme.dart

- Change: Dart's private default-text-theme subclass is the constructor `CupertinoTextThemeData::with_defaults`, which stores the theme's label colours in the defaults builder instead of overriding the getters.
  Reason: language — no subclass to override getters in; the builder already applies those colours.
  Affect: the text theme of a resolved `CupertinoThemeData` compares unequal to a fresh one only through those stored colours, which is the answer Dart's `runtimeType` check gives.

## icon_theme_data.rs → icon_theme_data.dart

- Change: `CupertinoIconThemeData::new()` returns widgets' `IconThemeData` with the cupertino `resolve` installed as its resolver; there is no separate type.
  Reason: language — `IconTheme.data` is a value (the widgets entry on `IconThemeData::resolver`).
  Affect: the value `CupertinoIconThemeData::new()` yields is an `IconThemeData`.

## icons.rs → icons.dart

- Change: the `cupertino_icons` 1.0.9 font ships in this crate's `assets/`, and `install_cupertino_icon_font(app)` registers it with the app-wide font collection under the package name `TextStyle::package` asks for.
  Reason: platform — there is no pub dependency, asset bundle or `pubspec.yaml` for the engine to read a font declaration from.
  Affect: call it once at start-up, where a Flutter app adds `cupertino_icons` to `pubspec.yaml`; until it runs an icon glyph has no font to shape from, and the whole font is registered where Flutter's tool subsets it to the icons an app names.

## expansion_tile.rs → expansion_tile.dart

- Change: the header's `CupertinoListTile` is not wrapped in Dart's `Semantics(hint:, onTapHint:)`, so the localised expansion hints are never read.
  Reason: platform — accessibility is deferred.
  Affect: nothing announces the tile's expanded state or what a tap will do.

## debug.rs → debug.dart

- Change: `debug_check_has_cupertino_localizations(app, context)` panics with the error's text; Dart's `describeMissingAncestor` is not appended.
  Reason: language — no `FlutterError` parts; the panic carries the summary, description and hint.
  Affect: the same failure, as a panic.

## button.rs → button.dart

- Change: `Semantics(button: true)` is not in the tree.
  Reason: platform — accessibility is deferred.
  Affect: nothing announces the button.

## list_tile.rs → list_tile.dart

## list_section.rs → list_section.dart

- Change: the constructor assert that a section has children or a header runs at the start of `build`.
  Reason: language — the fluent setters fill those fields after the constructor, so the pair is only complete once the widget builds.
  Affect: an empty section without a header panics on its first build rather than where it is constructed.

## scrollbar.rs → scrollbar.dart

- Change: the thumb-press handlers do not record the press position or compare the release position and velocity against it.
  Reason: platform — `HapticFeedback` is not ported, and that comparison exists only to decide whether to fire it.
  Affect: pressing and releasing the thumb produces no haptic feedback.

## page_scaffold.rs → page_scaffold.dart

- Change: `handleStatusBarTap` is the inherent `CupertinoPageScaffoldState::handle_status_bar_tap(app)`, and nothing calls it.
  Reason: platform — the binding observer has no status-bar-tap callback yet (Deferred).
  Affect: tapping the status bar does not scroll the primary scroll view to the top until the binding raises the event; the method itself behaves as Dart's.

## segmented_control.rs → segmented_control.dart

- Change: `children` is an `IndexMap<T, WidgetRef>` whose key type is `Copy + Eq + Hash + Debug`.
  Reason: language — a Dart `LinkedHashMap` keyed by an arbitrary object is an insertion-ordered map keyed by a hashable value, widgets' `RadioClient<T>` hands the value back through a vtable slot so it must be `Copy`, and `Debug` stands in for the `Object.toString` the segment's focus label reads.
  Affect: a segment key is a `Copy` value (an enum, an integer) rather than any object.

- Change: the `Semantics` wrapper around each segment is not in the tree.
  Reason: platform — accessibility is deferred.
  Affect: nothing announces a segment or which one is selected.

## route.rs → route.dart

- Change: `CupertinoRouteTransitionMixin` is a trait over `PageRoute` with a `CupertinoRouteTransitionMixinData` bag; the trait carries the mixin's `dispose` and `didChangePrevious` overrides, so a leaf writes the three `cupertino_route_transition_mixin_*_overrides!` macros in place of widgets' route-override macros.
  Reason: language — no mixin linearization, and a leaf cannot override one member a macro already wrote for it.
  Affect: a route that mixes this in holds the bag and writes the three macros; leaving out the `Route` one silently drops the previous-title bookkeeping and the notifier's disposal.

- Change: Dart's `route is CupertinoRouteTransitionMixin` is `route.interface::<Rc<dyn CupertinoRouteTransition>>()`, widgets' `Route::interface` query, which every route that writes `cupertino_route_transition_mixin_route_overrides!` answers; `CupertinoRouteTransition` is the mixin on a shared reference (`title`, `previous_title`, `as_route`).
  Reason: language — an erased route cannot be asked whether it implements a trait (widgets' navigator.rs entry).
  Affect: a route that mixes the trait in but hand-writes its `impl Route` must forward `interface` to the mixin, or it contributes no `previousTitle` to the route above it and `canTransitionTo` treats it as an unrelated page route.

- Change: `previous_title` returns the `Handle<ValueNotifier<Option<String>>>` itself, where Dart narrows it to a `ValueListenable`.
  Reason: language — foundation's erased value listenable would hand back a fresh `Rc` per call with no identity (widgets' ticker_provider.rs records the same).
  Affect: a caller can also write the notifier; wrap it in an `Rc` where a `ValueListenable` is wanted.

- Change: `CupertinoModalPopupRoute.buildPage` builds without Dart's `DisplayFeatureSubScreen`, and neither the routes nor the show functions take an `anchorPoint`.
  Reason: platform — the sub-screen widget needs `MediaQuery.displayFeatures` (Deferred; reveal-widgets records the same for `RawDialogRoute`).
  Affect: a popup or dialog is not confined to the display-feature sub-screen closest to an anchor point.

- Change: `CupertinoModalPopupRoute.barrierColor` is stored as an `AnyColor`, and the `ModalRoute` getter hands back its plain `Color`.
  Reason: language — `kCupertinoModalBarrierColor` is a `CupertinoDynamicColor` (colors.rs), and widgets' `ModalRoute::barrier_color` is a plain `Color`.
  Affect: `route.barrier_color(app, Some(color))` takes an `AnyColor`; an unresolved dynamic colour paints its own light value, as Dart's does when the route is pushed directly.

## sheet.rs → sheet.dart

- Change: Dart's "either scrollableBuilder or builder" assert on `CupertinoSheetRoute` runs on the first build.
  Reason: language — neither builder is a required argument, so both arrive through fluent setters (the list_section.rs shape).
  Affect: a route with neither panics the first time it builds its content, where Dart panics at construction.

- Change: `_CupertinoSheetRouteTransitionMixin` is a private trait over `PageRoute` with no bag, and `CupertinoSheetRoute` writes widgets' route-override and accessor macros and then points the six transition members at the trait by hand.
  Reason: language — no mixin linearization; the mixin is private with one implementor, so the forwarders are written out where route.rs exports macros.
  Affect: a second route that mixes the trait in repeats those six forwarders; forgetting one silently falls back to `PageRoute`'s transition.

## dialog.rs → dialog.dart

- Change: `CupertinoActionSheet`'s "at least one of actions, title, message, cancelButton" assert runs in `create_state`.
  Reason: language — the four fields arrive through fluent setters (the list_section.rs shape).
  Affect: an action sheet with none of the four set panics the first time it is built in debug, where Dart panics at construction.

- Change: `_RenderAlertDialogActionsLayout`'s `_debugHasValidConstraints` panics where Dart throws a `FlutterError` the binding catches.
  Reason: language — no catchable error; diagnostics deferred.
  Affect: an unbounded-width actions layout aborts the frame in debug instead of reporting the error and laying out at `constraints.smallest`.

## nav_bar.rs → nav_bar.dart

- Change: `heroTag` is a `HeroTagRef` (an `Rc<dyn HeroTag>`), and the file's default tag is one process-local `Rc` handed out by `default_hero_tag()`, so Dart's identity check against it and its `==` are both `Rc::ptr_eq`.
  Reason: language — a Dart `Object` tag compared with `==` and used as a map key is an `Rc<dyn HeroTag>` here (reveal-widgets records the trait), and a `const` value has no canonical instance to compare identity against.
  Affect: the constructor assert that a custom tag needs `transitionBetweenRoutes` false runs on whichever of the two setters is written second.

- Change: both the `middle` and `large_title` setters exist on the one `CupertinoNavigationBar` type, each asserting the other is unset.
  Reason: language — two Dart constructors of one class are two associated functions returning the same struct, so a field the other constructor forbids can only be guarded where it is written.
  Affect: writing both `middle` and `large_title` panics in debug, where Dart's two constructors make it unsayable.

## Deferred
- text_field.rs: `TextSelectionGestureDetectorBuilder` / `_CupertinoTextFieldSelectionGestureDetectorBuilder`; tap-to-focus is a `GestureDetector` that calls `requestKeyboard`. Trigger: widgets' `text_selection.dart` (the builder is listed there).
- text_field.rs: `_BaselineAlignedStack` / `_RenderBaselineAlignedStack`; the placeholder and the editable share a `Stack`. Trigger: widgets' `SlottedMultiChildRenderObjectWidget`.
- text_field.rs: `cupertinoTextSelectionHandleControls` / `cupertinoDesktopTextSelectionHandleControls` and `CupertinoAdaptiveTextSelectionToolbar`. The default `contextMenuBuilder` returns `SizedBox.shrink` when the system menu is not supported. Trigger: those cupertino files.
- text_field.rs: `CupertinoTextMagnifier` / `_iosMagnifierConfiguration`. Trigger: that cupertino file.
- text_field.rs: `CupertinoSpellCheckSuggestionsToolbar` / `defaultSpellCheckSuggestionsToolbarBuilder`. Trigger: that cupertino file.
- text_field.rs: the `Semantics` wrappers (enabled, onTap, accessibility focus). Trigger: accessibility (do not stub).
- text_field.rs: `strutStyle`. Trigger: painting's `StrutStyle` (valo has no strut).
- text_field.rs: `CupertinoTextFieldState` as `AutofillClient` / `TextSelectionGestureDetectorBuilderDelegate`. Trigger: the gesture builder; `EditableText` is the autofill client until then.
- nav_bar.rs: every `Semantics` wrapper the file carries — the header semantics around the static and sliver bars' large titles and around the persistent bar's middle, and the labelled button semantics around `CupertinoNavigationBarBackButton`'s content, whose label is `CupertinoLocalizations.backButtonLabel`. Trigger: accessibility (do not stub).
- nav_bar.rs: `_LargeTitleNavigationBarSliverDelegate`'s `DiagnosticableTreeMixin`. Trigger: diagnostics.
- route.rs, sheet.rs, nav_bar.rs: a `CurvedAnimation` that a transition state, a delegated-transition function or the sliver navigation bar's state disposes keeps its arena slot, and so does every `_AnimatedEvaluation` a `drive` mints — in the navigation bar, one set per `didChangeDependencies` and per hero flight. Trigger: an arena-slot lifetime for disposed foundation objects (the trigger reveal-widgets records for disposed routes).
- dialog.rs: every accessibility wrapper the file carries — the role-bearing `Semantics` around a dialog's and an action sheet's body, the button `Semantics` around an action's content, the `MergeSemantics` in the alert-dialog button background, the `excludeFromSemantics` on the action-sheet gesture detector, and the `TapSemanticEvent` an action sheet action sends on activation; `CupertinoLocalizations.alertDialogLabel` is read and dropped where the label would go. Trigger: accessibility (do not stub).
- dialog.rs: the `debugCheckHasMediaQuery` assert at the top of `CupertinoActionSheet.build`. Trigger: widgets' `debug.dart`.
- route.rs: `CupertinoPageTransitionsBuilder`. Trigger: widgets' `page_transitions_builder.dart`.
- route.rs: both page routes' `debugLabel`, which appends the settings' name, and `_CupertinoEdgeShadowDecoration`'s `debugFillProperties` and `hashCode`. Trigger: `TransitionRoute::debug_label` taking the `App`; diagnostics.
- route.rs: the `Semantics(scopesRoute:, explicitChildNodes:)` wrapper `CupertinoRouteTransitionMixin.buildPage` returns. Trigger: accessibility (do not stub).
- route.rs: `anchorPoint` on `CupertinoModalPopupRoute` and `CupertinoDialogRoute` and the `DisplayFeatureSubScreen` it feeds, so the two show functions have no `anchor_point` argument. Trigger: `display_feature_sub_screen.dart`, which needs `MediaQuery.displayFeatures`.
- theme.rs: `InheritedCupertinoTheme` as an `InheritedTheme`, so `captureAll` carries it; until then it is a plain inherited widget with `wrap` as an inherent method. Trigger: widgets' `InheritedTheme`.
- colors.rs, interface_level.rs, theme.rs, text_theme.rs, icon_theme_data.rs, page_scaffold.rs: the diagnostics members — `debugFillProperties`, `createCupertinoColorProperty`, and the theme types' `hashCode`. Trigger: diagnostics.
- localizations.rs / debug.rs: `describeMissingAncestor`, which Dart appends to the missing-localizations error; nothing else in either file waits (`debugCheckHasCupertinoLocalizations` is the whole of debug.dart at this commit).
- button.rs: `Semantics(button: true)` and the `TapSemanticEvent`, `debugFillProperties`, and the deprecated `minSize`. Trigger: accessibility; diagnostics.
- expansion_tile.rs: the header's `Semantics(hint:, onTapHint:)` and the `CupertinoLocalizations` hints it reads. Trigger: accessibility (do not stub).
- scrollbar.rs: `HapticFeedback.mediumImpact()` on a thumb press and on a slow release, and with it `_pressStartAxisPosition`. Trigger: `services/haptic_feedback.dart`.
- page_scaffold.rs: the `WidgetsBindingObserver.handleStatusBarTap` callback that would call `CupertinoPageScaffoldState::handle_status_bar_tap`. Trigger: the binding's status-bar tap event.
- segmented_control.rs: the `Semantics` around each segment. Trigger: accessibility (do not stub).
- app.rs: `CupertinoApp.router` and everything only it fills — the five router-only fields, `_usesRouter` and the `WidgetsApp.router` branch of `_buildWidgetApp`. Trigger: `WidgetsApp.router` / `router.dart`, which reveal-widgets defers with the same trigger.
- app.rs: the `DefaultSelectionStyle` between the `CupertinoTheme` and the `HeroControllerScope`, which gives the primary colour at 0.2 opacity as the selection colour and the primary colour as the cursor colour. Trigger: `default_selection_style.dart`.
- app.rs: the three inspector-button builders and the `_CupertinoInspectorButton` they build; `showPerformanceOverlay`, `showSemanticsDebugger` and `debugShowCheckedModeBanner` reach the `WidgetsApp` as Dart passes them. Trigger: `widget_inspector.dart`'s `InspectorButton`, which reveal-widgets defers with `WidgetsApp`'s three builder arguments.
- sheet.rs: `showCupertinoSheet`'s deprecated `pageBuilder`, a pure duplicate of the (also deprecated) `builder`. Trigger: never — port it only if Flutter un-deprecates it.
- sheet.rs: the `filterQuality: FilterQuality.medium` on all three `ScaleTransition`s. Trigger: `MatrixTransition.filterQuality`, which reveal-widgets defers with the same trigger.
