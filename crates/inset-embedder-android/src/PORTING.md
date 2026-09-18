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

- Change: the clipboard and haptics are absent, and the platform answers that it has neither.
  Reason: platform — both are Java APIs that need an activity of the app's own.
  Affect: copy and paste do nothing, and a widget that would buzz stays silent. The framework's own fallbacks run, rather than a capability that silently does nothing.

## input.rs

- Change: the pressure range reported with every touch is zero to one, rather than the range the device declares.
  Reason: platform — the engine reads it from the `InputDevice`, which the native glue does not hand out; reaching it needs a JNI lookup per device.
  Affect: a force press on a digitizer whose pressure is not scaled zero to one crosses its thresholds at the wrong pressure. Ordinary taps, drags and flings are unaffected.

## Deferred

- The keyboard, the clipboard, haptics, and the bars and keyboard as the system reports them. Trigger: any of them, which is a Java activity of the app's own, as Flutter's is.
- The predictive back gesture, with its peek animation. Trigger: a Java activity that registers with the system's back callback; the plain back button already pops.
- Deep links, which Flutter carries beside the back button. Trigger: an app that opens on a route the system names.
