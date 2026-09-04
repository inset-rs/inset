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
  Affect: to write a render object, declare the struct with the mixin fields, `impl RenderObject` (accessors, `perform_layout`, `paint`, `visit_children`) and `impl RenderBox` (accessors), then `RenderHandle::new_box(app, value)`. Parents store the child's edge (`child.as_box()`); the pipeline stores `as_object()`. Tree methods (`mark_needs_layout`, `adopt_child`, `parent`, …) come from the protocol trait, so import `RenderBox` or `RenderSliver` to call them. An edge downcasts with `as_box()` / `as_sliver()`, Dart's `as RenderBox`. A crate that calls an inherent `self: RenderHandle<Self>` method (a setter on `RenderPadding`, `set_child` on `RenderView`) needs `#![feature(arbitrary_self_types)]` itself; trait methods resolve without it.

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

## box.rs → box.dart (hit testing)

- Change: `BoxHitTestResult` only wraps: `BoxHitTestResult::wrap(&mut result)` is a view over a `HitTestResult`, and there is no standalone constructor. `BoxHitTestEntry` is the entry's *target*, carrying the box and its local position; `result.add(BoxHitTestEntry::new(box, position).into())`.
  Reason: language — no subclassing of the gesture crate's result and entry types.
  Affect: a `hit_test` receives `&mut BoxHitTestResult<'_>`; `handle_event` receives the `BoxHitTestEntry` and reads `local_position()` from it.

- Change: `RenderPointerListener`'s callbacks are set after construction (`set_on_pointer_down`, …), and `on_pointer_signal` receives the `PointerEvent` enum.
  Reason: language — no optional named constructor arguments; signal events are three enum variants with no common type.
  Affect: construct with `(app, behavior, child)` and set the callbacks you need; match the enum in a signal listener.

## painting_context.rs → object.dart (PaintingContext), layer.rs → layer.dart

- Change: there is no `Layer` object tree. A repaint boundary keeps its recording as retained items (pictures, references to child boundaries, push/pop effects) plus one `CompositedLayer`, Flutter's `OffsetLayer` / `OpacityLayer` / `TransformLayer` as a value. The host recomposes the frame from those retained pieces every time; a boundary that did not change contributes the same pictures.
  Reason: platform — valo composes a display list from retained pictures and has no engine layers to retain between frames; Flutter's layer tree exists for that engine boundary.
  Affect: `update_composited_layer` returns a `CompositedLayer` and `mark_needs_composited_layer_update` replaces it without repainting the subtree; `push_clip_*` / `push_transform` / `push_opacity` take no `needsCompositing` or `oldLayer` and return nothing; `schedule_initial_paint` takes a `CompositedLayer`; `debug_layer()` reports `attached()` and the composited layer.

- Change: no compositing bits (`needsCompositing`, `alwaysNeedsCompositing`, `flushCompositingBits`). Every pushed effect spans child boundaries.
  Reason: platform — the layer-versus-canvas choice exists because a Skia clip cannot cross an engine layer; retained items have no such split.
  Affect: `is_repaint_boundary` alone decides where recordings split; where Flutter calls `markNeedsCompositingBitsUpdate` because that answer changed, call `mark_needs_paint`. A frame is `flush_layout` then `flush_paint`.

## view.rs → view.dart, binding.rs → binding.dart

- Change: `RenderView` is its own kind of render object, neither box nor sliver: its `constraints` and `size` are inherent methods, its child slot is written out, and it has no protocol edge; `RendererBinding` reaches it through the typed handle.
  Reason: language — Flutter overrides the `constraints` getter on a `RenderObject`; here constraints belong to a protocol trait, and the view's come from its configuration.
  Affect: `RenderView::new(app, child, configuration, view)` returns a typed handle; use `as_object()` for the tree.

- Change: `composite_frame` composes the retained recording into a `Canvas` and hands the picture to `View::present`; there is no scene or physical size argument.
  Reason: platform — the host presents a display list, and it owns the surface size.
  Affect: none beyond the host trait.

- Change: `RendererBinding::draw_frame` calls a registered `RendererBindingOverrides` before layout and after compositing; the widgets binding registers itself with `set_overrides` and implements the object-side `RendererBindingOverridesObject`.
  Reason: language — Flutter's `WidgetsBinding` overrides `drawFrame` through mixin order; a crate above cannot override a method below it (the same shape as the gesture binding's overrides).
  Affect: none for callers.

## mouse_tracker.rs → mouse_tracker.dart

- Change: Dart's `target is MouseTrackerAnnotation` is the virtual `RenderObject::mouse_tracker_annotation`, `None` by default; `RenderMouseRegion` answers with its current callbacks, cursor, and validity. The tracker keys its per-device annotation maps by the render object (an `AnyRenderObject`), and reads the annotation from it again each time it dispatches.
  Reason: language — an erased render object cannot be asked whether it implements an interface; the object is the annotation's identity, as in Dart.
  Affect: a render object that should receive enter/exit events overrides `mouse_tracker_annotation`. A render object that has left the arena reads as no annotation, where Dart would still hold the object with `validForMouseTracker == false`; both skip it.

- Change: the hit-test callback given to `MouseTracker::new` is an `Rc<dyn Fn(&mut App, Offset, ViewId) -> HitTestResult>`; `mouse_is_connected` listeners are added through `Listenable`.
  Reason: language — the callback runs against `App` and lives inside an arena object.
  Affect: `RendererBinding::init_mouse_tracker` builds it; tests pass their own.

- Change: `RendererBinding` feeds the tracker through `GestureBindingOverrides::will_dispatch_event` and schedules `update_all_devices` as a post-frame callback from its persistent frame callback, as Flutter does.
  Reason: language — a crate above cannot override `GestureBinding.dispatchEvent`; see gestures `PORTING.md`.
  Affect: none for callers.

- Change: `RenderMouseRegion::new(app, valid_for_mouse_tracker, child)`; callbacks, `cursor`, `opaque`, and `hit_test_behavior` have setters.
  Reason: language — no optional named constructor arguments; `validForMouseTracker` is the one argument without a setter.
  Affect: pass `true` unless a test wants an invalid region.

## pipeline_owner.rs → object.dart (PipelineOwner)

- Change: `onNeedVisualUpdate` is a `Listener`, which receives `&mut App`.
  Reason: language — a Rust closure cannot capture what it mutates.
  Affect: `PipelineOwner::new(app, Some(Listener::new(|app| …)))`.

## sliver.rs → sliver.dart

- Change: `as_box_constraints` takes `(min_extent, max_extent, cross_axis_extent)` as `Option`s.
  Reason: language — no named optional parameters.
  Affect: pass `None` for Dart's defaults (`0`, infinity, this sliver's cross extent).

## proxy_box.rs → proxy_box.dart / shifted_box.rs → shifted_box.dart

- Change: Flutter's base classes and mixins (`RenderProxyBox`, `RenderShiftedBox`, `RenderAligningShiftedBox`, `RenderAnimatedOpacityMixin`) are traits holding the shared bodies; a render object is always a leaf struct that implements them.
  Reason: language — no inheritance.
  Affect: implement the marker trait and, where Dart would run the inherited method, call it by name: `RenderProxyBoxMixin::paint(self, app, context, offset)`. Mixin state is a field (`RenderAnimatedOpacityData`, `RenderAligningShiftedBoxData`) with an accessor.

- Change: `RenderOpacity` and `RenderAnimatedOpacity` have no `alwaysIncludeSemantics`; `RenderDecoratedBox` gives its painter no `onChanged`.
  Reason: platform — accessibility is deferred; a `BoxPainter` callback cannot reach `App` yet, so an image decoration cannot request a repaint when its image loads.
  Affect: pass no semantics flag; decorations that load images do not repaint on their own.

## paragraph.rs → paragraph.dart

- Change: `RenderParagraph::new(app, text, text_direction, fonts)`; Dart's other constructor arguments are setters. `fonts` is an optional `FontCollection` handle; `None` shapes against `PaintingBinding`'s app-wide one. It is a leaf: no inline children, no selection registrar, no `selectionColor`.
  Reason: language — no optional named parameters; platform — the font collection is explicit (see painting `binding.rs`), `WidgetSpan` needs placeholders the host lacks, and selection waits on `selection.dart`.
  Affect: set `overflow`, `max_lines`, `soft_wrap`, … after construction. A hit test stops at the paragraph (spans are not targets), so `RichText` recognizers cannot fire.

- Change: `TextOverflow::Fade` clips like `Clip`.
  Reason: platform — the fade is a gradient shader, and gradients are deferred.
  Affect: overflowing text is cut, not faded.

## Deferred

- `PaintingContext.addLayer` / `addCompositionCallback` / `pushColorFilter`, `Layer.find` annotations, `LeaderLayer` / `FollowerLayer`, `toImage`. Trigger: `AnnotatedRegion`, `CompositedTransformFollower`, `RepaintBoundary.toImage`.
- Debug paint overlays, `debugPaint`, `applyPaintTransform` / `getTransformTo`, `paintsChild`. Trigger: inspector; `RenderBox.localToGlobal`.
- Semantics on `PipelineOwner` and `RenderObject`. Trigger: a11y; do not stub.
- `PipelineManifold`. The widgets `View` creates a child `PipelineOwner` per `RenderView` (as Flutter); `RendererBinding::init_render_view` stays for render-tree-only hosts, rooting the implicit view's `RenderView` in `root_pipeline_owner` as Flutter's test binding does — never call it in an app that runs `run_app`. Trigger: semantics / the manifold's `onSemanticsEnabledChanged`.
- `computeDryLayout` / `_DebugSize` / `globalToLocal` / `localToGlobal`. Trigger: `RenderBox` public extras; `getTransformTo`.
- `RenderProxyBox` / `RenderShiftedBox` intrinsics and dry layout. Trigger: the first intrinsic-sizing parent (`Row`, `IntrinsicWidth`).
- `invokeLayoutCallback`. Trigger: `LayoutBuilder`; also widen `layout_without_resize` for a non-boundary layout-callback host.
- `layout` / `markNeedsLayout` / `constraints` as override points. Trigger: OverlayPortal, `RenderView`, the first `markNeedsLayout` override. Ask before adding.
- `RenderView.applyPaintTransform` / `updateSystemChrome`; `performReassemble`. Trigger: `getTransformTo`, hot reload.
- `RenderParagraph` intrinsics, dry layout, baselines, `RelayoutWhenSystemFontsChangeMixin`, `applyPaintTransform`; `RenderEditable`. Trigger: `RenderBox` intrinsics and baselines; `PaintingBinding.systemFonts`; `EditableText`.
- Viewport / `ViewportOffset` / sliver-to-box adapters / sliver parent data. Trigger: first viewport. `RenderObjectWithChildMixin` is box-only until then.
- `SliverConstraints.debugAssertIsValid` extra numeric checks. Trigger: a caller that relies on those messages.
