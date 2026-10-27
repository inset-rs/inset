# inset-winui-test-support

Unpublished GPU test support for this workspace. Add it only under a consuming package's `[dev-dependencies]`:

```toml
[dev-dependencies]
inset-winui-test-support.workspace = true
```

`Fixture::with_root(size, run)` takes a physical pixel size `[u32; 2]` and a setup callback `FnOnce(&mut App)` that calls `inset_widgets::run_app` to mount the test widget. The `mount` helper below creates that callback. The fixture can pump frames, send input, find text labels, and capture rendered output. Tests using it require an available GPU adapter. `capture` writes PNG files when `WINUI_CAPTURE_DIR` is set.

```rust,no_run
use inset_widgets::{IntoWidget, SizedBox};
use inset_winui_test_support::{Fixture, mount};

let fixture = Fixture::with_root(
    [400, 300],
    mount(|_| SizedBox::new().width(40.0).height(20.0).into_widget()),
);
fixture.capture("empty-box");
```

`CaptureView` and `gpu::headless_device` can also be used independently by tests that need their own frame driver. `Fixture::with_root` installs system fonts; the independent GPU and view helpers do not. Tests supply any additional fonts their controls require.

This crate does not depend on `inset-winui` or `winui_gallery`. Gallery startup, feature selection and page navigation remain in `examples/winui_gallery/tests/common` as `GalleryFixtureExt`. Control tests can use `Fixture::with_root` directly without loading the gallery.

The GPU and PNG helpers originated in Valo's test harness; its MIT notice is retained in `VALO_LICENSE`.
