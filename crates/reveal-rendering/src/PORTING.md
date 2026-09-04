# reveal-rendering/src
Flutter home: packages/flutter/lib/src/rendering
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- box.rs → BoxConstraints
- proxy_box.rs → HitTestBehavior
- object.rs → Constraints (`isTight` / `isNormalized` / `debugAssertIsValid`)
- sliver.rs → GrowthDirection, applyGrowthDirectionToAxisDirection / ScrollDirection, SliverConstraints (`isTight` / `isNormalized` / `asBoxConstraints` / `==`)
- viewport_offset.rs → ScrollDirection, flipScrollDirection

## debug.rs → debug.dart

- Change: `debug_*` / `set_debug_*` functions instead of assigning a library `bool`.
  Reason: language — Rust has no isolate-global assignable `bool`.
  Affect: call the setter; the getter is the Dart read.

## object.rs → object.dart (RenderObject)

- Change: a render object is a struct in the `App` arena, reached through a typed handle. Flutter's base-class fields are fields on the struct (`render_object`, `render_box`, a child slot), each with an accessor the trait asks for. Methods take `self: RenderHandle<Self>` and `&mut App`, not `&mut self`. A reference to "some render object" is an erased edge: `AnyRenderObject`, or `AnyRenderBox` / `AnyRenderSliver` when the protocol is known.
  Reason: language — no inheritance and no GC identity; a `&mut self` receiver would hold the node borrowed while its child lays out, and the child must be able to reach back into it.
  Affect: to write a render object, declare the struct with the mixin fields, `impl RenderObject` (accessors, `perform_layout`, `paint`, `visit_children`) and `impl RenderBox` (accessors), then `RenderHandle::new_box(app, value)`. Parents store the child's edge (`child.as_box()`); the pipeline stores `as_object()`. Tree methods (`mark_needs_layout`, `adopt_child`, `parent`, …) come from the protocol trait, so import `RenderBox` or `RenderSliver` to call them. An edge downcasts with `as_box()` / `as_sliver()`, Dart's `as RenderBox`.

- Change: Flutter's `attach(owner)` / `detach()` overrides are the hooks `did_attach` / `did_detach`, which run after the base body. The defaults walk `visit_children`, as do `redepth_children`'s.
  Reason: language — a trait default cannot call `super`. Every Flutter override calls `super.attach` first, and no `detach` override reads its own owner before `super.detach`, so a post-hook is equivalent.
  Affect: put what Dart writes after `super.attach(owner)` in `did_attach`; implement `visit_children` and the walks come for free.

- Change: a panic in `perform_layout`, `perform_resize`, or `paint` unwinds.
  Reason: language — no `FlutterError.reportError` hook to catch and continue.
  Affect: layout and paint do not survive a failing render object.

- Change: parent data is a trait object with typed access: `parent_data_of::<P>`, `parent_data_of_mut::<P>`, `parent_data_is::<P>`. Flutter's bare `ParentData()` is `EmptyParentData`.
  Reason: language — no subclass cast.
  Affect: write `child.parent_data_of::<BoxParentData>(app)` where Dart writes `child.parentData! as BoxParentData`.

- Change: `layout` is on the protocol edge and takes that protocol's constraints.
  Reason: language — there is no abstract `Constraints` an erased node can hold.
  Affect: `child.layout(app, box_constraints, parent_uses_size)` on an `AnyRenderBox`; there is no protocol-neutral `layout`.

## painting_context.rs → object.dart (PaintingContext), layer.rs → layer.dart

- Change: there is no `Layer` object tree. A repaint boundary keeps its recording as retained items (pictures, references to child boundaries, push/pop effects) plus one `CompositedLayer`, Flutter's `OffsetLayer` / `OpacityLayer` / `TransformLayer` as a value. The host recomposes the frame from those retained pieces every time; a boundary that did not change contributes the same pictures.
  Reason: platform — valo composes a display list from retained pictures and has no engine layers to retain between frames; Flutter's layer tree exists for that engine boundary.
  Affect: `update_composited_layer` returns a `CompositedLayer` and `mark_needs_composited_layer_update` replaces it without repainting the subtree; `push_clip_*` / `push_transform` / `push_opacity` take no `needsCompositing` or `oldLayer` and return nothing; `schedule_initial_paint` takes a `CompositedLayer`; `debug_layer()` reports `attached()` and the composited layer.

- Change: no compositing bits (`needsCompositing`, `alwaysNeedsCompositing`, `flushCompositingBits`). Every pushed effect spans child boundaries.
  Reason: platform — the layer-versus-canvas choice exists because a Skia clip cannot cross an engine layer; retained items have no such split.
  Affect: `is_repaint_boundary` alone decides where recordings split; a frame is `flush_layout` then `flush_paint`.

## pipeline_owner.rs → object.dart (PipelineOwner)

- Change: `onNeedVisualUpdate` is a `Listener`, which receives `&mut App`.
  Reason: language — a Rust closure cannot capture what it mutates.
  Affect: `PipelineOwner::new(app, Some(Listener::new(|app| …)))`.

## sliver.rs → sliver.dart

- Change: `as_box_constraints` takes `(min_extent, max_extent, cross_axis_extent)` as `Option`s.
  Reason: language — no named optional parameters.
  Affect: pass `None` for Dart's defaults (`0`, infinity, this sliver's cross extent).

## proxy_box.rs → proxy_box.dart / shifted_box.rs → shifted_box.dart

- Change: `RenderConstrainedBox` and `RenderPadding` are leaf structs; `RenderProxyBox` and `RenderShiftedBox` are not types.
  Reason: language — no inheritance; a leaf struct plus traits is the authoring shape.
  Affect: there is no base type to extend; write the leaf and its `paint`.

## Deferred

- `PaintingContext.addLayer` / `addCompositionCallback` / `pushColorFilter`, `Layer.find` annotations, `LeaderLayer` / `FollowerLayer`, `toImage`. Trigger: `AnnotatedRegion`, `CompositedTransformFollower`, `RepaintBoundary.toImage`.
- Debug paint overlays, `debugPaint`, `applyPaintTransform` / `getTransformTo`, `paintsChild`. Trigger: inspector; `RenderBox.localToGlobal`.
- Semantics on `PipelineOwner` and `RenderObject`. Trigger: a11y; do not stub.
- `PipelineManifold`. Trigger: `RendererBinding` attaching the root owner.
- `computeDryLayout` / `_DebugSize` / `BoxHitTestResult` / `globalToLocal`. Trigger: `RenderBox` public extras.
- `RenderProxyBox` / `RenderShiftedBox` hit-test and intrinsics. Trigger: `BoxHitTestResult`.
- `invokeLayoutCallback`. Trigger: `LayoutBuilder`; also widen `layout_without_resize` for a non-boundary layout-callback host.
- `layout` / `markNeedsLayout` / `constraints` as override points. Trigger: OverlayPortal, `RenderView`, the first `markNeedsLayout` override. Ask before adding.
- `RenderView` and `compositeFrame`. Trigger: R5; a box can stay `PipelineOwner.root_node` until then.
- `RenderParagraph` / `RenderEditable`. Trigger: `TextPainter`, container parent data, hit-test.
- Viewport / `ViewportOffset` / sliver-to-box adapters / sliver parent data. Trigger: first viewport. `RenderObjectWithChildMixin` is box-only until then.
- `SliverConstraints.debugAssertIsValid` extra numeric checks. Trigger: a caller that relies on those messages.
