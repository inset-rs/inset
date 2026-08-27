# reveal-foundation/src
Flutter home: packages/flutter/lib/src/foundation
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical

- observer_list.rs → observer_list.dart

## app.rs

- Change: `App` and `Handle<T>` live here; Flutter has no counterpart.
  Reason: Rust has no isolate-global GC, so a named owner has to hold the arena.

## constants.rs → constants.dart

- Change: `kProfileMode` is the constant `false`.
  Reason: Cargo profiles are invisible to the compiler. `cfg(debug_assertions)` is the only build distinction Rust exposes, and it splits two ways where Dart splits three.

- Change: `kIsWeb` and `kIsWasm` are always equal (`cfg!(target_family = "wasm")`).
  Reason: Rust has one wasm target family and no JS-compilation path, so there is no fact for Flutter's dart2js/dart2wasm distinction to rest on.
