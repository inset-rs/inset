# inset-test/src
No Flutter crate in this shape. Closest analogue: `flutter_test`'s `TestPlatformDispatcher` and `TestFlutterView`.

## clipboard.rs → `MockClipboard` in Flutter's `test/widgets/clipboard_utils.dart`

- Change: `MockClipboard` is the host's full clipboard in memory: items in several types, promises produced at each read, files and promised files, a promised file's bytes written into a folder as a host writes them where the receiver chose and removed when its producer fails, how the last copy is to be treated, and a count of its changes, where Flutter's holds one text.
  Reason: platform — it stands in for the host's clipboard, which holds every form a system's does.
  Affect: a test can check what an app copies in any type, and that a read after a change fails.

## menus.rs → no Flutter counterpart

- Change: `MockMenuBar`, `MockDockMenu`, `MockPopupMenus` and `MockTrayIcons` keep each menu set and each menu shown, and answer a shown menu with the item a test chose; the menu bar keeps the keys offered it and takes a key-down whose logical key is one of the test's shortcuts, by the key alone, since a key's data carries no modifiers; the tray answers each icon at once, offering both kinds of menu unless the test leaves one out as a system does, or refuses it.
  Reason: platform — they stand in for the host's menus, which Flutter's tests reach only as `flutter/menu` channel calls.
  Affect: a test checks the menus an app sets and drives its choices without a host.

## platform.rs → flutter_test `TestPlatformDispatcher`

- Change: `TestPlatform` is built by a test — which host it claims to be, its views, and each capability as the test's own implementation or absent — and keeps the frame requests and wake deadlines; `TestPlatformDispatcher` wraps the real dispatcher and overrides fields one at a time.
  Reason: platform — there is no real dispatcher under a test to wrap; the `Platform` the test hands to `AppCell` is the whole host.
  Affect: a test asserts on the frame requests and `wakes` instead of pumping a real engine, background work runs at once so a checkpoint sees its result, and a capability the test did not plug in answers `None`.

## Deferred

- `TestFlutterView`'s per-view overrides beyond size and pixel ratio (padding, insets, gesture settings).
