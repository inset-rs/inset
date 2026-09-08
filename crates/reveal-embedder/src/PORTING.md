# reveal-embedder/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.
No Flutter crate in this shape. Closest analogue: dart:ui `PlatformDispatcher` / `hooks.dart` / `FlutterView`.

## Identical

- key.rs → dart:ui key.dart
- text.rs → dart:ui text.dart (the text and font enums, `FontFeature`, `FontVariation`)
- locale.rs → dart:ui `Locale`

## painting.rs → dart:ui `Canvas` / `Paint` / `Paragraph` / `TextStyle`

- Change: `ImageFilter` is an enum over the filters this host can replay — a blur, a colour filter (Dart's `ColorFilter implements ImageFilter`) and a composition of two; the tile-mode blur and the dilate, erode, matrix and shader filters are absent.
  Reason: platform — valo's display list has a backdrop blur, a colour filter and a composition, and no other image filter.
  Affect: `ImageFilter::blur(sx, sy)` (plus `.bounds(rect)`), `ImageFilter::Color(filter)`, `ImageFilter::compose(outer, inner)`; a caller that needs another filter waits for valo. What a host's backdrop can show of a composition is recorded in `reveal-embedder-winit`'s `PORTING.md`.

## paragraph.rs → text.dart (`TextStyle`, `ParagraphStyle`, `ParagraphBuilder`, `Paragraph`), fonts.rs

- Change: fonts are explicit. `Platform::font_source` is the host's font lookup, and `ParagraphBuilder::build` takes the `FontCollection` to shape against, which the framework keeps (painting's `PaintingBinding`) and hands down.
  Reason: platform — Flutter's engine holds one font manager per process; valo shapes against a collection the caller owns.
  Affect: a host implements `font_source`; `builder.build(&mut fonts)` where Dart writes `builder.build()`.

- Change: `SystemFontSource` is the engine's font manager: valo's `FontManager` (the platform's font API in Skia's `SkFontMgr` shape) behind `FontSource`, answering `CupertinoSystemText` and `CupertinoSystemDisplay` with every weight of the platform's user-interface font at 17 and 29 points, as the engine's `platform_mac.mm` registers them.
  Reason: platform — the engine registers those names into its own font manager at startup; here the source answers them on demand, and a host with its own lookup (a guest across a boundary) passes a manager instead of the platform's.
  Affect: a host returns `SystemFontSource::platform()` (or `::new(manager)`) from `font_source`, and Cupertino text shapes with the system font on every platform that has one. The optical size the engine gets from CoreText (SF Pro Text below 29 points, Display above) is not applied: both names answer the same variable face at its default optical size.

- Change: `Paragraph` and `ParagraphBuilder` wrap valo's, with offsets kept as UTF-16 code units. There are no placeholders, the box styles are ignored (every box is tight and carries the paragraph's direction), and the ideographic baseline is the first line's bottom.
  Reason: platform — valo's paragraph has no placeholders, box styles, per-box direction or ideographic metrics.
  Affect: `add_placeholder` does not exist, so a `WidgetSpan` cannot be laid out; `TextBox.direction` is the paragraph's, not the run's; `BoxHeightStyle::Strut` and `Max` read as `Tight`.

- Change: a run has no decoration style, background paint, font features or variations, baseline, or leading distribution; a combined decoration paints underline, else overline, else line-through, and an unset `fontSize` shapes at valo's 14.
  Reason: platform — those are not valo span fields.
  Affect: setting them on a `TextStyle` has no visible effect.

## platform.rs → dart:ui `platform_dispatcher.dart`

- Change: Dart's isolate-global `PlatformDispatcher` is a host-supplied `Platform` that `App` holds; frame, clock and view lookup are requests on that object, not a callback table the framework assigns into.
  Reason: platform — there is no isolate-global dispatcher; the host supplies `Platform`.
  Affect: outgoing host requests go through `app.platform()`.

- Change: `defaultTargetPlatform` and `platformBrightness` are queries on `Platform`; there is no library global and no `debugDefaultTargetPlatformOverride`. The inert platform answers Android and light, as Flutter's test binding and view configuration do.
  Reason: platform — the host-supplied `Platform` is the source of truth, so two Apps can differ.
  Affect: a test that needs iOS installs a `Platform` that says so.

- Change: the system channels and dispatcher fields that reach the host are methods on `Platform`, each defaulted to do nothing or to answer a neutral value: the mouse cursor, restoration get and put, the locales and the application locale, the default route name, the application switcher description, the system UI overlay style and haptic feedback. The payloads cross as typed values, so Dart's `_toMap` string encodings are gone, and the argument-less `HapticFeedback.vibrate()` is the `Vibrate` kind.
  Reason: platform — there are no method channels; the host trait is the channel.
  Affect: `reveal-services` and the widget layer call them; a host with the capability overrides the method, and one without leaves the default — restoration stays off, the initial route is `/`, and the locale list is empty until the host reports it.

## restoration.rs → services `message_codecs.dart` (`StandardMessageCodec`)

- Change: `RestorationData` is a value enum over the kinds `StandardMessageCodec` can carry, `RestorationMap` is Dart's `Map<Object?, Object?>`, and `RestorationUpdate` is the `{enabled, data}` reply of the channel's `get`. There is no encode or decode; the types live here because `Platform` names them.
  Reason: platform — the framework and the host meet at a Rust trait, not at a byte channel, so the data travels as itself.
  Affect: a host keeps the `RestorationMap` it is given and hands the same value back.

## views.rs → dart:ui `FlutterView` / `ViewPadding` / `ViewConstraints`

- Change: `View` is a handle to the native surface; the host owns the window and answers metrics from it, and the framework keeps no copy.
  Reason: platform — the native window is the source of truth; a framework-side copy would go stale.
  Affect: after a view-lifecycle notification, read `view.metrics()` from the host's current `View`.

- Change: `View::present` takes a valo `Picture` where `FlutterView.render` takes a `Scene`, and `Canvas` is valo's `DisplayListBuilder`.
  Reason: platform — the host paints a valo display list, not an engine `Scene`.
  Affect: `view.present(&picture)` after recording into a `Canvas`.

- Change: text input is methods on `View` (`start_text_input`, `stop_text_input`, `set_text_input_editing_state`, composing and caret rects, client geometry), each defaulted to do nothing.
  Reason: platform — there are no method channels; the host trait is the channel.
  Affect: `TextInput` will call them; a host with an IME overrides the methods, and one without leaves the default.

## client.rs → dart:ui `hooks.dart`

- Change: the isolate-global engine hooks are methods on `EmbedderClient`: one `frame` is a complete engine frame, pointer packets and key data arrive as calls, and view lifecycle is typed notifications. `key_data` answers whether the framework handled the key.
  Reason: platform — there is no isolate-global hooks table; the host drives a client without naming `App`.
  Affect: the host delivers frames, packets, key data and view events, and does not run scheduler phases or convert to `PointerEvent` / `KeyEvent` itself; it reads `key_data`'s bool to decide whether to keep propagating the native event.

- Change: `EmbedderClient::wake(elapsed)` reports that a `Platform::wake_at` deadline passed, on the clock `Frame::elapsed` uses.
  Reason: platform — Dart's event loop fires `Timer`s on its own; the `App` owns its timers and needs the host to say time passed.
  Affect: a host calls `wake` when its wait ends; the shell turns it into `App::elapse`.

- Change: `platform_brightness_changed` and `locales_changed` are `onPlatformBrightnessChanged` and `onLocaleChanged`; the new value is read back from `Platform`, not carried by the call.
  Reason: platform — as with `frame`, the host pushes a notification and the framework reads the dispatcher.
  Affect: a host updates what its `Platform` answers before calling the hook; `MediaQuery` observers, and so `CupertinoTheme`, then rebuild.

- Change: `text_input_editing_value`, `text_input_action`, and `text_input_closed` are `TextInputClient.updateEditingValue` / `performAction` / `connectionClosed`, each defaulted to do nothing.
  Reason: platform — there are no method channels; the host trait is the channel.
  Affect: a host that has an IME calls them; `Shell` turns them into `TextInput` singleton methods.

## text_editing.rs → services `text_editing.dart`

- Change: `TextSelection` lives here because `View` names `TextEditingValue`, which holds it.
  Reason: platform — the host trait is the channel, so the payloads live with the trait.
  Affect: painting and rendering import it from `reveal-embedder`; widgets still use the services re-export.

## text_input.rs → services `text_input.dart`

- Change: `TextEditingValue`, `TextInputConfiguration`, and the neighbouring value types live here because `View` and `EmbedderClient` name them. `toJSON` / `fromJSON` are omitted.
  Reason: platform — there are no method channels; the host trait is the channel, so the data travels as itself.
  Affect: a host receives a `TextEditingValue`, not a map; framework callers still write `reveal_services::TextEditingValue`.

## pointer.rs → dart:ui `pointer.dart`

- Change: `PointerData.respond` / `onRespond` are omitted.
  Reason: platform — they exist to call `preventDefault` on the web DOM event that produced the sample.
  Affect: there is no `pointer_data.respond(..)`.

## Deferred

- `StrutStyle` / `ParagraphStyle.strutStyle`, `TextStyle.locale` / `ParagraphStyle.locale`, `ParagraphBuilder.addPlaceholder` / `placeholderScales`, `Paragraph.getBoxesForPlaceholders` contents. Trigger: strut, locale-specific glyphs, `WidgetSpan`; valo has none of them.
- `ImageFilter.blur(tileMode:)` and the `dilate` / `erode` / `matrix` / `shader` filters. Trigger: valo growing those ops.
- `PlatformDispatcher.locale`, the first of `locales`. Trigger: a caller of the single-locale getter.
- The system font's optical size (`opsz`) by text size, which CoreText applies for the engine. Trigger: valo-text setting variation axes from the text size.
- `FontFeature` named tag constructors (`alternative`, `fractions`, …). Trigger: a caller that uses those factories instead of `new` / `enable` / `disable`.
- `debugDefaultTargetPlatformOverride`. Trigger: a debug switcher that must override a live host without swapping `Platform`.
- Semantics callbacks. Trigger: semantics.
- `PlatformDispatcher` callback setters and Zones. Trigger: a requirement to expose dart:ui callbacks independently of `Shell`.
- `_updateFrameData` / the `frameNumber` argument to `_beginFrame`. Trigger: `FrameData`.
- The rest of `hooks.dart`. Trigger: the matching dart:ui callback.
