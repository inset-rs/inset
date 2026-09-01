# reveal-embedder/src
No Flutter crate in this shape. Closest analogue: dart:ui `PlatformDispatcher` / `hooks.dart` / `FlutterView`.

## platform.rs → dart:ui `platform_dispatcher.dart`

- Change: Dart's isolate-global `PlatformDispatcher` is a host-supplied `Platform` that `App` holds. Frame, clock, and view lookup are requests on that object — not a callback table the framework assigns into.
  Reason: platform — there is no isolate-global dispatcher; the host supplies `Platform`.
  Affect: outgoing host requests go through `app.platform()`.

- Change: [`TargetPlatform`](TargetPlatform) is a value type here; [`Platform::target_platform`](Platform::target_platform) reports the current one. There is no `defaultTargetPlatform` library global and no `debugDefaultTargetPlatformOverride`. `InertPlatform` answers Android, matching Flutter's test binding.
  Reason: platform — there is no isolate-global `dart:io`; the host-supplied `Platform` is the source of truth, so two Apps can differ.
  Affect: write `app.platform().target_platform()` where Dart writes `defaultTargetPlatform`. Tests that need iOS (etc.) install a `Platform` that returns that value.

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

## Deferred

- `onPointerDataPacket`. Trigger: gestures.
- `onMetricsChanged` / `onPlatformBrightnessChanged`. Trigger: `MediaQuery` / `CupertinoTheme`.
- `font_source`. Trigger: text.
- `debugDefaultTargetPlatformOverride`. Trigger: a debug switcher that must override a live host without swapping `Platform`.
- Semantics callbacks. Trigger: semantics.
- `PlatformDispatcher` callback setters and Zones. Trigger: a requirement to expose dart:ui callbacks independently of `Shell`.
- `_updateFrameData` / the `frameNumber` argument to `_beginFrame`. Trigger: `FrameData`.
- The rest of `hooks.dart`. Trigger: the matching dart:ui callback.
