# reveal-cupertino/src
Flutter home: packages/flutter/lib/src/cupertino
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- interface_level.rs → interface_level.dart
- list_tile.rs → list_tile.dart
- form_row.rs → form_row.dart
- activity_indicator.rs → activity_indicator.dart
- form_section.rs → form_section.dart
- focus_halo.rs → cupertino_focus_halo.dart
- app.rs → app.dart
- constants.rs → constants.dart
- theme.rs → theme.dart
- localizations.rs → localizations.dart
- text_field.rs → text_field.dart
- desktop_text_selection_toolbar.rs → desktop_text_selection_toolbar.dart
- desktop_text_selection_toolbar_button.rs → desktop_text_selection_toolbar_button.dart
- desktop_text_selection.rs → desktop_text_selection.dart
- text_selection.rs → text_selection.dart
- text_selection_toolbar_button.rs → text_selection_toolbar_button.dart

## adaptive_text_selection_toolbar.rs → adaptive_text_selection_toolbar.dart

- Change: `editable` is given a `TargetPlatform`, and the builders read the one the app reports.
  Reason: language — there is no process-wide `defaultTargetPlatform`.
  Affect: the button set follows the app's platform, and no global override can steer it for one widget.

## colors.rs → colors.dart

- Change: `CupertinoDynamicColor` is a colour extension carried by painting's `AnyColor`, and every `CupertinoColors` entry is an `AnyColor`.
  Reason: language — dart:ui `Color` is a Copy value and cannot be subclassed (reveal-painting's colors.rs entry).
  Affect: once a dynamic colour reaches a plain `Color` it is fixed at the value it was resolved to and stops following brightness, contrast and interface level.

- Change: `Debug` output leaves off Dart's `resolved by: <widget>` suffix; the resolving context is not stored.
  Reason: language — `Debug` has no `App` to reach the resolving element's widget.
  Affect: a printed colour names its label and variants but not who resolved it.

## icon_theme_data.rs → icon_theme_data.dart

- Change: `CupertinoIconThemeData::new()` returns widgets' `IconThemeData` with the cupertino `resolve` installed as its resolver; there is no separate type.
  Reason: language — `IconTheme.data` is a value (the widgets entry on `IconThemeData::resolver`).
  Affect: an icon theme built any other way does not resolve dynamic colours, and there is no cupertino type to test an icon theme against.

## icons.rs → icons.dart

- Change: the `cupertino_icons` font ships in this crate's `assets/`, and `install_cupertino_icon_font(app)` registers it with the app-wide font collection.
  Reason: platform — there is no asset bundle or `pubspec.yaml` for a host to read a font declaration from.
  Affect: every cupertino icon draws blank until that call runs, and the whole font is registered where Flutter's tool subsets it to the icons an app names.

## expansion_tile.rs → expansion_tile.dart

- Change: the header is not wrapped in Dart's `Semantics(hint:, onTapHint:)`.
  Reason: platform — accessibility is deferred.
  Affect: nothing announces the tile's expanded state or what a tap will do.

## button.rs → button.dart

- Change: `Semantics(button: true)` is not in the tree.
  Reason: platform — accessibility is deferred.
  Affect: nothing announces the button.

## list_section.rs → list_section.dart

- Change: the assert that a section has children or a header runs at the start of `build`.
  Reason: language — the fluent setters fill those fields after the constructor, so the pair is only complete once the widget builds.
  Affect: an empty section without a header panics on its first build rather than where it is constructed.

## scrollbar.rs → scrollbar.dart

- Change: the thumb-press handlers do not record the press position or compare the release position and velocity against it.
  Reason: platform — `HapticFeedback` is not ported, and that comparison exists only to decide whether to fire it.
  Affect: pressing and releasing the thumb produces no haptic feedback.

## page_scaffold.rs → page_scaffold.dart

- Change: `handle_status_bar_tap` is an inherent method that nothing calls.
  Reason: platform — the binding observer has no status-bar-tap callback yet (Deferred).
  Affect: tapping the status bar does not scroll the primary scroll view to the top.

## segmented_control.rs → segmented_control.dart

- Change: `children` is keyed by a `Copy + Eq + Hash + Debug` value where Dart keys it by any object.
  Reason: language — the key travels back to the caller through a vtable slot, and `Debug` stands in for the `Object.toString` a segment's focus label reads.
  Affect: a segment is keyed by an enum or an integer; a key that is not a plain value has no way in.

- Change: the `Semantics` wrapper around each segment is not in the tree.
  Reason: platform — accessibility is deferred.
  Affect: nothing announces a segment or which one is selected.

## route.rs → route.dart

- Change: `CupertinoRouteTransitionMixin` carries its `dispose` and `didChangePrevious` overrides itself, so a leaf route writes the three `cupertino_route_transition_mixin_*_overrides!` macros in place of widgets' route-override macros.
  Reason: language — no mixin linearization, and a leaf cannot override one member a macro already wrote for it.
  Affect: a route that leaves out the `Route` macro loses the previous-title bookkeeping and never disposes the notifier, without saying so.

- Change: Dart's `route is CupertinoRouteTransitionMixin` is a `Route::interface` query that the mixin's macro answers.
  Reason: language — an erased route cannot be asked whether it implements a trait (widgets' navigator.rs entry).
  Affect: a route that hand-writes `impl Route` and forgets to forward `interface` contributes no previous title to the route above it, and `can_transition_to` treats it as an unrelated page route.

- Change: `previous_title` hands back the notifier itself where Dart narrows it to a `ValueListenable`.
  Reason: language — foundation's erased value listenable would hand back a fresh object per call with no identity.
  Affect: a caller can write the previous title as well as read it.

- Change: `CupertinoModalPopupRoute.buildPage` builds without Dart's `DisplayFeatureSubScreen`, and neither the routes nor the show functions take an `anchorPoint`.
  Reason: platform — the sub-screen widget needs `MediaQuery.displayFeatures` (Deferred).
  Affect: a popup or dialog spans a folded or hinged display instead of the sub-screen nearest an anchor point.

- Change: `CupertinoModalPopupRoute` holds its barrier colour as an `AnyColor` and the `ModalRoute` getter hands back a plain `Color`.
  Reason: language — `kCupertinoModalBarrierColor` is a `CupertinoDynamicColor` (colors.rs) and widgets' `ModalRoute::barrier_color` is a plain `Color`.
  Affect: a barrier whose colour was never resolved paints its light value, as Dart's does when the route is pushed outside a `CupertinoTheme`.

## sheet.rs → sheet.dart

- Change: Dart's "either scrollableBuilder or builder" assert on `CupertinoSheetRoute` runs on the first build.
  Reason: language — neither builder is a required argument, so both arrive through fluent setters.
  Affect: a route with neither panics the first time it builds its content, where Dart panics at construction.

- Change: `CupertinoSheetRoute` points its six transition members at the private sheet-transition trait by hand instead of through macros.
  Reason: language — no mixin linearization, and the trait is private with one implementor.
  Affect: a second route that mixes the trait in repeats the six forwarders, and a missing one falls back to `PageRoute`'s transition without complaint.

## dialog.rs → dialog.dart

- Change: `CupertinoActionSheet`'s "at least one of actions, title, message, cancelButton" assert runs in `create_state`.
  Reason: language — the four fields arrive through fluent setters.
  Affect: an action sheet with none of the four set panics the first time it is built in debug, where Dart panics at construction.

- Change: the alert-dialog actions layout panics on invalid constraints where Dart throws a `FlutterError` the binding catches.
  Reason: language — no catchable error; diagnostics deferred.
  Affect: an unbounded-width actions layout aborts the frame in debug instead of reporting the error and laying out at `constraints.smallest`.

## nav_bar.rs → nav_bar.dart

- Change: `heroTag` is a shared `HeroTagRef`, and the file's default tag is one process-local instance, so Dart's identity check against it and its `==` are both pointer equality.
  Reason: language — a `const` tag value has no canonical instance to compare identity against.
  Affect: the assert that a custom tag needs `transitionBetweenRoutes` false fires on whichever of the two setters is written second.

- Change: both the `middle` and `large_title` setters exist on the one `CupertinoNavigationBar` type, each asserting the other is unset.
  Reason: language — two Dart constructors of one class are two associated functions returning the same struct, so a field the other constructor forbids can only be guarded where it is written.
  Affect: writing both `middle` and `large_title` panics in debug, where Dart's two constructors make it unsayable.

## Deferred
- text_field.rs: the placeholder and the editable sharing a `Stack` with baseline alignment. Trigger: widgets' `SlottedMultiChildRenderObjectWidget`.
- text_field.rs: `CupertinoTextMagnifier` and its magnifier configuration. Trigger: that cupertino file.
- text_field.rs: `CupertinoSpellCheckSuggestionsToolbar` and the default builder that shows it. Trigger: that cupertino file.
- text_field.rs: the `Semantics` wrappers. Trigger: accessibility (do not stub).
- text_field.rs: `strutStyle`. Trigger: painting's `StrutStyle` (valo has no strut).
- text_field.rs: `CupertinoTextFieldState` as `AutofillClient`; `EditableText` is the autofill client until then. Trigger: autofill.
- adaptive_text_selection_toolbar.rs: `CupertinoAdaptiveTextSelectionToolbar.selectable`. Trigger: `SelectableRegion`.
- nav_bar.rs: every `Semantics` wrapper the file carries, including the back button's localised label. Trigger: accessibility (do not stub).
- nav_bar.rs: the large-title sliver delegate's `DiagnosticableTreeMixin`. Trigger: diagnostics.
- route.rs, sheet.rs, nav_bar.rs: a disposed `CurvedAnimation` or driven animation keeps its arena slot. Trigger: an arena-slot lifetime for disposed foundation objects.
- route.rs: `CupertinoPageTransitionsBuilder`. Trigger: widgets' `page_transitions_builder.dart`.
- route.rs: the page routes' `debugLabel` and the edge shadow decoration's diagnostics and `hashCode`. Trigger: `TransitionRoute::debug_label` taking the `App`; diagnostics.
- route.rs: the `Semantics(scopesRoute:, explicitChildNodes:)` wrapper around a route's page. Trigger: accessibility (do not stub).
- route.rs: `anchorPoint` on `CupertinoModalPopupRoute` and `CupertinoDialogRoute`, and the `DisplayFeatureSubScreen` it feeds. Trigger: `display_feature_sub_screen.dart`, which needs `MediaQuery.displayFeatures`.
- sheet.rs: `showCupertinoSheet`'s deprecated `pageBuilder`, a duplicate of the deprecated `builder`. Trigger: never — port it only if Flutter un-deprecates it.
- sheet.rs: the `filterQuality` on the three `ScaleTransition`s. Trigger: `MatrixTransition.filterQuality`.
- dialog.rs: every accessibility wrapper the file carries, including the alert-dialog label that is read and dropped. Trigger: accessibility (do not stub).
- dialog.rs: the `debugCheckHasMediaQuery` assert in `CupertinoActionSheet.build`. Trigger: widgets' `debug.dart`.
- theme.rs: `InheritedCupertinoTheme` as an `InheritedTheme`, so `captureAll` carries it; until then `wrap` is an inherent method. Trigger: widgets' `InheritedTheme`.
- colors.rs, interface_level.rs, theme.rs, text_theme.rs, icon_theme_data.rs, page_scaffold.rs: the diagnostics members and the theme types' `hashCode`. Trigger: diagnostics.
- localizations.rs, debug.rs: `describeMissingAncestor`, which Dart appends to the missing-localizations error. Trigger: diagnostics.
- button.rs: the button `Semantics` and tap event, the diagnostics, and the deprecated `minSize`. Trigger: accessibility; diagnostics.
- expansion_tile.rs: the header's `Semantics(hint:, onTapHint:)` and the localised hints it reads. Trigger: accessibility (do not stub).
- scrollbar.rs: the haptic feedback on a thumb press and on a slow release. Trigger: `services/haptic_feedback.dart`.
- page_scaffold.rs: the binding observer's status-bar tap callback. Trigger: the binding's status-bar tap event.
- segmented_control.rs: the `Semantics` around each segment. Trigger: accessibility (do not stub).
- app.rs: `CupertinoApp.router` and everything only it fills. Trigger: `WidgetsApp.router` / `router.dart`.
- app.rs: the `DefaultSelectionStyle` between the `CupertinoTheme` and the `HeroControllerScope`. Trigger: `default_selection_style.dart`.
- app.rs: the three inspector-button builders and the button they build; the three debug flags reach the `WidgetsApp` as Dart passes them. Trigger: `widget_inspector.dart`'s `InspectorButton`.
