# inset-embedder

A host connects Inset's framework to one platform: it runs the loop, delivers input, and puts frames on screen. Inset has no engine, so the host also does the engine's part.

## How an embedder is built

Every embedder takes the same shape, whatever its system; the winit embedder (`crates/inset-embedder-winit`) is the worked example.

- **The main loop talks.** One type hears every event of the system and every request of the app, and only it calls the client or asks the system for something. Its reactions live in files by concern: windows, frames, input, quitting.
- **Helpers decide.** Each concern, such as a view's text input, the pointer or the quit being asked, is a plain type that takes an event and answers what follows from it. The main loop delivers the answer, so the rule lives in one place and is tested without a window.
- **Shared facts have one owner.** What both the main loop and `Platform` need, such as the views, the appearance and the app's handlers, lives in one state that both hold and reach only through its named operations. The main loop changes it; `Platform` answers the app from it and sends the rest to the loop as requests.
- **A system's extras are listed once.** What a system adds beyond the windowing library is a list of small traits, one per concern, each with the answer of a system that lacks it: nothing, or `None`. A system fills in only what it has.
- **Missing is `None`, all the way to the app.** A capability an app may ask for is an `Option` on `Platform`, `None` where the system lacks it, never a call that silently does nothing. An ability several systems have goes into this interface; one only a single system has stays the embedder's own, behind a method named for that system, such as `WinitPlatform::macos`.

## App and loop

**Starting**
- the host's own `run`, whose start closure takes a `PlatformRef` and returns the `EmbedderClient`; `Platform::target_platform`
- Call the start closure once, at launch, as soon as the views the platform gives then are in `Platform::views`, whether or not they have their windows yet, and keep the client it returns. A view with no window has no size, as Flutter's implicit view has empty metrics until its window reports them. A desktop gives none.

**Waking**
- `Dispatcher::wake_at`, `wake_now`
- Wake the framework at its timer, and as soon as possible after `wake_now`, from any thread.

**Background work**
- `Dispatcher::dispatch`
- Run dispatched work on the system's thread pool or the host's own threads, and on the web as a later task of the browser's loop.

**Quitting**
- `EmbedderClient::request_app_exit`, `AppExitReply`, `AppExit::quit`
- Ask the app at every request the system makes, a second one while the first is being decided included, quit when its reply is exit, and quit at once when it calls `quit`. Act on both on a turn of the loop, never inside the call that answered.

**Lifecycle states**
- `EmbedderClient::app_lifecycle_state_changed`, `Platform::initial_lifecycle_state`
- At each notice from the system, report the state the app is now in, and let the framework fill in the states between.

**Being opened, and back**
- `EmbedderClient::open_requested`, `OpenRequest`, `EmbedderClient::pop_route`, `Platform::default_route_name`; a reopen is the host's own
- Hand on every file and link the system itself asks the app to open, at launch and while it runs, the launch's own once the client is there: a path where the system gives one, and a file opened as a pick where it gives a document or a handle. Leave the command line to the app: the system gives only strings there, and which are files is the app's to say. Do the system's own back when `pop_route` answers false.

## Windows and views

**Views**
- `View`, `View::initial_data`, `Platform::views`, `EmbedderClient::view_added`, `view_removed`
- List every view the app has, given or asked for, update `Platform::views` before each `view_added` and `view_removed`, report with them only the views that come and go after launch, and release a view's native window as soon as the window closes. Hand on what an embedder attached to a view it gives as its initial data.

**Surfaces**
- none: a view's surface is the host's own; `EmbedderClient::frame`
- Give every view a surface whenever the system gives them and ask for a frame to draw on them, take every surface when the system takes them, and keep the app and its views throughout.

**Views the app asks for**
- `Platform::request_view`, `View::request_id`, `View::window`, `HostWindow`, `EmbedderClient::view_close_requested`, `view_request_failed`
- Make a window for each request on a turn of the loop, add its view carrying the request's id and initial data, or report the failure, and give each `WindowConfig` option the system's nearest equivalent. Refuse requests where the platform gives the app its views. Report the user's request to close a view and close nothing until the app does.

**Minimized, occluded and moved**
- `HostWindow::is_minimized`, `is_occluded`, `frame`, `EmbedderClient::window_changed`
- Answer whether each window is minimized and whether it is occluded as the system says, `false` where it does not say, and call `window_changed` once the window answers the new state, each time either changes or the window moves. A resize is reported as a metrics change alone.

**Size and scale**
- `ViewMetrics::physical_size`, `device_pixel_ratio`, `EmbedderClient::view_metrics_changed`
- Give every size and position in physical pixels from the view's top-left corner, and update `View::metrics` before calling `view_metrics_changed`.

**Padding and insets**
- `ViewMetrics::view_padding`, `view_insets`, `padding`
- Report what the system may cover as `view_padding`, what the keyboard hides as `view_insets`, and `padding` as `max(0, view_padding - view_insets)` on each side.

**Screens**
- `Platform::screens`, `Screens::screens`, `pointer_location`, `Screen`, `ScreenId`, `ViewMetrics::screen`, `EmbedderClient::screens_changed`
- List every screen the system has, a phone's one included, the main one first, and give every position in the coordinates of a window's frame, from the main screen's top-left corner. Give each screen an id it keeps while it stays connected, and every view the id of the screen it is on. Follow the system's notices as screens change, calling `screens_changed` once `Platform::screens` answers the new list, and as windows move between screens, reporting a view whose screen changed through `view_metrics_changed`: a window's screen sets its scale and its vsync.

**Visibility**
- `EmbedderClient::app_lifecycle_state_changed`
- A window nobody can see draws nothing, and draws again as soon as any part shows.

**Focus**
- `Platform::request_view_focus_change`, `EmbedderClient::view_focus_changed`
- Serve a focus request on a later turn of the loop, and report each native focus change with the system's direction, or `Undefined`.

## Frames

**Frames**
- `Platform::request_frame` → `EmbedderClient::frame`
- At each vsync of a screen, give one frame of the views on it that asked for one before it came. Hold a frame asked of a window that is minimized or occluded until the window shows. Draw a resized view's frame inside the resize, as a frame of that view alone, as below.

**Vsync**
- `Platform::vsync_waiter`, `VsyncWaiter::next_vsync`, `VsyncObserver::vsync`
- Take vsync from each display, or from a timer at its rate where the system gives none, so each visible window keeps ticking while others are hidden. Hand a producer that waits for a screen's refresh the same ticks the frames on that screen are drawn at, one call per wait, whether or not a window on it is in sight.

**Presenting**
- `View::present`
- Keep the last scene, and present it again as soon as the system can take it after a refused frame, and on a surface the system gives anew: a frame presents only the views that changed.

**Resizing**
- `EmbedderClient::view_metrics_changed`, then `EmbedderClient::frame`
- Draw the new size's frame inside the system's resize callback, so the size and the picture reach the screen in one transaction.

**Native surfaces**
- `Platform::compositor`, `Compositor::root`, `SurfaceParent::open_child`, `Surface::present`, `SurfaceFrame`, `Scene::surfaces`
- Place a scene's surfaces beneath its picture in the same native transaction, and a surface frame's children beneath its buffer in the same one as the buffer, moved, clipped and scaled with their parent; and run each buffer's `released` once the compositor is done with it: where the system only answers whether it still is, as Core Animation does, ask at the surface's next present and when it goes.

## Input

**Mouse**
- `EmbedderClient::pointer_data_packet`
- Complete the data as Flutter's engine does: `Add` before a device's first event, a new `pointer_identifier` for each press, deltas from the last position, and `Remove` once it leaves with its buttons up.

**Scrolling and trackpad gestures**
- `PointerSignalKind::Scroll`, `Scale`, `ScrollInertiaCancel`, `PointerChange::PanZoomStart`, `PanZoomUpdate`, `PanZoomEnd`
- Send fingers on a trackpad as one pan-zoom with its pan, scale and rotation, and leave momentum to the framework.

**Touch and stylus**
- `PointerDeviceKind::Touch`, `Stylus`, `InvertedStylus`, `View::gesture_settings`
- Make each finger its own device, with its `Add` in the packet of its down and its `Remove` in the packet of its up or cancel.

**Keyboard**
- `KeyData::physical`, `logical`, `synthesized`
- Repeats and releases carry the logical key their press had, and the presses and releases the system lost arrive `synthesized`.

**Key order**
- `EmbedderClient::key_data` and its answer
- A composing input method gets each key first, then the framework, and a key the framework declines goes on to text input and the system.

**Text input**
- `TextInputHost`, `EmbedderClient::text_input_*`
- Input methods can read and change the text around the caret, so composing, dead keys and accents work.

**Cursor**
- `Platform::mouse_cursor`, `MouseCursor::activate_system_cursor`
- Show the last cursor asked for over the view the pointer is on, and set it again each time the system resets it.

**Drag and drop**
- `EmbedderClient::drop_data`
- Send one `DropData` per phase, with all its files in that one report, and the position where the system gives one.

## System services

**Clipboard**
- `Platform::clipboard`, `Clipboard::items`, `write`, `PastedItem`, `ClipboardItem`, `Content`, `FileSink`, `CopyOptions`, `EmbedderClient::clipboard_changed`
- List each item in the MIME types the system offers it in without reading it, fail a read once the clipboard has changed, write every form and option the system carries and refuse the forms it does not, write a promised file's bytes from its producer into the file where the receiver chose and remove it when the producer fails, clear a copy that expires where the system has no expiry of its own, waking for its time as for the app's timers, and call `clipboard_changed` whenever the system says another writer changed it or the host looks and finds such a change, or clears a copy that expired, counting the app's own writes as seen.

**Copy, cut and paste**
- `EmbedderClient::clipboard_event`, `ClipboardEvent`, `CopyRequest`, `edit_command`, `EditCommand`, `TextInputHost::provides_edit_commands`
- Send the platform's own copy, cut and paste, which a key reaches only once the framework has left it unhandled, with what was pasted, and with where a copy is written; send Edit's undo, redo, select all and delete as `edit_command`; and say whether the system's menus send copy, cut and paste, so a text field leaves those keys to them.

**Menus**
- `MenuEntry`, `MenuItem`, `ProvidedItem`, `Platform::menu_bar`, `popup_menus`, `dock_menu`, `tray_icons`, `MenuBar`, `MenuBar::perform_key_equivalent`, `PopupMenus`, `DockMenu`, `TrayIcons`, `TrayIconFuture`, `TrayIconError`, `TrayIcon`, `AttachedMenu`, `MenuAtIcon`, `EmbedderClient::menu_selected`, `dock_menu_selected`, `tray_icon_event`
- Answer each place with the system's own menu, or `None` where the system has none, never one the host draws; build a menu whole from the value each time it is set; make a provided item act as the system's own, Edit's sent to the focused view, and leave out one the system has no counterpart for; show the system's standard menu bar until the app sets its own; give the framework a key before the menu bar's key equivalents, and keep each key-down's own event a short while, bounded, so a key the app hands back later reaches the menu bar as the system's event; answer a tray icon once it exists, or why the system did not show it; and remove a tray icon when its handle drops.

**Text services**
- `SystemContextMenu`, `TextServices`, `EmbedderClient::system_context_menu_hidden`, `custom_context_menu_action`
- Perform the system context menu's built-in items in the host, and report its custom items and its closing to the client.

**File dialogs**
- `Platform::file_dialogs`, `FileDialogs::open_file`, `open_files`, `open_folder`, `save_file`, `OpenOptions`, `FolderOptions`, `SaveOptions`, `FileType`, `PickedFile`, `FileHandle`, `FileWriter`, `FileStream`, `PickedFolder`, `FolderHandle`, `FolderEntry`, `FileDialogError`
- Show the system's own panel, as a sheet on the parent view's window where the system has sheets; answer a path where the system gives one and the system's handle where it gives none, opening each pick as it is answered; grant writing only to a pick the app asked to write, and to a save's one file; answer `None` where the system has no file dialogs, and an error where it could not show one.

**Opening**
- `Platform::opener`, `Opener::open_link`, `open_file`, `reveal_file`, `can_open_link`, `OpenFuture`, `OpenError`
- Open each in the app the system gives it, and answer `NoApp` where the system says none takes it; share a file through the system where another app cannot read it by its path; answer `can_open_link` only where the system answers for every scheme, or for the schemes the app declared, and `None` otherwise; fail what the system has no counterpart for, such as revealing a file on a phone.

**Players and cameras**
- `Platform::players`, `Players::open`, `PickedFile::url`, `MediaPlayer`, `PlayerState`, `PlayerStatus`, `Platform::cameras`, `Cameras::cameras`, `Camera`, `CameraId`, `CameraFacing`, `Cameras::open_session`, `CaptureSession`, `CameraState`, `CameraStatus`, `PhotoOutput`, `Photo`, `MovieOutput`, `MovieOptions`, `FrameReader`, `PixelBuffer`, `MediaError`
- Play and capture with the system's own media stack, answering `None` where it has none; send a picture to one destination at a time, drawn by the system into a surface's own layer, clipped and scaled with it, or handed to a frame reader as the newest GPU-importable buffer; read a player's or a session's state when asked and resolve its `changed` future at every change and when it goes; ask the user for the camera, and the microphone where a recording wants sound, at a session's start; and answer photos encoded and recordings written by the system.

**Dialogs**
- none yet
- Start each dialog with the system's non-blocking call and answer through a future, so the host's loop keeps running.

**Appearance and locale**
- `Platform::platform_brightness`, `locales`, `set_application_locale`, `EmbedderClient::platform_brightness_changed`, `locales_changed`
- Update what `Platform` answers before calling its change hook, and list the locales most preferred first.

**Fonts and images**
- `Platform::font_source`, `open_image_codec`, `import_pixels`, `EmbedderClient::system_fonts_changed`
- Call `system_fonts_changed` after registering faces that arrive late, and answer every decode, off the main thread, with its frames or an error.

**System chrome and haptics**
- `Platform::system_chrome`, `haptics`
- Style the system bars on the thread that owns the window, which on Android is the Java main thread.

**Restoration**
- `Platform::restoration`, `Restoration::get`, `put`
- Keep the last map `put` hands over until the system asks for it, and answer `get` with what the system gave back at launch.

**Accessibility**
- none yet
- A host adds accessibility when the framework does.

**Notifications**
- `Platform::notifications`, `Notifications::post`, `remove`, `permission`, `replies`, `scheduling`, `Notification`, `NotificationAction`, `NotificationPermission`, `PermissionState`, `NotificationReplies::reply_action`, `NotificationScheduling::schedule`, `PostFuture`, `NotificationError`, `EmbedderClient::notification_activated`, `NotificationActivation`
- Post each under its id as the system's own id or tag, so a later post under it replaces it and `remove` takes it away, shown or scheduled; ask with the system's own question before a post where the user was never asked, and answer `Denied` where they said no; hand on every click with its action and reply, the one that started the app once the client is there; answer `None` where the system would not show the app's notifications, and for a reply, a later delivery or a permission the system lacks, never standing in for one.

**Dock and taskbar**
- none yet
- Give the app the identity the system knows it by, a bundle on macOS and an AppUserModelID on Windows, before it reaches the Dock, the taskbar or notifications.

## Robustness

**Threads**
- `Dispatcher`, `Compositor`, `Surface` and `VsyncWaiter`, the parts other threads may call
- Call the client only on the main thread, and reach each native window on the thread that owns it.

**Re-entrancy**
- `Platform`, `View`, `HostWindow` and each capability
- Whatever a framework call or a system callback needs of the loop or the client becomes a request, served in order on a later turn.

**Nested loops**
- `PopupMenus::show`, `MenuAtIcon::show`, `PopupMenuFuture`
- Wakes, vsyncs and frames keep coming while the system runs a nested loop, such as a popup menu, a modal dialog or a live resize: open a popup menu after `show` returns, its loop outside every call into the app, and answer the item chosen through the future once it closes.

**Panics**
- none yet
- A panic in a platform callback aborts after printing its message, and background work drops its own panic.

## References

Paths start at each repository's root.

- **Flutter**: `engine/src/flutter/shell/platform/`, with `darwin/macos`, `darwin/ios`, `android`, `windows`, `linux` and the embedder API in `embedder`; the web's is `engine/src/flutter/lib/web_ui/`.
- **gpui**: Zed's `crates/gpui_macos`, `crates/gpui_windows`, `crates/gpui_linux` and `crates/gpui_web`.
- **Chromium**: macOS in `components/remote_cocoa/`, `ui/base/cocoa/` and `ui/display/mac/`; Windows in `ui/views/win/`, `ui/base/win/` and `ui/display/win/`.
- **winit**: `src/platform_impl/`, one folder per system.
