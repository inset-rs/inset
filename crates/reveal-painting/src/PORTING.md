# reveal-painting/src
Flutter home: packages/flutter/lib/src/painting
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- colors.rs → colors.dart (`HSVColor`, `HSLColor`)
- box_fit.rs → box_fit.dart
- geometry.rs → geometry.dart (`positionDependentBox`)
- beveled_rectangle_border.rs → beveled_rectangle_border.dart
- continuous_rectangle_border.rs → continuous_rectangle_border.dart
- paint_utilities.rs → paint_utilities.dart (`paintZigZag`)
- basic_types.rs dart:ui re-exports (`TextDirection`, `FontWeight`, `FontStyle`, `TextAlign`, `TextBaseline`, `TextDecoration`, `TextDecorationStyle`, `TextLeadingDistribution`, `TextHeightBehavior`, `FontFeature`, `FontVariation`)
- text_painter.rs → text_painter.dart (`kDefaultFontSize`, `TextOverflow`)

## colors.rs → colors.dart (+ the `Color` subclass mechanism)

- Change: Flutter's `Color` subclasses (`CupertinoDynamicColor`, `WidgetStateColor`, `ColorSwatch`) are extensions carried by `AnyColor { color: Color, extension }`: `color` is the value the subclass passed to `super`, `extension` the subclass (`ColorExtension`: `as_any` for Dart's `is`, `eq_extension` for its `==`). `AnyColor` derefs to `Color`, so inherited `Color` methods work and return a plain `Color`; `extension::<T>()` is Dart's `is T`. A `const` table entry holds `&'static dyn ColorExtension`; a run-time one an `Rc`.
  Reason: language — dart:ui `Color` is a Copy value here and cannot be subclassed; the engine only ever reads the value, the framework does the `is` checks.
  Affect: painting and framework fields that Flutter code may resolve or type-check hold `AnyColor`: `TextStyle.color` / `background_color` / `decoration_color`, `BoxDecoration.color`, `ShapeDecoration.color`, widgets' `IconThemeData.color`, theme data. Setters take `impl Into<AnyColor>`, so `.color(Color::RED)` is unchanged; reads compare with `.map(AnyColor::color)` or `Some(color.into())`. Paint-only fields (`BorderSide.color`, `BoxShadow.color`, gradient stops, `Paint`) stay `Color`: no Flutter code resolves a subclass there, and `BorderSide` / `BoxShadow` stay Copy values.

- Change: `AnyColor::lerp` returns a plain `Option<Color>`; a subclass does not survive `lerp`, `copyWith`-style setters, or `merge` beyond being carried through untouched.
  Reason: identical to Dart's `Color.lerp`, which constructs a new `Color`.
  Affect: none.

- Change: `ColorSwatch<T>` is an extension built with `ColorSwatch::new(primary, HashMap)` and turned into the color it is in Dart with `into_any()`; `get(&key)` is `operator []`.
  Reason: language — see above.
  Affect: `MaterialColor` / `MaterialAccentColor` wrap it; `any.extension::<ColorSwatch<i32>>()`.

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

## matrix_utils.rs → matrix_utils.dart

- Change: `transform_rect` is the four-corner accumulation only — Flutter's translation/scale/affine fast paths are skipped.
  Reason: platform — `Matrix4` is valo's f32 matrix; the fast paths compute the same numbers.
  Affect: none for the mapped rect. Corner math is f64 over f32 storage.

## draw.rs

- Change: `draw_rrect` / `draw_drrect` / `draw_oval` / `draw_rsuperellipse` stand in for dart:ui `Canvas.drawRRect` / `Canvas.drawDRRect` / `Canvas.drawOval` / `Canvas.drawRSuperellipse`. `drawDRRect` is one even-odd path. Oval is an elliptical rrect. Superellipse is a path.
  Reason: platform — valo has no `drawDRRect`, `drawOval`, or `drawRSuperellipse` primitive.
  Affect: call `draw_rrect(canvas, rrect, paint)` instead of `canvas.drawRRect`. Same for the others.

## clip.rs → clip.dart

- Change: `ClipContext` is a trait. `canvas()` is a short `&mut Canvas` borrow. The painter is `impl FnOnce(&mut Self)` instead of a `VoidCallback` that closes over the context.
  Reason: language — Rust cannot hold `&mut Canvas` across a callback that also needs `&mut Self`.
  Affect: write `|ctx| { ... }` and use `ctx.canvas()` inside; do not close over `self`.

- Change: `doAntiAlias` is ignored. `clip_rect` / `clip_rrect_radii_elliptical` / `clip_path` take a `ClipOp`, not an AA flag.
  Reason: platform — valo clips are anti-aliased.
  Affect: `Clip::HardEdge` and `Clip::AntiAlias` record the same clip.

- Change: `clip_path_and_paint` takes `&Arc<Path>` and clips with `FillRule::NonZero`.
  Reason: platform — valo `Path` has no `fillType`; `clip_path` takes the rule separately. Dart's default `PathFillType` is `nonZero`.
  Affect: pass `&path_builder.build()`. Even-odd clip paths need a fill-rule argument later.

- Change: `clip_r_superellipse_and_paint` records `clip_path` of a valo `rsuperellipse_radii` path, not `Canvas.clipRSuperellipse`.
  Reason: platform — valo 0.3.0 has no dedicated `clip_rsuperellipse` op. The path is the same Impeller geometry as `RSuperellipse.contains`.
  Affect: a display-list dump shows a path clip. Callers still pass `RSuperellipse`.

## box_shadow.rs → box_shadow.dart

- Change: `BoxShadow` (Dart: `extends ui.Shadow`) is a separate struct. Convert with [`From`].
  Reason: language — Rust has no inheritance.
  Affect: store `Shadow::from(box_shadow)` where Dart stores a `BoxShadow` as a `Shadow` (drops `spreadRadius` / `blurStyle`).

- Change: `to_paint` sets `mask_blur: Some(MaskBlur)` instead of Dart `MaskFilter.blur`.
  Reason: platform — valo `Paint` has `mask_blur`, not `maskFilter`.
  Affect: read `paint.mask_blur`.

- Change: `debug_disable_shadows` / `set_debug_disable_shadows` instead of assigning a library `bool`.
  Reason: language — Rust has no isolate-global assignable `bool` binding.
  Affect: call the setter; the getter is the Dart read.

## borders.rs → borders.dart

- Change: `to_paint` uses `PaintStyle::Stroke(Stroke::new(width))` instead of Dart `style` + `strokeWidth` fields.
  Reason: platform — valo stroke width lives on `PaintStyle::Stroke`.
  Affect: inspect `paint.style`, not a separate `strokeWidth`.

- Change: `ShapeBorder` / `OutlinedBorder` are traits. A stored border is `Box<dyn ShapeBorder>`. `clone_box` / `clone_outlined` stand in for Dart's shared immutable instances.
  Reason: language — the subclass set is open (`InputBorder`, tests, apps); a pairing enum cannot hold it. `dyn Trait` is not `Copy`.
  Affect: store `Box<dyn ShapeBorder>`. `a + b` is `a.plus(&b)`. `ShapeBorder.lerp` is `<dyn ShapeBorder>::lerp`. `is` / `as` is `as_any().downcast_ref`.

- Change: `OutlinedBorder.side` is a method; each concrete type holds `pub side: BorderSide`. `dimensions` is [`outlined_border_dimensions`].
  Reason: language — a Rust trait cannot hold a field.
  Affect: write `border.side()` (or the struct field). Outlined implementors call `outlined_border_dimensions(self.side)`.

- Change: `get_outer_path` / `get_inner_path` return `Arc<Path>`. `hit_test` uses `FillRule::NonZero`.
  Reason: platform — valo `Path` is already `Arc`; `contains` takes the fill rule separately.
  Affect: the path is `Arc`. Even-odd hit tests need a fill-rule argument later.

## box_border.rs → box_border.dart

- Change: `BoxBorder` is a trait. A stored box border is `Box<dyn BoxBorder>`. Factories live on `impl dyn BoxBorder`.
  Reason: language — Flutter tests and apps subclass `BoxBorder`; a pairing enum cannot hold that set. `dyn Trait` is not `Copy`.
  Affect: store `Box<dyn BoxBorder>`. `BoxBorder.lerp` is `<dyn BoxBorder>::lerp`. `BoxBorder.fromLTRB` is `<dyn BoxBorder>::from_ltrb` (all four sides required). `BoxBorder.all` takes `color`, `width`, `style`, `stroke_align` (Dart has defaults).

- Change: `BoxBorder::paint` takes `shape` and `border_radius` as required arguments. [`ShapeBorder::paint`](ShapeBorder::paint) is the three-argument form and forwards with `BoxShape::Rectangle` and `None`.
  Reason: language — Rust has no optional named parameters on a trait method that also overrides a shorter paint.
  Affect: callers that pass a shape or radius use `BoxBorder::paint`. The three-argument call is `ShapeBorder::paint`.

- Change: `Border::all` / inherent `scale` take every argument (no Dart named defaults). `scale` on the struct returns `Border` / `BorderDirectional`; [`ShapeBorder::scale`](ShapeBorder::scale) boxes that.
  Reason: language — Rust has no named defaults and no covariant override return.
  Affect: write `Border::all(color, width, style, stroke_align)` and `border.scale(t)` on the concrete type.

## circle_border.rs / oval_border.rs / rounded_rectangle_border.rs / stadium_border.rs

- Change: `OvalBorder` is a separate struct, not a subtype of `CircleBorder`. `CircleBorder::lerp_from` / `lerp_to` still run for a leftover `OvalBorder` via `OvalBorder`'s `super` forwarding.
  Reason: language — Rust has no inheritance. Same pattern as `FractionalOffset` vs `Alignment`.
  Affect: `OvalBorder` and `CircleBorder` with eccentricity 1.0 stay unequal (`runtimeType`). Lerp between them still returns a `CircleBorder`, as Dart's `OvalBorder is CircleBorder` does.

- Change: `draw_oval` / `draw_rsuperellipse` stand in for dart:ui `Canvas.drawOval` / `Canvas.drawRSuperellipse`.
  Reason: platform — valo has no those primitives. Oval is an elliptical `draw_rrect_radii_elliptical`; superellipse is a path.
  Affect: call `draw_oval` / `draw_rsuperellipse`. A display-list dump shows an rrect or path.

## image_provider.rs → image_provider.dart (`ImageConfiguration`) / services `asset_bundle.dart`

- Change: [`AssetBundle`](AssetBundle) is `load` returning `Result<Vec<u8>, String>`. There is no `ImageProvider`, `ImageStream`, or `ImageCache`.
  Reason: platform — a host or test can implement this completely; Flutter's Future plus codec/cache pipeline is the engine talking to Dart.
  Affect: `bundle.load(key)` is synchronous. Store `Option<Rc<dyn AssetBundle>>`. Equality is pointer identity, as Dart's default `==`.

- Change: [`ImageConfiguration`](ImageConfiguration) has no `locale` field.
  Reason: platform — `dart:ui` `Locale` is not ported; the field exists so `AssetImage` can pick a locale-specific asset, which is deferred with image loading.
  Affect: there is no `configuration.locale`. `copy_with` has no locale argument.

## decoration.rs → decoration.dart

- Change: [`Decoration`](Decoration) / [`BoxPainter`](BoxPainter) are traits. A stored decoration is `Box<dyn Decoration>`. `clone_box` stands in for Dart's shared immutable instances. `BoxPainter::paint` takes `&mut self`.
  Reason: language — the subclass set is open; a pairing enum cannot hold it. The painter caches `Paint` objects.
  Affect: store `Box<dyn Decoration>`. `Decoration.lerp` is `<dyn Decoration>::lerp`. `is` / `as` is `as_any().downcast_ref`. `create_box_painter` takes `Option<Box<dyn Fn()>>`.

- Change: `get_clip_path` returns `Arc<Path>`.
  Reason: platform — valo `Path` is already `Arc`.
  Affect: the path is `Arc`.

## box_decoration.rs → box_decoration.dart

- Change: [`BoxDecoration`](BoxDecoration) has no `image` or `gradient` field. Background paint is solid `color` only.
  Reason: platform — `DecorationImage` / `ImageProvider` and `Gradient.createShader` (valo shaders) are deferred; color, border, radius, and shadow do not need them.
  Affect: there is no `decoration.image` or `decoration.gradient`. A `background_blend_mode` still requires a `color`.

- Change: Dart's named constructor arguments are a fluent builder. `copyWith(color: c)` is `copy_with().color(c)`.
  Reason: language — Rust has no optional named parameters.
  Affect: write `BoxDecoration::new().color(c).border_radius(r)` where Dart writes `BoxDecoration(color: c, borderRadius: r)`. Set `color` before `background_blend_mode`.

## shape_decoration.rs → shape_decoration.dart

- Change: [`ShapeDecoration`](ShapeDecoration) has no `image` or `gradient` field. Interior paint is solid `color` only. `shape` is required on [`new`](ShapeDecoration::new); color and shadows are fluent.
  Reason: platform — `DecorationImage` and `Gradient.createShader` are deferred, same as [`BoxDecoration`](BoxDecoration). Language — Rust has no optional named parameters; `shape` is the one required field.
  Affect: write `ShapeDecoration::new(shape).color(c)` where Dart writes `ShapeDecoration(shape: shape, color: c)`. There is no `decoration.image` or `decoration.gradient`.

## linear_border.rs → linear_border.dart

- Change: Dart named constructors `LinearBorder.start` / `end` / `top` / `bottom` are `LinearBorder::start_side` / `end_side` / `top_side` / `bottom_side`. Fluent `.start(edge)` keeps the field name.
  Reason: language — an associated function and a method cannot share a name.
  Affect: write `LinearBorder::start_side(side, alignment, size)` or `LinearBorder::new().start(edge)` where Dart writes `LinearBorder.start(side: side)`.

## text_style.rs → text_style.dart

- Change: Dart's named constructor arguments and `copyWith` are a fluent builder. `copyWith(color: c)` is `copy_with().color(c)`.
  Reason: language — Rust has no optional named parameters.
  Affect: write `TextStyle::new().color(c).font_size(14.0)` where Dart writes `TextStyle(color: c, fontSize: 14)`.

- Change: `apply` is `style.apply().font_size_factor(2.0).into_style()`.
  Reason: language — Rust has no optional named parameters, and `apply` cannot return both a builder and a `TextStyle`.
  Affect: write `.into_style()` at the end of an `apply` chain. `style.apply().into_style()` is Dart `apply()` with defaults.

- Change: the `fontFamilyFallback` getter is [`font_family_fallback_list`](TextStyle::font_family_fallback_list). The setter keeps [`font_family_fallback`](TextStyle::font_family_fallback).
  Reason: language — a method and a setter cannot share a name.
  Affect: read `style.font_family_fallback_list()`.

- Change: [`TextStyle`](TextStyle) has no `locale` field.
  Reason: platform — `dart:ui` `Locale` is not ported; same as [`ImageConfiguration`](ImageConfiguration).
  Affect: there is no `style.locale`.

- Change: [`get_text_style`](TextStyle::get_text_style) takes no arguments; `get_text_style_with(&scaler)` is Dart `getTextStyle(textScaler: scaler)`. [`get_paragraph_style`](TextStyle::get_paragraph_style) is a fluent chain ending in `build()`.
  Reason: language — no optional named parameters.
  Affect: `style.get_paragraph_style().text_align(a).text_direction(d).build()` where Dart writes `style.getParagraphStyle(textAlign: a, textDirection: d)`. Neither takes `locale` or `strutStyle` (deferred).

## inline_span.rs → inline_span.dart, text_span.rs → text_span.dart

- Change: a span tree is shared: children are `InlineSpanRef` (`Rc<dyn InlineSpan>`), and a `TextSpan` becomes one with `into_span()`. Dart's named constructor arguments are the fluent setters `TextSpan::new().text("x").style(s)`. Equality is `*a == *b` on the trait object.
  Reason: language — no inheritance, no structural `==` on a trait object, no optional named parameters.
  Affect: build trees with `into_span()`; compare with `*a == *b` or `a.compare_to(&*b)`.

## binding.rs → binding.dart

- Change: `PaintingBinding` holds the app-wide `FontCollection`: `install_platform_fonts` (the shell, at start-up, from `Platform::font_source`) or `install_fonts` (tests, bundled fonts), then `fonts()`. `TextPainter`'s shaping methods (`layout`, `paint`, `preferred_line_height`, the caret queries, the two statics) take that collection as `&mut FontCollection`.
  Reason: platform — Flutter's engine owns one font manager per process and every `ui.Paragraph` shapes against it implicitly; here the collection is a value the framework owns and passes.
  Affect: `painter.layout(app.get_mut(fonts), min, max)` where Dart writes `painter.layout(minWidth:, maxWidth:)`; a paragraph built before fonts are installed panics with the message to install them.

## text_painter.rs → text_painter.dart

- Change: `TextPainter::new()` is the painter with every default; Dart's constructor arguments are the setters (`set_text`, `set_text_direction`, …). `compute_width` / `compute_max_intrinsic_width` take only `(text, text_direction, min_width, max_width)`.
  Reason: language — no optional named parameters.
  Affect: `let mut painter = TextPainter::new(); painter.set_text(Some(span)); painter.set_text_direction(Some(TextDirection::Ltr)); painter.layout(0.0, f64::INFINITY);`. To measure with other options than the two statics accept, build a painter the same way.

- Change: the getters that fill a cache take `&mut self`: `plain_text`, `preferred_line_height`, `compute_line_metrics`, `inline_placeholder_boxes`, and the caret queries (`get_offset_for_caret`, `get_full_height_for_caret`). `layout` and `paint` do too.
  Reason: language — Dart's getters write their cache through a `final` reference; Rust needs the borrow to say so.
  Affect: hold a `TextPainter` as `mut`, or behind `&mut`, to read those.

## Deferred
- `debug.dart` remainder (`debugNetworkImageHttpClientProvider`, …). Trigger: image loading / tests that are not `debugDisableShadows`.
- `ColorProperty` (diagnostics); `ColorSwatch` as a `const` table (`HashMap` is not const; a static swatch table needs a slice-backed form). Trigger: diagnostics; Material's `Colors`.
- Custom `TextScaler` / `SystemTextScaler`. Trigger: `MediaQuery`; `_ClampedTextScaler` is the `TextScaler::Clamped` arm, created by the default `clamp` on a non-linear scaler.
- `EdgeInsets.fromViewPadding` / `fromWindowPadding` / `EdgeInsetsGeometry.fromViewPadding`. Trigger: embedder / `MediaQuery`.
- `_MixedEdgeInsets` and cross-kind `add` / `subtract` / `flipped` / `infinity` / `clamp` / `EdgeInsetsGeometry.lerp`. Trigger: first consumer that adds an `EdgeInsets` to an `EdgeInsetsDirectional`.
- `_MixedAlignment` and cross-kind `AlignmentGeometry.add` / `AlignmentGeometry.lerp`. Trigger: first consumer that adds an `Alignment` to an `AlignmentDirectional`.
- `_MixedBorderRadius` and cross-kind `add` / `subtract` / `BorderRadiusGeometry.lerp`. Trigger: first consumer that adds a `BorderRadius` to a `BorderRadiusDirectional`. Same-kind `add` / `subtract` / `lerp` exist.
- `hashCode` / [`Hash`]. Trigger: the first map or set keyed by insets, alignment, border radius, `TextScaler`, or `TextStyle`.
- Diagnostics / `debugCheckCanResolveTextDirection`. Trigger: porting diagnostics; until then a missing `TextDirection` on resolve panics with the FlutterError summary string.
- `DecorationImage` / `BoxDecoration.image` / `ShapeDecoration.image`. Trigger: first decoration that paints an image.
- `ui.Locale` / `ImageConfiguration.locale` / `TextStyle.locale`. Trigger: locale-specific assets or region-specific glyphs.
- `Gradient` / `BoxDecoration.gradient` / `ShapeDecoration.gradient` / `createShader`. Trigger: first decoration that paints a gradient. valo covers linear, radial (with optional focus), and a full-turn sweep; `TileMode.decal` and sweep `endAngle` have no valo counterpart.
- `AssetBundle.loadString` / `loadStructuredData` / `loadBuffer` / caching / `NetworkAssetBundle` / `rootBundle`. Trigger: string or structured assets, or a default bundle on `App`.
- `StarBorder`. Trigger: a star or polygon `ShapeBorder`. Path verbs are mechanical (`conicTo` exists on valo); lerp to `CircleBorder` / `StadiumBorder` / `RoundedRectangleBorder` is large.
- `NotchedShape` / `CircularNotchedRectangle` / `AutomaticNotchedShape`. Trigger: `BottomAppBar`. valo has no `Path.arcToPoint` and no `Path.combine`.
- `PaintingBinding`. Trigger: image cache / shader warm-up.
- `strut_style.dart` / `getParagraphStyle(strutStyle:)` / `TextPainter.strutStyle`. Trigger: a paragraph that sets strut; the host has no strut, so `get_full_height_for_caret` uses the glyph's or the layout template's height.
- `TextPainter.locale` / `TextSpan.locale` / `TextSpan.spellOut`. Trigger: `Locale`.
- `PlaceholderSpan` / `WidgetSpan`. Trigger: `WidgetSpan`; the host has no placeholders.
- `TextSpan.recognizer`, `TextSpan` as a `HitTestTarget`. Trigger: `RichText` with a tappable span; needs an erased `GestureRecognizer`.
- `TextSpan.mouseCursor` / `onEnter` / `onExit` (the span as a `MouseTrackerAnnotation`). Trigger: a hoverable span; the mouse tracker keys annotations by render object.
- `TextSpan.semanticsLabel` / `semanticsIdentifier`, `InlineSpanSemanticsInformation`, `computeSemanticsInformation`. Trigger: accessibility; do not stub.
- `WordBoundary` / `TextPainter.wordBoundaries`. Trigger: text editing; needs services `TextBoundary`.
