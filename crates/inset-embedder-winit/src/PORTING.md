# inset-embedder-winit/src

Embedder implementation references: Flutter's C++ engine under `flutter/engine/src/flutter`.

## Identical

- app_exit.rs → how Flutter's desktop embedders answer `System.exitApplication`
- lifecycle.rs, os/macos/app_lifecycle.rs → the macOS embedder's lifecycle states (`FlutterEngine`)
- os/macos/app_delegate.rs, quitting → the macOS embedder's `FlutterAppDelegate`, which asks the framework before the app quits
- windows.rs, frame_scheduler.rs → the embedders' frame, in which the framework draws every view
- frame_scheduler.rs, vsync_source.rs, os/macos/display_links.rs → the macOS embedder's vsync, from the display the first view is on
- launch.rs, main_loop/ → the engine's start (`Shell` adds the implicit view with empty metrics): the app starts with the implicit view at no size, and its window and surface come after
- platform.rs, main_loop/windows.rs → the macOS embedder's implicit view, which the framework never hears removed: it outlives its window and draws nowhere once the window closes
- main_loop/quitting.rs, os/macos/app_delegate.rs → the macOS template's last window, whose closing asks the framework whether the app quits, as Cmd-Q does
- main_loop/windows.rs, view.rs, rasterizer.rs → the shell's surfaces (`Shell::OnPlatformViewCreated`, `OnPlatformViewDestroyed`): the view stays while the system has taken its surface, and each surface given gets a frame
- touch.rs → the iOS embedder's touches, one pointer per finger, added before its down and removed after its up

## lib.rs / main_loop/

- Change: whether closing the last window quits the app is a setting of `WinitEmbedder`, on by default, where a Flutter macOS app answers AppKit's question in its own application delegate.
  Reason: platform — winit owns the application delegate, so an app cannot answer it there.
  Affect: an app that lives in the Dock turns the setting off and runs on with no window, without being asked whether to quit.

- Change: on macOS a panic inside a call from AppKit aborts the process once the panic hook has printed it.
  Reason: platform — a panic cannot unwind through AppKit's frames.
  Affect: such a panic ends the process rather than coming out of `run`.

## frame_scheduler.rs / vsync_source.rs

- Change: without a screen's display link, on macOS before 14 and on the other desktops, frames follow a timer at the display's refresh rate.
  Reason: platform — winit offers no vsync, and only macOS 14 and later gives a screen's display link.
  Affect: an animation's frames come at the display's rate but not in step with its refresh.

## view.rs / rasterizer.rs

- Change: the valo context hides missing glyphs.
  Reason: platform — Valo paints `.notdef` tofu unless asked not to.
  Affect: a character with no face occupies layout space and draws nothing.

## keyboard.rs / keys.rs

- Change: the embedder builds `KeyData` from winit's W3C key codes with Flutter's own tables, deriving the logical key as Flutter's web engine does.
  Reason: platform — winit reports the W3C values Flutter's tables are keyed by, not the system's own key codes.
  Affect: physical keys are the ones a Flutter app expects on any keyboard layout.

- Change: the embedder remembers the logical key each held physical key went down with and reports that one on its repeats and its up.
  Reason: platform — winit re-derives the logical key from the current modifier state on every event.
  Affect: a release after a modifier change still reports the key the press did, so `HardwareKeyboard`'s pressed-key sets stay consistent.

- Change: a key Flutter's tables have no code for sends no event, where Flutter's engine makes up a code for it.
  Reason: platform — Flutter's tables have no plane for winit, and a made-up code could collide with a real key's.
  Affect: rare keys, such as F25 and up or a key winit cannot name, do nothing.

## text_input.rs / ime.rs

- Change: a view's text input runs on winit's input method, and the embedder turns its composing text and commits into editing values.
  Reason: platform — winit's input method reports composing text and commits, not Flutter's editing state.
  Affect: a field hears composing text and commits, and the input method turning off closes its connection.

- Change: the embedder answers no to `handles_editing_keys` and does not forward the editing commands macOS names for a key.
  Reason: platform — winit keeps only the plain key press and drops the command macOS names for it.
  Affect: text fields are edited by the framework's own key bindings, so a user's own key bindings and macOS's Control-key editing bindings do not reach them.

## mouse_cursor.rs

- Change: the embedder's `MouseCursor` capability sets the winit cursor icon on the window the pointer was last seen in; `None` hides the cursor and kinds winit lacks show the arrow.
  Reason: platform — winit has one cursor per window where Flutter's engine maps cursor kinds per embedder.
  Affect: a `MouseRegion` cursor shows on hover, the device id is ignored (one mouse), and an unsupported kind is silently the arrow.

## clipboard.rs

- Change: the clipboard methods talk to the OS pasteboard through arboard, where Flutter's `Clipboard` messages go to the engine.
  Reason: platform — winit does not own the pasteboard, so the embedder implements the channel.
  Affect: copy and paste work with other apps on the desktops, and an embedder that cannot open the pasteboard reads an empty clipboard rather than failing.

## platform.rs

- Change: the embedder offers neither the `Haptics` nor the `SystemChrome` capability, so both answer `None`.
  Reason: platform — a desktop machine has no haptic engine and no status bar, and Flutter's own Linux and Windows embedders answer both with not-implemented.
  Affect: `HapticFeedback` calls are silent and the status-bar style a `CupertinoNavigationBar` asks for shows nowhere; the same calls work on a phone embedder.

- Change: `Platform::font_source` is `SystemFontSource::platform()` over the OS font API.
  Reason: platform — Flutter's engine finds platform fonts itself; valo needs a `FontSource`.
  Affect: text uses installed fonts, the Cupertino system-font names resolve to SF, and an uninstalled family falls back to whatever the OS picks for the character.

## os/macos/composition/ → the macOS embedder's platform views (`FlutterSurfaceManager`, `FlutterMutatorView`)

- Change: each native surface is a Core Animation layer beneath the framework's layer, showing its producer's buffer directly.
  Reason: platform — Core Animation composites the buffers it's given, so a producer's frame needs no copy and no framework frame.
  Affect: framework content can sit above a surface but not beneath it.

- Change: a buffer is released once the system says it's no longer in use, checked at each vsync.
  Reason: platform — Core Animation gives no signal when it lets go of a buffer.
  Affect: a producer needs three buffers, not two.

- Change: a surface's vsync comes from its window's screen, and stops while nobody can see the window.
  Reason: platform — vsync belongs to a display, and macOS tells a window's visibility only through notices.
  Affect: a program in a hidden or fully covered window draws nothing until part of it shows.

## view_focus.rs → native view focus

- Change: a window's focus change reaches the framework with no traversal direction.
  Reason: platform — winit reports window focus without a direction.
  Affect: focus arriving from the system always reads as `ViewFocusDirection::Undefined`.

## drag_drop.rs → the macOS embedder's `draggingEntered:`/`performDragOperation:`

- Change: files dragged over a window reach the app as one report per phase, with no position.
  Reason: platform — winit reports each dragged file on its own and never says where the drag is.
  Affect: an app hears a drag enter, drop and leave but never move, and must treat the whole window as the target.

## system_requests.rs / os/macos/app_delegate.rs → the macOS embedder's `FlutterAppDelegate`

- Change: on macOS, files the system asks the app to open go to the handler the app sets through `Platform::file_opening`, and a reopen, such as a click on the Dock icon, to the one it sets through `WinitPlatform::macos`, where a Flutter app answers them in its own application delegate.
  Reason: platform — winit owns the application delegate, so an app cannot write its own.
  Affect: files that arrive before the app sets its handler wait for it, and a reopen with no handler set is dropped.

## macos.rs / os/macos → what only macOS offers beyond `Platform`

- Change: `WinitPlatform::macos` slides a window in and hides the app, neither of which Flutter's framework offers.
  Reason: platform — an app that places a window against the screen, such as a panel above its Dock icon, needs them, and `Platform` stays Flutter's.
  Affect: an app reaches them only on macOS; on other systems `macos()` is `None`, as `Platform::screens` is.

## Deferred

- The lifecycle state on iOS, Windows and Linux, where the framework hears none. Trigger: an app there that must pause in the background.
- Asking the framework before the last window closes, on Windows and Linux. Trigger: an app there that must refuse to close.
- Files opened from the system on Windows and Linux. Trigger: an app that wants them through `Platform::file_opening` there too.
- The layout keymap for character keys, so a shifted symbol reports `!` where Flutter reports `digit1`. Trigger: a shortcut that matches a shifted symbol by logical key.
- Dead keys and `Key::Unidentified`. Trigger: composing input, or a key of the system's that winit cannot name.
- Synthesizing the key ups a focus change swallowed. Trigger: `HardwareKeyboard::sync_keyboard_state` gaining a way to ask the system.
