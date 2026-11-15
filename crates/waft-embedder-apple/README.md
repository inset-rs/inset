# waft-embedder-apple

What Waft's hosts on macOS and iOS share, with no windowing of its own: Core Animation's compositor of composition layers and the layer a view draws into, `IOSurface` hardware buffers and Metal textures over them, AVFoundation's players, media assets and cameras, Image I/O's codecs, Media Accessibility's caption settings, the notification center, and the pasteboards' type names, promised files and main-thread futures.

It takes `waft-embedder`'s traits and valo's and wgpu's types, and provides each of them as the system has it, for a host to hand the app through its `Platform`. A host places what it gets here in its own views and windows: nothing here names a window, so a layer, a buffer or a notification behaves the same under either host. `waft-embedder-winit` uses it for macOS. Elsewhere than macOS and iOS the crate is empty.
