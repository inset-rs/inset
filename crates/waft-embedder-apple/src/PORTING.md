# waft-embedder-apple/src

Embedder implementation references: Flutter's C++ engine under `flutter/engine/src/flutter`, and the Flutter plugins named below.

## Identical

- local_time.rs → the Dart VM's `os_macos.cc` (`OS::GetTimeZoneOffsetInSeconds`, `OS::GetTimeZoneName`), which `DateTime`'s natives call

## composition/ → the macOS and iOS embedders' platform views (`FlutterSurfaceManager`, `FlutterMutatorView`, `FlutterPlatformViewsController`)

- Change: each composition layer is a Core Animation layer beneath the framework's layer, showing its producer's buffer directly, on macOS and iOS alike.
  Reason: platform — Core Animation composites the buffers it's given, so a producer's frame needs no copy and no framework frame.
  Affect: framework content can sit above a composition layer but not beneath it.

- Change: on macOS each view draws into a `CAMetalLayer` Waft makes as the content of a composition layer of the view's own, which its window places to fill the window's view as that view's bounds and scale change, with the composition layers the view's scenes place beneath it, where Flutter's macOS embedder shows buffers it draws itself in layers it adds to its view's layer.
  Reason: platform — wgpu presents a frame through a `CAMetalLayer`'s drawables, not as a buffer the host hands Core Animation, and a view on a layer of its own draws alike wherever that layer is placed.
  Affect: a picture that has not caught up with its window's new size shows stretched over the window, where Flutter's shows at its own size until the next frame.

- Change: a composition layer's Core Animation layers take every transform and clip above it, a path's included.
  Reason: platform — Core Animation turns a layer by any transform and masks it by any path.
  Affect: a turned, rounded or path-clipped composition layer shows exactly where the framework places it.

- Change: a composition layer's buffer and its children sit in one Core Animation layer placed by a transform, the children beneath the buffer, and a present shows its buffer and places its children in one transaction; on macOS the app may place a composition layer in a `CALayer` of its own (`MacOs::place_layer_in`), such as the root of a `CAContext` another process shows.
  Reason: platform — a program whose picture is itself a composition layer in another app's window shows its own layers as that picture's children, and Core Animation moves, clips and scales a layer's sublayers with it.
  Affect: a child moves, is cut and shrinks with its parent, and shows through its parent's clear pixels.

- Change: a buffer is released once the system says it's no longer in use, checked at the layer's next present and when the layer goes.
  Reason: platform — Core Animation gives no signal when it lets go of a buffer.
  Affect: a producer never waits for a buffer to come back, but draws into another when none is known free, as Flutter's `FlutterSurfaceManager` does.

- Change: a composition layer answers what it shows now (`CompositionLayer::current_picture`) with the buffer presented to it last, or, while a player's or a camera's layer is its content, with what that source's own output beside the layer has for now, the frame read last staying the answer until a newer comes; a view's `CAMetalLayer` and an app's own system layer answer nothing.
  Reason: platform — Core Animation hands nothing back of what an `AVPlayerLayer`, an `AVCaptureVideoPreviewLayer` or a `CAMetalLayer` shows, so only an output the source feeds beside it can.
  Affect: a picture of a window shows its videos and camera previews, a paused video's frame among them, each a `CoreVideoFrame` lent as a reader's frames are.

## media_futures.rs, media/ → `video_player_avfoundation` and `camera_avfoundation`

- Change: a player is an `AVPlayer` whose state and position are read from it on the main thread when the app asks, its changes told by key-value observing, its end-of-item, time-jumped and rate notifications and each seek's completion, where Flutter's plugin copies events over an event channel and polls the position on a timer.
  Reason: platform — AVFoundation changes a player's state on the main queue, where the app reads it, keeps its current time for the app to read, tells when that time jumps and why its rate fell to zero, and completes each seek.
  Affect: the app's player hears its state change at once on load, play, pause, a wait for data and end, a seek's landing and a loop, and nothing as it plays on; before macOS 12 and iOS 15, which tell no reason for a fall of the rate, a pause the app did not ask for has none.

- Change: a player shown on a composition layer makes an `AVPlayerLayer` its content (`MacOs::show_system_layer` for an app's own system layer), with an `AVPlayerItemVideoOutput` added to its item beside it, which the layer reads its current picture from, and one given a frame reader adds an `AVPlayerItemVideoOutput` of BGRA `IOSurface`-backed pixel buffers to its item, which the reader reads, where Flutter's plugin copies every frame into a texture.
  Reason: platform — Core Animation clips and scales a layer with the one it is the content of, and a pixel buffer backed by an `IOSurface` reaches the GPU with no copy.
  Affect: a video moves, clips and shrinks with the framework's frame, and a frame reader hands out the frame for the moment the picture reaches the screen, keeping none for later.

- Change: a player's audio and text tracks are the options of its asset's audible and legible media selection groups, selected with the item's `selectMediaOption:inMediaSelectionGroup:`, and media without such a group, such as an MP3, lists that kind from its item's tracks, enabling the one selected, beside the item's video tracks, where Flutter's plugin lists no tracks.
  Reason: platform — AVFoundation offers a choice of audio and subtitles through selection groups, and plays the tracks outside any group while they are enabled.
  Affect: a stream lists its renditions under ids that stay for the player's run, a file lists its own tracks, and a stream's video track is listed again under a new id each time it switches variant.

- Change: a player's tags are its asset's common metadata, read once AVFoundation has loaded them, its artwork named by the image format its bytes start with, and its subtitles come from an `AVPlayerItemLegibleOutput` at its `SourceAndRulesOnly` styling that keeps AVPlayer from drawing them, its attributed strings and their Core Media markup made into cues, a field the markup gives no value for left unsaid, where Flutter's plugin reads neither.
  Reason: platform — Apple's legible output gives every subtitle AVPlayer would draw as text with markup, with the media's own styling alone at that resolution, as WebKit asks for it.
  Affect: the app gets subtitles placed where Core Media places them in the media's own style, to draw in the user's caption style, and the cues go at a seek or a change of text track until the next arrive.

- Change: on macOS and iOS the user's caption settings are Media Accessibility's, the style read from the user's domain each time the app asks and the change future resolved at its settings-changed notice, a setting applying always where Apple's behaviour is `UseValue` in a style the user customized, as WebKit has it, its colours matched to sRGB with their opacities apart, and its font the family of the one it gives for the default style; Flutter reads none.
  Reason: platform — Media Accessibility gives each caption setting with whether content may override it, and tells when the user changes one.
  Affect: the app draws subtitles in the user's caption style and follows its changes, a style the user never customized giving way to the media's own, and a style whose font is the system's hidden one leaves the family to the framework.

- Change: a frame reader is made empty and reads the output of the source it was given to last, a player item's video output or a capture session's video data output, which that source makes and adds when it is given the reader, a camera as it starts.
  Reason: platform — AVFoundation has no one output that a player and a camera both feed, as Android's `ImageReader` is fed by a decoder and a camera alike.
  Affect: one reader serves a player or a camera, it reads nothing until it is given to one, and a reader the host did not make reads nothing, which the host says once.

- Change: the cameras are the devices of one discovery session the host keeps, whose `devices` it key-value observes to tell the app the list changed, where Flutter's plugin makes a discovery session for each `availableCameras` and observes none.
  Reason: platform — Apple's header gives key-value observing a discovery session's `devices` as the way to hear its list change, and gives `AVCaptureDeviceWasConnectedNotification` and `AVCaptureDeviceWasDisconnectedNotification` for `AVCaptureDevice.devices`, which it deprecated.
  Affect: on a Mac, an app hears the cameras change as one is plugged in or out.

- Change: a camera opens once the user lets the app use the cameras, lists its device's own formats, and runs as an `AVCaptureSession` given its camera, format and outputs at once and started on a serial queue of its own, its start answering once it runs, is interrupted or fails, asking for the microphone first where a recording wants sound, with its preview a layer that is a composition layer's content and one video data output, added for a preview or a reader, that both read.
  Reason: platform — AVFoundation configures a session's inputs and outputs together, each output is AVFoundation's own, and Apple's documentation of `AVCaptureSession` starts a session on a serial queue, since `startRunning` blocks until it runs or fails.
  Affect: the app's thread goes on while a camera starts, a session reports running, interrupted or ended as AVFoundation tells it, photos arrive as JPEG at the size asked for, and recordings as QuickTime movies.

- Change: the first session in the process on a camera sets the format it picked and keeps the camera locked while any session in the process runs it, and a later session runs in that format; where another process holds the lock, a session runs in the format the camera has.
  Reason: platform — a camera has one active format for every session using it, two sessions in one process share one camera, and Apple gives locking a camera as the way to keep its settings unchanged.
  Affect: a second session on a camera reports the first one's format, and other apps cannot change the camera's settings while the app runs it.

- Change: an interrupted session takes the reason iOS gives, any but the background and system pressure read as another app having the camera, and on macOS none.
  Reason: platform — Apple marks the interruption notice's reason key unavailable on macOS, and documents the notice for another application taking the hardware a session needs.
  Affect: on a Mac an interruption has no reason.
