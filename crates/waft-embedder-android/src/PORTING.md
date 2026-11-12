# waft-embedder-android/src

Host implementation. Flutter analogue: the engine's Android embedder under
`shell/platform/android`, whose activity and view are written in Java.

## native_activity.rs / app_thread.rs / host.rs → the Android embedder's `FlutterActivity` and `FlutterActivityAndFragmentDelegate`

- Change: the app runs on a thread of its own, started with the process's first activity and kept for the process, which every activity's callbacks hand their events to from Android's main thread, each activity a view of the one app; where Flutter's activity owns an engine of its own, destroyed with it unless the engine is cached.
  Reason: design — one app per process, whose views are its activities' windows, as a desktop app's are its windows.
  Affect: an activity finished by Back or by the system takes its view with it while the app runs on, and the activity Android makes next, in the same process, is a new view of the running app rather than a new start of it.

- Change: an activity Android destroys without finishing it, for a configuration change the manifest does not absorb or under "Don't keep activities", saves its view's id with a value made fresh in each process, and the activity made in its place attaches to that view, which was kept without a window; Flutter keeps its plugins attached across a configuration change but destroys an engine that is not cached with its activity.
  Reason: design — the view is the app's, not the activity's, and only the activity's window goes.
  Affect: the app's state, a scrolled list or a typed text, stays through "Don't keep activities"; an activity brought back after its process died starts a new view, as the saved value is another process's.

- Change: at each refresh of the display the input waiting in the views' queues is read before the frame is decided and drawn.
  Reason: platform — the input queues and the choreographer are both on the app's looper, which fixes no order between them within a poll, where Android's `Choreographer.doFrame` runs its input callbacks before its frame.
  Affect: a touch that arrived before a refresh moves that refresh's frame, not the next one.

## window_insets.rs / view.rs → the Android embedder's `FlutterView.onApplyWindowInsets`

- Change: the insets are heard by a listener on the window's decor view, `rs.waft.WaftWindowInsets`, which `WaftActivity` adds, and measured from the window's edges, where Flutter's view overrides `onApplyWindowInsets` itself and is told what is left of them where it was laid out.
  Reason: platform — `NativeActivity` hands its native code the window's whole surface and no Java view of it, and tells it only the rectangle its content was laid out in, which does not say the bars from the keyboard.
  Affect: an app draws behind the status bar and the navigation bar on every release, each reported as view padding, as a Flutter app does once its window is edge to edge; under the bare `NativeActivity` the bars are what the content rect leaves of the window, and no keyboard is known.

- Change: the cutout's bounds, the display's corner radii and the caption bar, which `onApplyWindowInsets` also reads, are not read.
  Reason: design — `ViewMetrics` carries the three paddings and the system gesture insets alone.
  Affect: `MediaQuery.displayFeatures` is empty, so an app cannot tell where the cutout is.

- Change: the keyboard's inset arrives once, at its full height, as the keyboard starts to show, and goes as it starts to hide; Flutter's `ImeSyncDeferringInsetsCallback` reports it frame by frame with the keyboard's animation.
  Reason: design — not ported yet.
  Affect: what sits above the keyboard jumps to its place while the keyboard slides.

## text_input.rs / editor_info.rs → the Android embedder's `TextInputPlugin`, `InputConnectionAdaptor` and `ListenableEditingState`

- Change: the keyboard edits text through Waft's own view, `rs.waft.WaftTextInput`, which `WaftActivity` adds beside `NativeActivity`'s content view and which holds the field's value on the main thread, while the app's thread hands it the framework's calls and hears the keyboard's changes once a batch of edits ends; Flutter's keyboard talks to the `FlutterView` that also draws and takes touches.
  Reason: platform — a keyboard edits only through the `InputConnection` of a focused Java view, which `NativeActivity`'s content view does not give, and it must be answered on the main thread at once, where the app's thread runs the framework.
  Affect: an app built by waft-cli opens the keyboard for a focused field and is typed into; under the bare `NativeActivity` no keyboard opens and a field stays empty.

- Change: the keyboard connects again where the field's configuration changed or the framework replaced its text or composing region, as for an `EditText` whose text is set; Flutter's plugin connects it again at each new client and where the framework changed the composing text.
  Reason: platform — `start` is the attach, the show and the configuration update at once, so a new client is told by what changed.
  Affect: moving from one field to another with the same configuration and the same text keeps the keyboard's state, such as its shift.

## platform.rs

- Change: haptics are absent, and the platform answers that it has none.
  Reason: platform — haptics are a Java API that needs an activity of the app's own.
  Affect: a widget that would buzz stays silent. The framework's own fallbacks run, rather than a capability that silently does nothing.

## clipboard.rs

- Change: the clipboard is `ClipboardManager` through JNI: its clip listed from its description as one item, read as text, HTML, a URI and the URI's content, and written as text with HTML beside it only, a sensitive copy marked so and one that expires cleared by the host.
  Reason: platform — every other form, and a form produced when pasted, is a content provider's, which is Java code of the app's own.
  Affect: text copies and pastes with other apps, and an image another app copied can be read, but the app cannot copy an image or a file.

- Change: the app hears the clipboard changed as the activity takes the focus, by the clip's time, its own writes counted as seen, and when a copy of its own expired and was cleared.
  Reason: platform — Android tells changes only to a listener Java code provides.
  Affect: a copy another app makes while this one is in front is heard at the next focus.

- Change: a copy that expires is cleared by the host's loop, which wakes for it as it wakes for the app's timers, if the clip is still the app's.
  Reason: platform — Android has no expiry an app can set.
  Affect: a copy whose app quits before its time stays on the clipboard.

## file_dialogs.rs → `file_selector_android`

- Change: the file dialogs are the Storage Access Framework's pickers (`ACTION_OPEN_DOCUMENT`, `ACTION_CREATE_DOCUMENT`), started by the activity for a result, which Waft's one Java class, `rs.waft.WaftActivity`, hands to native code; under another activity the dialogs are `None`.
  Reason: platform — `NativeActivity` forwards no activity result to native code, which `file_selector_android` gets from its plugin's Java.
  Affect: an app built by waft-cli has file dialogs; one run under the bare `NativeActivity` has none.

- Change: each pick is opened as it is answered, `"rw"` when the app asked to write and `"r"` otherwise, its permission taken as persistable: a handle where the document seeks, a stream where the provider answers a pipe, where `file_selector_android` copies the document into the app's cache and answers the copy's path.
  Reason: platform — a document has no path, and a copy could not be written back.
  Affect: an app writes a picked document in place, and reads a pipe through once.

- Change: the pickers filter by MIME type alone, a type's extensions mapped by `MimeTypeMap`; an initial folder and a confirming label are not taken.
  Reason: platform — the pickers take MIME types and a content URI, not a path.
  Affect: a type with extensions Android does not know filters nothing.

- Change: a folder pick is the document tree `ACTION_OPEN_DOCUMENT_TREE` answers, its permission taken as persistable, as a `FolderHandle` over `DocumentsContract`: listed by its children, a child file opened as a picked document is, a child folder as another handle, and files and folders made by `createDocument` when writable; `file_selector_android` answers no folder there.
  Reason: platform — a document tree has no path, and its documents are reached through the provider alone.
  Affect: an app lists, reads and, when it asked to write, makes files in a folder the user picked.

## opener.rs → `url_launcher_android`

- Change: a link opens through an `ACTION_VIEW` intent on its URI, `ActivityNotFoundException` being no app; a file through an `ACTION_VIEW` intent on the URI Waft's file provider, `rs.waft.WaftFileProvider`, gives it at `<package>.waft.files`, granting the app that opens it reading that URI; revealing a file fails, where `url_launcher_android` opens only URIs.
  Reason: platform — no other app reads the app's storage by path, and Android has no file manager that shows a file in its folder; androidx's `FileProvider` comes with a Gradle dependency the package does not have, so the provider is Waft's own, which waft-cli compiles and declares, not exported.
  Affect: an app built by waft-cli opens its own files in another app; without the provider opening a file fails.

- Change: `can_open_link` is `resolveActivity`'s answer, `None` from Android 11 for a scheme the manifest's `<queries>` does not name, which the application's meta-data `rs.waft.queried_schemes` lists, where `canLaunchUrl` answers false.
  Reason: platform — from Android 11 an app sees only the apps its `<queries>` names.
  Affect: an app built by waft-cli is answered for `http`, `https` and `mailto`.

## intents.rs / open_requests.rs → the Android embedder's `FlutterActivity` intent handling

- Change: the intent the activity started with and each one `onNewIntent` brings, which `WaftActivity` hands on, reach the client as `open_requested` when they are `ACTION_VIEW`: a `content://` document opened as a picked one is, to write where the intent grants writing, a `file://` URI as a path, and any other URI as a link; Flutter's activity hands them to its route and its plugins.
  Reason: platform — `NativeActivity` keeps `onNewIntent` to itself, and a document has no path.
  Affect: an app opened with a document reads it as it reads a pick; under the bare `NativeActivity` only the intent it started with comes.

## notifications/ → `flutter_local_notifications`' Android implementation

- Change: a notification is built by Waft's notification receiver, `rs.waft.WaftNotificationReceiver`, from a `Bundle` the host fills, and shown with `NotificationManager.notify` under its string id as the tag, so posting the id again replaces it, on one of two channels the receiver makes, "Notifications" and "Silent notifications", both `IMPORTANCE_HIGH` and the silent one without sound or vibration; `flutter_local_notifications` takes an int id and the channel the app describes.
  Reason: platform — a notification for later is built when its alarm goes off, in a process where the app's native code may not run, so the one builder is Java's, which waft-cli compiles and declares, not exported; Android 8 sets sound by channel, and `setSilent` came only with Android 12.
  Affect: a notification pops up, as the other systems' do, with the app's launcher icon as its small icon and the image as its large icon, fitted to the system's size; the user sees the two channels in the app's settings and may change either.

- Change: a tap on a notification, on one of its buttons, or a reply typed into its `RemoteInput` field starts `WaftActivity` with an intent of the receiver's own action, which the host reads at launch and from `onNewIntent` (`intents.rs`) and hands on as `notification_activated`; a button or a reply also takes the notification away.
  Reason: platform — `NativeActivity` keeps `onNewIntent` to itself, Android before 15 shows a reply as sending until its notification changes, and from Android 12 a receiver a tap reaches may not start the activity.
  Affect: the app hears a tap whether it was running or started by it, and every action, a reply included, brings it to the front; from Android 15 a replied notification stays, showing the reply, until the user dismisses it, as the system holds it past the host's cancel.

- Change: the permission is Android 13's `POST_NOTIFICATIONS`, which waft-cli declares, asked for with `requestPermissions` and answered through `WaftActivity.onRequestPermissionsResult`, with `shouldShowRequestPermissionRationale` telling a refusal the user may still be asked about; before Android 13, or for a package that targets an earlier release, it is `areNotificationsEnabled` alone, with no question.
  Reason: platform — Android tells no app a permission never asked from one refused for good.
  Affect: `state` answers `NotAsked` for a permission refused for good, and asking then answers `Denied` at once, without a question; a post never asks, as `notify` does not, and answers `Denied` until the permission is granted.

- Change: a schedule is an `AlarmManager` alarm whose intent carries the notification to the receiver, exact (`setExactAndAllowWhileIdle`) where the app may set exact alarms, and inexact (`setAndAllowWhileIdle`) where not; waft-cli declares `SCHEDULE_EXACT_ALARM`.
  Reason: platform — from Android 14 an app sets exact alarms only once the user grants it in the app's settings, and the system forgets alarms when the device restarts.
  Affect: a scheduled notification shows whether or not the app runs, late by as much as the system defers it where exact alarms are not granted, and not at all after a restart.

## display.rs / host.rs

- Change: the one screen is the activity's display, numbered by its own id, read when a window arrives and at each configuration change, and the app hears it changed only then; Flutter's embedder numbers it 0 and hears its refresh rate through a display listener.
  Reason: platform — a display listener is a Java interface, which `NativeActivity` gives the app no class to implement.
  Affect: the screen reads 0 on a phone as Flutter's does; a rate the panel changes on its own, with nothing else, is not heard until the next configuration change, and before API 30 the visible frame is the whole screen.

## picture.rs / composition/ → the Android embedder's `FlutterSurfaceView`, `PlatformViewsController` and its hybrid composition

- Change: where Waft's device is Vulkan with `VK_ANDROID_external_memory_android_hardware_buffer`, the view's picture is drawn into `AHardwareBuffer`s of its own and shown as the content of a surface control made from the window, with the scene's composition layers beneath it as child surface controls, all in one `ASurfaceTransaction` a frame; on OpenGL ES the view draws through a swapchain on the window and the platform has no compositor, where Flutter's engine draws into the window's surface and shows platform views as Android views or overlay surfaces beside it.
  Reason: platform — a swapchain cannot join the transaction its layers are placed in, so a moved layer would land a frame off its hole; only a device that imports hardware buffers can draw buffers the compositor shows.
  Affect: a composition layer moves with the framework's picture in the same frame on Vulkan; on an emulator or a phone with no Vulkan the app shows no composition layers.

- Change: a composition layer is placed by its box, its scale and the bounds of the clips above it (`setPosition`, `setScale`, `setCrop`), with no rotation, shear, rounded or path clip.
  Reason: platform — the NDK places a surface control by an integer position, one scale and an axis-aligned crop alone.
  Affect: a turned layer shows upright over the box around it, and a rounded clip's corners show only as far as the picture above them covers.

- Change: a replaced buffer goes back to its producer once the release fence the system hands over for it has signalled, waited on by a thread of the host's own; Impeller's swapchain takes its buffer back at `OnComplete` and imports the release fence as a semaphore its next draw into the buffer waits on.
  Reason: platform — `OnComplete` arrives before the buffer is free, and a producer's buffer goes back to it through `CompositionLayer::present`'s `released`, which promises it free.
  Affect: a producer gets its buffer back about a refresh after the present that replaced it, and the view draws into a buffer only once the compositor is done with it, so its pool holds a buffer or so more than Impeller's would.

## hardware_buffers.rs / gpu_interop.rs

- Change: buffers are `AHardwareBuffer`s in RGBA8, imported as textures only on a device opened with the import's extensions, which Waft's own is and a device an app opens itself is not; `read_pixels` answers `Unsupported` for a buffer allocated without `cpu_read`, whatever the device's allocator allows.
  Reason: platform — wgpu enables no import of Android's buffers itself, and some allocators refuse a CPU lock of a buffer allocated without a CPU usage.
  Affect: an app draws into a buffer on the renderer's device, and reads back only one it asked to read.

## input.rs

- Change: the pressure range reported with every touch is zero to one, rather than the range the device declares.
  Reason: platform — the engine reads it from the `InputDevice`, which the native glue does not hand out; reaching it needs a JNI lookup per device.
  Affect: a force press on a digitizer whose pressure is not scaled zero to one crosses its thresholds at the wrong pressure. Ordinary taps, drags and flings are unaffected.

## Deferred

- Haptics. Trigger: an app that needs them, which is Java code of the activity's own, as Flutter's is.
- The framework's key events, Android's keys mapped to Flutter's: until then every key but back is left to the text input view, which types it into the field that takes text, deleting, moving the caret and acting on enter as Flutter's framework would, and to the system, which sets the volume. Trigger: a shortcut, such as select all, or an app that reads keys on Android.
- The keyboard's own cut, copy, paste and select all, the images it commits, autofill, hint locales and the delta model, which Flutter's text input plugin takes. Trigger: an app that needs one.
- The predictive back gesture, with its peek animation. Trigger: a Java activity that registers with the system's back callback; the plain back button already pops.
- Deep links, which Flutter carries beside the back button. Trigger: an app that opens on a route the system names.
- A window the app manages around the activity's view, with whether it is minimized and whether it is occluded, which the activity's stop and start would tell. Trigger: an app that manages its window on Android, as in desktop windowing.
