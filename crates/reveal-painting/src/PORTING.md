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

- Change: Flutter's `Color` subclasses (`CupertinoDynamicColor`, `WidgetStateColor`, `ColorSwatch`) are extensions carried by `AnyColor`: the plain `Color` the subclass passed to `super` plus a `ColorExtension` payload that answers Dart's `is` and `==`. `AnyColor` derefs to `Color`, so inherited methods work and return a plain `Color`, and `extension::<T>()` is Dart's `is T`.
  Reason: language — dart:ui `Color` is a Copy value here and cannot be subclassed; the engine only ever reads the value, and the framework does the `is` checks.
  Affect: fields Flutter code may resolve or type-check hold `AnyColor` — text and decoration colours, icon theme and theme data; setters take `impl Into<AnyColor>`, so `.color(Color::RED)` is unchanged, and a read compares through `.color()` or `Some(color.into())`. Paint-only fields (border sides, shadows, gradient stops, `Paint`) stay `Color`. `ColorSwatch<T>` is one such extension, read back with `extension::<ColorSwatch<i32>>()`.

## draw.rs

- Change: `draw_rrect` / `draw_drrect` / `draw_oval` / `draw_rsuperellipse` are free functions standing in for the dart:ui `Canvas` methods: `drawDRRect` is one even-odd path, an oval is an elliptical rrect, and a superellipse is a path.
  Reason: platform — valo has no `drawDRRect`, `drawOval` or `drawRSuperellipse` primitive.
  Affect: a display-list dump shows an rrect or a path.

## clip.rs → clip.dart

- Change: `ClipContext` is a trait whose `canvas()` is a short `&mut Canvas` borrow, and the painter is `impl FnOnce(&mut Self)` instead of a `VoidCallback` that closes over the context.
  Reason: language — Rust cannot hold `&mut Canvas` across a callback that also needs `&mut Self`.
  Affect: write `|ctx| { .. }` and use `ctx.canvas()` inside; do not close over `self`.

- Change: `doAntiAlias` is ignored; the clip methods take a `ClipOp`, not an AA flag.
  Reason: platform — valo clips are anti-aliased.
  Affect: `Clip::HardEdge` and `Clip::AntiAlias` record the same clip.

- Change: clipping and hit-testing use `FillRule::NonZero`.
  Reason: platform — valo `Path` carries no `fillType`; the rule is passed separately, and Dart's default is `nonZero`.
  Affect: an even-odd clip or hit test needs a fill-rule argument later.

- Change: `clip_r_superellipse_and_paint` records a `clip_path` of valo's superellipse path, not `Canvas.clipRSuperellipse`.
  Reason: platform — valo 0.3.0 has no dedicated `clip_rsuperellipse` op; the path is the same Impeller geometry as `RSuperellipse.contains`.
  Affect: callers still pass an `RSuperellipse`; a display-list dump shows a path clip.

## box_shadow.rs → box_shadow.dart

- Change: `BoxShadow` (Dart: `extends ui.Shadow`) is a separate struct; convert with `From`.
  Reason: language — Rust has no inheritance.
  Affect: store `Shadow::from(box_shadow)` where Dart stores a `BoxShadow` as a `Shadow` (drops `spreadRadius` / `blurStyle`).

## borders.rs → borders.dart

- Change: `ShapeBorder` / `OutlinedBorder` are traits, a stored border is `Box<dyn ShapeBorder>`, and `clone_box` / `clone_outlined` stand in for Dart's shared immutable instances.
  Reason: language — the subclass set is open (`InputBorder`, tests, apps), so a pairing enum cannot hold it, and `dyn Trait` is not `Copy`.
  Affect: store `Box<dyn ShapeBorder>`; `is` / `as` is `as_any().downcast_ref`.

- Change: `OutlinedBorder` inherits no `side` field and no `dimensions` body; each concrete type holds `pub side: BorderSide` and calls `outlined_border_dimensions`.
  Reason: language — a Rust trait cannot hold a field, so it cannot default a body that reads one.
  Affect: outlined implementors write `dimensions` as `outlined_border_dimensions(self.side)`.

## image_provider.rs → image_provider.dart (`ImageConfiguration`) / services `asset_bundle.dart`

- Change: `AssetBundle` is one `load` returning `Result<Vec<u8>, String>`; there is no `ImageProvider`, `ImageStream` or `ImageCache`.
  Reason: platform — a host or test can implement this completely; Flutter's Future plus codec/cache pipeline is the engine talking to Dart.
  Affect: `bundle.load(key)` is synchronous.

## decoration.rs → decoration.dart

- Change: `Decoration` / `BoxPainter` are traits, a stored decoration is `Box<dyn Decoration>`, and `clone_box` stands in for Dart's shared immutable instances.
  Reason: language — the subclass set is open, so a pairing enum cannot hold it.
  Affect: store `Box<dyn Decoration>`; `is` / `as` is `as_any().downcast_ref`.

## box_decoration.rs → box_decoration.dart

- Change: `BoxDecoration` and `ShapeDecoration` have no `image` or `gradient` field; the background is a solid `color` only.
  Reason: platform — `DecorationImage` / `ImageProvider` and `Gradient.createShader` (valo shaders) are deferred, and colour, border, radius and shadow do not need them.
  Affect: there is no `decoration.image` or `decoration.gradient`; a `background_blend_mode` still requires a `color`.

- Change: Dart's `assert(backgroundBlendMode == null || color != null)` runs when `background_blend_mode` is set, not once at the end of a constructor.
  Reason: language — a builder checks each field as it arrives.
  Affect: set `color` before `background_blend_mode`; the other order trips the debug assert.

## text_style.rs → text_style.dart

- Change: `TextStyle` has no `locale` field.
  Reason: platform — valo shapes text without a locale, so dart:ui's `TextStyle.locale` has nothing to feed.
  Affect: there is no `style.locale`.

## binding.rs → binding.dart

- Change: `PaintingBinding` holds the app-wide `FontCollection` — installed once by the shell from `Platform::font_source` or by a test from bundled fonts, extended one face at a time by `register_font` (what loading a `pubspec.yaml` font asset does) — and `TextPainter`'s shaping methods take that collection as `&mut FontCollection`.
  Reason: platform — Flutter's engine owns one font manager per process that every `ui.Paragraph` shapes against implicitly; here the collection is a value the framework owns and passes.
  Affect: `painter.layout(app.get_mut(fonts), min, max)` where Dart writes `painter.layout(minWidth:, maxWidth:)`; a paragraph built before fonts are installed panics with the message to install them.

## text_painter.rs → text_painter.dart

- Change: the statics `compute_width` / `compute_max_intrinsic_width` take only the text, direction and width bounds; Dart's other measuring options are dropped.
  Reason: language — no optional named parameters.
  Affect: to measure with a scaler, `max_lines` or the other options, build a `TextPainter` and lay it out.

## Deferred

- `debug.dart` remainder (`debugNetworkImageHttpClientProvider`, …). Trigger: image loading / tests that are not `debugDisableShadows`.
- `ColorProperty` (diagnostics); `ColorSwatch` as a `const` table (`HashMap` is not const; a static swatch table needs a slice-backed form). Trigger: diagnostics; Material's `Colors`.
- Custom `TextScaler` / `SystemTextScaler`. Trigger: `MediaQuery`; `_ClampedTextScaler` is the `TextScaler::Clamped` arm, created by the default `clamp` on a non-linear scaler.
- `EdgeInsets.fromViewPadding` / `fromWindowPadding` / `EdgeInsetsGeometry.fromViewPadding`. Trigger: embedder / `MediaQuery`.
- `_MixedBorderRadius` and cross-kind `add` / `subtract` / `BorderRadiusGeometry.lerp`. Trigger: first consumer that adds a `BorderRadius` to a `BorderRadiusDirectional`; same-kind `add` / `subtract` / `lerp` exist.
- `hashCode` / `Hash`. Trigger: the first map or set keyed by insets, alignment, border radius, `TextScaler`, or `TextStyle`.
- Diagnostics / `debugCheckCanResolveTextDirection`. Trigger: porting diagnostics; until then a missing `TextDirection` on resolve panics with the FlutterError summary string.
- `DecorationImage` / `BoxDecoration.image` / `ShapeDecoration.image`. Trigger: first decoration that paints an image.
- `ui.Locale` / `ImageConfiguration.locale` / `TextStyle.locale`. Trigger: locale-specific assets or region-specific glyphs.
- `Gradient` / `BoxDecoration.gradient` / `ShapeDecoration.gradient` / `createShader`. Trigger: first decoration that paints a gradient; valo covers linear, radial (with optional focus) and a full-turn sweep, while `TileMode.decal` and sweep `endAngle` have no valo counterpart.
- `AssetBundle.loadString` / `loadStructuredData` / `loadBuffer` / caching / `NetworkAssetBundle` / `rootBundle`. Trigger: string or structured assets, or a default bundle on `App`.
- `StarBorder`. Trigger: a star or polygon `ShapeBorder`; path verbs are mechanical (`conicTo` exists on valo), the lerp to `CircleBorder` / `StadiumBorder` / `RoundedRectangleBorder` is large.
- `NotchedShape` / `CircularNotchedRectangle` / `AutomaticNotchedShape`. Trigger: `BottomAppBar`; valo has no `Path.arcToPoint` and no `Path.combine`.
- The rest of `PaintingBinding`. Trigger: image cache / shader warm-up.
- `strut_style.dart` / `getParagraphStyle(strutStyle:)` / `TextPainter.strutStyle`. Trigger: a paragraph that sets strut; the host has no strut, so `get_full_height_for_caret` uses the glyph's or the layout template's height.
- `TextPainter.locale` / `TextSpan.locale` / `TextSpan.spellOut`. Trigger: `Locale`.
- `PlaceholderSpan` / `WidgetSpan`. Trigger: `WidgetSpan`; the host has no placeholders.
- `TextSpan.recognizer`, `TextSpan` as a `HitTestTarget`. Trigger: `RichText` with a tappable span; needs an erased `GestureRecognizer`.
- `TextSpan.mouseCursor` / `onEnter` / `onExit` (the span as a `MouseTrackerAnnotation`). Trigger: a hoverable span; the mouse tracker keys annotations by render object.
- `TextSpan.semanticsLabel` / `semanticsIdentifier`, `InlineSpanSemanticsInformation`, `computeSemanticsInformation`. Trigger: accessibility; do not stub.
- `WordBoundary` / `TextPainter.wordBoundaries`. Trigger: text editing; needs services `TextBoundary`.
