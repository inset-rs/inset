# waft-material-color-utilities/src
Dart home: material_color_utilities 0.13.0 (pub.dev)

## Identical
- utils/math_utils.rs → utils/math_utils.dart
- utils/color_utils.rs → utils/color_utils.dart
- utils/string_utils.rs → utils/string_utils.dart
- hct/cam16.rs → hct/cam16.dart
- hct/viewing_conditions.rs → hct/viewing_conditions.dart
- hct/src/hct_solver.rs → hct/src/hct_solver.dart
- contrast/contrast.rs → contrast/contrast.dart
- blend/blend.rs → blend/blend.dart
- dislike/dislike_analyzer.rs → dislike/dislike_analyzer.dart
- palettes/tonal_palette.rs → palettes/tonal_palette.dart
- palettes/core_palette.rs → palettes/core_palette.dart
- palettes/core_palettes.rs → palettes/core_palettes.dart
- temperature/temperature_cache.rs → temperature/temperature_cache.dart
- quantize/quantizer_wsmeans.rs → quantize/quantizer_wsmeans.dart
- quantize/src/point_provider.rs → quantize/src/point_provider.dart
- quantize/src/point_provider_lab.rs → quantize/src/point_provider_lab.dart
- dynamiccolor/dynamic_color.rs → dynamiccolor/dynamic_color.dart
- dynamiccolor/dynamic_scheme.rs → dynamiccolor/dynamic_scheme.dart
- dynamiccolor/variant.rs → dynamiccolor/variant.dart
- dynamiccolor/src/contrast_curve.rs → dynamiccolor/src/contrast_curve.dart
- dynamiccolor/src/tone_delta_pair.rs → dynamiccolor/src/tone_delta_pair.dart
- scheme/scheme.rs → scheme/scheme.dart
- dart_math.rs → the Dart VM's `math.max`, `math.min`, `double.compareTo`, double `%`, `round`/`floor`/`ceil` to `int`, `List.sort` and seeded `Random` (Dart SDK e5add7a25b32, `lib/_internal/vm/lib/math_patch.dart`, `double.dart`, `lib/core/num.dart`, `lib/internal/sort.dart`)

## hct/hct.rs → hct/hct.dart
- Change: `Hct` is a `Copy` value; `set_hue`, `set_chroma` and `set_tone` change only the copy they are called on.
  Reason: language — a Dart `Hct` is a shared mutable object, and Rust hands out copies or borrows, not shared mutable references.
  Affect: changing the tone of the `Hct` read from a palette's `key_color()` or a scheme's `source_color_hct()` leaves the palette or scheme as it was.

## dynamiccolor/material_dynamic_colors.rs → dynamiccolor/material_dynamic_colors.dart
- Change: each `static final DynamicColor` is a function that returns an `Rc<DynamicColor>`, made on its first call on a thread and the same color, with its cache, on every later call there.
  Reason: language — a Rust static must be `Sync`, and a `DynamicColor` holds `Rc` closures and a `RefCell` cache.
  Affect: each thread has its own colors and caches, as each Dart isolate has its own statics; `MaterialDynamicColors::primary()` is a call, not a field.

## quantize/quantizer.rs, quantizer_map.rs, quantizer_wu.rs, quantizer_celebi.rs → the same .dart files
- Change: `Quantizer::quantize` does its work in the call and returns a ready `std::future::Ready<QuantizerResult>`; Celebi takes Wu's result with `into_inner` instead of awaiting it.
  Reason: language — Rust futures run only when polled, and these bodies await nothing but each other's already-complete results.
  Affect: quantizing happens when `quantize` is called, where Dart runs Celebi's k-means in a later microtask; awaiting the future gives the same result.

## scheme/scheme_content.rs, scheme_expressive.rs, scheme_fidelity.rs, scheme_fruit_salad.rs, scheme_monochrome.rs, scheme_neutral.rs, scheme_rainbow.rs, scheme_tonal_spot.rs, scheme_vibrant.rs → the same .dart files
- Change: `SchemeTonalSpot` and the other variants are unit structs whose `new(source_color_hct, is_dark, contrast_level)` returns a `DynamicScheme`.
  Reason: language — Rust has no subclasses, and these Dart subclasses add nothing but a constructor.
  Affect: a scheme cannot be asked whether it is a `SchemeTonalSpot`; its `variant()` answers.

## dart_math.rs (`List.sort`), score/score.rs, temperature/temperature_cache.rs, quantize/quantizer_wsmeans.rs
- Change: every `List.sort` of the package runs `dart_math::sort`, a port of the Dart VM's `Sort.sort` (insertion sort up to 33 elements, dual-pivot quicksort above), with Dart's `compareTo` comparators.
  Reason: language — Rust's `sort_by` is stable and panics on an inconsistent order, and Dart's `List.sort` documents no order for ties; the VM's order is its implementation's.
  Affect: ties land as the Dart VM puts them (checked against Dart 3.13.1), and scores that are all NaN sort without panicking; a Dart runtime with another sort could order ties differently.

## lib.rs → material_color_utilities.dart
- Change: the crate root re-exports everything the package's barrel exports except Wu's `Box`, which stays at `quantize::quantizer_wu::Box`.
  Reason: language — a glob import of the crate root would replace the prelude's `Box` with Wu's.
  Affect: a caller names `quantize::quantizer_wu::Box`, not `waft_material_color_utilities::Box`.
