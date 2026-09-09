# reveal-painting/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/painting
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- colors.rs → colors.dart (`HSVColor`, `HSLColor`)
- box_fit.rs → box_fit.dart
- geometry.rs → geometry.dart (`positionDependentBox`)
- beveled_rectangle_border.rs → beveled_rectangle_border.dart
- continuous_rectangle_border.rs → continuous_rectangle_border.dart
- paint_utilities.rs → paint_utilities.dart (`paintZigZag`)
- basic_types.rs → basic_types.dart (the dart:ui re-exports)
- text_painter.rs → text_painter.dart (`kDefaultFontSize`, `TextOverflow`)
- matrix_utils.rs → matrix_utils.dart
- alignment.rs → alignment.dart
- border_radius.rs → border_radius.dart
- edge_insets.rs → edge_insets.dart
- fractional_offset.rs → fractional_offset.dart
- text_scaler.rs → text_scaler.dart
- box_border.rs → box_border.dart
- circle_border.rs → circle_border.dart
- oval_border.rs → oval_border.dart
- rounded_rectangle_border.rs → rounded_rectangle_border.dart
- stadium_border.rs → stadium_border.dart
- inline_span.rs → inline_span.dart
- text_span.rs → text_span.dart

## colors.rs → colors.dart (+ the `Color` subclass mechanism)

- Change: Flutter's `Color` subclasses (`CupertinoDynamicColor`, `WidgetStateColor`, `ColorSwatch`) are `ColorExtension` payloads that `AnyColor` carries beside the plain `Color`; `AnyColor` derefs to `Color` and `extension::<T>()` is Dart's `is T`.
  Reason: language — dart:ui `Color` is a `Copy` value here and cannot be subclassed.
  Affect: only the fields the framework resolves or type-checks hold `AnyColor` — text and decoration colours, icon theme, theme data; a colour that reaches a paint-only field (border sides, shadows, gradient stops, `Paint`) has dropped its payload and no longer resolves or compares as its subclass.

## draw.rs

- Change: `draw_rrect` / `draw_drrect` / `draw_oval` / `draw_rsuperellipse` are free functions that reduce the dart:ui `Canvas` methods to an rrect or a path.
  Reason: platform — valo has no `drawDRRect`, `drawOval` or `drawRSuperellipse` primitive.
  Affect: a display-list dump shows an rrect or a path where Flutter's shows the shape.

## clip.rs → clip.dart

- Change: `doAntiAlias` is ignored; the clip methods take a `ClipOp`, not an AA flag.
  Reason: platform — valo clips are anti-aliased.
  Affect: `Clip::HardEdge` and `Clip::AntiAlias` record the same clip, so a hard-edge clip has smooth edges.

- Change: clipping and hit-testing pass `FillRule::NonZero` alongside the path.
  Reason: platform — valo `Path` carries no `fillType`, and Dart's default is `nonZero`.
  Affect: a path with self-crossing subpaths clips and hit-tests as non-zero; there is no even-odd option.

- Change: `clip_r_superellipse_and_paint` records a `clip_path` of valo's superellipse path.
  Reason: platform — valo has no `clip_rsuperellipse` op; the path is the same Impeller geometry as `RSuperellipse.contains`.
  Affect: a display-list dump shows a path clip.

## box_shadow.rs → box_shadow.dart

- Change: `BoxShadow` is a separate struct from `Shadow` (Dart subclasses it), converted with `From`.
  Reason: language — Rust has no inheritance.
  Affect: a `BoxShadow` stored where Flutter stores it as a `Shadow` loses `spread_radius` and `blur_style`, so it paints tighter than Flutter's.

## image_provider.rs → image_provider.dart (`ImageConfiguration`) / services `asset_bundle.dart`

- Change: `AssetBundle` is one synchronous `load`; there is no `ImageProvider`, `ImageStream` or `ImageCache`.
  Reason: platform — a host or test can implement this completely, where Flutter's Future plus codec/cache pipeline is the engine talking to Dart.
  Affect: an asset resolves within the frame that asks for it, and nothing is cached or shared between two loads of the same key.

## box_decoration.rs → box_decoration.dart

- Change: `BoxDecoration` and `ShapeDecoration` have no `image` or `gradient` field.
  Reason: platform — `DecorationImage` and `Gradient.createShader` are deferred (valo shaders).
  Affect: a decoration paints a solid colour, border, radius and shadow only; `background_blend_mode` still needs a `color`.

- Change: Dart's `assert(backgroundBlendMode == null || color != null)` runs when `background_blend_mode` is set, not once at the end of the constructor.
  Reason: language — a builder checks each field as it arrives.
  Affect: setting `background_blend_mode` before `color` trips the debug assert; the other order is fine.

## binding.rs → binding.dart

- Change: `PaintingBinding` holds the one `FontCollection` and hands it to `TextPainter`'s shaping methods, rather than the engine holding a process-wide font manager every paragraph shapes against implicitly; the shell installs it from `Platform::font_source` and `register_font` adds one face.
  Reason: platform — valo has no default font manager, so the collection is a value the framework owns and passes.
  Affect: a paragraph built before fonts are installed panics asking for them, and unspecified-family text shapes against the platform UI font.

## text_painter.rs → text_painter.dart

- Change: the statics `compute_width` / `compute_max_intrinsic_width` take only text, direction and width bounds; Dart's other measuring options are dropped.
  Reason: language — no optional named parameters.
  Affect: measuring with a scaler, `max_lines` or the other options means building a `TextPainter` and laying it out.

## Deferred

- `debug.dart` beyond `debugDisableShadows`. Trigger: image loading, or tests that need it.
- `ColorProperty`; `ColorSwatch` as a `const` table. Trigger: diagnostics; Material's `Colors`.
- Custom `TextScaler` / `SystemTextScaler`. Trigger: `MediaQuery`.
- `EdgeInsets.fromViewPadding` / `fromWindowPadding`. Trigger: embedder / `MediaQuery`.
- `_MixedBorderRadius` and cross-kind `add` / `subtract` / `lerp`. Trigger: first consumer that adds a `BorderRadius` to a `BorderRadiusDirectional`.
- `hashCode` / `Hash`. Trigger: the first map or set keyed by insets, alignment, border radius, `TextScaler` or `TextStyle`.
- Diagnostics / `debugCheckCanResolveTextDirection`. Trigger: porting diagnostics; a missing `TextDirection` on resolve panics until then.
- `DecorationImage` and the decoration `image` fields. Trigger: first decoration that paints an image.
- `ui.Locale`, `ImageConfiguration.locale`, `TextStyle.locale`. Trigger: locale-specific assets or region-specific glyphs.
- `Gradient`, the decoration `gradient` fields, `createShader`. Trigger: first decoration that paints a gradient; `TileMode.decal` and sweep `endAngle` have no valo counterpart.
- `AssetBundle.loadString` / `loadStructuredData` / `loadBuffer` / caching, `NetworkAssetBundle`, `rootBundle`. Trigger: string or structured assets, or a default bundle on `App`.
- `StarBorder`. Trigger: a star or polygon `ShapeBorder`.
- `NotchedShape` / `CircularNotchedRectangle` / `AutomaticNotchedShape`. Trigger: `BottomAppBar`; valo has no `Path.arcToPoint` or `Path.combine`.
- The rest of `PaintingBinding`. Trigger: image cache / shader warm-up.
- `strut_style.dart` and `TextPainter.strutStyle`. Trigger: a paragraph that sets strut; the host has no strut.
- `TextPainter.locale` / `TextSpan.locale` / `TextSpan.spellOut`. Trigger: `Locale`.
- `PlaceholderSpan` / `WidgetSpan`. Trigger: `WidgetSpan`; the host has no placeholders.
- `TextSpan.recognizer` and `TextSpan` as a `HitTestTarget`. Trigger: `RichText` with a tappable span.
- `TextSpan.mouseCursor` / `onEnter` / `onExit`. Trigger: a hoverable span; the mouse tracker keys annotations by render object.
- `TextSpan` semantics and `computeSemanticsInformation`. Trigger: accessibility; do not stub.
- `TextPainter.wordBoundaries`: `WordBoundary` reads a laid-out paragraph, which lives in the render object here, so it sits with `RenderEditable` in rendering. Trigger: a word boundary wanted without a render object.
