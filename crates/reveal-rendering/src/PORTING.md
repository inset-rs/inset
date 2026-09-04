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

- Change: `debug_*` / `set_debug_*` instead of assigning a library `bool`.
  Reason: language — Rust has no isolate-global assignable `bool` binding.
  Affect: call the setter; the getter is the Dart read.

## object.rs → object.dart (RenderObject)

- Change: a concrete node is `RenderHandle<T>` over the authored struct. Tree edges are `AnyRenderObject` / `AnyRenderBox` / `AnyRenderSliver` (`HandleId` + a `&'static` vtable built per type from the trait impl). Methods take `self: RenderHandle<Self>` ([`arbitrary_self_types`](https://github.com/rust-lang/rust/issues/44874)), not `&mut self`.
  Reason: language — no inheritance or GC identity, and holding `&mut T` across a child `layout` would recreate shaft-rs-next's take/put lease. A downstream crate cannot inherent-impl `RenderHandle<MyBox>`.
  Affect: write methods on `impl RenderFoo` / `impl RenderBox for RenderFoo` with `self: RenderHandle<Self>`. `RenderHandle::new_box(app, value)`; store `AnyRenderBox` on box parents and `AnyRenderObject` on `PipelineOwner`.

- Change: `RenderObject` / `RenderBox` fields are mixin data on the struct (`render_object`, `render_box`) with `xxx_data` accessors.
  Reason: language — no inherited fields.
  Affect: every leaf constructs `RenderObjectData::new()` / `RenderBoxData::new()` and implements the accessors (or `render_object_accessors!` / `render_box_accessors!`).

- Change: Flutter's `attach(owner)` / `detach()` overrides are the hooks `did_attach` / `did_detach`, called after the base body. Defaults walk `visit_children`; `redepth_children` likewise.
  Reason: language — a trait default cannot call `super`, and the base body needs the erased edge, which a typed handle does not carry; `AnyRenderObject::attach` runs the base body, then the hook. Checked against Flutter: all 36 `attach` overrides call `super.attach(owner)` first, so the hook is exact; of 48 `detach` overrides, 9 call `super.detach()` first (child walks) and 36 last (listener cleanup), and none of the pre-`super` code reads this node's `owner` or `attached`, so running it after `_owner = null` is equivalent.
  Affect: put what Dart writes after `super.attach(owner)` in `did_attach`; implement `visit_children` only for the child walk. Never call `attach` on a typed handle: use `as_object().attach(app, owner)`.

- Change: a panic in `performLayout` / `performResize` unwinds.
  Reason: language — no `FlutterError.reportError` isolate hook (same as gesture `invokeCallback`).
  Affect: layout does not catch and continue.

- Change: `ParentData` is a trait; storage is `Option<Box<dyn ParentData>>`. Typed access is `parent_data_of::<P>` / `parent_data_of_mut::<P>` / `parent_data_is::<P>`. Flutter's `ParentData()` is `EmptyParentData`.
  Reason: language — Rust has no subclassed `ParentData` you can `as`.
  Affect: write `child.parent_data_of::<BoxParentData>(app)` where Dart writes `child.parentData! as BoxParentData`.

## pipeline_owner.rs → object.dart (PipelineOwner)

- Change: `PipelineOwner` is a Handle newtype; `onNeedVisualUpdate` is `Option<Listener>`.
  Reason: language — no GC object, and `VoidCallback` must receive `App`.
  Affect: `PipelineOwner::new(app, on_need_visual_update)`.

## object.rs → object.dart (layout / child mixin)

- Change: `AnyRenderBox::layout` takes `BoxConstraints`; `AnyRenderSliver::layout` takes `SliverConstraints`. Each protocol stores its own constraints on its mixin data.
  Reason: language — Rust has no abstract `Constraints` object identity on an erased node.
  Affect: call `layout` on the protocol handle; there is no unified constraints enum.

- Change: `RenderObjectWithChildMixin` child access is `this.child(app)` / `this.set_child(app, child)` over a `RenderObjectWithChildData<AnyRenderBox>` field.
  Reason: language — mixin fields are not inherited; the handle is the receiver.
  Affect: write `this.child(app)` where Dart writes `child`. `BoxParentData` is installed by `RenderBox::setup_parent_data`.

## sliver.rs → sliver.dart

- Change: `as_box_constraints` takes `(min_extent, max_extent, cross_axis_extent)` instead of Dart named optionals. Defaults are `0`, infinity, and this sliver's cross extent when the `Option` is `None`.
  Reason: language — no named optional parameters.
  Affect: pass the three arguments; `None` for cross uses the constraint's cross extent.

## proxy_box.rs → proxy_box.dart / shifted_box.rs → shifted_box.dart

- Change: `RenderConstrainedBox` and `RenderPadding` are leaf structs; `RenderProxyBox` / `RenderShiftedBox` as types are not here.
  Reason: language — no inheritance; the accepted authoring shape is the leaf struct plus traits.
  Affect: construct with `RenderPadding::new(app, …)` / `RenderConstrainedBox::new(app, …)`.

## Deferred

- `PaintingContext` / layers / `flushPaint` / compositing bits. Trigger: paint.
- `SemanticsBinding` / `markNeedsSemanticsUpdate` / semantics callbacks on `PipelineOwner`. Trigger: a11y; do not stub.
- `PipelineManifold`. Trigger: `RendererBinding` attaching the root owner.
- `computeDryLayout` / `_DebugSize` / `BoxHitTestResult` / `globalToLocal`. Trigger: `RenderBox` public extras.
- `RenderProxyBox` / `RenderShiftedBox` paint, hit-test, and intrinsics. Trigger: `PaintingContext` / `BoxHitTestResult`.
- `invokeLayoutCallback`. Trigger: `LayoutBuilder`. Also widen `_layoutWithoutResize` so a non-boundary `RenderObjectWithLayoutCallbackMixin` can flush (Flutter's `this is RenderObjectWithLayoutCallbackMixin` arm).
- `layout` / `markNeedsLayout` / `constraints` as ops-table override points. Trigger: OverlayPortal (`layout` + `debugLayoutParent`), `RenderView` (constraints getter; not a box), first `markNeedsLayout` override (`RenderBox` intrinsics, `RenderParagraph`, OverlayPortal). Ask before adding; do not invent a parallel flag on `T`.
- OverlayPortal / `_RenderDeferredLayoutBox`. Trigger: `layout` on the ops table.
- `RenderView`. Trigger: `constraints` getter override + paint (`compositeFrame`). A box can stay `PipelineOwner.root_node` until then.
- `RenderParagraph` / `RenderEditable`. Trigger: `TextPainter`, container `ParentData`, paint, hit-test — not a Handle failure.
- Viewport / `ViewportOffset` / sliver-to-box adapters / sliver `ParentData`. Trigger: first viewport.
- `SliverConstraints.debugAssertIsValid` extra numeric checks. Trigger: a caller that relies on those messages.
