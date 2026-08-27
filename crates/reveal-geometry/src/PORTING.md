# reveal-geometry/src
Flutter home: engine/src/flutter/lib/ui
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- math.rs → math.dart
- lerp.rs → lerp.dart

## geometry.rs → geometry.dart

- Change: Dart's `<`/`<=`/`>`/`>=` on `Offset` and `Size` are the methods `lt`/`le`/`gt`/`ge`. They are not [`PartialOrd`].
  Reason: `PartialOrd` defines `<=` from one `partial_cmp`. Flutter's is component-wise and independent.
  Affect: write `a.le(&b)`, not `a <= b`. `Offset(1, 2) <= Offset(1, 3)` would be the wrong answer: Flutter's `<=` is true while `<` and `==` are both false.

## Deferred

- `Radius`, `RRect`, `RSuperellipse`, `RSTransform`. Trigger: painting borders/clips / transforms.
- `Size.copy`. Trigger: `box.dart`'s `_DebugSize` hack.
- `hashCode` / [`Hash`]. Trigger: the first map or set keyed by `Offset`, `Size`, or `Rect`.
- `_lerpInt`. Trigger: `FontWeight.lerp`.
