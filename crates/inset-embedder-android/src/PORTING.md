# inset-embedder-android/src

Host implementation. Flutter analogue: the engine's Android embedder under
`shell/platform/android`, whose activity and view are written in Java.

## host.rs / app_thread.rs → the Android embedder's `FlutterActivity` and `FlutterEngine`

- Change: every activity the system creates is a view of one running app, from its creation to its destruction, and the app outlives them all; Flutter's `FlutterActivity` starts an engine of its own unless it is handed a cached one.
  Reason: platform — Inset's platform gives one app all of its views, as a desktop gives it its windows.
  Affect: a second activity, or one created after the last was destroyed, opens a new view of the app already running, which keeps its state.

## host_window.rs / task.rs → Flutter's windowing API (`widgets/_window.dart`) and `PlatformPlugin.setTaskDescription`

- Change: a view's window is its activity's task: asking for a view starts a task of its own, with its own card in Recents, the window's title and icon are the card's, and closing the window ends the task; Flutter's windowing API makes windows on desktops only.
  Reason: platform — an Android app's windows are its tasks, each with its card in Recents.
  Affect: a view the app asks for on Android opens as another card in Recents, which the user switches to as to another app, and a task Android brings back after the app's process was gone comes back as a view carrying the same initial data.

- Change: `SystemChrome.setApplicationSwitcherDescription` describes every task of the app, under the title and icon each view's window was given of its own; Flutter's describes its one activity's task.
  Reason: platform — each view is a task of its own, whose card the app's description and the window's own share.
  Affect: a window's own title and icon stay on its card when a `Title` widget is built again.

## view.rs

- Change: the keyboard's height is taken as whatever the content rect loses at the bottom beyond the widest it has been at this window size.
  Reason: platform — `NativeActivity` reports one content rect and never `onApplyWindowInsets`, which is where the engine's Java view reads the bars and the keyboard apart.
  Affect: the keyboard lifts the layout as it does on every other host, but a bar that grows while the window keeps its size, a navigation bar appearing, is taken for a keyboard until the window is resized.

- Change: there is no text input, so no soft keyboard opens and nothing can be typed.
  Reason: platform — typing needs an `InputConnection`, which belongs to a Java view; `NativeActivity` has none.
  Affect: a text field draws, scrolls and can be selected in, and stays empty.

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

- Change: the file dialogs are the Storage Access Framework's pickers (`ACTION_OPEN_DOCUMENT`, `ACTION_CREATE_DOCUMENT`), started by the activity for a result, which Inset's one Java class, `rs.inset.InsetActivity`, hands to native code; under another activity the dialogs are `None`.
  Reason: platform — `NativeActivity` forwards no activity result to native code, which `file_selector_android` gets from its plugin's Java.
  Affect: an app built by inset-cli has file dialogs; one run under the bare `NativeActivity` has none.

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

- Change: a link opens through an `ACTION_VIEW` intent on its URI, `ActivityNotFoundException` being no app; a file through an `ACTION_VIEW` intent on the URI Inset's file provider, `rs.inset.InsetFileProvider`, gives it at `<package>.inset.files`, granting the app that opens it reading that URI; revealing a file fails, where `url_launcher_android` opens only URIs.
  Reason: platform — no other app reads the app's storage by path, and Android has no file manager that shows a file in its folder; androidx's `FileProvider` comes with a Gradle dependency the package does not have, so the provider is Inset's own, which inset-cli compiles and declares, not exported.
  Affect: an app built by inset-cli opens its own files in another app; without the provider opening a file fails.

- Change: `can_open_link` is `resolveActivity`'s answer, `None` from Android 11 for a scheme the manifest's `<queries>` does not name, which the application's meta-data `rs.inset.queried_schemes` lists, where `canLaunchUrl` answers false.
  Reason: platform — from Android 11 an app sees only the apps its `<queries>` names.
  Affect: an app built by inset-cli is answered for `http`, `https` and `mailto`.

## intents.rs / open_requests.rs → the Android embedder's `FlutterActivity` intent handling

- Change: the intent the activity started with and each one `onNewIntent` brings, which `InsetActivity` hands on, reach the client as `open_requested` when they are `ACTION_VIEW`: a `content://` document opened as a picked one is, to write where the intent grants writing, a `file://` URI as a path, and any other URI as a link; Flutter's activity hands them to its route and its plugins.
  Reason: platform — `NativeActivity` keeps `onNewIntent` to itself, and a document has no path.
  Affect: an app opened with a document reads it as it reads a pick; under the bare `NativeActivity` only the intent it started with comes.

## notifications/ → `flutter_local_notifications`' Android implementation

- Change: a notification is built by Inset's notification receiver, `rs.inset.InsetNotificationReceiver`, from a `Bundle` the host fills, and shown with `NotificationManager.notify` under its string id as the tag, so posting the id again replaces it, on one of two channels the receiver makes, "Notifications" and "Silent notifications", both `IMPORTANCE_HIGH` and the silent one without sound or vibration; `flutter_local_notifications` takes an int id and the channel the app describes.
  Reason: platform — a notification for later is built when its alarm goes off, in a process where the app's native code may not run, so the one builder is Java's, which inset-cli compiles and declares, not exported; Android 8 sets sound by channel, and `setSilent` came only with Android 12.
  Affect: a notification pops up, as the other systems' do, with the app's launcher icon as its small icon and the image as its large icon, fitted to the system's size; the user sees the two channels in the app's settings and may change either.

- Change: a section of the app's notifications is a notification channel group the host makes with two channels of its own, the sounding and the silent, both named as the app's own two; `flutter_local_notifications` makes the groups and channels the app describes in full.
  Reason: platform — Android sets the sound by channel, and the user turns notifications on and off by channel and by group.
  Affect: each section has its own heading and switch in the app's notification settings, and posts either kind; a later notification under an id replaces the one there in any section.

- Change: a tap on a notification, on one of its buttons, or a reply typed into its `RemoteInput` field starts `InsetActivity` with an intent of the receiver's own action, which the host reads at launch and from `onNewIntent` (`intents.rs`) and hands on as `notification_activated`; a button or a reply also takes the notification away.
  Reason: platform — `NativeActivity` keeps `onNewIntent` to itself, Android before 15 shows a reply as sending until its notification changes, and from Android 12 a receiver a tap reaches may not start the activity.
  Affect: the app hears a tap whether it was running or started by it, and every action, a reply included, brings it to the front; from Android 15 a replied notification stays, showing the reply, until the user dismisses it, as the system holds it past the host's cancel.

- Change: the permission is Android 13's `POST_NOTIFICATIONS`, which inset-cli declares, asked for with `requestPermissions` and answered through `InsetActivity.onRequestPermissionsResult`, with `shouldShowRequestPermissionRationale` telling a refusal the user may still be asked about; before Android 13, or for a package that targets an earlier release, it is `areNotificationsEnabled` alone, with no question.
  Reason: platform — Android tells no app a permission never asked from one refused for good.
  Affect: `state` answers `NotAsked` for a permission refused for good, and asking then answers `Denied` at once, without a question; a post never asks, as `notify` does not, and answers `Denied` until the permission is granted.

- Change: a schedule is an `AlarmManager` alarm whose intent carries the notification to the receiver, exact (`setExactAndAllowWhileIdle`) where the app may set exact alarms, and inexact (`setAndAllowWhileIdle`) where not; inset-cli declares `SCHEDULE_EXACT_ALARM`.
  Reason: platform — from Android 14 an app sets exact alarms only once the user grants it in the app's settings, and the system forgets alarms when the device restarts.
  Affect: a scheduled notification shows whether or not the app runs, late by as much as the system defers it where exact alarms are not granted, and not at all after a restart.

## display.rs / host.rs

- Change: the one screen is the activity's display, numbered by its own id, read when a window arrives and at each configuration change, and the app hears it changed only then; Flutter's embedder numbers it 0 and hears its refresh rate through a display listener.
  Reason: platform — a display listener is a Java interface, which `NativeActivity` gives the app no class to implement.
  Affect: the screen reads 0 on a phone as Flutter's does; a rate the panel changes on its own, with nothing else, is not heard until the next configuration change, and before API 30 the visible frame is the whole screen.

## input.rs

- Change: the pressure range reported with every touch is zero to one, rather than the range the device declares.
  Reason: platform — the engine reads it from the `InputDevice`, which the native glue does not hand out; reaching it needs a JNI lookup per device.
  Affect: a force press on a digitizer whose pressure is not scaled zero to one crosses its thresholds at the wrong pressure. Ordinary taps, drags and flings are unaffected.

## Deferred

- The keyboard, haptics, and the bars and keyboard as the system reports them. Trigger: any of them, which is a Java activity of the app's own, as Flutter's is.
- The predictive back gesture, with its peek animation. Trigger: a Java activity that registers with the system's back callback; the plain back button already pops.
- Deep links, which Flutter carries beside the back button. Trigger: an app that opens on a route the system names.
- Whether a view's window is minimized or occluded, which the activity's stop and start would tell, and where the window is on the screen. Trigger: an app that follows its windows on Android, as in desktop windowing.
