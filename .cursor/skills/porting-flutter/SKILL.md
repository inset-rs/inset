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

Each diverge is Change / Reason / Affect, one sentence each: the idea, the kind and the fact behind it, what a reader would notice.

- **Change:** what ours does. Name the ported type, then write what it does in words.
- **Reason:** named kind, then the fact. Only two kinds:
  - `language` — Rust cannot express Flutter's mechanism (no isolate GC, no trait state, no closure identity, orphan rule, …).
  - `platform` — the host API is a small trait surface a real platform or a test can implement completely. Flutter's dart:ui is the engine talking to Dart; we do not copy that. Diverge when the host/test shape requires it (who owns the window, how a frame is requested, how the host talks to the framework).
  Not scope, not taste, not existing code, not crate layering. If you cannot name one of the two kinds, it is not a diverge — fix it or ask.
- **Affect:** what someone using the type notices when the code runs.

Not an entry:

- Rust's standard type in place of Dart's: `Duration`, `String`, the collections. Record it only where the substitute behaves differently and a caller sees it, such as an unordered map where Dart's keeps insertion order.
- What every ported type does the same way: the `App` parameter and typed handles, closures in `Rc`, `Debug` for `toString`. These are written down once, above, in `AGENTS.md` and in `patterns/`, and not repeated for one type.
- Spelling: names, `iterator` → `iter()`, `~/` → `truncating_div`, widget construction.
- Anything else a reader cannot notice when the code runs. That is Identical.

`## Deferred` is one line per item: what is missing, and the trigger.

```
# <crate>/src
Flutter home: packages/flutter/lib/src/<dir>
Ported against: <commit>

## Identical
- bar.rs → bar.dart

## foo.rs → foo.dart
- Change: `TextOverflow::Fade` clips like `Clip`.
  Reason: platform — the fade is a gradient shader, and gradients are deferred.
  Affect: overflowing text is cut, not faded.

- Change: each overlay owns its context-menu `OverlayEntry`; there is no `ContextMenuController` singleton.
  Reason: language — Dart's static `_shownInstance` is isolate-global.
  Affect: two `SelectionOverlay`s can show two menus at once.

- Change: a panic in `perform_layout`, `perform_resize` or `paint` unwinds.
  Reason: language — no `FlutterError.reportError` hook to catch and continue.
  Affect: layout and paint do not survive a failing render object.

## Deferred
- `Listenable.merge`. Trigger: first widget that holds a `Listenable` as a value.
```
- Widget construction and what counts as a divergence: [patterns/widget-syntax.md](patterns/widget-syntax.md).
