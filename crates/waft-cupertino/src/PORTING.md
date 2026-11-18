# waft-cupertino/src
Flutter home: flutter/packages: packages/cupertino_ui/lib/src
Ported against: 4765bd950d31784578608cfbb0c723453cc49216

## Identical

- interface_level.rs → interface_level.dart
- list_tile.rs → list_tile.dart
- form_row.rs → form_row.dart
- activity_indicator.rs → activity_indicator.dart
- form_section.rs → form_section.dart
- focus_halo.rs → cupertino_focus_halo.dart
- constants.rs → constants.dart
- localizations.rs → localizations.dart
- text_field.rs → text_field.dart
- desktop_text_selection_toolbar.rs → desktop_text_selection_toolbar.dart
- desktop_text_selection_toolbar_button.rs → desktop_text_selection_toolbar_button.dart
- desktop_text_selection.rs → desktop_text_selection.dart
- text_selection.rs → text_selection.dart
- text_selection_toolbar_button.rs → text_selection_toolbar_button.dart
- thumb_painter.rs → thumb_painter.dart
- magnifier.rs → magnifier.dart
- radio.rs → radio.dart
- spell_check_suggestions_toolbar.rs → spell_check_suggestions_toolbar.dart
- search_field.rs → search_field.dart

## show_* functions → their .dart files
- Change: `show_cupertino_modal_popup`, `show_cupertino_dialog` and `show_cupertino_sheet` are builders — `ShowCupertinoModalPopup::new(builder)`, `ShowCupertinoDialog::new(builder)` and `ShowCupertinoSheet::new()`, with a setter per optional Dart argument — whose `show(app, context)` runs the body the free function ran.
  Reason: language — Rust has no named optional arguments, so the required arguments stay in `new` and every optional one is a fluent setter.
  Affect: a call site writes `ShowCupertinoDialog::new(builder).barrier_dismissible(true).show(app, context)` where Dart writes named arguments, and the free functions no longer exist.

## app.rs → app.dart

- Change: the overlay style `CupertinoApp` sets from its brightness is set for the view it is built in (`View::maybe_of`), and for none outside a view.
  Reason: platform — the overlay style is a view's (waft-services' system_chrome.rs entry).
  Affect: two `CupertinoApp`s in two views, such as two Android tasks of one app, each style their own view's bars.

## adaptive_text_selection_toolbar.rs → adaptive_text_selection_toolbar.dart

- Change: `editable` is given a `TargetPlatform`, and the builders read the one the app reports.
  Reason: language — there is no process-wide `defaultTargetPlatform`.
  Affect: the button set follows the app's platform, and no global override can steer it for one widget.

## colors.rs → colors.dart

- Change: `CupertinoDynamicColor` is a colour extension carried by painting's `AnyColor`, and every `CupertinoColors` entry is an `AnyColor`.
  Reason: language — dart:ui `Color` is a Copy value and cannot be subclassed (waft-painting's colors.rs entry).
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

## list_section.rs → list_section.dart

- Change: Dart's assert that a section has children or a header runs when the widget is turned into a `WidgetRef` (`into_widget`).
  Reason: language — the fluent setters fill those fields after the constructor, so the pair is only complete once the chain ends.
  Affect: an empty section without a header panics where it is handed to the tree, not where it is constructed.

## page_scaffold.rs → page_scaffold.dart

- Change: `handle_status_bar_tap` is an inherent method that nothing calls.
  Reason: platform — the binding observer has no status-bar-tap callback yet (Deferred).
  Affect: tapping the status bar does not scroll the primary scroll view to the top.

## segmented_control.rs → segmented_control.dart

- Change: `children` is keyed by a `Copy + Eq + Hash + Debug` value where Dart keys it by any object.
  Reason: language — the key travels back to the caller through a vtable slot, and `Debug` stands in for the `Object.toString` a segment's focus label reads.
  Affect: a segment is keyed by an enum or an integer; a key that is not a plain value has no way in.

## switch.rs → switch.dart

- Change: `activeThumbImage`, `inactiveThumbImage` and their two error listeners are not on `CupertinoSwitch`; the thumb is its colour and, when set, its `thumb_icon`.
  Reason: platform — painting's `DecorationImage`, which the thumb's `ShapeDecoration` paints them with, is deferred (waft-painting's PORTING.md).
  Affect: a switch cannot show a picture on its thumb.

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

- Change: `CupertinoDialogRoute`'s scale tween is a `ValueTween` constant, where Dart's is a `static final` `Tween`.
  Reason: language — an arena tween held by a `static` would never be freed (the porting skill's value-tweens pattern).
  Affect: none beyond the tween's type.

## sheet.rs → sheet.dart

- Change: Dart's "either scrollableBuilder or builder" assert on `CupertinoSheetRoute` runs on the first build.
  Reason: language — neither builder is a required argument, so both arrive through fluent setters.
  Affect: a route with neither panics the first time it builds its content, where Dart panics at construction.

- Change: `CupertinoSheetRoute` points its six transition members at the private sheet-transition trait by hand instead of through macros.
  Reason: language — no mixin linearization, and the trait is private with one implementor.
  Affect: a second route that mixes the trait in repeats the six forwarders, and a missing one falls back to `PageRoute`'s transition without complaint.

## dialog.rs → dialog.dart

- Change: `CupertinoActionSheet`'s "at least one of actions, title, message, cancelButton" constructor assert runs when the widget is turned into a `WidgetRef` (`into_widget`).
  Reason: language — the four fields arrive through fluent setters.
  Affect: an action sheet with none of the four set panics in debug where it is handed to the tree, not where it is constructed.

- Change: the alert-dialog actions layout panics on invalid constraints where Dart throws a `FlutterError` the binding catches.
  Reason: language — no catchable error; diagnostics deferred.
  Affect: an unbounded-width actions layout aborts the frame in debug instead of reporting the error and laying out at `constraints.smallest`.

## nav_bar.rs → nav_bar.dart

- Change: `heroTag` is a shared `Rc<dyn AnyEq>`, and the file's default tag is one process-local instance, so Dart's identity check against it and its `==` are both pointer equality.
  Reason: language — a `const` tag value has no canonical instance to compare identity against.
  Affect: the assert that a custom tag needs `transitionBetweenRoutes` false fires on whichever of the two setters is written second.

- Change: both the `middle` and `large_title` setters exist on the one `CupertinoNavigationBar` type, each asserting the other is unset.
  Reason: language — two Dart constructors of one class are two associated functions returning the same struct, so a field the other constructor forbids can only be guarded where it is written.
  Affect: writing both `middle` and `large_title` panics in debug, where Dart's two constructors make it unsayable.

## checkbox.rs → checkbox.dart

- Change: Dart's `tristate || value != null` constructor assert runs when the widget is turned into a `WidgetRef` (`into_widget`).
  Reason: language — `tristate` arrives through a fluent setter after `new`, so the pair is only complete once the chain ends.
  Affect: a checkbox that is not tristate and has no value panics where it is handed to the tree, not where it is constructed.

## slider.rs → slider.dart

- Change: Dart's `value >= min && value <= max` constructor assert runs when the widget is turned into a `WidgetRef` (`into_widget`); `divisions > 0` is asserted in the `divisions` setter.
  Reason: language — `min` and `max` arrive through fluent setters after `new`.
  Affect: an out-of-range value panics where the slider is handed to the tree, not where it is constructed.

## refresh.rs → refresh.dart

- Change: Dart's `refreshTriggerPullDistance >= refreshIndicatorExtent` constructor assert runs when the widget is turned into a `WidgetRef` (`into_widget`); the single-field asserts run in their setters.
  Reason: language — both distances arrive through fluent setters, so the pair is only complete once the chain ends.
  Affect: a control whose indicator extent is larger than its trigger distance panics in debug where it is handed to the tree, not where it is constructed.

## theme.rs → theme.dart

- Change: a `CupertinoThemeData` subclass is a `CupertinoThemeDataOverrides` the theme data holds (`CupertinoThemeData::raw_with_overrides`), which answers `brightness`, `primary_color`, `primary_contrasting_color`, `scaffold_background_color` and `resolve_from` in place of the base's bodies.
  Reason: language — Rust cannot subclass a value type or override its inherent getters.
  Affect: a subclass cannot override `text_theme`, `bar_background_color`, `selection_handle_color`, `apply_theme_to_all`, `no_default`, `copy_with` or `==`; `copy_with` and the `with_*` setters keep the subclass.

## Deferred
- checkbox.rs: the deprecated `inactiveColor`, which nothing reads. Trigger: Flutter un-deprecating it.
- slider.rs: `describeSemanticsConfiguration`, `_increaseAction`, `_decreaseAction`, `_semanticActionUnit`, `_kAdjustmentUnit`, and the setters' `markNeedsSemanticsUpdate`. Trigger: the semantics tree.
- text_field.rs: the placeholder and the editable sharing a `Stack` with baseline alignment. Trigger: the next edit of text_field.rs (widgets' `SlottedMultiChildRenderObjectWidget` exists).
- text_field.rs: `CupertinoTextField.defaultSpellCheckSuggestionsToolbarBuilder` and its use as the spell-check configuration's fallback. Trigger: wiring spell_check_suggestions_toolbar.rs into text_field.rs.
- spell_check_suggestions_toolbar.rs: nothing gives `EditableTextState` spell-check results, so `build_button_items` answers `None` and no field shows the toolbar. Trigger: widgets' spell-check fetch.
- text_field.rs: `CupertinoTextFieldState` as `AutofillClient`; `EditableText` is the autofill client until then. Trigger: autofill.
- adaptive_text_selection_toolbar.rs: `CupertinoAdaptiveTextSelectionToolbar.selectable`. Trigger: `SelectableRegion`.
- nav_bar.rs: the large-title sliver delegate's `DiagnosticableTreeMixin`. Trigger: diagnostics.
- route.rs: the page routes' `debugLabel` and the edge shadow decoration's diagnostics and `hashCode`. Trigger: `TransitionRoute::debug_label` taking the `App`; diagnostics.
- route.rs: `anchorPoint` on `CupertinoModalPopupRoute` and `CupertinoDialogRoute`, and the `DisplayFeatureSubScreen` it feeds. Trigger: `display_feature_sub_screen.dart`, which needs `MediaQuery.displayFeatures`.
- sheet.rs: `showCupertinoSheet`'s deprecated `pageBuilder`, a duplicate of the deprecated `builder`. Trigger: never — port it only if Flutter un-deprecates it.
- sheet.rs: the `filterQuality` on the three `ScaleTransition`s, and `hasPlatformViews` on `showCupertinoSheet`, `CupertinoSheetTransition`, `CupertinoSheetRoute` and `delegateTransition`, which turns that filter quality off and makes the route offer a second delegated-transition function (`_delegateTransitionWithPlatformViews`), so that a sheet with platform views and one without do not compare equal. Trigger: `MatrixTransition.filterQuality`.
- dialog.rs: the `sendSemanticsEvent(TapSemanticEvent())` calls and the gesture detector's `excludeFromSemantics`. Trigger: the semantics tree.
- dialog.rs: the `debugCheckHasMediaQuery` assert in `CupertinoActionSheet.build`. Trigger: the next edit of dialog.rs (widgets' `debug_check_has_media_query` exists).
- colors.rs, interface_level.rs, theme.rs, text_theme.rs, icon_theme_data.rs, page_scaffold.rs: the diagnostics members and the theme types' `hashCode`. Trigger: diagnostics.
- localizations.rs, debug.rs: `describeMissingAncestor`, which Dart appends to the missing-localizations error. Trigger: diagnostics.
- interface_level.rs: the 'The context used was' lines print the widget's type where Dart describes the element. Trigger: diagnostics.
- button.rs: the tap's `sendSemanticsEvent`, the diagnostics, and the deprecated `minSize`. Trigger: the semantics tree; diagnostics.
- scrollbar.rs: the haptic feedback on a thumb press and on a slow release, with the press position and release comparison that decide it. Trigger: the next edit of scrollbar.rs (services' `HapticFeedback` exists).
- page_scaffold.rs: the binding observer's status-bar tap callback. Trigger: the binding's status-bar tap event.
- switch.rs: the gesture detector's `excludeFromSemantics`. Trigger: the semantics tree.
- switch.rs: `activeThumbImage`, `inactiveThumbImage`, `onActiveThumbImageError` and `onInactiveThumbImageError`, and the painter's cache of them. Trigger: painting's `DecorationImage` and the `image` field on `ShapeDecoration`.
- switch.rs: the deprecated `activeColor` and `trackColor`, Dart's aliases of `activeTrackColor` and `inactiveTrackColor`. Trigger: never — port them only if Flutter un-deprecates them.
- search_field.rs: the three tests of the field resizing under a scroll (its icons and placeholder fading, its top padding moving up), which the field's scroll handling ports. Trigger: widgets' `SliverResizingHeader`.
- app.rs: `CupertinoApp.router` and everything only it fills. Trigger: `WidgetsApp.router` / `router.dart`.
- app.rs: the three inspector-button builders and the button they build; the three debug flags reach the `WidgetsApp` as Dart passes them. Trigger: `widget_inspector.dart`'s `InspectorButton`.
