# reveal-shell/src
Syntax (constructors, setters, `Option`, erasure calls) follows `.cursor/skills/porting-flutter/patterns/widget-syntax.md` and is not a divergence.

No Flutter counterpart. Dart has no type — the engine owns the isolate.

## lib.rs

- Change: `Shell` owns the `App` and is the `EmbedderClient`; it lives above scheduler and gestures.
  Reason: platform — the host talks to a client and must not name `App`; a lower crate cannot name a higher binding.
  Affect: the host's start closure returns `Shell::new(platform, setup)`.

- Change: the host delivers frames, pointer packets and key data to `Shell` as calls: `frame` runs begin-frame, the mid-frame microtask flush and draw-frame; `pointer_data_packet` hands the packet to `GestureBinding` and drains microtasks; `key_data` hands it to `KeyEventManager`, drains, and returns what the manager answered. Dart assigns those handlers onto `PlatformDispatcher` callback fields.
  Reason: platform — there is no isolate-global callback table; the host reads the key answer from the call.
  Affect: a host calls `client.frame(..)`, `client.pointer_data_packet(..)` and `client.key_data(..)`, learning from the last whether a `HardwareKeyboard` handler took the key; tests may still pump the handlers directly.

- Change: `Shell::new` installs the platform's fonts into `PaintingBinding` before `setup` runs.
  Reason: platform — Flutter's engine collects platform fonts on its own; here the shell asks `Platform::font_source` once.
  Affect: text shapes against the OS fonts without application code; a test shell with the inert platform has an empty collection until `PaintingBinding::install_fonts`.

- Change: `Shell` keeps the app clock: `frame` and `wake` first `App::elapse` up to the platform's `elapsed`, so due `Timer`s fire before the frame's scheduler phases.
  Reason: platform — Dart's event loop runs timers between engine events; the `App` clock only moves when the host says so.
  Affect: a `Timer` (and so `run_app`'s root attach) fires at the next wake or frame, never during `setup`.

## Deferred

- `view_added` / `view_removed`. Trigger: the widget layer's `View`; `RendererBinding` does not create render views for new host views, while `view_metrics_changed` already reaches `RendererBinding::handle_metrics_changed`.
