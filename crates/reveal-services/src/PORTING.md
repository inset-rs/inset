# reveal-services/src
Flutter home: packages/flutter/lib/src/services
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

Only mouse cursors, the mouse tracking annotation, and `TextSelection` are here; the rest of the package waits.

## mouse_cursor.rs → mouse_cursor.dart

- Change: a cursor is a shared trait object, `MouseCursorRef`. Dart's `MouseCursor.defer` / `MouseCursor.uncontrolled` are `<dyn MouseCursor>::defer()` / `uncontrolled()`, equal to every other value of their class; `SystemMouseCursors::CLICK` is a `Copy` constant that becomes a `MouseCursorRef` with `.into()`.
  Reason: language — no `static const` on a trait, and no canonical const objects to compare by identity.
  Affect: compare cursors as `*a == *b`; store `MouseCursorRef` where Dart stores `MouseCursor`.

- Change: a session's `activate` asks the host directly: `Platform::activate_system_cursor(device, kind)`, and returns nothing. `SystemMouseCursor::kind` is the embedder's `SystemMouseCursorKind` enum, not a string.
  Reason: platform — Flutter sends `activateSystemCursor` over `SystemChannels.mouseCursor`; there are no method channels, the host trait is the channel and nothing needs a string encoding.
  Affect: a host that can show cursors implements that method; the inert one ignores it.

- Change: `MouseCursorManager::handle_device_cursor_update` takes the `Platform` it should talk to.
  Reason: language — the manager lives inside another arena object and cannot reach `App` from `&mut self`.
  Affect: pass `&*app.platform()`.

## mouse_tracking.rs → mouse_tracking.dart

- Change: `MouseTrackerAnnotation` is a plain struct with `Default`; the listener typedefs (`PointerEnterEventListener`, `PointerExitEventListener`, `PointerHoverEventListener`) live in `reveal-gestures` next to the events.
  Reason: language — no named optional constructor arguments; gestures already defines the sibling listener typedefs.
  Affect: `MouseTrackerAnnotation { on_enter: Some(..), ..Default::default() }`. Who *is* an annotation (Dart's `implements MouseTrackerAnnotation`) is decided in rendering: see its `PORTING.md`.

## text_editing.rs → text_editing.dart

- Change: `TextSelection` does not extend `TextRange`: [`range()`](TextSelection::range) is the normalized range, and `start()` / `end()` / `is_valid()` / `is_collapsed()` are methods. `copyWith` is the fluent chain `copy_with().extent_offset(n)`.
  Reason: language — no inheritance; no optional named parameters.
  Affect: write `selection.start()` where Dart reads `selection.start`; pass `selection.range()` where a `TextRange` is wanted.

## Deferred

- Platform channels, `SystemChannels`, `BinaryMessenger`. Trigger: the first service that talks to the host over a named channel; so far each need is a `Platform` method.
- `MouseCursor` diagnostics (`debugFillProperties`, `toString(minLevel)`). Trigger: diagnostics.
- Keyboard, text input, clipboard, system chrome, haptics, asset bundles. Trigger: `Focus`, `EditableText`, `CupertinoButton` haptics (none on the button path yet).
