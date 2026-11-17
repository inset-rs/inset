# waft-material/src/compose
Compose home: compose/material3/adaptive (adaptive, adaptive-layout, adaptive-navigation3) and compose/material3/material3-adaptive-navigation-suite
Ported against: b8f12c706099a47e1e44d9772d03e14a4ae37146

## window_size_class.rs → androidx.window's WindowSizeClass
- Change: `WindowSizeClass::of` and `maybe_of` read the nearest `MediaQuery`'s size through the `App`, where Compose reads `currentWindowAdaptiveInfoV2()`.
  Reason: framework — Flutter's window size is a `MediaQuery` a widget reads with its context, not an ambient Compose value.
  Affect: a widget calls `WindowSizeClass::of(app, context)` and rebuilds when `MediaQuery` changes; there is no scope widget.

## pane_scaffold_directive.rs → PaneScaffoldDirective.kt
- Change: `PaneScaffoldDirective` has no `excluded_bounds` or `should_auto_focus_current_destination`, and its constructors ignore posture, tabletop and hinges.
  Reason: framework — Flutter's `MediaQuery` carries no posture or hinges, and the embedders do not report them yet.
  Affect: a folded or tabletop window gets Compose's ordinary layout instead of keeping content off the hinge.

## three_pane_scaffold_value.rs → ThreePaneScaffoldValue.kt
- Change: `ThreePaneScaffoldValue` is only its three pane values; it carries no current destination, and `AdaptStrategy` and `PaneAdaptedValue` have no `Levitate`.
  Reason: framework — levitated panes would be a modal layer Flutter's pane scaffold does not have, and the value is computed by the app's scene rather than stored with a navigator.
  Affect: `ThreePaneScaffoldValue::calculate` takes the entries and returns the three values, and no pane is shown floating above the others.

## three_pane_scene_strategy.rs → ListDetailSceneStrategy.kt, SupportingPaneSceneStrategy.kt, ThreePaneScaffoldScene.kt
- Change: `ThreePaneSceneStrategy::calculate_scene` runs over the app's own `Option<ThreePaneScaffoldDestinationItem<T>>` slice and answers absolute back-stack indices, where Compose's strategy runs over Navigation 3 entries and a scene keeps those entries.
  Reason: framework — Flutter's Navigator 2.0 shows an app-owned `pages` list; the app owns the back stack and the scene only says which trailing entries share one page.
  Affect: an app maps its entries to destinations, asks for a scene, and builds one page per entry before it plus one page holding the scaffold.

## three_pane_scaffold.rs → ThreePaneScaffold.kt, Pane.kt, PaneMotion.kt
- Change: pane bounds move with `AnimatedPositioned` and panes fade with `AnimatedOpacity` on a 300 ms implicit curve, where Compose animates their bounds with a spring (damping 0.8, stiffness 380) from `PaneMotion` and `LookaheadScope`.
  Reason: framework — Flutter's implicit animations take the place of Compose's animation core and lookahead bounds animation.
  Affect: a hidden pane slides off the side and fades instead of taking Compose's per-role enter and exit transitions; the settled layout and which pane keeps its state are the same.

## three_pane_navigator.rs → ThreePaneScaffoldNavigator.kt (replaced)
- Change: `ThreePaneNavigator` turns the app's back stack and a `ThreePaneSceneStrategy` into Waft `Navigator` pages; there is no `ThreePaneScaffoldNavigator` and no pane history beside the app's navigation.
  Reason: framework — Flutter's Navigator 2.0 owns the page list and its back gestures, and the app owns the back stack.
  Affect: an app supplies one entry-page builder, one scene-page builder and a truncate callback; a phone's detail is an ordinary pushed page with predictive back and the platform's swipe.

## navigation_suite_scaffold.rs → NavigationSuiteScaffold.kt
- Change: `NavigationSuiteType` has Compose's bar, rail and drawer plus Flutter's extended `NavigationRail`, and the default is a bar when the width or height is Compact, the rail at Medium and Expanded, and the extended rail from Large.
  Reason: framework — Flutter has an extended rail and neither Compose's short navigation bar nor its wide navigation rail, and wide Flutter windows conventionally take a labelled rail.
  Affect: crossing 1200 runs the rail's own extend animation instead of swapping to Compose's collapsed wide rail, and a drawer is only used when the app asks for one.
- Change: the suite builds Waft's `NavigationBar`, `NavigationRail` and `NavigationDrawer`, and removes the padding that the bar or rail covers from the content's `MediaQuery`.
  Reason: framework — Flutter's navigation widgets already carry their own layout and Material styling.
  Affect: an app gets the existing Material themes and the content starts inside the bar's or rail's inset.

## Deferred
- levitated panes (`AdaptStrategy::Levitate`, the scrim and drag-to-resize state). Trigger: a modal pane layer in `ThreePaneScaffold`.
- pane expansion (`PaneExpansionState`, the draggable divider). Trigger: an optional field on `ThreePaneScaffold`.
- predictive back inside a scene (`seekTo`, `PredictiveBackScaleState`). Trigger: the scaffold observing the back gesture; single-pane steps are already pages.
- custom pane enter and exit transitions (`AnimatedPane`'s transitions and transition scope). Trigger: optional fields on `Pane`.
- pane margins (`PaneMargins`, `RectRulers`). Trigger: a pane inset model beyond `SafeArea` and `MediaQuery`.
- auto-focus of the current destination, and several scenes keyed in one stack (`sceneKey`). Trigger: `FocusNode` support and a key on the destination item.
- `NavigationSuiteScaffoldState` (show and hide the suite). Trigger: a controller for the suite.
- Compose's short navigation bar, its wide navigation rail, and a modal drawer chosen by platform. Trigger: those components' own Material ports.
- savers and `rememberSaveable` for pane state. Trigger: restoration in Waft.
