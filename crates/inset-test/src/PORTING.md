# inset-test/src
No Flutter crate in this shape. Closest analogue: `flutter_test`'s `TestPlatformDispatcher` and `TestFlutterView`.

## clipboard.rs → `MockClipboard` in Flutter's `test/widgets/clipboard_utils.dart`

- Change: `MockClipboard` is the host's full clipboard in memory: items in several types, promises produced at each read, files and promised files, a promised file's bytes written into a folder as a host writes them where the receiver chose and removed when its producer fails, how the last copy is to be treated, and a count of its changes, where Flutter's holds one text.
  Reason: platform — it stands in for the host's clipboard, which holds every form a system's does.
  Affect: a test can check what an app copies in any type, and that a read after a change fails.

## file_dialogs.rs → no Flutter counterpart

- Change: `MockFileDialogs` answers every dialog at once with the paths or picks the test gave, or a failure, and keeps each dialog asked for; `MemoryFile` is a picked file in memory whose writes land when its writer finishes, and `MemoryFolder` a picked folder in memory, which lists, opens and, when writable, makes its files and folders.
  Reason: platform — it stands in for the host's dialogs, which Flutter's tests reach only as plugin channel calls.
  Affect: a test drives an app's open and save without a host, and checks what it wrote.

## menus.rs → no Flutter counterpart

- Change: `MockMenuBar`, `MockDockMenu`, `MockPopupMenus` and `MockTrayIcons` keep each menu set and each menu shown, and answer a shown menu with the item a test chose; the menu bar keeps the keys offered it and takes a key-down whose logical key is one of the test's shortcuts, by the key alone, since a key's data carries no modifiers; the tray answers each icon at once, offering both kinds of menu unless the test leaves one out as a system does, or refuses it.
  Reason: platform — they stand in for the host's menus, which Flutter's tests reach only as `flutter/menu` channel calls.
  Affect: a test checks the menus an app sets and drives its choices without a host.

## notifications.rs → no Flutter counterpart

- Change: `MockNotifications` shows nothing: it keeps the notifications posted and scheduled by id, a later one replacing the one under its id and `remove` taking either away, and answers as a system with no question unless the test gave it one (`asking`), whose first post asks and whose user answers as the test set; replies and scheduling are there unless the test leaves them out.
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

## Deferred

- `TestFlutterView`'s per-view overrides beyond size and pixel ratio (padding, insets, gesture settings).
