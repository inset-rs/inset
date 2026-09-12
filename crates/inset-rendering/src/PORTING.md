# inset-rendering/src
Flutter home: packages/flutter/lib/src/rendering
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- box.rs → BoxConstraints
- proxy_box.rs → HitTestBehavior
- object.rs → Constraints (`isTight` / `isNormalized` / `debugAssertIsValid`)
- relayout_when_system_fonts_change.rs → object.dart (`RelayoutWhenSystemFontsChangeMixin`)
- sliver.rs → GrowthDirection, applyGrowthDirectionToAxisDirection / ScrollDirection, SliverConstraints (`isTight` / `isNormalized` / `asBoxConstraints` / `==`)
- viewport_offset.rs → ScrollDirection, flipScrollDirection
- flex.rs → FlexFit, MainAxisSize, MainAxisAlignment
- stack.rs → StackFit
- box.rs → RenderBox intrinsics, dry layout and dry baselines with their caches
- layout_helper.rs → layout_helper.dart
- proxy_box.rs → RenderAspectRatio, RenderIntrinsicWidth, RenderIntrinsicHeight, RenderFittedBox
- shifted_box.rs → OverflowBoxFit, RenderConstrainedOverflowBox, RenderSizedOverflowBox, RenderFractionallySizedOverflowBox, RenderCustomSingleChildLayoutBox, SingleChildLayoutDelegate, RenderBaseline
- animated_size.rs → animated_size.dart
- viewport.rs → RenderViewportBase.debugThrowIfNotCheckingIntrinsics and the four intrinsics, RenderViewport.computeDryLayout
- object.rs / box.rs → getTransformTo, globalToLocal, localToGlobal
- editable.rs → TextSelectionPoint
- debug.rs → debug.dart
- pipeline_owner.rs → object.dart (PipelineOwner)
- sliver_fixed_extent_list.rs → sliver_fixed_extent_list.dart
- image.rs → image.dart (RenderImage)

## object.rs → object.dart (RenderObject)

- Change: Flutter's `attach` / `detach` overrides are the post-hooks `did_attach` / `did_detach`, which run after the base body.
  Reason: language — a trait default cannot call `super`; no Flutter override reads its owner before `super.detach`.
  Affect: a hook cannot act before the owner is set, and `did_detach` runs with the owner already cleared.

- Change: a panic in `perform_layout`, `perform_resize` or `paint` unwinds.
  Reason: language — no `FlutterError.reportError` hook to catch and continue.
  Affect: layout and paint do not survive a failing render object.

- Change: `layout` lives on the protocol's type-erased handle and takes that protocol's constraints.
  Reason: language — there is no abstract `Constraints` an erased node can hold.
  Affect: a parent that does not know its child's protocol cannot lay it out; there is no protocol-neutral `layout`.

## box.rs → box.dart (hit testing)

- Change: `BoxHitTestResult` is a view over a `HitTestResult` with no standalone constructor, and `BoxHitTestEntry` is the entry's target rather than a subclass of the entry.
  Reason: language — no subclassing of the gesture crate's result and entry types.
  Affect: a box hit test only runs inside a live `HitTestResult`; `handle_event` receives the `BoxHitTestEntry`.

- Change: a render object named by a pressed pointer's hit-test path, or by the mouse tracker, outlives its `dispose` until they let go.
  Reason: language — Dart keeps it alive by holding it; here the holder retains it (`App::retain`).
  Affect: a `Listener` rebuilt mid-press still receives its release; a region that just left the tree still receives its exit.

## painting_context.rs → object.dart (PaintingContext), layer.rs → layer.dart

- Change: there are no engine layers: `addToScene` runs for every layer every frame, and the retained-layer members (`engineLayer`, `markNeedsAddToScene`, `addRetained` and the rest) do not exist.
  Reason: platform — the host paints one display list per frame and has nothing to retain between frames.
  Affect: a layer property setter needs no "mark", and no subtree is ever skipped at composite time.

- Change: a `LayerHandle` dropped without being cleared keeps its layer.
  Reason: language — the handle lives inside an arena object, so its setter takes the `App` (`LayerHandle::set_layer`) and its drop has none.
  Affect: that layer's entry lives until the `App` drops, where Dart's collector would take it.

- Change: `Layer.dispose` ends in `App::destroy`.
  Reason: language — no GC.
  Affect: a handle to a disposed layer is stale and `app.get` panics on it; `AnyLayer::debug_disposed` answers true, and a composition callback for it is a no-op.

- Change: annotation search is the non-generic `AnnotationSearch` in place of `AnnotationResult<S>`.
  Reason: language — a generic method cannot sit in a vtable.
  Affect: an annotated region's value is an `Rc<dyn Any>` found only by its exact type, and a `find_annotations` override asks `result.accepts(..)` before adding.

- Change: `LeaderLayer` / `FollowerLayer` hold the `LayerLink` as a `RetainedHandle`.
  Reason: language — Dart's GC keeps a link alive while a detached layer still names it.
  Affect: the link's entry lingers until those layers are disposed, past the `dispose` of the state that owns it.

## view.rs → view.dart, binding.rs → binding.dart

- Change: `RendererBinding` calls the hooks `will_draw_frame`, `did_draw_frame` and `did_handle_metrics_changed` on a registered `RendererBindingOverrides` instead of being overridden.
  Reason: language — Flutter's `WidgetsBinding` overrides `drawFrame` and `handleMetricsChanged` through mixin order, and a crate above cannot override a method below it.
  Affect: a binding layered above registers with `set_overrides` to get its build phase; a render-tree-only host runs the plain frame.

## mouse_tracker.rs → mouse_tracker.dart

- Change: Dart's `target is MouseTrackerAnnotation` is the virtual `RenderObject::mouse_tracker_annotation`, which the tracker re-reads from the render object on every dispatch.
  Reason: language — an erased render object cannot be asked whether it implements an interface.
  Affect: a render object receives enter and exit only if it overrides that virtual; one that has left the arena reads as no annotation and is skipped.

## proxy_box.rs → proxy_box.dart / shifted_box.rs → shifted_box.dart

- Change: `RenderOpacity` and `RenderAnimatedOpacity` have no `alwaysIncludeSemantics`, and `RenderDecoratedBox` gives its painter no `onChanged`.
  Reason: platform — accessibility is deferred, and a `BoxPainter` callback cannot reach `App` yet.
  Affect: a decoration that loads an image does not repaint when the image arrives.

- Change: `CustomClipper<T>` is a trait held as an `Rc`: `reclip()` is a method the implementor answers, `should_reclip` receives the old clipper as `&dyn CustomClipper<T>`, and two clippers compare by `Rc::ptr_eq`.
  Reason: language — a Rust trait has no fields, no constructor, no `runtimeType` and no identity equality on a boxed value.
  Affect: a fresh `Rc` around an equal clipper counts as a different clipper and re-subscribes; `old_clipper` is read with `as_any().downcast_ref()`.

- Change: `RenderTransform` has no `filterQuality`.
  Reason: platform — the filtered path is an `ImageFilterLayer` over `ImageFilter.matrix`, which neither the embedder's `Paint` nor `SceneBuilder` has.
  Affect: a transform always paints through `push_transform`, or through the child's paint offset when the matrix is a translation.

- Change: `RenderColoredBox` does not pass `is_anti_alias` to the canvas.
  Reason: platform — valo's `Paint` has no anti-alias flag; painting's `PORTING.md` records the same for `Clip`.
  Affect: the fill is drawn the way valo draws it, and setting the flag only marks a repaint.

## shifted_box.rs → shifted_box.dart (RenderConstraintsTransformBox)

- Change: `BoxConstraintsTransform` is a plain `fn(BoxConstraints) -> BoxConstraints`, and the setter compares the old and new transform by function address.
  Reason: language — a Rust closure has no identity, and Dart's setter compares two function values to decide whether anything changed.
  Affect: a transform that captures state cannot be used; when the addresses differ for the same function the setter still only marks layout if the current constraints map to a different value.

## image_filter_config.rs → image_filter_config.dart

- Change: a blur has no `tileMode`.
  Reason: platform — valo's blur has no tile mode (`inset-embedder`'s `PORTING.md` records that).
  Affect: every blur uses valo's edge behaviour, and `debug_short_description` prints no tile-mode word.

## custom_paint.rs → custom_paint.dart

- Change: `CustomPainter` is a trait held as an `Rc`, with `repaint()` a method the implementor answers and identity by `Rc::ptr_eq`.
  Reason: language — a trait has no fields, constructor or identity.
  Affect: a painter wrapped in a new `Rc` each frame is unsubscribed and re-subscribed each frame.

## paragraph.rs → paragraph.dart

- Change: `RenderParagraph` takes an optional `FontCollection` handle, falling back to `PaintingBinding`'s app-wide one, and is a leaf: no inline children, no selection registrar, no `selectionColor`.
  Reason: platform — the font collection is explicit (painting's `binding.rs`), `WidgetSpan` needs placeholders the host lacks, and selection waits on `selection.dart`.
  Affect: a hit test stops at the paragraph, so a `RichText` span recognizer never fires and text in it cannot be selected.

- Change: `TextOverflow::Fade` clips like `Clip`.
  Reason: platform — the fade is a gradient shader, and gradients are deferred.
  Affect: overflowing text is cut, not faded.

## editable.rs → editable.dart

- Change: `RenderEditable` is a leaf: no inline children, no custom-paint child boxes, no internal tap / long-press recognizers, and caret, selection and the handle leaders paint in `paint`.
  Reason: platform — `WidgetSpan` placeholders and engine layers wait, as on `RenderParagraph`.
  Affect: selection is driven from above (`select_position_at` under an `ignore_pointer`) rather than by the editable's own recognizers, and the handles' links are led from `paint` for a `CompositedTransformFollower`.

- Change: `VerticalCaretMovementRun::is_valid` compares the editable's layout generation.
  Reason: language — `compute_line_metrics` returns a new `Vec` each call, so Dart's `identical` on the list would always fail.
  Affect: a run stays valid across reads until the next layout.

## object.rs / box.rs → object.dart / box.dart (container children)

- Change: `ParentData` answers the type-keyed query `provide(TypeId)` / `provide_mut`, read as `part::<P>()`, so a value built from several halves answers for each of them; `ContainerBoxParentData` joins the container half with `BoxParentData`.
  Reason: language — an exact downcast cannot reach Dart's `parentData! as BoxParentData` on a subclass, and one cast method per half would close the set of halves to this crate.
  Affect: parent data that does not answer `BoxParentData` from `provide` cannot be positioned by a box parent; a child's offset is read as `child.box_parent_data(app).offset`.

## custom_layout.rs → custom_layout.dart

- Change: `MultiChildLayoutDelegate` is a trait held as an `Rc`, with `relayout()` a method the implementor answers and identity by `Rc::ptr_eq`.
  Reason: language — a Rust trait has no constructor, no fields, no `runtimeType` and no identity equality on a boxed value.
  Affect: a delegate rewrapped in a new `Rc` re-subscribes; `old_delegate` is read with `as_any().downcast_ref()`.

- Change: Dart's four `FlutterError`s (no such child, a child laid out twice, invalid constraints, a child left unlaid-out) are panics, and the ones Dart raises from an `assert` are debug-only.
  Reason: language — no catchable exception, and the set of children still needing layout is debug state.
  Affect: a delegate that misuses its children takes the frame down instead of reporting a `FlutterError`.

## stack.rs → stack.dart

- Change: `RelativeRect::lerp(Some(a), None, t)` interpolates `a` towards `RelativeRect::FILL`.
  Reason: language — Dart's branch reads `b!` on the null `b`, which Rust cannot express.
  Affect: the call returns a value where Dart throws.

## sliver.rs → sliver.dart

- Change: `SliverGeometry::is_zero()` answers Dart's `geometry == SliverGeometry.zero` by comparing the fields.
  Reason: language — `SliverGeometry` does not override `==`, so Dart compares canonicalized const identity, which a `Copy` value has none of.
  Affect: a geometry that happens to equal `SliverGeometry::ZERO` field by field reads as zero where Dart's identity check says no.

## viewport.rs → viewport.dart

- Change: Dart's `object is RenderAbstractViewport` is the type-keyed query `RenderObject::interface(TypeId)`, read as `object.interface::<AnyRenderAbstractViewport>()`.
  Reason: language — an erased render object cannot be asked whether it implements an interface, and one virtual per interface would close the set of interfaces to this crate.
  Affect: a viewport that does not answer that id is not found by `RenderAbstractViewport::of`, and neither is a floating header that does not answer its own.

## sliver_persistent_header.rs → sliver_persistent_header.dart

- Change: Flutter's `markNeedsLayout` override is the named `RenderSliverPersistentHeader::mark_needs_layout`.
  Reason: language — `mark_needs_layout` is an inherent method on the type-erased handle, not a virtual.
  Affect: marking a persistent header through the erased handle does not remeasure its child; the named call does.

## tweens.rs → tweens.dart

- Change: `FractionalOffsetTween`, `AlignmentTween` and `AlignmentGeometryTween` are `Clone` values that implement `Animatable<T>` directly, where `inset_animation`'s tweens are arena objects.
  Reason: language — orphan rule: an `Animatable` impl for a `Handle` of one of these tweens names no type of this crate.
  Affect: `drive` takes a clone, so writing `begin` or `end` afterwards does not reach the driven animation, as it does on a Dart `Tween`.

## Deferred

- `RenderImage`'s `color` / `colorBlendMode` and `invertColors`. Trigger: painting's colour filters.
- `RenderImage`'s `centerSlice`. Trigger: an image drawn as a resizable frame.
- `RenderImage`'s `filterQuality` and `isAntiAlias`. Trigger: a caller that needs to choose how an image is sampled.
- `RenderImage`'s semantics label. Trigger: accessibility.
- `TextureLayer`, `PlatformViewLayer`, `PerformanceOverlayLayer`, `ClipRSuperellipseLayer`, `ColorFilterLayer`, `ImageFilterLayer`, `ShaderMaskLayer`, `OffsetLayer.toImage` / `toImageSync`, `PaintingContext.pushColorFilter`, `RenderView._updateSystemChrome`. Trigger: a texture or platform view, a superellipse clip, a colour/image/shader filter widget, `RepaintBoundary.toImage`, system chrome.
- `PaintingContext.addCompositionCallback`. Trigger: a caller that needs composition callbacks on the painting context (layers already have them).
- Debug paint overlays on boxes, slivers and the viewport, and the overflow indicators of `RenderFlex` and `RenderConstraintsTransformBox` (an overflowing box clips but paints no striped hint); with them, `paintsChild` as a virtual. Trigger: inspector.
- Semantics on `PipelineOwner`, `RenderObject`, `RenderCustomPaint`, the ignore- and absorb-pointer boxes, `RenderOffstage`, the viewport and the slivers. Trigger: a11y; do not stub.
- The semantics half of `PipelineManifold`; the frame-request half is ported, and `RendererBinding::init_render_view` stays for render-tree-only hosts and must never be called in an app that runs `run_app`. Trigger: semantics.
- The layout-contract asserts: `_DebugSize` and `debugAssertDoesMeetConstraints` on box, sliver and the fixed-extent adaptor; `SliverGeometry::debug_assert_is_valid` is ported and the viewport calls it. Trigger: the first layout bug one of them would have caught.
- `layout` and `constraints` as override points; `markNeedsLayout` is one on the box protocol only. Trigger: OverlayPortal, `RenderView`. Ask before adding.
- `RenderView.applyPaintTransform` / `updateSystemChrome`; `performReassemble`. Trigger: `getTransformTo`, hot reload.
- `RenderParagraph.applyPaintTransform`. Trigger: a caller that needs the paragraph's paint transform.
- `RenderEditable` inline children, the custom-paint child boxes and the internal tap / long-press recognizers. Trigger: `WidgetSpan`; a field that does not set `ignorePointer`.
- `SliverConstraints.debugAssertIsValid` extra numeric checks. Trigger: a caller that relies on those messages.
- Baseline alignment on the multi-child boxes and `RenderIgnoreBaseline`; meanwhile a `CrossAxisAlignment::Baseline` row top-aligns its children. Trigger: the first baseline-aligned `Row`.
- `RenderBoxBase` sits in `proxy_box.rs` beside its callers instead of next to `RenderBox`. Trigger: the next edit of `box.rs`.
- `RenderTransform.filterQuality` and Dart's `ImageFilterLayer` path. Trigger: the first `Transform(filterQuality:)`, with `PaintingContext.pushColorFilter`.
