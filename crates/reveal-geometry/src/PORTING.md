# reveal-geometry/src
Flutter home: engine/src/flutter/lib/ui
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- math.rs → math.dart
- lerp.rs → lerp.dart
- color.rs → painting.dart (Color, ColorSpace)
- rrect.rs → geometry.dart (`RRect`, `RSuperellipse` except `contains`)

## geometry.rs → geometry.dart

- Change: Dart's `<`/`<=`/`>`/`>=` on `Offset` and `Size` are the methods `lt`/`le`/`gt`/`ge`. They are not [`PartialOrd`].
  Reason: language — `PartialOrd` defines `<=` from one `partial_cmp`. Flutter's is component-wise and independent.
  Affect: write `a.le(&b)`, not `a <= b`. `Offset(1, 2) <= Offset(1, 3)` would be the wrong answer: Flutter's `<=` is true while `<` and `==` are both false.

## Deferred

- `RSuperellipse.contains`. Trigger: Impeller hit-test / a Path. Dart's method is engine FFI, not the `RRect` ellipse test.
- `RSTransform`. Trigger: `Canvas.drawAtlas`.
- `Size.copy`. Trigger: `box.dart`'s `_DebugSize` hack.
- `hashCode` / [`Hash`]. Trigger: the first map or set keyed by `Offset`, `Size`, `Rect`, `Radius`, or `Color`.
- Subclassing `Color` / overriding `value`. Trigger: `CupertinoDynamicColor`.
- `_lerpInt`. Trigger: `FontWeight.lerp`.
