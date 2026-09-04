# reveal-widgets/src
Syntax (constructors, setters, `Option`, `into_widget`) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/widgets
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## framework/ → framework.dart

- Change: Dart's `class Foo extends StatelessWidget` is `impl StatelessWidget for Foo` (likewise `StatefulWidget`, `InheritedWidget`, `ParentDataWidget`, `LeafRenderObjectWidget`, `SingleChildRenderObjectWidget`), and a widget value becomes a tree node with `.into_widget()`, a `WidgetRef` (`Rc<dyn Widget>`). `Widget` itself is the erased trait the kind adapters implement; user code does not implement it.
  Reason: language — a blanket `impl Widget` per kind would overlap (E0119); the kind-tagged `IntoWidget<Kind>` picks the adapter without per-type boilerplate.
  Affect: write `Padding { child: Text { .. }.into_widget() }.into_widget()`; child fields are `WidgetRef`. `widget == other` is identity (`same_widget`); `Widget.canUpdate` is `can_update`.

- Change: a `StatefulWidget` names its state class as the associated type `State`, and `State` names its widget as `Widget`; `create_state` returns the value. The state is one struct in the arena with a `StateData` bag under the field `state` (`state_accessors!()`), and its methods take `self: Handle<Self>`; `set_state(app, |state| ..)` mutates it and marks the element.
  Reason: language — Dart's `State<T>` already fixes the widget type; an associated type gives `widget()` typed access with no cast, and the arena receiver rule applies to every object with identity.
  Affect: `impl StatefulWidget for Counter { type State = CounterState; fn create_state(&self) -> CounterState }`, `impl State for CounterState { type Widget = Counter; state_accessors!(); fn build(self: Handle<Self>, app, context) -> WidgetRef }`.

- Change: an element is one struct in the arena with an `ElementData` bag under the field `element` (`element_accessors!()`); the erased edge `AnyElement` is also the `BuildContext`. Dart's `Element` bodies are on `ElementBase`, a trait every element implements; an override calls `ElementBase::mount(self, app, ..)` where Dart calls `super.mount(..)`. A base class between `Element` and the leaf (`ComponentElement`, `ProxyElement`, `RenderObjectElement`, `RenderTreeRootElement`) is a trait whose overrides keep Dart's names, so the leaf's `impl Element` forwards `fn update_slot(..) { RenderObjectElement::update_slot(self, app, new_slot) }` and a super call is always `Trait::method(self, ..)`. Concrete elements are generic over their widget type (`StatelessElement<W>`, `SingleChildRenderObjectElement<W>`, …) so the widget is read without a cast. Dart's `is RenderObjectElement` / `is InheritedElement` / `is ParentDataElement` / `state is T` checks are vtable slots.
  Reason: language — same shape as `reveal-rendering`'s objects; no inheritance, no runtime interface queries on an erased edge.
  Affect: only framework code writes elements. `BuildContext` methods are on `AnyElement`: `context.depend_on_inherited_widget_of_exact_type::<Theme>(app)` returns `Option<&Theme>`.

- Change: `RenderObjectWidget` names its render object as the associated type `RenderObject`; `create_render_object` returns the erased `AnyRenderObject` (`handle.as_object()`), and `update_render_object` / `did_unmount_render_object` receive the typed `RenderHandle`. `SingleChildRenderObjectWidget` requires a `RenderObjectWithChildMixin` render object.
  Reason: language — Dart's `covariant RenderObject renderObject` is the typed parameter; the element tree holds the erased edge.
  Affect: `type RenderObject = RenderPadding; fn create_render_object(..) -> AnyRenderObject { RenderPadding::new(app, ..).as_object() }`.

- Change: `GlobalKey` is not generic; `current_state::<S>(app)` names the state. `GlobalObjectKey` and `ObjectKey` wait.
  Reason: language — an erased key cannot be recognised as a generic type; the state type only mattered for `currentState`.
  Affect: `let key = GlobalKey::new(); key.current_state::<CounterState>(app)`.

- Change: a widget's `slot` is the `Slot` enum: `Indexed(IndexedSlot)` or `Custom(Rc<dyn Any>)` compared by identity; `None` is Dart's null slot.
  Reason: language — Dart's slot is `Object?`.
  Affect: none for widget authors.

- Change: `BuildOwner::build_scope(app, context, callback)` takes the callback as `Option<Box<dyn FnOnce(&mut App)>>`; `lock_state` takes a closure. `BuildOwner::new(app, on_build_scheduled)` has no focus manager.
  Reason: language — callbacks re-enter `App`; platform — the focus system is deferred.
  Affect: `owner.build_scope(app, root, Some(Box::new(|app| ..)))`.

- Change: a `State` is destroyed with its element at unmount; a retained `Handle<S>` is stale afterwards, and `mounted` (which checks the slot first) is the only safe call on it.
  Reason: language — no garbage collector keeps Dart's disposed `State` object alive for late callers; the arena frees the slot.
  Affect: the Dart guard `if (mounted) setState(..)` from a timer or listener works; reading `widget` or `context` on an unmounted state panics with a stale-handle message instead of returning the last widget.

- Change: a panic in `build` unwinds; there is no `ErrorWidget` and no `FlutterError.reportError`.
  Reason: language — no catchable exception for ordinary control flow; diagnostics are deferred.
  Affect: a failing build takes the frame down instead of painting the red error box.

## binding.rs → binding.dart

- Change: `WidgetsBinding::instance(app)` creates the binding (Flutter's `WidgetsFlutterBinding.ensureInitialized`) and hooks the renderer's frame through `RendererBindingOverridesObject`: `will_draw_frame` builds the root's scope, `did_draw_frame` finalizes the tree. `drawFrame` itself stays in `reveal-rendering`.
  Reason: language — Dart's `WidgetsBinding.drawFrame` overrides `RendererBinding.drawFrame` and calls `super`; a hook pair is the override without inheritance.
  Affect: none for app code; `run_app(app, widget)` is the entry point.

- Change: `run_app` schedules the root attach with a zero-duration `Timer` (Dart's `Timer.run`) and then schedules a normal frame; there is no warm-up frame. `run_widget` skips the default `View`.
  Reason: platform — `scheduleWarmUpFrame` waits in `reveal-scheduler`.
  Affect: a host must `app.elapse(..)` (or run its event loop) before the first frame builds anything; the first frame is a regular frame.

- Change: `RootWidget::attach` consumes the widget and returns `Handle<RootElement>`; `RootElement::new_widget` is the re-attach path.
  Reason: language — the widget is erased into the element's `WidgetRef` at attach.
  Affect: none for app code.

## view.rs → view.dart

- Change: `View` wraps its child in `MediaQuery::from_view` and `RawView` only, without the focus traversal group and focus scope; `RawView` has no `_deprecatedPipelineOwner` / `_deprecatedRenderView` path. `_RawViewInternal` keys its element by the view's id (`ValueKey<u64>`) instead of `_DeprecatedRawViewKey`.
  Reason: platform — focus waits; the deprecated path served the pre-multi-view binding.
  Affect: `FocusScope.of` is unavailable under a `View` until the focus system lands.

- Change: `_RawViewElement` disposes its `PipelineOwner` by destroying the arena slot at unmount; `PipelineOwner.dispose` waits with semantics.
  Reason: platform — the owner holds nothing but its tree links here.
  Affect: none.

## widgets/text.rs → text.dart (+ `RichText` from basic.dart)

- Change: `DefaultTextStyle::of(app, context)` returns an owned `DefaultTextStyle` (the widget is `Clone`), not a reference to the tree's instance.
  Reason: language — the fallback has no tree slot to borrow from, and `Rc<dyn Widget>` is `!Sync`, so no static fallback can be handed out.
  Affect: `let default = DefaultTextStyle::of(app, context); default.style` is a value with no lifetime tied to `app`.

## widgets/basic.rs → basic.dart (+ `DecoratedBox` from container.dart, `WidgetBuilder` from framework.dart)

- Change: `Directionality::of(app, context)` / `maybe_of(app, context)` take `App`; `Builder.builder` is `WidgetBuilder = Rc<dyn Fn(&mut App, BuildContext) -> WidgetRef>`, defined here.
  Reason: language — a Rust callback cannot capture what it mutates; every framework call takes `&mut App`.
  Affect: write `Directionality::of(app, context)` and `Builder { builder: Rc::new(|app, context| ..) }`.

- Change: constructor asserts (`Opacity` 0..=1, `Align` factors >= 0, `ConstrainedBox.constraints` valid) run in the render object's constructor and setters, not at widget construction.
  Reason: language — a struct literal has no constructor body.
  Affect: an invalid value panics when the element creates or updates its render object (during build), not where the widget is written.

- Change: `Opacity` has no `alwaysIncludeSemantics`.
  Reason: platform — accessibility is deferred (`RenderOpacity` carries no flag).
  Affect: pass no semantics flag.

- Identical: `Directionality` is a plain `InheritedWidget`; Flutter's `_UbiquitousInheritedWidget` element skips per-dependent bookkeeping with the same observable notifications. `Padding`, `Align`, `Center`, `ConstrainedBox`, `DecoratedBox`, `Listener`, `MouseRegion`, `RepaintBoundary`, `Builder` are as in Dart.

## widgets/ticker_provider.rs → ticker_provider.dart

- Change: `SingleTickerProviderStateMixin` / `TickerProviderStateMixin` are traits over `State` with a data bag (`SingleTickerProviderStateMixinData` under `single_ticker_provider`, `TickerProviderStateMixinData` under `ticker_provider`) and an accessor pair; the state's own `State::activate` / `State::dispose` call `SingleTickerProviderStateMixin::activate(self, app)` / `::dispose(self, app)` (after disposing its controllers) where Dart's mixin body would run.
  Reason: language — a Rust trait holds no state and cannot interpose on another trait's hooks (no mixin linearization).
  Affect: a state that omits the two calls silently keeps a stale ticker-mode subscription after a `GlobalKey` move and leaks its listener on dispose.

- Change: Dart's `vsync: this` is `self`: the state implements scheduler's `TickerProviderObject` by forwarding to the mixin's `create_ticker`, and the blanket `impl<T: TickerProviderObject> TickerProvider for Handle<T>` makes the handle the provider.
  Reason: language — orphan rule; a blanket over the mixin trait cannot implement the foreign twin, so each state writes the three-line forwarding impl.
  Affect: `impl TickerProviderObject for MyState { fn create_ticker(self: Handle<Self>, app, on_tick) -> Handle<Ticker> { SingleTickerProviderStateMixin::create_ticker(self, app, on_tick) } }`, then `AnimationController::create(app, .., self)`.

- Change: `TickerMode::get_notifier` / `get_values_notifier` return `Rc<dyn ValueListenable<_>>`, wrapping the ancestor's `Handle<ValueNotifier<_>>` anew on every call; the fallback is a fresh constant listenable.
  Reason: language — a `dyn` value has no identity of its own, and foundation's erased value form of a `Listenable` is deferred (this is its trigger).
  Affect: two results for one `TickerMode` are not `Rc::ptr_eq`; the mixins remove and re-add their listener on `activate` instead of keeping it (same listener set).

- Change: a ticker vended by `TickerProviderStateMixin` is a plain `Ticker` and stays in the state's set after it is disposed.
  Reason: language — `Ticker::dispose` is an inherent method on a concrete arena struct; Dart's `_WidgetTicker` subclasses `Ticker` to override `dispose` and unregister.
  Affect: a long-lived state that creates and disposes many controllers keeps one handle per dead ticker until it disposes; muting a disposed ticker is a no-op.

## widgets/transitions.rs → transitions.dart

- Change: Dart's `abstract class AnimatedWidget extends StatefulWidget` is the trait `AnimatedWidget` (`key`, `listenable() -> Rc<dyn Listenable>`, `build`); `.into_widget()` (tag `AnimatedKind`) wraps the value in `Animated<W>`, the `StatefulWidget` whose state `AnimatedState<W>` is Dart's `_AnimatedState`.
  Reason: language — no inheritance; the kind-tagged `IntoWidget` scheme is the framework's shape for every widget kind.
  Affect: `impl AnimatedWidget for Spinner { fn listenable(&self) -> Rc<dyn Listenable> { Rc::new(self.animation) } .. }` and `Spinner { .. }.into_widget()`; the state is `AnimatedState<Spinner>`. Dart's `widget.listenable != oldWidget.listenable` is `Rc::ptr_eq`: keep one `Rc` and clone it into each rebuilt widget to keep the subscription; wrapping a handle anew re-subscribes on every update (harmless).

## widgets/icon_theme_data.rs → icon_theme_data.dart, widgets/icon_theme.rs → icon_theme.dart

- Identical: `IconThemeData` (fields, `fallback`, `copy_with`, `merge`, `resolve`, `is_concrete`, `lerp`) and `IconTheme` (`of`, `merge` over `Builder`, `update_should_notify`). Dart's clamped private `_opacity` is the public `opacity` field, clamped by the setter and `lerp`; `IconTheme::merge(key, data, child)` takes the key positionally because the result is an erased `Builder`.

## widgets/gesture_detector.rs → gesture_detector.dart

- Change: `Map<Type, GestureRecognizerFactory>` is `GestureRecognizerFactories`, a `Vec<(TypeId, GestureRecognizerFactoryRef)>` in insertion order; the value is a factory erased with `factory.into_factory()`. `RawGestureDetectorState._recognizers` is the same shape over `AnyGestureRecognizer`.
  Reason: language — Dart's `Type` is `TypeId`, and a heterogeneous `GestureRecognizerFactory<T>` map needs an erased value; an association list keeps Dart's `Map` iteration order without a new dependency.
  Affect: write `gestures: vec![(TypeId::of::<TapGestureRecognizer>(), GestureRecognizerFactoryWithHandlers::<TapGestureRecognizer>::new(ctor, init).into_factory())]`; `replace_gesture_recognizers(app, gestures)` takes the same.

- Change: the factory closures take `&mut App` and typed handles: `constructor: Fn(&mut App) -> Handle<T>`, `initializer: Fn(&mut App, Handle<T>)`.
  Reason: language — recognizers live in the arena; a callback cannot capture what it mutates.
  Affect: `move |app, instance| instance.set_on_tap(app, Some(Listener::new(..)))`.

- Change: `GestureDetector` is `Clone` (its build captures a clone of itself where Dart's closures capture `this`); a callback field is `Option<Rc<dyn Fn(&mut App, Details)>>` / `Option<Listener>`.
  Reason: language — closure capture of `&self` cannot be `'static`; callbacks receive `App`.
  Affect: construct with `GestureDetector { on_tap: Some(Listener::new(|app| ..)), ..GestureDetector::default() }`.

- Change: `replaceGestureRecognizers`'s FlutterError is a `debug_assert!`.
  Reason: language — no catchable exception; diagnostics deferred.
  Affect: calling it outside layout panics in debug and proceeds in release.

## widgets/icon_theme.rs, icon_theme_data.rs → icon_theme.dart, icon_theme_data.dart

- Change: `IconThemeData::resolve(&self, app, context)` returns a clone and `IconTheme::of(app, context)` an owned `IconThemeData`.
  Reason: language — the tree hands out no references that outlive an `App` call (as `DefaultTextStyle::of`).
  Affect: value semantics; `IconTheme::of(app, context).size`.

- Change: `IconThemeData.color` is painting's `AnyColor` (a `CupertinoDynamicColor` may live there; `CupertinoIconThemeData.resolve` resolves it).
  Reason: language — see painting's `colors.rs` entry.
  Affect: `.color(Color::RED)` unchanged; compare with `.map(AnyColor::color)`.

- Change: `IconThemeData.resolve` is overridable through a `resolver` closure set with `IconThemeData::resolver(..)`; Dart's `class CupertinoIconThemeData extends IconThemeData` is a constructor returning an `IconThemeData` with that override, `copy_with` / `merge` keep it, `lerp` drops it, and `==` treats the override's presence as Dart's `runtimeType` check.
  Reason: language — `IconTheme.data` is a value, not a subclassable object.
  Affect: `CupertinoIconThemeData::new().color(..)` where Dart writes `CupertinoIconThemeData(color: ..)`.

- Identical: `IconThemeData` (`fallback`, `copy_with`, `merge`, `is_concrete`, `lerp`, the range asserts), `IconTheme` (`merge` over `Builder`, `of` with the concrete fallback fill-in, `update_should_notify`).

## widgets/media_query.rs → media_query.dart

- Change: `MediaQueryData::from_view(app, view, platform_data)` reads the platform's brightness from `app.platform()` and leaves the accessibility flags, 24-hour format, text scaler (`NO_SCALING`), system context menu flag, text style overrides, system gesture insets, and display corner radii at their defaults.
  Reason: platform — `Platform` exposes `platform_brightness` only; `ViewMetrics` has no system gesture insets or corner radii.
  Affect: those fields are only non-default when an ambient `MediaQuery` supplies `platform_data`.

- Change: `MediaQuery::from_view` re-derives its data on dependency and widget changes only.
  Reason: platform — no `WidgetsBindingObserver` yet.
  Affect: a view metrics change after mount is not reflected until something above rebuilds.

- Change: `MediaQuery::of` and the non-maybe accessors panic with `debugCheckHasMediaQuery`'s summary in all builds.
  Reason: language — no catchable error; diagnostics deferred.
  Affect: same message in release as in debug.

- Identical: `MediaQueryData` (fields, `copy_with`, `remove_*`, `apply_*`, `orientation`, `==`), `MediaQuery` (`InheritedModel<Aspect = MediaQueryAspect>`, every `*_of` / `maybe_*_of`, `update_should_notify_dependent`), `NavigationMode`, `_MediaQueryFromView`. `with_clamped_text_scaling(key, ..)` takes a key it does not forward, as Dart does.

## widgets/image.rs → image.dart (`createLocalImageConfiguration` only)

- Change: `create_local_image_configuration(app, context, size)` takes `App` and leaves `bundle` and `locale` unset.
  Reason: platform — `DefaultAssetBundle` and `Localizations` wait.
  Affect: an `ImageProvider` keyed by bundle or locale resolves against `None`.

## widgets/widget_state.rs → widget_state.dart

- Change: a constraint is erased into `WidgetStatesConstraintRef` (a newtype over `Rc<dyn WidgetStatesConstraint>`), which the mixin's operators produce and carry: `WidgetState::Hovered & WidgetState::Focused`, `~WidgetState::Disabled`, `WidgetState::any()` (a fn). `WidgetStateMap<T>` is `Vec<(WidgetStatesConstraintRef, T)>` in insertion order; a bare enum key is written `WidgetState::Error.into()`.
  Reason: language — the orphan rule forbids `Not` / `BitAnd` impls on a plain `Rc<dyn Trait>` alias; constraints are not hashable (`hashCode` waits), so Dart's `Map` is an ordered entry list.
  Affect: reuse a stored combination with `.clone()` (`active.clone() & WidgetState::Error`); `==` follows Dart's overrides on the combinators, `any` by class.

- Change: `WidgetStateMapper<T>::resolve` panics with Dart's `ArgumentError` message when no key is satisfied, for every `T`; a map that should resolve to `None` ends with `(WidgetState::any(), None)`.
  Reason: language — Dart's `null as T` is a runtime test of `T`'s nullability; a Rust generic cannot tell `Option<X>` from `X` without specialization.
  Affect: `WidgetStateMapper<Option<X>>` (Dart's `<X?>`) panics on an unmatched set instead of resolving `null` unless it carries the `any` entry.

- Change: Dart's static members of `WidgetStateProperty` are on `<dyn WidgetStateProperty<T>>`; `from_map`, `resolve_with`, `all`, `lerp` return `WidgetStatePropertyRef<T>` (`Rc<dyn WidgetStateProperty<T>>`); `lerp(a, b, t, f)` takes `Option<WidgetStatePropertyRef<T>>` sides and an `Fn(Option<T>, Option<T>, f64) -> Option<T>`, returning `Option<WidgetStatePropertyRef<Option<T>>>`.
  Reason: language — a Rust trait has no static members; `T?` is `Option<T>`.
  Affect: `<dyn WidgetStateProperty<MouseCursorRef>>::resolve_with(|states| ..)`; a `WidgetStateProperty<T>?` field is `Option<WidgetStatePropertyRef<T>>`.

- Change: `WidgetStateProperty.resolveAs<T>(value, states)` is `<dyn WidgetStateProperty<T>>::resolve_as(&value, &states)` over `MaybeWidgetStateProperty<T>` (`as_widget_state_property` / `from_resolved`), implemented for `AnyColor` (its `WidgetStateColor` extension), `MouseCursorRef` (`as_any`), and `Option` of either; it returns the value's own type.
  Reason: language — Dart's `value is WidgetStateProperty<T>` is a runtime interface query; the value types answer it here.
  Affect: `<dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&widget.mouse_cursor, &states)` where Dart writes `WidgetStateProperty.resolveAs<MouseCursor?>(widget.mouseCursor, states)`; a plain `AnyColor` comes back unchanged, other extensions intact.

- Change: `WidgetStateColor` is one struct over Dart's three private subclasses (`resolve_with`, `transparent()`, `from_map`) and becomes the `Color` it is in Dart with `into_any()` / `.into()`: an `AnyColor` whose value is the empty-state resolution (`super(defaultValue)`), read back with `color.extension::<WidgetStateColor>()`. `WidgetStateColor.transparent` is a fn.
  Reason: language — painting's `Color` is a value (see its `colors.rs` entry); an `Rc` resolver cannot live in a `static`.
  Affect: `TextStyle::new().color(WidgetStateColor::resolve_with(..))`; a `from_map` color read as a plain `Color` is its empty-state resolution (Dart throws via `noSuchMethod`), panicking with the mapper's message only when no key matches the empty set. `==` is Dart's `Color.==` (same subclass and value), map equality for `from_map`.

- Change: `WidgetStateMouseCursor` is one struct over Dart's two private subclasses (`resolve_with(callback, debug_description: Option<&str>)`, `from_map`); `clickable()` / `adaptive_clickable()` / `textable()` are fns; `.into()` gives the `MouseCursorRef`, recovered with `as_any().downcast_ref::<WidgetStateMouseCursor>()`. A `from_map` cursor used as a plain cursor resolves the empty set like `resolve_with` does.
  Reason: language — no `static const` object holding an `Rc`; no `noSuchMethod`.
  Affect: two `clickable()` results are not `==` (identity; Dart's `const` is canonical), so a `MouseRegion` handed a fresh one each build re-annotates; map equality holds for `from_map`.

- Change: `WidgetStatesController` holds `ChangeNotifierData` and the set directly and re-states `ValueNotifier`'s `value` / `set_value` / `dispose`; `Handle<WidgetStatesController>` is `Listenable`, not `ValueListenable<WidgetStates>`; `new(app, Option<WidgetStates>)` returns the handle.
  Reason: language — foundation's `ValueNotifier` is a leaf arena object with no `&mut` access to its value and `set_value` on its own handle; there is no `ValueNotifierData` bag nor a `ValueListenableObject` twin (orphan rule).
  Affect: `controller.update(app, state, add)`, `app.get(controller).value()`; passing it where a `ValueListenable` is wanted waits.

## Deferred

- `MultiChildRenderObjectElement` / `MultiChildRenderObjectWidget`. Trigger: the first multi-child render object (`Row`, `Stack`); needs rendering's `ContainerRenderObjectMixin`.
- `ErrorWidget`, `_reportException`, `debugWidgetBuilderValue`, the build timeline. Trigger: diagnostics.
- `Notification` / `NotifiableElementMixin` / `dispatchNotification`. Trigger: `NotificationListener`, scrolling.
- `reassemble`. Trigger: hot reload.
- `ObjectKey`, `GlobalObjectKey`. Trigger: a widget keyed by object identity.
- `BuildOwner.focusManager`. Trigger: focus.
- `Element.debugFillProperties` and the other diagnostics, `DebugCreator`, `debugGetCreatorChain`. Trigger: diagnostics.
- `WidgetsBindingObserver` and its callbacks (`didChangeMetrics`, locale, lifecycle, memory pressure, back gestures, view focus), `handleAppLifecycleStateChanged`, `performReassemble`, the platform menu delegate and windowing owner, `scheduleWarmUpFrame`, `framesEnabled`. Trigger: the shell forwarding those platform events.
- `ViewCollection`, `ViewAnchor`, `View.forgetChild` diagnostics, `_ViewState`'s focus scope and `onViewFocusChange`. Trigger: multi-view and focus.
- Semantics owner callbacks on the view's pipeline owner. Trigger: accessibility.
- `debugCheckHasDirectionality` in `RichText`. Trigger: diagnostics.
- `DefaultTextStyle.merge` (needs `Builder`), `InheritedTheme` (`wrap`, `captureAll`) on `DefaultTextStyle` / `DefaultTextHeightBehavior`. Trigger: `Builder`, `InheritedTheme`.
- `Text`: selection (`selectionRegistrar`, `selectionColor`, `_SelectableTextContainer`), `locale`, `strutStyle`, `semanticsLabel` / `semanticsIdentifier`, `WidgetSpan` children (`RichText` is a leaf), the deprecated `textScaleFactor`. Trigger: `SelectionContainer`, `Locale` / `StrutStyle`, inline children in `RenderParagraph`.
- basic.rs: `LimitedBox`, `ColoredBox`, `IgnorePointer`, `AbsorbPointer` (their render objects are absent), `Positioned` (needs `Stack`), `Semantics`, `RepaintBoundary.wrap` / `wrapAll` (needs a key over Dart's `Object`), `debugCheckHasDirectionality`. Trigger: the render objects, multi-child widgets, accessibility, diagnostics.
- image.rs: `Image`, `precacheImage`, the image-loading state; `DefaultAssetBundle` / `Localizations` inputs of `create_local_image_configuration`. Trigger: the image cache; asset bundles; localizations.
- ticker_provider.rs: `_WidgetTicker` / `TickerProviderStateMixin._removeTicker` (needs a dispose hook on scheduler's `Ticker`), `TickerMode.merge` (needs `Builder`). Trigger: a state that churns controllers; `Builder`.
- transitions.rs: `SlideTransition`, `MatrixTransition`, `ScaleTransition`, `RotationTransition`, `SizeTransition`, `AlignTransition`, `DecoratedBoxTransition`, `DefaultTextStyleTransition`, `PositionedTransition`, `RelativePositionedTransition`, `SliverFadeTransition`, `ListenableBuilder`, `AnimatedBuilder`, `DelegatedTransitionBuilder`, `FadeTransition.alwaysIncludeSemantics`. Trigger: their render objects and basic widgets; accessibility.
- icon_theme: `InheritedTheme` (`wrap`, `captureAll`) on `IconTheme`; `IconThemeData.resolve` as an override point (inherent, called directly by `IconTheme::of`) — cupertino's `CupertinoIconThemeData` resolves a dynamic color there; `hashCode` (no `Hash` over `f64`). Trigger: `InheritedTheme`; the cupertino crate; a map keyed by `IconThemeData`.
- `WidgetsBinding`: `firstFrameRasterized` / `waitUntilFirstFrameRasterized` / `debugDidSendFirstFrameEvent` (need scheduler `addTimingsCallback`), `handlePushRoute` / `handlePopRoute` / `handlePushRouteInformation`, `renderViewElement`. Trigger: `SchedulerBinding.addTimingsCallback`; navigation.
- `LookupBoundary` (`View.of` / `maybe_of` look straight up the tree). Trigger: `LookupBoundary` in framework.
- gesture_detector.rs: `excludeFromSemantics`, `semantics`, `SemanticsGestureDelegate`, `_GestureSemantics`, `replaceSemanticsActions`, `RenderSemanticsGestureHandler` (a11y — do not stub); `onDoubleTap*`, `on*Drag*`, `onPan*`, `onScale*`, `onForcePress*`, `dragStartBehavior`, `trackpadScrollCausesScale`, `trackpadScrollToScaleFactor`, `ScrollConfiguration.of(context).getMultitouchDragStrategy` (their recognizers). Trigger: accessibility; the drag / scale / force-press recognizers. `onTapMove` is a field Flutter does not wire at this commit either.
- icon_theme.rs: `InheritedTheme` (`wrap`, `captureAll`) on `IconTheme`; `CupertinoIconThemeData`. Trigger: `InheritedTheme`; cupertino.
- media_query.rs: `WidgetsBindingObserver` registration and `didChangeMetrics` / `didChangeAccessibilityFeatures` / `didChangeTextScaleFactor` / `didChangePlatformBrightness` on the from-view state; `DisplayFeature` (`displayFeatures`, `removeDisplayFeatures`, `displayFeaturesOf`, its aspect); `SystemTextScaler`; platform sources for the accessibility flags, `alwaysUse24HourFormat`, `supportsShowingSystemContextMenu`, text style overrides, `displayCornerRadii`, `systemGestureInsets`; `debugCheckHasMediaQuery`, `debugBrightnessOverride`; `EdgeInsets.fromViewPadding` lives as a private fn here until painting takes it. Trigger: observers in binding.rs; `DisplayFeature` and those members on `Platform` / `ViewMetrics`; the debug files; painting.
- widget_state.rs: `WidgetStateBorderSide` / `_LerpSides` (needs an extension slot on painting's Copy `BorderSide`), `WidgetStateOutlinedBorder` (needs a default-state stand-in over `Box<dyn OutlinedBorder>`), `WidgetStateTextStyle` (needs an extension slot on the value `TextStyle`), `WidgetStatesController` as `ValueListenable<WidgetStates>` (foundation `ValueListenableObject` twin or `ValueNotifierData` bag), `hashCode` / `Hash` on constraints, mappers, `WidgetStatePropertyAll`, `WidgetStateMapper.noSuchMethod`, `Diagnosticable`. Trigger: `ChipThemeData.side` / `.shape`, `InputDecoration.labelStyle`, the first consumer holding the controller as a `ValueListenable`, theme-data hashing, diagnostics.
