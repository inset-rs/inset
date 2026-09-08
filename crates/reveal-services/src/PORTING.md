# reveal-services/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
Flutter home: packages/flutter/lib/src/services
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

Only the hardware keyboard, haptic feedback, mouse cursors, the mouse tracking annotation, state restoration, the application switcher description, the system overlay style, `TextInput` / `TextInputConnection` / `TextInputClient` / `TextSelectionDelegate`, text layout metric statics, the clipboard, text input formatters, `SelectionChangedCause`, autofill (`AutofillHints` / `AutofillClient` / `AutofillScope`), keyboard-inserted content, spell check, live text, process text, `UndoManager` / `UndoManagerClient`, and re-exports of the text-input value types are here; the rest of the package waits.

## Identical

- text_editing.rs → text_editing.dart (re-export of embedder `TextSelection`)
- text_layout_metrics.rs → text_layout_metrics.dart (`isWhitespace` / `isLineTerminator`)

## text_input.rs → text_input.dart

- Change: `_PlatformTextInputControl` talks to `View` (`start_text_input`, `stop_text_input`, editing state, composing/caret rects, client geometry) instead of `SystemChannels.textInput`.
  Reason: platform — there are no method channels; the host trait is the channel.
  Affect: `TextInput::attach` / `TextInputConnection::show` reach the view; a host without IME leaves the `View` defaults.

- Change: inbound `updateEditingValue` / `performAction` / `connectionClosed` are methods on the `TextInput` singleton; the host calls `EmbedderClient` and `Shell` forwards them. There is no method-call dispatcher and no client-id check.
  Reason: platform — the host trait is the channel, and one connection is attached at a time.
  Affect: a host calls `text_input_editing_value` / `text_input_action` / `text_input_closed`; it does not send a client id.

- Change: `TextInputClient` is a handle-receiver trait; a `TextInputConnection` holds an [`AnyTextInputClient`].
  Reason: language — Dart's mixin is also a type; a Rust trait is not stored by value.
  Affect: `TextInput::attach(app, client.as_text_input_client(), configuration)`.

- Change: `TextInputConnection.attached` takes `&mut App` so it can mint the `TextInput` singleton.
  Reason: language — Dart's `_instance` is a static field; ours is created on first `instance`.
  Affect: `connection.attached(app)` needs a mutable `App`.

- Change: `performPrivateCommand` takes only the action string; `insertContent`, `currentAutofillScope`, `didChangeInputControl`, `onFocusReceived`, and `TextInputStyle.toJson` are omitted.
  Reason: platform — those payloads were maps or a custom `TextInputControl`; `KeyboardInsertedContent` and `AutofillScope` exist but are not wired onto this client yet.
  Affect: a private-command `data` map is dropped; a client does not receive inserted content or expose its autofill scope here.

- Change: `TextInput::request_autofill` and `TextInput::finish_autofill_context` are empty.
  Reason: platform — there is no host autofill API; Flutter talks to `TextInputControl` over a method channel.
  Affect: `TextInput::finish_autofill_context(app, true)` where Dart writes `TextInput.finishAutofillContext()`; `TextInput::request_autofill(app)` where Dart writes `connection.requestAutofill()`. The calls exist so `AutofillGroup` dispose can invoke them; nothing reaches the host.

- Change: `TextSelectionDelegate` is a handle-receiver trait; `RenderEditable` holds an [`AnyTextSelectionDelegate`]. `pasteText` is synchronous. The toolbar methods default to no-ops.
  Reason: language — Dart's mixin is also a type; a Rust trait is not stored by value. Platform — `Clipboard` answers within the call.
  Affect: `editable.set_text_selection_delegate(app, state.as_text_selection_delegate())`; a leaf that only drives selection can omit cut/copy/paste.

## clipboard.rs → clipboard.dart

- Change: `Clipboard.setData` / `getData` / `hasStrings` are synchronous calls on `Platform` that return nothing or the value within the call.
  Reason: platform — there are no method channels and no `Future` to await; the host trait is the channel.
  Affect: `Clipboard::set_data(app, data)`; a host without a clipboard leaves `get_data` as `None`.

## keyboard_inserted_content.rs → keyboard_inserted_content.dart

- Change: `fromJson` is omitted.
  Reason: platform — there are no method-channel maps.
  Affect: construct with `KeyboardInsertedContent::new(mime, uri).data(bytes)`.

## spell_check.rs → spell_check.dart

- Change: `fetchSpellCheckSuggestions` is synchronous and answers `None`; `spellCheckChannel` is omitted.
  Reason: platform — there are no method channels; a host that can spell-check implements [`SpellCheckService`].
  Affect: `service.fetch_spell_check_suggestions(app, locale, text)` returns `None`.

## live_text.rs → live_text.dart

- Change: `isLiveTextInputAvailable` / `startLiveTextInput` take `&App` and answer `false` / do nothing.
  Reason: platform — there are no method channels.
  Affect: `LiveText::is_live_text_input_available(app)` is `false`; `start_live_text_input` is a no-op.

## process_text.rs → process_text.dart

- Change: `queryTextActions` / `processTextAction` are synchronous and answer an empty list / `None`; `setChannel` is omitted.
  Reason: platform — there are no method channels.
  Affect: `service.query_text_actions(app)` is empty; tests implement [`ProcessTextService`] instead of injecting a channel.

## autofill.rs → autofill.dart

- Change: `AutofillClient` and `AutofillScope` are handle-receiver traits; a scope holds [`AnyAutofillClient`]. `AutofillConfiguration` stays in reveal-embedder (re-exported from `text_input.rs`).
  Reason: language — Dart's mixin is also a type; a Rust trait is not stored by value.
  Affect: `client.as_autofill_client()`; `scope.attach(app, trigger.as_text_input_client(), configuration)`.

- Change: `AutofillScope.attach` calls `TextInput::attach` with the trigger's configuration and does not wrap it in `_AutofillScopeTextInputConfiguration`.
  Reason: platform — that subclass exists only to add `fields` to `toJson`; there is no channel JSON, and `View::start_text_input` takes one `TextInputConfiguration`.
  Affect: sibling autofill clients are not sent to the host on attach.

## text_formatter.rs → text_formatter.dart

- Change: Dart's `Pattern` is [`FilterPattern`] (`Literal` or `Digits`); `TextInputFormatter.withFunction` is [`TextInputFormatterRef::with_function`].
  Reason: language — there is no `regex` crate and no const factory on a trait.
  Affect: `FilteringTextInputFormatter::deny(FilterPattern::Literal("\\n".into()))`; `digits_only` is the `[0-9]` allow list.

- Change: `LengthLimitingTextInputFormatter` counts Unicode scalar values, and `getDefaultMaxLengthEnforcement` has no web branch.
  Reason: language — Dart's `characters` package counts grapheme clusters. Platform — there is no `kIsWeb`.
  Affect: a family emoji counts as more than one character; web is not a case.

## undo_manager.rs → undo_manager.dart

- Change: `UndoManagerClient` is a handle-receiver trait; `UndoManager` holds [`AnyUndoManagerClient`].
  Reason: language — Dart's mixin is also a type; a Rust trait is not stored by value.
  Affect: `UndoManager::set_client(app, Some(state.as_undo_manager_client()))`.

- Change: `setUndoState` is a no-op; `setChannel` is omitted. Inbound `handleUndo` is `UndoManager::handle_platform_undo`.
  Reason: platform — there are no method channels; the host trait is the channel.
  Affect: the host is not told `canUndo` / `canRedo`; a host calls `UndoManager::handle_platform_undo(app, direction)` instead of injecting a channel.

## text_layout_metrics.rs → text_layout_metrics.dart

- Change: only the statics live here; `getLineAtOffset` and the other instance methods live on `RenderEditable`.
  Reason: language — those methods need the arena (`App`) and the render handle, which this crate cannot name.
  Affect: `TextLayoutMetrics::is_whitespace(c)`; `editable.get_line_at_offset(app, position)`.

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
- The rest of text input (`ScribbleClient`, `DeltaTextInputClient`, `TextInputControl` / `setInputControl`, `SystemContextMenuController`, `setSelectionRects` reaching the host, `updateStyle` reaching the host), asset bundles. Trigger: `EditableText`, scribble, a custom input control, or a host that implements autofill. The `request_autofill` / `finish_autofill_context` interface exists; the host no-ops.
- The rest of `SystemChrome` (`setPreferredOrientations`, `setEnabledSystemUIMode` with `restoreSystemUIOverlays` — which restores what only that call sets — `setSystemUIChangeCallback`, `handleAppLifecycleStateChanged`, `DeviceOrientation`, `SystemUiMode`); `SystemUiOverlay` is here, as the type `setEnabledSystemUIMode` and `SystemUiChangeCallback` name. Trigger: an orientation-aware or fullscreen-capable host, and the app lifecycle.
- The raw key path: `RawKeyboard`, `RawKeyEvent`, `RawKeyEventData*`, `KeyMessage`, `KeyMessageHandler`, `KeyDataTransitMode`, `KeyEventManager.handleRawKeyMessage`; Flutter has deprecated all of it. Trigger: an embedder that can only send raw key data.
- `debugPrintKeyboardEvents` and `HardwareKeyboard._logEventIfIrregular`. Trigger: `services/debug.dart`.
- The host side of `sync_keyboard_state` (`SystemChannels.keyboard` `getKeyboardState`). Trigger: a `Platform` that can report the keys held when the app starts or regains focus.
- `KeyEvent` / `KeyboardKey` diagnostics (`debugFillProperties`); `Debug` prints the same fields. Trigger: diagnostics.
