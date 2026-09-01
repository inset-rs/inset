# reveal-embedder/src/geometry
Flutter home: engine/src/flutter/lib/ui
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- math.rs → math.dart
- lerp.rs → lerp.dart
- color.rs → painting.dart (Color, ColorSpace)
- clip.rs → painting.dart (`Clip`)

## geometry.rs → geometry.dart

- Change: Dart's `<`/`<=`/`>`/`>=` on `Offset` and `Size` are the methods `lt`/`le`/`gt`/`ge`. They are not [`PartialOrd`].
  Reason: language — `PartialOrd` defines `<=` from one `partial_cmp`. Flutter's is component-wise and independent.
  Affect: write `a.le(&b)`, not `a <= b`. `Offset(1, 2) <= Offset(1, 3)` would be the wrong answer: Flutter's `<=` is true while `<` and `==` are both false.

## rrect.rs → geometry.dart (`RRect`, `RSuperellipse`)

- Change: `RSuperellipse.contains` uses valo `RoundSuperellipse::contains` (Impeller param math).
  Reason: platform — Dart's method is engine FFI; valo is the host geometry.
  Affect: none for the inside/outside answer.

## shadow.rs → painting.dart (`Shadow`)

- Change: `to_paint` sets `mask_blur: Some(MaskBlur)` instead of Dart `MaskFilter.blur`.
  Reason: platform — valo `Paint` has `mask_blur`, not `maskFilter`.
  Affect: read `paint.mask_blur`.

## mod.rs → dart:ui `Matrix4` / paint-boundary `From`

- Change: `Matrix4` is valo `Matrix` (glam f32, column-major). Flutter `vector_math` `Matrix4` is f64.
  Reason: platform — valo paints in f32, matching what Flutter hands the engine.
  Affect: construct with `Matrix4::translation` / `scale` / `rotation` (f32 args). `transform3` still returns f64 `Offset`.

- Change: `From` into valo `Color` / `Rect` / `Point` narrows f64 to f32 and drops `ColorSpace` (valo is sRGB).
  Reason: platform — the paint backend is f32 sRGB.
  Affect: write `rect.into()` / `color.into()` at a `Canvas` call.

## Deferred

- `RSTransform`. Trigger: `Canvas.drawAtlas`.
- `Size.copy`. Trigger: `box.dart`'s `_DebugSize` hack.
- `hashCode` / [`Hash`]. Trigger: the first map or set keyed by `Offset`, `Size`, `Rect`, `Radius`, or `Color`.
- Subclassing `Color` / overriding `value`. Trigger: `CupertinoDynamicColor`.
- `_lerpInt`. Trigger: `FontWeight.lerp`.
