# inset-embedder/src
No Flutter crate in this shape. Closest analogue: dart:ui `PlatformDispatcher` / `hooks.dart` / `FlutterView`.

## Identical

- key.rs → dart:ui key.dart
- text.rs → dart:ui text.dart (the text and font enums, `FontFeature`, `FontVariation`)
- locale.rs → dart:ui `Locale`
- `AppLifecycleState`, `AppExitResponse`, `AppExitType` in platform.rs → dart:ui `platform_dispatcher.dart`

## painting.rs → dart:ui `Canvas` / `Paint` / `Paragraph` / `TextStyle`

- Change: `ImageFilter` is an enum over what this host can replay — a blur, a colour filter and a composition of two.
  Reason: platform — valo's display list has no other image filter.
  Affect: the tile-mode blur and the dilate, erode, matrix and shader filters cannot be built at all.

## paragraph.rs → text.dart (`TextStyle`, `ParagraphStyle`, `ParagraphBuilder`, `Paragraph`), fonts.rs

- Change: fonts are explicit — `Platform::font_source` is the host's font lookup, and `ParagraphBuilder::build` shapes against a `FontCollection` the caller passes.
  Reason: platform — Flutter's engine holds one font manager per process; valo shapes against a collection the caller owns.
  Affect: two Apps in one process can shape the same text against different fonts, and a host that supplies no source renders no text.

- Change: `SystemFontSource` answers `CupertinoSystemText` and `CupertinoSystemDisplay` from the platform's user-interface font on demand, where the engine registers those names into its font manager at startup.
  Reason: platform — a host with its own lookup (a guest across a boundary) passes a font manager instead of the platform's.
  Affect: Cupertino text shapes with the system font on every platform that has one.

- Change: `default_font_families` on wasm answers `Roboto`, Flutter web's default, not `Arial`.
  Reason: platform — the browser has no OS UI font; Flutter's web engine ships Roboto as the fallback family.
  Affect: unspecified-family text on the web host waits for Roboto (and Noto chunks for other scripts) instead of looking for Arial.

- Change: `set_default_font_manager` registers the platform's default family as the collection's fallback chain, as Skia's `FontCollection::setDefaultFontManager` does.
  Reason: platform — valo has no default font manager, so an empty `families` list would walk fallbacks and land on `FontId(0)`.
  Affect: text that leaves `font_family` unset paints in the platform UI font instead of whichever face happened to be registered first.

- Change: `Paragraph` and `ParagraphBuilder` wrap valo's, with offsets in UTF-16 code units; there are no placeholders, box styles are ignored, and the ideographic baseline is the first line's bottom.
  Reason: platform — valo's paragraph has no placeholders, box styles, per-box direction or ideographic metrics.
  Affect: a `WidgetSpan` cannot be laid out, `TextBox::direction` is the paragraph's rather than the run's, and `BoxHeightStyle::Strut` and `Max` measure as `Tight`.

- Change: a run carries no decoration style, background paint, font features or variations, baseline or leading distribution; a combined decoration paints underline, else overline, else line-through, and an unset `font_size` shapes at valo's 14.
  Reason: platform — those are not valo span fields.
  Affect: setting them on a `TextStyle` changes nothing on screen.

## platform.rs → dart:ui `platform_dispatcher.dart`

- Change: `Platform` is `Any`, and `downcast_ref` on the trait object hands an app its host's own type; `PlatformDispatcher` is one class on every platform.
  Reason: platform — what a host offers beyond the interface (a picture from an `IOSurface`-backed buffer, its renderer's image context) belongs to that host, and naming each system's kinds in the interface would close it to the next system.
  Affect: the interface carries only what every host can answer, such as `import_pixels` for `decodeImageFromPixels`; an app that wants more names its host crate and downcasts.

- Change: Dart's isolate-global `PlatformDispatcher` is a host-supplied `Platform` that `App` holds; frame, clock and view lookup are requests on that object, not a callback table the framework assigns into.
  Reason: platform — the host supplies `Platform`, and there is no isolate-global dispatcher.
  Affect: nothing can install a dispatcher callback behind the host's back, and two Apps in one process answer independently.

- Change: there is no implicit view: `Platform::views` lists the views the platform gives, such as a phone's screen, and those the app asked for with `Platform::request_view`, each of which arrives carrying its request's id, where Flutter's engine gives an implicit view and its experimental windowing API builds window controllers over further views.
  Reason: platform — on every platform that can make a view for the app, asking is a request whose view arrives later, as iOS scenes and Android activities do, so one set of views and a request cover a phone, a desktop and an embedder alike.
  Affect: an app on a desktop has no view until it asks; the framework tells the views it asked for from those it was given by their request ids.

- Change: a requested view's window is made from a `WindowConfig` (size, position, decorations, level, activation, every desktop, background, shadow) and reached through `View::window` as a `HostWindow`, its frame, visibility, closing and native handle, as `NSView.window` is, where Flutter builds regular, dialog, popup, tooltip and satellite controllers and its `FlutterView` names no window.
  Reason: platform — the common configuration and the raw handle cover what a Rust app configures itself, as winit and wgpu expose theirs.
  Affect: an app names its windows' properties directly rather than choosing an archetype; a property the config lacks is set through the native handle; a host without windows refuses requests.

- Change: a view the platform gave may be in a window the app manages (`View::window`), and a view closes only through its window, where Flutter's implicit view has no window and its windows close through their controllers.
  Reason: platform — a host such as WaOS shows a given view in a window the app may move, title and close, as an iPad scene and an Android desktop window are, while a phone's screen, a web page's element and a host app's view are in nothing the app may close.
  Affect: closing a given view in a window closes the window; a view with no window is not the app's to close, and its way out is to quit.

- Change: `MouseCursor::activate_system_cursor` also takes the view the pointer is over.
  Reason: platform — where each window has its own cursor, the host must know which window to change.
  Affect: in an app with several windows, a cursor change reaches only the window under the pointer.

- Change: a host's `PopupMenus` capability shows the host's own popup menu at the pointer and answers the chosen entry; Flutter has no such call, only in-view context menus and the macOS menu bar.
  Reason: platform — a desktop window narrower than its menu, such as a panel at a screen's edge, cannot hold a menu drawn inside its view.
  Affect: a menu shown this way is the system's, drawn outside the window and styled by the OS; the app waits inside the call while it is open, and hosts without menus answer `None`.

- Change: a host's `Screens` capability lists the screens, with what the system's bars leave free, and where the pointer is on them; Flutter's `Display` has a size but no place.
  Reason: platform — a desktop app that places a window against a screen's edge or its own icon needs to know where the screens are.
  Affect: an app can place a window in screen coordinates, and hosts with no screens to place against answer `None`.

- Change: a host's `FileOpening` capability takes one handler for the files the system asks the app to open; Flutter's macOS embedder hands them to its plugins through `FlutterAppLifecycleDelegate.handleOpenURLs:`.
  Reason: platform — there are no plugins, and the app is the only listener on the host's side.
  Affect: an app sets the handler once, files asked for before then wait for it, and hosts whose system never asks answer `None`.

- Change: `default_target_platform` and `platform_brightness` are queries on `Platform`, with no library global and no `debugDefaultTargetPlatformOverride`; the inert platform answers Android and light, as Flutter's test binding does.
  Reason: platform — the host-supplied `Platform` is the only source of truth.
  Affect: changing the target platform means swapping the host's `Platform`, and two Apps can disagree at once.

- Change: the system channels that reach the host are capability traits a `Platform` hands out, and a host that has none of one answers `None`.
  Reason: platform — there are no method channels; the host trait is the channel, so payloads cross as values rather than Dart's `_toMap` encodings.
  Affect: a caller sees an unsupported capability as a `None` it can act on, where Dart's missing-channel failure and a defaulted method are both invisible to it.

- Change: `Platform` asks the host whether its own text input turns editing keys into edits — backspace, delete, caret movement — and the default answer is no.
  Reason: platform — Flutter writes one embedder per host and settles this per target platform, where one framework here meets hosts that differ on the same platform.
  Affect: on a host that reports plain key presses, a text field's editing keys work whichever platform the host claims to be.

- Change: `Platform::dispatcher` hands out the host's threads as one `Send + Sync` object: any thread asks it for a turn at a time, answered by `EmbedderClient::wake`, and the framework hands it work to run off the main thread; Dart's isolate is woken by its message port and `Isolate.run` uses the VM's own threads.
  Reason: platform — the host owns the event loop and knows its system's work queues, and a task completed on another thread has no other way to give the app a turn.
  Affect: a spawned task can await a future completed on any thread and runs at the next checkpoint instead of at the next input, and `run_in_background` work lands on the system's queue where there is one.

- Change: `Platform::compositor` hands out native surfaces that a producer feeds with the platform's own buffers from any thread, in place of Flutter's texture registry and platform-view plumbing.
  Reason: platform — the framework, the host and every producer share one process, so a buffer can go to the host's compositor directly.
  Affect: a host without a compositor answers `None`, and a surface placed there leaves only a clear rectangle.

## image.rs → dart:ui `instantiateImageCodec` / `Codec` / `FrameInfo`

- Change: `Image` is a counted handle to the host's texture, with no clone and no dispose.
  Reason: language — the texture is freed when the last holder drops it, where Dart's collector needs `Image.clone` and `Image.dispose` to be told when that is.
  Affect: a picture is freed when the last holder lets go of it, so Flutter code being ported has no `dispose` call to make and no freed picture left to draw by mistake.


## restoration.rs → services `message_codecs.dart` (`StandardMessageCodec`)

- Change: `RestorationData` is a value enum over the kinds `StandardMessageCodec` can carry, and there is no encode or decode step.
  Reason: platform — the framework and the host meet at a Rust trait, not at a byte channel.
  Affect: a host stores and returns the `RestorationMap` it was given; a host that needs bytes on disk has to serialise it itself.

## views.rs → dart:ui `FlutterView` / `ViewPadding` / `ViewConstraints`

- Change: `View` is a handle to the native surface; the host owns the window and answers metrics from it, and the framework keeps no copy.
  Reason: platform — the native window is the source of truth; a framework-side copy would go stale.
  Affect: metrics read after a view-lifecycle notification are the window's current ones, never a cached frame behind.

- Change: `View::present` takes a shared `Scene`, a valo `Picture` with the surfaces it places, where `FlutterView.render` takes a `Scene` the engine consumes.
  Reason: platform — the host paints a valo display list, not an engine `Scene`, and keeps it: a window seen again is repainted from it without a frame, and a picture equal to the last, which valo can tell from its retained nested lists, is not drawn again.
  Affect: what reaches the host is a recorded display list it replays itself and may hold past the call; a frame whose windows all show what they showed before draws nothing.

- Change: text input is a capability a `View` hands out, and a view with no IME behind it answers `None`.
  Reason: platform — there are no method channels; the host trait is the channel.
  Affect: a caller sees the absent IME as a `None` it can act on, where a focused field still edits from key events but nothing composing reaches the platform.

## client.rs → dart:ui `hooks.dart`

- Change: the isolate-global engine hooks are methods on `EmbedderClient`: one `frame` is a complete engine frame, pointer packets and key data arrive as calls, view lifecycle is a typed notification, and `key_data` returns whether the framework handled the key.
  Reason: platform — there is no isolate-global hooks table; the host drives a client without naming `App`.
  Affect: the host does not run scheduler phases or build `PointerEvent` / `KeyEvent` itself, and decides from `key_data`'s answer whether to keep propagating the native event.

- Change: `EmbedderClient::wake(elapsed)` reports that a `Platform::wake_at` deadline passed, on the clock `Frame::elapsed` uses.
  Reason: platform — the `App` owns its timers and has no event loop firing them.
  Affect: a `Timer` fires only when the host calls `wake` or delivers a frame, so a host that never wakes never runs one.

- Change: `platform_brightness_changed` and `locales_changed` carry no value; the new one is read back from `Platform`.
  Reason: platform — the host pushes a notification and the framework reads the dispatcher, as with `frame`.
  Affect: a host must update what its `Platform` answers before calling the hook, or observers rebuild against the old value.

- Change: `system_fonts_changed` is a client method, where Dart's engine posts a `fontsChange` system message that `PaintingBinding` turns into `systemFonts.notifyListeners`.
  Reason: platform — there are no method channels; the host trait is the channel.
  Affect: a host that loads faces after start-up must call this or laid-out text keeps the glyphs it had.

- Change: `TextInputClient.updateEditingValue` / `performAction` / `connectionClosed` are `EmbedderClient` methods the host calls.
  Reason: platform — there are no method channels; the host trait is the channel.
  Affect: IME edits reach `TextInput` only through these calls, in the host's own order relative to key data.

- Change: `EmbedderClient::drop_data` reports files dragged from outside the application over a view as a `DropData` — the entered, moved, dropped and exited phases, a batch of paths, a physical position when the host has one — where Flutter has no drop hook and leaves it to plugins.
  Reason: platform — every desktop toolkit reports this shape at the window, and a host is the only place it can come from.
  Affect: an app hears external drops through the widgets binding's observers; a host that reports no position, as winit does, leaves the app to treat the whole view as the target.

- Change: `EmbedderClient::pop_route` answers at once whether the application went back, and the framework never tells the host in advance whether it handles back.
  Reason: platform — Dart's `popRoute` crosses a channel and cannot answer in time, which is why Flutter tells the host in advance; this call is direct.
  Affect: when the answer is no, the host does what its platform does without the app, which on Android is to close the activity.

- Change: `EmbedderClient::request_app_exit` takes an `AppExitReply` the framework answers, rather than replying on a channel.
  Reason: platform — there are no method channels; the host trait is the channel.
  Affect: the answer may come long after the call, such as once the user has chosen whether to save, and its reply posts it to the host's own loop; dropped unanswered, it answers exit.

- Change: `AppExit::quit` is `System.exitApplication` for a required exit only: a cancelable exit stays in the framework, which asks its own listeners and then quits, where Flutter's engine asks the framework back.
  Reason: platform — only the framework's answer decides, so the host has nothing to add between the question and the answer.
  Affect: a host implements quitting alone, and an app sees the same answers as in Flutter.

- Change: `EmbedderClient::view_close_requested` reports the user's request to close a view, and the host closes nothing itself, where Flutter's engine calls the window controller's delegate, whose `onWindowCloseRequested` closes it by default.
  Reason: platform — the host trait is the channel, and the framework decides each view's close.
  Affect: a window stays open until the app closes it.

- Change: a host reports the lifecycle as `AppLifecycleState` values through `EmbedderClient::app_lifecycle_state_changed`, and `Platform::initial_lifecycle_state` answers `None` where Dart has an empty string.
  Reason: platform — there are no method channels; the host trait is the channel.
  Affect: a host cannot report a state the framework does not know.

## scene_builder.rs → dart:ui `SceneBuilder` / `Scene`

- Change: a push records into a display list of its own, which `pop` returns as the engine layer; there is no `oldLayer`, `addTexture` or the shader-mask, colour-filter and image-filter pushes.
  Reason: platform — valo composites display lists, and a list closes at its restore.
  Affect: the engine layer is known at `pop`, not at the push.

- Change: `SceneBuilder::add_surface` places a native surface beside the picture as a rectangle, a scale and a rectangular clip, where Flutter's `addPlatformView` hands the embedder the whole stack of transforms and clips.
  Reason: platform — a native surface cannot be part of valo's display list, and a rectangle, a scale and a rectangular clip are what every compositor Inset targets can apply to a layer.
  Affect: a rotated surface is placed by its bounding box, a rounded clip's corners show the surface unless the picture paints over them, and an opacity above it is ignored.

## Deferred

- Decoding at any size but the file's own, which in Dart is `instantiateImageCodec`'s target width and height and the `getTargetSize` callback of `instantiateImageCodecWithSize`. Trigger: `ResizeImage`, which is what the image widget's `cacheWidth` and `cacheHeight` become.
- Reading an image's pixels back (`Image.toByteData`). Trigger: golden tests.
- The colour space an image carries beyond sRGB, so a photograph in a wider one is drawn as though it were sRGB. Trigger: a display that shows more colours than sRGB.
- Making an image from pixels that were never encoded (`ImageDescriptor.raw`, `decodeImageFromPixels`). Trigger: a caller holding pixels rather than a file.
- How far along a decode is, which Dart's codec reports as it reads the file. Trigger: a provider that reports progress.
- `StrutStyle`, `TextStyle.locale` / `ParagraphStyle.locale`, `addPlaceholder` / `placeholderScales`, `getBoxesForPlaceholders` contents. Trigger: strut, locale-specific glyphs, `WidgetSpan`; valo has none of them.
- `ImageFilter.blur(tileMode:)` and the `dilate` / `erode` / `matrix` / `shader` filters. Trigger: valo growing those ops.
- `PlatformDispatcher.locale`, the first of `locales`. Trigger: a caller of the single-locale getter.
- The system font's optical size by text size, which CoreText applies for the engine. Trigger: valo-text setting variation axes from the text size.
- `FontFeature` named tag constructors. Trigger: a caller that uses those factories instead of `new` / `enable` / `disable`.
- `debugDefaultTargetPlatformOverride`. Trigger: a debug switcher that must override a live host without swapping `Platform`.
- `PointerData.respond` / `onRespond`. Trigger: a web host, where they call `preventDefault` on the originating DOM event.
- Semantics callbacks. Trigger: semantics.
- `PlatformDispatcher` callback setters and Zones. Trigger: a requirement to expose dart:ui callbacks independently of `Shell`.
- `_updateFrameData` and the `frameNumber` argument to `_beginFrame`. Trigger: `FrameData`.
- The rest of `hooks.dart`. Trigger: the matching dart:ui callback.
