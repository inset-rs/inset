# reveal-painting/src
Flutter home: packages/flutter/lib/src/painting
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- colors.rs → colors.dart (`HSVColor`, `HSLColor`)
- box_fit.rs → box_fit.dart
- geometry.rs → geometry.dart (`positionDependentBox`)

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

## fractional_offset.rs → fractional_offset.dart

- Change: `FractionalOffset` (Dart: `extends Alignment`) is a separate struct. Convert with `From` / `Alignment::from`. `==` with `Alignment` compares `x` and `y`. `+` / `-` with an `Alignment` return `Alignment`, matching Dart's `super ± other`.
  Reason: language — Rust has no inheritance.
  Affect: store `Alignment::from(offset)` (or `offset.into()`) where Dart stores a `FractionalOffset` as an `Alignment`.

## text_scaler.rs → text_scaler.dart

- Change: `TextScaler` (abstract, two private implementors in this file) is a `#[non_exhaustive]` pairing enum over `_LinearTextScaler` and `_ClampedTextScaler`.
  Reason: language — Rust has no inheritance for a stored abstract type.
  Affect: store `TextScaler::Linear` / `::Clamped`. `clamp` takes `min_scale_factor` and `max_scale_factor` (Dart defaults `0` and `infinity`).

## Deferred

- `ColorSwatch` / `ColorProperty`. Trigger: Material colors / diagnostics.
- Custom `TextScaler` / `SystemTextScaler`. Trigger: `MediaQuery`; `_ClampedTextScaler` is the `TextScaler::Clamped` arm, created by the default `clamp` on a non-linear scaler.
- `EdgeInsets.fromViewPadding` / `fromWindowPadding` / `EdgeInsetsGeometry.fromViewPadding`. Trigger: embedder / `MediaQuery`.
- `_MixedEdgeInsets` and cross-kind `add` / `subtract` / `flipped` / `infinity` / `clamp` / `EdgeInsetsGeometry.lerp`. Trigger: first consumer that adds an `EdgeInsets` to an `EdgeInsetsDirectional`.
- `_MixedAlignment` and cross-kind `AlignmentGeometry.add` / `AlignmentGeometry.lerp`. Trigger: first consumer that adds an `Alignment` to an `AlignmentDirectional`.
- `_MixedBorderRadius` and cross-kind `add` / `subtract` / `BorderRadiusGeometry.lerp`. Trigger: first consumer that adds a `BorderRadius` to a `BorderRadiusDirectional`.
- `hashCode` / [`Hash`]. Trigger: the first map or set keyed by insets, alignment, border radius, or `TextScaler`.
- Diagnostics / `debugCheckCanResolveTextDirection`. Trigger: porting diagnostics; until then a missing `TextDirection` on resolve panics with the FlutterError summary string.
