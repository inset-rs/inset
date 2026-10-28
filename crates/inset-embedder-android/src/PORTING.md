# inset-embedder-android/src

Host implementation. Flutter analogue: the engine's Android embedder under
`shell/platform/android`, whose activity and view are written in Java.

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
- A window the app manages around the activity's view, with whether it is minimized and whether it is occluded, which the activity's stop and start would tell. Trigger: an app that manages its window on Android, as in desktop windowing.
