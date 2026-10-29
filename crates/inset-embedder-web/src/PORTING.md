# inset-embedder-web/src

Host implementation. Flutter analogue: the web engine under `engine/lib/web_ui`.

## gpu.rs / platform.rs

- Change: each frame draws into a backing texture that has `COPY_SRC`, then blits 1:1 onto the canvas swapchain.
  Reason: platform — WebGPU's canvas swapchain is `RENDER_ATTACHMENT` only, and Valo's advanced blends and backdrop filters copy the target.
  Affect: a cupertino nav-bar blur or a scrolling clip still presents; drawing straight to the swapchain invalidates the command buffer.

- Change: the valo context hides missing glyphs.
  Reason: platform — Valo paints `.notdef` tofu unless asked not to.
  Affect: a character with no face yet occupies layout space and draws nothing.

## fonts.rs / font_fallback.rs / google_fonts.rs

- Change: the interface font is compiled into the binary instead of downloaded at start-up, and no other family name is answered.
  Reason: platform — Flutter web fetches Roboto before the first frame and waits for it; shipping the same slice needs neither the wait nor the request, and Flutter web has no font by any other name either.
  Affect: the first frame has text at every weight; a font an application wants beyond that is bytes it registers, never a name fetched from the network.

- Change: a character outside that slice is drawn from one slice of the family carrying its script — Roboto for the scripts Roboto has, otherwise that script's Noto family — fetched on first use together with the family's slice list; a character no slice carries, or whose slice fails to download three times, is given up on.
  Reason: platform — Flutter ships a generated list of every Noto slice inside the engine; the same list for one family arrives with its stylesheet, so it is read when the family is first needed.
  Affect: a new script costs one stylesheet and the slices its text falls in, which the browser caches across pages, and appears a frame or two after the text; a character nothing can draw is asked for once.

- Change: the family for a script is chosen from a table of one family per script, reduced from Flutter's generated data.
  Reason: with slice lists read per family, only the family needs choosing; which slice carries a character is in the family's own list.
  Affect: a character its script's family lacks stays blank where Flutter would try further fonts; scripts other than Roboto's come in one weight, so bold text in them renders regular, as on Flutter web.

## screen.rs / host.rs

- Change: the page's one screen is `window.screen`, numbered 0 as Flutter web's display is, read again at each resize, with no refresh rate where Flutter web says 60, and a frame in CSS pixels placed where the browser says.
  Reason: platform — the browser reports no refresh rate, and gives no screen's place but through the non-standard `screen.left` and `screen.top`.
  Affect: the screen's rate reads `None`, a zoom is heard as the screens changing, and a browser without `screen.left` places the screen at the origin.

## images.rs / platform.rs

- Change: `Platform::open_image_codec` decodes with `createImageBitmap` and uploads through valo's `ImageContext` (`copyExternalImageToTexture`), then answers a one-frame codec.
  Reason: platform — the browser is the codec; valo copies an `ImageBitmap` onto the GPU without a CPU round-trip.
  Affect: PNG, JPEG, WebP, GIF and BMP the browser reads become drawable images; an animated file shows its first frame only.

## text_input.rs / platform.rs

- Change: `View` text input is a transparent DOM `input` or `textarea`, styled and placed like Flutter web's editing element, with `input` / composition / Enter forwarded as `TextEditingValue` and `TextInputAction`. While composition is in progress, `set_editing_state` does not write the DOM value or selection.
  Reason: platform — the browser IME talks to a focused HTML field, not winit's preedit events, and assigning `input.value` cancels composition.
  Affect: a field that has called `TextInput::attach` accepts typing and composition, a composing CJK cluster is not wiped by a caret update, the candidate window sits at the caret, and autofill / delta-model / a visible overlay field are absent.

## file_dialogs.rs → `file_selector_web`

- Change: where the browser has the File System Access API, a pick is the browser's file handle, read through `File.slice` and written through `createWritable({ keepExistingData: true })`, the file replaced when the writer finishes, where `file_selector_web` reads through an `<input type="file">` only and saves nothing.
  Reason: platform — a page has no paths, and the handle is the one thing the browser lets a page read and write again.
  Affect: on Chrome and Edge an app opens, writes and saves a file the user chose; an open pick asks for write permission only when the app asked to write.

- Change: elsewhere opening falls back to an `<input type="file">`, whose files are read-only handles, and a save fails.
  Reason: platform — Safari and Firefox let a page read a file the user chose, never write one, and a download would give the app nothing it could write again.
  Affect: on Safari and Firefox an app opens files but saves none through a dialog.

- Change: a pick's URL is a `blob:` URL of its file as it was picked, which a file handle's file is taken for as the pick is answered, made once and never revoked, where `file_selector_web`'s `XFile` holds a blob URL of its input's file.
  Reason: platform — a file handle gives its file only through a promise, and a player may read the URL after the pick is let go.
  Affect: a player plays a pick on the web, the URL shows the file as it was picked, and the page keeps the file until it unloads.

- Change: a folder pick is the browser's directory handle (`showDirectoryPicker`, in `readwrite` mode when the app asks to write), as a `FolderHandle`: listed through `values()`, its files and folders opened by name, and made when writable; without the File System Access API a folder pick fails.
  Reason: platform — a page has no paths, and Safari's and Firefox's `webkitdirectory` input gives a flat list of files no folder could be written through.
  Affect: on Chrome and Edge an app lists, reads and writes a folder the user picked; elsewhere it asks for one in vain.

## opener.rs / launch_queue.rs → `url_launcher_web`, and the web app's file handlers

- Change: a link opens in a new browsing context through `window.open`, its `opener` cut as a `noopener` link's is, and fails where the browser blocks the window, as it does outside the user's click or key; `url_launcher_web` opens with `noopener` and cannot tell a blocked window.
  Reason: platform — `window.open` with `noopener` answers no window whether it opened one or not.
  Affect: an app opens links while it handles a click, and hears a failure when the browser refused.

- Change: opening and revealing a file fail, and `can_open_link` is `None`.
  Reason: platform — a page has no paths, and the browser never says which schemes some app takes.
  Affect: an app that opens files does so on the systems that have them.

- Change: an installed app opened with files hears them through `window.launchQueue`, at launch and, where its manifest launches into the window it has, while it runs, each a read-only `FileSystemFileHandle`; Flutter's web engine does not read the launch queue.
  Reason: platform — the browser grants a launch's files reading alone, and asking to write needs the user's gesture.
  Affect: an installed app whose manifest declares `file_handlers` opens the user's files on Chrome and Edge, and a folder handle in a launch is left out; a scheme the manifest's `protocol_handlers` registers reaches the page in its own address, which the app reads itself.

## notifications.rs → `flutter_local_notifications`' implementations, on the browser's `Notification`

- Change: the notifications are the page's own, `new Notification` with the app's id as its `tag`, which the browser replaces by, alerting again unless the notification is silent; the page keeps the object it made for each id, so `remove` closes it, and a click on one reaches the app as an activation with no action, the browser's own response of bringing the page forward left as it is; they are `None` where the page has no `Notification`, as outside a secure context.
  Reason: platform — a notification a page shows itself belongs to that page, and only a service worker's outlives it.
  Affect: an app's notifications and their clicks work while its page is open, a click after the page closed reaches nothing, and `remove` reaches only the notifications this visit showed.

- Change: a notification shows without its actions, and `replies()` and `scheduling()` are `None`.
  Reason: platform — buttons and typed replies belong to a service worker's notifications (`showNotification`, `notificationclick`), which Inset gives the app none of, and the browser holds no notification for later.
  Affect: on the web a notification's buttons are left out, a click on it names no action, and a reminder for later is the app's to keep while its page runs.

- Change: `permission()` is the browser's for the page's origin, `default` being `NotAsked`, and a post where the user was never asked calls `Notification.requestPermission` at once, before the app awaits it, and answers `Denied` where the user declined or dismissed the question.
  Reason: platform — browsers show the question only while the page handles the user's click or key, and some ignore it outside one.
  Affect: an app's first post shows the browser's question when it posts while handling a click, and is denied without one where the browser requires one.

- Change: the image is the notification's `icon`, a PNG `data:` URL the browser's own canvas encodes; a post the browser refuses, as Chrome on Android refuses every page's own notification, fails with what it threw.
  Reason: platform — a notification takes its icon by URL, and a page cannot tell beforehand whether the browser will make its notifications.
  Affect: an app's image shows where the browser shows icons, and on Chrome for Android every post fails.

## Deferred

- Hearing the pixel ratio change without a resize, as on a move to a screen at another ratio, which Flutter web hears through a `(resolution)` media query, and the screen's own `change` event. Trigger: an app that must follow its page across screens.
- Animated image frames after the first. Trigger: a GIF or APNG that must play.
- Flutter web autofill form, delta model, and putting the editing element inside a view's `textEditingHost`. Trigger: AutofillGroup; `enableDeltaModel`.
- A window the app manages around the page's view, with whether it is occluded, which the page's visibility state and an intersection observer on the canvas would tell, as winit's web backend reads it. Trigger: an app that must stop drawing while its page is hidden.
- A notification's buttons, and its clicks once the page has closed, through a service worker's `showNotification` and `notificationclick`. Trigger: an installed web app whose notifications carry actions.

## clipboard.rs → the web engine's clipboard and its text field's paste

- Change: the app's own clipboard is the browser's asynchronous clipboard in every type it carries, listed by `read` and written by `write`, a promise resolved at the write, files refused and options left out.
  Reason: platform — Flutter's web engine reads and writes plain text only; the page reaches as much as the browser lets it.
  Affect: an app copies HTML or an image to other apps; a read may ask the user first, and Firefox and Safari show their own Paste button.

- Change: the user's copy, cut and paste on the canvas or the text field reach the framework as clipboard events, the browser's default prevented, where Flutter's web engine lets the browser paste into its hidden text field.
  Reason: platform — the browser's `paste` event is the only read it allows without asking.
  Affect: a paste reaches whatever widget has the focus, in every type pasted, files included, and never asks the user.

- Change: the app hears the clipboard changed at the browser's `clipboardchange`, and at each focus where the browser has none.
  Reason: platform — a page cannot otherwise tell another application wrote the clipboard without reading it.
  Affect: on a browser without the event, the app hears a change it cannot confirm each time the page takes the focus.
