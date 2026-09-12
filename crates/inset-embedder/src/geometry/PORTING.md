# inset-embedder/src/geometry
Flutter home: engine/src/flutter/lib/ui
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- math.rs → math.dart
- lerp.rs → lerp.dart
- color.rs → painting.dart (Color, ColorSpace)
- clip.rs → painting.dart (`Clip`)
- rrect.rs → geometry.dart (`RRect`, `RSuperellipse`)
- geometry.rs → geometry.dart
- shadow.rs → painting.dart (`Shadow`)
- matrix.rs → `Matrix4` / `Vector3` / `Quaternion` (package:vector_math)

## mod.rs → dart:ui `Matrix4` / paint-boundary `From`

- Change: `Matrix4` is valo's `Matrix` (glam f32, column-major) where Flutter's `vector_math` `Matrix4` is f64.
  Reason: platform — valo paints in f32, matching what Flutter hands the engine.
  Affect: construct with `Matrix4::translation` / `scale` / `rotation` (f32 arguments); `transform3` still returns an f64 `Offset`.

- Change: `From` into valo's `Color` / `Rect` / `Point` narrows f64 to f32 and drops `ColorSpace`.
  Reason: platform — the paint backend is f32 sRGB.
  Affect: write `rect.into()` / `color.into()` at a `Canvas` call.

## Deferred

- `RSTransform`. Trigger: `Canvas.drawAtlas`.
- `Size.copy`. Trigger: `box.dart`'s `_DebugSize` hack.
- `hashCode` / `Hash`. Trigger: the first map or set keyed by `Offset`, `Size`, `Rect`, `Radius`, or `Color`.
- Subclassing `Color` / overriding `value`. Trigger: `CupertinoDynamicColor`.
