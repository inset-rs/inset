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

## clipboard/, os/macos/pasteboard/, os/ios/pasteboard/

- Change: on macOS the clipboard is the general pasteboard, its items in every type it carries, given or promised, files and promised files, a promised file written at the receiver's URL as its producer makes its bytes and removed when it fails, a sensitive copy marked concealed, one for this device only written for this host only, and one that expires cleared by the embedder; elsewhere it is plain text through arboard, every other form refused.
  Reason: platform — Flutter's desktop engines carry plain text over the platform channel, and winit owns no clipboard, so the embedder reaches each system's itself.
  Affect: on macOS an app copies and pastes images, HTML and files with other apps, and on Windows and Linux text alone.

- Change: on iOS the clipboard is UIKit's general pasteboard, its items in every type it carries, a copy with every form given written whole, and one with a promised form or file written as item providers, a promised file written by its producer into a temporary file the provider hands over, one for this device only written local only, and one that expires carrying its expiration date.
  Reason: platform — Flutter's iOS engine carries plain text over the platform channel, and arboard has no iOS code.
  Affect: an iOS app copies and pastes images, HTML and files with other apps, and its expired copy is cleared by the system after the app has quit too.

- Change: on iOS listing the pasteboard reads only its types, and a read loads the form through the item's provider, off the main thread.
  Reason: platform — iOS shows its paste alert when an app reads another app's content, and the app's own promised form is produced on the main thread.
  Affect: the alert shows when the app reads, never when it lists, and a read of the app's own promise never stalls it.

- Change: on Windows and Linux a sensitive copy is kept out of the clipboard's history, and on Windows out of its monitoring, a copy for this device only is kept off Windows' cloud clipboard, and one that expires is cleared by the main loop, which wakes for it as it wakes for the app's timers.
  Reason: platform — arboard carries those of the options the systems have, and neither system has an expiry of its own.
  Affect: a password copied stays out of Windows' clipboard history, and a copy whose app quits before its time stays on the clipboard.

- Change: the app hears the clipboard changed when it becomes active and when one of its windows takes the focus, if another writer moved the system's count of changes (on Linux, changed the text), when iOS says the pasteboard changed, and when a copy of its own expired and was cleared.
  Reason: platform — no desktop system tells an application that another one wrote the clipboard, and iOS tells only an app that is active.
  Affect: a toolbar's Paste button follows a copy made in another app once the user comes back to this one.

- Change: on macOS a promised form is produced while the paste waits, on the main thread, for up to ten seconds.
  Reason: platform — AppKit asks the pasteboard's owner for promised data synchronously, inside the paste.
  Affect: a promise whose producer waits on the app's own loop is never answered, and the app stalls each time someone pastes it.

## platform.rs

- Change: the embedder offers neither the `Haptics` nor the `SystemChrome` capability, so both answer `None`.
  Reason: platform — a desktop machine has no haptic engine and no status bar, and Flutter's own Linux and Windows embedders answer both with not-implemented.
  Affect: `HapticFeedback` calls are silent and the status-bar style a `CupertinoNavigationBar` asks for shows nowhere; the same calls work on a phone embedder.

- Change: `Platform::font_source` is `SystemFontSource::platform()` over the OS font API.
  Reason: platform — Flutter's engine finds platform fonts itself; valo needs a `FontSource`.
  Affect: text uses installed fonts, the Cupertino system-font names resolve to SF, and an uninstalled family falls back to whatever the OS picks for the character.

## os/apple/composition/ → the macOS and iOS embedders' platform views (`FlutterSurfaceManager`, `FlutterMutatorView`, `FlutterPlatformViewsController`)

- Change: each native surface is a Core Animation layer beneath the framework's layer, showing its producer's buffer directly, on macOS and iOS alike.
  Reason: platform — Core Animation composites the buffers it's given, so a producer's frame needs no copy and no framework frame.
  Affect: framework content can sit above a surface but not beneath it.

- Change: a surface's buffer and its children sit in one layer scaled by a transform, the children beneath the buffer, and a present shows a surface frame's buffer and places its children in one transaction.
  Reason: platform — a program whose picture is itself a surface in another app's window places its own surfaces in that picture, and Core Animation moves, clips and scales a layer's sublayers with it.
  Affect: a child moves, is cut and shrinks with its parent, and shows through its parent's clear pixels.

- Change: a buffer is released once the system says it's no longer in use, checked at the surface's next present and when the surface goes.
  Reason: platform — Core Animation gives no signal when it lets go of a buffer.
  Affect: a producer never waits for a buffer to come back, but draws into another when none is known free, as Flutter's `FlutterSurfaceManager` does.

## media.rs, os/apple/media/ → `video_player_avfoundation` and `camera_avfoundation`

- Change: a player is an `AVPlayer` whose state is read from it on the main thread when the app hears it changed, its changes told by key-value observing, a periodic time observer every half second and its end-of-item notification, where Flutter's plugin copies events over an event channel.
  Reason: platform — AVFoundation changes a player's state on the main queue, where the app reads it.
  Affect: the app's player hears its state change at once on load, play, pause, seek and end, and its position every half second while it plays.

- Change: a player shown on a surface hangs an `AVPlayerLayer` in the surface's own layers, and one read as frames adds an `AVPlayerItemVideoOutput` of BGRA `IOSurface`-backed pixel buffers, where Flutter's plugin copies every frame into a texture.
  Reason: platform — Core Animation clips and scales a layer with the surface it hangs in, and a pixel buffer backed by an `IOSurface` reaches the GPU with no copy.
  Affect: a video moves, clips and shrinks with the framework's frame, and a frame reader hands out the newest picture only.

- Change: a camera session is an `AVCaptureSession` configured and run on a queue of its own, asking for the camera, and for the microphone where a recording wants sound, at its start, with its preview a layer hung in a surface and its frames, photos and recordings the system's video data, photo and movie file outputs.
  Reason: platform — starting a session blocks, and each output is AVFoundation's own.
  Affect: a session reports starting, running, denied or failed, photos arrive as JPEG and recordings as QuickTime movies.

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

## system_requests.rs / launch.rs / os/macos/app_delegate.rs / os/ios/open_requests.rs → the macOS embedder's `FlutterAppDelegate`, iOS's `FlutterAppDelegate` and `FlutterSceneDelegate`

- Change: the files and links the system asks the app to open reach the client as `EmbedderClient::open_requested`, and a reopen, such as a click on the Dock icon, the handler the app sets through `WinitPlatform::macos`, where a Flutter app answers them in its own application delegate: macOS's `application:openURLs:`, a file URL as a path and any other as a link, at launch and while the app runs.
  Reason: platform — winit owns the application delegate, so an app cannot write its own.
  Affect: the launch's request waits in the `App` for the app's handler, and a reopen with no handler set is dropped.

- Change: on Windows and Linux the embedder reads no launch arguments: the app reads its own command line, as a Flutter app does in `main`, and hands what it decides are files to `App::open_requested`.
  Reason: platform — the system starts the app with strings on its command line, and which of them are files the app chose when it registered its file types; Windows and Linux start a new process with what it opens, so a running app hears a later launch only through `inset-single-instance`.
  Affect: an app's own words on its command line, such as a subcommand, are never taken for files to open.

- Change: on iOS a link or a file opened with the app comes through the application delegate's `application:openURL:options:`, where Flutter's scene-based apps hear `scene:openURLContexts:`; the launch's URL is read from the launch options when the application finishes launching, and a delegate of the embedder's is set then. A file URL's security-scoped access starts as it arrives and is kept for the app's life, as a picked document's is.
  Reason: platform — winit gives `UIApplicationMain` no delegate class and runs no scenes, so the scene delegate's calls never come; the application delegate's are what UIKit sends an app without scenes.
  Affect: an app opened with a file or a link on iOS hears it as on every other system; once winit runs scenes, the delegate's calls move to the scene's.

## menus/, popup_menus.rs, main_loop/menus.rs, os/macos/menus/, os/windows/menus/, os/linux/status_notifier_items/ → the macOS embedder's `FlutterMenuPlugin`, and the system menus Flutter does not reach

- Change: on macOS the menu bar shows the system's standard menus, the application's (About, Services, Hide, Hide Others, Show All, Quit), Edit and Window, until the app sets its own, in place of winit's default menu, which is left out; Flutter's macOS template shows the menu in the app's `MainMenu.xib`.
  Reason: platform — an app here has no nib, and winit's default menu has no Edit menu for AppKit to recognise.
  Affect: an app that sets no menu bar quits with ⌘Q through its quit flow and copies with ⌘C through `clipboard_event`.

- Change: on macOS a key goes to the framework before the menu bar's key equivalents, and a key the framework leaves goes to the menu bar after, through the menu bar's own `performKeyEquivalent:` declining a key while one of the app's windows is key and the main loop offering it again; Flutter's macOS embedder redispatches a key its framework left, from its view's `performKeyEquivalent:`.
  Reason: platform — winit's view takes no key equivalent, so AppKit would give each key to the menu first, and the menu bar is an `NSMenu` subclass of the embedder's, which needs no change to winit's classes.
  Affect: a shortcut the app handles wins over a menu item with the same key, and a key while another window is key, such as the About panel, goes to the menu at once.

- Change: on macOS each key-down reported is kept, the 16 latest for at most 3 seconds, so a key the app claimed and hands back later (`MenuBar::perform_key_equivalent`) goes to the menu bar as the event AppKit delivered, through the same redispatch; a key no longer kept answers no.
  Reason: platform — an app that waits on another thread, as WaOS does for a program, answers after AppKit's call has returned, and Chrome keeps its event for the same reason; an event rebuilt from the key's data would have no modifiers and no keyboard layout, and could choose another item.
  Affect: a key handed back within seconds chooses its item as if the app had left it at once, and one handed back later does nothing.

- Change: a menu item's action goes to the main loop as an event, and the app hears it on a turn of its own, where `FlutterMenuPlugin` sends `Menu.selectedCallback` from the action.
  Reason: platform — AppKit sends the action from inside its own dispatch, where the client may already be in a call.
  Affect: the app hears a choice after the menu has closed, never inside a call of its own.

- Change: Edit's provided items send the focused view, the one whose window has the focus, copy, cut and paste as `clipboard_event` and undo, redo, select all and delete as `edit_command`; Flutter's macOS embedder sends the selectors to its text input plugin.
  Reason: platform — the host's copy and paste go through the clipboard's events, and any focused widget may act on them, not only a text field.
  Affect: Edit works on a selectable text as on a field, and with no focused view nothing happens.

- Change: on macOS the Dock icon's menu is answered from `applicationDockMenu:`, which the embedder adds to winit's application delegate class, as it adds its other delegate methods; status items are `NSStatusItem`s whose button reports a click as `Activated`, a right click or a Control-click as `ContextMenuRequested` and a double click as `DoubleClicked`, at the pointer, and whose attached menu is the item's own menu.
  Reason: platform — winit owns the application delegate, and AppKit opens a status item's menu by itself on a click.
  Affect: a status item with an attached menu reports no click, as AppKit sends it none.

- Change: on Windows a menu at a point is `TrackPopupMenuEx`, opened from a hidden window of the embedder's once the framework's call has returned, and a tray icon is `Shell_NotifyIcon`'s at version 4, reporting a click or a key as `Activated`, a right click or the menu key as `ContextMenuRequested`, a middle click as `SecondaryActivated` and a double click as `DoubleClicked`, at the point Windows gives in logical pixels; its menu is opened at the pointer by the app.
  Reason: platform — a menu's modal loop must run outside every call into the app, and the notification area tells an icon's events to a window, which also hears the taskbar come back after Explorer restarts.
  Affect: the icons come back after Explorer restarts, an icon added while the taskbar is not there is answered as an error, and an app on Windows opens an icon's menu when it hears `ContextMenuRequested`.

- Change: on Linux a tray icon is a StatusNotifierItem with its DBus menu, over zbus on the session bus, each icon a connection of its own answered once the watcher has registered it, or with the bus's error, and the tray is `None` where no StatusNotifierWatcher is on the bus at start; there is no menu at a point and no menu the app opens at an icon.
  Reason: platform — the desktop draws a StatusNotifierItem's menu and decides when to open it, and Linux's toolkits draw their own menus at a point.
  Affect: a click reaches the app as the desktop reports it, `Activate`, `SecondaryActivate`, `ContextMenu` or a vertical scroll, at the desktop's position, which Wayland often leaves at zero.

## file_dialogs.rs, os/*/file_dialogs.rs → `file_selector`'s macOS, Windows, Linux and iOS implementations

- Change: on macOS the dialogs are `NSOpenPanel` and `NSSavePanel`, begun without blocking, as a sheet on the parent view's window where there is one, as `file_selector_macos` shows them; a type's extensions and MIME types become the uniform types the panel enables, with no label, since a label needs an accessory view.
  Reason: platform — a sheet keeps the dialog with its window and the app's loop running.
  Affect: a dialog with a parent slides out of its window; every pick is a path the app may read and write, as AppKit grants a sandboxed app.

- Change: on Windows the dialogs are the shell's `IFileOpenDialog` and `IFileSaveDialog`, owned by the parent's window, shown from the embedder's hidden window once the framework's call has returned, as its menus are; a type's extensions become a filter under its label, and its MIME types filter nothing.
  Reason: platform — a dialog runs its own modal loop, which must run outside every call into the app.
  Affect: the app goes on drawing while the dialog is open, and a type with MIME types alone shows every file.

- Change: on Linux the dialogs are the desktop portal's `FileChooser` over the session bus, asked from a thread of its own that waits for the portal's `Response`, answering its `file://` URIs as paths; the dialogs are `None` where the bus has no portal, running or activatable. A dialog is modal to its parent on X11 only, since naming a Wayland window would need it exported, which winit does not.
  Reason: platform — the portal is the desktop's own dialog, on GNOME, KDE and in a Flatpak alike, without linking GTK or Qt.
  Affect: the dialog is the desktop's, and on Wayland it stands apart from its window.

- Change: on iOS the dialogs are `UIDocumentPickerViewController`, presented by the parent view's view controller or the screen view's, opening in place; each picked URL's security-scoped access starts when it is answered and is kept for the app's life, since a path carries no lifetime to end it by. A save makes an empty file under the suggested name in a folder of its own in the temporary folder and has the picker export it, answering the file where the user put it.
  Reason: platform — iOS has no dialog that answers a file yet to be written, and a picked document outside the sandbox is reached only while its access lasts.
  Affect: a picked path stays readable and writable until the app quits; an app picking without end would run out of the kernel's sandbox extensions.

## os/*/opener.rs, os/linux/portal.rs → `url_launcher`'s and Electron's `shell` implementations

- Change: on macOS the opener is `NSWorkspace`: a link or a file opens in the app Launch Services gives it, asked first through `URLForApplicationToOpenURL:`, which also answers `can_open_link` for every scheme; a file is revealed with `activateFileViewerSelectingURLs:`.
  Reason: platform — `openURL:` answers only whether it handed the URL on, so no app is told from a failure by asking first.
  Affect: opening answers `NoApp` for a scheme or a kind no app takes, at once.

- Change: on Windows the opener is the shell: `ShellExecuteW`'s `open` of the link or the path, run on a thread of its own in a COM apartment of its own, `SE_ERR_NOASSOC` being no app; a file is revealed with `SHOpenFolderAndSelectItems`, and `can_open_link` is whether the registry associates an executable with the scheme (`AssocQueryStringW`).
  Reason: platform — the shell may wait on the app it starts, and asks its caller for an apartment, which the main thread's loop must not be held for.
  Affect: opening answers once the shell has handed the link or file on.

- Change: on Linux the opener is the desktop portal's `OpenURI`, over the session bus, as the file dialogs are: `OpenURI` for a link, `OpenFile` for a file and `OpenDirectory` to reveal one, each handed the file's descriptor and asked from a thread of its own that waits for the portal's `Response`; `can_open_link` is `None`, since the portal never says; the opener is `None` where the bus has no portal.
  Reason: platform — the portal is the desktop's own way, in a Flatpak too, and reveals a file the way its file manager does.
  Affect: the user may be asked to choose an app, and a choice cancelled answers a failure.

- Change: on iOS the opener is `UIApplication`'s `openURL:options:completionHandler:` for a link, its `false` being no app, and `UIDocumentInteractionController`'s Open In menu for a file, presented from the first window's view controller, its `false` being no app; revealing fails, and `can_open_link` asks `canOpenURL:` only for a scheme the app's `LSApplicationQueriesSchemes` declares, `None` for another.
  Reason: platform — iOS shares a file with another app only through a controller the user chooses the app in, and answers `canOpenURL:` falsely for a scheme the app did not declare.
  Affect: opening a file on iOS shows the user the apps that take it.

## notifications.rs, os/*/notifications → `flutter_local_notifications`' macOS, iOS, Windows and Linux implementations

- Change: on macOS and iOS the notifications are `UNUserNotificationCenter`'s, and they are `None` for a process outside an application bundle, as `cargo run` starts one: a notification is a request under the app's id, its actions a category named after the actions themselves, its reply a `UNTextInputNotificationAction` sent with a button of its label, its image an attachment of a PNG file the embedder writes, and a click comes to the center's delegate, set before the loop runs.
  Reason: platform — UserNotifications raises an exception, which aborts the process, when a process without a bundle touches the center, and it reports the click that launched the app only to a delegate set before the launch finishes.
  Affect: an app has notifications once it runs from its `.app`, as `cargo inset build` makes it, and hears the click that launched it like any other.

- Change: on macOS and iOS `permission()` answers from `getNotificationSettings`, a provisional or an ephemeral authorization being `Granted`, and its `request` asks with `requestAuthorizationWithOptions:` (alerts and sounds); a post reads the settings first and answers `Denied` without touching the center where the app is not let notify.
  Reason: platform — the center's `add` never asks, fails with `notificationsNotAllowed` for an app not let notify, and keeps no category for an app the user was never asked about.
  Affect: an app asks before its first post, which otherwise answers `Denied` and shows nothing.

- Change: on macOS and iOS each action brings the app forward (`UNNotificationActionOptionForeground`), and a notification shows while the app is in front, as a banner and in the list, with its sound unless it is silent, where `flutter_local_notifications` leaves both to the app's settings.
  Reason: platform — the center shows nothing for an app in front unless its delegate asks, and a click on an action is the app's to act on, as a click on the notification is.
  Affect: an app's notifications look the same in front and behind, and an action opens the app, as on Android and Windows.

- Change: on macOS and iOS the categories the app's notifications need are kept by the embedder, and each `setNotificationCategories:` is the center's own set joined with them, followed by a `getNotificationCategories` the request waits for; a category's id is a hash of its actions, the same in every run.
  Reason: platform — the center replaces its whole set at each call, keeps it across runs for scheduled notifications, and shows a request added before its category is in place without its actions.
  Affect: a scheduled notification keeps its buttons after the app restarts, and the center's categories grow only with the distinct sets of actions the app shows.

- Change: on macOS and iOS a scheduled notification is a calendar trigger at the time, rounded up to the whole second, in a Gregorian calendar at GMT; a time less than a second away is shown at once.
  Reason: platform — a `SystemTime` is the time a clock reads, which a calendar trigger follows when the clock is set, where a time-interval trigger counts from the request.
  Affect: a reminder fires when the clock reads its time, wherever the user travels.

- Change: on Windows the notifications are WinRT's toasts (`ToastNotificationManager`): a notification is a `ToastGeneric` toast tagged with its id, or a hash of an id longer than 64 characters, in the group `inset`, which a later toast of its tag replaces and `remove` takes out of the history and the schedule; its image is the app's logo on the toast, a PNG file the embedder writes in the temporary folder and names by its pixels; a reply action is a text box with a send button beside it; a silent one has `<audio silent="true"/>`.
  Reason: platform — the platform replaces and removes a toast by its tag and group, takes a tag of 64 characters at most, and reads a toast's image from a file whenever it draws the toast, in the Action Center too.
  Affect: an app's notifications replace and go as elsewhere whatever the length of their ids, and the temporary folder keeps one file for each image shown.

- Change: on Windows the notifications are `None` for an unpackaged process that set no AppUserModelID of its own (`SetCurrentProcessExplicitAppUserModelID`) before the embedder runs; a packaged app is known by its package; the embedder registers nothing.
  Reason: platform — the platform shows the toasts of an app it knows: a package, or an AppUserModelID that a Start-menu shortcut carries or that the key `HKCU\Software\Classes\AppUserModelId\<id>` registers with its `DisplayName` and `IconUri`, which are the app's installer's to write.
  Affect: `cargo run` has no notifications on Windows, and an installed app has them once it sets its id at start and its installer registers that id.

- Change: on Windows a click comes through the app's toast activator where its registration's `CustomActivator` names one: the embedder registers that COM class (`INotificationActivationCallback`) before the loop runs, from a thread of its own, and the click that started the app comes through it too; without one, a click comes through the shown toast's own `Activated` event while the app runs.
  Reason: platform — the platform starts an app for a click, or reaches a running one from the Action Center, only through a COM class registered as the app's local server (`HKCU\Software\Classes\CLSID\{clsid}\LocalServer32`), and a toast's events live only in the process that showed it.
  Affect: an app registered with an activator hears every click, at launch too; one without hears clicks on the toasts it showed while it runs, and none on a scheduled one or after it quit.

- Change: on Windows `permission()` is the app's switch in Settings (`ToastNotifier.Setting`), `request()` asks nothing, and a post answers `Denied` while the switch is off.
  Reason: platform — Windows asks the user no question; they turn an app's notifications off in Settings.
  Affect: an app learns that its notifications are off, and its first post shows without a question.

- Change: on Windows a scheduled notification is a `ScheduledToastNotification` at the time, a time less than a second away is shown at once, and a post or a schedule under an id first takes the toast scheduled under it out of the schedule.
  Reason: platform — the schedule refuses a time it has passed, and keeps two toasts of one tag rather than replacing one.
  Affect: a reminder shows at its time whether the app runs, and a later post under its id replaces it.

- Change: on Linux the notifications are the desktop's notification server, `org.freedesktop.Notifications` on the session bus: the app's id is mapped to the server's number, which a later post passes as `replaces_id` and a remove closes; a button is one of the server's action keys and a click on the notification itself the `default` one, heard as `ActionInvoked` while the app runs; the notifications are `None` where the bus has no server, running or one it starts.
  Reason: platform — every desktop's server speaks the specification, numbers the notifications itself, and keeps no click for an app that is not running.
  Affect: a click on a notification after the app quit reaches nothing, where on the other systems it starts the app.

- Change: on Linux `replies()` is KDE's `inline-reply` where the server's capabilities advertise it: a notification's first reply action is the server's `inline-reply` action, its placeholder the `x-kde-reply-placeholder-text` hint, and the typed text comes as `NotificationReplied`; a later reply action is a button.
  Reason: platform — the specification has no reply, and KDE's extension takes one field a notification.
  Affect: an app offers a reply on KDE Plasma, and has no `replies()` on GNOME and the others.

- Change: on Linux `permission()` and `scheduling()` are `None`.
  Reason: platform — the specification keeps no permission for an app and holds no notification for later.
  Affect: an app posts without asking, and a reminder for later is the app's to keep while it runs.

- Change: on Linux a body is escaped where the server reads markup (`body-markup`), the image is the `image-data` hint of its pixels, a silent notification has the `suppress-sound` hint, and the `desktop-entry` hint names the app's desktop file where the system says which it is: a Flatpak's id, or the file GLib's launcher started this process from.
  Reason: platform — a server that reads markup would read the app's `<` and `&` as markup, and a server finds an app's name and icon by its desktop file, which the embedder cannot otherwise know.
  Affect: a body shows as the app wrote it, and the desktop shows the app's own name and icon where it knows the desktop file, its process's elsewhere.

## macos.rs / os/macos → what only macOS offers beyond `Platform`

- Change: `WinitPlatform::macos` slides a window in and hides the app, neither of which Flutter's framework offers.
  Reason: platform — an app that places a window against the screen, such as a panel above its Dock icon, needs them, and `Platform` stays Flutter's.
  Affect: an app reaches them only on macOS; on other systems `macos()` is `None`.

## Deferred

- The lifecycle state on iOS, Windows and Linux, where the framework hears none. Trigger: an app there that must pause in the background.
- The fallback languages Flutter's Windows and Linux embedders add after the user's: Windows's own (`MUI_UI_FALLBACK`), and GLib's, such as `en` after `en_US`, and `C`. Trigger: an app whose locale resolution needs a fallback the user did not set.
- Hearing the user's languages change on Windows (`WM_SETTINGCHANGE`), which winit does not report and Flutter's Windows embedder does not hear either. Trigger: an app there that must follow a change without a restart.
- Letting a logout or shutdown go on once the app agrees, through `NSTerminateLater`: the delegate cancels it at once, as Flutter's does, so an Inset app stops every logout. Trigger: an app that must not stop a logout it agrees to; AppKit's loop while it waits needs an experiment first.
- The layout keymap for character keys, so a shifted symbol reports `!` where Flutter reports `digit1`. Trigger: a shortcut that matches a shifted symbol by logical key.
- Dead keys and `Key::Unidentified`. Trigger: composing input, or a key of the system's that winit cannot name.
- Synthesizing the key ups a focus change swallowed. Trigger: `HardwareKeyboard::sync_keyboard_state` gaining a way to ask the system.
- Registering a StatusNotifierItem again when the watcher comes back on the bus, and a tray that appears after the app started. Trigger: a desktop whose panel restarts.
- Bringing the app forward on a click on Linux, with the activation token a server sends before `ActionInvoked` (`ActivationToken`, specification 1.2): winit 0.30 activates no window already shown with a token. Trigger: winit taking an `xdg_activation` token for a shown window.
- A menu item's icon at the screen's scale on Windows, where it shows at its own size in pixels. Trigger: a menu with icons on a display scaled above 100%.
- Translating the labels of macOS's and Windows' provided items, which are English. Trigger: an app localized beyond English.
