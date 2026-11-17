# waft-material/src/compose
Compose home: compose/material3/adaptive (adaptive, adaptive-layout, adaptive-navigation3), compose/material3/material3-adaptive-navigation-suite, and compose/material3/material3 (the Expressive components Flutter lacks)
Ported against: b8f12c706099a47e1e44d9772d03e14a4ae37146

Reasons are the porting skill's `language` and `platform`, plus `framework`: Compose's mechanism has no place in Flutter's model.

## Identical
- adapt_strategy.rs → AdaptStrategy.kt, ThreePaneScaffoldAdaptStrategies.kt
- back_navigation_behavior.rs → BackNavigationBehavior.kt
- pane_adapted_value.rs → PaneAdaptedValue.kt
- pane_motion.rs → PaneMotion.kt, ThreePaneMotion.kt: the spring (damping 0.8, stiffness 380) on Flutter's `SpringSimulation`
- pane_scaffold_role.rs → PaneScaffoldRole.kt
- three_pane_scaffold_horizontal_order.rs → ThreePaneScaffoldHorizontalOrder.kt

## window_size_class.rs → androidx.window's WindowSizeClass
- Change: `WindowSizeClass::of` and `maybe_of` read the nearest `MediaQuery`'s size through the `App`, where Compose reads `currentWindowAdaptiveInfoV2()`.
  Reason: framework — Flutter's window size is a `MediaQuery` a widget reads with its context, not an ambient Compose value.
  Affect: a widget calls `WindowSizeClass::of(app, context)` and rebuilds when `MediaQuery` changes; there is no scope widget.
- Change: `WindowWidthSizeClass::margin` answers Material 3's window margin, 16 when Compact and 24 from Medium, which neither androidx.window nor Compose carries.
  Reason: framework — Compose's `PaneScaffoldDirective` dropped its `contentPadding` before 1.0 and leaves margins to the app, and an app needs the spec's value to give `Pane::margins`.
  Affect: an app reads the margin for its window instead of writing 16 or 24 itself.

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

## three_pane_scaffold.rs → ThreePaneScaffold.kt, Pane.kt
- Change: `Pane::margins` takes fixed margins only, without the `RectRulers` insets Compose's `paneMargins` also accepts.
  Reason: framework — Flutter has no rulers; its insets are `MediaQuery` padding, which a pane's content already reads.
  Affect: a pane keeps its margins from the scaffold's edges, and its content still pads itself for the system's insets.
- Change: a hidden pane stays mounted under an `Offstage`, where Compose disposes it once its exit motion ends and keeps its saved state in a `SaveableStateHolder`.
  Reason: framework — Flutter keeps a subtree's state by keeping its element, and has no saveable state holder.
  Affect: a list hidden behind its detail keeps its scroll position and its live state, and comes back as it was.

## three_pane_navigator.rs → ThreePaneScaffoldNavigator.kt (replaced), Navigation 3's NavDisplay
- Change: `ThreePaneNavigator` turns the app's back stack and a `ThreePaneSceneStrategy` into the pages of a nested Waft `Navigator`, behind a `NavigatorPopHandler`; there is no `ThreePaneScaffoldNavigator` and no pane history beside the app's navigation.
  Reason: framework — Flutter's Navigator 2.0 owns the page list and its back gestures, and the app owns the back stack.
  Affect: an app supplies its own entries, a function naming each entry's pane, an entry-page builder, a scene-page builder and a callback that truncates its stack; a phone's detail is an ordinary pushed page with predictive back and the platform's swipe, and the system back reaches it first.
- Change: pages that a resize adds or removes, because the scene strategy changed while the back stack did not, come and go without a transition.
  Reason: framework — Navigation 3 swaps scenes in place, where a Flutter `Navigator` pushes and pops pages unless its `TransitionDelegate` says otherwise.
  Affect: crossing 840 swaps the detail's page for the shared page at once, instead of sliding it away.

## navigation_suite_scaffold.rs → NavigationSuiteScaffold.kt
- Change: `NavigationSuiteType` has Compose's bar, rail and drawer plus Flutter's extended `NavigationRail`, and the default is a bar when the width or height is Compact, the rail at Medium and Expanded, and the extended rail from Large.
  Reason: framework — Flutter has an extended rail and neither Compose's short navigation bar nor its wide navigation rail, and wide Flutter windows conventionally take a labelled rail.
  Affect: crossing 1200 runs the rail's own extend animation instead of swapping to Compose's collapsed wide rail, and a drawer is only used when the app asks for one.
- Change: the suite builds Waft's `NavigationBar`, `NavigationRail` and `NavigationDrawer`, and removes the padding that the bar or rail covers from the content's `MediaQuery`.
  Reason: framework — Flutter's navigation widgets already carry their own layout and Material styling.
  Affect: an app gets the existing Material themes and the content starts inside the bar's or rail's inset.
- Change: `NavigationSuiteColors` carries only the bar's, rail's and drawer's container colours, and the scaffold has no `contentColor`.
  Reason: framework — Flutter's navigation widgets and `Material` take their foreground colours from their themes, not from a content colour passed down.
  Affect: an app sets icon and label colours through `NavigationBarTheme`, `NavigationRailTheme` and `NavigationDrawerTheme`.
- Change: `navigation_item_vertical_arrangement` places the rail's destinations through Flutter's `group_alignment`, and leaves the permanent drawer's at the top.
  Reason: framework — Flutter's `NavigationDrawer` is a scrolling list with no group alignment, where Compose's drawer sheet is a column it can space.
  Affect: a centred arrangement centres a rail's destinations, and a drawer still lists them under its header.

## split_button.rs → SplitButton.kt

- Change: a part is one of Flutter's buttons — `FilledButton`, `ElevatedButton` or `OutlinedButton` — with the split's shape, padding and minimum size layered over its style, where Compose builds the part from `Surface` and `ButtonDefaults`.
  Reason: framework — Flutter's buttons already carry Compose's colours, elevation and disabled behaviour, and a Flutter `ButtonStyle` takes a shape per state.
  Affect: a part's colours and elevation are the Flutter button's, the same values Compose's defaults draw, and its styles can be swapped only by the four variants.
- Change: a part's corner change animates over the button `Material`'s animation duration and curve, where Compose drives it with the Expressive effects spring.
  Reason: framework — Flutter animates a `Material`'s shape change itself, and a button has no hook to replace that animation.
  Affect: pressing a part morphs its inner corners over 200ms on Flutter's curve instead of the bouncier spring.
- Change: the outer corners are the circle radius of the height the shapes were chosen for, where Compose's `CornerSize(50%)` resolves against the part's measured size.
  Reason: framework — Flutter's border radii are absolute, with no percentage corner.
  Affect: a part with a custom height keeps the radius of its token size, so a part much taller than its size has slightly less round outer corners than Compose's.
- Change: the layout is an `IntrinsicHeight` around a `Row` whose children stretch, and the leading part is laid out first, where Compose measures the trailing part first for width and forces both to the taller intrinsic height.
  Reason: framework — Flutter has no multi-child render object to add here, and its `Row` measures in child order.
  Affect: both parts still share one height; under a width too small for the pair the leading part keeps its width and the trailing one shrinks, the opposite priority to Compose.
- Change: the trailing part's content is centred by its equal start and end padding; Compose's optical centring for unequal paddings is not ported.
  Reason: framework — Flutter's button padding has no optical-centring step.
  Affect: a trailing part with different start and end padding looks off-centre where Compose shifts its content to the middle of the shape.

## button_group.rs → ButtonGroup.kt

- Change: a group lays out buttons the app builds, and an app styles a connected group's buttons from `ButtonGroupDefaults`' three shapes, where Compose's `ButtonGroupScope` builds `clickableItem`s and `toggleableItem`s itself.
  Reason: framework — Flutter has no content scope; a child list is its equivalent, and a child's style belongs to the child.
  Affect: an app makes each button and asks for the connected shapes it needs.
- Change: the standard and connected arrangements are a `Row` with the group's spacing and alignment.
  Reason: framework — Flutter's `Row` already spaces and aligns children.
  Affect: the buttons sit at Compose's spacings, 12 and 2.

## Deferred
- split button shape morphing on the Expressive spring, and a part's own `ButtonColors`, `ButtonElevation`, border and interaction source. Trigger: an animation scheme on the button, and style arguments on a part.
- pressing a group's button expands it and compresses its neighbours (`EnlargeOnPress`, `expandedRatio`). Trigger: a layout that measures children while their widths animate.
- the group's overflow (`overflowIndicator`, `ButtonGroupMenuState`, hiding the items that do not fit into a menu, and its saved state). Trigger: a layout that measures which children fit and a menu built from the hidden ones.
- the group's `clickableItem`, `toggleableItem` and `customItem` content scope. Trigger: Compose's Expressive `ToggleButton`, which Flutter and Waft lack, for the toggleable item.
- a group item's `Modifier.weight` and vertical `align`. Trigger: float weights and per-child alignment in the group's layout.
- levitated panes (`AdaptStrategy::Levitate`, the scrim and drag-to-resize state). Trigger: a modal pane layer in `ThreePaneScaffold`.
- pane expansion (`PaneExpansionState`, the draggable divider). Trigger: an optional field on `ThreePaneScaffold`.
- predictive back inside a scene (`seekTo`, `PredictiveBackScaleState`). Trigger: the scaffold observing the back gesture; single-pane steps are already pages.
- custom pane enter and exit transitions (`AnimatedPane`'s transitions and transition scope). Trigger: optional fields on `Pane`.
- pane margins from insets (`RectRulers`). Trigger: a pane inset model beyond `SafeArea` and `MediaQuery`.
- auto-focus of the current destination, and several scenes keyed in one stack (`sceneKey`). Trigger: `FocusNode` support and a key on the destination item.
- `NavigationSuiteScaffoldState` (show and hide the suite). Trigger: a controller for the suite.
- Compose's short navigation bar, its wide navigation rail, and a modal drawer chosen by platform. Trigger: those components' own Material ports.
- savers and `rememberSaveable` for pane state. Trigger: restoration in Waft.
