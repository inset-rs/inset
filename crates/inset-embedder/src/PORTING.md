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

- Change: `Platform::default_font_families` asks the host for `txt::GetDefaultFontFamilies()`, by default the engine's answer on the OS this build targets, where Flutter's engine has its own OS's answer compiled in.
  Reason: platform — one build can run on several systems, as a WebAssembly component does, and only its host knows at run time which one it is on.
  Affect: such a build draws text that leaves `font_family` unset in the default family of the system it runs on, not of one fixed when it was compiled.

- Change: `set_default_font_manager` registers the platform's default family as the collection's fallback chain, as Skia's `FontCollection::setDefaultFontManager` does.
  Reason: platform — valo has no default font manager, so an empty `families` list would walk fallbacks and land on `FontId(0)`.
  Affect: text that leaves `font_family` unset paints in the platform UI font instead of whichever face happened to be registered first.

- Change: `Paragraph` and `ParagraphBuilder` wrap valo's, with offsets in UTF-16 code units; box styles are ignored, and the ideographic baseline is the first line's bottom.
  Reason: platform — valo's paragraph has no box styles, per-box direction for a range or ideographic metrics.
  Affect: a range's `TextBox::direction` is the paragraph's rather than the run's (a placeholder's is its run's), and `BoxHeightStyle::Strut` and `Max` measure as `Tight`.

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

- Change: a window says whether it is minimized and whether it is occluded, and the host tells the app each time either changes or the window moves, where Flutter's controllers ask each platform whether a window is minimized, hear its changes from that platform's own windowing, and have no word for a covered window.
  Reason: platform — the host is what hears a window minimized, covered or moved, and an app that stops drawing what nobody sees must know it of each window rather than of the whole application.
  Affect: an app tells a minimized or covered window from a shown one while its other windows show, and where the system does not say, as Windows does not say a window is covered and Wayland says neither, the window answers no.

- Change: the framework asks the host for a frame of one view, and the host gives one refresh of a screen at a time with the views on it that asked, where Flutter asks for and gets one frame of every view.
  Reason: platform — each window is on a screen with its own refresh rate and can be minimized or occluded, and a host that paces every view by one screen stops them all while that one is out of sight.
  Affect: each window draws at its own screen's refresh rate, and a minimized or occluded window draws nothing until it shows while the others keep drawing.

- Change: `MouseCursor::activate_system_cursor` also takes the view the pointer is over.
  Reason: platform — where each window has its own cursor, the host must know which window to change.
  Affect: in an app with several windows, a cursor change reaches only the window under the pointer.

- Change: `PlatformDispatcher.displays` is a host's `Screens` capability, whose `Screen` is Flutter's `Display` with a name, a frame placed on the desktop in points in place of a size in pixels, and what the system's bars leave free, and which also says where the pointer is.
  Reason: platform — a desktop app that places a window against a screen's edge or its own icon needs to know where the screens are, in the points its window frames are in.
  Affect: an app can place a window in screen coordinates, `Display.size` is the frame's size at `scale`, and a host with no screens to report answers `None`.

- Change: `FlutterView.display` is the screen id in the view's metrics, looked up in `Platform::screens`, and `None` while the view is on no screen the host can name; a refresh rate the host cannot say is `None` where Flutter's is 0.
  Reason: platform — a view holds no reference to the platform, and a host has no screen for a view without a window.
  Affect: a view's screen comes from the platform's list, and a view moved to another screen is reported as a metrics change, as Flutter's window metrics event carries its `display_id`.

- Change: `default_target_platform` and `platform_brightness` are queries on `Platform`, with no library global and no `debugDefaultTargetPlatformOverride`; the inert platform answers Android and light, as Flutter's test binding does.
  Reason: platform — the host-supplied `Platform` is the only source of truth.
  Affect: changing the target platform means swapping the host's `Platform`, and two Apps can disagree at once.

- Change: the system channels that reach the host are capability traits a `Platform` hands out, and a host that has none of one answers `None`.
  Reason: platform — there are no method channels; the host trait is the channel, so payloads cross as values rather than Dart's `_toMap` encodings.
  Affect: a caller sees an unsupported capability as a `None` it can act on, where Dart's missing-channel failure and a defaulted method are both invisible to it.

- Change: `Platform` asks the host whether its own text input turns editing keys into edits — backspace, delete, caret movement — and the default answer is no.
  Reason: platform — Flutter writes one embedder per host and settles this per target platform, where one framework here meets hosts that differ on the same platform.
  Affect: on a host that reports plain key presses, a text field's editing keys work whichever platform the host claims to be.

- Change: the host's text input also says whether the system's menus send Edit's cut, copy and paste (`TextInputHost::provides_edit_commands`), as macOS's menu bar does while it holds those items, and the default answer is no.
  Reason: platform — such a menu takes the key the framework leaves and sends the system's own command, which is what lets a host give a paste to the user's command alone.
  Affect: a text field there leaves ⌘C, ⌘X and ⌘V to the menu and acts on the `clipboard_event` it sends; elsewhere it handles them itself.

- Change: `Platform::dispatcher` hands out the host's threads as one `Send + Sync` object: any thread asks it for a turn at a time, answered by `EmbedderClient::wake`, and the framework hands it work to run off the main thread; Dart's isolate is woken by its message port and `Isolate.run` uses the VM's own threads.
  Reason: platform — the host owns the event loop and knows its system's work queues, and a task completed on another thread has no other way to give the app a turn.
  Affect: a spawned task can await a future completed on any thread and runs at the next checkpoint instead of at the next input, and `run_in_background` work lands on the system's queue where there is one.

- Change: `Platform::compositor` hands out native surfaces that a producer feeds with the platform's own buffers from any thread, in place of Flutter's texture registry and platform-view plumbing.
  Reason: platform — the framework, the host and every producer share one process, so a buffer can go to the host's compositor directly.
  Affect: a host without a compositor answers `None`, and a surface placed there leaves only a clear rectangle.

- Change: a surface is opened under a parent, a view's root (`Compositor::root`) or another surface, with `SurfaceParent::open_child`, and placed by that parent's next frame: a view's scene, or a surface's `SurfaceFrame`, its buffer with its children placed beneath it in the buffer's pixels.
  Reason: platform — every compositor Inset targets makes a layer under a parent and places it with that parent's frame, and a host that draws programs shows each program's picture as a surface whose own surfaces go beneath it, where Flutter's platform views are always the view's own.
  Affect: a child shows only where its parent's last present placed it, and moves, is clipped and is scaled with its parent.

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

## pointer_report.rs → the embedder API's `FlutterPointerEvent` (`embedder.h`), copied into `PointerData` as `FlutterEngineSendPointerEvent` does

- Change: a host reports a `PointerReport`, whose `ReportedChange` carries only the fields its change uses (buttons, a scroll's delta, a pan-zoom's totals), where `FlutterPointerEvent` is one flat struct whose phase and signal kind say which of its fields count.
  Reason: platform — the report is all the embedder interface takes from a host about a pointer, and the enum leaves a host no field the converter fills or its change ignores.
  Affect: a host cannot set a press's identifier, a delta or `synthesized`, nor give a press a scroll delta; it reports what the system said, and the client's converter adds the rest.

## pointer_data_packet_converter.rs → engine `lib/ui/window/pointer_data_packet_converter.cc`

- Change: a move, an up, a remove, or a pan-zoom's update or end for a device the converter holds no state of is dropped, and the engine's `FML_DCHECK`s on the order of changes are not checked.
  Reason: language — the engine reads past the end of its map there, which C++ leaves undefined and Rust cannot do, and its checks run only in the engine's own unoptimized builds, where a `debug_assert!` would stop every app's debug build.
  Affect: a host that reports a press's end, or a gesture's update, for a device it never reported loses that report in every build, as the engine drops a cancel for such a device.

## client.rs → dart:ui `hooks.dart`

- Change: the isolate-global engine hooks are methods on `EmbedderClient`: one `frame` is a complete engine frame of the views one refresh names, pointer reports and key data arrive as calls, view lifecycle is a typed notification, and `key_data` returns whether the framework handled the key.
  Reason: platform — there is no isolate-global hooks table; the host drives a client without naming `App`.
  Affect: the host does not run scheduler phases or build `PointerEvent` / `KeyEvent` itself, and decides from `key_data`'s answer whether to keep propagating the native event.

- Change: `EmbedderClient::wake(elapsed)` reports that a `Platform::wake_at` deadline passed, on the clock `Frame::elapsed` uses.
  Reason: platform — the `App` owns its timers and has no event loop firing them.
  Affect: a `Timer` fires only when the host calls `wake` or delivers a frame, so a host that never wakes never runs one.

- Change: `platform_brightness_changed`, `locales_changed` and `screens_changed` carry no value; the new one is read back from `Platform`.
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

- Change: `EmbedderClient::open_requested` hands the client what the system asks the app to open, an `OpenRequest` of files and links, at launch and while it runs; Flutter's macOS embedder hands a running app's to its plugins through `FlutterAppLifecycleDelegate.handleOpenURLs:`, iOS's through `application:openURL:options:` and `scene:openURLContexts:`, and Android's through `onNewIntent`, while a launch's arguments reach `main`, where the app reads them as here.
  Reason: platform — there are no plugins, and the app is the only listener; one call covers every system's way, whose files are the file dialogs' `PickedFile`, as Android hands one over as a `content://` document and the web's `launchQueue` as a file handle.
  Affect: an app hears what it is asked to open in one place on every system, the launch's request included, which waits for its handler.

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

- Change: `menu_selected` reports an item of the menu bar chosen by its id, as the `Menu.selectedCallback` message does, and `dock_menu_selected` and `tray_icon_event` report the Dock menu's and a tray icon's, which Flutter has not.
  Reason: platform — there are no method channels, and each place's ids are its own, so each reports through its own hook.
  Affect: an id set in one place never reaches the handler of another.

- Change: `edit_command` reports Edit's undo, redo, select all and delete, chosen from a system's menu, for the view with the focus; no Flutter hook says so.
  Reason: platform — a provided Edit item acts on the focused view, as AppKit sends its action to the first responder, and copy, cut and paste already arrive as `clipboard_event`.
  Affect: a text field undoes, selects all or deletes when the user chooses it from the system's Edit menu, without the app handling the item.

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
- `StrutStyle`, `TextStyle.locale` / `ParagraphStyle.locale`. Trigger: strut, locale-specific glyphs; valo has neither.
- `ImageFilter.blur(tileMode:)` and the `dilate` / `erode` / `matrix` / `shader` filters. Trigger: valo growing those ops.
- `PlatformDispatcher.locale`, the first of `locales`. Trigger: a caller of the single-locale getter.
- The system font's optical size by text size, which CoreText applies for the engine. Trigger: valo-text setting variation axes from the text size.
- `FontFeature` named tag constructors. Trigger: a caller that uses those factories instead of `new` / `enable` / `disable`.
- `debugDefaultTargetPlatformOverride`. Trigger: a debug switcher that must override a live host without swapping `Platform`.
- `PointerData.respond` / `onRespond`. Trigger: a web host, where they call `preventDefault` on the originating DOM event.
- A report's `embedder_id`, which Flutter's Android embedder sets to the `MotionEvent`'s id so that a platform view can take back the system's touch, and every other embedder leaves at 0. Trigger: a platform view that takes back the system's touch.
- Semantics callbacks. Trigger: semantics.
- `PlatformDispatcher` callback setters and Zones. Trigger: a requirement to expose dart:ui callbacks independently of `Shell`.
- `_updateFrameData` and the `frameNumber` argument to `_beginFrame`. Trigger: `FrameData`.
- The rest of `hooks.dart`. Trigger: the matching dart:ui callback.

## clipboard.rs → `services/clipboard.dart`'s `Clipboard.setData`, `getData` and `hasStrings` on the platform channel

- Change: the host's clipboard holds items, each offered in several MIME types, given or promised, files given or promised, and how a copy is to be treated, where Flutter's channel carries plain text.
  Reason: platform — every system's clipboard holds items in several types, which an app reads and writes as the system's own apps do.
  Affect: an app can copy an image or a file, and read what another app copied in any type the system names.

- Change: a promised file produces its bytes into a sink the host hands it (`FileSink`), and the host writes them where the receiver chose.
  Reason: platform — Windows and Android take a promised file as a stream, which macOS's write at a path is a copy of, while a path would need a temporary file to stream.
  Affect: an app promising a file never learns where the receiver put it, and one in a sandbox writes nothing outside it.

- Change: the host tells the client when the clipboard changed (`clipboard_changed`) and when the user copied, cut or pasted through the platform's own command (`clipboard_event`), which Flutter's engines never say.
  Reason: platform — where only the user's own paste may read the clipboard, the paste's content comes with the host's word of it.
  Affect: the framework learns of a copy made elsewhere, and pastes what the user pasted even where it may not read the clipboard itself.

## menus.rs → services `system_channels.dart`'s `SystemChannels.menu` (`Menu.setMenus`, `Menu.selectedCallback`), `platform_menu_bar.dart`'s `PlatformProvidedMenuItemType`

- Change: a menu is a value of `MenuEntry`s (the app's items, the system's provided items, submenus and separators) set whole at each place the system draws menus: the menu bar (`Platform::menu_bar`), a menu at a point in a view (`popup_menus`), the Dock icon's (`dock_menu`) and a tray icon's; Flutter's `flutter/menu` channel carries the menu bar alone, as maps.
  Reason: platform — every place a system draws a menu takes the same tree, and there are no method channels.
  Affect: an app builds each menu from its state and sets it again when the state changes, and a place the system lacks answers `None`, where the app draws its own.

- Change: an item carries a check (a checkbox on, off or mixed, or a radio on or off) and an icon, which Flutter's serialized `PlatformMenuItem` does not.
  Reason: platform — AppKit, Win32 and the DBus menu draw both, and only a menu the framework draws itself could draw them otherwise.
  Affect: a native menu shows check marks and icons; Windows, which has no mixed state, shows a mixed one unchecked.

- Change: `ProvidedItem` adds Edit's undo, redo, cut, copy, paste, select all and delete to Flutter's provided items, and leaves out the speaking ones; each acts as the system's own, Edit's sent to the focused view as `clipboard_event` and `edit_command`, and an item the system has no counterpart for is left out of its menu.
  Reason: platform — a system knows its Edit menu by its standard items, as macOS adds dictation and emoji to it, and a host never stands in for an item its system lacks.
  Affect: Edit's items work in every text field without the app handling them, and the same menu shows fewer provided items on Windows and Linux than on macOS.

- Change: the macOS menu bar shows the system's standard menus, the application's, Edit and Window, until the app sets its own, where Flutter's macOS embedder shows the one in the app's `MainMenu.xib`.
  Reason: platform — an app here has no nib, and one without a menu bar would have no Quit, Hide or Edit.
  Affect: an app that sets no menu bar still quits with ⌘Q and copies with ⌘C, and one that sets its own replaces the standard menus whole.

- Change: `PopupMenus::show` shows the system's own menu at a point in a view and answers the chosen item's id once it closes, as a future; Flutter has no such call, only in-view context menus and the macOS menu bar.
  Reason: platform — a window narrower than its menu, such as a panel at a screen's edge, cannot hold a menu drawn inside its view.
  Affect: the menu is the system's, drawn outside the window, and Linux answers `None`: its desktops have no menu of the system's, as GTK and Qt draw theirs in a pop-up surface, which winit offers from 0.31, where a framework can draw them.

- Change: `TrayIcons::add` shows an icon in the system's tray with an image and a tooltip, removed when its handle drops, and reports what the system tells of it as `TrayIconEvent`s; its menu is one the system opens by itself (`attached_menu`, macOS and Linux) or one the app opens at it (`menu_at_icon`, macOS and Windows), each `None` where the system has not; Flutter has no tray.
  Reason: platform — each system offers some of these and decides when a tray's menu opens, so a convention such as Windows' right click opening a menu is the app's.
  Affect: an app on Windows opens a menu when it hears `ContextMenuRequested`, and on macOS a click on an icon with an attached menu opens the menu and reports nothing.

- Change: `TrayIcons::add` answers the icon as a future once it exists, or a `TrayIconError` where the system did not show it, as `PopupMenus::show` and `Clipboard::write` answer theirs; the icon then says which menus it offers.
  Reason: platform — a host may make the icon elsewhere, on another thread or in another process, as WaOS makes a program's, and a system may refuse it, as a Linux desktop's bus can.
  Affect: an app has an icon, and knows which menus it offers, only once the future answers; a native host answers at once.

- Change: `MenuBar::perform_key_equivalent` takes a key-down the app claimed when it came and leaves later, as Chrome's browser process sends on to the menu a key its page did not consume; Flutter's engines answer every key at once.
  Reason: platform — AppKit wants a key's answer before its call returns, and an app that waits on another thread, as WaOS waits on a program, answers later.
  Affect: a key a program leaves still chooses the menu bar's item, from the system's own event, until the host lets go of that event a few seconds later.

- Change: `DockMenu::set_menu` sets the items macOS adds to the Dock icon's menu; Flutter has none.
  Reason: platform — AppKit asks the application's delegate for it, which only the host is.
  Affect: an app's Dock menu is set whole like any other, and every other system answers `None`.

## opener.rs → the `url_launcher` package's `launchUrl` and `canLaunchUrl`

- Change: opening is a host capability, `Platform::opener`, with a call for each kind, `open_link`, `open_file` and `reveal_file`, as Electron's `shell` and Tauri's opener have them, where `url_launcher` takes one URI.
  Reason: platform — phones share a file through the system (Android's `FileProvider`, iOS's `UIDocumentInteractionController`) rather than by a `file://` URI, and only a file can be revealed.
  Affect: an app names a file by its path, and a host without a file manager answers `reveal_file` with a failure.

- Change: `Opener::can_open_link` answers `None` where the system will not say, where `canLaunchUrl` answers false.
  Reason: platform — iOS answers only for the schemes the app declares, Android only for the intents its manifest's `<queries>` names, and Linux's portal not at all, since asking about many schemes tells which apps the user has.
  Affect: an app learns that nothing opens a link when opening it fails with `OpenError::NoApp`.

- Change: opening answers `Result<(), OpenError>` through a future, `NoApp` where no app takes the link or file, where `launchUrl` answers a bool.
  Reason: platform — the systems tell no app from another failure, and Windows' and Linux's ask a service that answers later.
  Affect: an app can tell the user that nothing opens a kind of link or file.

## notifications.rs → the `flutter_local_notifications` package's `show`, `zonedSchedule`, `cancel`, `requestPermissions` and `onDidReceiveNotificationResponse`

- Change: notifications are a host capability, `Platform::notifications`, and a notification is a value posted under a string id, which a later post under the same id replaces, where the plugin shows one under an int id over method channels.
  Reason: platform — there are no plugins, and each system's own model is a notification under a name of the app's (a request's identifier on Apple's, Android's tag, the web's tag, a Windows toast's tag, Linux's `replaces_id`), which a string reaches on every one.
  Affect: an app names its notifications by strings, and a host where the system would not show the app's answers `None`, as macOS does for a program outside an application bundle.

- Change: a typed reply (`replies`), delivery at a later time (`scheduling`) and the user's permission (`permission`) are capabilities, each `None` where the system lacks it, and a reply action is made only through `NotificationReplies::reply_action`, where the plugin offers each everywhere and fails where the system has none.
  Reason: platform — Linux and the web hold no notification for later, Linux takes a reply only where the server advertises KDE's `inline-reply` and the web none without a service worker, and Linux keeps no permission for an app.
  Affect: an app offers a reply field or a reminder only where `replies()` or `scheduling()` answers, and has a reply action only there.

- Change: `post` and `schedule` never ask the user: where the system does not let the app notify, because the user was never asked or said no, they answer `NotificationError::Denied`, and the app asks first with `NotificationPermission::request`, the plugin's `requestPermissions`; the plugin's `show` answers nothing either way.
  Reason: platform — no system's own post asks (`UNUserNotificationCenter`'s `add` fails with `notificationsNotAllowed`, Android 13's `notify` and the web's `Notification` show nothing), and only the system's own request does.
  Affect: an app asks before its first post, and learns from a post's answer that it was not shown.

- Change: a click on a notification, one of its actions or a reply reaches the app as `EmbedderClient::notification_activated(NotificationActivation { id, action, reply })`, the click that started the app included, where the plugin hands a running app's to `onDidReceiveNotificationResponse` and the launch's to `getNotificationAppLaunchDetails`.
  Reason: platform — there are no plugins, and the client is the only listener; the launch's click arrives once the client exists, as a launch's open request does.
  Affect: an app hears every click in one place, by the notification's id and the action's, the launch's waiting for its handler.

## file_dialogs.rs → the `file_selector` package's `openFile`, `openFiles`, `getDirectoryPath`, `getSaveLocation`, and cross_file's `XFile`

- Change: the system's file dialogs are a host capability, `Platform::file_dialogs`, with one function a kind, where Flutter reaches them through the `file_selector` plugin over method channels.
  Reason: platform — there are no plugins and no method channels; the host trait is the channel.
  Affect: an app asks the platform for its dialogs, and a host without them answers `None`.

- Change: a pick is a `PickedFile`: a path where the system gives one, the system's handle where it gives none (the web; Android where the document seeks), and a stream where the system gives a pipe (Android), where Flutter's `XFile` holds a path or a blob, reading only; `PickedFile::read_all` is `XFile.readAsBytes`.
  Reason: platform — Android's documents and the web's files have no path, and a copy into the app's cache could not be written back.
  Affect: an app reads and writes a handle at any offset where the system gives no path, and reads a stream through once.

- Change: `PickedFile::url` answers the URL the system reads a pick by: a percent-encoded `file:` URL for a path, Android's `content:` URI, the web's `blob:` URL, where `XFile.path` is a path or a blob URL by platform.
  Reason: platform — every system's player and every API that takes a URL reads a pick by it, and a pick without a path has a URL all the same.
  Affect: an app hands any pick to a player, or another API that takes a URL, without reading it first.

- Change: a folder pick is a `PickedFolder`: a path where the system gives one, and the system's folder handle where it gives none (the web's directory handle, Android's document tree), which lists the folder, opens its files and folders by name, and makes them when writable, where `getDirectoryPath` answers a path or nothing.
  Reason: platform — neither the web nor Android gives a folder a path, and a copy could not be written back.
  Affect: an app reaches a picked folder's files through the handle on the web and Android, and through the path elsewhere.

- Change: an open pick grants reading only unless the app asks to write (`OpenOptions::writable`), and a save grants writing to the one file chosen; `file_selector` has no such option.
  Reason: platform — Android grants write only with `FLAG_GRANT_WRITE_URI_PERMISSION`, and the web only when asked for `readwrite`.
  Affect: a handle opens for writing only when the pick asked to write.

- Change: every dialog takes the parent view whose window it belongs to, which macOS shows it as a sheet on.
  Reason: platform — a file dialog belongs to a window, which a view-based interface names by its view.
  Affect: a dialog opened for a window stays with it on macOS, and stands alone without one.

## media.rs → the `video_player` package's `VideoPlayerController` and the `camera` package's `CameraController`, through their platform interfaces

- Change: players and cameras are host capabilities, `Platform::players` and `Platform::cameras`: a player is one host object made from a URL, a link or a pick's `PickedFile::url`, loading by itself, controlled and observed through its state, and a camera a session on one device with an output per use, where Flutter's plugins have controllers that are initialized over method channels.
  Reason: platform — the host is the system's media stack, and its players and capture sessions are that model on every system Inset targets.
  Affect: an app makes a player and calls it at once, without awaiting an initialization, and a system without players or cameras answers `None`.

- Change: a player's or a camera's picture goes to one destination at a time, a surface it is given and the system draws into (`set_video_surface`, `set_preview_surface`, as Media3's `setVideoSurface`; `None` for none), or a frame reader handing out the platform's own pixel buffers, where Flutter's plugins register a texture the engine copies from.
  Reason: platform — protected and HDR video show only where the system draws them itself, and Windows' and Android's decoders feed one output.
  Affect: a player shown on a surface has no frames to read, and one read as frames shows on no surface.

- Change: a camera's photos are answered encoded by the system, as bytes and their MIME type, and its recordings written by the system to a path the app gives, where the `camera` plugin answers an `XFile` in a folder of its own.
  Reason: platform — the system encodes and writes them, and an app chooses where its files go.
  Affect: a photo arrives as JPEG bytes, and a recording is a QuickTime movie at the app's path.

