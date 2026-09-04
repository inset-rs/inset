# reveal-services/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/services
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

Only mouse cursors, the mouse tracking annotation, and `TextSelection` are here; the rest of the package waits.

## mouse_cursor.rs → mouse_cursor.dart

- Change: a cursor is a shared trait object, `MouseCursorRef`. Dart's canonical const instances (`MouseCursor.defer`, `SystemMouseCursors.click`) are values equal to every other value of their class, not one identity.
  Reason: language — no canonical const objects to compare by identity.
  Affect: compare cursors as `*a == *b`; store `MouseCursorRef` where Dart stores `MouseCursor`.

- Change: a session's `activate` asks the host directly: `Platform::activate_system_cursor(device, kind)`, and returns nothing. `SystemMouseCursor::kind` is the embedder's `SystemMouseCursorKind` enum, not a string.
  Reason: platform — Flutter sends `activateSystemCursor` over `SystemChannels.mouseCursor`; there are no method channels, the host trait is the channel and nothing needs a string encoding.
  Affect: a host that can show cursors implements that method; the inert one ignores it.

- Change: `MouseCursorManager::handle_device_cursor_update` takes the `Platform` it should talk to.
  Reason: language — the manager lives inside another arena object and cannot reach `App` from `&mut self`.
  Affect: pass `&*app.platform()`.

## text_editing.rs → text_editing.dart

- Change: `TextSelection` does not extend `TextRange`; [`range()`](TextSelection::range) is the normalized range.
  Reason: language — no inheritance.
  Affect: pass `selection.range()` where a `TextRange` is wanted.

## Deferred

- Platform channels, `SystemChannels`, `BinaryMessenger`. Trigger: the first service that talks to the host over a named channel; so far each need is a `Platform` method.
- `MouseCursor` diagnostics (`debugFillProperties`, `toString(minLevel)`). Trigger: diagnostics.
- Keyboard, text input, clipboard, system chrome, haptics, asset bundles. Trigger: `Focus`, `EditableText`, `CupertinoButton` haptics (none on the button path yet).
