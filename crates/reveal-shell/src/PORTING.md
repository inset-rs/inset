# reveal-shell/src

No Flutter counterpart. Dart has no type — the engine owns the isolate.

## lib.rs

- Change: `Shell` owns the `App` and is the `EmbedderClient` the host drives.
  Reason: platform — the host must not name `App`, and a lower crate cannot name a higher binding.
  Affect: there is no isolate-global binding a host can reach into; everything it does to the framework goes through the client it was handed.

- Change: the host delivers frames, pointer packets and key data as calls on `Shell` rather than by assigning handlers onto `PlatformDispatcher` callback fields, and `key_data` returns whether a `HardwareKeyboard` handler took the key.
  Reason: platform — there is no isolate-global callback table, and the host reads the key answer from the call.
  Affect: a host cannot swap the frame or pointer handler at run time, and learns from the `key_data` return whether to pass the key on.

- Change: `platform_brightness_changed` and `locales_changed` invoke the callbacks `WidgetsBinding` assigned on `App::platform_callbacks`, each in a turn of its own.
  Reason: platform — the shell cannot name `WidgetsBinding`, which lives above it.
  Affect: a host that reports a theme or locale change reaches `MediaQuery` observers without the shell knowing widgets.

- Change: `system_fonts_changed` notifies `PaintingBinding.systemFonts` in a turn of its own.
  Reason: platform — Dart's engine posts `fontsChange` on a system channel; here the host calls the client.
  Affect: paragraphs listening to `systemFonts` relayout after the host loads faces.

- Change: `Shell::new` installs the platform's fonts into `PaintingBinding` before `setup` runs.
  Reason: platform — Flutter's engine collects platform fonts on its own; here the shell asks `Platform::font_source` once.
  Affect: text shapes against the OS fonts with no application code, while a test shell on the inert platform has an empty collection until `PaintingBinding::install_fonts`.

- Change: `Shell` keeps the app clock — `frame` and `wake` advance it to the platform's `elapsed` first, so due `Timer`s fire before the frame's scheduler phases.
  Reason: platform — Dart's event loop runs timers between engine events; here the clock only moves when the host says so.
  Affect: a `Timer` (and so `run_app`'s root attach) fires at the next wake or frame, never during `setup`.

## Deferred

- `view_added` / `view_removed`. Trigger: the widget layer's `View`; `view_metrics_changed` already reaches `RendererBinding::handle_metrics_changed`.
