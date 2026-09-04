# reveal-embedder-winit/src

No Flutter counterpart. Flutter's engine is C++ and is not in this checkout.

## lib.rs / window.rs

- Change: this crate is a host: it implements `Platform` and `View`, creates the implicit window if configured, then hands `Platform` to a start closure that returns the client. It never names `App`.
  Reason: platform — Flutter's native embedding does not name the framework; winit cannot create a window until `resumed`.
  Affect: `WinitEmbedder::run(|platform| Shell::new(platform, setup))`.

- Change: the scheduler asks for one frame for the whole app; the host maps that onto a native redraw. Native windows and their `View` handles live in a host-side registry — `View` is type-erased, the `Window` is not.
  Reason: platform — winit redraw is per window and the window must stay on the event-loop thread, while the scheduler requests one frame.
  Affect: closing the last window ends `run`. A frame runs the scheduler; `View::present` draws the valo picture.

- Change: `Platform::activate_system_cursor` records the requested kind and pokes the loop; the loop sets the winit cursor icon on the window the pointer was last seen in. `None` hides the cursor; kinds winit lacks show the arrow.
  Reason: platform — Flutter's engine maps cursor kinds per host; here winit is the host and it has one cursor per window.
  Affect: a `MouseRegion` cursor shows up on hover. The device id is ignored: one mouse.

- Change: `Platform::font_source` is `valo-system-fonts`' OS scanner.
  Reason: platform — Flutter's engine finds platform fonts itself; valo needs a `FontSource`.
  Affect: text uses installed fonts; a family that is not installed falls back to the nearest face.

## Deferred

- `ThemeChanged` → `onPlatformBrightnessChanged`. Trigger: `MediaQuery` / `CupertinoTheme`.
