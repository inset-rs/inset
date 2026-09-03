# reveal-rendering/src
Flutter home: packages/flutter/lib/src/rendering
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- box.rs → BoxConstraints
- proxy_box.rs → HitTestBehavior
- object.rs → Constraints (`isTight` / `isNormalized` / `debugAssertIsValid`)

## debug.rs → debug.dart

- Change: `debug_*` / `set_debug_*` instead of assigning a library `bool`.
  Reason: language — Rust has no isolate-global assignable `bool` binding.
  Affect: call the setter; the getter is the Dart read.

## Deferred

- `RenderObject` / `PipelineOwner` / `PaintingContext`. Trigger: a Handle tree that can re-enter during layout; skip `SemanticsBinding`.
- `RenderBox` / `BoxHitTestResult` / `_DebugSize`. Trigger: `RenderObject`.
- `debugCurrentRepaintColor` / `debugOnProfilePaint`. Trigger: `RenderView.compositeFrame` / inspector.
- Layers (`OpacityLayer`, …). Trigger: `PaintingContext`.
