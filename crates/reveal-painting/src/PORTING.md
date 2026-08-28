# reveal-painting/src
Flutter home: packages/flutter/lib/src/painting
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## basic_types.rs → basic_types.dart

- Change: `TextDirection` (`dart:ui`) is defined in this module until dart:ui value types move into `reveal-embedder`.
  Reason: painting is where Flutter code imports it from (`basic_types.dart` re-exports it).
  Affect: import `TextDirection` from `reveal-painting`, not `reveal-embedder`.

## alignment.rs → alignment.dart

- Change: `AlignmentGeometry` (abstract, two public subclasses) is a `#[non_exhaustive]` pairing enum over `Alignment` and `AlignmentDirectional`.
  Reason: language — Rust has no inheritance for a stored abstract type whose kind is the two public subclasses.
  Affect: store `AlignmentGeometry::Alignment` / `::Directional` (or convert with `From`). Cross-kind `==` compares `(_x, _start, _y)` with no kind check, as Dart's base class does.

## border_radius.rs → border_radius.dart

- Change: `BorderRadiusGeometry` (abstract, two public subclasses) is a `#[non_exhaustive]` pairing enum over `BorderRadius` and `BorderRadiusDirectional`.
  Reason: language — Rust has no inheritance for a stored abstract type whose kind is the two public subclasses.
  Affect: store `BorderRadiusGeometry::BorderRadius` / `::Directional` (or convert with `From`). Dart's `==` checks `runtimeType`, so zero visual and zero directional stay unequal.

## edge_insets.rs → edge_insets.dart

- Change: `EdgeInsetsGeometry` (abstract, two public subclasses) is a `#[non_exhaustive]` pairing enum over `EdgeInsets` and `EdgeInsetsDirectional`.
  Reason: language — Rust has no inheritance for a stored abstract type whose kind is the two public subclasses.
  Affect: store `EdgeInsetsGeometry::Insets` / `::Directional` (or convert with `From`). Cross-kind `==` compares the six components with no kind check, as Dart's base class does: zero-horizontal visual and directional values compare equal.

## Deferred

- `EdgeInsets.fromViewPadding` / `fromWindowPadding` / `EdgeInsetsGeometry.fromViewPadding`. Trigger: embedder / `MediaQuery`.
- `_MixedEdgeInsets` and cross-kind `add` / `subtract` / `flipped` / `infinity` / `clamp` / `EdgeInsetsGeometry.lerp`. Trigger: first consumer that adds an `EdgeInsets` to an `EdgeInsetsDirectional`.
- `EdgeInsets.inflateRRect` / `deflateRRect`. Trigger: `RRect`.
- `_MixedAlignment` and cross-kind `AlignmentGeometry.add` / `AlignmentGeometry.lerp`. Trigger: first consumer that adds an `Alignment` to an `AlignmentDirectional`.
- `BorderRadius.toRRect` / `toRSuperellipse`. Trigger: `RRect` / `RSuperellipse`.
- `_MixedBorderRadius` and cross-kind `add` / `subtract` / `BorderRadiusGeometry.lerp`. Trigger: first consumer that adds a `BorderRadius` to a `BorderRadiusDirectional`.
- `hashCode` / [`Hash`]. Trigger: the first map or set keyed by insets, alignment, or border radius.
- Diagnostics / `debugCheckCanResolveTextDirection`. Trigger: porting diagnostics; until then a missing `TextDirection` on resolve panics with the FlutterError summary string.
