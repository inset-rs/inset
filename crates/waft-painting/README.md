# waft-painting

Flutter's `painting` library in Rust: colours and geometry, text styles and layout, decorations, shapes, image providers, the image stream and the image cache.

## What it takes

`waft-embedder`'s values and host traits — `Color`, `Offset`, `Rect`, `Canvas`, the image codecs, and the platform's HTTP client and file system for `NetworkImage` and `FileImage`; `waft-foundation`'s `App`, handles and futures; and `waft-scheduler`'s frame callbacks. A test asks `waft-test` for a platform whose HTTP client and file system are in memory.

## What it provides

The values a widget draws with (`Alignment`, `EdgeInsets`, `Border`, `BoxDecoration`, `TextStyle`), `TextPainter` and its layout, `PaintImage` and the decoration image, and the image machinery: `ImageProvider` (`MemoryImage`, `AssetImage`, `NetworkImage`, `FileImage`), `ImageStream` with its `ImageChunkEvent` progress, `ImageStreamCompleter`, and the memory `ImageCache`. A network or file image is fetched or read through the app's platform, decoded by its codecs, and shared by every widget showing the same key; a host with no HTTP client or no file system fails such an image with a message saying so.

## Not built yet

`ResizeImage`, image providers naming a resolution variant, and an image cache on disk.
