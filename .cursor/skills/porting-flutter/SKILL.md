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

When something does not fit: stop and ask. Use a proved pattern (`Handle` receiver, mixin Data+Mixin, fluent optional fields, `Drop` guard), not a one-off. Do not drop public members or invent a stand-in for a missing dependency without asking.

A pattern earns a file under `patterns/` on its second instance. Until then, the note lives in that folder's `PORTING.md`.

## Verified

- Handle receiver and erased edges: [patterns/handle-receiver.md](patterns/handle-receiver.md)
- Mixins and base classes: [patterns/mixin.md](patterns/mixin.md)
- Leaf inheritance: [patterns/leaf-inheritance.md](patterns/leaf-inheritance.md)
- Many optional fields: [patterns/many-optional-fields.md](patterns/many-optional-fields.md)
- Callback identity: [patterns/callback-identity.md](patterns/callback-identity.md)

## Rust traps

- Dart `assert` → `debug_assert!`. Never put a side effect in one — it does not run in release. `assert(() { …; return true; }())` becomes `if cfg!(debug_assertions) { … }`. Keep protocol invariants; drop inspector-only asserts.
- Do not `#[derive(PartialEq)]` unless Dart overrides `==`. Dart defaults to identity; a derive answers "unchanged" and skips work.
- `try` / `finally` → a `Drop` guard. Cleanup at the end of the block is skipped on panic.
- `_foo` → private `foo`. `toString` / `debugFillProperties` → `Debug`.

## PORTING.md

For a reader who knows Rust and only surface Flutter. Records **functional** divergences. Nothing to say: omit the file. Straight copy: one line under `## Identical`.

Each diverge is Change / Reason / Affect.

- **Change:** what ours does
- **Reason:** named kind, then the fact. Only two kinds:
  - `language` — Rust cannot express Flutter's mechanism (no isolate GC, no trait state, no closure identity, orphan rule, …).
  - `platform` — the host API is a small trait surface a real platform or a test can implement completely. Flutter's dart:ui is the engine talking to Dart; we do not copy that. Diverge when the host/test shape requires it (who owns the window, how a frame is requested, how the host talks to the framework).
  Not scope, not taste, not existing code, not crate layering. If you cannot name one of the two kinds, it is not a diverge — fix it or ask.
- **Affect:** the direct effect on a user of the type — a different call, a different operator, a different observable.

No visible Affect means Identical: do not record the entry. Naming, `iterator` → `iter()`, `~/` → `truncating_div`, dropped `growable` flags, and other spelling that does not change what a caller can do, stay out.

```
# <crate>/src
Flutter home: packages/flutter/lib/src/<dir>
Ported against: <commit>

## Identical
- foo.rs → foo.dart

## foo.rs → foo.dart
- Change: Dart's `Key('x')` factory is `ValueKey::new("x")`.
  Reason: language — a Rust trait has no constructor that picks a concrete implementor.
  Affect: write `ValueKey::new("x")` where Dart writes `Key('x')`.

## Deferred
- `Listenable.merge`. Trigger: first widget that holds a `Listenable` as a value.
```
