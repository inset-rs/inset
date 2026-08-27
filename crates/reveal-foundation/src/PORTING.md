# reveal-foundation/src
Flutter home: packages/flutter/lib/src/foundation
Ported against: (none yet — this crate holds `App` / `Handle` only)

## app.rs

- Change: `App` and `Handle<T>` live here; Flutter has no counterpart.
  Reason: Rust has no isolate-global GC, so a named owner has to hold the arena.
