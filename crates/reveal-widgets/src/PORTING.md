# reveal-widgets/src
Syntax (constructors, setters, `Option`, `into_widget`) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/widgets
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical
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
- widgets/preferred_size.rs → preferred_size.dart

## framework/ → framework.dart

- Change: `set_state(app, |state| ..)` mutates the state through a closure and marks the element.
  Reason: language — the arena receiver rule applies to every object with identity, so the state is borrowed out of the `App` for the closure.
  Affect: `set_state` takes a closure over the state, which cannot reach the `App`, so work that needs it sits outside the closure.

- Change: a `State` is destroyed with its element at unmount; a retained `Handle<S>` is stale afterwards, and `mounted` is the only safe call on it.
  Reason: language — no garbage collector keeps Dart's disposed `State` alive for late callers; the arena frees the slot.
  Affect: the Dart guard `if (mounted) setState(..)` from a timer or listener works; reading `widget` or `context` on an unmounted state panics with a stale-handle message instead of returning the last widget.

- Change: the single- and multi-child render object elements are split into a data bag plus a base trait carrying Dart's bodies, and their widgets gained a defaulted `create_element`.
  Reason: language — Flutter both instantiates and subclasses those elements (`Viewport`'s, `SingleChildScrollView`'s), and Rust has no inheritance.
  Affect: an element with child bodies of its own holds the bag, calls the base bodies by name where Dart writes `super`, and its widget overrides `create_element`.

- Change: a panic in `build` unwinds; there is no `ErrorWidget` and no `FlutterError.reportError`, and the `of` lookups that carry a `FlutterError` message panic with it in every build.
  Reason: language — no catchable exception for ordinary control flow; diagnostics are deferred.
  Affect: a failing build takes the frame down instead of painting the red error box, and a failed lookup gives the same message in release as in debug.

- Change: `ParentDataWidget` gained a defaulted `debug_is_valid_render_object`, and `ParentDataElement` asks the widget which parent data it accepts instead of testing the type itself; `Positioned` overrides it to accept an overlay's `TheaterParentData` as well as a `StackParentData`.
  Reason: language — Dart's `_TheaterParentData extends StackParentData` passes an `is StackParentData` test; Rust has no subtyping between parent-data types.
  Affect: a `ParentDataWidget` whose Dart counterpart accepts a subclass of its parent data overrides `debug_is_valid_render_object` and resolves the concrete type in `apply_parent_data`.

## binding.rs → binding.dart

- Change: `WidgetsBindingObserver` is a trait carrying the metrics and locale callbacks the renderer raises; a `State` implements the object twin and registers `Rc::new(handle)`, which is removed again by that `Rc`'s identity.
  Reason: language — no mixin on a state; Dart's observer set compares identity.
  Affect: keep the registered `Rc` to remove it; the route, lifecycle, memory-pressure, back-gesture and view-focus callbacks wait with their platform events.

- Change: `WidgetsBinding::instance` assigns the brightness and locale callbacks on `App::platform_callbacks`, where Dart's `initInstances` assigns them onto `platformDispatcher`.
  Reason: platform — there is no dispatcher singleton to assign onto; the shell invokes what it finds there.
  Affect: observers hear `did_change_platform_brightness` and `did_change_locales` once a host reports the change through its client.

- Change: `run_app` schedules the root attach with a zero-duration `Timer` (Dart's `Timer.run`) and then a normal frame; there is no warm-up frame.
  Reason: platform — `scheduleWarmUpFrame` waits in `reveal-scheduler`.
  Affect: a host must `cell.elapse(..)` (or run its event loop) before the first frame builds anything; the first frame is a regular frame.

## widgets/banner.rs → banner.dart

- Change: `BannerPainter` answers no `repaint` listenable, where Dart repaints on `PaintingBinding.systemFonts`.
  Reason: platform — the binding raises no system-font change yet.
  Affect: a banner does not repaint when the system fonts change.

## widgets/basic.rs → basic.dart (+ `WidgetBuilder` from framework.dart, `Spacer` from spacer.dart, `IndexedStack` from indexed_stack.dart)

- Change: a Dart constructor assert runs on the setter that can violate it or, when it needs several fields, when the element creates or updates the render object. Every widget in this crate with such asserts follows this.
  Reason: language — a struct literal has no constructor body, and a fluent setter cannot see the arguments a later setter will supply.
  Affect: an invalid combination panics during build rather than where the widget is written, and a single-field assert fires at the setter, so a chain Dart's constructor would accept (`Positioned::fill(..).width(..)`) panics at `.width`.

- Change: the semantics flags (`Opacity.alwaysIncludeSemantics`, the deprecated `ignoringSemantics`) and `Transform.filterQuality` are absent; the same holds for the transitions and implicit animations built on these widgets.
  Reason: platform — accessibility is deferred, and `RenderTransform` carries no filter quality.
  Affect: there is no flag to pass; a transformed subtree is always applied by re-rendering the child, never rasterised through an image filter.

- `BackdropFilter`'s blend modes and anisotropic sigmas follow rendering's `RenderBackdropFilter` entry.

- Change: `MetaData.metaData` is `Option<Rc<dyn Any>>`.
  Reason: language — Dart's `dynamic`.
  Affect: pass `Rc::new(value)` and read it back through the render object with `downcast::<T>()`.

## widgets/container.rs → container.dart

- Change: `Container.color` and `decoration`'s mutual exclusion is asserted by whichever of the two setters runs second; the margin, padding and constraints asserts run on their own setters (basic.rs's assert entry).
  Reason: language — a struct literal has no constructor body.
  Affect: `.color(..).decoration(..)` panics at `.decoration` with Dart's message, in either order.

## widgets/ticker_provider.rs → ticker_provider.dart

- Change: Dart's `on State` mixins are traits over `State` with a data bag, and the state's own lifecycle hooks call the mixin's (`SingleTickerProviderStateMixin::activate(self, app)`, `::dispose`) where Dart's mixin body would run on its own. Every mixin in this crate (`RestorationMixin`, the scrollable's, the focus states') has this shape.
  Reason: language — a Rust trait holds no state and cannot interpose on another trait's hooks; there is no mixin linearization.
  Affect: a state that omits a call silently loses that part of the mixin's lifecycle: here a stale ticker-mode subscription after a `GlobalKey` move and a leaked listener on dispose.

- Change: Dart's `vsync: this` is `self`: the state implements scheduler's `TickerProviderObject` by forwarding to the mixin's `create_ticker`, and a blanket impl makes the handle the provider.
  Reason: language — orphan rule; a blanket over the mixin trait cannot implement the foreign twin.
  Affect: each state writes the three-line forwarding impl, then passes `self` to `AnimationController::create`.

- Change: `TickerMode::get_notifier` wraps the ancestor's notifier anew on every call, and the fallback is a fresh constant listenable.
  Reason: language — a `dyn` value has no identity of its own, and foundation's erased value form of a `Listenable` is deferred (this is its trigger).
  Affect: two results for one `TickerMode` are not `Rc::ptr_eq`; the mixins remove and re-add their listener on `activate` instead of keeping it.

- Change: a ticker vended by `TickerProviderStateMixin` is a plain `Ticker` and stays in the state's set after it is disposed.
  Reason: language — `Ticker::dispose` is an inherent method on a concrete arena struct; Dart's `_WidgetTicker` subclasses it to unregister.
  Affect: a long-lived state that creates and disposes many controllers keeps one handle per dead ticker until it disposes; muting a disposed ticker is a no-op.

## widgets/transitions.rs → transitions.dart

- Change: `AnimatedWidget::listenable` is an `Rc<dyn Listenable>`, and Dart's `widget.listenable != oldWidget.listenable` is `Rc::ptr_eq`.
  Reason: language — a `dyn` value has no identity of its own; the `Rc` is what is compared.
  Affect: keep one `Rc` and clone it into each rebuilt widget; wrapping a handle anew re-subscribes on every update (harmless).

- Change: `RelativeRectTween`, and every other tween this crate declares (implicit_animations.rs), is a `Clone` value that implements `Animatable` directly and is built without the `App`; `reveal_animation`'s tweens are arena objects.
  Reason: language — orphan rule: an `Animatable` impl on a `Handle` of a tween would name only foreign types.
  Affect: `controller.drive(app, RelativeRectTween::new(Some(a), Some(b)))` drives a clone, so writing `begin` or `end` afterwards does not reach the driven animation, as it does on a Dart `Tween`.

- `SizeTransition`'s constructor asserts run on its setters, the mutually exclusive `axis_alignment` / `alignment` on whichever runs second; see basic.rs's assert entry.
- `MatrixTransition` (so `ScaleTransition` and `RotationTransition`) has no `filterQuality` and `FadeTransition` no `alwaysIncludeSemantics`; see basic.rs's `Transform` entry.

## widgets/expansible.rs → expansible.dart

- Change: `ExpansibleController` is an arena `ChangeNotifier`, and `dispose` frees its slot.
  Reason: language — an object with identity and listeners lives in the arena.
  Affect: the handle is stale after `dispose`.

## widgets/icon_theme_data.rs → icon_theme_data.dart, widgets/icon_theme.rs → icon_theme.dart

- Change: `IconThemeData::resolve` is overridable through a `resolver` closure set on the value; Dart's `class CupertinoIconThemeData extends IconThemeData` is a constructor returning an `IconThemeData` carrying that override, which `copy_with` and `merge` keep, `lerp` drops, and `==` treats as Dart's `runtimeType` check.
  Reason: language — `IconTheme.data` is a value, not a subclassable object.
  Affect: `CupertinoIconThemeData::new()` returns an `IconThemeData`, not a subtype.

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

## widgets/image.rs → image.dart (`createLocalImageConfiguration` only)

- Change: `create_local_image_configuration` leaves `bundle` and `locale` unset.
  Reason: platform — `DefaultAssetBundle` and `Localizations` wait.
  Affect: an `ImageProvider` keyed by bundle or locale resolves against `None`.

## widgets/widget_state.rs → widget_state.dart

- Change: `WidgetStateMapper::resolve` panics with Dart's `ArgumentError` message when no key is satisfied, for every value type; a map that should resolve to `None` ends with a `WidgetState::any()` entry.
  Reason: language — Dart's `null as T` is a runtime test of `T`'s nullability, which a Rust generic cannot make without specialization.
  Affect: a mapper over an `Option` panics on an unmatched set instead of resolving `null` unless it carries the `any` entry.

- Change: `WidgetStateProperty.resolveAs` is `<dyn WidgetStateProperty<T>>::resolve_as`, and the value types that may be state properties (`AnyColor`, `MouseCursorRef`, and `Option` of either) answer the question themselves.
  Reason: language — Dart's `value is WidgetStateProperty<T>` is a runtime interface query.
  Affect: `<dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&widget.mouse_cursor, &states)`; a plain value comes back unchanged.

- Change: `WidgetStateColor` is one struct over Dart's three private subclasses and becomes a colour with `.into()`: an `AnyColor` whose plain value is the empty-state resolution, read back with `color.extension::<WidgetStateColor>()`.
  Reason: language — painting's `Color` is a value.
  Affect: a map-built colour read as a plain `Color` is its empty-state resolution (Dart throws), panicking only when no key matches the empty set.

- Change: `WidgetStateMouseCursor` is one struct over Dart's two private subclasses, and `clickable()` / `adaptive_clickable()` / `textable()` are fns returning fresh values.
  Reason: language — no `static const` object holding an `Rc`; no `noSuchMethod`.
  Affect: two `clickable()` results are not `==` (identity; Dart's `const` is canonical), so a `MouseRegion` handed a fresh one each build re-annotates; map-built cursors compare by map equality.

- Change: `WidgetStatesController` re-states `ValueNotifier`'s members directly, and its handle is a `Listenable`, not a `ValueListenable<WidgetStates>`.
  Reason: language — foundation's `ValueNotifier` is a leaf arena object with no bag or object twin to build on (orphan rule).
  Affect: passing it where a `ValueListenable` is wanted waits.

## framework: notifications → framework.dart (`Notification`, `NotifiableElementMixin`, `_NotificationNode`)

- Change: `NotificationListener<T>` takes a `NotificationTarget`: a concrete `Notification` type, or a family trait object such as `dyn ScrollNotification` matched through a defaulted vtable slot on `Notification`.
  Reason: language — Dart's `notification is T` covers subclasses; a trait family has no `TypeId` a downcast can name.
  Affect: `NotificationListener::<dyn ScrollNotification>::new(..)`; a new family adds a slot to `Notification` and an `impl NotificationTarget for dyn Family`.

## widgets/page_storage.rs → page_storage.dart

- Change: `PageStorageKey` is not generic, `PageStorageBucket` is an arena object, its data is `Rc<dyn Any>`, and an explicit identifier is a `KeyRef`.
  Reason: language — an erased key cannot be recognised as a generic type, and Dart's `Object` identifier needs hashing.
  Affect: read back with `bucket.read_state(app, context, None).and_then(|d| d.downcast_ref::<f64>().copied())`.

## widgets/layout_builder.rs → layout_builder.dart

- Change: the abstract generic layout builders collapse into `LayoutBuilder` over `BoxConstraints`, and the render-object mixin's callback is a `Listener` that reads the constraints from the render object instead of receiving them.
  Reason: language — one instantiation exists until `SliverLayoutBuilder`, and a callback taking `&mut App` cannot also borrow the constraints.
  Affect: `update_should_rebuild` is an inherent method, not an override point.

## widgets/scroll_delegate.rs → scroll_delegate.dart

- Change: `SliverChildDelegate` is a trait erased as `SliverChildDelegateRef`; `shouldRebuild`'s covariant parameter is `&dyn SliverChildDelegate` plus a defaulted `delegate_type()` the caller compares first and `as_any` for the downcast, and there is no `_createErrorWidget`.
  Reason: language — an abstract Dart class used as a type is a trait behind an `Rc`, there is no covariant parameter narrowing, and a failing builder unwinds (the framework's `ErrorWidget` entry).
  Affect: an implementor's `should_rebuild` downcasts through `old.as_any()`.

- Change: `SliverChildListDelegate::should_rebuild` compares the two child lists element by element (length, then `Rc::ptr_eq`).
  Reason: language — Dart compares the `List` references, and a Rust `Vec` has no identity.
  Affect: a new delegate built from a fresh `Vec` holding the same widget references does not rebuild its children, where Dart's list-identity check would; the rebuild it skips would have produced the same widgets.

## widgets/viewport.rs → viewport.dart

- `Viewport`'s center and cache-extent asserts run on whichever of the setters they need runs second; see basic.rs's assert entry.

## widgets/sliver_persistent_header.rs → sliver_persistent_header.dart

- Change: `SliverPersistentHeaderDelegate` is a trait erased as `SliverPersistentHeaderDelegateRef`, with `shouldRebuild`'s covariant parameter handled as scroll_delegate.rs's `SliverChildDelegate` is.
  Reason: language — an abstract Dart class used as a type is a trait behind an `Rc`, and there is no covariant parameter narrowing.
  Affect: an implementor downcasts `old_delegate.as_any()`.

## widgets/localizations.rs → localizations.dart

## widgets/default_text_editing_shortcuts.rs → default_text_editing_shortcuts.dart

- Change: Dart's `static final` shortcut tables are functions that build a fresh `ShortcutMap` per call (the two that read the target platform take `App`), and `intent_for_macos_selector` builds its intent per call.
  Reason: language — an `Rc` value cannot be a `const`, and the target platform is a property of the host-supplied `Platform`, not a library global.
  Affect: each build allocates the table it installs, and because a `ShortcutMap` compares by pairwise `Rc` identity (shortcuts.rs), the `ShortcutManager` re-indexes on every rebuild of this widget instead of seeing an unchanged const map.

## widgets/app.rs → app.dart

- `WidgetsApp`'s constructor asserts run when the widget is inflated, so a contradictory configuration panics at mount; see basic.rs's assert entry.

- Change: `WidgetsApp::default_shortcuts(app)` and `default_actions(app)` build a fresh value per call, and the actions are new arena objects.
  Reason: language — an `Rc` or an arena handle cannot be a `const`, and Dart's `defaultActions` is one mutable map of shared action objects.
  Affect: call `default_actions(app)`, modify the map, and pass it as `actions`; there is no global to assign into, and two apps do not share action instances.

- Change: the default navigation-notification handler stops the notification without telling the host, and there is no app lifecycle state.
  Reason: platform — the binding raises no lifecycle events and there is no `SystemNavigator.setFrameworkHandlesBack`, so Dart's lifecycle state would stay null, which is exactly its "not ready" branch.
  Affect: the host is never told whether the framework can handle a back gesture; an `on_navigation_notification` of your own still runs in place of the default.

## widgets/shared_app_data.rs → shared_app_data.dart

- Change: a key is any `Hash + Eq + 'static` value erased as `SharedAppDataKey`, and a value any `PartialEq + 'static` value (`Clone` to read it back); the map is shared between the model and the state and replaced on `set_value`, as Dart's copy is.
  Reason: language — Dart's `K extends Object` / `Object?` need hashing and equality that erased Rust values do not carry.
  Affect: a value type without `PartialEq` cannot be stored.

## widgets/annotated_region.rs → annotated_region.dart

- Change: `AnnotatedRegion<T>` needs `T: PartialEq + Debug`.
  Reason: language — rendering's `RenderAnnotatedRegion<T>` carries the `PartialEq` bound (its entry there), and a widget must be `Debug`.
  Affect: a value type without `PartialEq` or `Debug` cannot be annotated.

## widgets/scroll_context.rs → scroll_context.dart

- Change: `ScrollContext` takes `&self` and the `App`, a scrollable's `Handle` implements it, and a position holds the context as an `Rc<dyn ScrollContext>` compared by identity. The trait declares `type_name` for Dart's `runtimeType`, and `vsync` returns the scrollable's own handle as an `Rc<dyn TickerProvider>`.
  Reason: language — every scroll context lives in the arena, a Rust callback cannot capture what it mutates, and an `Rc` has no `==` by object identity.
  Affect: a scroll context implements `ScrollContext` for its `Handle` and hands its positions one `Rc::new(self)` for its lifetime — a fresh `Rc` per position fails `absorb`'s identity check.

## widgets/scroll_physics.rs → scroll_physics.dart

- Change: `ScrollPhysics` is a trait erased as `ScrollPhysicsRef`, its base bodies are associated functions on the `ScrollPhysicsBase` namespace, and the trait declares `as_any`.
  Reason: language — no inheritance; a second trait holding the base bodies would make every call ambiguous, and Dart's `runtimeType` comparison of a physics chain cannot be read off a `dyn` without `as_any`.
  Affect: inside an override, `ScrollPhysicsBase::method(self, ..)` where Dart writes `super.method(..)`; an implementor writes `as_any`. `apply_to` returns `ScrollPhysicsRef`, not the concrete type Dart's covariant return promises.

- Change: the deprecated `tolerance` getter reads the implicit view's device pixel ratio from `App::platform`, not an isolate-global window.
  Reason: platform — the host owns the views.
  Affect: `physics.tolerance(app)` panics without an implicit view; call `tolerance_for(metrics)`.

## widgets/scroll_notification.rs → scroll_notification.dart

- Change: `ViewportNotificationMixin`'s depth counter lives in a `Cell` inside a data bag, and Dart's `notification is ViewportNotificationMixin` is a defaulted `viewport_depth()` vtable slot on `Notification`.
  Reason: language — a notification bubbles as `&dyn Notification`, so the element that counts a viewport never holds it exclusively, and a mixin is not a type a downcast can name.
  Affect: a viewport-like element mixes in `ViewportElementMixin` and forwards the two notification virtuals from its `impl Element`.

- Change: `ScrollNotification` is a trait over a data bag; the five notifications are structs that answer Dart's `is ScrollNotification` through a defaulted vtable slot on `Notification`, which is what `NotificationListener<dyn ScrollNotification>` matches on. It does not extend `LayoutChangedNotification`.
  Reason: language — no inheritance, and the framework's listener matches by `TypeId`.
  Affect: a `NotificationListener<LayoutChangedNotification>` does not see scroll notifications.

## widgets/scroll_activity.rs → scroll_activity.dart

- Change: `ScrollActivityDelegate` and `ScrollActivity` are traits whose members take `self: Handle<Self>`, erased as `Any…` handles; `ScrollHoldController` takes `&self` and the `App`, implemented for a handle through `ScrollHoldControllerObject` and held as an `Rc<dyn ScrollHoldController>`; `ScrollActivity`'s fields are a data bag, its shared bodies associated functions on `ScrollActivityBase`, and Dart's `activity is ScrollHoldController` a defaulted vtable slot.
  Reason: language — no inheritance, and an erased reference to an arena object is an id plus a static table, or an `Rc` over the handle.
  Affect: an activity that overrides `dispose` ends with `ScrollActivityBase::dispose(self, app)`; `ScrollPosition::hold` hands back `Rc::new(activity)`.

## widgets/scroll_position.rs → scroll_position.dart

- Change: `ScrollPosition` is a trait over `ViewportOffset` with a `ScrollPositionData` bag, its shared bodies on the `ScrollPositionBase` namespace and the type-erased handle `AnyScrollPosition`; Dart's constructor body is `ScrollPositionBase::init`, called right after `App::create`, and a leaf's `impl ViewportOffset` forwards the members `ScrollPosition` overrides to the base.
  Reason: language — no inheritance, and a subtrait cannot give a supertrait's method a body.
  Affect: an implementor holds the bag, writes both impls, and calls `ScrollPositionBase::…` where Dart writes `super.…`.

- Change: `ScrollPosition` does not implement the `ScrollMetrics` trait; it re-declares those members with `self: Handle<Self>` plus `App`, and `copy_with(app)` is the `FixedScrollMetrics` snapshot handed to everything that reads the metrics — the physics, a `ScrollMetricsNotification` (whose `metrics` is an `Rc<dyn ScrollMetrics>`), `ScrollIncrementDetails`.
  Reason: language — `ScrollMetrics` takes `&self`, but the axis direction and device pixel ratio are live reads of the `ScrollContext`, which needs the arena.
  Affect: `physics.apply_boundary_conditions(&position.copy_with(app), value)` where Dart passes `this`; a `ScrollPhysics` sees a snapshot, not the live position.

- Change: the semantic-action bookkeeping and the `notifyListeners` override that ran it are gone.
  Reason: platform — `ScrollContext.setSemanticsActions` is deferred (accessibility).
  Affect: nothing schedules a semantics update when the offset or the dimensions change.

## widgets/scroll_position_with_single_context.rs → scroll_position_with_single_context.dart

- Change: `ScrollPositionWithSingleContext` is a base class in the leaf shape this crate uses wherever Flutter subclasses a concrete class: a data bag on the leaf, a `…Leaf` trait holding the overridable members, the shared bodies as associated functions an override calls by name where Dart writes `super`, and `…_overrides!()` macros that point the inherited virtuals at the trait. Dart's constructor body is `init`, called right after `App::create`.
  Reason: language — no inheritance, and a `Handle` carries no vtable, so a body a subclass overrides has to live on a namespace the subclass does not implement.
  Affect: `ScrollPositionWithSingleContext::new(app, ..)` is unchanged; a subclass (the draggable sheet's position, cupertino's sheet position) holds the bags, writes the macros, and overrides members on `ScrollPositionWithSingleContextLeaf`.

- Change: `absorb`'s `other is! ScrollPositionWithSingleContext` is a downcast to the leaf's own type rather than a test for the base class.
  Reason: language — the arena answers only the type a handle was created with, and the type-erased handle's vtable carries no "is a single-context position" slot.
  Affect: a position that absorbs one of a different leaf type goes idle and keeps its own scroll direction and drag, where Dart hands both over; every position a `Scrollable` replaces is made by the same controller, so the leaf types match in practice.

## widgets/scroll_configuration.rs → scroll_configuration.dart

- Change: `ScrollBehavior` is a trait whose bodies live on the `ScrollBehaviorBase` namespace (which takes the behavior so its `getPlatform` call stays virtual), and a field Dart types as `ScrollBehavior` is `ScrollBehaviorRef`. `copy_with` is a method on the ref that returns the public `WrappedScrollBehavior`; copying a wrapper flattens it, as Dart's override does.
  Reason: language — no inheritance, and `copyWith` captures `this` as the new wrapper's delegate, which only an `Rc` can be, not `&self`.
  Affect: a `build_scrollbar` override calls `ScrollBehaviorBase::build_scrollbar(self, app, ..)` for `super`, and no implementor can override `copy_with`.

- Change: `build_overscroll_indicator` returns the child on every platform.
  Reason: platform — `GlowingOverscrollIndicator` is not ported (`## Deferred`).
  Affect: a scroll view has no overscroll glow, so its tree is one node shallower.

## widgets/scroll_notification_observer.rs → scroll_notification_observer.dart

- Change: `ScrollNotificationCallback` is an `Rc` closure compared by `Rc::ptr_eq`, held in a `Vec` rather than Dart's intrusive linked list.
  Reason: language — a Rust closure has no identity.
  Affect: keep the `Rc` you gave `add_listener` and pass it back to `remove_listener`.

- Change: a listener that panics takes the frame down; there is no `FlutterError.reportError` around the dispatch.
  Reason: language — no catchable exception (the framework entry records the same).
  Affect: one failing listener stops the rest.

## widgets/scrollable.rs → scrollable.dart, widgets/scrollable_helpers.rs → scrollable_helpers.dart

- Change: `build` wraps the viewport in `Listener` → `RawGestureDetector` → `IgnorePointer` only: the semantics wrappers and the selection handler are gone, and `Scrollable` carries no `excludeFromSemantics` or `semanticChildCount` (`ScrollView` carries the latter but nothing reads it).
  Reason: platform — accessibility and selection are deferred (`## Deferred`).
  Affect: a scrollable exposes no scroll actions to a screen reader and cannot be selected across; its tree is three nodes shallower.

- Change: a pointer scroll signal is resolved through gestures' `PointerSignalResolver`, but the host is not told the event was handled.
  Reason: platform — `PointerEvent.respond` is omitted on signal events (reveal-gestures records it).
  Affect: an embedder that would scroll the page itself is not told the scrollable took the event.

## widgets/scroll_view.rs → scroll_view.dart

- Change: `ScrollView` is a trait over a `ScrollViewData` bag whose shared `build`, `buildViewport` and `getDirection` bodies live on `ScrollViewBase`, and `BoxScrollView` is the same shape one level down; the trait requires `Clone`.
  Reason: language — no inheritance, and `Scrollable.viewportBuilder` is an `Rc<dyn Fn>` that cannot borrow `self`, so `build` hands the closure a clone of the scroll view.
  Affect: a new scroll view holds the bags and writes the two impls, forwarding `StatelessWidget::build` to `ScrollViewBase::build`; `CustomScrollView::new().slivers(..)` and `ListView::new().children(..)` read as the Dart does.

- Change: Dart's `physics` constructor initializer, which substitutes an `AlwaysScrollableScrollPhysics` when the view is primary, runs in `ScrollViewBase::build`; the `physics` field holds what the caller passed (basic.rs's assert entry).
  Reason: language — a fluent setter cannot see the `primary`, `controller` and direction a later setter will supply.
  Affect: `view.physics` reads `None` where Dart's field already holds the implied physics; the `Scrollable` is given the same physics either way.

- Change: `ListView`'s default constructor keeps its `children` and `add*` arguments, so whichever of their setters runs last rebuilds the delegate out of all of them; `ListView::builder(..)` and `separated(..)` are values carrying the delegate's setters that convert to the widget, as `SliverList::builder(..)` is.
  Reason: language — a fluent setter cannot see the arguments a later setter will supply, and the delegate is built from four of them.
  Affect: any order of `.children(..)` and the three `add*` setters lands on the delegate Dart's constructor builds; those four setters panic on a builder-, separated- or custom-constructed `ListView`, whose own arguments are the builder value's setters.

## widgets/scrollbar.rs → scrollbar.dart

- Change: `RawScrollbar` and `RawScrollbarState` are base classes in the leaf shape (scroll_position_with_single_context.rs); `RawScrollbarStateLeaf` also asks for a `&mut self` bag accessor, which is what a `set_state` closure gets.
  Reason: language — no inheritance, and `Handle` carries no vtable.
  Affect: `CupertinoScrollbar` holds both bags and writes the impls; a caller writes `RawScrollbar::new(child).controller(..)` unchanged.

## widgets/focus_manager.rs → focus_manager.dart

- Change: the key-event and highlight-mode callbacks are `Rc` closures held in insertion order and removed by `Rc::ptr_eq`.
  Reason: language — a Rust closure has no identity, and a callback needs `App` to do anything.
  Affect: keep the `Rc` you passed to `add_early_key_event_handler` (or its siblings) and hand the same one back to remove it.

- Change: the manager receives key events through `HardwareKeyboard::add_handler`, one event at a time; there is no `KeyMessage`, no `keyMessageHandler` slot and no `RawKeyEvent`, so `FocusNode.onKey` and the Android virtual-keyboard check are absent.
  Reason: platform — services' raw key path and `KeyEventManager.keyMessageHandler` are deferred (`reveal-services/src/PORTING.md`).
  Affect: a node handles keys through `on_key_event` only; `register_global_handlers` adds one keyboard handler and one global pointer route, and `dispose` removes both.

- Change: `size`, `offset` and `rect` read the context's render object as a `RenderBox` and use its size where Dart reads `semanticBounds`.
  Reason: platform — `semanticBounds` waits with semantics; on a box it is exactly that rectangle.
  Affect: a focus node whose context hosts something other than a `RenderBox` panics instead of returning that object's paint bounds.

## widgets/focus_scope.rs → focus_scope.dart

- Change: `Focus.includeSemantics` is carried but nothing reads it, and the debug focus border is absent.
  Reason: platform — accessibility is deferred; diagnostics wait.
  Affect: a focusable widget exposes no semantics focusable / focused flags and cannot be focused by a semantics action.

## widgets/actions.rs → actions.dart

- Change: `Intent` is a trait whose values are `IntentRef`, and an `Actions` map is keyed by the intent's `TypeId` where Dart keys by `Type`; `Actions::find` / `maybe_find` take that `TypeId` instead of Dart's type argument or optional intent, and hand back the erased `AnyAction`.
  Reason: language — Rust has no implicit type argument and no runtime `Type` value, so both of Dart's spellings name the same `TypeId`.
  Affect: recover the leaf with `AnyAction::downcast::<MyAction>(app)`.

- Change: `Action<T>` is the trait `Action` with the associated type `Intent`, an `ActionData` bag and the type-erased handle `AnyAction`, which dispatches the enabled / invoke / key-result members through a hand-built vtable. An action bound to every intent (`DoNothingAction`) is `type Intent = dyn Intent`; `invoke` returns `Option<Rc<dyn Any>>` for Dart's `Object?`; `ContextAction` is a second trait whose members take the invoking context, and a leaf adds `context_action_overrides!()` to its `impl Action` — Dart's `this is ContextAction` test.
  Reason: language — no inheritance, no generic class with per-instance state behind a type-erased handle, and a runtime interface query on an erased object has to be answered by its vtable.
  Affect: `impl Action for MyAction { type Intent = MyIntent; action_accessors!(); fn invoke(..) }`; a context action adds `impl ContextAction` plus the macro, and calls `ContextAction::invoke(self, app, intent, context)` for `super`. An intent an action cannot cast panics where Dart asserts.

- Change: `ActionDispatcher` is a trait carrying Dart's two bodies as defaults, erased as `ActionDispatcherRef`; Dart's instantiable `const ActionDispatcher()` is `PlainActionDispatcher`.
  Reason: language — no inheritance, and a trait and a struct cannot share a name.
  Affect: `Rc::new(PlainActionDispatcher::new())` where Dart writes `const ActionDispatcher()`; two such values are distinct references, so an `Actions` widget that sets one rebuilds its dependents on every update.

- Change: `ActionListenerCallback` is an `Rc` closure removed by `Rc::ptr_eq`.
  Reason: language — a Rust closure has no identity.
  Affect: keep the `Rc` you passed to `add_action_listener` and hand the same one back to remove it.

- Change: `FocusableActionDetector.includeFocusSemantics` is carried but nothing reads it.
  Reason: platform — accessibility is deferred.
  Affect: a detector exposes no semantics focusable / focused flags.

## widgets/shortcuts.rs → shortcuts.dart

- Change: `Map<ShortcutActivator, Intent>` is `ShortcutMap`, an insertion-ordered list of activator and intent pairs whose equality is a pairwise `Rc::ptr_eq`; `CallbackShortcuts.bindings` is the same shape over `Listener`.
  Reason: language — an erased `ShortcutActivator` has no content hash, so it cannot be a map key.
  Affect: write the shortcuts as a list of pairs; two entries with equal activators both stay (Dart's map keeps the last), and reordering the list counts as a change.

- Change: Dart's `ShortcutManager` class is the trait `ShortcutManagerBase` over a data bag plus the leaf struct `ShortcutManager` and the type-erased handle `AnyShortcutManager`, which `Shortcuts.manager` holds; `handleKeypress`'s body lives on a crate-private sibling trait an override calls for `super`.
  Reason: language — no inheritance, and `radio_group.dart` subclasses the class and overrides `handleKeypress`, which a trait default cannot serve as `super` for.
  Affect: a custom manager is a struct with a `ShortcutManagerData` field that implements `ShortcutManagerBase` and `ChangeNotifier`; pass it as `manager.as_shortcut_manager()` and dispose it through `AnyShortcutManager::dispose`. `ShortcutManager::new(app)` is Dart's plain constructor.

## widgets/radio_group.rs → radio_group.dart

- Change: `RadioGroupRegistry<T>` and the `RadioClient<T>` mixin are traits whose methods take `self: Handle<Self>`, erased as `AnyRadioGroupRegistry<T>` / `AnyRadioClient<T>`; the client's one field is a bag under `radio_client_accessors!(T)`.
  Reason: language — no mixins, a Rust trait carries no state, and an erased reference to an arena object is an id plus a static table.
  Affect: a radio is a `State` that implements `RadioClient<T>` and assigns `self.set_registry(app, RadioGroup::<T>::maybe_of(app, context))` in `did_change_dependencies`.

- Change: `T` is `Copy + PartialEq + 'static`.
  Reason: language — the type-erased handles return the value through a vtable slot, so it has to be produced from behind a `&App`.
  Affect: a radio group's value type is a `Copy` key (an enum, an integer), where Dart takes any object.

## widgets/focus_traversal.rs → focus_traversal.dart

- Change: `FocusTraversalPolicy` is a trait with a data bag and the type-erased handle `AnyFocusTraversalPolicy`; a policy is an arena object created with `&mut App`, and every member takes `App`. `DirectionalFocusTraversalPolicyMixin` is a second trait with its own bag, and a policy that mixes it in adds `directional_focus_traversal_policy_overrides!()` to its `impl FocusTraversalPolicy` where Dart's mixin linearization would install the overrides.
  Reason: language — no inheritance and no mixin linearization; a Rust trait holds no state and cannot interpose on another trait's methods.
  Affect: a custom policy implements both traits and adds the macro, and omitting it silently loses directional traversal.

- Change: `FocusOrder` is a trait with `as_any`, its `compareTo` is the provided body and the covariant `doCompare` parameter is a downcast.
  Reason: language — there is no covariant parameter narrowing.
  Affect: an implementor's `do_compare` downcasts its argument through `as_any`.

- Change: `NextFocusAction` / `PreviousFocusAction` return their `bool` erased and read it back in `to_key_event_result`.
  Reason: language — `Action::invoke` returns Dart's `Object?` erased (actions.rs), so the covariant `toKeyEventResult` parameter arrives erased too.
  Affect: an action that refines `invoke`'s return type downcasts it in `to_key_event_result`.

## widgets/restoration.rs → restoration.dart, widgets/restoration_properties.rs → restoration_properties.dart

- Change: `RestorableProperty::dispose` ends with the post-hook `did_dispose`; a property whose Dart override runs work after `super.dispose()` implements that instead.
  Reason: language — a trait default cannot call itself as `super`, and overriding `dispose` would make the base body unreachable.
  Affect: `RestorableRouteFuture` (navigator.rs) implements `did_dispose`; a caller still calls `dispose`.

- Change: `RestorationMixin` is a trait over `State` with a data bag, in ticker_provider.rs's mixin shape: the state's own `did_change_dependencies`, `did_update_widget` and `dispose` call the mixin's counterparts, and `restoration_id` and `restore_state` are its two required members.
  Reason: language — a Rust trait holds no state and cannot interpose on another trait's hooks.
  Affect: a state that omits one of the three calls silently drops that part of the restoration lifecycle: no bucket is claimed, a renamed ID is not re-claimed, or the bucket outlives the state.

- Change: Dart's `RestorableProperty<Object?>` is the type-erased handle `AnyRestorableProperty`, and the mixin owner it points back to is `AnyRestorationMixin`; `RestorableProperty` names its value as an associated type, and that erased handle fuses Dart's `fromPrimitives` / `createDefaultValue` / `initWithValue` trio into one `restore(app, Option<&RestorationData>)`, where `None` is "nothing stored under this ID" and a stored null is `Some(RestorationData::Null)`.
  Reason: language — the mixin's property map is heterogeneous, and a type-erased handle cannot carry a `Value` from one of its slots into another.
  Affect: `property.state(app)` answers the type-erased state handle `AnyRestorationMixin`, so reach the concrete state with `state.downcast::<MyState>(app)`.

- Change: the root bucket is read within the call, so what Dart runs from the future's continuation runs inline and there is no first-frame deferral.
  Reason: platform — the host answers `Platform::restoration_get` within the call (reveal-services' `root_bucket`), so the wait the first frame would be deferred for cannot happen.
  Affect: the first frame is never held back and the empty container is never rendered; a `RootRestorationScope` hands its child a bucket on its very first build.

- Change: Dart's private primitive-value base classes are traits over a shared bag the leaf forwards to; Dart's `serialized as T` cast is the `RestorablePrimitive` trait (`bool`, `i64`, `f64`, `String`, and `Option` of each). `debugIsSerializableForRestoration` is gone: the value type decides what may be stored.
  Reason: language — no inheritance and no `Object?`.
  Affect: a property over another value type implements `RestorableValue` directly.

- Change: `RestorableValue::set_value` carries the setter's body and calls the `debug_assert_valid_value` pre-hook, where Dart's enum properties override the setter and then run the inherited one.
  Reason: language — a trait default body cannot be called as `super` from an override of itself.
  Affect: an implementor that must check a value before it is stored implements `debug_assert_valid_value` instead of overriding the setter.

- Change: `RestorableEnum` / `RestorableEnumN` serialize with `Debug` where Dart uses `EnumName.name`, and keep the `values` the constructor takes as a `Vec` (Dart keeps a `Set`).
  Reason: language — a Rust enum has no reflected variant name, and the bound is `Copy + Debug + PartialEq`, with no `Hash` or `Ord`.
  Affect: a value type with a hand-written `Debug` stores whatever that prints, so keep it the variant name; duplicates in `values` are kept rather than folded.

- Change: `RestorableChangeNotifier` requires `dispose_value(app, value)` where Dart's `_disposeOldValue` calls the virtual `dispose`, and `RestorableListenable`'s `notifyListeners` tear-off is `notification_listener()`, built once so the value's listener can be removed again.
  Reason: language — `Listenable` has no `dispose`, the wrapped notifier is a typed value without a vtable slot, and a Rust closure has no identity.
  Affect: an implementor says how its notifier is discarded; nothing else changes.

## widgets/overlay.rs → overlay.dart

- Change: the theater's parent data holds a `StackParentData` in a field instead of extending it, and every theater child gets a `TheaterParentData` where Dart's mixin gives a plain `StackParentData`.
  Reason: language — no inheritance; parent data is stored under one concrete type.
  Affect: a `ParentDataWidget` that writes a `StackParentData` reaches a theater child through `stack_mut()`; `Positioned` does (framework/), so an overlay entry and an `OverlayPortal` overlay child can still be positioned.

- Change: the theater does not suppress the `markNeedsLayout` that adopting or dropping an overlay child performs, and the deferred layout box always re-schedules its subtree instead of only when it is dirty or its constraints changed.
  Reason: language — a Rust impl cannot call the trait default body it overrides, so neither the outstanding-update counter nor the dirty flag can be maintained.
  Affect: showing or hiding an `OverlayPortal` child relays out the whole `Overlay`, and the overlay child's subtree is laid out again whenever the `Overlay` or the `OverlayPortal` lays out; the resulting tree is the same, the work is not.

- Change: the theater's and the deferred layout box's children are collected into a `Vec` before each layout, paint and hit-test walk, where Dart iterates a lazy generator that tolerates the child model changing mid-walk.
  Reason: language — a lazy iterator would borrow `App` for the whole walk, and the body of the walk needs `&mut App`.
  Affect: an overlay child added or removed during the `Overlay`'s own layout is picked up on the next layout pass, not the current one.

## widgets/navigator.rs → navigator.dart

- Change: `Route::will_pop` and `NavigatorState::maybe_pop` answer within the call, where Dart returns a future.
  Reason: language — the only asynchrony in Dart's deprecated `willPop` is the `_willPopCallbacks` loop it awaits, which is deferred with `WillPopScope` (see `## Deferred`), so nothing remains for a future to wait on.
  Affect: `if navigator.maybe_pop(app, result) { .. }` where Dart writes `if (await navigator.maybePop(result))`; when the callback loop is ported, both become `CompleterFuture`s.

- Change: Dart's type argument `T` on `Route<T>` / `Page<T>` / `TransitionDelegate<T>` is erased: a route result is `RouteResult = Option<Rc<dyn Any>>`.
  Reason: language — the navigator holds routes of mixed result types in one history, and a type-erased handle cannot carry a per-route type parameter.
  Affect: `navigator.pop(app, Some(Rc::new(42)))` and `result.and_then(|r| r.downcast::<i32>().ok())` where Dart writes `Navigator.pop(context, 42)` and `int? result`.

- Change: Dart's `abstract class Route<T>` is the trait `Route` plus a `RouteData` field; a field or parameter Dart types as `Route<dynamic>` holds the erased `AnyRoute`, `route as T` is `AnyRoute::downcast`, and `Route`'s own bodies live on `RouteBase`, so a subclass override that runs Dart's `super.install()` calls `RouteBase::install(self, app)`.
  Reason: language — no inheritance, no fat pointer for an arena id, and a trait default cannot call itself as `super`.
  Affect: a route is one arena struct holding `RouteData`; write `impl Route for MyRoute { crate::route_accessors!(); .. }` and reach a base body through `RouteBase::`.

- Change: Dart's `route is SomeMixin` for a mixin defined outside this crate is the type-keyed query `Route::interface(TypeId)`, read as `route.interface::<Rc<dyn SomeMixin>>()` on the erased handle — the `Error::provide` shape rendering's `RenderObject::interface` uses: a route answers that id with its handle as an `Rc<dyn SomeMixin>`, boxed.
  Reason: language — an erased route cannot be asked whether it implements a foreign trait, and one virtual per mixin would close the set of mixins to this crate.
  Affect: a route that mixes in cupertino's `CupertinoRouteTransitionMixin` overrides `interface` (its route-overrides macro does); the mixins this crate defines keep their `as_transition_route` / `as_modal_route` / `as_page_route` slots.

- Change: `Page::create_route` takes the page as an extra `this: &PageRef` argument, which the route's settings must be set to.
  Reason: language — Dart's `this` inside `createRoute` is the shared reference the navigator holds; a `&self` receiver cannot produce the `Rc<dyn Page>` the settings need.
  Affect: `fn create_route(&self, app, context, this: &PageRef) -> AnyRoute { MyRoute::new(app).settings(app, RouteSettingsRef::Page(Rc::clone(this))).as_route() }`.

- Change: `NavigatorObserver` is a trait plus a `NavigatorObserverData` field, with `Navigator::observers` holding the erased `AnyNavigatorObserver`; `TransitionDelegate` is `Rc<dyn TransitionDelegate>` and `RouteTransitionRecord` is a trait the route entry implements.
  Reason: language — the observer list holds objects of mixed types and every observer has arena identity.
  Affect: `impl NavigatorObserver for MyObserver { crate::navigator_observer_accessors!(); .. }`.

## widgets/routes.rs → routes.dart

- Change: Dart's route class chain (`Route` → `OverlayRoute` → `TransitionRoute` → `LocalHistoryRoute` → `ModalRoute` → `PopupRoute` / `PageRoute`) is one trait per level over a per-level field bag, all held on the leaf struct; a level's trait carries that level's bodies, and the leaf's `impl Route` forwards each overridden member to the deepest level that overrides it, which the `*_overrides!()` macros write. A super call is `Level::method(self, ..)`; the root's bodies live on `RouteBase`, and `ModalRoute::build_modal_barrier`'s on `ModalRouteBase`.
  Reason: language — no inheritance; a trait default cannot call itself as `super`, so a body a leaf overrides has to live on a trait the leaf does not implement with a forwarder.
  Affect: a concrete route is one struct holding the bags for every level it inherits, with one `impl` per level; write `crate::modal_route_overrides!()` inside `impl Route for MyRoute` and put the leaf's own `opaque`, `transition_duration` and `barrier_color` in the matching level's impl.

- Change: `RouteObserver<R extends Route>` takes as a constructor argument the `RoutePredicate` Dart expresses as the type argument, and `RouteAware` takes `&self` and the `App`, implemented for a handle through `RouteAwareObject` and held by `subscribe` as an `Rc<dyn RouteAware>` compared by identity.
  Reason: language — a Rust type parameter cannot answer "is this erased route an `R`" without a witness, the subscriber list holds objects of mixed types, and an `Rc` has no `==` by object identity.
  Affect: `RouteObserver::new(app, Rc::new(|app, route| route.as_page_route(app).is_some()))` where Dart writes `RouteObserver<PageRoute<dynamic>>()`; a subscriber keeps the `Rc` it passed to `subscribe` and hands the same one to `unsubscribe`.

- Change: `PopEntry` takes `&self` and the `App`, implemented for a handle through `PopEntryObject`, and a `ModalRoute` holds each registered entry as an `Rc<dyn PopEntry>` compared by identity.
  Reason: language — every pop entry lives in the arena, a Rust callback cannot capture what it mutates, and an `Rc` has no `==` by object identity.
  Affect: a state that registers itself mints `Rc::new(self)` once and unregisters with that same `Rc`, as `PopScopeState` does.

- Change: `RawDialogRoute.buildPage` returns the page builder's widget directly, and there is no `anchorPoint`.
  Reason: platform — the `Semantics` and `DisplayFeatureSubScreen` wrappers Dart adds are deferred, and `anchorPoint` exists only to feed the latter.
  Affect: a dialog is not confined to the display-feature sub-screen closest to an anchor point.

- Change: `RawDialogRoute` is a base class in the leaf shape (scroll_position_with_single_context.rs), with `raw_dialog_route_modal_route_overrides!()` and `raw_dialog_route_transition_route_overrides!()` pointing the inherited virtuals at `RawDialogRouteLeaf`.
  Reason: language — no inheritance, and `Handle` carries no vtable.
  Affect: `CupertinoDialogRoute` holds the bag and writes the impls.

## widgets/heroes.rs → heroes.dart

- Change: Dart's `Tween<Rect?>` — what a `CreateRectTween` returns and what a flight holds — is `Rc<dyn RectTweenObject>`, a trait `reveal_animation`'s `RectTween` implements; the flight's diverted `ReverseTween` is a private reversal over that erased tween.
  Reason: language — Rust has no subtyping, so `RectTween` (an arena type of its own) and a custom arc tween share no nameable base; and `reveal_animation`'s `ReverseTween` takes a concrete handle and does not swap `begin` / `end`, which a diverted flight reads.
  Affect: a tween type other than `RectTween` implements `RectTweenObject` to be returned from `create_rect_tween`.

## widgets/implicit_animations.rs → implicit_animations.dart

- Change: this file's tweens (`EdgeInsetsTween`, `DecorationTween`, `Matrix4Tween`, `BorderRadiusTween` and the rest) are value tweens in transitions.rs's `RelativeRectTween` shape; the ones over a nullable Dart value animate an `Option`, and their `TweenSlot` impls flatten the endpoints, so an unset endpoint and one set to `None` are the same thing. `reveal_rendering`'s alignment tweens have the same shape for the same reason.
  Reason: language — the same orphan rule; Dart's `BorderRadius??` flattens and Rust's `Option<Option<_>>` does not, and `_constructTweens`' `tween.end ?? tween.begin` depends on that.
  Affect: drive a clone, as there; `BorderRadiusTween::new(begin, end)` takes and returns `Option<BorderRadius>`, and its `transform` at `t == 0` returns `begin` rather than panicking when it is `None`. `Matrix4Tween` interpolates the host's f32 matrices, so a round trip through `decompose` / `compose` is exact only to f32.

- Change: Dart's `TweenVisitor<dynamic>` is the `TweenVisitor` trait whose one `visit` method is generic over a `TweenSlot`, and `for_each_tween` takes `&mut V` rather than a closure; a slot holds whichever tween the property needs, and `TweenSlot` is the `begin` / `end` pair both the arena and the value tweens answer.
  Reason: language — no closure can be `Tween<dynamic>`-polymorphic over the several tween types one state holds, and `Box<dyn Decoration>` is a tween value that cannot implement `Clone`.
  Affect: a state's `for_each_tween` writes `visitor.visit(app, tween, target, constructor)` per slot and stores the result back, as Dart does; a tween type usable in a slot implements `TweenSlot`, and its value `TweenValue`.

- Change: `ImplicitlyAnimatedWidget` is a trait over a data bag, and `ImplicitlyAnimatedWidgetState` / `AnimatedWidgetBaseState` are traits over a state bag holding the controller and the curved animation; a leaf's `impl State` forwards `init_state`, `did_update_widget` and `dispose` to the trait bodies where Dart inherits them, and carries `SingleTickerProviderStateMixin` itself.
  Reason: language — no inheritance and no mixins, and a state is created without the `App`.
  Affect: an implicitly animated widget holds the bag and implements the trait; its state holds the bags and implements the state trait(s), `SingleTickerProviderStateMixin` and `TickerProviderObject`.

- Change: `ImplicitlyAnimatedWidgetState::dispose` runs `for_each_tween` once more with a visitor that frees each slot's tween; Dart's `dispose` leaves its tweens alone.
  Reason: language — no garbage collector: a `Handle<Tween<f64>>` a slot's constructor made would hold its arena slot for the life of the `App`.
  Affect: a `Handle<Tween<T>>` a state made is stale once the state is disposed; a slot that `for_each_tween` skips (Dart's `_AnimatedAlignState` skips one when its factor is null) is not freed.

- `AnimatedScale` and `AnimatedRotation` have no `filterQuality`, and `AnimatedOpacity` no `alwaysIncludeSemantics`; see basic.rs's `Transform` entry.
- `AnimatedContainer`'s colour / decoration exclusion and its other constructor asserts run on the setters, as container.rs records; `width` and `height` fold into `constraints` on assignment, as `Container`'s do.

## widgets/visibility.rs → indexed_stack.dart (`Visibility`)

Flutter folded `visibility.dart` into `indexed_stack.dart`; this file keeps the old name, and the shared scope lives in basic.rs with the rest of `IndexedStack`.

- `Visibility`'s five `maintain*` asserts run on the widget's first build, so any order of the setter chain is accepted; see basic.rs's assert entry.

- Change: `maintain_semantics` is carried but nothing reads it.
  Reason: platform — accessibility is deferred.
  Affect: a hidden child is never reported to accessibility tools, `maintain_semantics` or not.

## widgets/inherited_notifier.rs → inherited_notifier.dart

- Change: `InheritedNotifier<T>` is a trait over `InheritedWidget` with the associated type `Notifier`; a widget that is both names its own `IntoWidget` conversion, `inherited_notifier_overrides!()` writes Dart's `updateShouldNotify` inside its `impl InheritedWidget`, and the element repeats `InheritedElement`'s bodies rather than inheriting them.
  Reason: language — a blanket `impl Widget` per kind would overlap, a trait default cannot fill a supertrait's method, and `InheritedElement` is a concrete struct with no override point.
  Affect: write `impl InheritedNotifier for MyScope { type Notifier = MyNotifier; fn notifier(&self) -> Option<Handle<MyNotifier>> }`, the macro in its `impl InheritedWidget`, and an inherent `into_widget` naming `InheritedNotifierKind`.

## widgets/draggable_scrollable_sheet.rs → draggable_scrollable_sheet.dart

- Change: `_replaceExtent`'s `widget.snapSizes != oldWidget.snapSizes` compares the two lists by value.
  Reason: language — Dart's `List` `==` is identity, and a `Vec` has none.
  Affect: rebuilding a snapping sheet with a fresh list of the same sizes does not schedule the post-frame snap Dart schedules; the snap targets are the same either way, so the snap it skips would move the sheet nowhere new.

## Deferred

- focus_manager.rs: `FocusManager.listenToApplicationLifecycleChangesIfSupported`, `_AppLifecycleListener`, `_appLifecycleChange`, `_respondToLifecycleChange`, `_suspendedNode`. Trigger: `WidgetsBindingObserver.didChangeAppLifecycleState`.
- focus_manager.rs: `FocusNode.onKey` / `FocusOnKeyCallback` and `_HighlightModeManager._isKeyMessageFromAndroidIME`. Trigger: services' raw key path.
- focus_manager.rs: `_HighlightModeManager.handleSemanticsAction`; focus_scope.rs: the `Semantics` wrapper `Focus.includeSemantics` controls and `_FocusState`'s `onFocus` semantics action. Trigger: accessibility.
- focus_manager.rs: `debugFocusChanges`, `_focusDebug`, `debugDescribeFocusTree`, `debugDumpFocusTree`, and `debugFillProperties` / `debugDescribeChildren` / `toStringShort` on the nodes and the manager; focus_scope.rs: `_DebugFocusBorder` and `debugPaintFocusBoxes`. Trigger: diagnostics.
- focus_scope.rs: `_FocusInheritedScope` keeps the copy of `_InheritedNotifierElement<FocusNode>` it grew before inherited_notifier.rs was ported, instead of implementing `InheritedNotifier`. Trigger: the next change to that element.
- A disposed arena object keeps its slot — `dispose` does Dart's work and frees nothing — for `FocusNode`, `FocusScopeNode` and `FocusAttachment`; `ScrollPosition`, `ScrollController`, `ScrollActivity` and `ScrollDragController`; the draggable sheet's `_DraggableSheetExtent`, `_ResetNotifier` and the `AnimationController`s its `animateTo` and `goBallistic` mint; `RestorableProperty`; `Action`, `ActionDispatcher`, `ShortcutManager`, `ShortcutRegistry` and `FocusTraversalPolicy` (with the default policies traversal creates for a group or a secondary sort, which are never freed); and every route. Trigger: an arena-slot lifetime for disposed foundation objects.
- framework/: `ErrorWidget`, `_reportException`, `debugWidgetBuilderValue` and the build timeline; `MultiChildRenderObjectElement`'s `_debugCheckHasAssociatedRenderObject` (its whole `inflateWidget` override) and `debugChildrenHaveDuplicateKeys`; `Element.debugFillProperties` and the other diagnostics, `DebugCreator`, `debugGetCreatorChain`. Trigger: diagnostics.
- framework/: `reassemble`. Trigger: hot reload.
- framework/: `ObjectKey`, `GlobalObjectKey`. Trigger: a widget keyed by object identity.
- binding.rs: `WidgetsBindingObserver`'s remaining callbacks (locale, lifecycle, memory pressure, back gestures, view focus), `handleAppLifecycleStateChanged`, `performReassemble`, the platform menu delegate and windowing owner, `scheduleWarmUpFrame`, `framesEnabled`. Trigger: the shell forwarding those platform events.
- binding.rs: `firstFrameRasterized` / `waitUntilFirstFrameRasterized` / `debugDidSendFirstFrameEvent`, `handlePushRoute` / `handlePopRoute` / `handlePushRouteInformation`, `renderViewElement`. Trigger: `SchedulerBinding.addTimingsCallback`; navigation.
- view.rs: `ViewCollection`, `ViewAnchor`, `View.forgetChild` diagnostics, `_ViewState`'s `_scopeFocusChangeListener` and `didChangeViewFocus`; the semantics owner callbacks on the view's pipeline owner. Trigger: multi-view; the platform's view-focus channel; accessibility.
- `LookupBoundary` in `View.of` / `maybe_of`, `Overlay.of` / `maybeOf` and `_RenderTheaterMarker.maybeOf`, which look straight up the tree. Trigger: `LookupBoundary` in framework.
- `debugCheckHasDirectionality` in `RichText`, `Icon.build`, `Stack` / `_RawIndexedStack` / `Flex` (before resolving a directional alignment) and `ScrollView.buildViewport`. Trigger: diagnostics.
- `InheritedTheme` (`wrap`, `captureAll`) on `DefaultTextStyle`, `DefaultTextHeightBehavior` and `IconTheme`; `DefaultTextStyle.merge` (needs `Builder`). Trigger: `InheritedTheme`; `Builder`.
- text.rs: selection (`selectionRegistrar`, `selectionColor`, `_SelectableTextContainer`), `locale`, `strutStyle`, `semanticsLabel` / `semanticsIdentifier`, `WidgetSpan` children (`RichText` is a leaf), the deprecated `textScaleFactor`. Trigger: `SelectionContainer`; `Locale` / `StrutStyle`; inline children in `RenderParagraph`.
- basic.rs: `ListBody`, `Wrap`, `Flow`, `PositionedDirectional`, `PhysicalModel`, `PhysicalShape`, `RotatedBox`, `IgnoreBaseline`, `CustomSingleChildLayout`, `ShaderMask`, `CompositedTransformTarget` / `CompositedTransformFollower`, `RawImage`, `WidgetToRenderBoxAdapter`. Trigger: each one's render object.
- basic.rs: `RepaintBoundary.wrap` / `wrapAll`, `KeyedSubtree.wrap` and `ensureUniqueKeysForList` (they need a key over Dart's `Object`), `StatefulBuilder`, `DefaultAssetBundle`. Trigger: a key over object identity; a caller; asset bundles.
- basic.rs: `Semantics`, `MergeSemantics`, `BlockSemantics`, `ExcludeSemantics`, `IndexedSemantics`, `SliverSemantics`; the deprecated `IgnorePointer.ignoringSemantics` and `AbsorbPointer.ignoringSemantics`; `Opacity.alwaysIncludeSemantics` and with it `FadeTransition`'s and `AnimatedOpacity`'s. Trigger: accessibility (do not stub).
- `Transform.filterQuality`, and with it `MatrixTransition`'s (so `ScaleTransition`'s and `RotationTransition`'s), `AnimatedScale`'s and `AnimatedRotation`'s. Trigger: `RenderTransform.filterQuality`.
- basic.rs: `ConstraintsTransformBox.debugTransformType` (the inspector-only transform labels), `_OffstageElement.debugVisitOnstageChildren` and `_IndexedStackElement.debugVisitOnstageChildren` (both use the plain element), `ParentDataWidget.debugTypicalAncestorWidgetClass` (the "incorrect use of a ParentDataWidget" message is `ParentDataElement::apply_parent_data`'s assert instead). Trigger: diagnostics.
- icon.rs: `Icon.semanticLabel` and the `Semantics` / `ExcludeSemantics` wrappers `Icon.build` returns. Trigger: accessibility (do not stub).
- icon.rs: `IconDataProperty` and `Icon.debugFillProperties`. Trigger: diagnostics.
- visibility.rs: `SliverVisibility`, with its `_SliverVisibility` and `_RenderSliverVisibility`. Trigger: `SliverOpacity`, `SliverIgnorePointer` and `SliverOffstage` (sliver.rs's bullet below).
- visibility.rs: what `Visibility.maintainSemantics` controls — `_Visibility.maintainSemantics`, `_RenderVisibility.maintainSemantics` and its `visitChildrenForSemantics`. Trigger: accessibility (do not stub).
- visibility.rs: `Visibility.debugFillProperties`. Trigger: diagnostics.
- image.rs: `Image`, `precacheImage`, the image-loading state; the `DefaultAssetBundle` input of `create_local_image_configuration`. Trigger: the image cache; asset bundles.
- ticker_provider.rs: `_WidgetTicker` / `TickerProviderStateMixin._removeTicker` (needs a dispose hook on scheduler's `Ticker`), `TickerMode.merge` (needs `Builder`). Trigger: a state that churns controllers; `Builder`.
- transitions.rs: `SliverFadeTransition`, and with it implicit_animations.rs's `SliverAnimatedOpacity`. Trigger: slivers.
- transitions.rs and implicit_animations.rs: every `debugFillProperties`. Trigger: diagnostics.
- icon_theme: `IconThemeData.hashCode` (no `Hash` over `f64`). Trigger: a map keyed by `IconThemeData`.
- gesture_detector.rs: `excludeFromSemantics`, `semantics`, `SemanticsGestureDelegate`, `_GestureSemantics`, `replaceSemanticsActions`, `RenderSemanticsGestureHandler`; the `onDoubleTap*`, `on*Drag*`, `onPan*`, `onScale*` and `onForcePress*` callbacks with `dragStartBehavior`, `trackpadScrollCausesScale`, `trackpadScrollToScaleFactor` and the `getMultitouchDragStrategy` lookup. Trigger: accessibility (do not stub); the drag, scale and force-press recognizers. `onTapMove` is a field Flutter does not wire at this commit either.
- media_query.rs: `DisplayFeature` (`displayFeatures`, `removeDisplayFeatures`, `displayFeaturesOf`, its aspect); `SystemTextScaler`; platform sources for the accessibility flags, `alwaysUse24HourFormat`, `supportsShowingSystemContextMenu`, text style overrides, `displayCornerRadii`, `systemGestureInsets`; `debugCheckHasMediaQuery`, `debugBrightnessOverride`; `EdgeInsets.fromViewPadding` lives as a private fn here until painting takes it. Trigger: observers in binding.rs; `DisplayFeature` and those members on `Platform` / `ViewMetrics`; the debug files; painting.
- widget_state.rs: `WidgetStateBorderSide` / `_LerpSides` (needs an extension slot on painting's Copy `BorderSide`), `WidgetStateOutlinedBorder` (needs a default-state stand-in over `Box<dyn OutlinedBorder>`), `WidgetStateTextStyle` (needs an extension slot on the value `TextStyle`), `WidgetStatesController` as `ValueListenable<WidgetStates>` (foundation `ValueListenableObject` twin or `ValueNotifierData` bag), `hashCode` / `Hash` on constraints, mappers, `WidgetStatePropertyAll`, `WidgetStateMapper.noSuchMethod`, `Diagnosticable`. Trigger: `ChipThemeData.side` / `.shape`, `InputDecoration.labelStyle`, the first consumer holding the controller as a `ValueListenable`, theme-data hashing, diagnostics.
- localizations.rs: the `Semantics(localeForSubtree:, container:, textDirection:)` wrapper in `_LocalizationsState.build`, `LocalizationsResolver._debugCheckLocalizations` (a reported warning), `debugCheckHasWidgetsLocalizations`, `Localizations.debugFillProperties`. Trigger: semantics; diagnostics.
- banner.rs: repaint on `PaintingBinding.systemFonts`. Trigger: a host reporting system font changes.
- app.rs: `WidgetsApp.router` and every router field it fills (`routeInformationProvider`, `routeInformationParser`, `routerDelegate`, `routerConfig`, `backButtonDispatcher`), with `_usesRouterWithDelegates` / `_usesRouterWithConfig`, `_clearRouterResource`, `_effectiveRouteInformationProvider`, `_effectiveBackButtonDispatcher` and their branches of `_updateRouting` and `build`. Trigger: `router.dart`.
- app.rs: `_WidgetsAppState.didPopRoute` and `didPushRouteInformation`. Trigger: the binding's route events (`WidgetsBindingObserver`'s route callbacks).
- app.rs: `_appLifecycleState`, `didChangeAppLifecycleState`, and the `SystemNavigator.setFrameworkHandlesBack` branch of `_defaultOnNavigationNotification`. Trigger: `WidgetsBindingObserver.didChangeAppLifecycleState` and services' `SystemNavigator`.
- app.rs: the `TapRegionSurface` between the `FocusTraversalGroup` and the `ShortcutRegistrar`, and the escape-key handler of the `Focus` above the title, which calls `RawTooltip.dismissAllToolTips` (the `Focus` itself is built, without an `onKeyEvent`). Trigger: `tap_region.dart`; `raw_tooltip.dart`.
- app.rs: `showPerformanceOverlay`'s `PerformanceOverlay` stack, `showSemanticsDebugger`'s `SemanticsDebugger`, and `debugShowWidgetInspector`'s `ValueListenableBuilder` over `WidgetsBinding.debugShowWidgetInspectorOverrideNotifier` with the `WidgetInspector` and its three button builders (`exitWidgetSelectionButtonBuilder`, `moveExitWidgetSelectionButtonBuilder`, `tapBehaviorButtonBuilder`) and the deprecated `debugShowWidgetInspectorOverride` pair; the three flags are carried. Trigger: `performance_overlay.dart`; accessibility (do not stub); `widget_inspector.dart`.
- default_text_editing_shortcuts.rs: the text editing actions that consume these intents live in `editable_text.dart`. Trigger: `EditableText`.
- layout_builder.rs: `SliverLayoutBuilder` and the abstract generics, `ErrorWidget` on a builder panic. Trigger: slivers; diagnostics.
- sliver.rs: `SliverGrid` and `SliverOpacity` / `SliverIgnorePointer` / `SliverOffstage` (with `_SliverOffstageElement`), `SliverConstrainedCrossAxis` (with `_SliverZeroFlexParentDataWidget` and `_SliverConstrainedCrossAxis`), `SliverCrossAxisExpanded`, `SliverCrossAxisGroup`, `SliverMainAxisGroup` (with `_SliverMainAxisGroupElement`). Trigger: `RenderSliverGrid`, `RenderSliverOpacity`, `RenderSliverIgnorePointer`, `RenderSliverOffstage`, `RenderSliverConstrainedCrossAxis`, `RenderSliverCrossAxisGroup`, `RenderSliverMainAxisGroup`.
- sliver.rs: `SliverEnsureSemantics` and `_RenderSliverEnsureSemantics`; `SliverMultiBoxAdaptorElement.debugVisitOnstageChildren`. Trigger: accessibility (do not stub); diagnostics.
- Two-dimensional scrolling: sliver.rs's `KeepAlive` under a `TwoDimensionalViewport` (which carries a second `KeepAliveParentDataMixin` parent data, where `KeepAlive` names `SliverMultiBoxAdaptorParentData` only); scroll_delegate.rs's `TwoDimensionalChildDelegate`, `TwoDimensionalChildBuilderDelegate`, `TwoDimensionalChildListDelegate`; scrollable.rs's `TwoDimensionalScrollable`, `TwoDimensionalScrollableState`, `_TwoDimensionalScrollableScope`, `_VerticalOuterDimension`, `_HorizontalInnerDimension`, `DiagonalDragBehavior`, `TwoDimensionalViewportBuilder`. Trigger: `two_dimensional_viewport.dart`.
- scroll_delegate.rs: the `AutomaticKeepAlive` wrapper of `addAutomaticKeepAlives` (with `KeepAliveNotification`, `KeepAliveHandle`, `AutomaticKeepAliveClientMixin`) and the `_SelectionKeepAlive` inside it; the `IndexedSemantics` wrapper of `addSemanticIndexes` (the flag, `semanticIndexCallback` and `semanticIndexOffset` are carried and read); `SliverChildDelegate.debugFillDescription`'s "EXCEPTION" branch. Trigger: `automatic_keep_alive.dart` and `selection_container.dart`; accessibility (do not stub); diagnostics.
- viewport.rs: `_ViewportElement.debugVisitOnstageChildren` and both viewports' `debugFillProperties`. Trigger: diagnostics.
- `ScrollContext.setSemanticsActions`; scrollable.rs: `_ScrollSemantics` / `_RenderScrollSemantics`, `Scrollable.excludeFromSemantics` and `semanticChildCount`, `ScrollableState.setSemanticsActions`, `_scrollSemanticsKey` and `_handleScrollMetricsNotification`, the `Semantics` wrapper `build` puts around the `IgnorePointer`; scroll_position.rs: `_updateSemanticActions` and `_semanticActions`. Trigger: accessibility (do not stub).
- scrollable.rs: `_ScrollableSelectionHandler`, `_ScrollableSelectionHandlerState`, `_ScrollableSelectionContainerDelegate`, `_getDeltaToScrollOrigin`. Trigger: `SelectionContainer` / `SelectionRegistrar`.
- scroll_configuration.rs: the `GlowingOverscrollIndicator` `buildOverscrollIndicator` would build, with `OverscrollIndicatorNotification`; `PageScrollPhysics`. Trigger: `overscroll_indicator.dart`; `page_view.dart`.
- scrollable.rs / scroll_configuration.rs / primary_scroll_controller.rs: `debugFillProperties` on `Scrollable`, `ScrollableState`, `ScrollConfiguration` and `PrimaryScrollController`; `ScrollableDetails.hashCode` (no `Hash` over `Rc<dyn ScrollPhysics>`). Trigger: diagnostics; a map keyed by scrollable details.
- scroll_view.rs: `GridView` and its four constructors. Trigger: `RenderSliverGrid`.
- scroll_view.rs: `ListView.prototypeItem` and the `SliverPrototypeExtentList` branch of `buildChildLayout`. Trigger: `sliver_prototype_extent_list.dart`.
- scroll_view.rs: `ScrollView.debugFillProperties`, `BoxScrollView.debugFillProperties`, `ListView.debugFillProperties`. Trigger: diagnostics.
- single_child_scroll_view.rs: `_RenderSingleChildViewport.describeSemanticsClip`, the `markNeedsSemanticsUpdate` in its `clipBehavior` setter and in `_hasScrolled`, and `describeApproximatePaintClip` (deferred crate-wide in rendering's `PORTING.md`); its `debugFillProperties`. Trigger: accessibility (do not stub); the inspector; diagnostics.
- scrollbar.rs: `ScrollbarPainter.semanticsBuilder` / `shouldRebuildSemantics` and `toString`; `RawScrollbarState`'s `FlutterError.fromParts` diagnostics, which are a plain assert message here. Trigger: accessibility (do not stub); diagnostics.
- restoration_properties.rs: `RestorableTextEditingController` (and with it the only implementor of `RestorableChangeNotifier`). Trigger: `TextEditingController` (`editable_text.dart`).
- radio_group.rs: the `Semantics(container: true, role: SemanticsRole.radioGroup)` wrapper `_RadioGroupState.build` returns. Trigger: accessibility (do not stub).
- radio_group.rs: `_debugScheduleSingleSelectionCheck` / `_debugCheckOnlySingleSelection`, which post-frame-check that at most one radio carries the group value. Trigger: diagnostics; it needs `WidgetsBinding.instance.addPostFrameCallback`, which would create the binding from a state that may run without one.
- shortcuts.rs: `ShortcutMapProperty`, `MenuSerializableShortcut` / `serializeForMenu` on `SingleActivator` and `CharacterActivator`, the deprecated `ShortcutActivator.isActivatedBy`, `ShortcutRegistry._debugCheckForDuplicates`, and `Shortcuts.includeSemantics` (carried, but the `Focus` it builds has no `Semantics` wrapper). Trigger: diagnostics; `platform_menu_bar.dart`; content equality on `ShortcutActivator`; accessibility.
- actions.rs, shortcuts.rs, focus_traversal.rs: `debugFillProperties` / `toStringShort` on the actions, intents, activators, managers, policies and orders. Trigger: diagnostics.
- navigator.rs: the members Dart deprecates — `Route.onPopInvoked` (superseded by `onPopInvokedWithResult`) and `RouteTransitionRecord.markForRemove` (superseded by `markForComplete`); routes.rs: `PopEntry.onPopInvoked`; pop_scope.rs: `PopInvokedCallback` / `PopScope.onPopInvoked`. Trigger: never — port them only if Flutter un-deprecates them.
- navigator.rs: `_RouteEntry.lastFocusNode`, `NavigatorState._recordLastFocus` and the `ServicesBinding.accessibilityFocus` listener that restores the focused semantic node after a pop. Trigger: accessibility (do not stub).
- navigator.rs: `Navigator.debugRouteNames`, `NavigatorState.debugFillProperties` / `routeJsonable` / `settingsJsonable` / `description`, `_HistoryProperty._debugMapsEqual`, and the `FlutterError` reports around `onGenerateRoute` / `onUnknownRoute`. Trigger: diagnostics.
- routes.rs: `TransitionRoute._performanceModeRequestHandle`; a transition does not ask the engine for `DartPerformanceMode.latency`. Trigger: scheduler's `requestPerformanceMode` / `PerformanceModeRequestHandle`.
- routes.rs: `ModalRoute._willPopCallbacks` and with it the deprecated `addScopedWillPopCallback` / `removeScopedWillPopCallback` / `hasScopedWillPopCallback` and the `await callback()` loop in `ModalRoute.willPop`; `popGestureEnabled` and `_maybeDispatchNavigationNotification` read as if the list were always empty. Trigger: an `async` `WillPopCallback`, which needs an event loop — Flutter's own replacement is `PopEntry` / `PopScope`, which is ported.
- routes.rs: the `Semantics(sortKey: OrdinalSortKey(..))` wrappers `_buildModalBarrier` and `_buildModalScope` add, and `ModalRoute.semanticsDismissible`'s only effect (it is carried and read by the barrier). Trigger: accessibility (do not stub).
- routes.rs: `RawDialogRoute.anchorPoint` and the `DisplayFeatureSubScreen` / `Semantics(scopesRoute:)` wrappers in `RawDialogRoute.buildPage`; `showGeneralDialog` therefore has no `anchorPoint` argument. Trigger: `display_feature_sub_screen.dart`, which needs `MediaQuery.displayFeatures`; accessibility.
- modal_barrier.rs: `_SemanticsClipper` / `_RenderSemanticsClipper`, the `Semantics` / `BlockSemantics` / `ExcludeSemantics` wrappers `ModalBarrier.build` returns, and with them `defaultTargetPlatform`'s `platformSupportsDismissingBarrier` gate; `barrierSemanticsDismissible`, `semanticsLabel`, `clipDetailsNotifier` and `semanticsOnTapHint` are carried but nothing reads them. Trigger: accessibility (do not stub).
- modal_barrier.rs: `SystemSound.play(SystemSoundType.alert)` when a non-dismissible barrier is tapped. Trigger: services' `SystemSound` / `SystemChannels.platform`.
- overlay.rs: `_RenderTheater`'s intrinsics, `computeDryLayout` and `computeDryBaseline`, `_RenderTheaterMixin.computeDistanceToActualBaseline` / `baselineForChild`, `_RenderDeferredLayoutBox.computeDryBaseline`, and `_RenderLayoutBuilder`'s `debugCannotComputeDryLayout` asserts. Trigger: intrinsics and baselines through a theater child.
- overlay.rs: `Overlay.of`'s `debugRequiredFor`, `_TheaterElement.debugVisitOnstageChildren`, `_RenderTheater.debugDescribeChildren` / `debugFillProperties`, `_Theater` / `OverlayState.debugFillProperties`, `_OverlayPortalElement.debugFillProperties`, `_RenderDeferredLayoutBox.debugLayoutParent`, and the `StackTrace` `_OverlayEntryLocation._debugMarkLocationInvalid` records (a flag stands in). Trigger: diagnostics.
- overlay.rs: the `Semantics` wrapper around `OverlayPortal.child`, `_DeferredLayout.childIdentifier` with `_RenderDeferredLayoutBox.describeSemanticsConfiguration`, `_RenderTheater.visitChildrenForSemantics`, and every `markNeedsSemanticsUpdate`. Trigger: accessibility (do not stub).
- overlay.rs: `_RenderTheater`'s `LayerHandle<ClipRectLayer>` (and its `dispose`) and `describeApproximatePaintClip`, and `_OverlayEntryLocation`'s `markNeedsCompositingBitsUpdate`. Trigger: layer handles and compositing bits in `reveal-rendering`.
- overlay.rs: `_RenderTheater.markNeedsLayout`'s `_outstandingDeferredChildUpdateCalls` gate and `_RenderDeferredLayoutBox._needsLayout`. Trigger: a public `RenderBox` super-hook for `markNeedsLayout` and a readable dirty flag in `reveal-rendering`.
- heroes.rs: `Hero.debugFillProperties` (the `tag` property), `HeroMode.debugFillProperties` (the `mode` flag), `_HeroFlightManifest.toString`, `_HeroFlight.toString`, and the `FlutterError.fromParts` report behind `Hero._allHeroesFor`'s duplicate-tag check (a `debug_assert!` carrying Dart's message meanwhile). Trigger: diagnostics.
- implicit_animations.rs: `AnimatedPhysicalModel`. Trigger: `PhysicalModel` (basic.rs); its `BorderRadiusTween` and `ColorTween` slots are ready.

## widgets/drag_target.rs → drag_target.dart

- Change: drag payloads are `Option<Rc<T>>`, and target compatibility uses Rust's concrete payload type.
  Reason: language — Rust has no Dart runtime subtype relation or implicit object-reference sharing.
  Affect: targets with a shared payload model use the same Rust type, such as a shared enum; callbacks receive shared payload references.

- Change: active drags retain their source state, entered targets and feedback avatar through arena lifetime guards.
  Reason: language — Rust arena objects need explicit retention when pointer callbacks outlive widget disposal.
  Affect: store a `RetainedHandle` of those objects; drags continue after their source is removed, and completion or cancellation releases them.
