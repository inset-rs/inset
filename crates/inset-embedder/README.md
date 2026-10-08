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
- Call the start closure once, at launch, as soon as the views that exist then are in `Platform::views`, whether or not they have their windows yet, and keep the client it returns. A view with no window has no size, as Flutter's implicit view has empty metrics until its window reports them.

**Waking**
- `Dispatcher::wake_at`, `wake_now`
- Wake the framework at its timer, and as soon as possible after `wake_now`, from any thread.

**Background work**
- `Dispatcher::dispatch`
- Run dispatched work on the system's thread pool or the host's own threads, and on the web as a later task of the browser's loop.

**Quitting**
- `EmbedderClient::request_app_exit`, `AppExit::exit_application`
- Ask the app first, and quit only when it answers exit.

**Lifecycle states**
- `EmbedderClient::app_lifecycle_state_changed`, `Platform::initial_lifecycle_state`
- At each notice from the system, report the state the app is now in, and let the framework fill in the states between.

**Links, files and back**
- `EmbedderClient::pop_route`, `Platform::default_route_name`, `file_opening`, `FileOpening::set_open_files_handler`; a reopen is the host's own
- Keep what a launch hands over until the app listens, and do the system's own back when `pop_route` answers false.

## Windows and views

**Views**
- `View`, `Platform::views`, `implicit_view`, `EmbedderClient::view_added`, `view_removed`
- Update `Platform::views` before each `view_added` and `view_removed`, report with them only the views that come and go after launch, keep `implicit_view` the same view, among `Platform::views`, for the app's whole life, even once its window closes, and release a view's native window as soon as the window closes.

**Surfaces**
- none: a view's surface is the host's own; `EmbedderClient::frame`
- Give every view a surface whenever the system gives them and ask for a frame to draw on them, take every surface when the system takes them, and keep the app and its views throughout.

**Windows the app opens**
- `Platform::windowing_owner`, `WindowingOwner::create`, `adopt`, `HostWindow`
- Make each window on a turn of the loop, deliver it through the future, and give each `WindowConfig` option the system's nearest equivalent.

**Size and scale**
- `ViewMetrics::physical_size`, `device_pixel_ratio`, `EmbedderClient::view_metrics_changed`
- Give every size and position in physical pixels from the view's top-left corner, and update `View::metrics` before calling `view_metrics_changed`.

**Padding and insets**
- `ViewMetrics::view_padding`, `view_insets`, `padding`
- Report what the system may cover as `view_padding`, what the keyboard hides as `view_insets`, and `padding` as `max(0, view_padding - view_insets)` on each side.

**Screens**
- `Platform::screens`, `Screens::screens`, `pointer_location`
- Give every position in the coordinates of a window's frame, from the main screen's top-left corner, the main screen first.

**Displays**
- none yet
- Follow the system's notices as displays change and windows move between them, because a window's display sets its scale and its vsync.

**Visibility**
- `EmbedderClient::app_lifecycle_state_changed`, `Surface::next_vsync`
- A window nobody can see draws nothing, and draws again as soon as any part shows.

**Focus**
- `Platform::request_view_focus_change`, `EmbedderClient::view_focus_changed`
- Serve a focus request on a later turn of the loop, and report each native focus change with the system's direction, or `Undefined`.

## Frames

**Frames**
- `Platform::request_frame` → `EmbedderClient::frame`
- Draw one frame per vsync while the framework asks for one; a resize draws its frame at once.

**Vsync**
- `Surface::next_vsync`, `VsyncObserver::vsync`
- Take vsync from each display, or from a timer at its rate where the system gives none, so each visible window keeps ticking while others are hidden.

**Presenting**
- `View::present`
- Keep the last scene, and present it again as soon as the system can take it after a refused frame.

**Resizing**
- `EmbedderClient::view_metrics_changed`, then `EmbedderClient::frame`
- Draw the new size's frame inside the system's resize callback, so the size and the picture reach the screen in one transaction.

**Native surfaces**
- `Platform::compositor`, `Compositor::open_surface`, `Surface::present`, `Scene::surfaces`
- Place a scene's surfaces beneath its picture in the same native transaction, and run each buffer's `released` once the compositor is done with it.

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
- `Platform::clipboard`, `Clipboard::set_text`, `text`, `has_strings`
- Start each write inside the call, and answer each read once the text arrives, with nothing where the host cannot reach the clipboard.

**Menus and text services**
- `PopupMenus`, `SystemContextMenu`, `TextServices`, `EmbedderClient::system_context_menu_hidden`, `custom_context_menu_action`; none yet for the menu bar
- Perform the system context menu's built-in items in the host, and report its custom items and its closing to the client.

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

**Dock, taskbar and notifications**
- none yet
- Give the app the identity the system knows it by, a bundle on macOS and an AppUserModelID on Windows, before it reaches the Dock, the taskbar or notifications.

## Robustness

**Threads**
- `Dispatcher`, `Compositor`, `Surface` and `VsyncObserver`, the parts other threads may call
- Call the client only on the main thread, and reach each native window on the thread that owns it.

**Re-entrancy**
- `Platform`, `View`, `HostWindow`, `WindowingOwner` and each capability
- Whatever a framework call or a system callback needs of the loop or the client becomes a request, served in order on a later turn.

**Nested loops**
- `PopupMenus::show`
- Wakes and vsyncs keep coming while the system runs a nested loop, such as a popup menu, a modal dialog or a live resize.

**Panics**
- none yet
- A panic in a platform callback aborts after printing its message, and background work drops its own panic.

## References

Paths start at each repository's root.

- **Flutter**: `engine/src/flutter/shell/platform/`, with `darwin/macos`, `darwin/ios`, `android`, `windows`, `linux` and the embedder API in `embedder`; the web's is `engine/src/flutter/lib/web_ui/`.
- **gpui**: Zed's `crates/gpui_macos`, `crates/gpui_windows`, `crates/gpui_linux` and `crates/gpui_web`.
- **Chromium**: macOS in `components/remote_cocoa/`, `ui/base/cocoa/` and `ui/display/mac/`; Windows in `ui/views/win/`, `ui/base/win/` and `ui/display/win/`.
- **winit**: `src/platform_impl/`, one folder per system.
