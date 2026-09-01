# reveal-embedder-winit/src

No Flutter counterpart. Flutter's engine is C++ and is not in this checkout.

## lib.rs / window.rs

- Change: this crate is a host: it implements `Platform` and `View`, creates the implicit window if configured, then hands `Platform` to a start closure that returns the client. It never names `App`.
  Reason: platform — Flutter's native embedding does not name the framework; winit cannot create a window until `resumed`.
  Affect: `WinitEmbedder::run(|platform| Shell::new(platform, setup))`.

- Change: the scheduler asks for one frame for the whole app; the host maps that onto a native redraw. Native windows and their `View` handles live in a host-side registry — `View` is type-erased, the `Window` is not.
  Reason: platform — winit redraw is per window and the window must stay on the event-loop thread, while the scheduler requests one frame.
  Affect: closing the last window ends `run`. A frame runs the scheduler; `View::present` draws the valo picture.
