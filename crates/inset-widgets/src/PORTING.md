# inset-widgets/src
Flutter home: packages/flutter/lib/src/widgets
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical
- widgets/expansible.rs → expansible.dart
- widgets/sliver_persistent_header.rs → sliver_persistent_header.dart
- widgets/shared_app_data.rs → shared_app_data.dart
- widgets/annotated_region.rs → annotated_region.dart
- widgets/scroll_activity.rs → scroll_activity.dart
- widgets/scroll_notification_observer.rs → scroll_notification_observer.dart
- widgets/undo_history.rs → undo_history.dart
- widgets/autofill.rs → autofill.dart
- widgets/spell_check.rs → spell_check.dart
- view.rs → view.dart
- widgets/notification_listener.rs → notification_listener.dart
- widgets/safe_area.rs → safe_area.dart
- widgets/value_listenable_builder.rs → value_listenable_builder.dart
- widgets/sliver.rs → sliver.dart
- widgets/navigation_toolbar.rs → navigation_toolbar.dart
- widgets/standard_component_type.rs → standard_component_type.dart
- widgets/title.rs → title.dart
- widgets/scroll_simulation.rs → scroll_simulation.dart
- widgets/scroll_metrics.rs → scroll_metrics.dart
- widgets/primary_scroll_controller.rs → primary_scroll_controller.dart
- widgets/scroll_controller.rs → scroll_controller.dart
- widgets/single_child_scroll_view.rs → single_child_scroll_view.dart
- widgets/pages.rs → pages.dart
- widgets/pop_scope.rs → pop_scope.dart
- widgets/modal_barrier.rs → modal_barrier.dart
- widgets/navigator_pop_handler.rs → navigator_pop_handler.dart
- widgets/text.rs → text.dart (+ `RichText` from basic.dart)
- widgets/status_transitions.rs → status_transitions.dart
- widgets/text_editing_intents.rs → text_editing_intents.dart
- widgets/text.rs → text.dart (+ `RichText` from basic.dart)
- widgets/preferred_size.rs → preferred_size.dart
- widgets/context_menu_button_item.rs → context_menu_button_item.dart
- widgets/text_selection_toolbar_anchors.rs → text_selection_toolbar_anchors.dart
- widgets/desktop_text_selection_toolbar_layout_delegate.rs → desktop_text_selection_toolbar_layout_delegate.dart
- widgets/text_selection_toolbar_layout_delegate.rs → text_selection_toolbar_layout_delegate.dart
- widgets/animated_size.rs → animated_size.dart

## widgets/app_lifecycle_listener.rs → app_lifecycle_listener.dart

- Change: a listener starts with no known lifecycle state, and the binding has none to report.
  Reason: platform — `SchedulerBinding.lifecycleState` waits (scheduler's PORTING.md).
  Affect: the first transition a listener sees is treated as the "no previous state" case, and a state the host reached before the listener existed is never read.

- Change: `did_request_app_exit` and `on_exit_requested` answer within the call, where Dart returns a `Future`.
  Reason: language — the default has nothing to await.
  Affect: an exit handler must decide before it returns; it cannot hold the request open.

## framework/ → framework.dart

- Change: a `State` is destroyed with its element at unmount, and `mounted` is the only safe call on a retained handle afterwards.
  Reason: language — no garbage collector keeps Dart's disposed `State` alive; the arena frees the slot.
  Affect: Dart's `if (mounted) setState(..)` guard from a timer or listener still works, but reading `widget` or `context` on an unmounted state panics with a stale-handle message instead of returning the last widget.

- Change: a panic in `build` unwinds; there is no `ErrorWidget` and no `FlutterError.reportError`, and a failed `of` lookup panics with its message in every build.
  Reason: language — no catchable exception for ordinary control flow.
  Affect: a failing build takes the frame down instead of painting the red error box, and a failed lookup reports the same in release as in debug.

- Change: a `ParentDataWidget` names the parent-data types it accepts, and `Positioned` accepts an overlay theater's parent data as well as a `Stack`'s.
  Reason: language — no subtyping between parent-data types, so Dart's `is StackParentData` test cannot pass for a subclass.
  Affect: a `ParentDataWidget` over a render object whose Dart parent data is a subclass of the expected one is rejected unless the widget names that type too.

## binding.rs → binding.dart

- Change: `WidgetsBindingObserver` carries the metrics, locale and lifecycle callbacks only, and nothing raises the lifecycle ones.
  Reason: platform — the route, memory-pressure and back-gesture events have no host surface yet.
  Affect: an observer hears metrics and locale changes; a lifecycle change reaches it only when something calls the method directly.

- Change: `WidgetsBindingObserver::did_change_drop` and the `DropListener` in `widgets/drop_listener.rs` hear files dragged from outside the application, from `EmbedderClient::drop_data`; Flutter has no drop in its framework and leaves it to plugins.
  Reason: platform — the host reports the drag at the window, and the binding's observers are where window-level events reach the tree.
  Affect: any observer hears every drag over any view; with a host that reports no position, the whole view is the target.

## widgets/banner.rs → banner.dart

- Change: `BannerPainter` answers no `repaint` listenable, where Dart repaints on `PaintingBinding.systemFonts`.
  Reason: platform — the binding raises no system-font change yet.
  Affect: a banner does not repaint when the system fonts change.

## widgets/basic.rs → basic.dart (+ `WidgetBuilder` from framework.dart, `Spacer` from spacer.dart, `IndexedStack` from indexed_stack.dart)

- Change: a Dart constructor assert runs on the setter that can violate it, or, when it needs several fields, when the element creates or updates the render object; every widget in this crate with such asserts follows this.
  Reason: language — a struct literal has no constructor body, and a fluent setter cannot see the arguments a later setter will supply.
  Affect: an invalid combination panics during build rather than where the widget is written, and a single-field assert fires at its setter, so a chain Dart's constructor would accept (`Positioned::fill(..).width(..)`) panics at `.width`.

- Change: the semantics flags (`Opacity.alwaysIncludeSemantics`, the deprecated `ignoringSemantics`) and `Transform.filterQuality` are absent, here and in the transitions and implicit animations built on these widgets.
  Reason: platform — accessibility is deferred, and `RenderTransform` carries no filter quality.
  Affect: a transformed subtree is always applied by re-rendering the child, never rasterised through an image filter.

- `BackdropFilter`'s blend modes and anisotropic sigmas follow rendering's `RenderBackdropFilter` entry.

## widgets/container.rs → container.dart

- Change: `Container.color` and `decoration`'s mutual exclusion is asserted by whichever of the two setters runs second; the margin, padding and constraints asserts run on their own setters (basic.rs's assert entry).
  Reason: language — a struct literal has no constructor body.
  Affect: `.color(..).decoration(..)` panics at `.decoration` with Dart's message, in either order.

## widgets/ticker_provider.rs → ticker_provider.dart

- Change: `TickerMode::get_notifier` returns a fresh wrapper on every call, and its fallback is a fresh constant listenable.
  Reason: language — a `dyn` value has no identity of its own, and foundation's erased `Listenable` value is deferred (this is its trigger).
  Affect: two notifiers for one `TickerMode` never compare equal, so the ticker mixins drop and re-add their listener on every `activate`.

- Change: a ticker vended by `TickerProviderStateMixin` stays in the state's set after it is disposed.
  Reason: language — Dart's `_WidgetTicker` subclasses `Ticker` to unregister itself, and `dispose` here is an inherent method on a concrete arena struct.
  Affect: a state that churns animation controllers holds one handle per dead ticker until it is itself disposed, and muting a disposed ticker does nothing.

## widgets/transitions.rs → transitions.dart

- Change: the tweens this crate declares are `Clone` values built without the `App`, where `inset-animation`'s are arena objects.
  Reason: language — orphan rule: an `Animatable` impl on a handle to a tween would name only foreign types.
  Affect: `drive` drives a clone, so writing `begin` or `end` afterwards does not reach the running animation as it does on a Dart `Tween`.

- `SizeTransition`'s constructor asserts run on its setters, the mutually exclusive `axis_alignment` / `alignment` on whichever runs second; see basic.rs's assert entry.
- `MatrixTransition` (so `ScaleTransition` and `RotationTransition`) has no `filterQuality` and `FadeTransition` no `alwaysIncludeSemantics`; see basic.rs's `Transform` entry.

## widgets/icon_theme_data.rs → icon_theme_data.dart, widgets/icon_theme.rs → icon_theme.dart

- Change: a `CupertinoIconThemeData` is an `IconThemeData` carrying a `resolve` override, which `copy_with` and `merge` keep and `lerp` drops.
  Reason: language — `IconTheme.data` is a value, not a subclassable object.
  Affect: a lerped cupertino icon theme resolves like a plain one, where Dart's subclass keeps resolving as itself.

## widgets/icon.rs → icon.dart

- Change: `Icon` has no `semanticLabel`, and its build returns the sized glyph without Dart's `Semantics` wrappers.
  Reason: platform — accessibility is deferred.
  Affect: nothing announces the icon; the render tree is one node shallower on each side.

## widgets/gesture_detector.rs → gesture_detector.dart

- Change: `replaceGestureRecognizers`'s `FlutterError` is a `debug_assert!`.
  Reason: language — no catchable exception; diagnostics deferred.
  Affect: calling it outside layout panics in debug and proceeds in release.

## widgets/media_query.rs → media_query.dart

- Change: `MediaQueryData::from_view` reads the platform's brightness and leaves the accessibility, text-scaling, gesture-inset and corner-radius fields at their defaults.
  Reason: platform — `Platform` exposes `platform_brightness` only, and `ViewMetrics` has no gesture insets or corner radii.
  Affect: those fields are only non-default when an ambient `MediaQuery` supplies `platform_data`.

- Change: `MediaQuery::from_view` re-derives its data on dependency, widget and metrics changes only.
  Reason: platform — the text scale factor, brightness and accessibility flags have no host events yet.
  Affect: a platform brightness change after mount is not reflected until the shell forwards it.

## widgets/image.rs → image.dart

- Change: `ImageState` keeps a failed load to itself, where Dart also hands the failure to `FlutterError.onError`.
  Reason: language — there is no isolate-wide error hook to report it to.
  Affect: a failed image leaves an empty box and nothing in the log, and a caller that wants to react reads the failure off the state.

- Change: `create_local_image_configuration` leaves `bundle` and `locale` unset.
  Reason: platform — `DefaultAssetBundle` and `Localizations` wait.
  Affect: an `ImageProvider` keyed by bundle or locale resolves against `None`.

## widgets/widget_state.rs → widget_state.dart

- Change: `WidgetStateMapper::resolve` panics with Dart's `ArgumentError` message when no key is satisfied, for every value type.
  Reason: language — Dart's `null as T` is a runtime test of `T`'s nullability, which a Rust generic cannot make without specialization.
  Affect: a mapper over an `Option` panics on an unmatched set instead of resolving null, unless the map ends with a `WidgetState::any()` entry.

- Change: `WidgetStateColor` is an `AnyColor` whose plain value is its empty-state resolution, read back with `color.extension::<WidgetStateColor>()`.
  Reason: language — painting's `Color` is a value, not a subclassable object.
  Affect: a map-built colour read as a plain `Color` gives its empty-state resolution where Dart throws, and panics only when no key matches the empty set.

- Change: `clickable()`, `adaptive_clickable()` and `textable()` return a fresh `WidgetStateMouseCursor` per call.
  Reason: language — no `static const` object holding an `Rc`.
  Affect: two such cursors are never equal, so a `MouseRegion` handed one per build re-annotates every build where Dart's canonical `const` compares equal; map-built cursors compare by map equality.

## framework: notifications → framework.dart (`Notification`, `NotifiableElementMixin`, `_NotificationNode`)

- Change: `NotificationListener<T>` matches one concrete notification type, or a family such as `dyn ScrollNotification` that a notification declares itself part of.
  Reason: language — Dart's `notification is T` covers subclasses, and a trait family has no `TypeId` a downcast can name.
  Affect: a listener sees only that exact type or a declared family, where Dart's also catches every subclass.

## widgets/page_storage.rs → page_storage.dart

- Change: `PageStorageBucket` stores erased values and `PageStorageKey` carries no type argument.
  Reason: language — an erased key cannot be recognised as a generic type.
  Affect: reading state back under a type other than the one written yields `None` rather than the stored value.

## widgets/scroll_delegate.rs → scroll_delegate.dart

- Change: `SliverChildListDelegate::should_rebuild` compares the two child lists pairwise (length, then `Rc::ptr_eq`).
  Reason: language — Dart compares the two `List` references, and a `Vec` has no identity.
  Affect: a delegate rebuilt from a fresh list of the same widget references does not rebuild its children, where Dart's list-identity check would.

## widgets/viewport.rs → viewport.dart

- `Viewport`'s center and cache-extent asserts run on whichever of the setters they need runs second; see basic.rs's assert entry.

## widgets/default_text_editing_shortcuts.rs → default_text_editing_shortcuts.dart

- Change: the shortcut tables are functions that build a fresh `ShortcutMap` per call, and so is `intent_for_macos_selector`.
  Reason: language — an `Rc` value cannot be a `const`, and the target platform is a property of the host-supplied `Platform`.
  Affect: the `ShortcutManager` re-indexes on every rebuild of this widget instead of seeing an unchanged const map.

- Change: the keys a platform is said to handle itself are only stood aside for when the host says it turns editing keys into edits.
  Reason: platform — Dart stands aside for every macOS and iOS host because its embedders for those two are the only ones it has; a host that reports plain key presses interprets nothing, so standing aside would leave those keys to nobody.
  Affect: over such a host a field's backspace, delete and arrow keys edit the text instead of doing nothing, on macOS and iOS as everywhere else.

## widgets/app.rs → app.dart

- `WidgetsApp`'s constructor asserts run when the widget is inflated, so a contradictory configuration panics at mount; see basic.rs's assert entry.

- Change: `WidgetsApp::default_shortcuts` and `default_actions` build a fresh value per call, with new action objects.
  Reason: language — an `Rc` or an arena handle cannot be a `const`, and Dart's `defaultActions` is one mutable map of shared action objects.
  Affect: modifying the returned map changes only the app it is passed to, where Dart's global map is shared by every `WidgetsApp`.

- Change: the default navigation-notification handler stops the notification without telling the host.
  Reason: platform — the host is never told in advance whether the framework handles back, because it does not need to be: `EmbedderClient::pop_route` answers on the spot, where Dart's `popRoute` crosses a channel and cannot.
  Affect: the host is never told whether the framework can handle a back gesture; an `on_navigation_notification` of your own still runs in place of the default.

## widgets/scroll_context.rs → scroll_context.dart

- Change: a scroll context hands its positions one shared `Rc<dyn ScrollContext>`, compared by pointer where Dart compares `this`.
  Reason: language — an `Rc` has no `==` by object identity.
  Affect: a context that mints a fresh `Rc` per position fails `absorb`'s identity check.

## widgets/scroll_physics.rs → scroll_physics.dart

- Change: the deprecated `tolerance` getter reads the implicit view's device pixel ratio from `App::platform`, not an isolate-global window.
  Reason: platform — the host owns the views.
  Affect: `physics.tolerance(app)` panics without an implicit view; call `tolerance_for(metrics)`.

## widgets/scroll_notification.rs → scroll_notification.dart

- Change: the five scroll notifications answer `is ScrollNotification` through a declared family slot, and none of them is a `LayoutChangedNotification`.
  Reason: language — no inheritance, so a listener matches an exact type or a declared family.
  Affect: a `NotificationListener<LayoutChangedNotification>` does not see scroll notifications, where Dart's does.

## widgets/scroll_position.rs → scroll_position.dart

- Change: everything that reads a position's metrics — the physics, a `ScrollMetricsNotification`, `ScrollIncrementDetails` — is handed a `copy_with` snapshot instead of the position itself.
  Reason: language — the axis direction and device pixel ratio are live reads of the `ScrollContext`, which needs the arena the `&self` `ScrollMetrics` trait does not have.
  Affect: a `ScrollPhysics` sees the metrics as they were at the call, not the live position Dart lets it re-read.

- Change: the semantic-action bookkeeping and the `notifyListeners` override that ran it are gone.
  Reason: platform — `ScrollContext.setSemanticsActions` is deferred (accessibility).
  Affect: nothing schedules a semantics update when the offset or the dimensions change.

## widgets/scroll_position_with_single_context.rs → scroll_position_with_single_context.dart

- Change: `absorb` tests the other position by downcasting to its own leaf type rather than for the `ScrollPositionWithSingleContext` base.
  Reason: language — the arena answers only the type a handle was created with.
  Affect: absorbing a position of a different leaf type goes idle and keeps its own scroll direction and drag, where Dart hands both over.

## widgets/scroll_configuration.rs → scroll_configuration.dart

- Change: `build_overscroll_indicator` returns the child on every platform.
  Reason: platform — `GlowingOverscrollIndicator` is not ported (`## Deferred`).
  Affect: a scroll view has no overscroll glow, so its tree is one node shallower.

## widgets/scrollable.rs → scrollable.dart, widgets/scrollable_helpers.rs → scrollable_helpers.dart

- Change: `build` wraps the viewport in `Listener` → `RawGestureDetector` → `IgnorePointer` only: the semantics wrappers and the selection handler are gone.
  Reason: platform — accessibility and selection are deferred (`## Deferred`).
  Affect: a scrollable exposes no scroll actions to a screen reader and cannot be selected across; its tree is three nodes shallower.

- Change: a pointer scroll signal is resolved through gestures' `PointerSignalResolver`, but the host is not told the event was handled.
  Reason: platform — `PointerEvent.respond` is omitted on signal events (inset-gestures records it).
  Affect: an embedder that would scroll the page itself is not told the scrollable took the event.

## widgets/scroll_view.rs → scroll_view.dart

- Change: the physics a primary scroll view implies is substituted in `build`, not stored on the widget.
  Reason: language — a fluent setter cannot see the `primary`, `controller` and direction a later setter will supply.
  Affect: `view.physics` reads `None` where Dart's field already holds the implied `AlwaysScrollableScrollPhysics`; the `Scrollable` is given the same physics either way.

- Change: `ListView`'s `children` and `add*` arguments are setters, whichever of them runs last rebuilding the delegate out of all of them, and `ListView::builder(..)` and `separated(..)` are separate values that convert to the widget.
  Reason: language — a fluent setter cannot see the arguments a later setter will supply, and the delegate is built from four of them.
  Affect: any order of those four setters lands on the delegate Dart's constructor builds, and all four panic on a builder-, separated- or custom-constructed `ListView`.

## widgets/focus_manager.rs → focus_manager.dart

- Change: the manager takes key events one at a time from `HardwareKeyboard`; there is no `KeyMessage`, no `RawKeyEvent` and no `FocusNode.onKey`.
  Reason: platform — services' raw key path and `KeyEventManager.keyMessageHandler` are deferred (`inset-services/src/PORTING.md`).
  Affect: a node handles keys through `on_key_event` only, and the Android virtual-keyboard check that suppresses a highlight change does not run.

- Change: `size`, `offset` and `rect` read the context's render object as a `RenderBox` and use its size where Dart reads `semanticBounds`.
  Reason: platform — `semanticBounds` waits with semantics; on a box it is exactly that rectangle.
  Affect: a focus node whose context hosts something other than a `RenderBox` panics instead of returning that object's paint bounds.

## widgets/focus_scope.rs → focus_scope.dart

- Change: `Focus.includeSemantics` is carried but nothing reads it, and the debug focus border is absent.
  Reason: platform — accessibility is deferred; diagnostics wait.
  Affect: a focusable widget exposes no semantics focusable / focused flags and cannot be focused by a semantics action.

## widgets/actions.rs → actions.dart

- Change: Dart's `const ActionDispatcher()` is `PlainActionDispatcher`, a fresh object per call.
  Reason: language — no `const` object holding an `Rc`, and a trait and a struct cannot share a name.
  Affect: two dispatchers are never equal, so an `Actions` widget that sets one rebuilds its dependents on every update.

- Change: `FocusableActionDetector.includeFocusSemantics` is carried but nothing reads it.
  Reason: platform — accessibility is deferred.
  Affect: a detector exposes no semantics focusable / focused flags.

## widgets/shortcuts.rs → shortcuts.dart

- Change: shortcuts are an insertion-ordered list of activator and intent pairs, not a map; `CallbackShortcuts.bindings` has the same shape.
  Reason: language — an erased `ShortcutActivator` has no content hash to key a map with.
  Affect: two entries with equal activators both stay where Dart's map keeps only the last, and reordering the list counts as a change.

## widgets/restoration.rs → restoration.dart, widgets/restoration_properties.rs → restoration_properties.dart

- Change: the root bucket is read within the call, so what Dart runs from the future's continuation runs inline.
  Reason: platform — the host's `Restoration` capability answers within the call (inset-services' `root_bucket`).
  Affect: the first frame is never held back and a `RootRestorationScope` hands its child a bucket on its very first build, where Dart renders an empty container first.

- Change: `RestorableEnum` and `RestorableEnumN` serialize a value with its `Debug` output where Dart uses the enum's name, and keep duplicate `values`.
  Reason: language — a Rust enum has no reflected variant name.
  Affect: a hand-written `Debug` decides what is written to the restoration data, so a stored value is only restored while that output stays the variant name.

## widgets/overlay.rs → overlay.dart

- Change: the theater does not suppress the `markNeedsLayout` that adopting or dropping an overlay child performs, and the deferred layout box always re-schedules its subtree.
  Reason: language — a Rust impl cannot call the trait body it overrides, so neither the outstanding-update counter nor the dirty flag can be maintained.
  Affect: showing or hiding an `OverlayPortal` child relays out the whole `Overlay`, and the overlay child's subtree is laid out again whenever the `Overlay` or the `OverlayPortal` lays out; the resulting tree is the same, the work is not.

- Change: the theater's and the deferred layout box's children are collected into a `Vec` before each layout, paint and hit-test walk.
  Reason: language — a lazy iterator would borrow the `App` for the whole walk, and the body of the walk needs `&mut App`.
  Affect: an overlay child added or removed during the `Overlay`'s own layout is picked up on the next layout pass, not the current one.

## widgets/navigator.rs → navigator.dart

- Change: `Route::will_pop` and `NavigatorState::maybe_pop` answer within the call, where Dart returns a future.
  Reason: language — the only asynchrony in Dart's deprecated `willPop` is the callback loop deferred with `WillPopScope` (`## Deferred`).
  Affect: a pop is decided before the call returns, so nothing can hold the decision open; when the callback loop is ported, both become `CompleterFuture`s.

- Change: a route result is the erased `Option<Rc<dyn Any>>`; `Route`, `Page` and `TransitionDelegate` carry no result type.
  Reason: language — the navigator holds routes of mixed result types in one history, and a type-erased handle cannot carry a per-route type parameter.
  Affect: a popped result is downcast when it comes back and a mismatched type is simply `None`, where Dart's `T` is settled when the code is compiled.

## widgets/routes.rs → routes.dart

- Change: `RouteObserver` takes a `RoutePredicate` where Dart takes the type argument `R`.
  Reason: language — a Rust type parameter cannot answer "is this erased route an `R`" without a witness.
  Affect: which routes an observer is told about is whatever predicate it was given, not a route class.

- Change: `RawDialogRoute.buildPage` returns the page builder's widget directly, and there is no `anchorPoint`.
  Reason: platform — the `Semantics` and `DisplayFeatureSubScreen` wrappers Dart adds are deferred, and `anchorPoint` exists only to feed the latter.
  Affect: a dialog is not confined to the display-feature sub-screen closest to an anchor point.

- Change: `AnyTransitionRoute` carries `underlying_animation` beside `animation`, for the animation a route runs on rather than whatever a subclass answers with.
  Reason: language — Dart keeps the animation in a private field and offers a getter over it, and a `ModalRoute` overrides that getter with a proxy that reads complete while the route is offstage. Where the route behind measures the one ahead, Dart reaches past the override to the field, which library privacy lets it do; an erased route here is reached through a vtable, so the field needs an edge of its own, as `controller_value` already has.
  Affect: a route behind follows the next route's own animation and stays where it is while that route is offstage to be measured, instead of flashing the end of the transition before it plays.

## widgets/implicit_animations.rs → implicit_animations.dart

- Change: this file's tweens are value tweens in transitions.rs's shape, and the ones over a nullable Dart value animate an `Option` whose endpoints flatten, so an unset endpoint and one set to `None` are the same thing.
  Reason: language — the same orphan rule; Dart's `BorderRadius??` flattens where `Option<Option<_>>` does not, and `_constructTweens`' `tween.end ?? tween.begin` depends on that.
  Affect: a tween with an unset `begin` returns `None` at `t == 0` rather than panicking, and `Matrix4Tween` interpolates the host's f32 matrices, so a `decompose` / `compose` round trip is exact only to f32.

- Change: `ImplicitlyAnimatedWidgetState::dispose` runs `for_each_tween` once more to free each slot's tween; Dart's leaves its tweens alone.
  Reason: language — no garbage collector: a tween a slot's constructor made would hold its arena slot for the life of the `App`.
  Affect: a tween handle a state made is stale once the state is disposed, and a slot that `for_each_tween` skips (Dart's `_AnimatedAlignState` skips one when its factor is null) is never freed.

- `AnimatedScale` and `AnimatedRotation` have no `filterQuality`, and `AnimatedOpacity` no `alwaysIncludeSemantics`; see basic.rs's `Transform` entry.
- `AnimatedContainer`'s colour / decoration exclusion and its other constructor asserts run on the setters, as container.rs records; `width` and `height` fold into `constraints` on assignment, as `Container`'s do.

## widgets/visibility.rs → indexed_stack.dart (`Visibility`)

Flutter folded `visibility.dart` into `indexed_stack.dart`; this file keeps the old name, and the shared scope lives in basic.rs with the rest of `IndexedStack`.

- `Visibility`'s five `maintain*` asserts run on the widget's first build, so any order of the setter chain is accepted; see basic.rs's assert entry.

- Change: `maintain_semantics` is carried but nothing reads it.
  Reason: platform — accessibility is deferred.
  Affect: a hidden child is never reported to accessibility tools, `maintain_semantics` or not.

## widgets/draggable_scrollable_sheet.rs → draggable_scrollable_sheet.dart

- Change: `_replaceExtent` compares the old and new `snap_sizes` by value.
  Reason: language — Dart's `List` `==` is identity, and a `Vec` has none.
  Affect: rebuilding a snapping sheet with a fresh list of the same sizes does not schedule the post-frame snap Dart schedules.

## widgets/editable_text.rs → editable_text.dart

- Change: an app private command arrives with its action string only; the argument map Flutter sends is dropped.
  Reason: platform — the host's `perform_private_command` carries no map.
  Affect: a private command handler cannot read the arguments the platform sent with it.

- Change: `select_all_on_focus` is resolved from the target platform when it is used, and construction skips Dart's iOS password autocorrect rule and its platform keyboard-type tables, inferring the keyboard from `max_lines` and a small hint map.
  Reason: language — construction has no `App`, so `defaultTargetPlatform` cannot be read there.
  Affect: an iOS password field autocorrects and a hint-specific keyboard is not chosen unless `autocorrect` and `keyboard_type` are set explicitly.

- Change: `want_keep_alive` reads a focus flag the focus listener updates instead of walking the node.
  Reason: language — `FocusNode::has_focus` needs `&mut App`, which the getter does not have.
  Affect: keep-alive follows the last focus change the state handled, so it lags a change no listener has run for yet.

- Change: `EditableText` cannot tell whether its selection controls are handle-controls, so the default `toolbar_options` follow the read-only / obscured table in every case.
  Reason: language — an erased `TextSelectionControls` cannot be tested for the mixin.
  Affect: handle-controls no longer force an empty toolbar unless `toolbar_options` is set.

- Change: the selection's glyph heights are measured from its first and last Unicode scalar, not its grapheme clusters.
  Reason: language — there is no `characters` package.
  Affect: a selection that starts or ends inside a multi-scalar cluster (an emoji ZWJ sequence) sizes from that scalar's box, or falls back to `preferredLineHeight`.

## widgets/system_context_menu.rs → system_context_menu.dart

- Change: custom item callback ids are a process-wide counter string, not Dart's `hashCode`.
  Reason: language — the item data is an enum, not an object with an identity hash.
  Affect: the host sees incrementing ids such as `"1"` rather than an object hash.

## widgets/text_selection.rs → text_selection.dart

- Change: `ClipboardStatusNotifier::update` and `TextSelectionControls::handle_paste` finish within the call.
  Reason: platform — `Clipboard::has_strings` and `paste_text` are synchronous.
  Affect: a paste lands in the same turn as the gesture that asked for it, where Dart's completes a frame or more later.

- Change: `show_toolbar` uses `context_menu_builder` whenever it is set, and only otherwise the deprecated `build_toolbar` path.
  Reason: language — an erased `TextSelectionControls` cannot be asked whether it is handle-controls.
  Affect: old-style controls combined with a context-menu builder show the builder, where Dart would still call `buildToolbar`.

- Change: each overlay owns its context-menu and spell-check `OverlayEntry`; there is no `ContextMenuController` singleton.
  Reason: language — Dart's static `_shownInstance` is isolate-global.
  Affect: two `SelectionOverlay`s can show two menus at once.

- Change: the selection handles' glyph heights are measured from the first and last Unicode scalar, not the grapheme clusters.
  Reason: language — there is no `characters` package.
  Affect: a selection that starts or ends inside a multi-scalar cluster (an emoji ZWJ sequence) sizes from that scalar's box, or falls back to `preferredLineHeight`.

## widgets/feedback.rs → feedback.dart

- Change: `for_tap` is a no-op; `for_long_press` calls only `HapticFeedback` (`vibrate` on Android/Fuchsia, `heavy_impact` on iOS).
  Reason: platform — `SystemSound` and semantics events are not on the host surface.
  Affect: a tap plays no click; iOS long-press has no click sound.

## widgets/tap_region.rs → tap_region.dart

- Change: `RenderTapRegionSurface` records the hit path on its own hit-test entry target instead of an `Expando` keyed by entry identity.
  Reason: language — `BoxHitTestResult` is a borrow of `HitTestResult` and `BoxHitTestEntry` is a value; neither can be an `Expando` key holding the path.
  Affect: the surface appears in the hit-test path as that target rather than as a `BoxHitTestEntry`.

- Change: `RenderTapRegion` registers after `perform_layout` and unregisters in `did_detach`, where Dart overrides `layout` and `dispose`.
  Reason: language — `layout` and `dispose` are not virtuals a leaf can wrap.
  Affect: a region stops being registered as soon as it detaches, not only when its slot is destroyed, and the next layout registers it again.

## widgets/magnifier.rs → magnifier.dart

- Change: `MagnifierController::show` inserts the builder as-is, where Dart wraps it in an `InheritedTheme` capture from the originating context.
  Reason: language — `InheritedTheme`'s `wrap` and `capture` are not ported.
  Affect: a magnifier in the root overlay does not inherit themes from the context that showed it.

## widgets/drag_target.rs → drag_target.dart

- Change: a drag payload is an `Rc<T>` and a target accepts a draggable by matching that concrete Rust type.
  Reason: language — Rust has no runtime subtype relation to test a payload against a target's type.
  Affect: a draggable and a target pair up only on the same type, so a shared payload model has to be one type (an enum), where Dart also matches subtypes.

- Change: an active drag retains its source state, entered targets and feedback avatar in the arena until it ends.
  Reason: language — no garbage collector, and the pointer callbacks outlive the widgets that started the drag.
  Affect: a drag continues after its source widget is removed, and the retained objects are released when it completes or is cancelled.

## window.rs → _window.dart

- Change: `Window` and `WindowScope` carry the host's `HostWindow`, a view with a frame, visibility and a native handle, where Flutter's `Window` carries a `WindowController` and its siblings each carry one archetype's controller; there is no `WindowManager` or registry, and a `Window` does not rebuild on its own when the window's size changes.
  Reason: platform — the host makes windows from one configuration and hands back a view, and a size change reaches the tree through the view's metrics, as it does for the implicit view.
  Affect: an app lists its `Window`s in a `ViewCollection` itself and reads the window from `WindowScope::of`; size and position come from `HostWindow::frame`, not a listenable.

## Deferred

- image.rs: `Image.network` and `Image.file`. Trigger: an HTTP client; a file system.
- image.rs: `Image.asset`'s bundle lookup, and the bundle `createLocalImageConfiguration` would carry, without which `AssetImage` cannot resolve from a widget. Trigger: `DefaultAssetBundle`.
- image.rs: the frame, loading and error builders, and `gaplessPlayback`. Trigger: a caller that wants to show something while an image loads or after it fails.
- image.rs: `precacheImage`. Trigger: a caller that warms an image before showing it.
- image.rs: `color` / `colorBlendMode`, `centerSlice` and `filterQuality`. Trigger: painting's colour filters; an image drawn as a resizable frame; a caller that needs to choose how an image is sampled.
- image.rs: `excludeFromSemantics` and the semantics label. Trigger: accessibility.
- focus_manager.rs: the manager's app-lifecycle listening and suspended-node handling. Trigger: `WidgetsBindingObserver.didChangeAppLifecycleState`.
- focus_manager.rs: `FocusNode.onKey` and the Android IME key check. Trigger: services' raw key path.
- focus_manager.rs, focus_scope.rs: the focus semantics actions and the `Semantics` wrapper `Focus.includeSemantics` controls. Trigger: accessibility.
- focus_manager.rs, focus_scope.rs: focus-tree diagnostics and the debug focus border. Trigger: diagnostics.
- focus_scope.rs: `_FocusInheritedScope` keeps the copy of the inherited-notifier element it grew before inherited_notifier.rs was ported. Trigger: the next change to that element.
- A disposed arena object keeps its slot — `dispose` does Dart's work and frees nothing — across the focus, scroll, draggable-sheet, restoration, action, shortcut, traversal and route objects. Trigger: an arena-slot lifetime for disposed foundation objects.
- framework/: `ErrorWidget` with its exception reporting, and the build-time diagnostics and duplicate-key checks. Trigger: diagnostics.
- framework/: `reassemble`. Trigger: hot reload.
- framework/: `ObjectKey`, `GlobalObjectKey`. Trigger: a widget keyed by object identity.
- binding.rs: the observer callbacks with no host event (memory pressure, the predictive-back gesture, push-route and route information, lifecycle), the platform menu delegate and windowing owner, `scheduleWarmUpFrame`, `framesEnabled`. Trigger: the shell forwarding those platform events; predictive back also needs a Java activity that registers with the system.
- binding.rs: first-frame-rasterized reporting and the push/pop route handlers. Trigger: `SchedulerBinding.addTimingsCallback`; navigation.
- view.rs: `LookupBoundary` around a `ViewAnchor`'s view, and the view's semantics owner callbacks. Trigger: `LookupBoundary`; accessibility.
- `LookupBoundary` in the `View`, `Overlay` and theater-marker lookups, which look straight up the tree. Trigger: `LookupBoundary` in framework.
- `debugCheckHasDirectionality` in the widgets that resolve a directional value. Trigger: diagnostics.
- `InheritedTheme` capture on `DefaultTextStyle`, `DefaultTextHeightBehavior` and `IconTheme`, and `DefaultTextStyle.merge`. Trigger: `InheritedTheme`; `Builder`.
- text.rs: selection, `locale`, `strutStyle`, the semantics labels, `WidgetSpan` children, and the deprecated `textScaleFactor`. Trigger: `SelectionContainer`; `Locale` / `StrutStyle`; inline children in `RenderParagraph`.
- basic.rs: `ListBody`, `Wrap`, `Flow`, `PositionedDirectional`, `PhysicalModel`, `PhysicalShape`, `RotatedBox`, `IgnoreBaseline`, `ShaderMask`, `RawImage`, `WidgetToRenderBoxAdapter`. Trigger: each one's render object.
- basic.rs: the key-wrapping helpers, `StatefulBuilder`, `DefaultAssetBundle`. Trigger: a key over object identity; a caller; asset bundles.
- basic.rs: the semantics widgets and the semantics flags on the pointer and opacity widgets. Trigger: accessibility (do not stub).
- `Transform.filterQuality`, and with it the transitions and implicit animations that pass it on. Trigger: `RenderTransform.filterQuality`.
- basic.rs: the inspector-only transform labels and the onstage-child debug walks. Trigger: diagnostics.
- icon.rs: `Icon.semanticLabel` and the semantics wrappers `Icon.build` returns. Trigger: accessibility (do not stub).
- icon.rs: icon diagnostics. Trigger: diagnostics.
- visibility.rs: `SliverVisibility`. Trigger: `SliverOpacity`, `SliverIgnorePointer` and `SliverOffstage`.
- visibility.rs: what `Visibility.maintainSemantics` controls. Trigger: accessibility (do not stub).
- visibility.rs: `Visibility` diagnostics. Trigger: diagnostics.
- image.rs: `Image`, `precacheImage`, the image-loading state, and the asset-bundle input of `create_local_image_configuration`. Trigger: the image cache; asset bundles.
- ticker_provider.rs: the ticker unregistration Dart's `_WidgetTicker` does, and `TickerMode.merge`. Trigger: a dispose hook on scheduler's `Ticker`; `Builder`.
- transitions.rs: `SliverFadeTransition`, and with it implicit_animations.rs's `SliverAnimatedOpacity`. Trigger: slivers.
- transitions.rs, implicit_animations.rs: diagnostics. Trigger: diagnostics.
- icon_theme: `IconThemeData.hashCode` (no `Hash` over `f64`). Trigger: a map keyed by `IconThemeData`.
- gesture_detector.rs: the semantics gesture handling, and the double-tap, vertical and pan drag, scale and force-press callbacks with their trackpad settings. Trigger: accessibility (do not stub); the remaining drag, scale and force-press recognizers.
- media_query.rs: `DisplayFeature`, `SystemTextScaler`, the platform sources for the accessibility flags, 24-hour format, text style overrides, corner radii and gesture insets, and the media-query debug checks. Trigger: observers in binding.rs; those members on `Platform` / `ViewMetrics`; diagnostics.
- widget_state.rs: `WidgetStateBorderSide`, `WidgetStateOutlinedBorder`, `WidgetStateTextStyle`, `WidgetStatePropertyAll`, `WidgetStatesController` as a `ValueListenable`, and hashing. Trigger: the first theme or input decoration that needs them; theme-data hashing.
- localizations.rs: the semantics wrapper `_LocalizationsState.build` adds, and the localization debug checks. Trigger: semantics; diagnostics.
- banner.rs: repaint on `PaintingBinding.systemFonts`. Trigger: a host reporting system font changes.
- app.rs: `WidgetsApp.router` and every router field it fills, with their branches of `_updateRouting` and `build`. Trigger: `router.dart`.
- app.rs: the state's route-push and route-pop handlers. Trigger: the binding's route events.
- app.rs: the app lifecycle state and the `SystemNavigator.setFrameworkHandlesBack` branch of the default navigation-notification handler. Trigger: lifecycle events; services' `SystemNavigator`.
- app.rs: the escape-key handler that dismisses tooltips. Trigger: `raw_tooltip.dart`.
- app.rs: the performance overlay, semantics debugger and widget inspector the three carried debug flags would show. Trigger: `performance_overlay.dart`; accessibility (do not stub); `widget_inspector.dart`.
- default_text_editing_shortcuts.rs: the Control-key editing bindings macOS supplies from its own key tables (Control-A, Control-E, Control-K, Control-D), which Dart's macOS map leaves to the host. Trigger: nothing else supplies them over a host that reports plain key presses.
- editable_text.rs: `_Editable` stays a leaf until `WidgetSpan` and host placeholders exist. Trigger: embedder placeholders.
- editable_text.rs: scribble and stylus support, the spell-check fetch, the magnifier overlay, `StrutStyle`, the floating cursor, `insertContent`, and the semantics wrappers. Trigger: `StrutStyle`; `UndoManagerClient`; `RenderEditable.setFloatingCursor`; accessibility.
- app_lifecycle_listener.rs: diagnostics. Trigger: diagnostics.
- text_selection.rs: the resume refresh of the clipboard and live-text status notifiers. Trigger: `WidgetsBindingObserver.didChangeAppLifecycleState`.
- text_selection.rs: the selection handle overlay and the empty selection controls; handle entries do not yet follow a composited transform. Trigger: those types; `EditableText`.
- text_selection.rs: the toolbar wrapper's outer tap region. Trigger: `SelectableRegion`.
- feedback.rs: the system sound in `for_tap` and iOS `for_long_press`, and the semantic events they send. Trigger: services' `SystemSound`; accessibility (do not stub).
- text_selection.rs: the Android stylus path and the `Scribe` availability check in `onTapDown`. Trigger: host `Scribe`.
- magnifier.rs: the decorative magnifier widgets and their render objects. Trigger: those widgets.
- tap_region.rs: the surface's semantics listeners and semantics action handling. Trigger: accessibility (do not stub).
- automatic_keep_alive.rs: diagnostics. Trigger: diagnostics.
- layout_builder.rs: `SliverLayoutBuilder` with the abstract generic builders, and the error widget on a builder panic. Trigger: slivers; diagnostics.
- sliver.rs: `SliverGrid`, `SliverOpacity`, `SliverIgnorePointer`, `SliverOffstage`, `SliverConstrainedCrossAxis`, `SliverCrossAxisExpanded`, `SliverCrossAxisGroup` and `SliverMainAxisGroup`. Trigger: each one's render sliver.
- sliver.rs: `SliverEnsureSemantics` and the adaptor element's onstage-child debug walk. Trigger: accessibility (do not stub); diagnostics.
- Two-dimensional scrolling: the two-dimensional viewport, its scrollable and scopes, its child delegates, `DiagonalDragBehavior`, and sliver.rs's `KeepAlive` under such a viewport. Trigger: `two_dimensional_viewport.dart`.
- scroll_delegate.rs: the keep-alive and semantic-index wrappers the delegates add, and the delegate description's exception branch. Trigger: wiring `AutomaticKeepAlive`; `selection_container.dart`; accessibility (do not stub); diagnostics.
- viewport.rs: the viewport diagnostics and the onstage-child debug walk. Trigger: diagnostics.
- Scroll semantics: `ScrollContext.setSemanticsActions`, scrollable.rs's semantics widget and its render object with `excludeFromSemantics` and `semanticChildCount`, and scroll_position.rs's semantic action bookkeeping. Trigger: accessibility (do not stub).
- scrollable.rs: the selection handler and container delegate around a scrollable. Trigger: `SelectionContainer` / `SelectionRegistrar`.
- scroll_configuration.rs: the `GlowingOverscrollIndicator` `build_overscroll_indicator` would build, with `OverscrollIndicatorNotification`; `PageScrollPhysics`. Trigger: `overscroll_indicator.dart`; `page_view.dart`.
- scrollable.rs, scroll_configuration.rs, primary_scroll_controller.rs: diagnostics, and `ScrollableDetails.hashCode` (no `Hash` over an erased physics). Trigger: diagnostics; a map keyed by scrollable details.
- scroll_view.rs: `GridView` and its four constructors. Trigger: `RenderSliverGrid`.
- scroll_view.rs: `ListView.prototypeItem` and its branch of the child layout. Trigger: `sliver_prototype_extent_list.dart`.
- scroll_view.rs: the scroll view diagnostics. Trigger: diagnostics.
- single_child_scroll_view.rs: the semantics clip and the approximate paint clip (deferred crate-wide in rendering's PORTING.md), and its diagnostics. Trigger: accessibility (do not stub); the inspector; diagnostics.
- scrollbar.rs: the painter's semantics and the structured error reports, which are plain assert messages here. Trigger: accessibility (do not stub); diagnostics.
- radio_group.rs: the radio-group semantics wrapper. Trigger: accessibility (do not stub).
- radio_group.rs: the post-frame check that at most one radio carries the group value. Trigger: diagnostics, and a post-frame callback usable from a state that may run without a binding.
- shortcuts.rs: menu serialization of the activators, the deprecated `isActivatedBy`, the duplicate-shortcut check, and `Shortcuts.includeSemantics`. Trigger: `platform_menu_bar.dart`; content equality on `ShortcutActivator`; diagnostics; accessibility.
- actions.rs, shortcuts.rs, focus_traversal.rs: diagnostics on the actions, intents, activators, managers, policies and orders. Trigger: diagnostics.
- navigator.rs, routes.rs, pop_scope.rs: the members Dart deprecates in favour of the pop-with-result and mark-for-complete ones. Trigger: never — port them only if Flutter un-deprecates them.
- navigator.rs: restoring the focused semantic node after a pop. Trigger: accessibility (do not stub).
- navigator.rs: the navigator diagnostics and the error reports around route generation. Trigger: diagnostics.
- routes.rs: the latency performance mode a transition requests from the engine. Trigger: scheduler's `requestPerformanceMode`.
- routes.rs: `ModalRoute`'s deprecated scoped will-pop callbacks, whose list everything else reads as always empty. Trigger: an async `WillPopCallback`, which needs an event loop — Flutter's own replacement `PopScope` is ported.
- routes.rs: the semantics wrappers around the modal barrier and the modal scope, and what `semanticsDismissible` controls. Trigger: accessibility (do not stub).
- routes.rs: `RawDialogRoute.anchorPoint` with the display-feature sub-screen and semantics wrappers, so `showGeneralDialog` has no `anchorPoint`. Trigger: `display_feature_sub_screen.dart`, which needs `MediaQuery.displayFeatures`; accessibility.
- modal_barrier.rs: the barrier's semantics widgets and clipper, and the platform gate on dismissing a barrier. Trigger: accessibility (do not stub).
- modal_barrier.rs: the alert sound when a non-dismissible barrier is tapped. Trigger: services' `SystemSound`.
- overlay.rs: the theater's intrinsics, dry layout and baselines, and the deferred layout box's dry baseline. Trigger: intrinsics and baselines through a theater child.
- overlay.rs: the overlay diagnostics, including the stack trace an invalidated entry location records (a flag stands in). Trigger: diagnostics.
- overlay.rs: the overlay's semantics wrappers, child identifiers and semantics updates. Trigger: accessibility (do not stub).
- overlay.rs: the theater's clip layer handle and the entry location's compositing-bits update. Trigger: layer handles and compositing bits in `inset-rendering`.
- overlay.rs: the deferred-child gates on the theater's `markNeedsLayout` and the layout box's dirty flag. Trigger: a public `RenderBox` super-hook for `markNeedsLayout` and a readable dirty flag in `inset-rendering`.
- heroes.rs: the hero diagnostics and the structured report behind the duplicate-tag check (a `debug_assert!` carrying Dart's message meanwhile). Trigger: diagnostics.
- implicit_animations.rs: `AnimatedPhysicalModel`; its tween slots are ready. Trigger: `PhysicalModel` (basic.rs).

