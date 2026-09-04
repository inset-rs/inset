# reveal-embedder/src
No Flutter crate in this shape. Closest analogue: dart:ui `PlatformDispatcher` / `hooks.dart` / `FlutterView`.

## Identical

- text.rs → text.dart (`FontStyle`, `FontWeight`, `TextAlign`, `TextBaseline`, `TextDecoration`, `TextDecorationStyle`, `TextLeadingDistribution`, `TextHeightBehavior`, `TextDirection`, `kTextHeightNone`, `FontFeature` core, `FontVariation` core + named axes)

## painting.rs → dart:ui `Canvas` / `Paint` / `Paragraph` / `TextStyle`

- Change: [`TextStyle`](TextStyle) is valo `TextStyle`. There is no `Int32List` encode.
  Reason: platform — there is no Dart/C++ language boundary; `ParagraphBuilder` takes valo styles.
  Affect: pass a valo `TextStyle` into paragraph building. There is no encoded buffer.

## text.rs → text.dart (`FontFeature` / `FontVariation`)

- Change: `_encode` is omitted on [`FontFeature`](FontFeature) / [`FontVariation`](FontVariation).
  Reason: platform — there is no engine FFI; the values are used as themselves.
  Affect: there is no encode into a `ByteData` buffer.

## paragraph.rs → text.dart (`TextStyle`, `ParagraphStyle`, `ParagraphBuilder`, `Paragraph`), fonts.rs

- Change: fonts are explicit. [`Platform::font_source`](Platform::font_source) is the host's font lookup (Flutter's engine asks the OS itself), and [`ParagraphBuilder::build`](ParagraphBuilder::build) takes the [`FontCollection`](FontCollection) to shape against; the framework keeps that collection (painting's `PaintingBinding`) and hands it down.
  Reason: platform — Flutter's engine holds one font manager per process; valo shapes against a collection the caller owns, and a hidden global would tie every paragraph to one thread's state.
  Affect: a host implements `font_source`; `builder.build(&mut fonts)` where Dart writes `builder.build()`.

- Change: [`Paragraph`](Paragraph) and [`ParagraphBuilder`](ParagraphBuilder) wrap valo's. Offsets are UTF-16 code units as in Dart, mapped onto valo's UTF-8 bytes. `push_style` inherits the unset fields of the style below it, as the engine does. There are no placeholders: `add_placeholder` does not exist and placeholder boxes are empty. `get_boxes_for_range` returns tight boxes whatever `BoxHeightStyle` / `BoxWidthStyle` say, each with the paragraph's direction. `ideographic_baseline` is the first line's bottom.
  Reason: platform — valo's paragraph has no placeholders, box styles, per-box direction, or ideographic metrics.
  Affect: `TextBox.direction` is the paragraph's, not the run's; a `WidgetSpan` cannot be laid out; `BoxHeightStyle::Strut` and `Max` read as `Tight`.

- Change: valo runs have no `decorationStyle`, background paint, `fontFeatures`, `fontVariations`, `textBaseline`, or `leadingDistribution`; a combined decoration paints underline, else overline, else line-through. Unset `fontSize` shapes at valo's 14.
  Reason: platform — those are not valo span fields.
  Affect: setting them on a `TextStyle` has no visible effect.

- Change: `Canvas.drawParagraph` is [`CanvasText::draw_paragraph`](CanvasText::draw_paragraph), a trait on [`Canvas`](Canvas).
  Reason: language — `Canvas` is valo's builder under a Flutter name; a method needs a trait.
  Affect: `use reveal_embedder::CanvasText` at the call site.

## platform.rs → dart:ui `platform_dispatcher.dart`

- Change: Dart's isolate-global `PlatformDispatcher` is a host-supplied `Platform` that `App` holds. Frame, clock, and view lookup are requests on that object — not a callback table the framework assigns into.
  Reason: platform — there is no isolate-global dispatcher; the host supplies `Platform`.
  Affect: outgoing host requests go through `app.platform()`.

- Change: [`TargetPlatform`](TargetPlatform) is a value type here; [`Platform::target_platform`](Platform::target_platform) reports the current one. There is no `defaultTargetPlatform` library global and no `debugDefaultTargetPlatformOverride`. `InertPlatform` answers Android, matching Flutter's test binding.
  Reason: platform — there is no isolate-global `dart:io`; the host-supplied `Platform` is the source of truth, so two Apps can differ.
  Affect: write `app.platform().target_platform()` where Dart writes `defaultTargetPlatform`. Tests that need iOS (etc.) install a `Platform` that returns that value.

- Change: [`Brightness`](Brightness) is a value type here; [`Platform::platform_brightness`](Platform::platform_brightness) reports the current one. There is no `PlatformDispatcher.platformBrightness` library global. The trait default is light, matching Flutter's view configuration. `InertPlatform` uses that default.
  Reason: platform — there is no isolate-global dispatcher; the host-supplied `Platform` is the source of truth.
  Affect: write `app.platform().platform_brightness()` where Dart writes `PlatformDispatcher.instance.platformBrightness`.

- Change: [`Platform::activate_system_cursor`](Platform::activate_system_cursor) is the `activateSystemCursor` message of Flutter's `SystemChannels.mouseCursor`; it takes a [`SystemMouseCursorKind`](SystemMouseCursorKind) and defaults to doing nothing.
  Reason: platform — there are no method channels, so the kind needs no string encoding; a host capability is a method on `Platform`.
  Affect: `reveal-services` calls it with the cursor's kind; a host that shows cursors overrides it.

## views.rs → dart:ui `FlutterView` / `ViewPadding` / `ViewConstraints`

- Change: `View` is a handle to the native surface. The host owns the window and answers metrics from it; the framework does not keep a copy.
  Reason: platform — the native window is the source of truth; a framework-side copy would go stale.
  Affect: after a view-lifecycle notification, read `view.metrics()` from the host's current `View`.

- Change: `View::present` takes a valo `Picture` (`DisplayList`). Flutter `FlutterView.render` takes a `Scene`. `Canvas` is `DisplayListBuilder`.
  Reason: platform — the host paints a valo display list, not an engine `Scene`.
  Affect: `view.present(&picture)` after recording into a `Canvas`.

## client.rs → dart:ui `hooks.dart`

- Change: isolate-global engine hooks are methods on `EmbedderClient`. One `frame` is a complete engine frame; view lifecycle is typed notifications. The client owns begin/draw choreography.
  Reason: platform — there is no isolate-global hooks table; the host drives a client without naming `App`.
  Affect: the host delivers frames and view events; it does not run scheduler phases itself.

- Change: `PlatformDispatcher.onPointerDataPacket` is [`EmbedderClient::pointer_data_packet`](EmbedderClient::pointer_data_packet).
  Reason: platform — same as the other hooks: the host calls the client instead of assigning into an isolate-global callback field.
  Affect: the host delivers a [`PointerDataPacket`](PointerDataPacket); it does not convert to `PointerEvent`.

- Change: `EmbedderClient::wake(elapsed)` reports that a `Platform::wake_at` deadline passed, on the platform clock `Frame::elapsed` uses.
  Reason: platform — Dart's event loop fires `Timer`s on its own; the `App` owns its timers and needs the host to say time passed.
  Affect: a host calls `wake` when its wait ends; the shell turns it into `App::elapse`.

## pointer.rs → dart:ui `pointer.dart`

- Change: [`PointerData::view_id`](PointerData::view_id) is [`ViewId`](ViewId), not a bare `int`.
  Reason: platform — views are already `ViewId` on this host surface.
  Affect: fill `view_id: view.id()`, not a raw integer.

- Change: `PointerData.respond` / `onRespond` are omitted.
  Reason: platform — they exist to call `preventDefault` on the web DOM event that produced the sample.
  Affect: there is no `pointer_data.respond(...)`.

## Deferred
- `StrutStyle` / `ParagraphStyle.strutStyle`, `Locale`, `ParagraphBuilder.addPlaceholder` / `placeholderScales`, `Paragraph.getBoxesForPlaceholders` contents. Trigger: strut, locale-specific glyphs, `WidgetSpan`. valo has none of them.

- `onMetricsChanged` / `onPlatformBrightnessChanged`. Trigger: `MediaQuery` / `CupertinoTheme`.
- `FontFeature` named tag constructors (`alternative`, `fractions`, …). Trigger: a caller that uses those factories instead of `new` / `enable` / `disable`.
- `font_source`. Trigger: text.
- `debugDefaultTargetPlatformOverride`. Trigger: a debug switcher that must override a live host without swapping `Platform`.
- Semantics callbacks. Trigger: semantics.
- `PlatformDispatcher` callback setters and Zones. Trigger: a requirement to expose dart:ui callbacks independently of `Shell`.
- `_updateFrameData` / the `frameNumber` argument to `_beginFrame`. Trigger: `FrameData`.
- The rest of `hooks.dart`. Trigger: the matching dart:ui callback.
