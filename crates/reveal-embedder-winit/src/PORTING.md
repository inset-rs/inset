# reveal-embedder-winit/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.

No Flutter counterpart. Flutter's engine is C++ and is not in this checkout.

## lib.rs / window.rs / keys.rs

- Change: this crate is a host — it implements `Platform` and `View`, creates the implicit window once winit resumes, and hands `Platform` to a start closure that returns the client, never naming `App`.
  Reason: platform — Flutter's native embedding does not name the framework, and winit cannot create a window before `resumed`.
  Affect: no framework code runs until the event loop resumes, so anything the start closure sets up sees a window that already exists.

- Change: the scheduler's one frame request for the whole app is mapped onto native redraws, with windows and their `View` handles in a host-side registry.
  Reason: platform — winit redraw is per window and the window must stay on the event-loop thread.
  Affect: one frame request redraws every window, and closing the last window ends `run`.

- Change: `Platform::activate_system_cursor` sets the winit cursor icon on the window the pointer was last seen in; `None` hides the cursor and kinds winit lacks show the arrow.
  Reason: platform — winit has one cursor per window where Flutter's engine maps cursor kinds per host.
  Affect: a `MouseRegion` cursor shows on hover, the device id is ignored (one mouse), and an unsupported kind is silently the arrow.

- Change: winit key events become dart:ui `KeyData` — the physical key is the USB HID usage for winit's W3C code, and the logical key follows the web engine's derivation (named-key table, location table, then the lower-cased character).
  Reason: platform — winit is the OS here and reports the same W3C values Flutter's own key tables are keyed by.
  Affect: the physical key is the usage a Flutter app expects whatever the layout.

- Change: the host remembers the logical key each held physical key went down with and reports that one on its repeats and its up.
  Reason: platform — winit re-derives the logical key from the current modifier state on every event.
  Affect: a release after a modifier change still reports the key the press did, so `HardwareKeyboard`'s pressed-key sets stay consistent.

- Change: a key Flutter's tables have no code for produces no event, where Flutter's engine mints an id in its own platform plane.
  Reason: platform — there is no winit plane in `LogicalKeyboardKey`, and a minted physical usage would collide with a real one.
  Affect: `NumpadHash`, `NumpadStar`, `F25` and up, the legacy `Hiragana` / `Katakana` and keys winit cannot identify are silently dropped.

- Change: `View` text input maps onto winit IME — start and stop are `set_ime_allowed`, obscured text is `ImePurpose::Password`, the caret rect is `set_ime_cursor_area`, and `WindowEvent::Ime` becomes a `TextEditingValue` with UTF-8 offsets converted to UTF-16.
  Reason: platform — winit's IME is preedit/commit, not Flutter's method channel.
  Affect: a field that has called `TextInput::attach` gets commits and composing updates, and `Ime::Disabled` arrives as `text_input_closed`.

- Change: the clipboard methods talk to the OS pasteboard through arboard, where Flutter's `Clipboard` messages go to the engine.
  Reason: platform — winit does not own the pasteboard, so the host implements the channel.
  Affect: copy and paste work with other apps; a host that cannot open the pasteboard reads an empty clipboard rather than failing.

- Change: `haptic_feedback` and `set_system_ui_overlay_style` are left at the trait defaults, which drop the request.
  Reason: platform — a desktop machine has no haptic engine and no status bar, and Flutter's own Linux and Windows embedders answer both with not-implemented.
  Affect: `HapticFeedback` calls are silent and the status-bar style a `CupertinoNavigationBar` asks for shows nowhere; the same calls work on a phone host.

- Change: winit mouse buttons become dart:ui `PointerData.buttons` bits; a Down is sent when the first button goes down, Move while any remain, Up when the last is released.
  Reason: platform — Flutter's engine maps host mouse buttons this way, and winit is the OS here.
  Affect: a right-click reaches secondary-tap handlers and a middle-click is tertiary.

- Change: a backdrop filter is replayed as its blur only; a colour filter composed over the blur is dropped rather than moved onto the layer paint, which would filter the children too.
  Reason: platform — only the first blur of the filter tree reaches valo's backdrop op.
  Affect: a `CupertinoPopupSurface`, and any frosted surface that composes a colour matrix over a backdrop blur, is blurred but not saturated.

- Change: `Platform::font_source` is `SystemFontSource::platform()` over the OS font API.
  Reason: platform — Flutter's engine finds platform fonts itself; valo needs a `FontSource`.
  Affect: text uses installed fonts, the Cupertino system-font names resolve to SF, and an uninstalled family falls back to whatever the OS picks for the character.

- Change: `present` asks the window for another redraw when valo cannot acquire a surface texture, so the retained scene is presented next vsync.
  Reason: platform — Flutter's Metal surface always has a drawable, so its engine drops such a frame and resubmits only on Android's first-frame path.
  Affect: the first frame of a just-shown window appears without needing a resize, and no frame is lost to a surface that was not ready.

- Change: `WinitView` presents through Core Animation transactions on macOS, and a resize callback delivers updated metrics and a frame before returning.
  Reason: platform — AppKit commits window geometry independently of a later winit redraw.
  Affect: live resizing shows newly laid-out content together with the new window size instead of stretching the previous frame.

## window.rs → native view focus

- Change: view-focus requests are queued and `WindowEvent::Focused` is forwarded as a `ViewFocusEvent`.
  Reason: platform — winit reports window focus without a traversal direction.
  Affect: native focus events always carry `ViewFocusDirection::Undefined`, and a focus-loss request is ignored while a focus-gain request is honoured, as Flutter's macOS host does.

## Deferred

- The layout keymap for character keys: a shifted symbol reports the symbol's own logical key (`!`) where Flutter consults the layout and reports `digit1`. Trigger: a shortcut that matches a shifted symbol by logical key.
- Dead keys and `Key::Unidentified`. Trigger: composing input, or a host key winit cannot name.
- Synthesizing the key ups a focus change swallowed. Trigger: `HardwareKeyboard::sync_keyboard_state` gaining a host query.
