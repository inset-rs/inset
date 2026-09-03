# reveal-shell/src

No Flutter counterpart. Dart has no type — the engine owns the isolate.

## lib.rs

- Change: [`Shell`](Shell) owns [`App`](reveal_foundation::App) and is the [`EmbedderClient`](reveal_embedder::EmbedderClient). It lives above scheduler and gestures.
  Reason: platform — the host talks to a client and must not name `App`; a lower crate cannot name a higher binding.
  Affect: the host's start closure returns `Shell::new(platform, setup)`.

- Change: the host delivers one frame and one pointer packet to `Shell`. `frame` runs begin-frame, the mid-frame microtask flush, and draw-frame. `pointer_data_packet` calls [`GestureBinding::handle_pointer_data_packet`](reveal_gestures::GestureBinding::handle_pointer_data_packet) and drains microtasks. Dart assigns those handlers onto `PlatformDispatcher` callback fields.
  Reason: platform — there is no isolate-global callback table.
  Affect: a host calls `client.frame(...)` / `client.pointer_data_packet(...)`. Tests may still pump the handlers directly.

## Deferred

- `view_added` / `view_metrics_changed` / `view_removed`. Trigger: `RendererBinding`.
