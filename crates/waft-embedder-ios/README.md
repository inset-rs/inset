# waft-embedder-ios

Waft's iOS host: UIKit's application and its scenes, each scene's window showing one view of the app's, shaped as Flutter's iOS embedding is (`FlutterAppDelegate`, `FlutterSceneDelegate`, `FlutterViewController`). It takes an app's start closure, which `#[waft::main]`'s `main` hands it through `waft-embedder-default`, and iOS 15 or later; it provides `IosEmbedder`, which hands the process to `UIApplicationMain`, and `IosPlatform`, Waft's `Platform` on iOS. `src/lib.rs` says how the host is built. It uses `waft-embedder-apple` for what macOS shares, and `cargo waft` writes the `Info.plist` it needs: the `UIApplicationSceneManifest` naming its `WaftSceneDelegate`, which iOS 27 requires of an app built with its SDK, and `CADisableMinimumFrameDurationOnPhone`.

What is there:

- The scene lifecycle: the app starts as the launch finishes, with Flutter's implicit view and no window for it; each scene that connects shows a view in a window of its own, the implicit view in the first; a scene disconnected and connected again by its session gets its view back; the app's lifecycle state is its most forward scene's. One scene at a time for now (`UIApplicationSupportsMultipleScenes` is off), the registry shaped for more.
- Drawing: each view draws with valo into a `CAMetalLayer` of its own, with the composition layers its scenes place beneath it, at a display link's ticks, at the screen's highest rate; a resize draws its new layout before UIKit commits it.
- Metrics: the view's bounds and safe area in physical pixels, the docked keyboard as `view_insets`, the user's appearance and languages.
- Input: touches and the Apple Pencil as pointers; a hardware keyboard's presses as `KeyData`, handed back to UIKit where the framework leaves them.
- Text input: the system's keyboards, their autocorrection, suggestions and marked text, through a `UITextInput` field per connection, configured from the field's configuration as Flutter's iOS embedding configures its own.
- The general pasteboard, document pickers, opening links and files, the files and links the app is opened with, notifications, AVFoundation's players, media assets and cameras, Image I/O's codecs, `IOSurface` buffers.

What is not, and how an app sees it:

- No window the app asks for: `Platform::request_view` answers `Unsupported`.
- No iPad pointer, hover or scrolling; the keyboard's inset is set at once rather than moving with the keyboard; a reader's video frames are not drawn into a scene; no Scribble, floating cursor, dictation placeholders or autofill groups in a text field. `src/PORTING.md` lists these.
