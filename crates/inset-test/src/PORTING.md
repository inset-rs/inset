# inset-test/src
No Flutter crate in this shape. Closest analogue: `flutter_test`'s `TestPlatformDispatcher` and `TestFlutterView`.

## platform.rs → flutter_test `TestPlatformDispatcher`

- Change: `TestPlatform` is built by a test — which host it claims to be, its views, and each capability as the test's own implementation or absent — and counts frame requests and wake deadlines; `TestPlatformDispatcher` wraps the real dispatcher and overrides fields one at a time.
  Reason: platform — there is no real dispatcher under a test to wrap; the `Platform` the test hands to `AppCell` is the whole host.
  Affect: a test asserts on `frames_requested` and `wakes` instead of pumping a real engine, background work runs at once so a checkpoint sees its result, and a capability the test did not plug in answers `None`.

## Deferred

- `TestFlutterView`'s per-view overrides beyond size and pixel ratio (padding, insets, gesture settings).
