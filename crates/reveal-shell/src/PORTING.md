# reveal-shell/src

No Flutter counterpart. Dart has no type — the engine owns the isolate.

## lib.rs

- Change: [`Shell`](Shell) owns [`App`](reveal_foundation::App) and is the [`EmbedderClient`](reveal_embedder::EmbedderClient). It lives above scheduler and gestures.
  Reason: platform — the host talks to a client and must not name `App`; a lower crate cannot name a higher binding.
  Affect: the host's start closure returns `Shell::new(platform, setup)`.

- Change: the host delivers one frame and one pointer packet to `Shell`. `frame` runs begin-frame, the mid-frame microtask flush, and draw-frame. `pointer_data_packet` calls [`GestureBinding::handle_pointer_data_packet`](reveal_gestures::GestureBinding::handle_pointer_data_packet) and drains microtasks. Dart assigns those handlers onto `PlatformDispatcher` callback fields.
  Reason: platform — there is no isolate-global callback table.
  Affect: a host calls `client.frame(...)` / `client.pointer_data_packet(...)`. Tests may still pump the handlers directly.

- Change: `Shell::new` installs the platform's fonts into `PaintingBinding` before `setup` runs.
  Reason: platform — Flutter's engine collects platform fonts on its own; here the shell asks `Platform::font_source` once.
  Affect: text shapes against the OS fonts without application code; a test shell with the inert platform has an empty collection until `PaintingBinding::install_fonts`.

- Change: `Shell` keeps the app clock: `frame` and `wake` first `App::elapse` up to the platform's `elapsed`, so due `Timer`s fire before the frame's scheduler phases.
  Reason: platform — Dart's event loop runs timers between engine events; the `App` clock only moves when the host says so.
  Affect: a `Timer` (and so `run_app`'s root attach) fires at the next wake or frame, never during `setup`.

## Deferred

- `view_added` / `view_removed`. Trigger: the widget layer's `View`; `RendererBinding` does not create render views for new host views. `view_metrics_changed` reaches `RendererBinding::handle_metrics_changed`.
