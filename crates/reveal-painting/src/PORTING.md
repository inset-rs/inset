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

- Change: [`get_text_style`](TextStyle::get_text_style) returns valo `TextStyle`. Combined decorations keep underline, else overline, else line-through. `decorationStyle`, background paint, `fontFeatures`, `fontVariations`, `textBaseline`, and `leadingDistribution` are not valo fields. Unspecified `fontSize` becomes valo's 14.
  Reason: platform — valo `TextStyle` is the host span style; there is no engine encode.
  Affect: `get_text_style()` is a valo `TextStyle`. Combined underline+lineThrough paints only underline. `get_text_style_with(&scaler)` is Dart `getTextStyle(textScaler: scaler)`.

- Change: Dart's `getParagraphStyle` writes layout and a fallback font onto one `ui.ParagraphStyle`. Valo takes layout and font as two objects. [`get_paragraph_style`](TextStyle::get_paragraph_style) is the layout; the font is [`get_text_style`](TextStyle::get_text_style).
  Reason: platform — valo requires both styles; Flutter combines them on `ui.ParagraphStyle`.
  Affect: there is no `paragraph_style.font_size`. Pass both into the paragraph builder.

- Change: valo `TextAlign` has no `Start` / `End`. [`get_paragraph_style`](TextStyle::get_paragraph_style) resolves them to `Left` / `Right` from `text_direction`. Omitted direction is treated as LTR.
  Reason: platform — valo stores a physical alignment; it cannot keep Start/End until bidi direction is inferred.
  Affect: pass `text_direction` when the align is Start or End.

## Deferred
- `debug.dart` remainder (`debugNetworkImageHttpClientProvider`, …). Trigger: image loading / tests that are not `debugDisableShadows`.
- `ColorSwatch` / `ColorProperty`. Trigger: Material colors / diagnostics. `Color` stays the dart:ui struct (stored by value on `Paint` / `BorderSide` / `TextStyle`); a `Color` trait cannot be that field type. `ColorSwatch` can wrap the primary `Color` plus a table, like `FractionalOffset` vs `Alignment`.
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
- `text_painter.dart` remainder (`TextPainter`, `PlaceholderDimensions`, …). Trigger: painting glyphs / `Text`. `kDefaultFontSize` and `TextOverflow` live in `text_painter.rs`.
- `strut_style.dart` / `getParagraphStyle(strutStyle:)`. Trigger: a paragraph that sets strut.
