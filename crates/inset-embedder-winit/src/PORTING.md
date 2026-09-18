# inset-embedder-winit/src

Host implementation references: Flutter's C++ engine under `flutter/engine/src/flutter`.

## lib.rs / window.rs / platform.rs / view.rs / frames.rs / input.rs / keys.rs

- Change: this crate is a host — it implements `Platform` and `View`, creates the implicit window once winit resumes, and hands `Platform` to a start closure that returns the client, never naming `App`.
  Reason: platform — Flutter's native embedding does not name the framework, and winit cannot create a window before `resumed`.
  Affect: no framework code runs until the event loop resumes, so anything the start closure sets up sees a window that already exists.

- Change: when the system gives a window's surface back, each view makes its surface again on that window and presents the picture it still owes. The view, its id and the framework's tree are left alone.
  Reason: platform — winit reports the surface going and coming back as its loop suspending and resuming, and Flutter's iOS embedder keeps its view across the same thing.
  Affect: an app sent to the background and reopened keeps its state and shows its last frame at once. A frame drawn while the surface was gone is held for the return rather than lost.

- Change: every finger is its own pointer, numbered by the system's touch id. It is added before its down and removed after its up or cancel, all in one packet.
  Reason: platform — winit reports touches beside the mouse's events, and Flutter's iOS embedder sends that same add and remove around a touch.
  Affect: taps, drags and pinches work on iOS. A finger joins and leaves the framework's set of devices with its contact, and its press carries an identifier no mouse press shares.

- Change: the scheduler's one frame request for the whole app is mapped onto native redraws, with windows and their `View` handles in a host-side registry.
  Reason: platform — winit redraw is per window and the window must stay on the event-loop thread.
  Affect: one frame request redraws every window, and closing the last window ends `run`.

- Change: the host's `MouseCursor` capability sets the winit cursor icon on the window the pointer was last seen in; `None` hides the cursor and kinds winit lacks show the arrow.
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
  Affect: copy and paste work with other apps on the desktops, and a host that cannot open the pasteboard reads an empty clipboard rather than failing.

- Change: the host offers neither the `Haptics` nor the `SystemChrome` capability, so both answer `None`.
  Reason: platform — a desktop machine has no haptic engine and no status bar, and Flutter's own Linux and Windows embedders answer both with not-implemented.
  Affect: `HapticFeedback` calls are silent and the status-bar style a `CupertinoNavigationBar` asks for shows nowhere; the same calls work on a phone host.

- Change: a backdrop filter is replayed as its blur only; a colour filter composed over the blur is dropped rather than moved onto the layer paint, which would filter the children too.
  Reason: platform — only the first blur of the filter tree reaches valo's backdrop op.
  Affect: a `CupertinoPopupSurface`, and any frosted surface that composes a colour matrix over a backdrop blur, is blurred but not saturated.

- Change: `Platform::font_source` is `SystemFontSource::platform()` over the OS font API.
  Reason: platform — Flutter's engine finds platform fonts itself; valo needs a `FontSource`.
  Affect: text uses installed fonts, the Cupertino system-font names resolve to SF, and an uninstalled family falls back to whatever the OS picks for the character.

- Change: the valo context hides missing glyphs.
  Reason: platform — Valo paints `.notdef` tofu unless asked not to.
  Affect: a character with no face occupies layout space and draws nothing.

- Change: where the system has no display link for a view (macOS before 14, and the other desktops), the framework's frames are paced by a timer at the display's nominal refresh rate.
  Reason: platform — Flutter's embedders each take a vsync signal from their system; winit offers none, and only AppKit's `CADisplayLink` is reachable, through the view.
  Affect: on those hosts an animation's frames land at the display's rate but not on its refresh, so a step can arrive a little early or late.

- Change: the host's text input leaves `handles_editing_keys` at no, and does not forward the editing commands macOS names for a key.
  Reason: platform — winit does run the key through AppKit, but keeps only the plain key press and drops the command name, as gpui's own macOS window does.
  Affect: a text field is edited by the framework's own key bindings, so a user's personal key-binding overrides and the Control-key editing bindings macOS would supply do not reach it.

## window.rs / platform.rs → native view focus

- Change: view-focus requests are queued and `WindowEvent::Focused` is forwarded as a `ViewFocusEvent`.
  Reason: platform — winit reports window focus without a traversal direction.
  Affect: native focus events always carry `ViewFocusDirection::Undefined`, and a focus-loss request is ignored while a focus-gain request is honoured, as Flutter's macOS host does.

## drag_drop.rs → the macOS embedder's `draggingEntered:`/`performDragOperation:`

- Change: files dragged over a window reach the client as one `DropData` per phase, gathered over a turn of the loop from winit's one-event-per-file `HoveredFile`/`DroppedFile`/`HoveredFileCancelled`, with no position; Flutter's embedders have no drop at all.
  Reason: platform — winit 0.30 reports each file on its own and never where the drag is, and a turn's end is the only batch boundary it gives.
  Affect: an app hears an entered, a dropped and an exited report per drag, never a move, and must treat the whole window as the target.

## Deferred

- The layout keymap for character keys: a shifted symbol reports the symbol's own logical key (`!`) where Flutter consults the layout and reports `digit1`. Trigger: a shortcut that matches a shifted symbol by logical key.
- Dead keys and `Key::Unidentified`. Trigger: composing input, or a host key winit cannot name.
- Synthesizing the key ups a focus change swallowed. Trigger: `HardwareKeyboard::sync_keyboard_state` gaining a host query.
