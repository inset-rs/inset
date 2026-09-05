# reveal-rendering/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/rendering
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- box.rs → BoxConstraints
- proxy_box.rs → HitTestBehavior
- object.rs → Constraints (`isTight` / `isNormalized` / `debugAssertIsValid`)
- sliver.rs → GrowthDirection, applyGrowthDirectionToAxisDirection / ScrollDirection, SliverConstraints (`isTight` / `isNormalized` / `asBoxConstraints` / `==`)
- viewport_offset.rs → ScrollDirection, flipScrollDirection
- flex.rs → FlexFit, MainAxisSize, MainAxisAlignment
- stack.rs → StackFit
- box.rs → RenderBox intrinsics, dry layout and dry baselines with their caches
- layout_helper.rs → layout_helper.dart
- proxy_box.rs → RenderAspectRatio, RenderIntrinsicWidth, RenderIntrinsicHeight, RenderFittedBox
- shifted_box.rs → OverflowBoxFit, RenderConstrainedOverflowBox, RenderSizedOverflowBox, RenderFractionallySizedOverflowBox, RenderBaseline
- viewport.rs → RenderViewportBase.debugThrowIfNotCheckingIntrinsics and the four intrinsics, RenderViewport.computeDryLayout
- object.rs / box.rs → getTransformTo, globalToLocal, localToGlobal
- debug.rs → debug.dart
- pipeline_owner.rs → object.dart (PipelineOwner)
- sliver_fixed_extent_list.rs → sliver_fixed_extent_list.dart

## object.rs → object.dart (RenderObject)

- Change: a render object is a struct in the `App` arena reached through a typed `RenderHandle`. What Flutter inherits as state from a base class or mixin is a field on the struct with an accessor the trait asks for; what it inherits as a method body is a trait method the leaf calls by name where Dart would run the inherited one. Methods take the handle and `&mut App` rather than `&mut self`, and a reference to "some render object" is a type-erased handle: `AnyRenderObject`, or `AnyRenderBox` / `AnyRenderSliver` once the protocol is known.
  Reason: language — no inheritance and no GC identity; a `&mut self` receiver would hold the node borrowed while its child lays out, and the child must be able to reach back into it.
  Affect: a render object is a struct carrying the base fields, an `impl RenderObject` for layout, paint and the child walk, and an `impl RenderBox` (or `RenderSliver`) for the protocol's accessors and the overrides the protocol dispatches through, `apply_paint_transform` and `setup_parent_data` among them; `RenderHandle::new_box` creates it. Parents hold a child's type-erased handle, tree methods come from the protocol trait (import it to call them), and an erased handle downcasts with `as_box()` / `as_sliver()` where Dart writes `as RenderBox`. A crate that calls an inherent handle-receiver method needs `#![feature(arbitrary_self_types)]` itself; trait methods resolve without it.

- Change: Flutter's `attach` / `detach` overrides are the post-hooks `did_attach` / `did_detach`, run after the base body; their defaults, like `redepth_children`'s, walk `visit_children`.
  Reason: language — a trait default cannot call `super`; every Flutter override calls `super.attach` first and none reads its owner before `super.detach`, so a post-hook is equivalent.
  Affect: put what Dart writes after `super.attach(owner)` in `did_attach`; implement `visit_children` and the walks come for free.

- Change: a panic in `perform_layout`, `perform_resize` or `paint` unwinds.
  Reason: language — no `FlutterError.reportError` hook to catch and continue.
  Affect: layout and paint do not survive a failing render object.

- Change: `layout` lives on the protocol's type-erased handle and takes that protocol's constraints.
  Reason: language — there is no abstract `Constraints` an erased node can hold.
  Affect: `child.layout(app, box_constraints, parent_uses_size)` on an `AnyRenderBox`; there is no protocol-neutral `layout`.

## box.rs → box.dart (hit testing)

- Change: `BoxHitTestResult` only wraps: `BoxHitTestResult::wrap(&mut result)` is a view over a `HitTestResult` with no standalone constructor, and `BoxHitTestEntry` is the entry's target, carrying the box and its local position.
  Reason: language — no subclassing of the gesture crate's result and entry types.
  Affect: `hit_test` receives `&mut BoxHitTestResult<'_>` and adds with `result.add(BoxHitTestEntry::new(box, position).into())`; `handle_event` receives the `BoxHitTestEntry`.

## painting_context.rs → object.dart (PaintingContext), layer.rs → layer.dart

- Change: there is no `Layer` object tree. A repaint boundary keeps its recording as retained items (pictures, references to child boundaries, push/pop effects) plus one `CompositedLayer` value standing in for Flutter's offset, opacity and transform layers. The host recomposes the frame from those retained pieces every time, so a boundary that did not change contributes the same pictures.
  Reason: platform — valo composes a display list from retained pictures and has no engine layers to retain between frames.
  Affect: the `push_*` effects take no `needsCompositing` or `oldLayer` argument and return nothing; `update_composited_layer` returns a `CompositedLayer`, and `mark_needs_composited_layer_update` swaps it in without repainting the subtree; `schedule_initial_paint` takes a `CompositedLayer`.

- Change: no compositing bits: `needsCompositing`, `alwaysNeedsCompositing` and `flushCompositingBits` are gone, and every pushed effect spans child boundaries.
  Reason: platform — the layer-versus-canvas choice exists because a Skia clip cannot cross an engine layer, and retained items have no such split.
  Affect: `is_repaint_boundary` alone decides where recordings split; where Flutter calls `markNeedsCompositingBitsUpdate`, call `mark_needs_paint`. A frame is `flush_layout` then `flush_paint`.

- Change: `Layer.find` / `findAllAnnotations` are `BoundaryLayer::find` / `find_all_annotations`, which walk a boundary's recorded items in reverse. Each item kind carries its Dart layer's hit rule, so `findAnnotations` is not an override point, and `AnnotatedRegionLayer` is the one item that adds an annotation; its value is stored erased, since one recording carries annotations of every type, and comes back as an `Rc<T>` by exact downcast, Dart's `T == S`.
  Reason: platform — the recording has items, not retained layers (the entry above), so there is no `Layer` to subclass.
  Affect: `layer.find::<T>(app, position)` answers `Option<Rc<T>>`, and needs the `App` because a child boundary's recording lives in the arena; a render object annotates by pushing an `AnnotatedRegionLayer` through `PaintingContext::push_annotated_region`, never by overriding a method.

- Change: `RenderAnnotatedRegion<T>` needs `T: PartialEq`.
  Reason: language — Dart's `==` in the setter is a trait bound here.
  Affect: a value type that cannot be compared cannot be annotated.

## view.rs → view.dart, binding.rs → binding.dart

- Change: `RenderView` is its own kind of render object, neither box nor sliver: its `constraints` and `size` are inherent, its child slot is written out, and it has no type-erased protocol handle (neither `AnyRenderBox` nor `AnyRenderSliver`).
  Reason: language — Flutter overrides the `constraints` getter on `RenderObject`; here constraints belong to a protocol trait, and the view's come from its configuration.
  Affect: `RenderView::new(app, child, configuration, view)` returns a typed handle; use `as_object()` for the tree.

- Change: `RendererBinding` opens hooks (`will_draw_frame`, `did_draw_frame`, `did_handle_metrics_changed`) to a registered `RendererBindingOverrides`: before layout, after compositing, and after `handle_metrics_changed`. The widgets binding registers itself with `set_overrides` and implements the object-side `RendererBindingOverridesObject`.
  Reason: language — Flutter's `WidgetsBinding` overrides `drawFrame` and `handleMetricsChanged` through mixin order, and a crate above cannot override a method below it.
  Affect: a binding layered above this crate registers with `set_overrides` and fills the hooks instead of overriding `draw_frame`; a render-tree-only host sees nothing.

## mouse_tracker.rs → mouse_tracker.dart

- Change: Dart's `target is MouseTrackerAnnotation` is the virtual `RenderObject::mouse_tracker_annotation`, `None` by default, which `RenderMouseRegion` answers with its current callbacks and cursor. The tracker keys its per-device maps by the render object and reads the annotation from it again each time it dispatches.
  Reason: language — an erased render object cannot be asked whether it implements an interface; the object is the annotation's identity, as in Dart.
  Affect: a render object that should receive enter/exit events overrides `mouse_tracker_annotation`. One that has left the arena reads as no annotation, where Dart would still hold the object with `validForMouseTracker == false`; both skip it.

## proxy_box.rs → proxy_box.dart / shifted_box.rs → shifted_box.dart

- Change: `RenderBackdropFilter` paints through `PaintingContext::push_backdrop_filter`, a retained item that composites as a backdrop blur bounded to the render object's paint bounds (or the filter's own bounds), using the horizontal sigma alone, with the blend mode on the layer paint. Only the blur reaches the backdrop: a composed filter contributes the blur it composes, and a colour filter contributes nothing.
  Reason: platform — valo's backdrop is a blur with one sigma and a rect, and a colour filter on the layer paint would filter the children too.
  Affect: a backdrop filter blurs what lies under the widget's bounds (Flutter's unbounded filter blurs the enclosing clip's area, the same thing for a clipped dialog); an anisotropic blur uses its horizontal sigma; a colour filter composed into a backdrop filter is dropped, so a saturating frosted-glass surface is blurred but not saturated. `reveal-embedder-winit`'s `PORTING.md` records the same for the host.

- Change: Flutter's intermediate base classes and mixins here (`RenderProxyBox`, `RenderShiftedBox`, the aligning, custom-clip and animated-opacity mixins) take the trait-over-a-field shape from `object.rs`: the shared bodies live on the trait, mixin state is a data field with an accessor, and a render object is always a leaf struct that implements them.
  Reason: language — no inheritance.
  Affect: implement the trait and, where Dart would run the inherited method, call it by name, `RenderProxyBoxMixin::paint(self, app, context, offset)`; a clip leaf implements `default_clip` (Dart's `_defaultClip`) and reads the cached clip with `clip()` after `update_clip()`.

- Change: `RenderOpacity` and `RenderAnimatedOpacity` have no `alwaysIncludeSemantics`, and `RenderDecoratedBox` gives its painter no `onChanged`.
  Reason: platform — accessibility is deferred, and a `BoxPainter` callback cannot reach `App` yet.
  Affect: pass no semantics flag; a decoration that loads an image does not repaint on its own when the image arrives.

- Change: where a Flutter override calls `super` on a method whose Rust default the leaf's own impl shadows, the base body sits on a sibling `…Base` trait blanket-implemented for every implementor: `RenderBoxBase::hit_test` is `super.hitTest`.
  Reason: language — a trait default cannot call `super`, and a leaf's own `hit_test` shadows the default it would call.
  Affect: a box whose `hit_test` override needs the base behaviour, as the clips and the pointer-filtering boxes do, calls `RenderBoxBase::hit_test(self, app, result, position)` where Dart writes `super.hitTest`.

- Change: `CustomClipper<T>` is a trait: Dart's `reclip` constructor argument is a `reclip()` method the implementor answers, the trait object is itself the `Listenable` that forwards to it, `should_reclip` receives the old clipper as `&dyn CustomClipper<T>`, and `as_any` answers Dart's `runtimeType` comparison and `covariant` cast. A clipper is held as an `Rc`, and Dart's `==` between two of them is `Rc::ptr_eq`.
  Reason: language — a Rust trait has no fields, no constructor, no `runtimeType`, and no identity equality on a boxed value.
  Affect: implement `reclip` to get reclip-on-notify, downcast `old_clipper` with `as_any().downcast_ref()`, and pass the same `Rc` to keep a clipper's subscription.

- Change: `RenderTransform` has no `filterQuality`.
  Reason: platform — the filtered path is an `ImageFilterLayer` over `ImageFilter.matrix`, which neither the embedder's `Paint` nor the retained-item model has.
  Affect: a transform always paints through `push_transform`, or through the child's paint offset when the matrix is a translation.

- Change: `RenderColoredBox` (Flutter's private `_RenderColoredBox` from `widgets/basic.dart`) does not pass `is_anti_alias` to the canvas.
  Reason: platform — valo's `Paint` has no anti-alias flag; painting's `PORTING.md` records the same for `Clip`.
  Affect: the fill is drawn the way valo draws it, and setting the flag only marks a repaint.

## shifted_box.rs → shifted_box.dart (RenderConstraintsTransformBox)

- Change: `BoxConstraintsTransform` is a plain `fn(BoxConstraints) -> BoxConstraints`, and the setter compares the old and new transform by function address.
  Reason: language — a Rust closure has no identity, and Dart's setter compares two function values to decide whether anything changed.
  Affect: pass a named function or a non-capturing closure, not one that captures state; when the address comparison says "different" for the same function, the setter still only marks layout if the transform maps the current constraints to a different value.

## image_filter_config.rs → image_filter_config.dart

- Change: a blur has no `tileMode`.
  Reason: platform — valo's blur has no tile mode (`reveal-embedder`'s `PORTING.md` records that).
  Affect: `debug_short_description` on a blur reads `blur(5, 5, bounded)`, without Dart's tile-mode word.

## custom_paint.rs → custom_paint.dart

- Change: `CustomPainter` is the `CustomClipper` shape (a `repaint()` method, `as_any`, `Rc` identity).
  Reason: language — a trait has no fields, constructor or identity.
  Affect: implement `repaint` to get repaint-on-notify; `set_painter` / `set_foreground_painter` skip the same `Rc` and re-subscribe any other, so a painter wrapped in a new `Rc` each frame is re-subscribed each frame (the same listener set either way).

## paragraph.rs → paragraph.dart

- Change: `RenderParagraph::new` takes a `fonts` argument, an optional `FontCollection` handle that falls back to `PaintingBinding`'s app-wide one, and the paragraph is a leaf: no inline children, no selection registrar, no `selectionColor`.
  Reason: platform — the font collection is explicit (painting's `binding.rs`), `WidgetSpan` needs placeholders the host lacks, and selection waits on `selection.dart`.
  Affect: pass the fonts or `None`; a hit test stops at the paragraph, so `RichText` span recognizers cannot fire.

- Change: `TextOverflow::Fade` clips like `Clip`.
  Reason: platform — the fade is a gradient shader, and gradients are deferred.
  Affect: overflowing text is cut, not faded.

## object.rs / box.rs → object.dart / box.dart (container children)

- Change: the container mixins (`RenderObjectWithChildMixin`, `ContainerParentDataMixin`, `ContainerRenderObjectMixin`) take the trait-over-a-field shape on the object protocol, with Dart's type parameters as associated types: the child type is a `ErasedRenderObject` (`AnyRenderBox` or `AnyRenderSliver`), which converts to and from `AnyRenderObject`, and the container mixin also names its parent-data type. `RenderBoxContainerDefaultsMixin` is a stateless trait over a container of box children.
  Reason: language — no mixins, a trait cannot carry state, and a Dart type argument bounded by `RenderObject` has to name a concrete type-erased handle.
  Affect: a multi-child render object implements `ContainerRenderObjectMixin` naming the type-erased handle of its child and its parent-data type, its parent data implements `ContainerParentDataMixin` with the same child type, and a box container adds an empty `impl RenderBoxContainerDefaultsMixin`. Dart's `attach` / `detach` / `redepthChildren` / `visitChildren` overrides are the mixin's bodies, which the leaf's `impl RenderObject` calls by name.

- Change: `super.insert` / `move` / `remove` / `removeAll` from a container override, and `super.showOnScreen` / `super.markNeedsLayout`, take the sibling-base shape of `RenderBoxBase`: `ContainerRenderObjectBase` and `RenderObjectBase`. Because the base trait carries the virtual's name, it is path-called and never imported, or a plain `self.mark_needs_layout(app)` becomes ambiguous.
  Reason: language — a trait default cannot call `super`.
  Affect: a container that overrides one of them (`RenderSliverMultiBoxAdaptor`, the persistent headers) and the box protocol's `mark_needs_layout` call the base by name where Dart writes `super`.

- Change: `ContainerBoxParentData` is a trait joining the container half with `BoxParentData`, and `ParentData` gained the type-keyed query `provide(TypeId)` / `provide_mut`, read as `part::<P>()` on the erased value, the `Error::provide` shape: a half answers for its own type, and a type that embeds one answers for itself and then forwards to the half, so a chain composes. `AnyRenderBox` and `apply_paint_transform` read the offset through it.
  Reason: language — `parent_data_of::<BoxParentData>` is an exact downcast, so Dart's `parentData! as BoxParentData` cannot reach a subclass; the value has to answer the cast, and one cast method per half would close the set of halves to this crate.
  Affect: parent data a box parent positions must answer `BoxParentData` from `provide` and `provide_mut`; read a container child's offset with `child.box_parent_data(app).offset` or `parent_data.offset()`, and any other half with `parent_data.part::<P>()`.

## flex.rs → flex.dart

- Change: `RenderFlex`'s bodies live on `RenderFlexMixin` over a `RenderFlexData` field, and `RenderFlex` is a leaf that implements it with a one-line inherent wrapper per public getter and setter, so callers name no trait. This is the shape for every Flutter base class that is itself instantiable, where the trait cannot take the class's name.
  Reason: language — no inheritance.
  Affect: a flex subclass is a struct with a `RenderFlexData` field, an `impl RenderFlexMixin`, and render impls that call the base bodies by name, `RenderFlexMixin::perform_layout(self, app)`; `RenderFlexData::new()` with its setters is Dart's `super(direction:, mainAxisSize:)`.

## stack.rs → stack.dart

- Change: `RenderStack` and `RenderIndexedStack` are both leaves over `RenderStackBase` and a `RenderStackData` field, the `RenderFlex` shape; `RenderIndexedStack` overrides `paint_stack`.
  Reason: language — no inheritance, and Flutter's base class is itself instantiable.
  Affect: a stack subclass carries the data field, implements `RenderStackBase`, and calls the base bodies by name from its render impls.

- Change: `RelativeRect::lerp(Some(a), None, t)` interpolates `a` towards `RelativeRect::FILL`.
  Reason: language — Dart's branch reads `b!` on the null `b`, which Rust cannot express.
  Affect: the call returns a value where Dart throws.

## custom_layout.rs → custom_layout.dart

- Change: `MultiChildLayoutDelegate` is the `CustomClipper` shape: `relayout()` is a method the implementor answers, the delegate is held as an `Rc` compared by pointer, and `as_any` answers `runtimeType` and the `covariant oldDelegate` cast.
  Reason: language — a Rust trait has no constructor, no fields, no `runtimeType`, and no identity equality on a boxed value.
  Affect: implement `relayout` to get relayout-on-notify, downcast `old_delegate` with `as_any().downcast_ref()`, and pass the same `Rc` to keep the delegate's subscription.

- Change: Dart's four `FlutterError`s (no such child, a child laid out twice, invalid constraints, a child left unlaid-out) are panics, and the ones Dart raises from an `assert` are debug-only.
  Reason: language — no catchable exception, and the set of children still needing layout is debug state.
  Affect: a delegate that misuses its children panics instead of reporting a `FlutterError`.

## object.rs → object.dart (layout callbacks)

- Change: `RenderObjectWithLayoutCallbackMixin` is a trait on the box protocol over a `RenderObjectWithLayoutCallbackData` field; `PipelineOwner::enable_mutations_to_dirty_subtrees` is crate-private.
  Reason: language — the mixin needs the tree methods, which live on the protocol traits.
  Affect: a layout-building render object implements the mixin's accessors and `layout_callback`, and calls `run_layout_callback` from `perform_layout`.

## viewport_offset.rs → viewport_offset.dart

- Change: `super.moveTo` is `ViewportOffsetBase::move_to`, the sibling-base shape, and `debug_fill_description` is the `@protected` hook Dart's `toString` fills; `toString` itself is not ported.
  Reason: language — a trait default cannot call `super`, and widgets' `ScrollPosition` overrides both and calls the `super` bodies.
  Affect: an implementor that overrides either calls `ViewportOffsetBase::move_to(self, ..)` or `ViewportOffset::debug_fill_description(self, ..)` where Dart writes `super.…`.

## viewport.rs → viewport.dart

- Change: `RenderViewportBase` is a trait over a `RenderViewportBaseData` field, the `RenderFlex` shape; `RenderViewport` and `RenderShrinkWrappingViewport` are leaves that implement it alongside the box protocol, `RenderAbstractViewport` and the sliver-child container mixin, whose parent-data associated type is Dart's `ParentDataClass` parameter.
  Reason: language — no inheritance and no generic superclass.
  Affect: a viewport subclass carries the data field and calls the base bodies by name from its render impls, `RenderViewportBase::paint(self, app, context, offset)`.

- Change: Dart's `object is RenderAbstractViewport` is the type-keyed query `RenderObject::interface(TypeId)`, read as `object.interface::<AnyRenderAbstractViewport>()` on the type-erased handle, the `Error::provide` shape: a viewport answers that id with its own erased handle, boxed, and a type that implements several interfaces answers each.
  Reason: language — an erased render object cannot be asked whether it implements an interface, and one virtual per interface would close the set of interfaces to this crate.
  Affect: a render object that is a viewport overrides `interface` and answers `AnyRenderAbstractViewport` with `as_abstract_viewport()`; a floating header answers `AnyRenderSliverFloatingPersistentHeader` the same way.

## sliver.rs → sliver.dart

- Change: `SliverHitTestResult` and `SliverHitTestEntry` take the `BoxHitTestResult` shape: the result only wraps a `HitTestResult`, and the entry is the target, carrying the sliver and its two positions.
  Reason: language — no subclassing of the gesture crate's result and entry types.
  Affect: `hit_test` receives `&mut SliverHitTestResult<'_>` and adds with `result.add(SliverHitTestEntry::new(sliver, main, cross).into())`; `handle_event` receives the `SliverHitTestEntry`.

- Change: `SliverGeometry::is_zero()` answers Dart's `geometry == SliverGeometry.zero` by comparing the fields.
  Reason: language — `SliverGeometry` does not override `==`, so Dart compares canonicalized const identity, which a `Copy` value has none of.
  Affect: call `geometry.is_zero()`; a geometry that happens to equal `SliverGeometry::ZERO` field by field reads as zero.

- Change: the child-position helpers (`child_main_axis_position` and its siblings) take an `AnyRenderObject`.
  Reason: language — no `covariant` parameter narrowing.
  Affect: a sliver whose children are boxes downcasts with `child.as_box()`.

## sliver_multi_box_adaptor.rs → sliver_multi_box_adaptor.dart

- Change: Dart's `KeepAliveParentDataMixin` is the `KeepAliveParentData` half, and `parentData is KeepAliveParentDataMixin` is `parent_data.part::<KeepAliveParentData>()`, which `SliverMultiBoxAdaptorParentData` answers from `provide` with its embedded half.
  Reason: language — no mixins and no subclass check on an erased `ParentData`.
  Affect: parent data used under `RenderSliverWithKeepAliveMixin` embeds a `KeepAliveParentData` and answers it from `provide` and `provide_mut`; `KeepAlive` sets `keep_alive` on the half it gets back.

- Change: Flutter's `adoptChild` override is the post-hook `did_adopt_child`, the `did_attach` shape, which `insert` runs after the child joins the list; `paintsChild` is an inherent `paints_child`; and the keep-alive bucket rebuilds a returning child's parent data around `drop_child` instead of re-assigning the value Dart saved.
  Reason: language — `adopt_child` is an inherent method on the type-erased handle, not a virtual, and `drop_child` clears the parent data box rather than leaving a reference behind.
  Affect: a subclass that would override `adoptChild` implements `did_adopt_child`; a keep-alive child comes back out of the bucket with the same index, layout offset and keep-alive flag it went in with.

## sliver_persistent_header.rs → sliver_persistent_header.dart

- Change: the `vsync` field is a `TickerProviderRef`, an `Rc<dyn TickerProvider>` that compares by pointer.
  Reason: language — Dart compares the provider by object identity, which `Option<Rc<dyn TickerProvider>>` has no `==` for.
  Affect: `header.set_vsync(app, Some(TickerProviderRef::new(state_handle)))`.

- Change: `RenderSliverFloatingPinnedPersistentHeader` carries only its `update_geometry` body; a leaf that is one overrides `RenderSliverFloatingPersistentHeader::update_geometry` and forwards to it.
  Reason: language — no inheritance, so the pinned variant's override has to be installed on the trait the base body dispatches through.
  Affect: a floating pinned header's `impl RenderSliverFloatingPersistentHeader` forwards `update_geometry` to `RenderSliverFloatingPinnedPersistentHeader::update_geometry(self, app)` by name.

- Change: Flutter's `markNeedsLayout` override is the named `RenderSliverPersistentHeader::mark_needs_layout`.
  Reason: language — `mark_needs_layout` is an inherent method on the type-erased handle, not a virtual.
  Affect: call `RenderSliverPersistentHeader::mark_needs_layout(self, app)` on a persistent header so that the child is remeasured next layout; the type-erased handle's `mark_needs_layout` does not set that flag.

## tweens.rs → tweens.dart

- Change: `FractionalOffsetTween`, `AlignmentTween` and `AlignmentGeometryTween` are `Clone` values that implement `Animatable<T>` directly, and their `new(begin, end)` takes no `App`, where `reveal_animation`'s tweens are arena objects.
  Reason: language — orphan rule: an `Animatable` impl for a `Handle` of one of these tweens names no type of this crate (widgets' `EdgeInsetsTween` has the same shape for the same reason).
  Affect: `animation.drive(app, AlignmentGeometryTween::new(Some(a), Some(b)))` drives a clone; writing `begin` or `end` afterwards does not reach the driven animation, as it does on a Dart `Tween`.

## Deferred
- Anisotropic backdrop blurs, and a backdrop colour stage for the colour filter `ImageFilterConfig.compose` can carry. Trigger: valo growing an anisotropic backdrop blur and a `Backdrop` colour stage.
- `PaintingContext.addLayer` / `addCompositionCallback` / `pushColorFilter`, `LeaderLayer` / `FollowerLayer`, `toImage`. Trigger: `CompositedTransformFollower`, `RepaintBoundary.toImage`.
- Debug paint overlays: `debugPaint` on boxes, slivers and the viewport (with the sliver arrow and `debugPaintSize` helpers), `describeApproximatePaintClip` and `CustomClipper.getApproximateClipRect`, the custom clip's `debugPaintSize`, and the `DebugOverflowIndicatorMixin` overlays of `RenderFlex` and `RenderConstraintsTransformBox` (an overflowing box clips but paints no striped hint). With them, `paintsChild` as a virtual: `RenderOffstage` and `RenderFittedBox` keep it inherent meanwhile, and the opacity boxes' overrides wait. Trigger: inspector; semantics.
- Semantics: on `PipelineOwner` and `RenderObject`; `RenderCustomPaint`'s `CustomPainterSemantics` and the painter's semantics builder; the deprecated `ignoringSemantics` of the ignore- and absorb-pointer boxes; `RenderOffstage.visitChildrenForSemantics`; the viewport and sliver semantics overrides (configuration, clip, children, `useTwoPaneSemantics` / `excludeFromScrolling`, `ensureSemantics` / `semanticBounds`) and the `markNeedsSemanticsUpdate` calls in the viewport's `paintOrder` and `clipBehavior` setters. Trigger: a11y; do not stub.
- The semantics half of `PipelineManifold`: `semanticsEnabled`, its `Listenable` surface, and the semantics owner an attached `PipelineOwner` updates. The frame-request half is ported: the binding attaches its root owner to a manifold that calls `ensure_visual_update`, and `adopt_child` hands it down to the child owner the widgets `View` creates per `RenderView`. `RendererBinding::init_render_view` stays for render-tree-only hosts, rooting the implicit view's `RenderView` in `root_pipeline_owner` as Flutter's test binding does — never call it in an app that runs `run_app`. Trigger: semantics.
- The layout-contract asserts: `_DebugSize`, the wrapper that reports a child's size read during a dry layout; `RenderBox.debugAssertDoesMeetConstraints`; `RenderSliver.debugAssertDoesMeetConstraints` with the `geometry` setter's contract asserts; and `RenderSliverFixedExtentBoxAdaptor.debugAssertDoesMeetConstraints`. `SliverGeometry::debug_assert_is_valid` is ported and the viewport calls it on every child. Trigger: the first layout bug one of them would have caught.
- `layout` and `constraints` as override points. `markNeedsLayout` is one on the box protocol (its vtable slot resolves `RenderBox::mark_needs_layout`, which the layout cache overrides); a sliver and the view keep the base body. Trigger: OverlayPortal, `RenderView`. Ask before adding.
- `RenderView.applyPaintTransform` / `updateSystemChrome`; `performReassemble`. Trigger: `getTransformTo`, hot reload.
- `RenderParagraph.RelayoutWhenSystemFontsChangeMixin` and `applyPaintTransform`; `RenderEditable`. Trigger: `PaintingBinding.systemFonts`; `EditableText`.
- `SliverConstraints.debugAssertIsValid` extra numeric checks. Trigger: a caller that relies on those messages.
- Baseline alignment on the multi-child boxes: `RenderFlex`'s ascent/descent pass with its actual and dry baseline computations, `RenderStack` / `RenderIndexedStack`'s per-child baseline and their baseline computations, and `RenderIgnoreBaseline`; until it lands a `CrossAxisAlignment::Baseline` row top-aligns its children and stores the `text_baseline` unused. The one-child boxes report baselines, and `RenderBoxContainerDefaultsMixin`'s two baseline helpers are ported. Trigger: the first baseline-aligned `Row`.
- `RenderCustomSingleChildLayoutBox` and `SingleChildLayoutDelegate` (`shifted_box.dart`; `custom_layout.rs` holds only the multi-child pair, as Dart does). Trigger: `CustomSingleChildLayout`.
- `RenderBoxBase`, Flutter's `RenderBox.hitTest` body reachable from an override, sits in `proxy_box.rs` next to its callers and repeats `RenderBox::hit_test`'s default. Trigger: the next edit of `box.rs` — move it beside `RenderBox` and have the default forward to it.
- `RenderTransform.filterQuality` and Dart's `ImageFilterLayer` path. Trigger: the first `Transform(filterQuality:)`, with `PaintingContext.pushColorFilter`.
