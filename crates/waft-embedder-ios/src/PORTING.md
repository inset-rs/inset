# waft-embedder-ios/src

Embedder implementation references: Flutter's iOS embedding under `flutter/engine/src/flutter/shell/platform/darwin/ios/framework/Source`, and the Flutter plugins named below.

## Identical

- app_delegate.rs → `FlutterAppDelegate`: the application's delegate, which makes no window once the app adopts scenes
- scene_delegate.rs, open_requests.rs → `FlutterSceneDelegate`: each scene's connection, its moves between the foreground and the background, and the URLs it is opened for, at connection and later (`scene:openURLContexts:`)
- view_controller.rs, metrics.rs → `FlutterViewController`'s viewport metrics: the view's bounds and its safe area as padding, in physical pixels, read again at `viewDidLayoutSubviews` and `viewSafeAreaInsetsDidChange`
- touches.rs → `FlutterViewController`'s `dispatchTouches:`: one device per touch, its address, its force and radius, a pencil a stylus
- keys.rs → `FlutterEmbedderKeyResponder` and `KeyCodeMap.g.mm`: the physical key from the HID usage, the logical key from the special keys' names, the keys named by code or the character typed without modifiers, and the pressing records
- display_link.rs → `VSyncClient` and `DisplayLinkManager`: a display link in the main run loop's common modes at the screen's highest rate, `CADisableMinimumFrameDurationOnPhone` set

## view.rs, views.rs, host.rs → `FlutterViewController` in a `FlutterSceneDelegate`'s scene

- Change: every scene shows a view of one app, the implicit view the first scene's, and a view keeps its tree and last picture while its scene is disconnected, for the scene UIKit connects again by the same session; Flutter's engine is one scene's.
  Reason: architecture — Waft's app is one, with a view per window, as on the desktops and Android.
  Affect: the app starts before any scene, with the implicit view at no size, and an iPad's second window, once asked for, is a view of the same app.

- Change: the app's lifecycle state is its most forward scene's: resumed while any scene is active, detached once none is connected.
  Reason: architecture — one app with many scenes, where Flutter reports each engine's own scene.
  Affect: on an iPhone, with one scene, the states are Flutter's.

## keyboard_inset.rs, keyboard_notices.rs → `KeyboardInsetManager.swift`

- Change: the docked keyboard's inset is set to where the keyboard ends as its notice comes, where Flutter animates it along the keyboard's own animation; Slide Over's adjustment is left out.
  Reason: simplification — the animation follows the keyboard's frames at the display link's ticks, a step of its own.
  Affect: a view lays out for the keyboard's end as the keyboard starts to move, rather than moving with it.

## text_input/ → `FlutterTextInputPlugin` and `FlutterTextInputView`

- Change: the field sits in the view's `UIView` at the editable's origin, where Flutter's sits in an input hider beside it, and the view's `UIView` takes the first responder back when the framework closes a connection.
  Reason: architecture — a view's `UIView` is the responder the hardware keyboard's presses reach between fields (`ui_view.rs`), and the field's place in it is the one UIKit's autocorrection highlight needs on iOS 17.
  Affect: none an app sees.

- Change: UIKit's `UITextInputStringTokenizer` answers the field's tokenizer questions, where Flutter's own takes line boundaries from the selection rects.
  Reason: simplification — the line granularity serves the iOS 16 Scribble highlight and caret movement by line, which the floating cursor and Scribble need first.
  Affect: a keyboard asking for the line around a position in a multiline field gets the string tokenizer's answer, which knows of no wrapping.

- Change: a connection the framework replaces without closing it keeps the field of the one it replaces.
  Reason: architecture — the framework's `TextInput::attach` starts the host as `show` and `update_config` do, so the host sees a new connection only after a closed one; Flutter's `setClient` names each.
  Affect: a text field that takes another's connection before the other closes it gets the keyboard's state the other left, its suggestions and autocorrection among them.

## keys.rs → `FlutterEmbedderKeyResponder`

- Change: the modifier keys are not synchronized with the press's flags, and Caps Lock's tap is not synthesized.
  Reason: simplification — a modifier pressed or released while the app was away is the one case it covers.
  Affect: a modifier held as the app comes to the front is not reported down until it is pressed again.

## pasteboard/ → Flutter's `Clipboard` platform channel

- Change: the clipboard is UIKit's general pasteboard, its items in every type it carries, a copy with every form given written whole, and one with a promised form or file written as item providers, a promised file written by its producer into a temporary file the provider hands over, one for this device only written local only, and one that expires carrying its expiration date.
  Reason: platform — Flutter's iOS engine carries plain text over the platform channel.
  Affect: an iOS app copies and pastes images, HTML and files with other apps, and its expired copy is cleared by the system after the app has quit too.

- Change: listing the pasteboard reads only its types, and a read loads the form through the item's provider, off the main thread.
  Reason: platform — iOS shows its paste alert when an app reads another app's content, and the app's own promised form is produced on the main thread.
  Affect: the alert shows when the app reads, never when it lists, and a read of the app's own promise never stalls it.

- Change: the app hears the clipboard changed when UIKit says the pasteboard changed and when a scene becomes active, if another writer moved the pasteboard's count of changes.
  Reason: platform — iOS tells only an app that is active.
  Affect: a toolbar's Paste button follows a copy made in another app once the user comes back to this one.

## file_dialogs.rs → `file_selector`'s iOS implementation

- Change: the dialogs are `UIDocumentPickerViewController`, presented by the parent view's view controller or that of a view in sight, opening in place; each picked URL's security-scoped access starts when it is answered and is kept for the app's life, since a path carries no lifetime to end it by. A save makes an empty file under the suggested name in a folder of its own in the temporary folder and has the picker export it, answering the file where the user put it.
  Reason: platform — iOS has no dialog that answers a file yet to be written, and a picked document outside the sandbox is reached only while its access lasts.
  Affect: a picked path stays readable and writable until the app quits; an app picking without end would run out of the kernel's sandbox extensions.

## opener.rs → `url_launcher`'s iOS implementation

- Change: the opener is `UIApplication`'s `openURL:options:completionHandler:` for a link, its `false` being no app, and `UIDocumentInteractionController`'s Open In menu for a file, presented from a view in sight, its `false` being no app; revealing fails, and `can_open_link` asks `canOpenURL:` only for a scheme the app's `LSApplicationQueriesSchemes` declares, `None` for another.
  Reason: platform — iOS shares a file with another app only through a controller the user chooses the app in, and answers `canOpenURL:` falsely for a scheme the app did not declare.
  Affect: opening a file on iOS shows the user the apps that take it.

## Deferred

- The text field's Scribble and its focus, the floating cursor, dictation and text placeholders, and the edit menu interaction. Trigger: an app used with the Apple Pencil's handwriting, the space bar's trackpad, dictation, or the system's edit menu.
- Autofill groups, their password view and `finishAutofillContext`. Trigger: a sign-in form that saves and fills passwords.
- The autocorrection prompt's rect before iOS 17 (`showAutocorrectionPromptRect`). Trigger: an app supporting iOS 15 or 16 that wants the highlight drawn under the corrected word.
- The text input delta model, and the field's accessibility. Trigger: a client that asks for deltas; VoiceOver.
- The iPad's pointer, hover and scrolling (`hoverEvent:`, `discreteScrollEvent:`). Trigger: an iPad app used with a trackpad.
- A scene the app asks for (`Platform::request_view`). Trigger: iPad windows, which the scene registry is shaped for.
- Video frames read at the display link's ticks for a reader a scene draws. Trigger: an app that draws a player's frames itself rather than its layer.
