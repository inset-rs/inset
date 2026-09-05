# reveal-services/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/services
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

Only the hardware keyboard, haptic feedback, mouse cursors, the mouse tracking annotation, state restoration, the application switcher description, the system overlay style, `TextSelection` and `SelectionChangedCause` are here; the rest of the package waits.

## Identical

- text_input.rs → text_input.dart (`SelectionChangedCause`)
- text_editing.rs → text_editing.dart

## keyboard_key.rs → keyboard_key.g.dart

The key constants and the four tables are machine-written from the Dart, as Flutter machine-writes that file from `dev/tools/gen_keycodes`.

- Change: `KeyboardKey` is a marker trait with `Copy + Eq + Debug` supertraits, and the logical and physical key types are the values that implement it.
  Reason: language — Dart's abstract class is also a type; a trait whose implementors are `Copy` value types is not dyn-compatible.
  Affect: write `fn f<K: KeyboardKey>(key: K)` where Dart writes `KeyboardKey key`; a variable holds the concrete key type.

## hardware_keyboard.rs → hardware_keyboard.dart

- Change: a `KeyEventCallback` is `Rc<dyn Fn(&mut App, &KeyEvent) -> bool>`, and `remove_handler` matches the `Rc` allocation.
  Reason: language — a Rust closure has no identity, and a handler needs the App to do anything.
  Affect: keep the `Rc` you gave `add_handler` and pass it back by reference; a second `Rc::new` of the same closure does not match.

- Change: `sync_keyboard_state` takes the host's state map (USB HID usage to logical key id) and is synchronous, where Dart awaits `getKeyboardState` on `SystemChannels.keyboard`.
  Reason: platform — there are no method channels, and `Platform` has no keyboard-state query yet.
  Affect: a caller that can ask the host passes the map; nothing queries the host on its own.

- Change: `KeyEventManager::handle_key_data` dispatches to `HardwareKeyboard` at once and returns whether a handler took the event, where Dart infers a transit mode, queues the events until the next raw key message, and always returns false.
  Reason: platform — the raw key channel is not ported, so nothing would ever flush that queue; the host reads the answer from `key_data` instead of a channel reply.
  Affect: every key data reaches the handlers within the call; there is no `KeyMessage` and no `keyMessageHandler`.

- Change: `_dispatchKeyEvent` does not catch panics or report `FlutterError`.
  Reason: language — no catchable exception; diagnostics are deferred.
  Affect: a panicking handler unwinds the dispatch instead of letting the remaining handlers run.

## mouse_cursor.rs → mouse_cursor.dart

- Change: Dart's canonical const instances (`MouseCursor.defer`, `SystemMouseCursors.click`) are values equal to every other value of their class, not one identity.
  Reason: language — no canonical const objects to compare by identity.
  Affect: compare cursors as `*a == *b`.

- Change: a session's `activate` calls `Platform::activate_system_cursor(device, kind)` directly and returns nothing; the kind is the embedder's `SystemMouseCursorKind` enum, not a string.
  Reason: platform — there are no method channels; the host trait is the channel, and nothing needs a string encoding.
  Affect: a host that can show cursors implements that method; the inert one ignores it.

## restoration.rs → restoration.dart

- Change: restoration data is `RestorationData`, a value enum over the kinds `StandardMessageCodec` can carry, and Dart's `Map<Object?, Object?>` is `RestorationMap`, an insertion-ordered map keyed by that enum; the bucket's `read` / `write` / `remove` lose their type parameter, and `debugIsSerializableForRestoration` is gone.
  Reason: language — Rust has no `Object?`; the codec's value kinds are the only thing a bucket can hold, and the type system decides what may be stored.
  Affect: `bucket.write(app, "count", 10i64)`; `read` answers `Option<RestorationData>` that a caller narrows with `as_int` and friends; a value that cannot be serialized does not compile.

- Change: `SystemChannels.restoration` is two `Platform` methods, `restoration_get` (Dart's `{enabled, data}` reply, as `RestorationUpdate`) and `restoration_put`; the data crosses as a `RestorationMap`, so the encode / decode helpers, `initChannels` and the `push` method handler are gone.
  Reason: platform — there are no method channels; the host trait is the channel, and nothing on it needs a byte encoding.
  Affect: a host that can store restoration data implements the two methods, and pushes data it receives later by calling `RestorationManager::handle_restoration_update_from_engine` itself.

- Change: `Future<RestorationBucket?> get rootBucket` is `root_bucket`, which answers `Option<Handle<RestorationBucket>>` at once — when the root is not known yet it asks the host and builds the bucket within the call; there is no `Completer` and no `SynchronousFuture`. Listeners still fire from `handle_restoration_update_from_engine` exactly as in Dart.
  Reason: platform — the host answers a `Platform` call within the call; there is no channel reply to await.
  Affect: write `let root = manager.root_bucket(app);` where Dart writes `await manager.rootBucket`.

- Change: a `RestorationBucket` is an arena object that `dispose` destroys.
  Reason: language — the arena owns the object, and no GC keeps a disposed bucket alive.
  Affect: a handle to a disposed bucket is stale, so a release build panics on the stale handle where a debug build still raises Dart's "used after being disposed".

- Change: `RestorationBucket::root` takes the raw map by value, and the hierarchy owns it from then on; a bucket its parent stores reads and writes its map inside the parent's, which is the alias Dart gets from sharing one map object.
  Reason: language — a Rust value has one owner, so the caller cannot keep watching the map it handed over.
  Affect: read the data back through the buckets, or from what `restoration_put` receives; the map passed to `root` is gone.

## system_chrome.rs → system_chrome.dart

- Change: `setApplicationSwitcherDescription` and `setSystemUIOverlayStyle` are synchronous calls on `Platform` that return nothing; their argument types live in `reveal-embedder` because `Platform` names them, and the overlay style carries no `_toMap`.
  Reason: platform — there are no method channels and no `Future` to await; the host trait is the channel.
  Affect: `Title` and `CupertinoApp` call them in place; the failure Dart's `onError` handler reports cannot happen.

## haptic_feedback.rs → haptic_feedback.dart

- Change: each `HapticFeedback` member is a call on `Platform::haptic_feedback` with a `HapticFeedbackType`, returning nothing; the argument-less `HapticFeedback.vibrate()` passes the `Vibrate` kind where Dart passes no argument.
  Reason: platform — there are no method channels and no `Future` to await; the host trait is the channel, so the `'HapticFeedbackType.xxx'` strings are the enum's variants.
  Affect: a host without a vibrator (every desktop one) drops it.

## Deferred

- Platform channels, `SystemChannels`, `BinaryMessenger`. Trigger: the first service that talks to the host over a named channel; so far each need is a `Platform` method.
- `MouseCursor` diagnostics (`debugFillProperties`, `toString(minLevel)`). Trigger: diagnostics.
- An `EmbedderClient` hook for restoration data that arrives while the app runs (Flutter's `push` message on `SystemChannels.restoration`). Trigger: an embedder whose OS hands it new restoration data after start; until then the host calls `RestorationManager::handle_restoration_update_from_engine`.
- The rest of text input (`TextEditingValue`, `TextInputConnection`, `TextInputClient`, `TextSelectionDelegate`), the clipboard, asset bundles. Trigger: `Focus`, `EditableText`.
- The rest of `SystemChrome` (`setPreferredOrientations`, `setEnabledSystemUIMode` with `restoreSystemUIOverlays` — which restores what only that call sets — `setSystemUIChangeCallback`, `handleAppLifecycleStateChanged`, `DeviceOrientation`, `SystemUiMode`); `SystemUiOverlay` is here, as the type `setEnabledSystemUIMode` and `SystemUiChangeCallback` name. Trigger: an orientation-aware or fullscreen-capable host, and the app lifecycle.
- The raw key path: `RawKeyboard`, `RawKeyEvent`, `RawKeyEventData*`, `KeyMessage`, `KeyMessageHandler`, `KeyDataTransitMode`, `KeyEventManager.handleRawKeyMessage`; Flutter has deprecated all of it. Trigger: an embedder that can only send raw key data.
- `debugPrintKeyboardEvents` and `HardwareKeyboard._logEventIfIrregular`. Trigger: `services/debug.dart`.
- The host side of `sync_keyboard_state` (`SystemChannels.keyboard` `getKeyboardState`). Trigger: a `Platform` that can report the keys held when the app starts or regains focus.
- `KeyEvent` / `KeyboardKey` diagnostics (`debugFillProperties`); `Debug` prints the same fields. Trigger: diagnostics.
