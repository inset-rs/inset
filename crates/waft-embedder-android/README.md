# waft-embedder-android

Waft's Android host: `NativeActivity` with Waft's own glue, the app's loop on a thread of its own, and each activity's window as a view of the app. It takes an app's entry, `android_main`, which `#[waft::main]` writes, and Android 12 (API 31) or later. It provides `AndroidPlatform`, Waft's `Platform` on Android; `src/lib.rs` says how the host is built. `waft-embedder-default` uses it, and `cargo waft` packages it.

## Image codecs

`Platform::image_codecs` is Android's own decoder and encoder, the NDK's `AImageDecoder` and `AndroidBitmap_compress`, and Android's own EXIF reader, the framework's `android.media.ExifInterface`, called over JNI from the codecs' threads (`src/image_codecs/`).

What is there:

- Decoding, from bytes or from a file, of JPEG, PNG, GIF, WebP, BMP, ICO, WBMP and DNG on every device, and of HEIF and AVIF where the device's codecs decode them, which the host finds by decoding a small picture of each as it starts.
- An image's MIME type, its size upright, and an animation's frames with how long each shows (GIF and WebP).
- Its orientation as the file says it, and what EXIF says of how it was taken: the date with its offset from UTC, the camera's make and model, exposure time, f-number, ISO, focal length and its 35 mm equivalent, exposure bias, whether the flash fired, and the GPS position with its altitude, read from the same tags Apple's host reads, for every format `ExifInterface` reads: JPEG, PNG, WebP, HEIF, AVIF, DNG and other raw formats.
- A frame in the app's memory, RGBA, premultiplied, in sRGB, upright, scaled down to fit the size asked. The decoder turns a picture upright itself for the formats it does so for; for the others, a PNG on the Android versions tried, the host turns the decoded pixels, found at start by decoding a small mirrored picture of each format. An animation's frames decode fastest asked in order.
- Encoding the app's pixels as JPEG, PNG or lossy WebP, with no metadata. Android takes a quality for every encoding and has none of its own: where the app gives none, the host asks for 90 of 100.

What is not, and how an app sees it:

- No hardware codecs: `hardware_image_codecs` is `None`, since `AImageDecoder` decodes only into memory its caller holds.
- No lens: the framework's `ExifInterface` knows no `LensModel` tag (AndroidX's does), so `ImageMetadata::lens_model` is `None` even where the file names one.
- A format `ExifInterface` does not read, or a file it cannot parse, answers as a file with no EXIF: upright, every field `None`.
- No lossless WebP, no HEIF or AVIF encoding.
