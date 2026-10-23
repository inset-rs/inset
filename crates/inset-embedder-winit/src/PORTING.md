# inset-embedder-winit/src

Embedder implementation references: Flutter's C++ engine under `flutter/engine/src/flutter`.

## Identical

- app_exit.rs → how Flutter's desktop embedders quit for a required `System.exitApplication`, and hear the framework's reply to `System.requestAppExit`
- lifecycle.rs, os/macos/app_lifecycle.rs → the macOS embedder's lifecycle states (`FlutterEngine`)
- os/macos/app_delegate.rs, quitting → the macOS embedder's `FlutterAppDelegate`, which asks the framework before the app quits
- launch.rs, main_loop/ → the engine's start (`Shell` adds the implicit view with empty metrics): where the system's window is the screen, the app starts with the screen view at no size, and its window and surface come after
- platform.rs, main_loop/windows.rs → the embedders' implicit view, here the screen view, which the framework never hears removed: it outlives its window and draws nowhere once the window closes
- main_loop/windows.rs → the engine's window close request, which goes to the framework and closes nothing until the framework closes the window
- main_loop/windows.rs, view.rs, rasterizer.rs → the shell's surfaces (`Shell::OnPlatformViewCreated`, `OnPlatformViewDestroyed`): the view stays while the system has taken its surface, and each surface given gets the view's last scene, as Flutter's rasterizer keeps each view's last layer tree, and a frame
- touch.rs → the iOS embedder's touches, one pointer per finger, added before its down and removed after its up
- screens.rs, os/macos/screens.rs, os/macos/system_changes.rs, main_loop/app.rs → the macOS embedder's displays (`updateDisplayConfig`), every `NSScreen` numbered by its `NSScreenNumber`, sent again at `NSApplicationDidChangeScreenParametersNotification`, and a window's `display_id`, its screen's number, sent again when the window changes screen
- locales.rs, os/macos/system_changes.rs → the macOS embedder's locales (`sendUserLocales`), its preferred languages, sent again at `NSCurrentLocaleDidChangeNotification`; the Windows and Linux embedders', the user's languages, sent at start only

## lib.rs / main_loop/

- Change: a desktop app has no view until it asks for one: the host makes a window for each request and adds its view carrying the request's id, where Flutter's runners make the implicit view's window and only its multi-window apps make their own (`[win32] Multi-window should never have implicit view`).
  Reason: platform — the app owns its windows and decides every window's close, the last one's included, so the host makes no window the app did not ask for and has no rule for the last one.
  Affect: `WinitEmbedder` has no settings; `run_app` asks for the app's window, and what the last window's closing means is the app's to decide.

- Change: on macOS every `terminate:` asks the framework until the app has decided to quit, where Flutter's termination handler marks the quit as under way when it asks, which lets a second Cmd-Q through.
  Reason: platform — the host reports every request to quit to the framework, which decides each, such as quitting only at a second Cmd-Q.
  Affect: an app whose exit listeners never answer is not quit by pressing Cmd-Q again.

- Change: on macOS a panic inside a call from AppKit aborts the process once the panic hook has printed it.
  Reason: platform — a panic cannot unwind through AppKit's frames.
  Affect: such a panic ends the process rather than coming out of `run`.

## frame_scheduler.rs / vsync_source.rs / vsync_waiter.rs / os/macos/vsync_waiter.rs

- Change: without a screen's display link, on macOS before 14 and on the other desktops, frames follow a timer at the display's refresh rate.
  Reason: platform — winit offers no vsync, and only macOS 14 and later gives a screen's display link.
  Affect: an animation's frames come at the display's rate but not in step with its refresh.

- Change: the platform hands out Flutter's `VsyncWaiter` for any screen, served at the vsyncs the frames of the views on that screen are drawn at, whether or not a window on it is in sight.
  Reason: platform — Flutter's waiter is the engine's own, but a producer feeding a native surface, or a host drawing for programs, paces by the same refreshes from threads of its own.
  Affect: such a producer gets one call per wait from the screen's one source, and decides for itself whether to draw for a window nobody sees.

- Change: on macOS 14 and later the waiter serves each wait at the tick of its screen's display link, in the run loop's common modes, a wait from another thread starting the ticks through the main dispatch queue, as Flutter's `FlutterVSyncWaiter` hands its ticks to `FlutterRunLoop`, not through the main loop as the framework's own frames are.
  Reason: platform — the display links and the main queue run in every mode of the run loop, while a turn of the main loop waits for winit, which calls the embedder nothing while AppKit runs a loop of its own inside one of winit's calls.
  Affect: a producer is served at the tick itself, also while the main loop is held up.

## screens.rs / os/macos/screens.rs

- Change: on macOS a screen's refresh rate is its `NSScreen.maximumFramesPerSecond`, where Flutter's macOS embedder rounds the nominal rate of a display link it makes for the screen.
  Reason: platform — `NSScreen` answers the rate itself from macOS 12, with no display link to make.
  Affect: a screen that varies its rate reports the fastest, and before macOS 12 a screen reports none.

- Change: on Windows, Linux and iOS the screens are winit's monitors, listed through the app's first window, numbered in the order first seen, with no visible frame of their own and no notice when they change, where Flutter's Windows embedder lists `HMONITOR`s again at `WM_DISPLAYCHANGE` and its iOS embedder reports the main screen alone.
  Reason: platform — winit lists monitors only through a window or inside its own calls, gives them no number, knows no taskbar, and reports no monitor coming or going.
  Affect: an app there has no screens until its first window opens, which it hears as that view coming; a visible frame includes the taskbar; a monitor plugged in shows in the list the next time it is read, and a window moved onto it is reported as a metrics change.

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

- Change: a buffer is released once the system says it's no longer in use, checked at the surface's next present and when the surface goes.
  Reason: platform — Core Animation gives no signal when it lets go of a buffer.
  Affect: a producer never waits for a buffer to come back, but draws into another when none is known free, as Flutter's `FlutterSurfaceManager` does.

## host_window.rs / main_loop/windows.rs / os/macos/visibility.rs → Flutter's desktop window controllers (`_window_macos.dart`, `_window_win32.dart`, `_window_linux.dart`)

- Change: a window answers whether it is minimized and whether it is occluded, and the app hears each time either changes or the window moves, read on macOS from the window and its notices and elsewhere from winit's events, where Flutter's desktop embedders notify a window's controller at every resize and activation on Windows and at every configure and state change on Linux.
  Reason: platform — winit says whether a window is covered on macOS and X11 alone and reports no minimize, which Windows tells as a resize, macOS with a notice of its own and X11 with nothing winit hears, and Wayland tells a client neither its minimize nor its position.
  Affect: on Wayland a window never reads minimized and its moves go unheard, on Windows it never reads occluded, and under X11 a minimize is heard only when the window's focus changes with it.

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
  Affect: an app reaches them only on macOS; on other systems `macos()` is `None`.

## Deferred

- The lifecycle state on iOS, Windows and Linux, where the framework hears none. Trigger: an app there that must pause in the background.
- The fallback languages Flutter's Windows and Linux embedders add after the user's: Windows's own (`MUI_UI_FALLBACK`), and GLib's, such as `en` after `en_US`, and `C`. Trigger: an app whose locale resolution needs a fallback the user did not set.
- Hearing the user's languages change on Windows (`WM_SETTINGCHANGE`), which winit does not report and Flutter's Windows embedder does not hear either. Trigger: an app there that must follow a change without a restart.
- Letting a logout or shutdown go on once the app agrees, through `NSTerminateLater`: the delegate cancels it at once, as Flutter's does, so an Inset app stops every logout. Trigger: an app that must not stop a logout it agrees to; AppKit's loop while it waits needs an experiment first.
- Files opened from the system on Windows and Linux. Trigger: an app that wants them through `Platform::file_opening` there too.
- The layout keymap for character keys, so a shifted symbol reports `!` where Flutter reports `digit1`. Trigger: a shortcut that matches a shifted symbol by logical key.
- Dead keys and `Key::Unidentified`. Trigger: composing input, or a key of the system's that winit cannot name.
- Synthesizing the key ups a focus change swallowed. Trigger: `HardwareKeyboard::sync_keyboard_state` gaining a way to ask the system.
