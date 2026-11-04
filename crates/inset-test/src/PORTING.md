# inset-test/src
No Flutter crate in this shape. Closest analogue: `flutter_test`'s `TestPlatformDispatcher` and `TestFlutterView`.

## caption_style.rs → no Flutter counterpart

- Change: `MockCaptionSettings` answers the caption style the test gives, and each change the test makes wakes the app's watch of it.
  Reason: platform — it stands in for the system's caption settings, which Flutter's tests never reach.
  Affect: a test draws subtitles in a user's style and changes that style without a host.

## clipboard.rs → `MockClipboard` in Flutter's `test/widgets/clipboard_utils.dart`

- Change: `MockClipboard` is the host's full clipboard in memory: items in several types, promises produced at each read, files and promised files, a promised file's bytes written into a folder as a host writes them where the receiver chose and removed when its producer fails, how the last copy is to be treated, and a count of its changes, where Flutter's holds one text.
  Reason: platform — it stands in for the host's clipboard, which holds every form a system's does.
  Affect: a test can check what an app copies in any type, and that a read after a change fails.

## file_dialogs.rs → no Flutter counterpart

- Change: `MockFileDialogs` answers every dialog at once with the paths or picks the test gave, or a failure, and keeps each dialog asked for; `MemoryFile` is a picked file in memory whose writes land when its writer finishes and whose URL is `memory:` and its name, and `MemoryFolder` a picked folder in memory, which lists, opens and, when writable, makes its files and folders.
  Reason: platform — it stands in for the host's dialogs, which Flutter's tests reach only as plugin channel calls.
  Affect: a test drives an app's open and save without a host, and checks what it wrote.

## media.rs → `FakeController` in `video_player`'s tests and `FakeCameraPlatform` in `camera`'s

- Change: `MockCompositor` makes composition layers that show nothing and keeps each it made, and `MockVideoFrameReaders` frame readers that never have a frame; `MockPlayers` keeps each player opened, what it was asked, the tracks selected included, and the layer or reader its picture goes to, and `MockCameras` lists the cameras the test sets, at once, waking the app's watch of the list, opens a camera unless the test denies it, lists the formats the test gave, and keeps each session started, answering its start at once, with the format picked and the outputs given, a session on a camera another still runs joining its format; each reports the state the test sets, a player's metadata and cues too, waking the app's watch of it, and a player the position the test moves it to, waking the watch only at a jump; a session's photos answer what the test gave while it runs, and its recordings answer when stopped, writing nothing.
  Reason: platform — they stand in for the host's compositor and media stack, which Flutter's tests reach only as plugin channel calls.
  Affect: a test drives a player or a camera and its views without a host, and checks what the app asked of them.

## menus.rs → no Flutter counterpart

- Change: `MockMenuBar`, `MockDockMenu`, `MockPopupMenus` and `MockTrayIcons` keep each menu set and each menu shown, and answer a shown menu with the item a test chose; the menu bar keeps the keys offered it and takes a key-down whose logical key is one of the test's shortcuts, by the key alone, since a key's data carries no modifiers; the tray answers each icon at once, offering both kinds of menu unless the test leaves one out as a system does, or refuses it.
  Reason: platform — they stand in for the host's menus, which Flutter's tests reach only as `flutter/menu` channel calls.
  Affect: a test checks the menus an app sets and drives its choices without a host.

## notifications.rs → no Flutter counterpart

- Change: `MockNotifications` shows nothing: it keeps the notifications posted and scheduled by id, a later one replacing the one under its id and `remove` taking either away, and answers as a system with no question unless the test gave it one (`asking`), whose user answers as the test set when the app asks, a post never asking and being denied until the permission is granted; replies and scheduling are there unless the test leaves them out.
  Reason: platform — it stands in for the host's notifications, which Flutter's tests reach only as `flutter_local_notifications` channel calls.
  Affect: a test checks what an app posted and scheduled, and how it meets a refusal, without a host; a click is driven through the shell's `notification_activated`.

## opener.rs → no Flutter counterpart

- Change: `MockOpener` opens nothing: it keeps each link and file opened and revealed as an `Opened`, answers them with the failure the test gave, or success, and answers `can_open_link` as the test set.
  Reason: platform — it stands in for the host's opener, which Flutter's tests reach only as `url_launcher` channel calls.
  Affect: a test checks what an app opened without a host or another app.

## platform.rs → flutter_test `TestPlatformDispatcher`

- Change: `TestPlatform` is built by a test — which host it claims to be, its views, and each capability as the test's own implementation or absent — and keeps the frame requests and wake deadlines; `TestPlatformDispatcher` wraps the real dispatcher and overrides fields one at a time.
  Reason: platform — there is no real dispatcher under a test to wrap; the `Platform` the test hands to `AppCell` is the whole host.
  Affect: a test asserts on the frame requests and `wakes` instead of pumping a real engine, background work runs at once so a checkpoint sees its result, and a capability the test did not plug in answers `None`.

- Change: a `TestPlatform` with a compositor has a view embedder, which adds a view on any layer the app asks it to, at once, as a `TestView` at the request's metrics asking what it asks of the system of the app's host, lists it until the app removes it, and numbers every request, a window's or one on a layer, from one count.
  Reason: platform — it stands in for a host that draws a view into a layer the app made, as `TestWindows` stands in for one that makes windows.
  Affect: a test drives a view the app adds on a layer without a host, and tells the framework of its coming and going itself; the metrics, sight and input the app sends it go nowhere.

## view_host.rs → flutter_test `TestTextInput`

- Change: every `TestView` has a `TestViewHost`, which records the cursors shown over the view, the focus requested for it and what its text input was asked, unless the app added the view on a layer and gave it a host of its own, where `flutter_test` logs the text input channel in `TestTextInput` and leaves the mouse cursor channel to each test's own mock.
  Reason: platform — what a view asks of the system goes to that view's host, and every view has one.
  Affect: a test reads what a view asked of the system from the view, and a text field under test talks to a text input that answers nothing and edits nothing itself.

## Deferred

- `TestFlutterView`'s per-view overrides beyond size and pixel ratio (padding, insets, gesture settings).
