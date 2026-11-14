# waft-embedder-android

Waft's Android host: `NativeActivity` with Waft's own glue, the app's loop on a thread of its own, and each activity's window as a view of the app. It takes an app's entry, `android_main`, which `#[waft::main]` writes, and Android 12 (API 31) or later. It provides `AndroidPlatform`, Waft's `Platform` on Android; `src/lib.rs` says how the host is built. `waft-embedder-default` uses it, and `cargo waft` packages it.

## Cameras

`Platform::cameras` is the NDK's own camera API, `ACameraManager` and `ACameraDevice`, with no Java but the permission's question (`src/media/`); `Platform::video_frame_readers` the readers a camera's frames go to.

What is there:

- The cameras that take pictures as a camera app does (Android's `BACKWARD_COMPATIBLE`), listed without asking, with which way each looks, and the list told changed when Android says a camera came, went or was let go of and the list differs.
- Opening a camera asks for `android.permission.CAMERA` with the system's question over the activity in front where the app does not hold it, `NotAllowed` where the user refuses; a second opening while the question shows waits on the same answer. The manifest asks for the permission where the app declares the camera (`waft-cli`'s `usage_descriptions`).
- Formats read from the camera's characteristics without opening it: each size it streams for the GPU at (`PRIVATE`), the frame rates its auto-exposure ranges keep there, and every JPEG size as a photo size. A session asks for the fastest, widest range its size allows.
- The camera itself opened as a session starts, on a thread of its own, once for the process: a later session on the camera joins its format, as on Apple's systems. Android gives a camera one JPEG stream, made at the first session's photo size, so a session that joins takes its photos at that size, whatever it asked, and takes none where the first asked none.
- The preview on a composition layer: the camera's pictures, in its own format, shown by the compositor with no copy, turned upright for the screen and mirrored for a camera that looks at the user, whole within the layer's box. What the layer shows reads back as a frame.
- Frames for a reader: each picture drawn into a new RGBA8 `AHardwareBuffer` with OpenGL ES (`samplerExternalOES`), in the camera's own pixels, unturned, as the format's size says, reporting the turn that shows it upright on the screen as its `rotation`, and a `flip` for a camera that looks at the user; the frame is its own buffer, freed once nothing holds it, and `AndroidVideoFrame` for platform code. On the emulator a frame drawn so reads black where another device's Vulkan imports it, while its pixels read back right; on a phone it shows.
- Rotation angles: both of a session's, the preview's and the capture's, are one number, the sensor's orientation against the screen's turn, since Android tells no turn of the device apart from the screen's; told changed as the screen turns.
- Photos: JPEG from the camera's own stream, with the orientation Android writes into its EXIF for the screen's turn now.
- States: running; interrupted, with `InUseByAnotherApp`, where Android disconnects the camera or says another client has it, and with no reason where it disables it; opened again when Android says a camera became available, or the app comes to the front; ended where the camera or the service fails, the camera is unplugged, or the app may not use it.

What is not, and how an app sees it:

- No upright frames: Android's camera delivers its frames as its sensor takes them, so a start that asks for them fails with `NotSupported`; each frame reports how it shows instead.
- No recordings: the NDK has no recorder, so a session asked for movies starts without them, says so in the log once, and `start_recording` answers why.
- No interruption for the background or for the system's heat: Android tells neither; a camera the app loses in the background is `InUseByAnotherApp` or interrupted with no reason.
- Names are the host's, by which way a camera looks ("Back camera", "Front camera 2"): Android gives a camera none.

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
