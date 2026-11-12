# waft-embedder

A host connects Waft's framework to one platform: it runs the loop, delivers input, and puts frames on screen. Waft has no engine, so the host also does the engine's part.

## How an embedder is built

Every embedder takes the same shape, whatever its system; the winit embedder (`crates/waft-embedder-winit`) is the worked example.

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
- Make every view's surface from one source per system: where the compositor shows a view's drawing as a composition layer's content, as Core Animation does and Android's surface controls do, from a layer of the view's own, which the view's window places to fill it and places again in the transaction that changes the bounds or scale of the window's view; elsewhere from the window (native-surfaces decision 34, `waos/docs/proposals/2026-10-30-native-surfaces.md`).

**What a view asks of the system**
- `View::host`, `ViewHost`
- Give every view a host that takes what it asks of the system, its input methods, its cursor and its focus requests: a window's view its window's (native-surfaces decision 35, `waos/docs/proposals/2026-10-30-native-surfaces.md`).

**Views the app asks for**
- `Platform::request_view`, `View::request_id`, `View::window`, `HostWindow`, `EmbedderClient::view_close_requested`, `view_request_failed`
- Make a window for each request on a turn of the loop, add its view carrying the request's id and initial data, or report the failure, and give each `WindowConfig` option the system's nearest equivalent. Refuse requests where the platform gives the app its views. Report the user's request to close a view and close nothing until the app does.

**Size limits**
- `HostWindow::size_limits`, `WindowSizeLimits::set_min_size`, `set_max_size`
- Answer limits for each window its user resizes, and none where the system sizes the app's windows itself, as on a phone; hold the window's content, without the system's title bar and border, between them as the user resizes it, in logical points, and drop a limit set to none.

**Views the app adds on layers**
- `Platform::view_embedder`, `ViewEmbedder::add_view`, `set_view_metrics`, `set_view_in_sight`, `remove_view`, `send_pointer_reports`, `send_view_focus_event`
- Where the compositor shows a view's drawing as a composition layer's content, give the app a view embedder, and elsewhere none. Add a view on a layer of the app's own on a turn of the loop, as a window's view is added, carrying the request's id and asking the app's host what it asks of the system, and refuse at once a layer the view cannot draw into. Give the view the metrics, the sight and the end the app says, refusing them for any other view and reporting the first refusal, and hand the client the pointer reports and focus changes the app sends as the host's own (native-surfaces decisions 33 and 36, `waos/docs/proposals/2026-10-30-native-surfaces.md`).

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
- `ViewHost::request_focus_change`, `EmbedderClient::view_focus_changed`
- Serve a focus request on a later turn of the loop, and report each native focus change with the system's direction, or `Undefined`.

## Frames

**Frames**
- `Platform::request_frame` → `EmbedderClient::frame`
- At each vsync of a screen, give one frame of the views on it that asked for one before it came. Hold a frame asked of a window that is minimized or occluded until the window shows. Draw a resized view's frame inside the resize, as a frame of that view alone, as below.

**Vsync**
- `Platform::vsync_waiter`, `VsyncWaiter::next_vsync`, `VsyncObserver::vsync`
- Take vsync from each display, or from a timer at its rate where the system gives none, so each visible window keeps ticking while others are hidden. Hand a producer that waits for a screen's refresh the same ticks the frames on that screen are drawn at, one call per wait, whether or not a window on it is in sight, with the moment the picture drawn for that refresh reaches the screen: the display link's target time, or one refresh on where a timer stands in for it.

**Presenting**
- `View::present`, `Scene`, `slices`
- Keep the last scene, the frame's layer tree, and present it again as soon as the system can take it after a refused frame, and on a surface the system gives anew: a frame presents only the views that changed. Draw what `slices` makes of the tree, or walk the tree yourself where the system needs a walk of its own.

**Resizing**
- `EmbedderClient::view_metrics_changed`, then `EmbedderClient::frame`
- Draw the new size's frame inside the system's resize callback, so the size and the picture reach the screen in one transaction.

**Composition layers**
- `Platform::compositor`, `Compositor::make_layer`, `CompositionLayer::present`, `CompositionLayer::current_picture`, `LayerPicture`, `HardwareBuffer`, `slices`, `Slices::composition_layers`, `Slices::picture_with_room`, `Slices::paints`, `Placement`, `Mutator`
- Place the composition layers a scene embeds beneath the picture its slices make, with room made for each, in the same native transaction, applying as much of each layer's transforms and clips as the compositor can and never its opacity, which the room carries; draw nothing for a scene that paints nothing besides its layers where the system shows the layers itself, and one empty frame when such a scene follows one that painted; show a present's buffer, one of your own `HardwareBuffer`s, in place of what the layer showed, the buffers before it or a layer the system draws into, and its children beneath it in the same transaction as the buffer, moved, clipped and scaled with their parent, and let go of the children it no longer names; hold the buffer, and the frame it is the memory of, until the compositor is done with it; show nothing for a buffer another host handed out, and say so; give a layer one placer at a time, the first to place it, until that placer lets it go; and run each buffer's `released` once the compositor is done with it, at once for a buffer not shown: where the system only answers whether it still is, as Core Animation does, ask at the layer's next present and when it goes. Answer what a layer shows now: the buffer presented last, as `LayerPicture::Buffer`, or the current frame of the player's video or the camera's picture shown on it, as `LayerPicture::Frame`, one of the `VideoFrame`s your readers hand out, read from an output the source feeds beside the layer where the system hands nothing back of a layer it draws into; and `None` while the layer shows nothing, or a system layer you do not read.

**Hardware buffers**
- `Platform::hardware_buffer_allocator`, `HardwareBufferAllocator::allocate`, `HardwareBuffer`, `usage`, `read_pixels`, `HardwareBufferUsage`, `Platform::import_texture`, `Platform::import_hardware_buffer`
- Allocate GPU memory the compositor shows with the system's own allocator, such as `IOSurfaceCreate` or `AHardwareBuffer_allocate`, answering `None` where the system has none, from any thread, for at least the users its usage names, the display's overlays where `composer_overlay` and the CPU where `cpu_read`; refuse a format the system does not allocate as `Unsupported`, a side past the largest texture it makes as `TooLarge`, no memory left as `OutOfMemory`, and a side of no pixels as `Other`; implement `HardwareBuffer` over your system's buffer: its size; its usage, all the system lets it be used for, at least what it was allocated for, as Android's `AHardwareBuffer_describe` answers it, and every use for a buffer allocated on a system where any buffer has every use, as an `IOSurface` has; and its pixels copied into the app's memory, as they lie while the system lets the CPU read them, such as a surface locked for reading, and `Unsupported` for a buffer whose usage lacks `cpu_read`; import a buffer you handed out as a texture on the device given, over its memory with no copy, as your system's GPU API imports such memory, for what its usage allows, sampled where `gpu_sampled`, drawn into and copied into where `gpu_color_output`, and copied from whatever its usage, holding the buffer, and a frame's buffer its frame, until the GPU is done with it, or none for a buffer another host handed out and a device that cannot take its memory; and import a buffer on the renderer's device as an image (program-media decisions 35 to 37, `waos/docs/proposals/2026-11-02-program-media.md`).

## Input

**Mouse**
- `EmbedderClient::pointer_reports`, `PointerReport`, `ReportedChange`
- Report what the system says: `Add` when the mouse comes into a view, `Down`, `Move` and `Up` as its buttons change, each with the buttons held, and `Remove` once it leaves with its buttons up. The client adds what Flutter's engine adds (`PointerDataPacketConverter`): each press's identifier, the deltas, and the changes the system left out.

**Scrolling and trackpad gestures**
- `ReportedChange::Scroll`, `Scale`, `ScrollInertiaCancel`, `PanZoomStart`, `PanZoomUpdate`, `PanZoomEnd`
- Send fingers on a trackpad as one pan-zoom with its pan, scale and rotation, each the total since it started, and leave momentum to the framework.

**Touch and stylus**
- `PointerDeviceKind::Touch`, `Stylus`, `InvertedStylus`, `Contact`, `View::gesture_settings`
- Make each finger its own device, numbered apart from every other device the host reports, since the client keeps its state by that number, and remove it after its up or cancel.

**Keyboard**
- `KeyData::physical`, `logical`, `synthesized`
- Repeats and releases carry the logical key their press had, and the presses and releases the system lost arrive `synthesized`.

**Key order**
- `EmbedderClient::key_data` and its answer
- A composing input method gets each key first, then the framework, and a key the framework declines goes on to text input and the system.

**Text input**
- `ViewHost::text_input`, `TextInputHost`, `EmbedderClient::text_input_*`
- Input methods can read and change the text around the caret, so composing, dead keys and accents work.

**Cursor**
- `ViewHost::activate_system_cursor`
- Show the last cursor a view's host is asked for while the pointer is over that view, and set it again each time the system resets it.

**Drag and drop**
- `EmbedderClient::drag_report`
- Send one `DragReport` per phase, with the drag's items listed as a paste's are, through `DraggedItems` so they are read only while the drop is reported, as the HTML spec's drag data store is protected until the drop, and the position where the system gives one; fetch what is read then before the system lets the drop's data go, and refuse a drag carrying nothing the app could read.

## System services

**Clipboard**
- `Platform::clipboard`, `Clipboard::items`, `write`, `DataTransferItem`, `ClipboardItem`, `Content`, `FileSink`, `CopyOptions`, `EmbedderClient::clipboard_changed`
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

**Video frames**
- `SceneBuilder::add_video`, `VideoLayer`, `slices`' `frame_image`, `VideoFrameReader::frame_for`, `VideoFrame`, `VideoFrame::read_pixels`, `VideoFrame::hardware_buffer`
- Hand `slices` the image of each reader's frame as you draw, the frame's hardware buffer imported as a texture on your device and that as an image, asking the reader for the frame of the moment the picture reaches the screen, the display link's target time, when a draw first asks for it, and keep only the frames of the readers your draws still ask for; while a scene draws a video and its view is in sight, ask each reader so at every vsync and draw the scene again when a new frame came; and implement `VideoFrame` over your system's frame: its size; its pixels copied into the app's memory, in the layout the frame holds them, as it lies while the system lets the CPU read it, such as a pixel buffer locked for reading, or `Unsupported` for a frame in no `PixelFormat`; and its memory as a `HardwareBuffer` of yours for sampling, which holds the frame, and its slot in its pool, while it and every texture imported of it are held, since a decoder reuses a frame's memory once nothing holds the frame; a frame whose buffer your device cannot take, your drawing skips.

**Players and cameras**
- `Platform::players`, `Players::open`, `PickedFile::url`, `MediaPlayer`, `MediaPlayer::set_video_output`, `VideoOutput`, `PlayerState`, `PlayerStatus`, `PlayerActivity`, `PauseReason`, `WaitReason`, `PlayerError`, `PlayerErrorKind`, `TimeRange`, `SeekTolerance`, `MediaPlayer::position`, `MediaPlayer::set_muted`, `MediaTrack`, `MediaTrackKind`, `MediaPlayer::select_audio_track`, `select_text_track`, `metadata`, `cues`, `MediaMetadata`, `EncodedImage`, `Cue` and its `CueContent`, `CueTextRun`, `CueImage`, `CueLine`, `CueAnchor`, `CueTextAlignment`, `CueVertical`, `CueTextEdge`, `Platform::cameras`, `Cameras::cameras`, `CameraListFuture`, `Cameras::changed`, `Camera`, `CameraId`, `CameraFacing`, `Cameras::open`, `CameraOpenFuture`, `CameraDevice`, `CameraDevice::formats`, `CameraFormat`, `CameraDevice::start`, `CameraStartFuture`, `CameraOutputs`, `CaptureSession`, `CaptureSession::format`, `take_photo`, `start_recording`, `stop_recording`, `CaptureState`, `CameraInterruption`, `CameraError`, `Photo`, `MovieOptions`, `Platform::video_frame_readers`, `VideoFrameReaders::make_reader`, `VideoFrameReader`, `VideoFrame`, `MediaError`
- Play and capture with the system's own media stack, answering `None` where it has none; send a picture to one destination at a time, which the app made: drawn by the system into a layer that is a composition layer's content, clipped and scaled with it, or handed to a frame reader made on its own as the `VideoFrame` for the moment the picture reaches the screen; feed a reader from the source it was given to last, and from no source once that one is given another output or goes; list a player's tracks, play the audio track and show the text track the app selects, a text track allowing none where the media does, and answer the media's tags; hand out the cues of the text track shown as the media styles them, each field the media leaves unsaid `None` and the user's caption style not merged in, and draw no subtitles yourself; read a player's or a session's state, metadata and cues, and a player's position, when asked, and resolve its `changed` future at every change of any of them, at every jump of the player's position, such as a seek landing or a loop, and when it goes, but not as the position moves on while it plays; tell in a player's state what the system's player does, paused, waiting or playing, with the reason where the system gives one, a seek under way from its call until it lands, the position reading its target meanwhile, the parts loaded and seekable, the rate, volume and mute it plays with as the system reports them, and a failure with its kind whenever it comes; land a seek within the tolerance asked for, or exactly (program-media decisions 17, 18 and 22 to 28, `waos/docs/proposals/2026-11-02-program-media.md`); list cameras without asking the user, and resolve each future the cameras' `changed` answered at the list's next change, such as a camera plugged in or out, as the system tells it; ask for the cameras as one opens, and for the microphone as a session whose recordings want sound starts; list a camera's formats, and start it in the one picked with all its outputs configured together, or, where the system runs a camera in one format at a time and another session runs it, in that session's format, which the session's `format` answers, starting it where the system's documentation says, such as a serial queue of its own on Apple's systems, whose `startRunning` blocks, and answering the session once it runs or is interrupted; fail a start for a camera in use, unplugged or not allowed, and for a format it does not list; tell a session running, interrupted, for a reason where the system gives one, which it resumes from by itself, or ended for an error, for good; and answer photos encoded by the system at the size asked for and recordings written by the system.

**Media assets**
- `Platform::media_assets`, `MediaAssets::open`, `MediaAsset::duration`, `tracks`, `metadata`, `frame_at`, `MediaAssetFuture`
- Read media without playing it with the system's own reader, answering `None` where it has none; open it by the URL a player takes, reading nothing until asked; answer its length, `None` for a live stream as a player's state has, its tracks as a player lists them, each `selected` that a player would play unless asked otherwise, and its tags as a player's metadata; answer a picture at a time within the tolerance asked for, presented as a player presents it, its track's rotation applied, scaled down to fit the size asked for where one is given, as the `VideoFrame` your frame readers hand out, so the app draws it as any frame; and answer each future once, with a failure of the player's kinds where the system could not read it, whether or not the asset is still held (program-media decision 32, `waos/docs/proposals/2026-11-02-program-media.md`).

**Image codecs**
- `Platform::image_codecs`, the `image_codecs` module: `ImageCodecs::decoded_formats`, `encoded_formats`, `open`, `open_file`, `encode_pixels`, `hardware_image_codecs`, `ImageSource::mime_type`, `size`, `frames`, `orientation`, `metadata`, `decode_pixels`, `HardwareImageCodecs::decode`, `encode`, `ImageFrame`, `ImageOrientation`, `ImageRotation`, `ImageMetadata`, `DateTime`, `Location`, `ImageError`, `ImageErrorKind`, `ImageFuture`, `Platform::import_hardware_buffer`, and valo's `PixelBuffer`, `PixelLayout`, `PixelFormat` and `AlphaType`
- Decode and encode with the system's own codecs, answering `None` where it has none, and `None` for the hardware codecs where it cannot decode into a `HardwareBuffer` or encode from one; name each format by its MIME type; open an image from bytes or from a file, reading the file only as far as asked, and answer at once what it says of itself: its format, its size upright, its frames with an animation's delays, its EXIF orientation, and EXIF's common fields with its GPS location as the file holds them, each `None` where the file says nothing; decode a frame upright, scaled down to fit the size asked keeping its shape, into the app's memory in a layout you name in the `PixelBuffer`, and, by the hardware codecs, into a `HardwareBuffer` of the app's own for sampling, which holds no slot in a pool, `TooLarge` where it is larger than your system's buffers; encode pixels and hardware buffers, an allocated one's, a decoded image's and a frame's among them, and fail one another host handed out as `Unsupported`, writing no metadata, at the quality asked for a lossy format; fail a frame the image has not, a size of no pixels and a quality outside 0 to 1 as `Other`, a format you do not read or write as `Unsupported`, bytes you cannot read as `Damaged` and a missing file as `NotFound`; do the work off the main thread, and answer each future once, whether or not the app still holds what it asked of (program-media decision 33, `waos/docs/proposals/2026-11-02-program-media.md`).

**Caption settings**
- `Platform::caption_settings`, `CaptionSettings::style`, `changed`, `CaptionStyle`, `CaptionSetting`, `CaptionBehavior`, `CaptionTypeface`, `GenericCaptionFont`
- Answer the user's caption settings where the system keeps them, and `None` where it keeps none, such as Linux and the web; answer the style as the system's settings give it when asked, each setting with whether it applies always or only where the media says nothing, and `None` for a setting the system gives no value for; and resolve each future `changed` answered at the style's next change, as the system tells it (program-media decisions 18 and 19, `waos/docs/proposals/2026-11-02-program-media.md`).

**Dialogs**
- none yet
- Start each dialog with the system's non-blocking call and answer through a future, so the host's loop keeps running.

**Appearance and locale**
- `Platform::platform_brightness`, `locales`, `set_application_locale`, `EmbedderClient::platform_brightness_changed`, `locales_changed`
- Update what `Platform` answers before calling its change hook, and list the locales most preferred first.

**Fonts and images**
- `Platform::font_source`, `open_image_codec`, `import_pixels`, `import_video_frame`, `EmbedderClient::system_fonts_changed`
- Call `system_fonts_changed` after registering faces that arrive late, and answer every decode, off the main thread, with its frames or an error; import a frame the app holds as an image with no copy, its hardware buffer imported as a texture on the device your images are drawn with, and `None` where that device cannot take the frame's memory.

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
- Post each under its id as the system's own id or tag, so a later post under it replaces it and `remove` takes it away, shown or scheduled; ask the user only in `NotificationPermission::request`, with the system's own question, and answer a post or a schedule the system does not allow with `Denied`, never asking; hand on every click with its action and reply, the one that started the app once the client is there; answer `None` where the system would not show the app's notifications, and for a reply, a later delivery or a permission the system lacks, never standing in for one.

**Dock and taskbar**
- none yet
- Give the app the identity the system knows it by, a bundle on macOS and an AppUserModelID on Windows, before it reaches the Dock, the taskbar or notifications.

## Robustness

**Threads**
- `Dispatcher`, `Compositor`, `CompositionLayer`, `HardwareBufferAllocator`, `HardwareBuffer` and `VsyncWaiter`, the parts other threads may call
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

## The `serde` feature

Off by default. With it, the values a host gives a view and takes from it derive serde's `Serialize` and `Deserialize`: `PointerReport` and what it holds, `KeyData` and its kinds, `ViewFocusEvent` and its state and direction, `ViewMetrics` and what it holds, `ViewId`, `ScreenId` and `SystemMouseCursorKind`. It is for an app that sends a view's input, and what the view asks of its host, between processes, as WaOS does for a view it shows in a program's window: the app sends Waft's own types rather than copies of them.

## References

Paths start at each repository's root.

- **Flutter**: `engine/src/flutter/shell/platform/`, with `darwin/macos`, `darwin/ios`, `android`, `windows`, `linux` and the embedder API in `embedder`; the web's is `engine/src/flutter/lib/web_ui/`.
- **gpui**: Zed's `crates/gpui_macos`, `crates/gpui_windows`, `crates/gpui_linux` and `crates/gpui_web`.
- **Chromium**: macOS in `components/remote_cocoa/`, `ui/base/cocoa/` and `ui/display/mac/`; Windows in `ui/views/win/`, `ui/base/win/` and `ui/display/win/`.
- **winit**: `src/platform_impl/`, one folder per system.
