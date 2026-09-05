# reveal-embedder-winit/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.

No Flutter counterpart. Flutter's engine is C++ and is not in this checkout.

## lib.rs / window.rs / keys.rs

- Change: this crate is a host: it implements `Platform` and `View`, creates the implicit window if configured, then hands `Platform` to a start closure that returns the client. It never names `App`.
  Reason: platform — Flutter's native embedding does not name the framework, and winit cannot create a window until `resumed`.
  Affect: `WinitEmbedder::run(|platform| Shell::new(platform, setup))`.

- Change: the scheduler asks for one frame for the whole app and the host maps that onto a native redraw; native windows and their `View` handles live in a host-side registry.
  Reason: platform — winit redraw is per window and the window must stay on the event-loop thread, while the scheduler requests one frame.
  Affect: closing the last window ends `run`. A frame runs the scheduler; `View::present` draws the valo picture.

- Change: `Platform::activate_system_cursor` records the requested kind and pokes the loop, which sets the winit cursor icon on the window the pointer was last seen in; `None` hides the cursor, and kinds winit lacks show the arrow.
  Reason: platform — Flutter's engine maps cursor kinds per host; winit has one cursor per window.
  Affect: a `MouseRegion` cursor shows up on hover. The device id is ignored: one mouse.

- Change: winit key events become dart:ui `KeyData` in `keys.rs`: the physical key is the USB HID usage for winit's W3C code, the logical key is derived the way the web engine's converter does it (the named-key table, then the location table, then the lower-cased character), and repeat, character, synthesized and the time stamp are filled from the winit event. Both tables are transcribed from Flutter's own key data.
  Reason: platform — Flutter's engine has a hand-written key mapping per OS; winit is the OS here and reports the same W3C values Flutter keys its tables by.
  Affect: pressing a key reaches `HardwareKeyboard`; the physical key is the USB HID usage a Flutter app expects, whatever the layout.

- Change: the host remembers the logical key each held physical key went down with and reports that one on its repeats and its up (the engine's `_pressingRecords`).
  Reason: platform — winit re-derives the logical key from the current modifier state on every event, so a release after a modifier change would otherwise report a different key than the press did.
  Affect: a down/up pair always agrees, so `HardwareKeyboard`'s pressed-key sets stay consistent.

- Change: a key winit reports that Flutter's tables have no code for produces no event at all, where Flutter's engine mints an id out of its own platform plane.
  Reason: platform — there is no winit plane in `LogicalKeyboardKey`, and a minted physical usage would collide with a real one.
  Affect: `NumpadHash`, `NumpadStar`, `F25` and up, the legacy `Hiragana` / `Katakana`, and keys winit cannot identify are silently dropped.

- Change: `Platform::haptic_feedback` and `Platform::set_system_ui_overlay_style` are left at the trait defaults, which drop the request.
  Reason: platform — a desktop machine has no haptic engine and no status bar to style; Flutter's own Linux and Windows embedders answer both channels with not-implemented.
  Affect: `HapticFeedback::selection_click(&app)` and its siblings are silent, and the status-bar style a `CupertinoNavigationBar` or `CupertinoApp` asks for shows nowhere; the same calls work on a phone host.

- Change: a backdrop filter is replayed as a blur only; the colour filter a Flutter `ImageFilter.compose` puts over the blur is dropped rather than moved onto the layer paint, which would filter the children as well as the glass.
  Reason: platform — valo's backdrop has no colour stage, and its `Paint`'s colour filter applies to the whole composited layer.
  Affect: a `CupertinoPopupSurface` is blurred but not saturated, as is any frosted surface that composes a colour matrix over a backdrop blur; anything else composed into a backdrop filter is likewise dropped.

- Change: `Platform::font_source` is `valo-system-fonts`' OS scanner.
  Reason: platform — Flutter's engine finds platform fonts itself; valo needs a `FontSource`.
  Affect: text uses installed fonts; a family that is not installed falls back to the nearest face.

- Change: `present` asks the window for another redraw when valo cannot acquire a surface texture (wgpu's `Timeout` on the first frame of a just-shown window), so the retained scene is presented next vsync.
  Reason: platform — Flutter's Metal surface always has a drawable, so its engine drops a frame whose surface was not ready and resubmits only on Android's first-frame path; the redraw request is that resubmit here.
  Affect: the first frame appears without a resize; a frame is never dropped for a surface that was not ready.

## Deferred

- `ThemeChanged` → `onPlatformBrightnessChanged`. Trigger: `MediaQuery` / `CupertinoTheme`.
- The layout keymap for character keys: a shifted symbol reports the symbol's own logical key (`!`), where Flutter's engine consults the layout and reports the key that produced it (`digit1`); letters and digits are unaffected, since the character is lower-cased. Trigger: a shortcut that matches a shifted symbol by logical key.
- Dead keys (`Key::Dead`) and `Key::Unidentified`. Trigger: composing input, or a host key winit cannot name.
- Synthesizing the key ups a focus change swallowed; winit's `is_synthetic` covers Windows and X11, and macOS delivers nothing. Trigger: `HardwareKeyboard::sync_keyboard_state` gaining a host query.
