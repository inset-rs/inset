---
name: porting-flutter
description: >-
  Transcribe a Flutter Dart file into reveal-rs. Copy the Dart then modify;
  record divergences in PORTING.md; stop and ask when something does not fit.
  Use before writing or changing any Rust that corresponds to a Flutter source file.
---

# Porting Flutter

Ground truth: `/Users/mac/code/flutter/packages/flutter/lib/src`. Read the Dart. Do not write it from memory.

Copy the file, then change only what Rust forces. Names, member order, defaults, branches, who decides, when hooks run — same as Flutter. A Flutter-shaped name on a different mechanism is a bug.

When something does not fit: stop and ask. Use a proved pattern (`Handle`, mixin-as-field, `Drop` guard), not a one-off. Do not drop public members or invent a stand-in for a missing dependency without asking.

A pattern earns a file under `patterns/` on its second instance. Until then, the note lives in that folder's `PORTING.md`.

## Rust traps

- Dart `assert` → `debug_assert!`. Never put a side effect in one — it does not run in release. `assert(() { …; return true; }())` becomes `if cfg!(debug_assertions) { … }`. Keep protocol invariants; drop inspector-only asserts.
- Do not `#[derive(PartialEq)]` unless Dart overrides `==`. Dart defaults to identity; a derive answers "unchanged" and skips work.
- `try` / `finally` → a `Drop` guard. Cleanup at the end of the block is skipped on panic.
- `mixin class` with state (`ChangeNotifier`) → a struct field on the host, named after the mixin. `mixin` that is never instantiated → a trait.
- `_foo` → private `foo`. `toString` / `debugFillProperties` → `Debug`.

## PORTING.md

For a reader who knows Rust and only surface Flutter. Nothing to say: omit the file. Straight copy: one line under `## Identical`.

```
# <crate>/src
Flutter home: packages/flutter/lib/src/<dir>
Ported against: <commit>

## Identical
- foo.rs → foo.dart

## foo.rs → foo.dart
- Change: Dart's `Key('x')` factory is `ValueKey::new("x")`.
  Reason: a Rust trait has no constructor that picks a concrete implementor.

## Deferred
- `Listenable.merge`. Trigger: first widget that holds a `Listenable` as a value.
```

`Reason:` is a specific Rust language or crate-layering issue. Not scope, not taste, not existing code. If you cannot name one, it is not a diverge — fix it or ask.
