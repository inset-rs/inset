# inset-services/src
Flutter home: packages/flutter/lib/src/services
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

Only the hardware keyboard, haptic feedback, mouse cursors, the mouse tracking annotation, state restoration, the application switcher description, the system overlay style, `TextInput` / `TextInputConnection` / `TextInputClient` / `TextSelectionDelegate`, text layout metric statics, the text boundaries, the clipboard, text input formatters, `SelectionChangedCause`, autofill (`AutofillHints` / `AutofillClient` / `AutofillScope`), keyboard-inserted content, spell check, live text, process text, `UndoManager` / `UndoManagerClient`, and re-exports of the text-input value types are here; the rest of the package waits.

## Identical

- text_editing.rs → text_editing.dart (re-export of embedder `TextSelection`)
- text_layout_metrics.rs → text_layout_metrics.dart (`isWhitespace` / `isLineTerminator`)
- text_boundary.rs → text_boundary.dart (`LineBoundary` needs layout, so it sits in rendering)

## text_input.rs → text_input.dart

- Change: text input reaches the host through `View` — start and stop, editing state, composing and caret rects, client geometry.
  Reason: platform — there are no method channels; the host trait is the channel.
  Affect: on a host that does not implement those methods, `TextInputConnection::show` and the rest do nothing and no IME appears.

- Change: the host delivers `updateEditingValue` / `performAction` / `connectionClosed` by calling `EmbedderClient`, and `Shell` forwards them to the `TextInput` singleton without a client id.
  Reason: platform — the host trait is the channel, and one connection is attached at a time.
  Affect: an inbound edit always lands on the currently attached client; a message meant for a connection that has just been replaced cannot be told apart and is applied to the new one.

- Change: `performPrivateCommand` carries only the action string, and `insertContent`, `currentAutofillScope`, `didChangeInputControl` and `onFocusReceived` are absent from `TextInputClient`.
  Reason: platform — those payloads were method-channel maps, and no host sends them.
  Affect: a private command's `data` is dropped, and a client is never handed inserted content nor asked for its autofill scope.

- Change: `TextInput::request_autofill` and `TextInput::finish_autofill_context` exist but do nothing.
  Reason: platform — there is no host autofill API.
  Affect: `AutofillGroup` disposal still calls them, but the platform is never asked to fill a group or to commit a saved autofill context.

- Change: `TextSelectionDelegate::paste_text` completes within the call, and the toolbar and view methods default to no-ops.
  Reason: platform — `Clipboard` answers in place, so there is no `Future` to await.
  Affect: pasted text is in the field before the call returns; a delegate that only drives selection silently does nothing where Dart would not compile without the member.

## system_context_menu.rs → `SystemContextMenuController` / `IOSSystemContextMenuItemData*` in text_input.dart

- Change: the last-shown controller is registered per `App` rather than in a Dart static, menu items cross to the host as `SystemContextMenuItem` values instead of JSON maps, and a custom item's callback id is assigned on the item rather than derived from `hashCode`.
  Reason: language — there is no isolate-wide static; platform — the host trait is the channel and carries no JSON.
  Affect: two `App`s can each show a system menu at once, and two custom items with equal contents keep separate callbacks.

## clipboard.rs → clipboard.dart

- Change: `Clipboard::set_data` / `get_data` / `has_strings` answer within the call rather than returning a `Future`.
  Reason: platform — there are no method channels; the host trait is the channel and answers in place.
  Affect: paste happens in the same frame that asks for it; a host without a clipboard answers `None`.

## spell_check.rs → spell_check.dart

- Change: `fetch_spell_check_suggestions` answers within the call, and answers `None` unless a host implements [`SpellCheckService`].
  Reason: platform — there are no method channels, so there is no default OS spell checker to reach.
  Affect: text is never marked misspelled unless the embedder supplies a service.

## live_text.rs → live_text.dart

- Change: `LiveText::is_live_text_input_available` answers `false` and `start_live_text_input` does nothing.
  Reason: platform — there are no method channels and no host API for it.
  Affect: the "insert from camera" entry never appears in a text toolbar.

## process_text.rs → process_text.dart

- Change: `query_text_actions` answers an empty list and `process_text_action` answers `None`, both within the call.
  Reason: platform — there are no method channels; a host that has OS text actions implements [`ProcessTextService`].
  Affect: no OS text actions appear in the selection toolbar, and a test supplies a service instead of a fake channel.

## autofill.rs → autofill.dart

- Change: `AutofillScope::attach` hands `TextInput::attach` the trigger's own configuration and does not wrap it to carry the group's other fields.
  Reason: platform — Dart's wrapper exists only to add `fields` to the channel JSON, and `TextInputHost::start` takes one configuration.
  Affect: the host never learns the sibling fields of a group, so OS autofill fills the focused field alone.

## text_formatter.rs → text_formatter.dart

- Change: `FilteringTextInputFormatter` matches a [`FilterPattern`], either a literal or the digit class, where Dart takes any `Pattern`.
  Reason: language — there is no regular expression type in the port.
  Affect: a filter Dart writes as a regex cannot be expressed; only a literal and `digits_only` filter.

- Change: `LengthLimitingTextInputFormatter` counts Unicode scalar values, and `get_default_max_length_enforcement` has no web branch.
  Reason: language — Dart counts grapheme clusters through its `characters` package; platform — there is no web target.
  Affect: a family emoji spends several characters of the limit, and truncating at the limit can cut one apart.

## undo_manager.rs → undo_manager.dart

- Change: `set_undo_state` does nothing, and the host drives undo by calling `UndoManager::handle_platform_undo`.
  Reason: platform — the host trait is the channel and has no message carrying undo state.
  Affect: the OS undo affordance is never told whether undo or redo is available, so it cannot disable itself.

## keyboard_key.rs → keyboard_key.g.dart

The key constants and the four tables are machine-written from the Dart, as Flutter machine-writes that file from `dev/tools/gen_keycodes`.

## hardware_keyboard.rs → hardware_keyboard.dart

- Change: `sync_keyboard_state` takes the pressed-key map from its caller instead of asking the host for it.
  Reason: platform — `Platform` has no keyboard-state query.
  Affect: keys already held when the app starts or regains focus stay unknown unless an embedder passes them in.

- Change: `KeyEventManager::handle_key_data` dispatches to `HardwareKeyboard` within the call and answers whether a handler took the event, where Dart infers a transit mode and queues events until the next raw key message.
  Reason: platform — the raw key channel is not ported, so nothing would ever flush that queue.
  Affect: handlers see each key event as it arrives and the host reads the answer from `key_data`; there is no `KeyMessage` stage after them.

- Change: key dispatch does not catch a panicking handler.
  Reason: language — no `FlutterError.reportError` hook to catch and continue.
  Affect: one failing handler unwinds the dispatch and the remaining handlers do not run.

## mouse_cursor.rs → mouse_cursor.dart

- Change: a session's `activate` calls the host's `MouseCursor` capability with a `SystemMouseCursorKind` and returns nothing.
  Reason: platform — the host trait is the channel; there is no channel reply to await and nothing needs the string encoding.
  Affect: the cursor changes without waiting a frame, and a host with no such capability leaves the cursor as it was.

## restoration.rs → restoration.dart

- Change: restoration data is `RestorationData`, an enum over the kinds the codec can carry, and a bucket's `read` / `write` / `remove` lose Dart's type parameter.
  Reason: language — Rust has no `Object?`, so the enum is the only thing a bucket can hold.
  Affect: a value Dart would reject at run time cannot be written at all, and `read` answers data the caller narrows with `as_int` and friends.

- Change: restoration reaches the host through its `Restoration` capability, passing a `RestorationMap` rather than an encoded byte buffer.
  Reason: platform — the host trait is the channel and nothing on it needs an encoding.
  Affect: a host stores the map as it stands, and pushes data that arrives later by calling `RestorationManager::handle_restoration_update_from_engine` itself.

- Change: `root_bucket` asks the host and builds the bucket within the call, where Dart's getter is a `Future`.
  Reason: platform — the host answers a `Platform` call in place; there is no channel reply to await.
  Affect: the root bucket is there for the first frame that asks for it, and listeners still fire from `handle_restoration_update_from_engine` as in Dart.

- Change: `dispose` destroys the bucket's arena slot.
  Reason: language — the arena owns the object, and no collector keeps a disposed bucket alive.
  Affect: using a bucket after `dispose` panics in a release build too, where Dart only asserts in debug.

- Change: `RestorationBucket::root` takes the raw map by value and the hierarchy owns it from then on.
  Reason: language — a Rust value has one owner, so the caller cannot keep watching the map it handed over.
  Affect: a child bucket's writes are no longer visible through the caller's map; read the data back through the buckets or from what `Restoration::put` receives.

## system_chrome.rs → system_chrome.dart

- Change: the switcher description and the overlay style are calls on the host's `SystemChrome` capability that complete in place and cannot fail.
  Reason: platform — the host trait is the channel; there is no `Future` and no channel error.
  Affect: `Title` and `CupertinoApp` apply them mid-build, and the failure Dart's `onError` handler reports cannot happen.

## haptic_feedback.rs → haptic_feedback.dart

- Change: each `HapticFeedback` member calls the host's `Haptics` capability with a `HapticFeedbackType`, and the argument-less `vibrate` passes the `Vibrate` kind.
  Reason: platform — the host trait is the channel, so Dart's `'HapticFeedbackType.xxx'` strings are the enum's variants.
  Affect: a host without a vibrator, which is every desktop one, drops the call.

## Deferred

- Platform channels, `SystemChannels`, `BinaryMessenger`. Trigger: the first service that needs a named channel rather than a `Platform` method.
- `MouseCursor` diagnostics (`debugFillProperties`, `toString(minLevel)`). Trigger: diagnostics.
- An `EmbedderClient` hook for restoration data that arrives while the app runs. Trigger: an embedder whose OS hands it new data after start; until then the host calls `RestorationManager::handle_restoration_update_from_engine`.
- The rest of text input: `ScribbleClient`, `DeltaTextInputClient`, `TextInputControl` / `setInputControl`, `setSelectionRects` and `updateStyle` reaching the host, asset bundles. Trigger: scribble, a custom input control, or a host that implements autofill.
- The rest of `SystemChrome`: `setPreferredOrientations`, `setEnabledSystemUIMode`, `restoreSystemUIOverlays`, `setSystemUIChangeCallback`, `handleAppLifecycleStateChanged`, `DeviceOrientation`, `SystemUiMode`. Trigger: an orientation-aware or fullscreen-capable host, and the app lifecycle.
- The raw key path (`RawKeyboard`, `RawKeyEvent`, `KeyMessage`, `KeyDataTransitMode`, `KeyEventManager.handleRawKeyMessage`), which Flutter has deprecated. Trigger: an embedder that can only send raw key data.
- `debugPrintKeyboardEvents` and `HardwareKeyboard._logEventIfIrregular`. Trigger: `services/debug.dart`.
- The host side of `sync_keyboard_state`. Trigger: a `Platform` that can report the keys held at start or on regaining focus.
- `KeyEvent` / `KeyboardKey` diagnostics. Trigger: diagnostics.
